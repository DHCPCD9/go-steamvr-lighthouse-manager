//! Application state: persisted configuration, live stations, groups, platform
//! flags, and the BLE backend handle.

use crate::backend::ble::{self, BluetoothHandle, Command, Event};
use crate::backend::platform;
use crate::backend::steamvr;
use crate::backend::updates;
use crate::config;
use crate::models::{
    BaseStationConfiguration, Configuration, LighthouseGroup, LighthouseStation, Platform,
};
use std::collections::HashMap;

pub struct AppState {
    pub config: Configuration,
    pub lighthouses: HashMap<String, LighthouseStation>,
    pub steamvr_active: bool,
    pub steamvr_state_known: bool,
    pub scanning: bool,
    pub backend_ready: bool,
    pub platform: Platform,
    pub update_status: updates::Status,
    ble: BluetoothHandle,
    steamvr_monitor: Option<steamvr::Monitor>,
    updater: Option<updates::Handle>,
}

impl AppState {
    pub fn new() -> Self {
        let mut config = config::load();
        config.is_steamvr_installed = false;
        let steamvr_monitor = steamvr::start();
        // Initialize Bluetooth and SteamVR independently of the UI thread.
        let ble = ble::start();
        ble.send(Command::StartScan);
        let updater = updates::start();
        let mut lighthouses = HashMap::new();

        // Preload known base stations.
        for (id, bs) in &config.known_base_stations {
            lighthouses.insert(
                id.clone(),
                LighthouseStation {
                    id: id.clone(),
                    power_state: -1,
                    status: "preloaded".into(),
                    channel: bs.channel,
                    name: if bs.nickname.is_empty() {
                        id.clone()
                    } else {
                        bs.nickname.clone()
                    },
                    managed_flags: bs.managed_flags,
                    previous_status: String::new(),
                    previous_power_state: -1,
                },
            );
        }

        let mut state = Self {
            config,
            lighthouses,
            steamvr_active: false,
            steamvr_state_known: false,
            scanning: false,
            backend_ready: true,
            platform: platform::current(),
            update_status: if updates::is_supported() {
                updates::Status::Idle
            } else {
                updates::Status::Unsupported
            },
            ble,
            steamvr_monitor,
            updater,
        };
        #[cfg(target_os = "windows")]
        if let Some(error) = crate::backend::update_apply::startup_error() {
            state.update_status = updates::Status::Error(error);
            return state;
        }
        state.check_updates();
        state
    }

    pub fn ble_handle(&self) -> &BluetoothHandle {
        &self.ble
    }

    /// Clone the BLE event receiver for the UI thread to drain.
    pub fn ble_events(&self) -> flume::Receiver<Event> {
        self.ble.event_rx.clone()
    }

    pub fn steamvr_events(&self) -> Option<flume::Receiver<steamvr::Event>> {
        self.steamvr_monitor.as_ref().map(|monitor| monitor.events.clone())
    }

    pub fn update_events(&self) -> Option<flume::Receiver<updates::Event>> {
        self.updater.as_ref().map(|updater| updater.events.clone())
    }

    pub fn apply_update_status(&mut self, status: updates::Status) {
        self.update_status = status;
    }

    pub fn check_updates(&mut self) {
        self.send_update_command(false);
    }

    pub fn install_update(&mut self) {
        if matches!(self.update_status, updates::Status::Available { .. }) {
            self.send_update_command(true);
        }
    }

    fn send_update_command(&mut self, install: bool) {
        if self.update_status.busy() {
            return;
        }
        if let Some(updater) = &self.updater {
            let branch = self.config.branch.clone();
            let command = if install {
                updates::Command::Install { branch }
            } else {
                updates::Command::Check { branch }
            };
            self.update_status = match updater.send(command) {
                Ok(()) => updates::Status::Checking,
                Err(error) => updates::Status::Error(error.to_string()),
            };
        }
    }

    pub fn apply_steamvr_event(&mut self, event: steamvr::Event) {
        match event {
            steamvr::Event::State { active, installed } => {
                let changed = !self.steamvr_state_known || self.steamvr_active != active
                    || self.config.is_steamvr_installed != installed;
                self.steamvr_active = active;
                self.steamvr_state_known = true;
                self.config.is_steamvr_installed = installed;
                if changed {
                    self.reconcile_steamvr_power();
                }
            }
            steamvr::Event::Error(error) => {
                // An unreadable process snapshot is not a SteamVR exit.
                self.steamvr_state_known = false;
                log::error!("SteamVR monitoring failed: {error}");
            }
        }
    }

    fn reconcile_station_power(&self, station: &LighthouseStation) {
        if !self.steamvr_state_known || !self.config.is_steamvr_managed
            || !self.config.is_steamvr_installed || station.status != "ready"
        {
            return;
        }
        let (flag, state, satisfied) = if self.steamvr_active {
            (2, 1, station.is_awake())
        } else {
            (4, 0, station.power_state == 0)
        };
        if station.managed_flags & flag != 0 && !satisfied {
            self.ble.send(Command::ChangePower { id: station.id.clone(), state });
        }
    }

    fn reconcile_steamvr_power(&self) {
        for station in self.lighthouses.values() {
            self.reconcile_station_power(station);
        }
    }

    fn set_steamvr_management(&mut self, enabled: bool) {
        let changed = self.config.is_steamvr_managed != enabled;
        self.config.is_steamvr_managed = enabled;
        if changed && enabled {
            self.reconcile_steamvr_power();
        }
    }

    pub fn poll_ble_events(&mut self) {
        while let Ok(event) = self.ble.event_rx.try_recv() {
            self.apply_ble_event(event);
        }
    }

    pub fn apply_ble_event(&mut self, event: Event) {
        match event {
            Event::ScanStarted => {
                self.scanning = true;
            }
            Event::ScanStopped => {
                self.scanning = false;
            }
            Event::Found { id } => {
                let bs = self.lighthouses.entry(id.clone()).or_insert_with(|| {
                    ble::preloaded_station(id.clone(), id, -1)
                });
                Self::set_status(bs, "preloaded".into());
            }
            Event::Ready {
                id,
                address,
                channel,
                power_state,
            } => {
                let bs = self.lighthouses.entry(id.clone()).or_insert_with(|| {
                    ble::preloaded_station(id.clone(), id.clone(), -1)
                });
                Self::set_status(bs, "ready".into());
                bs.channel = channel;
                Self::set_power(bs, power_state);

                let cfg = self.config.known_base_stations.entry(id.clone()).or_insert_with(|| {
                    BaseStationConfiguration {
                        id: id.clone(),
                        nickname: bs.name.clone(),
                        managed_flags: bs.managed_flags,
                        ..Default::default()
                    }
                });
                cfg.mac_address = address;
                cfg.channel = channel;
                self.save();
                if let Some(station) = self.lighthouses.get(&id) {
                    self.reconcile_station_power(station);
                }
            }
            Event::Status { id, status } => {
                if let Some(bs) = self.lighthouses.get_mut(&id) {
                    Self::set_status(bs, status);
                }
            }
            Event::Channel { id, channel } => {
                if let Some(bs) = self.lighthouses.get_mut(&id) {
                    bs.channel = channel;
                }
            }
            Event::Power { id, power_state } => {
                if let Some(bs) = self.lighthouses.get_mut(&id) {
                    Self::set_power(bs, power_state);
                }
            }
            Event::Error { id, .. } => {
                if let Some(bs) = self.lighthouses.get_mut(&id) {
                    Self::set_status(bs, "error".into());
                }
            }
        }
    }

    /// Update a station's status, remembering the previous value for the
    /// status-dot color transition.
    fn set_status(bs: &mut LighthouseStation, status: String) {
        bs.previous_status = std::mem::take(&mut bs.status);
        bs.status = status;
    }

    /// Update a station's power state, remembering the previous value for the
    /// status-dot color transition.
    fn set_power(bs: &mut LighthouseStation, power_state: i32) {
        bs.previous_power_state = bs.power_state;
        bs.power_state = power_state;
    }

    pub fn start_scan(&mut self) {
        self.ble.send(Command::StartScan);
    }

    pub fn change_power(&mut self, id: &str, status: &str) -> String {
        let state = match status {
            "standingby" => 0x02u8,
            "sleep" => 0x00u8,
            "awake" => 0x01u8,
            _ => return "unknown status".into(),
        };
        self.ble.send(Command::ChangePower {
            id: id.to_string(),
            state,
        });
        if let Some(bs) = self.lighthouses.get_mut(id) {
            bs.power_state = state as i32;
        }
        "ok".into()
    }

    pub fn change_channel(&mut self, id: &str, channel: u8) -> String {
        self.ble.send(Command::ChangeChannel {
            id: id.to_string(),
            channel,
        });
        if let Some(bs) = self.lighthouses.get_mut(id) {
            bs.channel = channel as i32;
        }
        // Persist channel.
        if let Some(cfg) = self.config.known_base_stations.get_mut(id) {
            cfg.channel = channel as i32;
        }
        self.save();
        "ok".into()
    }

    pub fn identify(&mut self, id: &str) -> String {
        self.ble.send(Command::Identify { id: id.to_string() });
        "ok".into()
    }

    pub fn forget(&mut self, id: &str) {
        self.lighthouses.remove(id);
        self.config.known_base_stations.remove(id);
        self.save();
    }

    pub fn set_nickname(&mut self, id: &str, name: String) {
        let name = if name.is_empty() { id.to_string() } else { name };
        if let Some(bs) = self.lighthouses.get_mut(id) {
            bs.name = name.clone();
        }
        if let Some(cfg) = self.config.known_base_stations.get_mut(id) {
            cfg.nickname = name;
        }
        self.save();
    }

    pub fn set_managed_flags(&mut self, id: &str, flag: i32) {
        let current = self
            .lighthouses
            .get(id)
            .map(|bs| bs.managed_flags)
            .unwrap_or_else(|| {
                self.config
                    .known_base_stations
                    .get(id)
                    .map(|c| c.managed_flags)
                    .unwrap_or(0)
            });

        let next = if (current & flag) > 0 {
            current & !flag
        } else {
            current | flag
        };

        if let Some(bs) = self.lighthouses.get_mut(id) {
            bs.managed_flags = next;
        }
        if let Some(cfg) = self.config.known_base_stations.get_mut(id) {
            cfg.managed_flags = next;
        }
        self.save();
        if let Some(station) = self.lighthouses.get(id) {
            self.reconcile_station_power(station);
        }
    }

    pub fn create_group(&mut self, name: String, stations: Vec<String>) -> String {
        let id = config::create_group(&mut self.config, name);
        if let Some(group) = self.config.groups.get_mut(&id) {
            group.base_stations = stations;
        }
        self.save();
        id
    }

    pub fn rename_group(&mut self, id: &str, name: String) -> String {
        match self.config.groups.get_mut(id) {
            Some(group) => {
                group.name = name;
                self.save();
                "ok".into()
            }
            None => "error: Unknown group".into(),
        }
    }

    pub fn remove_group(&mut self, id: &str) {
        self.config.groups.remove(id);
        self.save();
    }

    pub fn update_group_flags(&mut self, id: &str, flags: i32) -> String {
        match self.config.groups.get_mut(id) {
            Some(group) => {
                group.managed_flags = flags;
                self.save();
                "ok".into()
            }
            None => "error: Unknown group".into(),
        }
    }

    pub fn toggle_steamvr_management(&mut self) {
        self.set_steamvr_management(!self.config.is_steamvr_managed);
        self.save();
    }

    pub fn toggle_tray(&mut self) {
        self.config.allow_tray = !self.config.allow_tray;
        self.save();
    }

    pub fn update_config_value(&mut self, key: &str, value: serde_json::Value) {
        match key {
            "allow_tray" => self.config.allow_tray = value.as_bool().unwrap_or(true),
            "is_steamvr_managed" => {
                self.set_steamvr_management(value.as_bool().unwrap_or(false));
            }
            "tray_notified" => self.config.tray_notified = value.as_bool().unwrap_or(false),
            "branch" => {
                self.set_branch(value.as_str().unwrap_or("main"));
                return;
            }
            _ => {}
        }
        self.save();
    }

    pub fn set_branch(&mut self, branch: &str) {
        if self.update_status.busy() || !matches!(branch, "main" | "beta") {
            return;
        }
        self.config.branch = branch.to_string();
        self.save();
        self.check_updates();
    }

    pub fn steamvr_connectivity_available(&self) -> bool {
        steamvr::connectivity_available()
    }

    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    pub fn save(&self) {
        config::save(&self.config);
    }

    /// Deterministically-ordered live base stations.
    pub fn lighthouses_sorted(&self) -> Vec<LighthouseStation> {
        let mut v: Vec<LighthouseStation> = self.lighthouses.values().cloned().collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    /// Deterministically-ordered groups `(id, group)`.
    pub fn groups_sorted(&self) -> Vec<(String, LighthouseGroup)> {
        let mut v: Vec<(String, LighthouseGroup)> = self
            .config
            .groups
            .iter()
            .map(|(k, g)| (k.clone(), g.clone()))
            .collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state(ble: BluetoothHandle) -> AppState {
        AppState {
            config: Configuration::default(),
            lighthouses: HashMap::new(),
            steamvr_active: false,
            steamvr_state_known: false,
            steamvr_monitor: None,
            scanning: false,
            backend_ready: true,
            platform: Platform {
                system: "windows".into(),
                version: String::new(),
                flags: String::new(),
            },
            update_status: updates::Status::Idle,
            updater: None,
            ble,
        }
    }

    #[test]
    fn discovery_registers_unknown_station_without_overwriting_settings() {
        let mut state = test_state(BluetoothHandle::disconnected());
        let id = "LHB-12345678".to_string();
        state.apply_ble_event(Event::Found { id: id.clone() });

        let station = state.lighthouses.get_mut(&id).expect("new station is visible");
        assert_eq!(station.name, id);
        assert_eq!(station.status, "preloaded");
        assert_eq!(station.channel, -1);
        assert_eq!(station.power_state, -1);
        assert_eq!(station.managed_flags, 6);

        station.name = "Desk".into();
        station.managed_flags = 2;
        state.apply_ble_event(Event::Found { id: id.clone() });
        let station = &state.lighthouses[&id];
        assert_eq!(station.name, "Desk");
        assert_eq!(station.managed_flags, 2);
    }

    fn station(state: &mut AppState, id: &str, flags: i32, power: i32, ready: bool) {
        let mut station = ble::preloaded_station(id.into(), id.into(), 1);
        station.managed_flags = flags;
        station.power_state = power;
        station.status = if ready { "ready" } else { "preloaded" }.into();
        state.lighthouses.insert(id.into(), station);
    }

    fn observe(state: &mut AppState, active: bool) {
        state.apply_steamvr_event(steamvr::Event::State { active, installed: true });
    }

    fn take_power(rx: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<(String, u8)> {
        let mut commands = Vec::new();
        while let Ok(command) = rx.try_recv() {
            match command {
                Command::ChangePower { id, state } => commands.push((id, state)),
                other => panic!("unexpected command: {other:?}"),
            }
        }
        commands.sort();
        commands
    }

    #[test]
    fn steamvr_start_and_exit_respect_individual_flags_and_acknowledgements() {
        let (ble, mut rx) = BluetoothHandle::test_pair();
        let mut state = test_state(ble);
        station(&mut state, "both", 6, 0, true);
        station(&mut state, "wake", 2, 0, true);
        station(&mut state, "sleep", 4, 11, true);
        station(&mut state, "manual", 0, 0, true);
        station(&mut state, "connecting", 6, 0, false);
        observe(&mut state, true);
        assert_eq!(take_power(&mut rx), vec![("both".into(), 1), ("wake".into(), 1)]);
        assert_eq!(state.lighthouses["both"].power_state, 0, "wait for BLE acknowledgement");
        for id in ["both", "wake"] {
            state.apply_ble_event(Event::Power { id: id.into(), power_state: 1 });
        }
        observe(&mut state, false);
        assert_eq!(take_power(&mut rx), vec![("both".into(), 0), ("sleep".into(), 0)]);
    }

    #[test]
    fn steamvr_initial_wait_and_late_connections_follow_current_state() {
        let (ble, mut rx) = BluetoothHandle::test_pair();
        let mut state = test_state(ble);
        station(&mut state, "early", 6, 11, true);
        observe(&mut state, false);
        assert_eq!(take_power(&mut rx), vec![("early".into(), 0)]);
        station(&mut state, "late", 6, 11, true);
        state.reconcile_station_power(&state.lighthouses["late"]);
        assert_eq!(take_power(&mut rx), vec![("late".into(), 0)]);
        observe(&mut state, true);
        take_power(&mut rx);
        station(&mut state, "late-awake", 6, 0, true);
        state.reconcile_station_power(&state.lighthouses["late-awake"]);
        assert_eq!(take_power(&mut rx), vec![("late-awake".into(), 1)]);
    }

    #[test]
    fn steamvr_manual_mode_and_unchanged_observations_do_not_override_power() {
        let (ble, mut rx) = BluetoothHandle::test_pair();
        let mut state = test_state(ble);
        station(&mut state, "station", 6, 0, true);
        state.set_steamvr_management(false);
        observe(&mut state, true);
        assert!(take_power(&mut rx).is_empty());
        state.set_steamvr_management(true);
        assert_eq!(take_power(&mut rx), vec![("station".into(), 1)]);
        observe(&mut state, true);
        assert!(take_power(&mut rx).is_empty());
        state.set_steamvr_management(false);
        observe(&mut state, false);
        assert!(take_power(&mut rx).is_empty());
    }

    #[test]
    fn steamvr_monitor_error_never_means_shutdown_and_recovery_reconciles() {
        let (ble, mut rx) = BluetoothHandle::test_pair();
        let mut state = test_state(ble);
        station(&mut state, "station", 6, 11, true);
        observe(&mut state, true);
        assert!(take_power(&mut rx).is_empty());
        state.apply_steamvr_event(steamvr::Event::Error("snapshot unavailable".into()));
        assert!(!state.steamvr_state_known);
        assert!(state.steamvr_active);
        state.reconcile_steamvr_power();
        assert!(take_power(&mut rx).is_empty());
        observe(&mut state, false);
        assert_eq!(take_power(&mut rx), vec![("station".into(), 0)]);
    }
}
