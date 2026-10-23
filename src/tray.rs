//! System tray icon (dark tile) with Open Hush / Quit, and hiding the main
//! window to the tray. The hotkey keeps working while the window is hidden.

use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Duration;

use gpui::{App, Timer, Window};

/// Main window HWND (0 until the first render).
static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);

/// Remember the main window so the tray can show/hide it.
#[cfg(windows)]
pub(crate) fn register_main(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    if let Ok(handle) = HasWindowHandle::window_handle(window)
        && let RawWindowHandle::Win32(win32) = handle.as_raw()
    {
        MAIN_HWND.store(win32.hwnd.get(), Ordering::Relaxed);
    }

}

/// GPUI 0.2.2's X11 window panics in window_handle() ("not implemented"), so
/// Linux finds the XID itself (platform::linux::own_main_window) on first hide.
#[cfg(not(windows))]
pub(crate) fn register_main(_window: &Window) {}

#[cfg(windows)]
fn hwnd() -> Option<windows::Win32::Foundation::HWND> {
    let raw = MAIN_HWND.load(Ordering::Relaxed);
    (raw != 0).then(|| windows::Win32::Foundation::HWND(raw as *mut _))
}

/// Hide the main window (it stays alive; the tray brings it back).
pub(crate) fn hide_main() {
    #[cfg(windows)]
    if let Some(h) = hwnd() {
        use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};
        unsafe {
            let _ = ShowWindow(h, SW_HIDE);
        }
    }
    #[cfg(target_os = "linux")]
    {
        let mut xid = MAIN_HWND.load(Ordering::Relaxed);
        if xid == 0 {
            xid = crate::platform::linux::own_main_window();
            MAIN_HWND.store(xid, Ordering::Relaxed);
        }
        if xid != 0 {
            crate::platform::linux::hide_window(xid);
        }
    }
}

/// Show, restore and focus the main window.
pub(crate) fn show_main() {
    #[cfg(windows)]
    if let Some(h) = hwnd() {
        use windows::Win32::UI::WindowsAndMessaging::{
            IsIconic, SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow,
        };
        unsafe {
            let _ = ShowWindow(h, SW_SHOW);
            if IsIconic(h).as_bool() {
                let _ = ShowWindow(h, SW_RESTORE);
            }
            let _ = SetForegroundWindow(h);
        }
    }
    #[cfg(target_os = "linux")]
    match MAIN_HWND.load(Ordering::Relaxed) {
        0 => crate::platform::linux::show_window(crate::platform::linux::own_main_window()),
        xid => crate::platform::linux::show_window(xid),
    }
}

/// Keeps the tray icon alive for the app's lifetime.
struct Tray {
    #[cfg(not(target_os = "linux"))]
    _icon: tray_icon::TrayIcon,
}

impl gpui::Global for Tray {}

/// Create the tray icon once (reused if it already exists) and pump its events.
pub(crate) fn init(cx: &mut App) {
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    if cx.has_global::<Tray>() {
        return;
    }
    #[cfg(not(target_os = "linux"))]
    let (open, quit, menu) = {
        let open = MenuItem::new("Open Hush", true, None);
        let quit = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        let _ = menu.append_items(&[&open, &PredefinedMenuItem::separator(), &quit]);
        (open, quit, menu)
    };
    #[cfg(not(target_os = "linux"))]
    let (open_id, quit_id) = (open.id().clone(), quit.id().clone());
    #[cfg(target_os = "linux")]
    let (open_id, quit_id) = (tray_icon::menu::MenuId::new("hush-open"), tray_icon::menu::MenuId::new("hush-quit"));
    #[cfg(not(target_os = "linux"))]
    {
        let mut builder = TrayIconBuilder::new()
            .with_tooltip("Hush")
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false);
        // Exe resource 1 is the dark tile icon (build.rs).
        #[cfg(windows)]
        if let Ok(icon) = tray_icon::Icon::from_resource(1, None) {
            builder = builder.with_icon(icon);
        }
        let Ok(icon) = builder.build() else {
            eprintln!("tray: could not create the tray icon");
            return;
        };
        cx.set_global(Tray { _icon: icon });
    }
    // Linux: tray-icon (libayatana-appindicator) needs GTK and a GTK main loop,
    // which GPUI doesn't run, so the icon lives on its own thread. Menu / click
    // events still arrive through the same global receivers polled below.
    #[cfg(target_os = "linux")]
    {
        let (oid, qid) = (open_id.clone(), quit_id.clone());
        std::thread::Builder::new()
            .name("tray-gtk".into())
            .spawn(move || {
                if gtk::init().is_err() {
                    eprintln!("tray: GTK init failed, no tray icon");
                    return;
                }
                let open = MenuItem::with_id(oid, "Open Hush", true, None);
                let quit = MenuItem::with_id(qid, "Quit", true, None);
                let menu = Menu::new();
                let _ = menu.append_items(&[&open, &PredefinedMenuItem::separator(), &quit]);
                let mut builder = TrayIconBuilder::new()
                    .with_tooltip("Hush")
                    .with_title("Hush")
                    .with_menu(Box::new(menu));
                const RGBA: &[u8] = include_bytes!("../assets/logo/tray-64.rgba");
                if let Ok(icon) = tray_icon::Icon::from_rgba(RGBA.to_vec(), 64, 64) {
                    builder = builder.with_icon(icon);
                }
                match builder.build() {
                    Ok(_icon) => gtk::main(),
                    Err(e) => eprintln!("tray: could not create the tray icon: {e}"),
                }
            })
            .ok();
        cx.set_global(Tray {});
    }

    cx.spawn(async move |cx| {
        loop {
            Timer::after(Duration::from_millis(100)).await;
            while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    show_main();
                }
            }
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if event.id == open_id {
                    show_main();
                } else if event.id == quit_id {
                    let _ = cx.update(|cx| cx.quit());
                    return;
                }
            }
        }
    })
    .detach();
}
