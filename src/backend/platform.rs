//! Platform detection and application version information.

use crate::models::Platform;

pub fn current() -> Platform {
    Platform {
        system: if cfg!(target_os = "windows") {
            "windows".into()
        } else if cfg!(target_os = "macos") {
            "macos".into()
        } else {
            "unix".into()
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        flags: String::new(),
    }
}
