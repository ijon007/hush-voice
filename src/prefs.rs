//! User preferences, persisted as `key=value` lines in
//! `%APPDATA%\hush-voice\settings`. Changes save immediately.

use gpui::{App, Global};

use crate::theme::app_dir;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayPosition {
    Bottom,
    Top,
}

/// How dictated text is cleaned up before it is inserted.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Style {
    Formal,
    Casual,
    /// Shown as "Lowercase"; stored as "verbatim" for old settings files.
    Lowercase,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeepHistory {
    Forever,
    Days30,
    Days7,
}

impl KeepHistory {
    pub(crate) fn days(self) -> Option<i64> {
        match self {
            KeepHistory::Forever => None,
            KeepHistory::Days30 => Some(30),
            KeepHistory::Days7 => Some(7),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Prefs {
    /// Short chime when the pill opens and closes.
    pub sound_cues: bool,
    pub overlay_position: OverlayPosition,
    /// Input device name; `None` follows the Windows default.
    pub mic: Option<String>,
    /// Selected model id.
    pub model: String,
    pub style: Style,
    /// Path of a user-picked .bin model (model id "custom").
    pub custom_model: Option<String>,
    /// "auto" or an ISO code; ignored by English-only models.
    pub language: String,
    /// Paste into the focused app; off = copy to the clipboard only.
    pub paste: bool,
    pub keep_history: KeepHistory,
    /// Close/minimize hides the window to the tray instead of quitting.
    pub minimize_to_tray: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            sound_cues: true,
            overlay_position: OverlayPosition::Bottom,
            mic: None,
            model: "balanced".into(),
            style: Style::Formal,
            custom_model: None,
            // Auto-detect costs an extra encoder pass (2-4x slower on CPU).
            language: "en".into(),
            paste: true,
            keep_history: KeepHistory::Forever,
            minimize_to_tray: true,
        }
    }
}

impl Global for Prefs {}

pub(crate) fn init(cx: &mut App) {
    cx.set_global(load());
}

pub(crate) fn prefs(cx: &App) -> &Prefs {
    cx.global::<Prefs>()
}

/// Change preferences, save them, and redraw every window.
pub(crate) fn update_prefs(cx: &mut App, f: impl FnOnce(&mut Prefs)) {
    f(cx.global_mut::<Prefs>());
    save(cx.global::<Prefs>());
    cx.refresh_windows();
}

fn path() -> Option<std::path::PathBuf> {
    Some(app_dir()?.join("settings"))
}

fn load() -> Prefs {
    let mut p = Prefs::default();
    let raw = path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    for line in raw.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.trim();
        match key.trim() {
            "sound_cues" => p.sound_cues = value == "true",
            "overlay_position" => {
                p.overlay_position =
                    if value == "top" { OverlayPosition::Top } else { OverlayPosition::Bottom }
            }
            "mic" => p.mic = (!value.is_empty()).then(|| value.to_string()),
            "model" if !value.is_empty() => p.model = value.to_string(),
            "custom_model" => p.custom_model = (!value.is_empty()).then(|| value.to_string()),
            "language" if !value.is_empty() => p.language = value.to_string(),
            "paste" => p.paste = value != "false",
            "minimize_to_tray" => p.minimize_to_tray = value != "false",
            "keep_history" => {
                p.keep_history = match value {
                    "30" => KeepHistory::Days30,
                    "7" => KeepHistory::Days7,
                    _ => KeepHistory::Forever,
                }
            }
            "style" => {
                p.style = match value {
                    "casual" => Style::Casual,
                    "verbatim" | "lowercase" => Style::Lowercase,
                    _ => Style::Formal,
                }
            }
            _ => {}
        }
    }
    if !matches!(p.model.as_str(), "fast" | "balanced" | "best" | "custom") {
        p.model = "balanced".into();
    }
    p
}

/// Read-only load for headless tools (no GPUI app).
pub(crate) fn load_standalone() -> Prefs {
    load()
}

fn save(p: &Prefs) {
    let Some(path) = path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let body = format!(
        "sound_cues={}\noverlay_position={}\nmic={}\nmodel={}\nstyle={}\ncustom_model={}\nlanguage={}\npaste={}\nkeep_history={}\nminimize_to_tray={}\n",
        p.sound_cues,
        match p.overlay_position {
            OverlayPosition::Bottom => "bottom",
            OverlayPosition::Top => "top",
        },
        p.mic.as_deref().unwrap_or(""),
        p.model,
        match p.style {
            Style::Formal => "formal",
            Style::Casual => "casual",
            Style::Lowercase => "verbatim",
        },
        p.custom_model.as_deref().unwrap_or(""),
        p.language,
        p.paste,
        match p.keep_history {
            KeepHistory::Forever => "forever",
            KeepHistory::Days30 => "30",
            KeepHistory::Days7 => "7",
        },
        p.minimize_to_tray,
    );
    let _ = std::fs::write(path, body);
}

/// Launch at login, stored where Windows reads it: the per-user Run key.
pub(crate) mod autostart {
    #[cfg(windows)]
    const RUN_KEY: windows::core::PCWSTR =
        windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    #[cfg(windows)]
    const VALUE: windows::core::PCWSTR = windows::core::w!("Hush");

    #[cfg(windows)]
    pub(crate) fn enabled() -> bool {
        use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
        unsafe { RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, VALUE, RRF_RT_REG_SZ, None, None, None) }
            .is_ok()
    }

    #[cfg(windows)]
    pub(crate) fn set(on: bool) {
        use windows::Win32::System::Registry::{
            HKEY_CURRENT_USER, REG_SZ, RegDeleteKeyValueW, RegSetKeyValueW,
        };
        unsafe {
            if on {
                let Ok(exe) = std::env::current_exe() else { return };
                let cmd: Vec<u16> = format!("\"{}\"", exe.display())
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let _ = RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    RUN_KEY,
                    VALUE,
                    REG_SZ.0,
                    Some(cmd.as_ptr() as *const _),
                    (cmd.len() * 2) as u32,
                );
            } else {
                let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, VALUE);
            }
        }
    }

    #[cfg(not(windows))]
    pub(crate) fn enabled() -> bool {
        false
    }

    #[cfg(not(windows))]
    pub(crate) fn set(_on: bool) {}
}
