mod audio;

use std::time::{Duration, Instant};

use gpui::{
    App, Bounds, BoxShadow, Context, Global, Timer, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, div, hsla,
    point, prelude::*, px, rgb, size,
};

use crate::theme::{RADIUS_PX, Theme};

use audio::MicMeter;

/// ponytail: true = pill stays up for screenshots (skips hide on key-up)
pub(crate) const PERSIST_OVERLAY: bool = false;

/// Window includes padding so the pill’s soft shadow isn’t clipped by the region.
const OVERLAY_WIDTH: f32 = 104.;
const OVERLAY_HEIGHT: f32 = 56.;
const OVERLAY_BOTTOM_MARGIN: f32 = 48.;

pub(crate) struct OverlayState {
    pub handle: Option<WindowHandle<RecordingPill>>,
    /// Locked via Ctrl+Win+Alt: releasing the hold shortcut won't dismiss it.
    pub locked: bool,
}

impl Global for OverlayState {}

pub(crate) struct RecordingPill {
    shaped: bool,
    /// Shows the lock glyph (Ctrl+Win+Alt mode).
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
}

const BAR_COUNT: usize = 7;
const BAR_W: f32 = 2.;
const BAR_GAP: f32 = 3.;
const BAR_MIN: f32 = 2.;
const BAR_MAX: f32 = 18.;
/// Level mapping in dBFS: the gate (and 0) at -50 dB, full height at -15 dB.
const DB_FLOOR: f32 = -50.;
const DB_CEIL: f32 = -15.;

impl RecordingPill {
    fn new(locked: bool) -> Self {
        let now = Instant::now();
        Self {
            shaped: false,
            locked,
            meter: MicMeter::start(),
            bars: [0.; BAR_COUNT],
            weights: [1.; BAR_COUNT],
            rng: 0x9e37_79b9,
            last_frame: now,
            last_roll: now,
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
        // dB mapping with a soft curve: normal speech moves the bars, hiss stays flat.
        let db = 20. * rms.max(1e-6).log10();
        let level = ((db - DB_FLOOR) / (DB_CEIL - DB_FLOOR)).clamp(0., 1.).powf(0.8);
        for i in 0..BAR_COUNT {
            // Middle bars reach a little higher.
            let center = 1. - (i as f32 - (BAR_COUNT as f32 - 1.) / 2.).abs() * 0.12;
            let target = (level * self.weights[i] * center).min(1.);
            let tau = if target > self.bars[i] { 0.05 } else { 0.18 };
            self.bars[i] += (target - self.bars[i]) * (1. - (-dt / tau).exp());
        }
    }
}

/// Voice meter: equal-width rounded bars, flat dots in silence.
fn meter_bars(bars: &[f32; BAR_COUNT], color: gpui::Rgba) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(BAR_GAP))
        .h(px(BAR_MAX))
        .children(bars.iter().map(move |&v| {
            div()
                .w(px(BAR_W))
                .h(px(BAR_MIN + (BAR_MAX - BAR_MIN) * v))
                .rounded_full()
                .bg(color)
        }))
}

/// Small filled padlock (shackle + body) for locked mode.
fn lock_glyph(color: gpui::Rgba) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .flex_none()
        .child(
            div()
                .w(px(6.))
                .h(px(5.))
                .border_t(px(1.6))
                .border_l(px(1.6))
                .border_r(px(1.6))
                .border_color(color)
                .rounded_t(px(3.)),
        )
        .child(div().w(px(10.)).h(px(7.)).rounded(px(2.)).bg(color))
}

impl Render for RecordingPill {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Redraw every frame only while a mic is live (window exists = visible).
        if self.meter.is_some() {
            self.tick();
            window.request_animation_frame();
        }
        // Always dark, regardless of the app theme.
        let t = Theme::DARK;
        let cream = rgb(0xfaf6f0);
        if !self.shaped {
            self.shaped = clip_overlay_to_capsule(window);
        }

        // Outer transparent canvas; inner material floats with a soft shadow.
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(7.))
                    .px(px(14.))
                    .h(px(36.))
                    .rounded(px(RADIUS_PX + 6.))
                    .overflow_hidden()
                    .bg(t.bg)
                    .border_1()
                    .border_color(t.border_strong)
                    .shadow(vec![BoxShadow {
                        color: hsla(0.0, 0.0, 0.0, 0.28),
                        offset: point(px(0.), px(4.)),
                        blur_radius: px(16.),
                        spread_radius: px(0.),
                    }])
                    .when(self.locked, |el| el.child(lock_glyph(cream)))
                    .child(meter_bars(&self.bars, cream)),
            )
    }
}

fn overlay_bounds(cx: &App) -> Bounds<gpui::Pixels> {
    let pill = size(px(OVERLAY_WIDTH), px(OVERLAY_HEIGHT));
    if let Some(display) = cx.primary_display() {
        let screen = display.bounds();
        Bounds {
            origin: point(
                screen.origin.x + (screen.size.width - pill.width) / 2.,
                screen.origin.y + screen.size.height - pill.height - px(OVERLAY_BOTTOM_MARGIN),
            ),
            size: pill,
        }
    } else {
        Bounds::centered(None, pill, cx)
    }
}

/// Hold-to-dictate: show the pill (no-op if already up).
pub(crate) fn show_overlay(cx: &mut App) {
    open_overlay(cx, false);
}

fn open_overlay(cx: &mut App, locked: bool) {
    if cx.global::<OverlayState>().handle.is_some() {
        return;
    }

    let bounds = overlay_bounds(cx);
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                window_background: WindowBackgroundAppearance::Transparent,
                kind: WindowKind::PopUp,
                focus: false,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                ..Default::default()
            },
            |window, cx| {
                apply_overlay_dwm(window);
                cx.new(|_| RecordingPill::new(locked))
            },
        )
        .ok();

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

/// Ctrl+Win+Alt: lock on (opening or upgrading a held pill), or off.
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
    if let Some(handle) = cx.global_mut::<OverlayState>().handle.take() {
        let _ = handle.update(cx, |pill, window, _| {
            // Release the mic before the window goes away.
            pill.meter = None;
            window.remove_window();
        });
    }
}

fn apply_overlay_dwm(_window: &Window) {
    #[cfg(windows)]
    {
        use windows::Win32::Graphics::Dwm::{
            DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
            DwmSetWindowAttribute,
        };

        let Some(hwnd) = overlay_hwnd(_window) else {
            return;
        };
        unsafe {
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

/// Soft-round the HWND so transparent corners don’t read as a hard rectangle.
fn clip_overlay_to_capsule(_window: &Window) -> bool {
    #[cfg(not(windows))]
    {
        let _ = _window;
        true
    }

    #[cfg(windows)]
    {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
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

        unsafe {
            // Soft round — enough to hide corners; pill itself is rounded in GPUI.
            let radius = height;
            let region = CreateRoundRectRgn(0, 0, width, height, radius, radius);
            SetWindowRgn(hwnd, Some(region), true);
            raise_overlay_window(hwnd);
        }
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
