use std::time::Duration;

use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use gpui::{App, Global, Timer};

use crate::chord::Chord;
use crate::overlay::{
    OverlayState, hide_overlay, overlay_visible_unlocked, show_overlay, toggle_lock,
};
use crate::prefs::prefs;

struct HotkeyService {
    manager: GlobalHotKeyManager,
    push_to_talk: Option<HotKey>,
    hold: Chord,
    toggle: Chord,
    capturing: bool,
}

impl Global for HotkeyService {}

/// Shortcuts:
/// - Hold chord: pill + mic while held; release hides and frees the mic.
/// - Toggle chord (press): lock dictation on / off.
pub(crate) fn register_hotkey(cx: &mut App) {
    let manager = GlobalHotKeyManager::new().expect("global hotkey manager");
    cx.set_global(HotkeyService {
        manager,
        push_to_talk: None,
        hold: Chord::hold_default(),
        toggle: Chord::toggle_default(),
        capturing: false,
    });
    cx.set_global(OverlayState {
        handle: None,
        locked: false,
    });
    reload(cx);

    cx.spawn(async move |cx| {
        let mut lock_was_down = false;
        let mut hold_was_down = false;
        loop {
            Timer::after(Duration::from_millis(16)).await;
            let _ = cx.update(|cx| {
                let (capturing, ptt, hold, toggle) = {
                    let s = cx.global::<HotkeyService>();
                    (s.capturing, s.push_to_talk.map(|h| h.id()), s.hold.clone(), s.toggle.clone())
                };
                while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                    if capturing || ptt != Some(event.id()) {
                        continue;
                    }
                    match event.state {
                        HotKeyState::Pressed => show_overlay(cx),
                        HotKeyState::Released => hide_overlay(cx),
                    }
                }

                let hold_down = chord_down(&hold);
                let lock_down = chord_down(&toggle);
                if capturing {
                    lock_was_down = lock_down;
                    hold_was_down = hold_down;
                    return;
                }
                // Rising edge only, so holding it doesn't flicker.
                if lock_down && !lock_was_down {
                    toggle_lock(cx);
                }
                lock_was_down = lock_down;

                if hold_down && !hold_was_down && ptt.is_none() {
                    show_overlay(cx);
                }
                hold_was_down = hold_down;

                // Safety net: a missed key-up must never leave the hold pill stuck.
                if !hold_down && overlay_visible_unlocked(cx) {
                    hide_overlay(cx);
                }
            });
        }
    })
    .detach();
}

pub(crate) fn set_capturing(cx: &mut App, on: bool) {
    let svc = cx.global_mut::<HotkeyService>();
    svc.capturing = on;
    if on {
        // Registered OS hotkeys swallow the combo before GPUI sees it.
        if let Some(old) = svc.push_to_talk.take() {
            let _ = svc.manager.unregister(old);
        }
    } else if svc.push_to_talk.is_none() {
        if let Some(hk) = global_hotkey(&svc.hold.clone()) {
            if svc.manager.register(hk).is_ok() {
                svc.push_to_talk = Some(hk);
            }
        }
    }
}

/// Re-read hold/toggle chords from prefs and re-register the OS hotkey.
pub(crate) fn reload(cx: &mut App) {
    let hold = prefs(cx).hold_chord();
    let toggle = prefs(cx).toggle_chord();
    let svc = cx.global_mut::<HotkeyService>();
    if let Some(old) = svc.push_to_talk.take() {
        let _ = svc.manager.unregister(old);
    }
    svc.hold = hold.clone();
    svc.toggle = toggle;
    if let Some(hk) = global_hotkey(&hold) {
        if svc.manager.register(hk).is_ok() {
            svc.push_to_talk = Some(hk);
        }
    }
}

pub(crate) fn global_hotkey(chord: &Chord) -> Option<HotKey> {
    let code = code_from_key(chord.key.as_deref()?)?;
    let mut mods = Modifiers::empty();
    if chord.ctrl {
        mods |= Modifiers::CONTROL;
    }
    if chord.alt {
        mods |= Modifiers::ALT;
    }
    if chord.shift {
        mods |= Modifiers::SHIFT;
    }
    if chord.win {
        mods |= Modifiers::SUPER;
    }
    Some(HotKey::new((!mods.is_empty()).then_some(mods), code))
}

fn code_from_key(key: &str) -> Option<Code> {
    if key.len() == 1 {
        let c = key.chars().next()?;
        if c.is_ascii_alphabetic() {
            return format!("Key{}", c.to_ascii_uppercase()).parse().ok();
        }
        if c.is_ascii_digit() {
            return format!("Digit{c}").parse().ok();
        }
    }
    if key.starts_with('f') && key[1..].chars().all(|c| c.is_ascii_digit()) {
        return key.to_ascii_uppercase().parse().ok();
    }
    let name = match key {
        "space" => "Space",
        "escape" => "Escape",
        "enter" => "Enter",
        "tab" => "Tab",
        "backspace" => "Backspace",
        "delete" => "Delete",
        "insert" => "Insert",
        "home" => "Home",
        "end" => "End",
        "pageup" => "PageUp",
        "pagedown" => "PageDown",
        "left" => "ArrowLeft",
        "right" => "ArrowRight",
        "up" => "ArrowUp",
        "down" => "ArrowDown",
        "minus" => "Minus",
        "equal" => "Equal",
        "comma" => "Comma",
        "period" => "Period",
        "slash" => "Slash",
        "backslash" => "Backslash",
        "semicolon" => "Semicolon",
        "quote" => "Quote",
        "backquote" => "Backquote",
        "capslock" => "CapsLock",
        other => other,
    };
    name.parse().ok()
}

fn chord_down(chord: &Chord) -> bool {
    if chord.is_empty() {
        return false;
    }
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
        };
        let down = |vk: VIRTUAL_KEY| unsafe { (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0 };
        if chord.ctrl && !down(VK_CONTROL) {
            return false;
        }
        if chord.alt && !down(VK_MENU) {
            return false;
        }
        if chord.shift && !down(VK_SHIFT) {
            return false;
        }
        if chord.win && !(down(VK_LWIN) || down(VK_RWIN)) {
            return false;
        }
        if let Some(key) = &chord.key {
            let Some(vk) = vk_for_key(key) else {
                return false;
            };
            if !down(VIRTUAL_KEY(vk)) {
                return false;
            }
        }
        true
    }
    #[cfg(not(windows))]
    {
        let _ = chord;
        false
    }
}

pub(crate) fn vk_for_key(key: &str) -> Option<u16> {
    if key.len() == 1 {
        let c = key.chars().next()?.to_ascii_uppercase();
        if c.is_ascii_alphanumeric() {
            return Some(c as u16);
        }
    }
    if let Some(n) = key.strip_prefix('f').and_then(|rest| rest.parse::<u8>().ok()) {
        if (1..=24).contains(&n) {
            return Some(0x6F + n as u16);
        }
    }
    Some(match key {
        "space" => 0x20,
        "escape" => 0x1B,
        "enter" => 0x0D,
        "tab" => 0x09,
        "backspace" => 0x08,
        "delete" => 0x2E,
        "insert" => 0x2D,
        "home" => 0x24,
        "end" => 0x23,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "left" => 0x25,
        "up" => 0x26,
        "right" => 0x27,
        "down" => 0x28,
        "minus" => 0xBD,
        "equal" => 0xBB,
        "comma" => 0xBC,
        "period" => 0xBE,
        "slash" => 0xBF,
        "backslash" => 0xDC,
        "semicolon" => 0xBA,
        "quote" => 0xDE,
        "backquote" => 0xC0,
        "capslock" => 0x14,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord::Chord;

    #[test]
    fn hold_default_registers_ctrl_space() {
        let hk = global_hotkey(&Chord::hold_default()).unwrap();
        assert_eq!(hk.key, Code::Space);
        assert!(hk.mods.contains(Modifiers::CONTROL));
    }

    #[test]
    fn modifier_only_has_no_os_hotkey() {
        assert!(global_hotkey(&Chord::toggle_default()).is_none());
    }

    #[test]
    fn vk_space_and_f1() {
        assert_eq!(vk_for_key("space"), Some(0x20));
        assert_eq!(vk_for_key("a"), Some(b'A' as u16));
        assert_eq!(vk_for_key("f1"), Some(0x70));
    }
}
