//! Platform detection, SteamVR process detection, Bluetooth LE, and release links.

pub mod ble;

#[cfg(target_os = "windows")]
#[path = "gatt_windows.rs"]
mod gatt;
#[cfg(not(target_os = "windows"))]
#[path = "gatt_btleplug.rs"]
mod gatt;

pub mod platform;
pub mod steamvr;
pub mod updates;

#[cfg(target_os = "windows")]
pub mod tray;

#[cfg(target_os = "windows")]
pub mod update_apply;
