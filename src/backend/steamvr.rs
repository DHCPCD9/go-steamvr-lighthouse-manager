//! Non-blocking SteamVR process monitoring. Power policy lives in AppState.

#[derive(Clone, Debug)]
pub enum Event {
    State { active: bool, installed: bool },
    Error(String),
}

pub struct Monitor {
    pub events: flume::Receiver<Event>,
    // Dropping the sender wakes and stops the worker.
    _stop: std::sync::mpsc::Sender<()>,
}

pub fn connectivity_available() -> bool {
    cfg!(target_os = "windows")
}

pub fn start() -> Option<Monitor> {
    #[cfg(target_os = "windows")]
    {
        let (event_tx, events) = flume::unbounded();
        let (stop_tx, stop_rx) = std::sync::mpsc::channel();
        let errors = event_tx.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("steamvr".into())
            .spawn(move || watch(event_tx, stop_rx))
        {
            let _ = errors.send(Event::Error(error.to_string()));
        }
        Some(Monitor { events, _stop: stop_tx })
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

#[cfg(target_os = "windows")]
fn watch(events: flume::Sender<Event>, stop: std::sync::mpsc::Receiver<()>) {
    use std::{path::PathBuf, time::Duration};
    let markers: Vec<PathBuf> = [
        ("ProgramFiles(x86)", r"Steam\config\appconfig.json"),
        ("LOCALAPPDATA", r"openvr\openvrpaths.vrpath"),
    ]
    .into_iter()
    .filter_map(|(var, suffix)| std::env::var_os(var).map(|p| PathBuf::from(p).join(suffix)))
    .collect();
    let mut installed = false;
    let mut previous = None;
    let mut failed = false;
    loop {
        match is_steamvr_running() {
            Ok(active) => {
                installed |= active || markers.iter().any(|p| p.is_file());
                let current = (active, installed);
                if previous != Some(current) || failed {
                    if events.send(Event::State { active, installed }).is_err() {
                        break;
                    }
                    previous = Some(current);
                }
                failed = false;
            }
            Err(error) => {
                if !failed && events.send(Event::Error(error.to_string())).is_err() {
                    break;
                }
                failed = true;
            }
        }
        if stop.recv_timeout(Duration::from_secs(1)) != Err(std::sync::mpsc::RecvTimeoutError::Timeout) {
            break;
        }
    }
}

#[cfg(target_os = "windows")]
fn is_steamvr_running() -> windows::core::Result<bool> {
    use windows::Win32::{
        Foundation::{CloseHandle, ERROR_NO_MORE_FILES},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
            PROCESSENTRY32W, TH32CS_SNAPPROCESS,
        },
    };
    // Always close the snapshot, including early matches and enumeration errors.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)? };
    let result = (|| {
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut next = unsafe { Process32FirstW(snapshot, &mut entry) };
        loop {
            match next {
                Ok(()) => {
                    const NAME: &[u8] = b"vrserver.exe";
                    if entry.szExeFile[NAME.len()] == 0
                        && entry.szExeFile[..NAME.len()].iter().zip(NAME).all(|(&a, &b)| {
                            a <= 0x7f && (a as u8).eq_ignore_ascii_case(&b)
                        })
                    {
                        return Ok(true);
                    }
                    next = unsafe { Process32NextW(snapshot, &mut entry) };
                }
                Err(error) if error.code() == ERROR_NO_MORE_FILES.to_hresult() => return Ok(false),
                Err(error) => return Err(error),
            }
        }
    })();
    unsafe { let _ = CloseHandle(snapshot); }
    result
}
