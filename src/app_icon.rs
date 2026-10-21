//! Runtime window icon: swaps the taskbar / Alt-Tab icon with the theme.
//! Light theme -> dark tile (resource 1), dark theme -> light tile (resource 2).

use gpui::Window;

#[cfg(windows)]
pub(crate) fn apply(window: &Window, dark_theme: bool) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_DEFAULTCOLOR, LoadImageW,
        SM_CXICON, SM_CXSMICON, SendMessageW, WM_SETICON,
    };
    use windows::core::PCWSTR;

    let Ok(handle) = HasWindowHandle::window_handle(window) else { return };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else { return };
    let hwnd = HWND(win32.hwnd.get() as *mut _);
    let id: usize = if dark_theme { 2 } else { 1 };

    unsafe {
        let Ok(module) = GetModuleHandleW(PCWSTR::null()) else { return };
        for (kind, metric) in [(ICON_BIG, SM_CXICON), (ICON_SMALL, SM_CXSMICON)] {
            let px = GetSystemMetrics(metric);
            if let Ok(icon) = LoadImageW(
                Some(module.into()),
                PCWSTR(id as *const u16),
                IMAGE_ICON,
                px,
                px,
                LR_DEFAULTCOLOR,
            ) {
                SendMessageW(
                    hwnd,
                    WM_SETICON,
                    Some(WPARAM(kind as usize)),
                    Some(LPARAM(icon.0 as isize)),
                );
            }
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn apply(_window: &Window, _dark_theme: bool) {}
