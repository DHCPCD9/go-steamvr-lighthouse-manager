//! Persisted configuration and UI data models.
//! JSON field names remain stable for saved settings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Live view of a base station, as broadcast to the UI.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LighthouseStation {
    pub id: String,
    pub power_state: i32,
    /// "error" | "preload" | "ready"
    pub status: String,
    pub channel: i32,
    pub name: String,
    pub managed_flags: i32,
    /// Status before the last change, used to animate the status dot color.
    #[serde(default)]
    pub previous_status: String,
    /// Power state before the last change, used to animate the status dot.
    #[serde(default)]
    pub previous_power_state: i32,
}

impl LighthouseStation {
    pub fn is_awake(&self) -> bool {
        Self::power_is_awake(self.power_state)
    }

    pub fn power_is_awake(power_state: i32) -> bool {
        matches!(power_state, 0x0b | 0x01 | 0x09)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LighthouseGroup {
    pub name: String,
    pub base_stations: Vec<String>,
    pub managed_flags: i32,
}

/// Persisted per-base-station config entry.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BaseStationConfiguration {
    #[serde(rename = "mac_address")]
    pub mac_address: String,
    #[serde(rename = "channel")]
    pub channel: i32,
    #[serde(rename = "nickname")]
    pub nickname: String,
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "managed_flags")]
    pub managed_flags: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    #[serde(rename = "is_steamvr_managed")]
    pub is_steamvr_managed: bool,
    #[serde(rename = "is_steamvr_installed")]
    pub is_steamvr_installed: bool,
    #[serde(rename = "allow_tray")]
    pub allow_tray: bool,
    #[serde(rename = "tray_notified")]
    pub tray_notified: bool,
    #[serde(rename = "known_base_stations")]
    pub known_base_stations: HashMap<String, BaseStationConfiguration>,
    #[serde(rename = "groups")]
    pub groups: HashMap<String, LighthouseGroup>,
    #[serde(rename = "branch")]
    pub branch: String,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            is_steamvr_managed: true,
            is_steamvr_installed: false,
            allow_tray: true,
            tray_notified: false,
            known_base_stations: HashMap::new(),
            groups: HashMap::new(),
            branch: "main".to_string(),
        }
    }
}

/// Platform and application version information.
#[derive(Clone, Debug)]
pub struct Platform {
    /// "windows" | "linux" | "macos"
    pub system: String,
    pub version: String,
    pub flags: String,
}

/// Application routes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Devices,
    SoftwareSettings,
    Updates,
    BaseStationSettings(String),
    Group(String),
    GroupSettings(String),
}

/// A single breadcrumb entry in the container title bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Breadcrumb {
    pub text: String,
    pub route: Route,
}
