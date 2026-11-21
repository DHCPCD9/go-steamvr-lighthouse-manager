//! Config load/save. Configuration lives in
//! `{user-config-dir}/Alumi/Base Station Manager/config.json`.

use crate::models::{Configuration, LighthouseGroup};
use anyhow::Context;
use std::fs;
use std::path::PathBuf;

fn config_folder() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("Alumi").join("Base Station Manager")
}

pub fn config_path() -> PathBuf {
    config_folder().join("config.json")
}

/// Load configuration, creating a default config file on first run.
pub fn load() -> Configuration {
    let folder = config_folder();
    let _ = fs::create_dir_all(&folder);

    let path = config_path();
    if !path.exists() {
        let default = Configuration::default();
        save(&default);
        return default;
    }

    let bytes = fs::read_to_string(&path)
        .unwrap_or_else(|_| "{}".to_string());
    match serde_json::from_str::<Configuration>(&bytes) {
        Ok(mut cfg) => {
            // Fill in fields that may be missing from older configs.
            if cfg.branch.is_empty() {
                cfg.branch = "main".to_string();
            }
            save(&cfg);
            cfg
        }
        Err(_) => {
            // Corrupt config -> recreate.
            let default = Configuration::default();
            save(&default);
            default
        }
    }
}

/// Persist configuration to disk.
pub fn save(cfg: &Configuration) {
    let folder = config_folder();
    let _ = fs::create_dir_all(&folder);
    if let Ok(data) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(config_path(), data);
    }
}

/// Create a group and return its new UUID.
pub fn create_group(cfg: &mut Configuration, name: String) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    cfg.groups.insert(
        id.clone(),
        LighthouseGroup {
            name,
            managed_flags: 6,
            base_stations: Vec::new(),
        },
    );
    id
}

/// Load persisted configuration, preserving read and parse errors.
pub fn load_result() -> anyhow::Result<Configuration> {
    let bytes = fs::read_to_string(config_path()).context("read config")?;
    serde_json::from_str(&bytes).context("parse config")
}
