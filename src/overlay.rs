use std::time::Duration;

use gpui::{
    App, Bounds, BoxShadow, Context, FontWeight, Global, Hsla, Timer, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, div, hsla,
    point, prelude::*, px, size,
};

use crate::theme::{RADIUS_PX, theme};

/// ponytail: true = pill stays up for screenshots (skips hide on key-up)
pub(crate) const PERSIST_OVERLAY: bool = false;

/// Window includes padding so the pill’s soft shadow isn’t clipped by the region.
const OVERLAY_WIDTH: f32 = 168.;
const OVERLAY_HEIGHT: f32 = 56.;
const OVERLAY_BOTTOM_MARGIN: f32 = 48.;

pub(crate) struct OverlayState {
    pub handle: Option<WindowHandle<RecordingPill>>,
    /// Sticky when opened via Ctrl+Alt+Space toggle; push-to-talk release won't dismiss it.
    pub sticky: bool,
}

impl Global for OverlayState {}

pub(crate) struct RecordingPill {
    shaped: bool,
}

impl Render for RecordingPill {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
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
                    .gap(px(8.))
                    .px(px(16.))
                    .h(px(36.))
                    .rounded(px(RADIUS_PX + 6.))
                    .overflow_hidden()
                    .bg(t.surface)
                    .border_1()
                    .border_color(t.border_strong)
                    .shadow(vec![BoxShadow {
                        color: hsla(0.0, 0.0, 0.0, 0.12),
                        offset: point(px(0.), px(4.)),
                        blur_radius: px(16.),
                        spread_radius: px(0.),
                    }])
                    .child(
                        div()
                            .size(px(7.))
                            .rounded_full()
                            .bg(t.recording)
                            .shadow(vec![BoxShadow {
                                color: Hsla {
                                    h: 0.01,
                                    s: 0.9,
                                    l: 0.55,
                                    a: 0.35,
                                },
                                offset: point(px(0.), px(0.)),
                                blur_radius: px(6.),
                                spread_radius: px(0.),
                            }]),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.text)
                            .child("Listening"),
                    ),
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

pub(crate) fn show_overlay(cx: &mut App) {
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

pub(crate) fn hide_overlay(cx: &mut App) {
    if PERSIST_OVERLAY {
        return;
    }
    if cx.global::<OverlayState>().sticky {
        return;
    }
    force_hide_overlay(cx);
}

pub(crate) fn toggle_overlay(cx: &mut App) {
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
        if width < 100 || height < 40 {
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
