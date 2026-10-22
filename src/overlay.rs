pub(crate) mod audio;

use std::time::{Duration, Instant};

use gpui::{
    App, Bounds, BoxShadow, Context, Corners, Global, Pixels, Timer, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, canvas, div,
    fill, hsla, point, prelude::*, px, rgb, size, svg,
};

use crate::prefs::{OverlayPosition, prefs};
use crate::{engine, history, paste, pipeline};
use crate::sounds::{Cue, play};
use crate::theme::Theme;

use audio::MicMeter;

/// ponytail: true = pill stays up for screenshots (skips hide on key-up)
pub(crate) const PERSIST_OVERLAY: bool = false;

/// Window includes padding so the pill’s soft shadow isn’t clipped by the region.
const OVERLAY_WIDTH: f32 = 224.;
const OVERLAY_HEIGHT: f32 = 48.;
/// Pill height (whole px; ~22% shorter than the old 36px).
const PILL_H: f32 = 28.;
const OVERLAY_BOTTOM_MARGIN: f32 = 48.;
const OVERLAY_TOP_MARGIN: f32 = 24.;

pub(crate) struct OverlayState {
    pub handle: Option<WindowHandle<RecordingPill>>,
    /// Locked via the toggle shortcut: releasing the hold shortcut won't dismiss it.
    pub locked: bool,
    /// Foreground window when recording started (paste target, 0 = unknown).
    pub target: isize,
}

impl Global for OverlayState {}

pub(crate) struct RecordingPill {
    shaped: bool,
    /// Shows the lock glyph (toggle-shortcut mode).
    locked: bool,
    /// Live mic level; `None` if no mic / permission denied (flat bars).
    meter: Option<MicMeter>,
    /// Per-bar smoothed heights 0..1 (fast attack, slow release).
    bars: [f32; BAR_COUNT],
    /// Per-bar random weights, re-rolled every ~90ms for natural motion.
    weights: [f32; BAR_COUNT],
    rng: u32,
    last_frame: Instant,
    last_roll: Instant,
    level_meter: LevelMeter,
    /// Entrance start; drives the fade + rise in.
    opened_at: Instant,
    /// Set when hiding; drives the quick fade + drop out.
    closing_at: Option<Instant>,
    /// Recording stopped, whisper is running: bars shimmer.
    transcribing: Option<Instant>,
    /// No model on disk: the pill shows a hint instead of bars.
    no_model: bool,
    #[cfg(debug_assertions)]
    last_log: Instant,
}

const BAR_COUNT: usize = 7;
const BAR_W: f32 = 2.;
const BAR_GAP: f32 = 3.;
const BAR_MIN: f32 = 2.;
const BAR_MAX: f32 = 20.;

/// Speech level from mic dBFS, relative to an adaptive ambient floor.
///
/// - Floor drops fast (~0.3s) toward quieter levels and rises slowly (~6s),
///   only while the gate is closed, so speech never drags it up.
/// - Gate opens at floor + 9 dB, closes below floor + 5 dB (hysteresis).
/// - Open: floor + 6 dB .. floor + 30 dB maps to 0..1 with a soft curve,
///   so normal talking lands around 70-100%.
pub(crate) struct LevelMeter {
    pub floor_db: f32,
    pub open: bool,
    /// Seconds of audio seen; the first ~150ms just seed the floor.
    warmup: f32,
}

const GATE_OPEN_DB: f32 = 9.;
const GATE_CLOSE_DB: f32 = 5.;
const RANGE_START_DB: f32 = 6.;
const RANGE_SPAN_DB: f32 = 24.;

impl LevelMeter {
    pub(crate) fn new() -> Self {
        Self { floor_db: 0., open: false, warmup: 0. }
    }

    /// Returns the 0..1 level for this frame (exactly 0 while gated).
    pub(crate) fn update(&mut self, db: f32, dt: f32) -> f32 {
        // Seed the floor from the room (gate stays closed): skip the first 200ms
        // (device start-up buffers read near-silent), then take the quietest of
        // the next 150ms.
        if self.warmup < 0.35 {
            if self.warmup >= 0.2 {
                self.floor_db = if self.floor_db == 0. { db } else { self.floor_db.min(db) };
            }
            self.warmup += dt;
            return 0.;
        }
        let rel = db - self.floor_db;
        if self.open {
            // Steady noise (fan, hum) must not hold the gate open forever:
            // creep toward it slowly; real speech is far too short to matter.
            self.floor_db += (db - self.floor_db) * (1. - (-dt / 3.0).exp());
        }
        self.open = if self.open { rel > GATE_CLOSE_DB } else { rel > GATE_OPEN_DB };
        if !self.open {
            let tau = if db < self.floor_db { 0.3 } else { 6.0 };
            self.floor_db += (db - self.floor_db) * (1. - (-dt / tau).exp());
            self.floor_db = self.floor_db.clamp(-90., -25.);
            return 0.;
        }
        ((rel - RANGE_START_DB) / RANGE_SPAN_DB).clamp(0., 1.).powf(0.7)
    }
}

impl RecordingPill {
    fn new(locked: bool, mic: Option<String>, no_model: bool) -> Self {
        let now = Instant::now();
        Self {
            shaped: false,
            locked,
            meter: if no_model { None } else { MicMeter::start(mic.as_deref()) },
            bars: [0.; BAR_COUNT],
            weights: [1.; BAR_COUNT],
            rng: 0x9e37_79b9,
            last_frame: now,
            last_roll: now,
            level_meter: LevelMeter::new(),
            opened_at: now,
            closing_at: None,
            transcribing: None,
            no_model,
            #[cfg(debug_assertions)]
            last_log: now,
        }
    }

    fn rand(&mut self) -> f32 {
        // xorshift32: cheap per-bar jitter, no extra crate.
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Advance the bar heights by one frame.
    fn tick(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        if (now - self.last_roll).as_millis() >= 90 {
            self.last_roll = now;
            for i in 0..BAR_COUNT {
                self.weights[i] = 0.45 + 0.55 * self.rand();
            }
        }
        let rms = self.meter.as_ref().map(|m| m.level()).unwrap_or(0.);
        let db = 20. * rms.max(1e-6).log10();
        let level = self.level_meter.update(db, dt);
        #[cfg(debug_assertions)]
        if (now - self.last_log).as_millis() >= 250 {
            self.last_log = now;
            eprintln!(
                "hush meter: db={db:.1} floor={:.1} gate={} level={level:.2}",
                self.level_meter.floor_db,
                if self.level_meter.open { "open" } else { "closed" },
            );
        }
        for i in 0..BAR_COUNT {
            // Middle bars reach a little higher.
            let center = 1. - (i as f32 - (BAR_COUNT as f32 - 1.) / 2.).abs() * 0.12;
            let target = (level * self.weights[i] * center).min(1.);
            let tau = if target > self.bars[i] { 0.05 } else { 0.18 };
            self.bars[i] += (target - self.bars[i]) * (1. - (-dt / tau).exp());
        }
    }
}

/// Entrance: 200ms, exit: 120ms.
const ENTER_MS: f32 = 200.;
const EXIT_MS: f32 = 120.;
/// Entrance rises this far (px); exit drops half of it.
const RISE: f32 = 6.;
const LOCK_SIZE: f32 = 9.;
const NO_MODEL_W: f32 = 200.;
const LOCK_GAP: f32 = 6.;
const PILL_PAD_X: f32 = 12.;

/// cubic-bezier(0.23, 1, 0.32, 1): strong ease-out.
fn ease_out(t: f32) -> f32 {
    cubic_bezier(0.23, 1., 0.32, 1., t.clamp(0., 1.))
}

fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    let bez = |a: f32, b: f32, t: f32| {
        let u = 1. - t;
        3. * u * u * t * a + 3. * u * t * t * b + t * t * t
    };
    // Solve x(t) = x by bisection (monotonic for 0 <= x1, x2 <= 1).
    let (mut lo, mut hi) = (0f32, 1f32);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.;
        if bez(x1, x2, mid) < x { lo = mid } else { hi = mid }
    }
    bez(y1, y2, (lo + hi) / 2.)
}

/// Round a logical length to whole device pixels.
fn snap(v: f32, scale: f32) -> f32 {
    (v * scale).round() / scale
}

/// Bar geometry in device pixels, so every bar is identical and crisp at any
/// DPI (fractional scales otherwise blur 2px bars into uneven 2-3px smears).
#[derive(Clone, Copy)]
struct BarMetrics {
    w: f32,
    gap: f32,
    min: f32,
    max: f32,
}

impl BarMetrics {
    fn new(scale: f32) -> Self {
        let dev = |v: f32| (v * scale).round().max(1.);
        // Heights even in device px so the vertical center lands on a pixel edge.
        let even = |v: f32| ((v * scale / 2.).round() * 2.).max(2.);
        Self {
            w: dev(BAR_W).max(2.),
            gap: dev(BAR_GAP),
            min: even(BAR_MIN),
            max: even(BAR_MAX),
        }
    }

    /// Group width in logical px.
    fn width(&self, scale: f32) -> f32 {
        (self.w * BAR_COUNT as f32 + self.gap * (BAR_COUNT as f32 - 1.)) / scale
    }
}

/// Voice meter painted as device-pixel-snapped capsules.
fn meter_bars(bars: [f32; BAR_COUNT], color: gpui::Rgba, m: BarMetrics, scale: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let to_dev = |p: Pixels| (p / px(1.)) * scale;
            let x0 = to_dev(bounds.origin.x).round();
            let cy = (to_dev(bounds.origin.y) + to_dev(bounds.size.height) / 2.).round();
            for (i, v) in bars.iter().enumerate() {
                let h_dev = ((m.min + (m.max - m.min) * v) / 2.).round() * 2.;
                let x = x0 + i as f32 * (m.w + m.gap);
                let y = cy - h_dev / 2.;
                let r = Bounds {
                    origin: point(px(x / scale), px(y / scale)),
                    size: size(px(m.w / scale), px(h_dev / scale)),
                };
                window.paint_quad(
                    fill(r, color).corner_radii(Corners::all(px(m.w / scale / 2.))),
                );
            }
        },
    )
    .w(px(m.width(scale)))
    .h(px(m.max / scale))
    .flex_none()
}

impl Render for RecordingPill {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        // Redraw every frame only while a mic is live (window exists = visible).
        if self.meter.is_some() {
            self.tick();
        } else if let Some(t0) = self.transcribing {
            // Gentle wave travelling across the bars while whisper runs.
            let t = (now - t0).as_secs_f32();
            for (i, bar) in self.bars.iter_mut().enumerate() {
                let wave = 0.5 + 0.5 * (t * 6.5 - i as f32 * 0.9).sin();
                let target = 0.08 + 0.22 * wave * (t * 4.).min(1.);
                *bar += (target - *bar) * 0.25;
            }
        }
        // Entrance / exit progress (1 = fully shown).
        let enter = ease_out((now - self.opened_at).as_secs_f32() * 1000. / ENTER_MS);
        let (shown, dy) = match self.closing_at {
            Some(t0) => {
                let k = ease_out((now - t0).as_secs_f32() * 1000. / EXIT_MS);
                let base = enter;
                (base * (1. - k), RISE / 2. * k)
            }
            None => (enter, RISE * (1. - enter)),
        };
        let animating = enter < 1. || self.closing_at.is_some();
        if self.meter.is_some() || self.transcribing.is_some() || animating {
            window.request_animation_frame();
        }

        // Always dark, regardless of the app theme.
        let t = Theme::DARK;
        let cream = rgb(0xfaf6f0);
        if !self.shaped {
            self.shaped = clip_overlay_to_capsule(window);
        }

        let scale = window.scale_factor();
        let m = BarMetrics::new(scale);
        let lock_w = if self.locked { snap(LOCK_SIZE, scale) + snap(LOCK_GAP, scale) } else { 0. };
        let pill_w = if self.no_model {
            snap(NO_MODEL_W, scale)
        } else {
            snap(PILL_PAD_X * 2. + m.width(scale) + lock_w, scale)
        };
        let pill_h = snap(PILL_H, scale);
        let left = snap((OVERLAY_WIDTH - pill_w) / 2., scale);
        let top = snap((OVERLAY_HEIGHT - pill_h) / 2. + dy, scale);

        // Outer transparent canvas; inner material floats with a soft shadow.
        div().size_full().relative().child(
            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(pill_w))
                .h(px(pill_h))
                .opacity(shown)
                .flex()
                .items_center()
                .px(px(snap(PILL_PAD_X, scale)))
                .gap(px(snap(LOCK_GAP, scale)))
                // Full capsule: radius = height / 2.
                .rounded(px(pill_h / 2.))
                .overflow_hidden()
                .bg(t.bg)
                .border_1()
                .border_color(t.border_strong)
                .shadow(vec![BoxShadow {
                    color: hsla(0.0, 0.0, 0.0, 0.28 * shown),
                    offset: point(px(0.), px(4.)),
                    blur_radius: px(16.),
                    spread_radius: px(0.),
                }])
                .when(self.locked, |el| {
                    el.child(
                        svg()
                            .path("icons/lock.svg")
                            .size(px(snap(LOCK_SIZE, scale)))
                            .flex_none()
                            .text_color(cream),
                    )
                })
                .when(self.no_model, |el| {
                    el.justify_center().child(
                        div()
                            .text_size(px(12.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(cream)
                            .child("Download a model in Settings"),
                    )
                })
                .when(!self.no_model, |el| el.child(meter_bars(self.bars, cream, m, scale))),
        )
    }
}

fn overlay_display(cx: &App) -> Option<std::rc::Rc<dyn gpui::PlatformDisplay>> {
    let displays = cx.displays();
    #[cfg(windows)]
    if let Some(i) = windows_active_monitor_index() {
        if let Some(d) = displays.get(i) {
            return Some(d.clone());
        }
    }
    cx.primary_display()
}

fn overlay_bounds_for(screen: Bounds<gpui::Pixels>, pos: OverlayPosition) -> Bounds<gpui::Pixels> {
    let pill = size(px(OVERLAY_WIDTH), px(OVERLAY_HEIGHT));
    // Snap to a 4px grid: whole device pixels at 100/125/150/175/200%.
    let grid = |v: Pixels| px(((v / px(1.)) / 4.).round() * 4.);
    Bounds {
        origin: point(
            grid(screen.origin.x + (screen.size.width - pill.width) / 2.),
            grid(match pos {
                OverlayPosition::Bottom => {
                    screen.origin.y + screen.size.height - pill.height - px(OVERLAY_BOTTOM_MARGIN)
                }
                OverlayPosition::Top => screen.origin.y + px(OVERLAY_TOP_MARGIN),
            }),
        ),
        size: pill,
    }
}

/// Index in `EnumDisplayMonitors` order, same as GPUI's `DisplayId`.
#[cfg(windows)]
fn windows_active_monitor_index() -> Option<usize> {
    use windows::Win32::Foundation::{LPARAM, POINT};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, HMONITOR, MONITOR_DEFAULTTONEAREST, MonitorFromPoint, MonitorFromWindow,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow};

    unsafe {
        let fg = GetForegroundWindow();
        let mut monitor = if fg.is_invalid() {
            HMONITOR::default()
        } else {
            MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST)
        };
        if monitor.is_invalid() {
            let mut pt = POINT::default();
            GetCursorPos(&mut pt).ok()?;
            monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        }
        if monitor.is_invalid() {
            return None;
        }

        let mut monitors: Vec<HMONITOR> = Vec::new();
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect_monitors),
            LPARAM(&mut monitors as *mut _ as isize),
        );
        monitors.iter().position(|h| *h == monitor)
    }
}

#[cfg(windows)]
unsafe extern "system" fn collect_monitors(
    hmonitor: windows::Win32::Graphics::Gdi::HMONITOR,
    _hdc: windows::Win32::Graphics::Gdi::HDC,
    _place: *mut windows::Win32::Foundation::RECT,
    data: windows::Win32::Foundation::LPARAM,
) -> windows::core::BOOL {
    let monitors = data.0 as *mut Vec<windows::Win32::Graphics::Gdi::HMONITOR>;
    unsafe { (*monitors).push(hmonitor) };
    windows::core::BOOL(1)
}

/// Hold-to-dictate: show the pill (no-op if already up).
pub(crate) fn show_overlay(cx: &mut App) {
    open_overlay(cx, false);
}

fn open_overlay(cx: &mut App, locked: bool) {
    if cx.global::<OverlayState>().handle.is_some() {
        return;
    }
    // Remember where the text should go before any Hush window appears.
    cx.global_mut::<OverlayState>().target = paste::foreground();

    let display = overlay_display(cx);
    let bounds = display
        .as_ref()
        .map(|d| overlay_bounds_for(d.bounds(), prefs(cx).overlay_position))
        .unwrap_or_else(|| {
            Bounds::centered(None, size(px(OVERLAY_WIDTH), px(OVERLAY_HEIGHT)), cx)
        });
    let mic = prefs(cx).mic.clone();
    let cues = prefs(cx).sound_cues;
    let model = engine::resolve(&prefs(cx).model, prefs(cx).custom_model.as_deref());
    let no_model = model.is_none();
    if let Some(path) = model {
        // Warm the model while the user talks.
        std::thread::spawn(move || engine::preload(&path));
    }
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                display_id: display.map(|d| d.id()),
                titlebar: None,
                window_background: WindowBackgroundAppearance::Opaque,
                kind: WindowKind::PopUp,
                focus: false,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                ..Default::default()
            },
            |window, cx| {
                apply_overlay_dwm(window);
                cx.new(|_| RecordingPill::new(locked, mic, no_model))
            },
        )
        .ok();

    if cues && handle.is_some() && !no_model {
        play(Cue::Start);
    }
    if let Some(handle) = handle.clone() {
        // Wait until HWND has a real client size before SetWindowRgn.
        cx.spawn(async move |cx| {
            for _ in 0..12 {
                Timer::after(Duration::from_millis(16)).await;
                let done = handle
                    .update(cx, |pill, window, _| {
                        if !pill.shaped && clip_overlay_to_capsule(window) {
                            pill.shaped = true;
                        }
                        pill.shaped
                    })
                    .unwrap_or(true);
                if done {
                    break;
                }
            }
        })
        .detach();
    }

    let state = cx.global_mut::<OverlayState>();
    state.handle = handle;
    state.locked = locked;
}

/// Hold released: hide unless locked.
pub(crate) fn hide_overlay(cx: &mut App) {
    if PERSIST_OVERLAY || cx.global::<OverlayState>().locked {
        return;
    }
    force_hide_overlay(cx);
}

pub(crate) fn overlay_visible_unlocked(cx: &App) -> bool {
    let s = cx.global::<OverlayState>();
    s.handle.is_some() && !s.locked
}

/// Toggle shortcut: lock on (opening or upgrading a held pill), or off.
pub(crate) fn toggle_lock(cx: &mut App) {
    let state = cx.global::<OverlayState>();
    match (state.handle.clone(), state.locked) {
        (Some(_), true) => force_hide_overlay(cx),
        (Some(handle), false) => {
            cx.global_mut::<OverlayState>().locked = true;
            let _ = handle.update(cx, |pill, _, cx| {
                pill.locked = true;
                cx.notify();
            });
        }
        (None, _) => open_overlay(cx, true),
    }
}

fn force_hide_overlay(cx: &mut App) {
    cx.global_mut::<OverlayState>().locked = false;
    let Some(handle) = cx.global_mut::<OverlayState>().handle.take() else { return };
    let target = cx.global::<OverlayState>().target;
    // Release the mic and grab what was said.
    let captured = handle
        .update(cx, |pill, _, _| pill.meter.take().map(|m| m.take_samples()))
        .ok()
        .flatten();
    let p = prefs(cx).clone();
    let model = engine::resolve(&p.model, p.custom_model.as_deref());
    let (Some((samples, rate)), Some(model)) = (captured, model) else {
        close_pill(cx, handle);
        return;
    };
    if p.sound_cues {
        play(Cue::Stop);
    }
    let _ = handle.update(cx, |pill, _, cx| {
        pill.transcribing = Some(Instant::now());
        cx.notify();
    });
    let settings = pipeline::Settings {
        style: p.style,
        shortcuts: crate::main_window::load_shortcuts(),
        words: crate::main_window::load_words(),
        language: p.language.clone(),
    };
    let label = engine::model_info(&p.model)
        .map(|m| m.name.to_string())
        .unwrap_or_else(|| {
            model.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
        });
    cx.spawn(async move |cx| {
        let result = cx
            .background_executor()
            .spawn(async move { pipeline::run(&model, &samples, rate, &settings) })
            .await;
        let _ = cx.update(|cx| {
            close_pill(cx, handle);
            match result {
                Ok(Some(out)) => {
                    paste::deliver(cx, &out.text, p.paste, target);
                    history::add(cx, &out.text, out.seconds, &label);
                }
                Ok(None) => {}
                Err(err) => eprintln!("hush: transcription failed: {err}"),
            }
        });
    })
    .detach();
}

/// Fade the pill out, then close its window.
fn close_pill(cx: &mut App, handle: WindowHandle<RecordingPill>) {
    let _ = handle.update(cx, |pill, _, cx| {
        pill.meter = None;
        pill.closing_at = Some(Instant::now());
        cx.notify();
    });
    cx.spawn(async move |cx| {
        Timer::after(Duration::from_millis(EXIT_MS as u64 + 20)).await;
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    })
    .detach();
}

fn apply_overlay_dwm(_window: &Window) {
    #[cfg(windows)]
    {
        use windows::Win32::Graphics::Dwm::{
            DWMNCRP_DISABLED, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_NCRENDERING_POLICY,
            DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DwmSetWindowAttribute,
        };

        let Some(hwnd) = overlay_hwnd(_window) else {
            return;
        };
        unsafe {
            strip_window_frame(hwnd);
            let policy = DWMNCRP_DISABLED;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_NCRENDERING_POLICY,
                &policy as *const _ as _,
                std::mem::size_of_val(&policy) as u32,
            );
            let color = DWMWA_COLOR_NONE;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_BORDER_COLOR,
                &color as *const _ as _,
                std::mem::size_of_val(&color) as u32,
            );
            let corner = DWMWCP_DONOTROUND;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner as *const _ as _,
                std::mem::size_of_val(&corner) as u32,
            );
        }
        raise_overlay_window(hwnd);
    }
}

/// GPUI gives the popup WS_CAPTION + WS_EX_WINDOWEDGE, which Windows draws as a
/// hairline frame around the transparent pill window. Strip every frame bit.
#[cfg(windows)]
unsafe fn strip_window_frame(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WS_BORDER, WS_CAPTION,
        WS_DLGFRAME, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME, WS_EX_STATICEDGE, WS_EX_WINDOWEDGE,
        WS_THICKFRAME,
    };
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let frame = (WS_CAPTION | WS_THICKFRAME | WS_BORDER | WS_DLGFRAME).0 as isize;
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let ex_frame =
            (WS_EX_WINDOWEDGE | WS_EX_CLIENTEDGE | WS_EX_DLGMODALFRAME | WS_EX_STATICEDGE).0 as isize;
        if style & frame == 0 && ex & ex_frame == 0 {
            return;
        }
        SetWindowLongPtrW(hwnd, GWL_STYLE, style & !frame);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex & !ex_frame);
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

/// Finish window setup once the HWND has a real size (DWM attrs + topmost).
fn clip_overlay_to_capsule(_window: &Window) -> bool {
    #[cfg(not(windows))]
    {
        let _ = _window;
        true
    }

    #[cfg(windows)]
    {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

        apply_overlay_dwm(_window);

        let Some(hwnd) = overlay_hwnd(_window) else {
            return false;
        };

        let mut rect = RECT::default();
        if unsafe { GetClientRect(hwnd, &mut rect) }.is_err() {
            return false;
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width < 40 || height < 40 {
            return false;
        }

        // No SetWindowRgn: GDI regions are aliased and were cutting the pill's
        // anti-aliased edge and shadow into a jagged outline. The window is
        // transparent, so GPUI's own AA rounding is all we need.
        raise_overlay_window(hwnd);
        true
    }
}

#[cfg(windows)]
fn raise_overlay_window(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetWindowPos,
    };

    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
}

#[cfg(windows)]
fn overlay_hwnd(window: &Window) -> Option<windows::Win32::Foundation::HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;

    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return None;
    };
    Some(HWND(win32.hwnd.get() as *mut _))
}

#[cfg(test)]
mod tests {
    use super::{OVERLAY_HEIGHT, OVERLAY_WIDTH, LevelMeter, overlay_bounds_for};
    use crate::prefs::OverlayPosition;
    use gpui::{Bounds, point, px, size};

    /// Room tone around -55 dB with +-3 dB jitter, then speech around -30 dB.
    #[test]
    fn silence_stays_flat_and_speech_moves() {
        let mut m = LevelMeter::new();
        let dt = 1. / 60.;
        // Device start-up reads ~-80 dB, then steady -42 dB room noise (seen
        // live): must stay closed from the first visible frame.
        let mut tail: f32 = 0.;
        for i in 0..600 {
            let db = if i < 8 { -80. } else { -42. };
            tail = tail.max(m.update(db, dt));
        }
        assert_eq!(tail, 0., "steady noise must close the gate");
        let mut m = LevelMeter::new();
        let mut max_silent: f32 = 0.;
        for i in 0..240 {
            let jitter = ((i * 7919) % 13) as f32 / 13. * 6. - 3.;
            max_silent = max_silent.max(m.update(-55. + jitter, dt));
        }
        assert_eq!(max_silent, 0., "gate must stay closed on room tone");
        let mut speech = Vec::new();
        for i in 0..60 {
            let jitter = ((i * 104729) % 11) as f32 / 11. * 8. - 4.;
            speech.push(m.update(-30. + jitter, dt));
        }
        let avg = speech.iter().sum::<f32>() / speech.len() as f32;
        assert!(avg > 0.7, "normal speech should reach 70%+, got {avg}");
        // A fan switching on (steady +15 dB) closes the gate again within ~5s.
        let mut last = 1.;
        for _ in 0..600 {
            last = m.update(-40., dt);
        }
        assert_eq!(last, 0., "steady fan noise must be absorbed into the floor");
    }

    #[test]
    fn pill_follows_the_given_monitor() {
        let second = Bounds {
            origin: point(px(1920.), px(0.)),
            size: size(px(1920.), px(1080.)),
        };
        let bounds = overlay_bounds_for(second, OverlayPosition::Bottom);
        assert_eq!(bounds.size, size(px(OVERLAY_WIDTH), px(OVERLAY_HEIGHT)));
        assert!(
            bounds.origin.x >= px(1920.),
            "pill must sit on the second monitor, got x={}",
            f32::from(bounds.origin.x)
        );
        assert!(bounds.origin.x < px(1920. + 1920. - OVERLAY_WIDTH));
    }
}
