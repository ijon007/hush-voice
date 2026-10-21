//! hush-voice visual tokens. Every UI reads colors from `theme(cx)`, which
//! resolves the user's Light / Dark / System choice to a `Theme` palette.

use gpui::{App, Global, Rgba, WindowAppearance};

const fn hex(rgb: u32) -> Rgba {
    Rgba {
        r: ((rgb >> 16) & 0xff) as f32 / 255.0,
        g: ((rgb >> 8) & 0xff) as f32 / 255.0,
        b: (rgb & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

/// Soft continuous radius for groups and controls.
pub const RADIUS_PX: f32 = 12.0;
/// Tighter radius for chips / keycaps inside groups.
pub const RADIUS_SM_PX: f32 = 8.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    Light,
    Dark,
    System,
}

#[derive(Clone, Copy)]
pub struct Theme {
    /// App canvas.
    pub bg: Rgba,
    /// Sidebar rail.
    pub sidebar: Rgba,
    /// Raised / grouped surfaces (cards, lists).
    pub surface: Rgba,
    /// Hover wash on interactive rows; clearly distinct from `bg` and `surface`.
    pub surface_hover: Rgba,
    /// Selected row fill; distinct from hover.
    pub surface_selected: Rgba,
    /// Pressed state.
    pub surface_active: Rgba,
    /// Segmented control track / inset chrome.
    pub track: Rgba,
    /// Hairline borders.
    pub border: Rgba,
    /// Stronger edge for selected chrome / keycaps.
    pub border_strong: Rgba,
    pub text: Rgba,
    pub text_secondary: Rgba,
    pub text_muted: Rgba,
    /// Primary CTA fill and its label.
    pub primary: Rgba,
    pub primary_text: Rgba,
    pub primary_active: Rgba,
    /// Recording / listening dot.
    pub recording: Rgba,
    /// Window close button.
    pub close_hover: Rgba,
    pub close_active: Rgba,
}

impl Theme {
    pub const LIGHT: Theme = Theme {
        bg: hex(0xfaf6f0),
        sidebar: hex(0xf3eee6),
        surface: hex(0xffffff),
        surface_hover: hex(0xf0eae1),
        surface_selected: hex(0xf7f3ed),
        surface_active: hex(0xe8e1d7),
        track: hex(0xebe4da),
        border: hex(0xe8e0d6),
        border_strong: hex(0xd9cfc3),
        text: hex(0x1d1d1f),
        text_secondary: hex(0x6e6e73),
        text_muted: hex(0x8e8e93),
        primary: hex(0x1d1d1f),
        primary_text: hex(0xffffff),
        primary_active: hex(0x3a3a3c),
        recording: hex(0xff3b30),
        close_hover: hex(0xc42b1c),
        close_active: hex(0xa52318),
    };

    pub const DARK: Theme = Theme {
        bg: hex(0x161618),
        sidebar: hex(0x1c1c1f),
        surface: hex(0x202023),
        surface_hover: hex(0x2f2f34),
        surface_selected: hex(0x28282c),
        surface_active: hex(0x38383e),
        track: hex(0x2a2a2e),
        border: hex(0x2e2e33),
        border_strong: hex(0x46464d),
        text: hex(0xf5f5f7),
        text_secondary: hex(0xa1a1a6),
        text_muted: hex(0x8a8a90),
        primary: hex(0xf5f5f7),
        primary_text: hex(0x1d1d1f),
        primary_active: hex(0xd8d8dc),
        recording: hex(0xff453a),
        close_hover: hex(0xc42b1c),
        close_active: hex(0xa52318),
    };
}

pub struct ThemeState {
    pub mode: ThemeMode,
}

impl Global for ThemeState {}

pub fn init(cx: &mut App) {
    cx.set_global(ThemeState { mode: load_mode() });
}

pub fn theme_mode(cx: &App) -> ThemeMode {
    cx.global::<ThemeState>().mode
}

/// Resolved palette for the current mode.
pub fn theme(cx: &App) -> Theme {
    match theme_mode(cx) {
        ThemeMode::Light => Theme::LIGHT,
        ThemeMode::Dark => Theme::DARK,
        ThemeMode::System => match cx.window_appearance() {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Theme::DARK,
            _ => Theme::LIGHT,
        },
    }
}

pub fn set_theme_mode(mode: ThemeMode, cx: &mut App) {
    cx.global_mut::<ThemeState>().mode = mode;
    save_mode(mode);
    cx.refresh_windows();
}

fn settings_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(std::path::PathBuf::from(base).join("hush-voice").join("theme"))
}

fn load_mode() -> ThemeMode {
    let raw = settings_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    match raw.trim() {
        "dark" => ThemeMode::Dark,
        "system" => ThemeMode::System,
        _ => ThemeMode::Light,
    }
}

fn save_mode(mode: ThemeMode) {
    let Some(path) = settings_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let value = match mode {
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
        ThemeMode::System => "system",
    };
    let _ = std::fs::write(path, value);
}
