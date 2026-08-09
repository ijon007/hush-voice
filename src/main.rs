mod theme;

use std::time::Duration;

use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use gpui::{
    App, Application, Bounds, Context, FontWeight, Global, MouseButton, SharedString, Timer,
    TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowControlArea,
    WindowHandle, WindowKind, WindowOptions, div, point, prelude::*, px, rgb, size,
};
use theme::{
    BG, BORDER, BORDER_STRONG, PRIMARY, PRIMARY_ACTIVE, PRIMARY_TEXT, RADIUS_PX, SURFACE,
    SURFACE_ACTIVE, SURFACE_HOVER, TEXT, TEXT_MUTED, TEXT_SECONDARY,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Settings,
    History,
}

#[derive(Clone)]
struct ModelOption {
    id: &'static str,
    name: SharedString,
    detail: SharedString,
}

#[derive(Clone)]
struct TranscriptRow {
    text: SharedString,
    model: SharedString,
    duration: SharedString,
}

struct HotkeyService {
    _manager: GlobalHotKeyManager,
    push_to_talk_id: u32,
    toggle_id: u32,
}

impl Global for HotkeyService {}

struct OverlayState {
    handle: Option<WindowHandle<RecordingPill>>,
    /// Sticky when opened via Ctrl+Alt+Space toggle; push-to-talk release won't dismiss it.
    sticky: bool,
}

impl Global for OverlayState {}

/// ponytail: true = pill stays up for screenshots (skips hide on key-up)
const PERSIST_OVERLAY: bool = false;

const OVERLAY_WIDTH: f32 = 100.;
const OVERLAY_HEIGHT: f32 = 20.;
const OVERLAY_RADIUS: f32 = 50.;
const OVERLAY_BOTTOM_MARGIN: f32 = 56.;

struct RecordingPill {
    shaped: bool,
}

impl Render for RecordingPill {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if !self.shaped {
            self.shaped = clip_overlay_to_capsule(window);
        }

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.))
            .px(px(12.))
            .rounded(px(OVERLAY_RADIUS))
            .overflow_hidden()
            .bg(SURFACE)
            .child(div().size(px(5.)).rounded_full().bg(PRIMARY))
            .child(
                div()
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(TEXT)
                    .child("Listening"),
            )
    }
}

struct MainWindow {
    tab: Tab,
    models: Vec<ModelOption>,
    active_model: usize,
    hotkey: SharedString,
    history: Vec<TranscriptRow>,
    history_filter: Option<&'static str>,
}

impl MainWindow {
    fn new() -> Self {
        Self {
            tab: Tab::History,
            models: vec![
                ModelOption {
                    id: "whisper-local",
                    name: "Whisper Local".into(),
                    detail: "On-device · fast · private".into(),
                },
                ModelOption {
                    id: "whisper-large",
                    name: "Whisper Large".into(),
                    detail: "Higher accuracy · slower".into(),
                },
                ModelOption {
                    id: "cloud-stt",
                    name: "Cloud STT".into(),
                    detail: "Lowest latency · requires network".into(),
                },
            ],
            active_model: 0,
            hotkey: "Ctrl + Space".into(),
            history: sample_history(),
            history_filter: None,
        }
    }

    fn set_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.tab = tab;
        cx.notify();
    }

    fn select_model(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_model = index;
        cx.notify();
    }

    fn set_history_filter(&mut self, filter: Option<&'static str>, cx: &mut Context<Self>) {
        self.history_filter = filter;
        cx.notify();
    }

    fn filtered_history(&self) -> impl Iterator<Item = &TranscriptRow> {
        let filter = self.history_filter;
        self.history.iter().filter(move |row| match filter {
            None => true,
            Some(model) => row.model.as_ref() == model,
        })
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(BG)
            .text_color(TEXT)
            .font_family(".SystemUIFont")
            .child(self.render_titlebar())
            .child(self.render_header(cx))
            .child(
                div()
                    .id("main-content")
                    .flex_1()
                    .w_full()
                    .px(px(24.))
                    .pb(px(24.))
                    .pt(px(4.))
                    .overflow_y_scroll()
                    .child(match self.tab {
                        Tab::Settings => self.render_settings(cx).into_any_element(),
                        Tab::History => self.render_history(cx).into_any_element(),
                    }),
            )
    }
}

impl MainWindow {
    fn render_titlebar(&self) -> impl IntoElement {
        div()
            .id("titlebar")
            .flex()
            .items_center()
            .w_full()
            .h(px(36.))
            .border_b_1()
            .border_color(BORDER)
            .child(
                div()
                    .id("titlebar-drag")
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .pl(px(14.))
                    .window_control_area(WindowControlArea::Drag)
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(TEXT)
                            .child("Hush"),
                    ),
            )
            .child(window_btn("win-close", "✕", |window, _| {
                window.remove_window();
            }))
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .px(px(24.))
            .pt(px(14.))
            .pb(px(14.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(20.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Welcome back!"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(TEXT_MUTED)
                            .child("Hold Ctrl + Space · Ctrl + Alt + Space toggles"),
                    ),
            )
            .child(self.render_tabs(cx))
    }

    fn render_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .p(px(3.))
            .gap(px(2.))
            .rounded(px(RADIUS_PX))
            .bg(SURFACE)
            .border_1()
            .border_color(BORDER)
            .child(self.tab_button("History", Tab::History, cx))
            .child(self.tab_button("Settings", Tab::Settings, cx))
    }

    fn tab_button(
        &self,
        label: &'static str,
        tab: Tab,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.tab == tab;
        div()
            .id(label)
            .px(px(12.))
            .py(px(6.))
            .rounded(px(RADIUS_PX - 2.))
            .text_size(px(13.))
            .font_weight(if selected {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            })
            .text_color(if selected { TEXT } else { TEXT_SECONDARY })
            .when(selected, |el| el.bg(SURFACE_HOVER))
            .hover(|s| {
                if selected {
                    s
                } else {
                    s.bg(SURFACE_ACTIVE).text_color(TEXT)
                }
            })
            .active(|s| s.opacity(0.85))
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| this.set_tab(tab, cx)))
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(20.))
            .w_full()
            .child(section_label("Model"))
            .child(self.render_model_list(cx))
            .child(section_label("Hotkey"))
            .child(self.render_hotkey())
            .child(
                div()
                    .flex()
                    .justify_end()
                    .pt(px(4.))
                    .child(primary_button("Save changes", "save-settings")),
            )
    }

    fn render_model_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows = Vec::new();
        for (index, model) in self.models.iter().enumerate() {
            rows.push(self.model_row(index, model, cx).into_any_element());
        }
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .children(rows)
    }

    fn model_row(
        &self,
        index: usize,
        model: &ModelOption,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.active_model == index;
        div()
            .id(model.id)
            .flex()
            .items_center()
            .justify_between()
            .px(px(14.))
            .py(px(12.))
            .rounded(px(RADIUS_PX))
            .bg(SURFACE)
            .border_1()
            .border_color(if selected { BORDER_STRONG } else { BORDER })
            .hover(|s| s.bg(SURFACE_HOVER))
            .active(|s| s.bg(SURFACE_ACTIVE).opacity(0.95))
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(model.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(TEXT_MUTED)
                            .child(model.detail.clone()),
                    ),
            )
            .child(selection_dot(selected))
            .on_click(cx.listener(move |this, _, _, cx| this.select_model(index, cx)))
    }

    fn render_hotkey(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(14.))
            .py(px(12.))
            .rounded(px(RADIUS_PX))
            .bg(SURFACE)
            .border_1()
            .border_color(BORDER)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .child("Push to talk"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(TEXT_MUTED)
                            .child("Hold to show overlay, release to hide"),
                    ),
            )
            .child(
                div()
                    .px(px(10.))
                    .py(px(5.))
                    .rounded(px(8.))
                    .bg(BG)
                    .border_1()
                    .border_color(BORDER_STRONG)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(TEXT)
                    .child(self.hotkey.clone()),
            )
    }

    fn render_history(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<_> = self.filtered_history().cloned().collect();

        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .w_full()
            .child(self.render_history_toolbar(cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .children(if rows.is_empty() {
                        vec![empty_history().into_any_element()]
                    } else {
                        rows.into_iter()
                            .map(|row| history_row(row).into_any_element())
                            .collect()
                    }),
            )
    }

    fn render_history_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .h(px(36.))
                    .rounded(px(RADIUS_PX))
                    .bg(SURFACE)
                    .border_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(TEXT_MUTED)
                            .child("Search transcripts…"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(6.))
                    .child(self.filter_chip("All", None, cx))
                    .child(self.filter_chip("Whisper Local", Some("Whisper Local"), cx))
                    .child(self.filter_chip("Cloud STT", Some("Cloud STT"), cx)),
            )
    }

    fn filter_chip(
        &self,
        label: &'static str,
        filter: Option<&'static str>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.history_filter == filter;
        div()
            .id(label)
            .px(px(10.))
            .py(px(5.))
            .rounded(px(8.))
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if selected { PRIMARY_TEXT } else { TEXT_SECONDARY })
            .bg(if selected { PRIMARY } else { SURFACE })
            .border_1()
            .border_color(if selected { PRIMARY } else { BORDER })
            .hover(|s| {
                if selected {
                    s.bg(PRIMARY_ACTIVE)
                } else {
                    s.bg(SURFACE_HOVER).text_color(TEXT)
                }
            })
            .active(|s| s.opacity(0.9))
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| this.set_history_filter(filter, cx)))
    }
}

fn window_btn(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> gpui::AnyElement {
    let is_close = id == "win-close";
    div()
        .id(id)
        .w(px(46.))
        .h(px(36.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.))
        .text_color(TEXT_SECONDARY)
        .hover(move |s| {
            if is_close {
                s.bg(rgb(0xc42b1c)).text_color(TEXT)
            } else {
                s.bg(SURFACE_HOVER).text_color(TEXT)
            }
        })
        .active(|s| s.opacity(0.9))
        .cursor_pointer()
        .child(label)
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, window, cx| on_click(window, cx))
        .into_any_element()
}

fn section_label(text: &'static str) -> impl IntoElement {
    div()
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(TEXT_MUTED)
        .child(text.to_uppercase())
}

fn selection_dot(selected: bool) -> impl IntoElement {
    div()
        .size(px(14.))
        .rounded_full()
        .border_1()
        .border_color(if selected { PRIMARY } else { BORDER_STRONG })
        .when(selected, |el| {
            el.bg(PRIMARY).child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().size(px(5.)).rounded_full().bg(PRIMARY_TEXT)),
            )
        })
}

fn primary_button(label: &'static str, id: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .px(px(14.))
        .py(px(8.))
        .rounded(px(RADIUS_PX))
        .bg(PRIMARY)
        .text_color(PRIMARY_TEXT)
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .hover(|s| s.bg(PRIMARY_ACTIVE))
        .active(|s| s.opacity(0.92))
        .cursor_pointer()
        .child(label)
}

fn history_row(row: TranscriptRow) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .px(px(14.))
        .py(px(12.))
        .rounded(px(RADIUS_PX))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .hover(|s| s.bg(SURFACE_HOVER).border_color(BORDER_STRONG))
        .child(
            div()
                .text_size(px(13.))
                .text_color(TEXT)
                .line_height(px(20.))
                .child(row.text),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(meta_pill(row.model))
                .child(meta_text(row.duration)),
        )
}

fn meta_pill(text: SharedString) -> impl IntoElement {
    div()
        .px(px(7.))
        .py(px(2.))
        .rounded(px(6.))
        .bg(BG)
        .border_1()
        .border_color(BORDER)
        .text_size(px(11.))
        .text_color(TEXT_SECONDARY)
        .child(text)
}

fn meta_text(text: SharedString) -> impl IntoElement {
    div()
        .text_size(px(11.))
        .text_color(TEXT_MUTED)
        .child(text)
}

fn empty_history() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .py(px(56.))
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(TEXT_SECONDARY)
                .child("No transcripts yet"),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(TEXT_MUTED)
                .child("Hold Ctrl + Space to capture the first one"),
        )
}

fn sample_history() -> Vec<TranscriptRow> {
    vec![
        TranscriptRow {
            text: "Schedule a design review for Thursday afternoon.".into(),
            model: "Whisper Local".into(),
            duration: "2.4s".into(),
        },
        TranscriptRow {
            text: "Remind me to send the hush-voice build notes.".into(),
            model: "Cloud STT".into(),
            duration: "1.8s".into(),
        },
        TranscriptRow {
            text: "Draft a calm reply declining the meeting.".into(),
            model: "Whisper Local".into(),
            duration: "3.1s".into(),
        },
        TranscriptRow {
            text: "Open the settings panel and pick a model.".into(),
            model: "Whisper Large".into(),
            duration: "2.0s".into(),
        },
        TranscriptRow {
            text: "Capture action items from the standup recording.".into(),
            model: "Whisper Local".into(),
            duration: "4.2s".into(),
        },
        TranscriptRow {
            text: "Rewrite the onboarding copy to feel quieter.".into(),
            model: "Cloud STT".into(),
            duration: "2.7s".into(),
        },
        TranscriptRow {
            text: "Pin the overlay near the cursor on the next pass.".into(),
            model: "Whisper Local".into(),
            duration: "3.4s".into(),
        },
        TranscriptRow {
            text: "Check whether SendInput needs a focus fallback.".into(),
            model: "Whisper Large".into(),
            duration: "2.1s".into(),
        },
        TranscriptRow {
            text: "Ship a tray icon before wiring the SQLite history store.".into(),
            model: "Cloud STT".into(),
            duration: "3.9s".into(),
        },
        TranscriptRow {
            text: "Keep the recording pill critically damped — no bounce.".into(),
            model: "Whisper Local".into(),
            duration: "2.6s".into(),
        },
    ]
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

fn show_overlay(cx: &mut App) {
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
                cx.new(|_| RecordingPill { shaped: false })
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

    cx.global_mut::<OverlayState>().handle = handle;
}

fn hide_overlay(cx: &mut App) {
    if PERSIST_OVERLAY {
        return;
    }
    if cx.global::<OverlayState>().sticky {
        return;
    }
    force_hide_overlay(cx);
}

fn toggle_overlay(cx: &mut App) {
    if cx.global::<OverlayState>().handle.is_some() {
        cx.global_mut::<OverlayState>().sticky = false;
        force_hide_overlay(cx);
        return;
    }
    cx.global_mut::<OverlayState>().sticky = true;
    show_overlay(cx);
}

fn force_hide_overlay(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<OverlayState>().handle.take() {
        cx.global_mut::<OverlayState>().sticky = false;
        let _ = handle.update(cx, |_, window, _| {
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

/// Clip HWND to a capsule so transparent corners don't show as a rectangle.
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
        if width < 80 || height < 20 {
            return false;
        }

        unsafe {
            // right/bottom exclusive — pass client width/height as the exclusive edge.
            let region = CreateRoundRectRgn(0, 0, width, height, height, height);
            SetWindowRgn(hwnd, Some(region), true);
            raise_overlay_window(hwnd);
        }
        true
    }
}

#[cfg(windows)]
fn raise_overlay_window(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_TOPMOST, SetWindowPos, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
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

fn register_hotkey(cx: &mut App) {
    let manager = GlobalHotKeyManager::new().expect("global hotkey manager");
    let push_to_talk = HotKey::new(Some(Modifiers::CONTROL), Code::Space);
    let toggle = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
    let push_to_talk_id = push_to_talk.id();
    let toggle_id = toggle.id();
    manager
        .register(push_to_talk)
        .expect("register Ctrl+Space hotkey");
    manager
        .register(toggle)
        .expect("register Ctrl+Alt+Space hotkey");
    cx.set_global(HotkeyService {
        _manager: manager,
        push_to_talk_id,
        toggle_id,
    });
    cx.set_global(OverlayState {
        handle: None,
        sticky: false,
    });

    cx.spawn(async move |cx| {
        loop {
            Timer::after(Duration::from_millis(16)).await;
            while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                let _ = cx.update(|cx| {
                    let ids = cx.global::<HotkeyService>();
                    let ptt = ids.push_to_talk_id;
                    let toggle = ids.toggle_id;
                    let id = event.id();
                    match event.state {
                        HotKeyState::Pressed if id == toggle => toggle_overlay(cx),
                        HotKeyState::Pressed if id == ptt => show_overlay(cx),
                        HotKeyState::Released if id == ptt => hide_overlay(cx),
                        _ => {}
                    }
                });
            }
        }
    })
    .detach();
}

fn main() {
    Application::new().run(|cx: &mut App| {
        register_hotkey(cx);

        let bounds = Bounds::centered(None, size(px(1280.), px(720.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Hush".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                is_resizable: false,
                is_minimizable: false,
                window_min_size: Some(size(px(720.), px(520.))),
                ..Default::default()
            },
            |_, cx| cx.new(|_| MainWindow::new()),
        )
        .unwrap();

        if PERSIST_OVERLAY {
            show_overlay(cx);
        }

        cx.activate(true);
    });
}
