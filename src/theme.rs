//! hush-voice visual tokens — neutral dark, white primary, medium radius.

use gpui::Rgba;

const fn hex(rgb: u32) -> Rgba {
    Rgba {
        r: ((rgb >> 16) & 0xff) as f32 / 255.0,
        g: ((rgb >> 8) & 0xff) as f32 / 255.0,
        b: (rgb & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

/// Canvas behind everything. Near-black so elevated surfaces can breathe.
pub const BG: Rgba = hex(0x0c0c0c);
/// Raised surfaces (panels, rows, fields).
pub const SURFACE: Rgba = hex(0x161616);
/// Hover / selected wash on surfaces.
pub const SURFACE_HOVER: Rgba = hex(0x1f1f1f);
/// Pressed state — slightly darker than hover.
pub const SURFACE_ACTIVE: Rgba = hex(0x181818);
/// Hairline borders. Quiet, not decorative.
pub const BORDER: Rgba = hex(0x2a2a2a);
/// Stronger edge for focused / selected chrome.
pub const BORDER_STRONG: Rgba = hex(0x3d3d3d);

pub const TEXT: Rgba = hex(0xffffff);
pub const TEXT_SECONDARY: Rgba = hex(0xa3a3a3);
pub const TEXT_MUTED: Rgba = hex(0x737373);

/// Primary CTA: white fill, dark label.
pub const PRIMARY: Rgba = hex(0xffffff);
pub const PRIMARY_TEXT: Rgba = hex(0x0c0c0c);
pub const PRIMARY_ACTIVE: Rgba = hex(0xe8e8e8);

/// Medium radius — neither chip nor pill.
pub const RADIUS_PX: f32 = 10.0;
