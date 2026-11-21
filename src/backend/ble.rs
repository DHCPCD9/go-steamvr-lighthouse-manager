//! Bluetooth LE backend. Runs a Tokio runtime on a dedicated thread and reports
//! state changes to the UI over a channel.

use crate::models::LighthouseStation;
use super::gatt::Device;
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::task::JoinSet;

/// Commands sent from the UI thread to the BLE thread.
#[derive(Clone, Debug)]
pub enum Command {
    StartScan,
    ChangePower { id: String, state: u8 },
    ChangeChannel { id: String, channel: u8 },
    Identify { id: String },
}

/// Events sent from the BLE thread back to the UI thread.
#[derive(Clone, Debug)]
pub enum Event {
    ScanStarted,
    ScanStopped,
    Found { id: String },
    Ready { id: String, address: String, channel: i32, power_state: i32 },
    Status { id: String, status: String },
    Channel { id: String, channel: i32 },
    Power { id: String, power_state: i32 },
    Error { id: String, message: String },
}

/// Handle to the running BLE thread.
pub struct BluetoothHandle {
    command_tx: tokio::sync::mpsc::UnboundedSender<Command>,
    pub event_rx: flume::Receiver<Event>,
}

impl BluetoothHandle {
    pub fn send(&self, cmd: Command) {
        let _ = self.command_tx.send(cmd);
    }
}

#[cfg(test)]
impl BluetoothHandle {
    pub(crate) fn disconnected() -> Self {
        Self::test_pair().0
    }

    pub(crate) fn test_pair() -> (Self, tokio::sync::mpsc::UnboundedReceiver<Command>) {
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        let (_, event_rx) = flume::unbounded();
        (Self { command_tx, event_rx }, command_rx)
    }
}

/// Start the BLE thread and return its handle.
pub fn start() -> BluetoothHandle {
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (event_tx, event_rx) = flume::unbounded();

    std::thread::Builder::new()
        .name("bluetooth".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("build tokio runtime");
            rt.block_on(run(command_rx, event_tx));
        })
        .expect("spawn bluetooth thread");

    BluetoothHandle {
        command_tx,
        event_rx,
    }
}

async fn run(
    mut command_rx: tokio::sync::mpsc::UnboundedReceiver<Command>,
    event_tx: flume::Sender<Event>,
) {
    let manager = match Manager::new().await {
        Ok(m) => m,
        Err(e) => {
            log::error!("bluetooth manager unavailable: {e}");
            return;
        }
    };
    let central = match pick_adapter(&manager).await {
        Some(c) => c,
        None => {
            log::error!("no bluetooth adapter found");
            return;
        }
    };
    log::info!("bluetooth adapter found");
    let mut events = match central.events().await {
        Ok(events) => events,
        Err(e) => {
            log::error!("failed to subscribe to bluetooth events: {e}");
            return;
        }
    };
    let mut devices = HashMap::new();
    let mut connections = JoinSet::new();
    let mut connecting = HashSet::new();
    let mut attempted = HashSet::new();
    let mut scanning = false;
    let scan_timer = tokio::time::sleep(Duration::from_secs(6));
    tokio::pin!(scan_timer);

    loop {
        tokio::select! {
            command = command_rx.recv() => {
                let Some(command) = command else { break; };
                match command {
                    Command::StartScan => {
                        if scanning { continue; }
                        if let Err(e) = central.start_scan(ScanFilter::default()).await {
                            log::error!("failed to start bluetooth scan: {e}");
                            continue;
                        }
                        attempted.clear();
                        scanning = true;
                        scan_timer.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(6));
                        log::info!("bluetooth scan started");
                        let _ = event_tx.send(Event::ScanStarted);
                    }
                    Command::ChangePower { id, state } => {
                        if let Some(device) = devices.get(&id) {
                            write_power(device, state, &id, &event_tx).await;
                        }
                    }
                    Command::ChangeChannel { id, channel } => {
                        if let Some(device) = devices.get(&id) {
                            write_channel(device, channel, &id, &event_tx).await;
                        }
                    }
                    Command::Identify { id } => {
                        if let Some(device) = devices.get(&id) {
                            write_identify(device, &id, &event_tx).await;
                        }
                    }
                }
            }
            event = events.next() => {
                let Some(event) = event else { break; };
                if !scanning { continue; }
                if let CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) = event {
                    if attempted.contains(&id) || connecting.contains(&id) { continue; }
                    let Ok(peripheral) = central.peripheral(&id).await else { continue; };
                    let Ok(Some(props)) = peripheral.properties().await else { continue; };
                    let Some(name) = props.local_name else { continue; };
                    if !name.starts_with("LHB-") || devices.contains_key(&name) { continue; }
                    attempted.insert(id.clone());
                    connecting.insert(id.clone());
                    let _ = event_tx.send(Event::Found { id: name.clone() });
                    let event_tx = event_tx.clone();
                    connections.spawn(async move {
                        let device = initialize_device(peripheral, &name, &event_tx).await;
                        (id, name, device)
                    });
                }
            }
            result = connections.join_next(), if !connections.is_empty() => {
                match result {
                    Some(Ok((id, name, device))) => {
                        connecting.remove(&id);
                        if let Some((device, channel, power)) = device {
                            let address = device.address().to_string();
                            devices.insert(name.clone(), device);
                            let _ = event_tx.send(Event::Ready {
                                id: name, address, channel, power_state: power,
                            });
                        }
                    }
                    Some(Err(e)) => log::error!("connection task failed: {e}"),
                    None => {}
                }
            }
            _ = &mut scan_timer, if scanning => {
                let _ = central.stop_scan().await;
                scanning = false;
                log::info!("bluetooth scan stopped");
                let _ = event_tx.send(Event::ScanStopped);
            }
        }
    }
    if scanning {
        let _ = central.stop_scan().await;
    }
}

async fn pick_adapter(manager: &Manager) -> Option<Adapter> {
    manager.adapters().await.ok().and_then(|mut a| a.pop())
}

/// Connect and read initial state without blocking the BLE command loop.
async fn initialize_device(
    peripheral: Peripheral,
    name: &str,
    event_tx: &flume::Sender<Event>,
) -> Option<(Device, i32, i32)> {
    let started = Instant::now();
    log::info!("{name}: initializing lighthouse connection");
    let device = match Device::connect(peripheral).await {
        Ok(device) => device,
        Err(e) => {
            log::warn!("{name}: failed to initialize connection: {e:#}");
            let _ = event_tx.send(Event::Error { id: name.into(), message: e.to_string() });
            return None;
        }
    };
    let (channel, power) = tokio::join!(device.read_channel(), device.read_power());
    let channel = channel.map(i32::from).unwrap_or(-1);
    let power = power.map(i32::from).unwrap_or(-1);
    log::info!("{name}: services discovered in {} ms; channel={channel}, power_state={power}",
        started.elapsed().as_millis());
    Some((device, channel, power))
}

async fn write_power(device: &Device, state: u8, id: &str, event_tx: &flume::Sender<Event>) {
    if device.write_power(state).await.is_ok() {
        let _ = event_tx.send(Event::Power {
            id: id.to_string(),
            power_state: state as i32,
        });
    }
}

async fn write_channel(device: &Device, channel: u8, id: &str, event_tx: &flume::Sender<Event>) {
    if device.write_channel(channel).await.is_ok() {
        let _ = event_tx.send(Event::Channel {
            id: id.to_string(),
            channel: channel as i32,
        });
    }
}

async fn write_identify(device: &Device, id: &str, event_tx: &flume::Sender<Event>) {
    if device.identify().await.is_ok() {
        let _ = event_tx.send(Event::Status {
            id: id.to_string(),
            status: "ready".into(),
        });
    }
}

/// Build a station placeholder before its Bluetooth connection is ready.
pub fn preloaded_station(id: String, nickname: String, channel: i32) -> LighthouseStation {
    LighthouseStation {
        id,
        power_state: -1,
        status: "preloaded".into(),
        channel,
        name: nickname,
        managed_flags: 6,
        previous_status: String::new(),
        previous_power_state: -1,
    }
}
