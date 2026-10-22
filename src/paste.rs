//! Deliver text to the focused app: clipboard + Ctrl+V, then put the
//! user's clipboard back.

use std::time::Duration;

use gpui::{App, ClipboardItem, Timer};

pub(crate) fn deliver(cx: &mut App, text: &str, paste: bool) {
    if !paste {
        cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
        return;
    }
    let saved = cx.read_from_clipboard();
    cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
    send_ctrl_v();
    cx.spawn(async move |cx| {
        Timer::after(Duration::from_millis(300)).await;
        let _ = cx.update(|cx| {
            if let Some(item) = saved {
                cx.write_to_clipboard(item);
            }
        });
    })
    .detach();
}

#[cfg(windows)]
fn send_ctrl_v() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
        VIRTUAL_KEY, VK_CONTROL, VK_V,
    };
    let key = |vk: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inputs = [
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(not(windows))]
fn send_ctrl_v() {}
