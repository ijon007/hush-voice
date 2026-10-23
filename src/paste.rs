//! Deliver text to the focused app: clipboard + Ctrl+V, then put the
//! user's clipboard back.
//!
//! Robustness (intermittent "saved to history but not pasted"):
//! - the app that was focused when recording started is remembered and
//!   re-focused if focus moved to Hush (pill / main window);
//! - we wait for the user's hotkey modifiers (Ctrl/Win/Alt/Shift) to be
//!   physically released, otherwise Ctrl+V arrives as e.g. Ctrl+Win+Alt+V;
//! - the clipboard is opened with retries (another app may hold it);
//! - SendInput's return value is checked; elevated targets (UIPI drops our
//!   input silently) are detected;
//! - the user's clipboard is restored only after the target had time to read
//!   it, only if it still holds our text, and never when the paste failed or
//!   is doubtful (the text then stays on the clipboard for a manual Ctrl+V).
//! Every outcome is appended to %APPDATA%\hush-voice\logs\paste.log.

use std::time::Duration;

use gpui::{App, ClipboardItem, Timer};

/// How long the user may keep holding modifiers before we force key-ups.
const MODIFIER_WAIT_MS: u64 = 1500;
/// Delay before restoring the user's clipboard (the target reads it async).
const RESTORE_DELAY_MS: u64 = 800;

/// Foreground window as a raw handle (0 = none). Taken at record start.
pub(crate) fn foreground() -> isize {
    imp::foreground()
}

/// Prefix a space so back-to-back dictations don't glue together
/// ("working.Alright").
pub(crate) fn with_lead_space(text: &str) -> String {
    if text.is_empty() || text.starts_with(char::is_whitespace) {
        text.to_string()
    } else {
        format!(" {text}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    Pasted,
    /// Input was sent but may not have landed (e.g. elevated target).
    #[cfg_attr(not(windows), allow(dead_code))]
    Doubtful(String),
    Failed(String),
}

impl Outcome {
    /// Put the user's old clipboard back only when we're confident the paste
    /// landed and nobody replaced our text meanwhile.
    pub(crate) fn should_restore(&self, clipboard_still_ours: bool) -> bool {
        matches!(self, Outcome::Pasted) && clipboard_still_ours
    }
}

pub(crate) fn log_line(unix: u64, outcome: &Outcome, chars: usize, detail: &str) -> String {
    let (kind, why) = match outcome {
        Outcome::Pasted => ("pasted", ""),
        Outcome::Doubtful(w) => ("doubtful", w.as_str()),
        Outcome::Failed(w) => ("FAILED", w.as_str()),
    };
    let mut s = format!("{unix} {kind} chars={chars}");
    if !why.is_empty() {
        s.push_str(&format!(" reason=\"{why}\""));
    }
    if !detail.is_empty() {
        s.push_str(&format!(" {detail}"));
    }
    s.push('\n');
    s
}

fn log(outcome: &Outcome, chars: usize, detail: &str) {
    use std::io::Write;
    let Some(dir) = crate::crashlog::logs_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = log_line(unix, outcome, chars, detail);
    if !matches!(outcome, Outcome::Pasted) {
        eprint!("hush paste: {line}");
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("paste.log")) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// `target`: foreground window when recording started (0 = unknown).
pub(crate) fn deliver(cx: &mut App, text: &str, paste: bool, target: isize) {
    if !paste {
        cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
        return;
    }
    let text = with_lead_space(text);
    let saved = cx.read_from_clipboard();
    let chars = text.chars().count();
    // Linux: X11 clipboard is served by a live owner, so GPUI (our process)
    // owns it; Windows sets it inside imp::paste via Win32.
    #[cfg(not(windows))]
    cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
    let ours = text.clone();
    let work = cx.background_executor().spawn(async move { imp::paste(&ours, target) });
    cx.spawn(async move |cx| {
        let (outcome, detail) = work.await;
        log(&outcome, chars, &detail);
        if matches!(outcome, Outcome::Failed(_)) {
            // Make sure the text is at least on the clipboard.
            let _ = cx.update(|cx| cx.write_to_clipboard(ClipboardItem::new_string(text.clone())));
            return;
        }
        Timer::after(Duration::from_millis(RESTORE_DELAY_MS)).await;
        let _ = cx.update(|cx| {
            let still_ours = cx
                .read_from_clipboard()
                .and_then(|c| c.text())
                .is_some_and(|t| t == text);
            if outcome.should_restore(still_ours)
                && let Some(item) = saved
            {
                cx.write_to_clipboard(item);
            }
        });
    })
    .detach();
}

#[cfg(windows)]
mod imp {
    use super::{MODIFIER_WAIT_MS, Outcome};
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::{CloseHandle, GetLastError, GlobalFree, HANDLE, HWND};
    use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
    use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
    use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
    use windows::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
        SendInput, VIRTUAL_KEY, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL,
        VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_V,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
    };

    const CF_UNICODETEXT: u32 = 13;
    const MODS: [VIRTUAL_KEY; 8] =
        [VK_LCONTROL, VK_RCONTROL, VK_LMENU, VK_RMENU, VK_LSHIFT, VK_RSHIFT, VK_LWIN, VK_RWIN];

    pub(super) fn foreground() -> isize {
        unsafe { GetForegroundWindow().0 as isize }
    }

    fn hwnd(raw: isize) -> HWND {
        HWND(raw as *mut _)
    }

    fn pid_of(h: HWND) -> u32 {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        pid
    }

    fn class_of(h: HWND) -> String {
        let mut buf = [0u16; 128];
        let n = unsafe { GetClassNameW(h, &mut buf) }.max(0) as usize;
        String::from_utf16_lossy(&buf[..n])
    }

    fn elevated(pid: u32) -> Option<bool> {
        unsafe {
            let proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut tok = HANDLE::default();
            let ok = OpenProcessToken(proc, TOKEN_QUERY, &mut tok).is_ok();
            let _ = CloseHandle(proc);
            if !ok {
                return None;
            }
            let mut e = TOKEN_ELEVATION::default();
            let mut len = 0u32;
            let r = GetTokenInformation(
                tok,
                TokenElevation,
                Some(&mut e as *mut _ as *mut _),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut len,
            );
            let _ = CloseHandle(tok);
            r.ok()?;
            Some(e.TokenIsElevated != 0)
        }
    }

    fn held_mods() -> Vec<VIRTUAL_KEY> {
        MODS.iter().copied().filter(|vk| unsafe { GetAsyncKeyState(vk.0 as i32) } as u16 & 0x8000 != 0).collect()
    }

    fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
        INPUT {
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
        }
    }

    fn send(inputs: &[INPUT]) -> Result<(), String> {
        let n = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
        if n as usize == inputs.len() {
            Ok(())
        } else {
            Err(format!("SendInput sent {n}/{} (err {})", inputs.len(), unsafe { GetLastError().0 }))
        }
    }

    fn set_clipboard(text: &str) -> Result<(), String> {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let mut opened = false;
        for _ in 0..25 {
            if unsafe { OpenClipboard(None) }.is_ok() {
                opened = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if !opened {
            return Err("clipboard busy (OpenClipboard failed 25x)".into());
        }
        let r = unsafe {
            (|| -> Result<(), String> {
                EmptyClipboard().map_err(|e| format!("EmptyClipboard: {e}"))?;
                let mem = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2).map_err(|e| format!("GlobalAlloc: {e}"))?;
                let p = GlobalLock(mem) as *mut u16;
                if p.is_null() {
                    let _ = GlobalFree(Some(mem));
                    return Err("GlobalLock failed".into());
                }
                std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len());
                let _ = GlobalUnlock(mem);
                if let Err(e) = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(mem.0))) {
                    let _ = GlobalFree(Some(mem));
                    return Err(format!("SetClipboardData: {e}"));
                }
                Ok(())
            })()
        };
        unsafe {
            let _ = CloseClipboard();
        }
        r
    }

    pub(super) fn paste(text: &str, target: isize) -> (Outcome, String) {
        let me = unsafe { GetCurrentProcessId() };
        let mut notes: Vec<String> = Vec::new();

        // 1. Clipboard first (retrying while another app holds it).
        if let Err(e) = set_clipboard(text) {
            return (Outcome::Failed(e), String::new());
        }

        // 2. Let the user's hotkey modifiers go up.
        let start = Instant::now();
        let mut held = held_mods();
        while !held.is_empty() && start.elapsed() < Duration::from_millis(MODIFIER_WAIT_MS) {
            std::thread::sleep(Duration::from_millis(15));
            held = held_mods();
        }
        let waited = start.elapsed().as_millis();
        if waited > 20 {
            notes.push(format!("mod_wait_ms={waited}"));
        }
        if !held.is_empty() {
            // Still held: release them synthetically. A neutral key first
            // keeps a lone Win/Alt key-up from opening Start / the menu bar.
            let mut ups = vec![key(VIRTUAL_KEY(0xE8), false), key(VIRTUAL_KEY(0xE8), true)];
            ups.extend(held.iter().map(|vk| key(*vk, true)));
            ups.extend([key(VK_CONTROL, true), key(VK_MENU, true), key(VK_SHIFT, true)]);
            let _ = send(&ups);
            notes.push(format!("forced_keyup={}", held.len()));
        }

        // 3. Make sure the original app is in front, not the pill / Hush.
        let mut fg = unsafe { GetForegroundWindow() };
        if target != 0
            && fg.0 as isize != target
            && unsafe { IsWindow(Some(hwnd(target))) }.as_bool()
            && (fg.0.is_null() || pid_of(fg) == me)
        {
            let ok = unsafe { SetForegroundWindow(hwnd(target)) }.as_bool();
            std::thread::sleep(Duration::from_millis(40));
            fg = unsafe { GetForegroundWindow() };
            notes.push(format!("refocus={}", if ok && fg.0 as isize == target { "ok" } else { "failed" }));
        }
        if fg.0.is_null() {
            return (Outcome::Failed("no foreground window".into()), notes.join(" "));
        }
        let pid = pid_of(fg);
        notes.push(format!("class={}", class_of(fg)));
        if pid == me {
            return (Outcome::Failed("Hush itself is focused".into()), notes.join(" "));
        }
        if target != 0 && fg.0 as isize != target {
            notes.push("focus_changed_since_start".into());
        }

        // 4. Ctrl+V.
        if let Err(e) = send(&[key(VK_CONTROL, false), key(VK_V, false), key(VK_V, true), key(VK_CONTROL, true)]) {
            return (Outcome::Failed(e), notes.join(" "));
        }
        let me_elev = elevated(me).unwrap_or(false);
        if !me_elev && elevated(pid) == Some(true) {
            return (Outcome::Doubtful("target is elevated (UIPI may block input)".into()), notes.join(" "));
        }
        (Outcome::Pasted, notes.join(" "))
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{MODIFIER_WAIT_MS, Outcome};
    use crate::platform::linux as x;
    use std::time::{Duration, Instant};

    pub(super) fn foreground() -> isize {
        x::active_window()
    }

    /// Clipboard was set by `deliver` (GPUI owns the X selection).
    pub(super) fn paste(_text: &str, target: isize) -> (Outcome, String) {
        let mut notes = Vec::new();
        if !x::has_x11() && !x::is_wayland() {
            return (Outcome::Failed("no display".into()), String::new());
        }
        // 1. Wait for the hotkey modifiers to be released, then force key-ups.
        let start = Instant::now();
        let held = |m: (bool, bool, bool, bool)| m.0 || m.1 || m.2 || m.3;
        while held(x::pressed_modifiers()) && start.elapsed() < Duration::from_millis(MODIFIER_WAIT_MS) {
            std::thread::sleep(Duration::from_millis(15));
        }
        if held(x::pressed_modifiers()) {
            notes.push("forced_modifier_release".into());
            x::release_modifiers();
        }
        // 2. Refocus the original app if Hush took focus.
        let me = std::process::id();
        let mut fg = x::active_window();
        if target != 0 && (fg == 0 || x::window_pid(fg) == Some(me)) {
            let ok = x::activate(target);
            notes.push(format!("refocus={ok}"));
            fg = x::active_window();
        }
        if x::has_x11() && x::window_pid(fg) == Some(me) {
            return (Outcome::Failed("Hush itself is focused".into()), notes.join(" "));
        }
        if target != 0 && fg != target {
            notes.push("focus_changed_since_start".into());
        }
        // Give the clipboard owner a beat before the target asks for it.
        std::thread::sleep(Duration::from_millis(40));
        match x::send_ctrl_v() {
            Ok(via) => {
                notes.push(format!("via={via}"));
                (Outcome::Pasted, notes.join(" "))
            }
            Err(e) => (Outcome::Failed(e), notes.join(" ")),
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    use super::Outcome;
    pub(super) fn foreground() -> isize {
        0
    }
    pub(super) fn paste(_text: &str, _target: isize) -> (Outcome, String) {
        (Outcome::Failed("paste unsupported on this OS".into()), String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lead_space() {
        assert_eq!(with_lead_space("Hi."), " Hi.");
        assert_eq!(with_lead_space(" Hi."), " Hi.");
        assert_eq!(with_lead_space("\nHi"), "\nHi");
        assert_eq!(with_lead_space(""), "");
    }

    #[test]
    fn restore_only_when_pasted_and_ours() {
        assert!(Outcome::Pasted.should_restore(true));
        assert!(!Outcome::Pasted.should_restore(false));
        assert!(!Outcome::Doubtful("x".into()).should_restore(true));
        assert!(!Outcome::Failed("x".into()).should_restore(true));
    }

    #[test]
    fn log_format() {
        assert_eq!(log_line(5, &Outcome::Pasted, 3, ""), "5 pasted chars=3\n");
        assert_eq!(
            log_line(5, &Outcome::Failed("busy".into()), 3, "class=X"),
            "5 FAILED chars=3 reason=\"busy\" class=X\n"
        );
    }
}
