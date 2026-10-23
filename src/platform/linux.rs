//! Linux system layer: X11 via Xlib + XTest (works under XWayland for X11
//! apps), xdotool as a fallback, XDG autostart. On a pure Wayland session
//! (no DISPLAY) every call degrades to "not available" instead of failing.

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uchar, c_uint, c_ulong};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use x11::{xlib, xtest};

struct Conn(*mut xlib::Display);
// Every use goes through the mutex below, so Xlib never sees two threads at once.
unsafe impl Send for Conn {}

static CONN: Mutex<Option<Conn>> = Mutex::new(None);
static OPENED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Run `f` with our own X connection (opened lazily). None without X11.
fn with_x<T>(f: impl FnOnce(*mut xlib::Display) -> T) -> Option<T> {
    let mut guard = CONN.lock().ok()?;
    if guard.is_none() {
        if OPENED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            return None; // tried before and failed
        }
        let d = unsafe { xlib::XOpenDisplay(std::ptr::null()) };
        if d.is_null() {
            eprintln!("hush: no X11 display (pure Wayland?) - global keys/paste limited");
            return None;
        }
        *guard = Some(Conn(d));
    }
    Some(f(guard.as_ref()?.0))
}

pub(crate) fn has_x11() -> bool {
    std::env::var_os("DISPLAY").is_some() && with_x(|_| ()).is_some()
}

pub(crate) fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE").is_ok_and(|s| s == "wayland")
}

fn tool_exists(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {name}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

// ---------- keyboard state ----------

/// X keysym name for a chord key (same names as hotkey.rs uses).
fn keysym_name(key: &str) -> Option<String> {
    if key.len() == 1 {
        let c = key.chars().next()?;
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_lowercase().to_string());
        }
    }
    if let Some(n) = key.strip_prefix('f').and_then(|r| r.parse::<u8>().ok()) {
        if (1..=24).contains(&n) {
            return Some(format!("F{n}"));
        }
    }
    Some(
        match key {
            "space" => "space",
            "escape" => "Escape",
            "enter" => "Return",
            "tab" => "Tab",
            "backspace" => "BackSpace",
            "delete" => "Delete",
            "insert" => "Insert",
            "home" => "Home",
            "end" => "End",
            "pageup" => "Prior",
            "pagedown" => "Next",
            "left" => "Left",
            "right" => "Right",
            "up" => "Up",
            "down" => "Down",
            "minus" => "minus",
            "equal" => "equal",
            "comma" => "comma",
            "period" => "period",
            "slash" => "slash",
            "backslash" => "backslash",
            "semicolon" => "semicolon",
            "quote" => "apostrophe",
            "backquote" => "grave",
            "capslock" => "Caps_Lock",
            _ => return None,
        }
        .to_string(),
    )
}

fn keycode(d: *mut xlib::Display, sym: &str) -> Option<c_uchar> {
    let name = CString::new(sym).ok()?;
    let ks = unsafe { xlib::XStringToKeysym(name.as_ptr()) };
    if ks == 0 {
        return None;
    }
    let kc = unsafe { xlib::XKeysymToKeycode(d, ks) };
    (kc != 0).then_some(kc)
}

fn keymap(d: *mut xlib::Display) -> [c_char; 32] {
    let mut map = [0 as c_char; 32];
    unsafe { xlib::XQueryKeymap(d, map.as_mut_ptr()) };
    map
}

fn is_down(map: &[c_char; 32], kc: c_uchar) -> bool {
    (map[(kc / 8) as usize] as u8) & (1 << (kc % 8)) != 0
}

fn any_down(d: *mut xlib::Display, map: &[c_char; 32], syms: &[&str]) -> bool {
    syms.iter().any(|s| keycode(d, s).is_some_and(|kc| is_down(map, kc)))
}

/// Physical (ctrl, alt, shift, super).
pub(crate) fn pressed_modifiers() -> (bool, bool, bool, bool) {
    with_x(|d| {
        let m = keymap(d);
        (
            any_down(d, &m, &["Control_L", "Control_R"]),
            any_down(d, &m, &["Alt_L", "Alt_R", "ISO_Level3_Shift"]),
            any_down(d, &m, &["Shift_L", "Shift_R"]),
            any_down(d, &m, &["Super_L", "Super_R"]),
        )
    })
    .unwrap_or_default()
}

/// Is the chord's non-modifier key physically down?
pub(crate) fn key_down(key: &str) -> bool {
    let Some(sym) = keysym_name(key) else { return false };
    with_x(|d| keycode(d, &sym).is_some_and(|kc| is_down(&keymap(d), kc))).unwrap_or(false)
}

// ---------- windows / focus ----------

fn atom(d: *mut xlib::Display, name: &str) -> xlib::Atom {
    let c = CString::new(name).unwrap();
    unsafe { xlib::XInternAtom(d, c.as_ptr(), xlib::False) }
}

fn window_prop(d: *mut xlib::Display, w: c_ulong, prop: &str) -> Option<c_ulong> {
    unsafe {
        let (mut ty, mut fmt, mut n, mut after) = (0, 0 as c_int, 0, 0);
        let mut data: *mut c_uchar = std::ptr::null_mut();
        let ok = xlib::XGetWindowProperty(
            d, w, atom(d, prop), 0, 1, xlib::False, xlib::AnyPropertyType as c_ulong,
            &mut ty, &mut fmt, &mut n, &mut after, &mut data,
        );
        if ok != 0 || data.is_null() || n == 0 {
            if !data.is_null() {
                xlib::XFree(data as *mut _);
            }
            return None;
        }
        let v = *(data as *const c_ulong);
        xlib::XFree(data as *mut _);
        Some(v)
    }
}

/// Focused top-level window (`_NET_ACTIVE_WINDOW`), 0 if unknown.
pub(crate) fn active_window() -> isize {
    with_x(|d| unsafe {
        let root = xlib::XDefaultRootWindow(d);
        window_prop(d, root, "_NET_ACTIVE_WINDOW").filter(|w| *w != 0).unwrap_or_else(|| {
            let (mut w, mut rev) = (0, 0);
            xlib::XGetInputFocus(d, &mut w, &mut rev);
            if w > 1 { w } else { 0 }
        }) as isize
    })
    .unwrap_or(0)
}

pub(crate) fn window_pid(w: isize) -> Option<u32> {
    if w == 0 {
        return None;
    }
    with_x(|d| window_prop(d, w as c_ulong, "_NET_WM_PID").map(|p| p as u32)).flatten()
}

/// Ask the window manager to focus `w` (EWMH), falling back to xdotool.
pub(crate) fn activate(w: isize) -> bool {
    if w == 0 {
        return false;
    }
    let ok = with_x(|d| unsafe {
        let root = xlib::XDefaultRootWindow(d);
        let mut ev: xlib::XEvent = std::mem::zeroed();
        ev.client_message.type_ = xlib::ClientMessage;
        ev.client_message.window = w as c_ulong;
        ev.client_message.message_type = atom(d, "_NET_ACTIVE_WINDOW");
        ev.client_message.format = 32;
        ev.client_message.data.set_long(0, 2); // source: pager
        ev.client_message.data.set_long(1, xlib::CurrentTime as i64);
        let mask = xlib::SubstructureRedirectMask | xlib::SubstructureNotifyMask;
        xlib::XSendEvent(d, root, xlib::False, mask, &mut ev);
        xlib::XMapRaised(d, w as c_ulong);
        xlib::XFlush(d);
        true
    })
    .unwrap_or(false);
    std::thread::sleep(std::time::Duration::from_millis(60));
    if active_window() == w {
        return true;
    }
    let _ = Command::new("xdotool").args(["windowactivate", "--sync", &w.to_string()]).status();
    ok && active_window() == w
}

fn own_windows(d: *mut xlib::Display) -> Vec<(c_ulong, i32, i32)> {
    let me = std::process::id() as c_ulong;
    unsafe {
        let root = xlib::XDefaultRootWindow(d);
        let (mut ty, mut fmt, mut n, mut after) = (0, 0 as c_int, 0, 0);
        let mut data: *mut c_uchar = std::ptr::null_mut();
        let ok = xlib::XGetWindowProperty(
            d, root, atom(d, "_NET_CLIENT_LIST"), 0, 1024, xlib::False, xlib::XA_WINDOW,
            &mut ty, &mut fmt, &mut n, &mut after, &mut data,
        );
        if ok != 0 || data.is_null() {
            return Vec::new();
        }
        let list = std::slice::from_raw_parts(data as *const c_ulong, n as usize).to_vec();
        xlib::XFree(data as *mut _);
        list.into_iter()
            .filter(|w| window_prop(d, *w, "_NET_WM_PID") == Some(me))
            .filter_map(|w| {
                let mut a: xlib::XWindowAttributes = std::mem::zeroed();
                (xlib::XGetWindowAttributes(d, w, &mut a) != 0).then_some((w, a.width, a.height))
            })
            .collect()
    }
}

fn wm_state(d: *mut xlib::Display, w: c_ulong, states: &[&str]) {
    unsafe {
        let root = xlib::XDefaultRootWindow(d);
        for st in states {
            let mut ev: xlib::XEvent = std::mem::zeroed();
            ev.client_message.type_ = xlib::ClientMessage;
            ev.client_message.window = w;
            ev.client_message.message_type = atom(d, "_NET_WM_STATE");
            ev.client_message.format = 32;
            ev.client_message.data.set_long(0, 1); // _NET_WM_STATE_ADD
            ev.client_message.data.set_long(1, atom(d, st) as i64);
            ev.client_message.data.set_long(3, 1);
            let mask = xlib::SubstructureRedirectMask | xlib::SubstructureNotifyMask;
            xlib::XSendEvent(d, root, xlib::False, mask, &mut ev);
        }
    }
}

/// Move the recording pill (our window of exactly w x h) to x,y and keep it on top.
/// False until the window is mapped, so the caller retries.
pub(crate) fn place_overlay(x: i32, y: i32, w: i32, h: i32) -> bool {
    with_x(|d| {
        let Some((win, ..)) = own_windows(d).into_iter().find(|(_, ww, hh)| *ww == w && *hh == h) else {
            return false;
        };
        unsafe {
            // No WM frame/title bar (non-composited WMs like Openbox add one
            // even for client-decorated windows): _MOTIF_WM_HINTS decorations=0.
            let hints: [std::os::raw::c_long; 5] = [2, 0, 0, 0, 0];
            let motif = atom(d, "_MOTIF_WM_HINTS");
            xlib::XChangeProperty(d, win, motif, motif, 32, xlib::PropModeReplace, hints.as_ptr() as *const c_uchar, 5);
            xlib::XMoveWindow(d, win, x, y);
            wm_state(d, win, &["_NET_WM_STATE_ABOVE", "_NET_WM_STATE_SKIP_TASKBAR", "_NET_WM_STATE_SKIP_PAGER"]);
            xlib::XFlush(d);
        }
        true
    })
    .unwrap_or(true) // no X11 (Wayland): nothing we can do, stop retrying
}

/// Our biggest top-level window (the main window; the pill is small).
pub(crate) fn own_main_window() -> isize {
    let me = std::process::id();
    with_x(|d| unsafe {
        let root = xlib::XDefaultRootWindow(d);
        let (mut ty, mut fmt, mut n, mut after) = (0, 0 as c_int, 0, 0);
        let mut data: *mut c_uchar = std::ptr::null_mut();
        let ok = xlib::XGetWindowProperty(
            d, root, atom(d, "_NET_CLIENT_LIST"), 0, 1024, xlib::False, xlib::XA_WINDOW,
            &mut ty, &mut fmt, &mut n, &mut after, &mut data,
        );
        if ok != 0 || data.is_null() {
            return 0;
        }
        let list = std::slice::from_raw_parts(data as *const c_ulong, n as usize).to_vec();
        xlib::XFree(data as *mut _);
        let mut best = (0u64, 0isize);
        for w in list {
            if window_prop(d, w, "_NET_WM_PID") != Some(me as c_ulong) {
                continue;
            }
            let mut attrs: xlib::XWindowAttributes = std::mem::zeroed();
            if xlib::XGetWindowAttributes(d, w, &mut attrs) == 0 {
                continue;
            }
            let area = attrs.width as u64 * attrs.height as u64;
            if area > best.0 {
                best = (area, w as isize);
            }
        }
        best.1
    })
    .unwrap_or(0)
}

pub(crate) fn hide_window(w: isize) {
    let _ = with_x(|d| unsafe {
        xlib::XUnmapWindow(d, w as c_ulong);
        xlib::XFlush(d);
    });
}

pub(crate) fn show_window(w: isize) {
    if w == 0 {
        return;
    }
    let _ = with_x(|d| unsafe {
        xlib::XMapRaised(d, w as c_ulong);
        xlib::XFlush(d);
    });
    activate(w);
}

pub(crate) fn cursor_pos() -> Option<(i32, i32)> {
    with_x(|d| unsafe {
        let root = xlib::XDefaultRootWindow(d);
        let (mut r, mut c, mut x, mut y, mut wx, mut wy, mut m) = (0, 0, 0, 0, 0, 0, 0 as c_uint);
        (xlib::XQueryPointer(d, root, &mut r, &mut c, &mut x, &mut y, &mut wx, &mut wy, &mut m) != 0)
            .then_some((x, y))
    })
    .flatten()
}

// ---------- paste ----------

/// Release any modifiers still held (XTest key-ups), so Ctrl+V lands clean.
pub(crate) fn release_modifiers() {
    let _ = with_x(|d| unsafe {
        let m = keymap(d);
        for s in ["Control_L", "Control_R", "Alt_L", "Alt_R", "Shift_L", "Shift_R", "Super_L", "Super_R"] {
            if let Some(kc) = keycode(d, s).filter(|kc| is_down(&m, *kc)) {
                xtest::XTestFakeKeyEvent(d, kc as c_uint, xlib::False, 0);
            }
        }
        xlib::XFlush(d);
    });
}

/// Ctrl+V into the focused window: XTest first, xdotool / wtype as fallback.
pub(crate) fn send_ctrl_v() -> Result<&'static str, String> {
    let xtest = with_x(|d| unsafe {
        let (mut ev, mut er, mut maj, mut min) = (0, 0, 0, 0);
        if xtest::XTestQueryExtension(d, &mut ev, &mut er, &mut maj, &mut min) == 0 {
            return false;
        }
        let (Some(ctrl), Some(v)) = (keycode(d, "Control_L"), keycode(d, "v")) else { return false };
        xtest::XTestFakeKeyEvent(d, ctrl as c_uint, xlib::True, 0);
        xtest::XTestFakeKeyEvent(d, v as c_uint, xlib::True, 0);
        xtest::XTestFakeKeyEvent(d, v as c_uint, xlib::False, 0);
        xtest::XTestFakeKeyEvent(d, ctrl as c_uint, xlib::False, 0);
        xlib::XSync(d, xlib::False);
        true
    });
    if xtest == Some(true) {
        return Ok("xtest");
    }
    if has_x11() && tool_exists("xdotool") {
        let ok = Command::new("xdotool").args(["key", "--clearmodifiers", "ctrl+v"]).status();
        if ok.is_ok_and(|s| s.success()) {
            return Ok("xdotool");
        }
    }
    if is_wayland() && tool_exists("wtype") {
        let ok = Command::new("wtype").args(["-M", "ctrl", "v", "-m", "ctrl"]).status();
        if ok.is_ok_and(|s| s.success()) {
            return Ok("wtype");
        }
    }
    Err(if is_wayland() && !has_x11() {
        "Wayland without XWayland/wtype: text left on clipboard".into()
    } else {
        "no XTest/xdotool".into()
    })
}

// ---------- autostart (XDG) ----------

fn autostart_file() -> Option<std::path::PathBuf> {
    Some(dirs::config_dir()?.join("autostart").join("hush-voice.desktop"))
}

pub(crate) fn autostart_enabled() -> bool {
    autostart_file().is_some_and(|p| p.is_file())
}

pub(crate) fn set_autostart(on: bool) {
    let Some(path) = autostart_file() else { return };
    if !on {
        let _ = std::fs::remove_file(path);
        return;
    }
    let Ok(exe) = std::env::current_exe() else { return };
    // AppImage: point at the image, not the temporary mount.
    let exe = std::env::var_os("APPIMAGE").map(std::path::PathBuf::from).unwrap_or(exe);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let body = format!(
        "[Desktop Entry]\nType=Application\nName=Hush\nComment=Local voice dictation\nExec=\"{}\"\nIcon=hush-voice\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        exe.display()
    );
    let _ = std::fs::write(path, body);
}

/// Open a folder / URL with the desktop's default handler.
pub(crate) fn open(target: &std::path::Path) {
    let _ = Command::new("xdg-open").arg(target).spawn();
}

/// Play a WAV buffer without blocking (paplay / pw-play / aplay).
pub(crate) fn play_wav(name: &str, buf: &'static [u8]) {
    let path = std::env::temp_dir().join(format!("hush-{name}.wav"));
    if !path.is_file() {
        let _ = std::fs::write(&path, buf);
    }
    std::thread::spawn(move || {
        for player in ["paplay", "pw-play", "aplay"] {
            let mut cmd = Command::new(player);
            if player == "aplay" {
                cmd.arg("-q");
            }
            if cmd.arg(&path).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success()) {
                return;
            }
        }
    });
}

#[link(name = "Xext")]
unsafe extern "C" {
    fn XShapeCombineRectangles(
        d: *mut xlib::Display, w: c_ulong, dest_kind: c_int, x: c_int, y: c_int,
        rects: *mut xlib::XRectangle, n: c_int, op: c_int, ordering: c_int,
    );
}

/// True when a compositing manager runs (so window transparency works).
fn compositor_running(d: *mut xlib::Display) -> bool {
    unsafe {
        let screen = xlib::XDefaultScreen(d);
        let sel = atom(d, &format!("_NET_WM_CM_S{screen}"));
        xlib::XGetSelectionOwner(d, sel) != 0
    }
}

/// Without a compositor the pill's transparent window shows as an opaque
/// box. Clip our w x h window to the capsule `(x, y, cw, ch)` with XShape.
/// True when done (or not needed); false until the window is mapped.
pub(crate) fn shape_overlay(w: i32, h: i32, (x, y, cw, ch): (i32, i32, i32, i32)) -> bool {
    with_x(|d| {
        if compositor_running(d) {
            return true;
        }
        let Some((win, ..)) = own_windows(d).into_iter().find(|(_, ww, hh)| *ww == w && *hh == h) else {
            return false;
        };
        // One rectangle per row approximating the rounded ends.
        let r = ch as f32 / 2.0;
        let mut rects: Vec<xlib::XRectangle> = (0..ch)
            .map(|row| {
                let dy = (row as f32 + 0.5 - r).abs();
                let inset = (r - (r * r - dy * dy).max(0.0).sqrt()).round() as i32;
                xlib::XRectangle {
                    x: (x + inset) as i16,
                    y: (y + row) as i16,
                    width: (cw - 2 * inset).max(0) as u16,
                    height: 1,
                }
            })
            .collect();
        unsafe {
            // ShapeBounding = 0, ShapeSet = 0, Unsorted = 0
            XShapeCombineRectangles(d, win, 0, 0, 0, rects.as_mut_ptr(), rects.len() as c_int, 0, 0);
            xlib::XFlush(d);
        }
        crate::crashlog::debug("pill: no compositor, clipped window to capsule");
        true
    })
    .unwrap_or(true)
}
