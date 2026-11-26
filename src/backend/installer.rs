//! Opt-in, per-user installation. Configuration stays in the existing config directory.

use anyhow::{ensure, Context, Result};
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const NAME: &str = "SteamVR Lighthouse Manager";
const BINARY: &str = "steamvr-lighthouse-manager";
const MARKER: &str = ".lighthouse-manager-install";
const OWNER: &[u8] = b"steamvr-lighthouse-manager user installation v1\n";
const ICON: &[u8] = include_bytes!("../../steamvr-lighthouse-manager.png");

#[derive(Clone, Debug)]
pub struct InstallPlan {
    pub directory: PathBuf,
    pub executable: PathBuf,
}

/// A fixed user-local destination; never requests administrator privileges.
pub fn plan() -> Result<InstallPlan> {
    #[cfg(target_os = "windows")]
    let directory = dirs::data_local_dir().context("Cannot locate LocalAppData")?
        .join("Programs").join(NAME);
    #[cfg(target_os = "linux")]
    let directory = dirs::data_local_dir().context("Cannot locate the user data directory")?
        .join(BINARY);
    #[cfg(target_os = "macos")]
    let directory = dirs::home_dir().context("Cannot locate the home directory")?
        .join("Applications").join(format!("{NAME}.app"));
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    anyhow::bail!("User-local installation is not supported on this operating system");

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        #[cfg(target_os = "windows")]
        let executable = directory.join(format!("{BINARY}.exe"));
        #[cfg(target_os = "linux")]
        let executable = directory.join(BINARY);
        #[cfg(target_os = "macos")]
        let executable = directory.join("Contents/MacOS").join(BINARY);
        Ok(InstallPlan { directory, executable })
    }
}

pub fn should_offer() -> bool {
    offer_for(std::env::args_os().skip(1), cfg!(feature = "installer"), || {
        let plan = plan()?;
        same_file(&std::env::current_exe()?, &plan.executable)
    })
}

fn offer_for(arguments: impl Iterator<Item = OsString>, feature: bool,
    installed: impl FnOnce() -> Result<bool>) -> bool {
    let mut explicit = false;
    for argument in arguments {
        if argument == "--portable" { return false; }
        explicit |= argument == "--install";
    }
    explicit || (feature && !installed().unwrap_or(false))
}

pub fn install(plan: &InstallPlan) -> Result<()> {
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    anyhow::bail!("User-local installation is not supported on this operating system");
    // Resolve launcher destinations before touching the installation directory.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    let launcher = launcher_path()?;
    let source = std::env::current_exe().context("Locate the running executable")?;
    install_files(&source, plan)?;
    #[cfg(target_os = "linux")]
    install_desktop(plan, &launcher)?;
    #[cfg(target_os = "windows")]
    install_shortcut(plan, &launcher)?;
    #[cfg(target_os = "macos")]
    install_bundle(plan)?;
    Ok(())
}

/// Installed starts (including updater restarts) must never re-enter the chooser.
pub fn launch(plan: &InstallPlan) -> Result<()> {
    let arguments = std::env::args_os().skip(1)
        .filter(|argument| argument != "--install" && argument != "--portable");
    Command::new(&plan.executable).args(arguments).arg("--portable")
        .current_dir(&plan.directory)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().with_context(|| format!("Start {}", plan.executable.display()))?;
    Ok(())
}

fn same_file(left: &Path, right: &Path) -> Result<bool> {
    match fs::canonicalize(right) {
        Ok(right) => Ok(fs::canonicalize(left)? == right),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn regular_if_present(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(metadata.file_type().is_file(),
                "Refusing to replace a symlink or non-file: {}", path.display());
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("Inspect {}", path.display())),
    }
}

fn owned_directory(directory: &Path) -> Result<()> {
    let parent = directory.parent().context("Installation directory has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("Create {}", parent.display()))?;
    match fs::create_dir(directory) {
        Ok(()) => {
            // A marker is only created in a directory that this call created.
            let mut marker = OpenOptions::new().write(true).create_new(true)
                .open(directory.join(MARKER)).context("Create installation ownership marker")?;
            marker.write_all(OWNER)?;
            marker.sync_all()?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            ensure!(fs::symlink_metadata(directory)?.file_type().is_dir(),
                "Installation destination is not a real directory: {}", directory.display());
            let marker = directory.join(MARKER);
            ensure!(regular_if_present(&marker)? && fs::read(&marker)? == OWNER,
                "Refusing to overwrite an existing, unrecognized directory: {}. Move it aside before installing.", directory.display());
        }
        Err(error) => return Err(error).with_context(|| format!("Create {}", directory.display())),
    }
    Ok(())
}

fn install_files(source: &Path, plan: &InstallPlan) -> Result<()> {
    ensure!(fs::metadata(source)?.is_file(), "Installation source is not a regular executable");
    owned_directory(&plan.directory)?;
    #[cfg(target_os = "macos")]
    {
        // Refuse linked bundle subdirectories before creating or writing within them.
        for directory in [plan.directory.join("Contents"), plan.directory.join("Contents/MacOS")] {
            match fs::symlink_metadata(&directory) {
                Ok(metadata) => ensure!(metadata.file_type().is_dir(),
                    "Bundle directory is not a real directory: {}", directory.display()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&directory)?,
                Err(error) => return Err(error.into()),
            }
        }
    }
    regular_if_present(&plan.executable)?;
    if !same_file(source, &plan.executable)? {
        let mut staged = StagedFile::new(&plan.executable)?;
        let mut input = File::open(source).context("Open the running executable for installation")?;
        std::io::copy(&mut input, staged.writer()).context("Copy executable into installation staging")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // No copied setuid/setgid bits; the installed app runs as this user.
            staged.writer().set_permissions(fs::Permissions::from_mode(0o755))?;
        }
        staged.commit(&plan.executable).context(
            "Replace the installed executable. Close any running installed copy and retry; the previous executable has not been deleted")?;
    }
    replace_bytes(&plan.directory.join("icon.png"), ICON)?;
    Ok(())
}

/// A sibling create-new file makes failed copies harmless and renames same-volume.
struct StagedFile {
    path: PathBuf,
    file: Option<File>,
}

impl StagedFile {
    fn new(destination: &Path) -> Result<Self> {
        let parent = destination.parent().context("Destination has no parent")?;
        let path = parent.join(format!(".lighthouse-install-{}.tmp", uuid::Uuid::new_v4()));
        let file = OpenOptions::new().write(true).create_new(true).open(&path)
            .with_context(|| format!("Stage installation in {}", parent.display()))?;
        Ok(Self { path, file: Some(file) })
    }

    fn writer(&mut self) -> &mut File {
        self.file.as_mut().expect("staging file is open until commit")
    }

    fn commit(mut self, destination: &Path) -> Result<()> {
        self.writer().sync_all()?;
        // Close before renaming, also allowing Windows COM to save a shortcut
        // without conflicting with an already-open destination handle.
        drop(self.file.take());
        regular_if_present(destination)?;
        #[cfg(not(target_os = "windows"))]
        fs::rename(&self.path, destination)
            .with_context(|| format!("Publish {}", destination.display()))?;
        #[cfg(target_os = "windows")]
        {
            use windows::{core::PCWSTR, Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH}};
            let source = wide(self.path.as_os_str())?;
            let target = wide(destination.as_os_str())?;
            unsafe { MoveFileExW(PCWSTR(source.as_ptr()), PCWSTR(target.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) }
                .with_context(|| format!("Publish {} (close the installed app if it is running)", destination.display()))?;
        }
        Ok(())
    }
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        drop(self.file.take());
        // This unpredictable path was created by us; never remove destination files.
        let _ = fs::remove_file(&self.path);
    }
}

fn replace_bytes(path: &Path, content: &[u8]) -> Result<()> {
    regular_if_present(path)?;
    let mut staged = StagedFile::new(path)?;
    staged.writer().write_all(content)?;
    staged.commit(path)
}

#[cfg(target_os = "linux")]
fn launcher_path() -> Result<PathBuf> {
    Ok(dirs::data_dir().context("Cannot locate the user applications directory")?
        .join("applications").join(format!("{BINARY}.desktop")))
}

#[cfg(target_os = "linux")]
fn desktop_path(path: &Path) -> Result<String> {
    let path = path.to_str().context("Desktop launchers require UTF-8 installation paths")?;
    ensure!(!path.contains(['\n', '\r', '\0']), "Desktop launcher paths cannot contain line breaks or NUL");
    Ok(path.replace('\\', "\\\\"))
}

#[cfg(target_os = "linux")]
fn desktop_entry(plan: &InstallPlan) -> Result<String> {
    // Exec quoting and desktop string escaping are separate layers. Percent signs
    // must also be doubled, otherwise a literal filename becomes a field code.
    let executable = plan.executable.to_str().context("Desktop launchers require UTF-8 executable paths")?;
    ensure!(!executable.contains(['\n', '\r', '\0']), "Desktop executable paths cannot contain line breaks or NUL");
    let quoted = executable.replace('\\', "\\\\").replace('"', "\\\"")
        .replace('`', "\\`").replace('$', "\\$").replace('%', "%%").replace('\\', "\\\\");
    let icon = desktop_path(&plan.directory.join("icon.png"))?;
    Ok(format!("[Desktop Entry]\nType=Application\nName={NAME}\nComment=Manage SteamVR Lighthouse base stations\nExec=\"{quoted}\" --portable\nIcon={icon}\nTerminal=false\nCategories=Utility;\nX-Lighthouse-Manager-Installer=1\n"))
}

#[cfg(target_os = "linux")]
fn install_desktop(plan: &InstallPlan, launcher: &Path) -> Result<()> {
    let entry = desktop_entry(plan)?;
    if regular_if_present(launcher)? {
        let previous = fs::read_to_string(launcher).context("Read existing application launcher")?;
        ensure!(previous.lines().any(|line| line == "X-Lighthouse-Manager-Installer=1"),
            "Refusing to overwrite an unrecognized launcher: {}", launcher.display());
    }
    fs::create_dir_all(launcher.parent().context("Launcher has no parent")?)?;
    replace_bytes(launcher, entry.as_bytes()).context("Install the user application launcher")
}

#[cfg(target_os = "macos")]
fn install_bundle(plan: &InstallPlan) -> Result<()> {
    // Finder recognizes the per-user .app bundle directly; configuration is not bundled.
    let plist = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>CFBundleName</key><string>{NAME}</string><key>CFBundleDisplayName</key><string>{NAME}</string><key>CFBundleIdentifier</key><string>io.github.steamvr-lighthouse-manager</string><key>CFBundleExecutable</key><string>{BINARY}</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>{}</string><key>NSBluetoothAlwaysUsageDescription</key><string>Connect to and control your SteamVR Lighthouse base stations.</string><key>NSHighResolutionCapable</key><true/></dict></plist>\n", env!("CARGO_PKG_VERSION").split('-').next().unwrap_or("1.0.0"));
    replace_bytes(&plan.directory.join("Contents/Info.plist"), plist.as_bytes())
        .context("Create the Finder application bundle")
}

#[cfg(target_os = "windows")]
fn wide(value: &std::ffi::OsStr) -> Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;
    let mut value: Vec<u16> = value.encode_wide().collect();
    ensure!(!value.contains(&0), "Windows installation paths cannot contain NUL");
    value.push(0);
    Ok(value)
}

#[cfg(target_os = "windows")]
fn launcher_path() -> Result<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::{System::Com::CoTaskMemFree, UI::Shell::{SHGetKnownFolderPath, FOLDERID_Programs, KF_FLAG_DEFAULT}};
    let raw = unsafe { SHGetKnownFolderPath(&FOLDERID_Programs, KF_FLAG_DEFAULT, None) }
        .context("Locate the current user's Start Menu Programs folder")?;
    let path = unsafe { PathBuf::from(OsString::from_wide(raw.as_wide())) };
    unsafe { CoTaskMemFree(Some(raw.0.cast())) };
    Ok(path.join(format!("{NAME}.lnk")))
}

#[cfg(target_os = "windows")]
fn install_shortcut(plan: &InstallPlan, launcher: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStringExt;
    use windows::{core::{w, Interface, PCWSTR}, Win32::{
        System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, STGM_READ},
        UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH},
    }};
    struct Apartment(bool);
    impl Drop for Apartment {
        fn drop(&mut self) { if self.0 { unsafe { CoUninitialize() }; } }
    }
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    // An executor thread may already belong to an MTA. Use it without balancing
    // someone else's initialization; ShellLink supports either apartment model.
    if initialized.is_err() && initialized.0 != 0x80010106u32 as i32 {
        initialized.ok().context("Initialize Windows shortcut support")?;
    }
    let _apartment = Apartment(initialized.is_ok());
    let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
        .context("Create Windows Start Menu shortcut")?;
    let persistence: IPersistFile = link.cast()?;
    if regular_if_present(launcher)? {
        let path = wide(launcher.as_os_str())?;
        unsafe { persistence.Load(PCWSTR(path.as_ptr()), STGM_READ) }
            .context("Read the existing Start Menu shortcut; refusing to replace an unrecognized file")?;
        let mut target = vec![0u16; 32768];
        unsafe { link.GetPath(&mut target, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32) }?;
        let end = target.iter().position(|value| *value == 0).context("Invalid existing shortcut target")?;
        ensure!(same_file(&PathBuf::from(OsString::from_wide(&target[..end])), &plan.executable)?,
            "Refusing to overwrite a shortcut belonging to another application: {}", launcher.display());
    }
    let executable = wide(plan.executable.as_os_str())?;
    let directory = wide(plan.directory.as_os_str())?;
    unsafe {
        link.SetPath(PCWSTR(executable.as_ptr()))?;
        link.SetArguments(w!("--portable"))?;
        link.SetDescription(w!("Manage SteamVR Lighthouse base stations"))?;
        link.SetWorkingDirectory(PCWSTR(directory.as_ptr()))?;
        link.SetIconLocation(PCWSTR(executable.as_ptr()), 0)?;
    }
    fs::create_dir_all(launcher.parent().context("Shortcut has no parent")?)?;
    let mut staged = StagedFile::new(launcher)?;
    drop(staged.file.take());
    let temporary = wide(staged.path.as_os_str())?;
    unsafe { persistence.Save(PCWSTR(temporary.as_ptr()), true) }.context("Write the Start Menu shortcut")?;
    staged.file = Some(OpenOptions::new().write(true).open(&staged.path)?);
    staged.commit(launcher).context("Publish the Start Menu shortcut")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sandbox(PathBuf);
    impl Sandbox {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("lighthouse-installer-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn plan(&self) -> InstallPlan {
            let directory = self.0.join("installed");
            #[cfg(target_os = "macos")]
            let executable = directory.join("Contents/MacOS").join(BINARY);
            #[cfg(not(target_os = "macos"))]
            let executable = directory.join(BINARY);
            InstallPlan { directory, executable }
        }
        fn source(&self) -> PathBuf {
            let source = self.0.join("portable");
            fs::write(&source, b"test executable bytes").unwrap();
            source
        }
    }
    impl Drop for Sandbox {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn refuses_unowned_directory_without_touching_its_files() {
        let sandbox = Sandbox::new();
        let plan = sandbox.plan();
        fs::create_dir(&plan.directory).unwrap();
        let unrelated = plan.directory.join("existing-user-file");
        fs::write(&unrelated, b"do not touch").unwrap();
        assert!(install_files(&sandbox.source(), &plan).is_err());
        assert_eq!(fs::read(unrelated).unwrap(), b"do not touch");
        assert!(!plan.directory.join(MARKER).exists());
        fs::write(plan.directory.join(MARKER), b"another application's marker").unwrap();
        assert!(install_files(&sandbox.source(), &plan).is_err());
        assert_eq!(fs::read(plan.directory.join(MARKER)).unwrap(), b"another application's marker");
    }

    #[test]
    fn installs_atomically_and_preserves_other_files_on_reinstall() {
        let sandbox = Sandbox::new();
        let plan = sandbox.plan();
        let source = sandbox.source();
        install_files(&source, &plan).unwrap();
        let user_file = plan.directory.join("user-notes");
        fs::write(&user_file, b"keep me").unwrap();
        fs::write(&source, b"replacement executable").unwrap();
        install_files(&source, &plan).unwrap();
        assert_eq!(fs::read(&plan.executable).unwrap(), b"replacement executable");
        assert_eq!(fs::read(user_file).unwrap(), b"keep me");
        assert_eq!(fs::read(plan.directory.join("icon.png")).unwrap(), ICON);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&plan.executable).unwrap().permissions().mode() & 0o7777, 0o755);
        }
        // Explicit --install from an installed binary must not truncate itself.
        install_files(&plan.executable, &plan).unwrap();
        assert_eq!(fs::read(&plan.executable).unwrap(), b"replacement executable");
    }

    #[test]
    fn failed_staging_does_not_change_destination_or_leave_partial_files() {
        let sandbox = Sandbox::new();
        let destination = sandbox.0.join("executable");
        fs::write(&destination, b"previous executable").unwrap();
        let temporary;
        {
            let mut staged = StagedFile::new(&destination).unwrap();
            temporary = staged.path.clone();
            staged.writer().write_all(b"incomplete executable").unwrap();
            // A copy error unwinds through Drop before commit is attempted.
        }
        assert!(!temporary.exists());
        assert_eq!(fs::read(destination).unwrap(), b"previous executable");
    }

    #[test]
    fn failed_publish_preserves_non_file_destination_and_cleans_staging() {
        let sandbox = Sandbox::new();
        let destination = sandbox.0.join("executable");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("user-file"), b"preserved").unwrap();
        let mut staged = StagedFile::new(&destination).unwrap();
        let temporary = staged.path.clone();
        staged.writer().write_all(b"complete executable").unwrap();
        assert!(staged.commit(&destination).is_err());
        assert!(!temporary.exists());
        assert_eq!(fs::read(destination.join("user-file")).unwrap(), b"preserved");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_linked_install_directory_and_executable() {
        use std::os::unix::fs::symlink;
        let sandbox = Sandbox::new();
        let plan = sandbox.plan();
        let source = sandbox.source();
        let other = sandbox.0.join("other");
        fs::create_dir(&other).unwrap();
        symlink(&other, &plan.directory).unwrap();
        assert!(install_files(&source, &plan).is_err());
        fs::remove_file(&plan.directory).unwrap();
        install_files(&source, &plan).unwrap();
        fs::remove_file(&plan.executable).unwrap();
        symlink(&source, &plan.executable).unwrap();
        assert!(install_files(&source, &plan).is_err());
        assert_eq!(fs::read(source).unwrap(), b"test executable bytes");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn desktop_launcher_quotes_paths_and_does_not_overwrite_unowned_launchers() {
        let sandbox = Sandbox::new();
        let plan = InstallPlan {
            directory: sandbox.0.join("space $ and % and \\\""),
            executable: PathBuf::from("/tmp/a b/$HOME/100%/\"`app"),
        };
        let entry = desktop_entry(&plan).unwrap();
        assert!(entry.lines().any(|line| line == r#"Exec="/tmp/a b/\\$HOME/100%%/\\"\\`app" --portable"#));
        let launcher = sandbox.0.join("manager.desktop");
        fs::write(&launcher, b"unrelated desktop entry").unwrap();
        assert!(install_desktop(&plan, &launcher).is_err());
        assert_eq!(fs::read(&launcher).unwrap(), b"unrelated desktop entry");
        fs::remove_file(&launcher).unwrap();
        install_desktop(&plan, &launcher).unwrap();
        assert_eq!(fs::read_to_string(launcher).unwrap(), entry);
    }

    #[test]
    fn portable_never_checks_or_mutates_install_state() {
        let args = ["--install", "--portable"].map(OsString::from);
        assert!(!offer_for(args.into_iter(), true, || panic!("portable must short circuit")));
        assert!(!offer_for(std::iter::empty(), false, || panic!("regular build must short circuit")));
        assert!(!offer_for(std::iter::empty(), true, || Ok(true)));
        assert!(offer_for([OsString::from("--install")].into_iter(), false, || Ok(true)));
    }
}
