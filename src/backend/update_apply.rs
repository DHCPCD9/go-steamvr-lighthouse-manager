//! Same-user, same-volume Windows update handoff. No shell or elevation is involved.

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    ffi::{OsStr, OsString},
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    mem::size_of,
    os::windows::{ffi::{OsStrExt, OsStringExt}, fs::{MetadataExt, OpenOptionsExt}, io::AsRawHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::OnceLock,
};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{CloseHandle, GetLastError, LocalFree, FILETIME, HANDLE, HLOCAL, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
        Security::{
            Authorization::{ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
            GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
        },
        Storage::FileSystem::{
            CreateDirectoryW, GetFileInformationByHandle, MoveFileExW, BY_HANDLE_FILE_INFORMATION,
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_SHARE_READ, MOVEFILE_WRITE_THROUGH,
        },
        System::Threading::{
            CreateEventW, GetCurrentProcess, GetCurrentProcessId, GetProcessTimes, OpenEventW,
            OpenProcess, OpenProcessToken, QueryFullProcessImageNameW, SetEvent,
            WaitForMultipleObjects, WaitForSingleObject, EVENT_MODIFY_STATE,
            PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
            SYNCHRONIZATION_SYNCHRONIZE,
        },
    },
};

const APPLY_ARG: &str = "--slm-apply-update";
const CLEANUP_ARG: &str = "--slm-update-cleanup";
const ERROR_ARG: &str = "--slm-update-error";
const STAGE_PREFIX: &str = ".slm-update-";
const READY_MS: u32 = 45_000;
const PARENT_EXIT_MS: u32 = 60_000;
const MAX_PAYLOAD: u64 = 512 * 1024 * 1024;
const MAX_JOB: u64 = 128 * 1024;
const FIXED_FILES: &[&str] = &["payload.exe", "helper.exe", "backup.exe", "job.json", "failure.txt"];
static STARTUP_ERROR: OnceLock<String> = OnceLock::new();

/// An unarmed stage is removed on every error path, without recursive deletion.
pub struct Staging {
    directory: PathBuf,
    target: PathBuf,
    payload: PathBuf,
    nonce: String,
    armed: bool,
}

impl Staging {
    pub fn payload_path(&self) -> &Path { &self.payload }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if !self.armed { cleanup_fixed(&self.directory); }
    }
}

/// Create a private sibling, so both replacement renames stay on the same volume.
pub fn prepare() -> Result<Staging> {
    let target = fs::canonicalize(std::env::current_exe()?).context("Resolve the running executable")?;
    ensure_regular(&target)?;
    let parent = target.parent().context("The executable has no install directory")?;
    let security = PrivateSecurity::new()?;
    let nonce = uuid::Uuid::new_v4().to_string();
    let directory = parent.join(format!("{STAGE_PREFIX}{nonce}"));
    let name = wide(directory.as_os_str())?;
    unsafe { CreateDirectoryW(PCWSTR(name.as_ptr()), Some(&security.attributes())) }
        .with_context(|| format!("Cannot update in {}. Move the portable app to a directory writable by your Windows account (for example under LocalAppData), then retry. Administrator permission will not be requested.", parent.display()))?;
    let staging = Staging {
        payload: directory.join("payload.exe"), directory, target, nonce, armed: false,
    };
    // Probe with create_new, then leave the download path absent for create_new.
    let probe = OpenOptions::new().write(true).create_new(true).open(&staging.payload)
        .context("The update directory cannot accept files; move the app to a user-writable directory")?;
    drop(probe);
    fs::remove_file(&staging.payload).context("Remove the staging write-permission probe")?;
    Ok(staging)
}

/// Return only after the helper has locked the payload and validated the live parent.
pub fn handoff(mut staging: Staging, expected_sha256: [u8; 32]) -> Result<()> {
    let source = open_locked(&staging.target)?;
    let original = file_identity(&source)?;
    let job = Job {
        schema: 1,
        nonce: staging.nonce.clone(),
        target: Native::from(staging.target.as_os_str()),
        working_dir: Native::from(std::env::current_dir()?.as_os_str()),
        arguments: launch_arguments()?,
        parent_pid: unsafe { GetCurrentProcessId() },
        parent_created: process_created(unsafe { GetCurrentProcess() })?,
        original,
        expected_sha256,
    };
    let helper_path = staging.directory.join("helper.exe");
    // Copy from the same locked file whose identity the helper will check.
    let mut source = source;
    let mut helper_file = OpenOptions::new().write(true).create_new(true).open(&helper_path)?;
    std::io::copy(&mut source, &mut helper_file).context("Create the current-user update helper")?;
    helper_file.sync_all()?;
    drop(helper_file);
    let job_path = staging.directory.join("job.json");
    let mut job_file = OpenOptions::new().write(true).create_new(true).open(&job_path)?;
    serde_json::to_writer(&mut job_file, &job)?;
    job_file.sync_all()?;
    drop(job_file);
    let security = PrivateSecurity::new()?;
    let ready = Event::create(&job.nonce, "ready", &security)?;
    let arm = Event::create(&job.nonce, "arm", &security)?;
    let cancel = Event::create(&job.nonce, "cancel", &security)?;
    let mut child = Command::new(&helper_path).arg(APPLY_ARG).arg(&job_path)
        .current_dir(staging.target.parent().unwrap())
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW, not an elevation verb.
        .spawn().context("Start the update helper under the current Windows token")?;
    let outcome = (|| -> Result<()> {
        let process = HANDLE(child.as_raw_handle());
        let result = unsafe { WaitForMultipleObjects(&[process, ready.0.0], false, READY_MS) };
        if result == WAIT_OBJECT_0 {
            let detail = read_failure(&staging.directory).unwrap_or_else(|| "The update helper exited before becoming ready".into());
            bail!("{detail}");
        }
        ensure!(result.0 == WAIT_OBJECT_0.0 + 1,
            "The update helper did not become ready within 45 seconds ({})", wait_detail(result));
        let helper_pid = child.id();
        let helper_created = process_created(process)?;
        let cleanup_path = job_path.clone();
        // If the UI fails to exit, this original process is still available to
        // remove the timed-out helper's fixed files. Otherwise the new app does it.
        std::thread::Builder::new().name("update-parent-cleanup".into()).spawn(move || {
            if let Err(error) = cleanup_after_helper(&cleanup_path, helper_pid, helper_created, READY_MS + PARENT_EXIT_MS + 10_000) {
                persistent_log(&format!("Parent cleanup: {error:#}"));
            }
        }).context("Monitor the update helper for cleanup")?;
        // Nothing fallible follows the arm signal. On any earlier failure the helper
        // never touches the target, even if the original process exits unexpectedly.
        unsafe { SetEvent(arm.0.0) }.context("Authorize the validated update helper")?;
        Ok(())
    })();
    if outcome.is_err() {
        let _ = unsafe { SetEvent(cancel.0.0) };
        stop_unarmed_helper(&mut child);
    } else {
        staging.armed = true;
    }
    outcome
}

/// Called before GPUI/logging. Cleanup waits happen on a detached background thread.
pub fn handle_startup() -> Option<Result<()>> {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    if arguments.first().is_some_and(|arg| arg == APPLY_ARG) {
        let result = if arguments.len() == 2 {
            run_helper(Path::new(&arguments[1]))
        } else { Err(anyhow!("Invalid update-helper arguments")) };
        if let Err(error) = &result { persistent_log(&format!("Helper: {error:#}")); }
        return Some(result);
    }
    if let Some(index) = arguments.iter().position(|arg| arg == ERROR_ARG) {
        if let Some(message) = arguments.get(index + 1) {
            let _ = STARTUP_ERROR.set(message.to_string_lossy().chars().take(8192).collect());
        }
    }
    if let Some(index) = arguments.iter().position(|arg| arg == CLEANUP_ARG) {
        let cleanup = (|| -> Result<_> {
            let path = PathBuf::from(arguments.get(index + 1).context("Missing cleanup job")?);
            let pid = arguments.get(index + 2).context("Missing cleanup PID")?.to_str().context("Invalid cleanup PID")?.parse::<u32>()?;
            let created = arguments.get(index + 3).context("Missing cleanup process identity")?.to_str().context("Invalid cleanup identity")?.parse::<u64>()?;
            Ok((path, pid, created))
        })();
        match cleanup {
            Ok((path, pid, created)) => {
                let _ = std::thread::Builder::new().name("update-cleanup".into()).spawn(move || {
                    if let Err(error) = cleanup_after_helper(&path, pid, created, 30_000) {
                        persistent_log(&format!("Cleanup: {error:#}"));
                    }
                });
            }
            Err(error) => persistent_log(&format!("Invalid cleanup arguments: {error:#}")),
        }
    }
    None
}

pub fn startup_error() -> Option<String> { STARTUP_ERROR.get().cloned() }

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Native(Vec<u16>);
impl Native {
    fn from(value: &OsStr) -> Self { Self(value.encode_wide().collect()) }
    fn os(&self) -> Result<OsString> {
        ensure!(self.0.len() <= 32767 && !self.0.contains(&0), "Invalid native update-job string");
        Ok(OsString::from_wide(&self.0))
    }
    fn path(&self) -> Result<PathBuf> { Ok(PathBuf::from(self.os()?)) }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Job {
    schema: u32,
    nonce: String,
    target: Native,
    working_dir: Native,
    arguments: Vec<Native>,
    parent_pid: u32,
    parent_created: u64,
    original: Identity,
    expected_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Identity { volume: u32, index: u64, size: u64, modified: u64 }

struct Layout { directory: PathBuf, target: PathBuf, helper: PathBuf, payload: PathBuf, backup: PathBuf }

fn read_job(path: &Path) -> Result<(Job, Layout)> {
    ensure!(path.is_absolute() && path.file_name() == Some(OsStr::new("job.json")), "Invalid update-job path");
    ensure_regular(path)?;
    let file = open_locked(path)?;
    ensure!(file.metadata()?.len() <= MAX_JOB, "Update job exceeds its size limit");
    let job: Job = serde_json::from_reader(file.take(MAX_JOB + 1)).context("Read the update job")?;
    ensure!(job.schema == 1, "Unsupported update-job format");
    let nonce = uuid::Uuid::parse_str(&job.nonce).context("Invalid update nonce")?;
    ensure!(nonce.to_string() == job.nonce, "Noncanonical update nonce");
    let target = job.target.path()?;
    ensure!(target.is_absolute() && target.file_name().is_some(), "Invalid update target");
    let parent = target.parent().context("Missing update target directory")?;
    ensure!(fs::canonicalize(parent)? == parent, "The install directory changed since staging");
    let directory = parent.join(format!("{STAGE_PREFIX}{}", job.nonce));
    ensure!(path.parent() == Some(directory.as_path()), "Update job is not a sibling of its target");
    ensure!(fs::canonicalize(&directory)? == directory, "The staging directory is a reparse point");
    ensure!(!is_reparse(&fs::symlink_metadata(&directory)?), "The staging directory is a reparse point");
    ensure!(job.arguments.len() <= 256, "Too many original launch arguments");
    let mut argument_size = 0;
    for argument in &job.arguments {
        argument.os()?;
        argument_size += argument.0.len() + 1;
    }
    ensure!(argument_size <= 24000, "Original launch arguments are too long to restart safely");
    ensure!(job.working_dir.path()?.is_absolute(), "The original working directory is not absolute");
    let layout = Layout {
        helper: directory.join("helper.exe"), payload: directory.join("payload.exe"),
        backup: directory.join("backup.exe"), directory, target,
    };
    Ok((job, layout))
}

fn run_helper(path: &Path) -> Result<()> {
    let (job, layout) = read_job(path)?;
    let result = helper_inner(&job, &layout, path);
    if let Err(error) = &result {
        let message = format!("Update failed: {error:#}");
        let _ = fs::write(layout.directory.join("failure.txt"), &message);
        persistent_log(&message);
    }
    result
}

fn helper_inner(job: &Job, layout: &Layout, path: &Path) -> Result<()> {
    ensure!(fs::canonicalize(std::env::current_exe()?)? == layout.helper,
        "The update helper is not running from the expected private staging directory");
    ensure_regular(&layout.helper)?;
    let parent = OwnedHandle(unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, false, job.parent_pid)
    }.context("Open the original process without elevation")?);
    ensure!(process_created(parent.0)? == job.parent_created, "The original process PID was reused");
    ensure!(process_image(parent.0)? == layout.target, "The original process image does not match the update target");
    ensure!(unsafe { WaitForSingleObject(parent.0, 0) } == WAIT_TIMEOUT, "The original process has already exited");
    let original = open_locked(&layout.target)?;
    ensure!(file_identity(&original)? == job.original, "The original executable changed since staging");
    ensure!(!layout.backup.try_exists()?, "The update backup path is already occupied");
    // Keeping this handle open denies all writes/deletion until the last validation.
    let mut payload = open_locked(&layout.payload)?;
    verify_payload(&mut payload, &job.expected_sha256)?;
    let ready = Event::open(&job.nonce, "ready")?;
    let arm = Event::open(&job.nonce, "arm")?;
    let cancel = Event::open(&job.nonce, "cancel")?;
    unsafe { SetEvent(ready.0.0) }.context("Signal helper readiness")?;
    let armed = unsafe { WaitForMultipleObjects(&[cancel.0.0, arm.0.0, parent.0], false, READY_MS) };
    ensure!(armed.0 == WAIT_OBJECT_0.0 + 1, "The update handoff was cancelled or was not authorized ({})", wait_detail(armed));
    let exited = unsafe { WaitForMultipleObjects(&[cancel.0.0, parent.0], false, PARENT_EXIT_MS) };
    ensure!(exited.0 == WAIT_OBJECT_0.0 + 1,
        "The original app did not exit within 60 seconds; its executable was left untouched ({})", wait_detail(exited));
    let revalidation = (|| -> Result<()> {
        verify_payload(&mut payload, &job.expected_sha256).context("Revalidate the payload immediately before replacement")?;
        ensure!(file_identity(&original)? == job.original, "The original executable identity changed");
        Ok(())
    })();
    drop(payload);
    drop(original);
    if let Err(error) = revalidation { return recover(job, layout, path, error, false, false); }
    // MOVEFILE_COPY_ALLOWED and MOVEFILE_REPLACE_EXISTING are deliberately absent.
    // A newly occupied destination must fail, never overwrite an unrelated file.
    if let Err(error) = rename_exact(&layout.target, &layout.backup) {
        return recover(job, layout, path, error.context("Back up the running executable"), false, false);
    }
    if let Err(error) = rename_exact(&layout.payload, &layout.target) {
        return recover(job, layout, path, error.context("Install the staged executable"), true, false);
    }
    match restart(job, layout, path, &layout.target, None, true, false) {
        Ok(()) => Ok(()),
        Err(error) => recover(job, layout, path, error.context("Restart the updated app"), true, true),
    }
}

fn recover(job: &Job, layout: &Layout, path: &Path, cause: anyhow::Error, backed_up: bool, installed: bool) -> Result<()> {
    let mut message = format!("Update failed: {cause:#}");
    let rollback = (|| -> Result<()> {
        if installed {
            let mut replacement = open_locked(&layout.target)?;
            verify_payload(&mut replacement, &job.expected_sha256)
                .context("Refuse to move an unrelated executable during rollback")?;
            drop(replacement);
            rename_exact(&layout.target, &layout.payload).context("Move the failed replacement out of the way")?;
        }
        if backed_up {
            let backup = open_locked(&layout.backup)?;
            ensure!(file_identity(&backup)? == job.original, "Refuse to restore an unrelated backup");
            drop(backup);
            rename_exact(&layout.backup, &layout.target).context("Restore the original executable")?;
        }
        Ok(())
    })();
    let restored = rollback.is_ok();
    if let Err(error) = rollback { message.push_str(&format!("\nRollback failed: {error:#}. Original backup: {}", layout.backup.display())); }
    persistent_log(&message);
    let _ = fs::write(layout.directory.join("failure.txt"), &message);
    let executable = if restored { &layout.target } else { &layout.backup };
    // On failed rollback retain the entire stage, especially the recovery executable.
    if let Err(error) = restart(job, layout, path, executable, Some(&message), restored, true) {
        message.push_str(&format!("\nCould not restart the old app: {error:#}. Start {} manually.", executable.display()));
        persistent_log(&message);
        let _ = fs::write(layout.directory.join("failure.txt"), &message);
    }
    Err(anyhow!(message))
}

fn restart(job: &Job, layout: &Layout, path: &Path, executable: &Path, error: Option<&str>, cleanup: bool, allow_cwd_fallback: bool) -> Result<()> {
    if error.is_some() {
        let original = open_locked(executable)?;
        ensure!(file_identity(&original)? == job.original, "Refuse to relaunch an unrelated recovery executable");
    }
    let spawn = |cwd: &Path| -> Result<()> {
        let mut command = Command::new(executable);
        for argument in &job.arguments { command.arg(argument.os()?); }
        if let Some(error) = error { command.arg(ERROR_ARG).arg(error.chars().take(8192).collect::<String>()); }
        if cleanup {
            command.arg(CLEANUP_ARG).arg(path)
                .arg(unsafe { GetCurrentProcessId() }.to_string())
                .arg(process_created(unsafe { GetCurrentProcess() })?.to_string());
        }
        command.current_dir(cwd).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        command.spawn().with_context(|| format!("Launch {} as the current Windows user", executable.display()))?;
        Ok(())
    };
    let cwd = job.working_dir.path()?;
    match spawn(&cwd) {
        Ok(()) => Ok(()),
        Err(error) if allow_cwd_fallback && !cwd.is_dir() => {
            persistent_log(&format!("Original working directory is unavailable; restarting the restored app in its install directory: {error:#}"));
            spawn(layout.target.parent().unwrap())
        }
        Err(error) => Err(error),
    }
}

fn cleanup_after_helper(path: &Path, pid: u32, created: u64, wait_ms: u32) -> Result<()> {
    let (_, layout) = read_job(path)?;
    ensure!(fs::canonicalize(std::env::current_exe()?)? == layout.target, "Cleanup target is not the running app");
    match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, false, pid) } {
        Ok(handle) => {
            let helper = OwnedHandle(handle);
            if process_created(helper.0)? == created {
                // PID + creation time identifies the helper even after exit;
                // Windows can no longer provide its image path at that point.
                ensure!(unsafe { WaitForSingleObject(helper.0, wait_ms) } == WAIT_OBJECT_0, "The update helper did not exit before cleanup");
            } // A reused PID means the actual helper has already exited.
        }
        Err(error) if error.code().0 as u32 == 0x80070057 => {} // ERROR_INVALID_PARAMETER: exited PID.
        Err(error) => return Err(error).context("Open the helper for fixed-file cleanup"),
    }
    cleanup_fixed(&layout.directory);
    Ok(())
}

fn cleanup_fixed(directory: &Path) {
    // Never follow directory reparses or recursively delete directory contents.
    if fs::symlink_metadata(directory).map_or(true, |metadata| !metadata.is_dir() || is_reparse(&metadata)) { return; }
    for name in FIXED_FILES { let _ = fs::remove_file(directory.join(name)); }
    let _ = fs::remove_dir(directory);
}

fn launch_arguments() -> Result<Vec<Native>> {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let mut result = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == CLEANUP_ARG { index += 4; }
        else if arguments[index] == ERROR_ARG { index += 2; }
        else {
            ensure!(arguments[index] != APPLY_ARG, "An update helper cannot install another update");
            result.push(Native::from(&arguments[index]));
            index += 1;
        }
    }
    ensure!(result.len() <= 256 && result.iter().map(|arg| arg.0.len() + 1).sum::<usize>() <= 24000,
        "The current launch arguments are too long for a safe update restart");
    Ok(result)
}

fn ensure_regular(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).with_context(|| format!("Inspect {}", path.display()))?;
    ensure!(metadata.is_file() && !is_reparse(&metadata), "{} is not a regular, non-reparse file", path.display());
    Ok(())
}
fn is_reparse(metadata: &fs::Metadata) -> bool { metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 }
fn open_locked(path: &Path) -> Result<File> {
    ensure_regular(path)?;
    OpenOptions::new().read(true).share_mode(FILE_SHARE_READ.0).open(path)
        .with_context(|| format!("Lock {} against modification", path.display()))
}
fn file_identity(file: &File) -> Result<Identity> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }?;
    Ok(Identity { volume: info.dwVolumeSerialNumber, index: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        size: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow), modified: filetime(info.ftLastWriteTime) })
}
fn filetime(time: FILETIME) -> u64 { (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime) }
fn process_created(handle: HANDLE) -> Result<u64> {
    let (mut created, mut exit, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    unsafe { GetProcessTimes(handle, &mut created, &mut exit, &mut kernel, &mut user) }?;
    Ok(filetime(created))
}
fn process_image(handle: HANDLE) -> Result<PathBuf> {
    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    unsafe { QueryFullProcessImageNameW(handle, PROCESS_NAME_FORMAT(0), PWSTR(buffer.as_mut_ptr()), &mut length) }
        .context("Read the original process image path")?;
    fs::canonicalize(PathBuf::from(OsString::from_wide(&buffer[..length as usize])))
        .context("Resolve the original process image path")
}
fn rename_exact(from: &Path, to: &Path) -> Result<()> {
    let from_w = wide(from.as_os_str())?;
    let to_w = wide(to.as_os_str())?;
    unsafe { MoveFileExW(PCWSTR(from_w.as_ptr()), PCWSTR(to_w.as_ptr()), MOVEFILE_WRITE_THROUGH) }
        .with_context(|| format!("Rename {} to {} without replacing an occupied destination", from.display(), to.display()))
}
fn wide(value: &OsStr) -> Result<Vec<u16>> {
    let mut value: Vec<u16> = value.encode_wide().collect();
    ensure!(!value.contains(&0) && value.len() <= 32767, "Invalid Windows path or object name");
    value.push(0);
    Ok(value)
}

fn verify_payload(file: &mut File, expected: &[u8; 32]) -> Result<()> {
    let size = file.metadata()?.len();
    ensure!((512..=MAX_PAYLOAD).contains(&size), "The update payload size is invalid");
    file.seek(SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut received = 0u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 { break; }
        received += count as u64;
        ensure!(received <= MAX_PAYLOAD, "The update payload exceeds its size limit");
        hasher.update(&buffer[..count]);
    }
    let actual: [u8; 32] = hasher.finalize().into();
    ensure!(received == size && &actual == expected, "The update payload SHA-256 does not match the release checksum");
    verify_pe(file, size)
}

fn verify_pe(reader: &mut (impl Read + Seek), size: u64) -> Result<()> {
    let mut dos = [0u8; 64];
    reader.seek(SeekFrom::Start(0))?;
    reader.read_exact(&mut dos).context("Read the DOS executable header")?;
    ensure!(&dos[..2] == b"MZ", "The update is not a Windows executable");
    let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap()) as u64;
    ensure!(offset >= 64 && offset.checked_add(24).is_some_and(|end| end <= size), "Invalid PE header offset");
    reader.seek(SeekFrom::Start(offset))?;
    let mut header = [0u8; 24];
    reader.read_exact(&mut header)?;
    ensure!(&header[..4] == b"PE\0\0" && u16_at(&header, 4) == 0x8664, "The update must be an x86_64 Windows PE executable");
    let sections = u16_at(&header, 6) as u64;
    let optional_size = u16_at(&header, 20) as u64;
    let flags = u16_at(&header, 22);
    ensure!(sections > 0 && sections <= 96 && flags & 0x0002 != 0 && flags & 0x2000 == 0, "Invalid executable PE characteristics");
    let table_end = offset + 24 + optional_size + sections * 40;
    ensure!(optional_size >= 112 && optional_size <= 4096 && table_end <= size, "Truncated PE optional header or section table");
    let mut optional = [0u8; 112];
    reader.read_exact(&mut optional)?;
    ensure!(u16_at(&optional, 0) == 0x20b && matches!(u16_at(&optional, 68), 2 | 3), "The update must be a native 64-bit Windows application");
    let entry = u32_at(&optional, 16) as u64;
    let image_size = u32_at(&optional, 56) as u64;
    let headers_size = u32_at(&optional, 60) as u64;
    ensure!(entry != 0 && entry < image_size && headers_size >= table_end && headers_size <= size, "Invalid executable image bounds");
    reader.seek(SeekFrom::Start(offset + 24 + optional_size))?;
    let mut executable_entry = false;
    for _ in 0..sections {
        let mut section = [0u8; 40];
        reader.read_exact(&mut section)?;
        let virtual_size = u32_at(&section, 8) as u64;
        let virtual_address = u32_at(&section, 12) as u64;
        let raw_size = u32_at(&section, 16) as u64;
        let raw_offset = u32_at(&section, 20) as u64;
        ensure!(raw_size == 0 || (raw_offset >= headers_size && raw_offset + raw_size <= size), "A PE section lies outside the payload");
        ensure!(virtual_address + virtual_size.max(raw_size) <= image_size, "A PE section lies outside its image");
        if u32_at(&section, 36) & 0x20000000 != 0 && entry >= virtual_address && entry < virtual_address + raw_size {
            executable_entry = true;
        }
    }
    ensure!(executable_entry, "The PE entry point is not backed by an executable section");
    Ok(())
}
fn u16_at(bytes: &[u8], offset: usize) -> u16 { u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap()) }
fn u32_at(bytes: &[u8], offset: usize) -> u32 { u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) }

struct OwnedHandle(HANDLE);
impl Drop for OwnedHandle { fn drop(&mut self) { let _ = unsafe { CloseHandle(self.0) }; } }
struct Event(OwnedHandle);
impl Event {
    fn name(nonce: &str, role: &str) -> Result<Vec<u16>> { wide(OsStr::new(&format!("Local\\SLMUpdate-{nonce}-{role}"))) }
    fn create(nonce: &str, role: &str, security: &PrivateSecurity) -> Result<Self> {
        let name = Self::name(nonce, role)?;
        let handle = OwnedHandle(unsafe { CreateEventW(Some(&security.attributes()), true, false, PCWSTR(name.as_ptr())) }?);
        // Nonces are unguessable, but a preexisting object must still fail closed.
        ensure!(unsafe { GetLastError() }.0 != 183, "The private update event already exists");
        Ok(Self(handle))
    }
    fn open(nonce: &str, role: &str) -> Result<Self> {
        let name = Self::name(nonce, role)?;
        Ok(Self(OwnedHandle(unsafe { OpenEventW(EVENT_MODIFY_STATE | SYNCHRONIZATION_SYNCHRONIZE, false, PCWSTR(name.as_ptr())) }?)))
    }
}

struct PrivateSecurity(PSECURITY_DESCRIPTOR);
impl PrivateSecurity {
    fn new() -> Result<Self> {
        let mut token = HANDLE::default();
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }?;
        let token = OwnedHandle(token);
        let mut needed = 0u32;
        let _ = unsafe { GetTokenInformation(token.0, TokenUser, None, 0, &mut needed) };
        ensure!(needed as usize >= size_of::<TOKEN_USER>() && needed <= 65536, "Cannot determine the current Windows user SID");
        // u64 storage provides alignment for TOKEN_USER and its in-buffer SID.
        let mut storage = vec![0u64; (needed as usize).div_ceil(8)];
        unsafe { GetTokenInformation(token.0, TokenUser, Some(storage.as_mut_ptr().cast()), needed, &mut needed) }?;
        let user = unsafe { &*storage.as_ptr().cast::<TOKEN_USER>() };
        let mut sid = PWSTR::null();
        unsafe { ConvertSidToStringSidW(user.User.Sid, &mut sid) }?;
        let sid_string = unsafe { sid.to_string() };
        unsafe { LocalFree(Some(HLOCAL(sid.0.cast()))) };
        let sddl = wide(OsStr::new(&format!("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;{})", sid_string?)))?;
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe { ConvertStringSecurityDescriptorToSecurityDescriptorW(PCWSTR(sddl.as_ptr()), SDDL_REVISION_1, &mut descriptor, None) }?;
        Ok(Self(descriptor))
    }
    fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES { nLength: size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: self.0.0, bInheritHandle: false.into() }
    }
}
impl Drop for PrivateSecurity { fn drop(&mut self) { unsafe { LocalFree(Some(HLOCAL(self.0.0))) }; } }

fn stop_unarmed_helper(child: &mut Child) {
    let handle = HANDLE(child.as_raw_handle());
    if unsafe { WaitForSingleObject(handle, 2000) } == WAIT_TIMEOUT {
        let _ = child.kill();
        let _ = unsafe { WaitForSingleObject(handle, 2000) };
    }
}
fn wait_detail(result: windows::Win32::Foundation::WAIT_EVENT) -> String {
    if result == WAIT_FAILED { format!("Windows error {}", unsafe { GetLastError() }.0) }
    else if result == WAIT_TIMEOUT { "timeout".into() }
    else { format!("wait result {}", result.0) }
}
fn read_failure(directory: &Path) -> Option<String> {
    let mut text = String::new();
    File::open(directory.join("failure.txt")).ok()?.take(16384).read_to_string(&mut text).ok()?;
    Some(text)
}
fn persistent_log(message: &str) {
    // A stable per-user diagnostic survives cleanup of the nonce directory.
    if let Some(directory) = dirs::data_local_dir().map(|root| root.join("steamvr-lighthouse-manager")) {
        let write = || -> std::io::Result<()> {
            fs::create_dir_all(&directory)?;
            let path = directory.join("update.log");
            let mut file = OpenOptions::new().create(true).append(true).open(path)?;
            writeln!(file, "{:?}: {message}", std::time::SystemTime::now())?;
            file.sync_data()
        };
        let _ = write();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn pe() -> Vec<u8> {
        let mut bytes = vec![0u8; 1024];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&64u32.to_le_bytes());
        bytes[64..68].copy_from_slice(b"PE\0\0");
        bytes[68..70].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[70..72].copy_from_slice(&1u16.to_le_bytes());
        bytes[84..86].copy_from_slice(&240u16.to_le_bytes());
        bytes[86..88].copy_from_slice(&2u16.to_le_bytes());
        bytes[88..90].copy_from_slice(&0x20bu16.to_le_bytes());
        bytes[104..108].copy_from_slice(&4096u32.to_le_bytes());
        bytes[144..148].copy_from_slice(&8192u32.to_le_bytes());
        bytes[148..152].copy_from_slice(&512u32.to_le_bytes());
        bytes[156..158].copy_from_slice(&2u16.to_le_bytes());
        bytes[336..340].copy_from_slice(&512u32.to_le_bytes());
        bytes[340..344].copy_from_slice(&4096u32.to_le_bytes());
        bytes[344..348].copy_from_slice(&512u32.to_le_bytes());
        bytes[348..352].copy_from_slice(&512u32.to_le_bytes());
        bytes[364..368].copy_from_slice(&0x60000020u32.to_le_bytes());
        bytes
    }
    #[test]
    fn accepts_native_x64_pe_only() {
        let bytes = pe();
        assert!(verify_pe(&mut Cursor::new(&bytes), bytes.len() as u64).is_ok());
        for (offset, data) in [(68, vec![0x4c, 1]), (88, vec![0x0b, 1]), (86, vec![2, 0x20]), (156, vec![1, 0]), (64, b"NE\0\0".to_vec())] {
            let mut invalid = bytes.clone();
            invalid[offset..offset + data.len()].copy_from_slice(&data);
            assert!(verify_pe(&mut Cursor::new(&invalid), invalid.len() as u64).is_err(), "offset {offset}");
        }
    }
    #[test]
    fn rejects_truncation_and_unbacked_entry_point() {
        let bytes = pe();
        for size in [0, 63, 80, 180, 367, 1023] {
            assert!(verify_pe(&mut Cursor::new(&bytes[..size]), size as u64).is_err());
        }
        let mut invalid = bytes;
        invalid[104..108].copy_from_slice(&6000u32.to_le_bytes());
        assert!(verify_pe(&mut Cursor::new(&invalid), invalid.len() as u64).is_err());
    }
    #[test]
    fn checksum_and_pe_are_both_required() {
        let path = std::env::temp_dir().join(format!("slm-hash-test-{}.exe", uuid::Uuid::new_v4()));
        let bytes = pe();
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        fs::write(&path, &bytes).unwrap();
        let mut file = File::open(&path).unwrap();
        assert!(verify_payload(&mut file, &digest).is_ok());
        let mut wrong = digest;
        wrong[0] ^= 1;
        assert!(verify_payload(&mut file, &wrong).is_err());
        drop(file);
        let mut invalid = bytes;
        invalid[68..70].copy_from_slice(&0x14cu16.to_le_bytes());
        fs::write(&path, &invalid).unwrap();
        let invalid_digest: [u8; 32] = Sha256::digest(&invalid).into();
        let mut file = File::open(&path).unwrap();
        assert!(verify_payload(&mut file, &invalid_digest).is_err());
        drop(file);
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn fixed_cleanup_preserves_unknown_files() {
        let root = std::env::temp_dir().join(format!("slm-cleanup-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("payload.exe"), b"test").unwrap();
        fs::write(root.join("unrelated.txt"), b"keep").unwrap();
        cleanup_fixed(&root);
        assert!(!root.join("payload.exe").exists());
        assert_eq!(fs::read(root.join("unrelated.txt")).unwrap(), b"keep");
        fs::remove_file(root.join("unrelated.txt")).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn native_strings_roundtrip_and_reject_nul() {
        let text = OsStr::new("C:\\Users\\更新\\application.exe");
        assert_eq!(Native::from(text).os().unwrap(), text);
        assert!(Native(vec![65, 0, 66]).os().is_err());
    }
}
