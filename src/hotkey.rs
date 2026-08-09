use std::time::Duration;

use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use gpui::{App, Global, Timer};

use crate::overlay::{
    OverlayState, hide_overlay, show_overlay, toggle_overlay,
};

struct HotkeyService {
    _manager: GlobalHotKeyManager,
    push_to_talk_id: u32,
    toggle_id: u32,
}

impl Global for HotkeyService {}

pub(crate) fn register_hotkey(cx: &mut App) {
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
