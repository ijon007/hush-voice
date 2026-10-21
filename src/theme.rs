//! hush-voice visual tokens. Every UI reads colors from `theme(cx)`, which
//! resolves the user's Light / Dark choice to a `Theme` palette.

use gpui::{App, Global, Rgba};

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
}

#[derive(Clone, Copy)]
pub struct Theme {
    /// Window background; the sidebar and title bar sit directly on it.
    pub bg: Rgba,
    /// Inset content panel.
    pub panel: Rgba,
    /// Grouped surfaces inside the panel (cards, lists).
    pub surface: Rgba,
    /// Very subtle lift on hovered rows inside cards.
    pub surface_hover: Rgba,
    /// Selected row fill; clearly but softly distinct from hover.
    pub surface_selected: Rgba,
    /// Pressed state.
    pub surface_active: Rgba,
    /// Subtle hover for sidebar items / chrome on `bg`.
    pub nav_hover: Rgba,
    /// Selected sidebar pill on `bg`.
    pub nav_selected: Rgba,
    /// Segmented control track / inset chrome.
    pub track: Rgba,
    /// Hairline borders.
    pub border: Rgba,
    /// Stronger edge for keycaps / unselected controls.
    pub border_strong: Rgba,
    pub text: Rgba,
    pub text_secondary: Rgba,
    pub text_muted: Rgba,
    /// Primary CTA and selection controls (radio dots, toggles).
    pub primary: Rgba,
    pub primary_text: Rgba,
    pub primary_active: Rgba,
    /// Recording / listening dot.
    pub recording: Rgba,
    /// Window close button.
    pub close_hover: Rgba,
    pub close_active: Rgba,
}

/// The app's "black": a warm near-black used as the dark base and as the
/// primary / selection color in light mode.
pub const INK: u32 = 0x141210;

impl Theme {
    pub const LIGHT: Theme = Theme {
        bg: hex(0xece7df),
        panel: hex(0xfbf9f6),
        surface: hex(0xffffff),
        surface_hover: hex(0xfcfaf7),
        surface_selected: hex(0xf6f2ec),
        surface_active: hex(0xf1ece5),
        nav_hover: hex(0xf2eee8),
        nav_selected: hex(0xdfd9d0),
        track: hex(0xefeae3),
        border: hex(0xe7e1d8),
        border_strong: hex(0xd6cec3),
        text: hex(INK),
        text_secondary: hex(0x6b655e),
        text_muted: hex(0x8f8880),
        primary: hex(INK),
        primary_text: hex(0xffffff),
        primary_active: hex(0x2c2925),
        recording: hex(0xff3b30),
        close_hover: hex(0xc42b1c),
        close_active: hex(0xa52318),
    };

    pub const DARK: Theme = Theme {
        bg: hex(INK),
        panel: hex(0x1f1d1a),
        surface: hex(0x262420),
        surface_hover: hex(0x2a2824),
        surface_selected: hex(0x302d29),
        surface_active: hex(0x34312c),
        nav_hover: hex(0x1b1916),
        nav_selected: hex(0x262320),
        track: hex(0x1a1815),
        border: hex(0x312e2a),
        border_strong: hex(0x3d3934),
        text: hex(0xf2eee8),
        text_secondary: hex(0xa8a199),
        text_muted: hex(0x857e76),
        primary: hex(0xf2eee8),
        primary_text: hex(INK),
        primary_active: hex(0xdcd6cd),
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
    };
    let _ = std::fs::write(path, value);
}
