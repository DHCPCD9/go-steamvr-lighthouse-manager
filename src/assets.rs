//! Embedded UI assets: SVG icons and Poppins font faces.

use anyhow::Result;
use gpui::{prelude::*, AssetSource, SharedString};
use std::borrow::Cow;

pub struct Assets;

pub const FONTS: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/Poppins-Regular.ttf"),
    include_bytes!("../assets/fonts/Poppins-Medium.ttf"),
    include_bytes!("../assets/fonts/Poppins-SemiBold.ttf"),
    include_bytes!("../assets/fonts/Poppins-Bold.ttf"),
];

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes: &'static [u8] = match path {
            "icons/base_station.svg" => include_bytes!("../assets/icons/base_station.svg"),
            "icons/status_circle.svg" => include_bytes!("../assets/icons/status_circle.svg"),
            "icons/grouped.svg" => include_bytes!("../assets/icons/grouped.svg"),
            "icons/chevron_down.svg" => include_bytes!("../assets/icons/chevron_down.svg"),
            "icons/power.svg" => include_bytes!("../assets/icons/power.svg"),
            "icons/settings.svg" => include_bytes!("../assets/icons/settings.svg"),
            "icons/close.svg" => include_bytes!("../assets/icons/close.svg"),
            "icons/chevron_right.svg" => include_bytes!("../assets/icons/chevron_right.svg"),
            "icons/eye.svg" => include_bytes!("../assets/icons/eye.svg"),
            "icons/pencil.svg" => include_bytes!("../assets/icons/pencil.svg"),
            "icons/pencil_off.svg" => include_bytes!("../assets/icons/pencil_off.svg"),
            "icons/plus.svg" => include_bytes!("../assets/icons/plus.svg"),
            "icons/refresh.svg" => include_bytes!("../assets/icons/refresh.svg"),
            "icons/download.svg" => include_bytes!("../assets/icons/download.svg"),
            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(bytes)))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(vec![])
    }
}
