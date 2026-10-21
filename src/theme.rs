//! hush-voice visual tokens — calm light, black primary, soft radius.

use gpui::Rgba;

const fn hex(rgb: u32) -> Rgba {
    Rgba {
        r: ((rgb >> 16) & 0xff) as f32 / 255.0,
        g: ((rgb >> 8) & 0xff) as f32 / 255.0,
        b: (rgb & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

/// App canvas — warm light amber; white groups lift without going cool/violet.
pub const BG: Rgba = hex(0xfaf6f0);
/// Raised / grouped surfaces.
pub const SURFACE: Rgba = hex(0xffffff);
/// Hover wash on interactive rows.
pub const SURFACE_HOVER: Rgba = hex(0xf0eae1);
/// Selected row fill (distinct from hover).
pub const SURFACE_SELECTED: Rgba = hex(0xf7f3ed);
/// Pressed state.
pub const SURFACE_ACTIVE: Rgba = hex(0xe8e1d7);
/// Segmented control track / inset chrome.
pub const TRACK: Rgba = hex(0xefe8df);
/// Hairline borders.
pub const BORDER: Rgba = hex(0xe8e0d6);
/// Stronger edge for selected chrome / keycaps.
pub const BORDER_STRONG: Rgba = hex(0xd9cfc3);

pub const TEXT: Rgba = hex(0x1d1d1f);
pub const TEXT_SECONDARY: Rgba = hex(0x6e6e73);
pub const TEXT_MUTED: Rgba = hex(0x8e8e93);

/// Primary CTA: near-black fill, white label.
pub const PRIMARY: Rgba = hex(0x1d1d1f);
pub const PRIMARY_TEXT: Rgba = hex(0xffffff);
pub const PRIMARY_ACTIVE: Rgba = hex(0x3a3a3c);

/// Recording / listening — used sparingly (overlay only).
pub const RECORDING: Rgba = hex(0xff3b30);

/// Soft continuous radius for groups and controls.
pub const RADIUS_PX: f32 = 12.0;
/// Tighter radius for chips / keycaps inside groups.
pub const RADIUS_SM_PX: f32 = 8.0;
