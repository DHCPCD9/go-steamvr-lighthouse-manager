//! Application color palette.

use gpui::{prelude::*, rgb, Hsla, Rgba};

/// Linear interpolation between two HSLA colors, for smooth color transitions.
pub fn lerp_color(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let t = t.clamp(0.0, 1.0);
    gpui::hsla(
        a.h + (b.h - a.h) * t,
        a.s + (b.s - a.s) * t,
        a.l + (b.l - a.l) * t,
        a.a + (b.a - a.a) * t,
    )
}

pub fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}

pub fn c_a(hex: u32, alpha: f32) -> Hsla {
    c(hex).opacity(alpha)
}

pub const BG: u32 = 0x181818; // body background
pub const CARD: u32 = 0x1f1f1f; // base-station / option cards
pub const CARD_HOVER: u32 = 0x434343; // hover / selected card
pub const ACTIVE: u32 = 0x434343; // active pressed card
pub const ACCENT: u32 = 0x1d81ff; // primary blue
pub const ACCENT_HOVER: u32 = 0x66aaff;
pub const ACCENT_BORDER: u32 = 0x9ec9ff;
pub const ACCENT_DISABLED: u32 = 0x2a63ab;
pub const TEXT_MUTED: u32 = 0xc6c6c6;
pub const TEXT_DIM: u32 = 0x888888;
pub const TEXT_DIMMEST: u32 = 0x505050;
pub const RED: u32 = 0xff7373;
pub const GREEN: u32 = 0x7aff73;
pub const CONNECTING: u32 = 0x2b7fff; // frontend Tailwind 4 blue-500
pub const ICON_BLUE: u32 = 0x66aaff; // base-station icon fill
pub const INPUT_BG: u32 = 0x121212; // dropdown button / input field bg
pub const DARKEST: u32 = 0x0d0d0d; // active pressed button

/// A red used as a translucent badge background (`#FF7373` at ~12%).
pub fn red_badge() -> Hsla {
    c(RED).opacity(0.2)
}

/// Muted text `#C6C6C6`.
pub fn muted() -> Hsla {
    c(TEXT_MUTED)
}

/// Dim text `#888888`.
pub fn dim() -> Hsla {
    c(TEXT_DIM)
}

pub fn white() -> Hsla {
    gpui::white()
}

pub fn icon_blue() -> Rgba {
    rgb(ICON_BLUE)
}
