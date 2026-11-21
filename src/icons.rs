//! Icon helpers. Each returns a tintable `svg()` element whose `currentColor`
//! is controlled via `.text_color(...)` by the caller.

use gpui::{Fill::Color, SharedString, Svg, prelude::*, px, svg};

fn icon(path: &'static str, w: f32, h: f32) -> Svg {
    // Note: gpui renders SVGs as monochrome sprites tinted by the element's
    // text color, so callers must apply `.text_color(...)` to give each icon
    // its original color (blue base-station glyphs, muted stroke icons, ...).
    svg().path(SharedString::from(path)).w(px(w)).h(px(h))
}

pub fn base_station() -> Svg {
    icon("icons/base_station.svg", 26.0, 32.0)
}

pub fn status_circle() -> Svg {
    icon("icons/status_circle.svg", 8.0, 8.0)
}

pub fn grouped() -> Svg {
    icon("icons/grouped.svg", 36.0, 36.0)
}

pub fn chevron_down() -> Svg {
    icon("icons/chevron_down.svg", 8.0, 6.0)
}

pub fn power() -> Svg {
    icon("icons/power.svg", 24.0, 24.0)
}

pub fn settings() -> Svg {
    icon("icons/settings.svg", 24.0, 24.0)
}

pub fn close() -> Svg {
    icon("icons/close.svg", 24.0, 24.0)
}

pub fn chevron_right() -> Svg {
    icon("icons/chevron_right.svg", 24.0, 24.0)
}

pub fn eye() -> Svg {
    icon("icons/eye.svg", 24.0, 24.0)
}

pub fn pencil() -> Svg {
    icon("icons/pencil.svg", 24.0, 24.0)
}

pub fn pencil_off() -> Svg {
    icon("icons/pencil_off.svg", 24.0, 24.0)
}

pub fn plus() -> Svg {
    icon("icons/plus.svg", 24.0, 24.0)
}

pub fn refresh() -> Svg {
    icon("icons/refresh.svg", 24.0, 24.0)
}

pub fn download() -> Svg {
    icon("icons/download.svg", 24.0, 24.0)
}
