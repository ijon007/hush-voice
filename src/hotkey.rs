use std::time::Duration;

use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use gpui::{App, Global, Timer};

use crate::overlay::{
    OverlayState, hide_overlay, overlay_visible_unlocked, show_overlay, toggle_lock,
};

struct HotkeyService {
    _manager: GlobalHotKeyManager,
    push_to_talk_id: u32,
}

impl Global for HotkeyService {}

/// Shortcuts:
/// - Hold Ctrl+Space: pill + mic while held; release hides and frees the mic.
/// - Ctrl+Win+Alt (press): toggle locked dictation on / off.
pub(crate) fn register_hotkey(cx: &mut App) {
    let manager = GlobalHotKeyManager::new().expect("global hotkey manager");
    let push_to_talk = HotKey::new(Some(Modifiers::CONTROL), Code::Space);
    let push_to_talk_id = push_to_talk.id();
    manager
        .register(push_to_talk)
        .expect("register Ctrl+Space hotkey");
    cx.set_global(HotkeyService {
        _manager: manager,
        push_to_talk_id,
    });
    cx.set_global(OverlayState {
        handle: None,
        locked: false,
    });

    cx.spawn(async move |cx| {
        let mut lock_was_down = false;
        loop {
            Timer::after(Duration::from_millis(16)).await;
            while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                let _ = cx.update(|cx| {
                    let ptt = cx.global::<HotkeyService>().push_to_talk_id;
                    if event.id() != ptt {
                        return;
                    }
                    match event.state {
                        HotKeyState::Pressed => show_overlay(cx),
                        HotKeyState::Released => hide_overlay(cx),
                    }
                });
            }

            let keys = KeySnapshot::read();
            // Lock chord: rising edge only, so holding it doesn't flicker.
            let lock_down = keys.ctrl && keys.alt && keys.win;
            if lock_down && !lock_was_down {
                let _ = cx.update(|cx| toggle_lock(cx));
            }
            lock_was_down = lock_down;

            // Safety net: a missed key-up must never leave the hold pill stuck.
            if !(keys.ctrl && keys.space) {
                let _ = cx.update(|cx| {
                    if overlay_visible_unlocked(cx) {
                        hide_overlay(cx);
                    }
                });
            }
        }
    })
    .detach();
}

struct KeySnapshot {
    ctrl: bool,
    alt: bool,
    win: bool,
    space: bool,
}

impl KeySnapshot {
    #[cfg(windows)]
    fn read() -> Self {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SPACE,
        };
        let down = |vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY| unsafe {
            (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0
        };
        Self {
            ctrl: down(VK_CONTROL),
            alt: down(VK_MENU),
            win: down(VK_LWIN) || down(VK_RWIN),
            space: down(VK_SPACE),
        }
    }

    #[cfg(not(windows))]
    fn read() -> Self {
        Self { ctrl: false, alt: false, win: false, space: false }
    }
}
