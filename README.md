# SteamVR Lighthouse Manager

A native Rust/GPUI application for controlling SteamVR Base Stations 2.0 over
Bluetooth LE. No SteamVR headset is required.

## Features

- Discover base stations and read their power state and channel.
- Change power state and channel, and identify individual stations.
- Organize stations into groups and assign nicknames.
- Save discovered stations and settings between launches.
- Windows SteamVR monitoring with automatic station wake/sleep and live waiting status.
- Windows tray menu with Show, Wake up, Sleep, and Quit.
- Windows x64 self-updates under the current user account, without an elevated installer.
- Embedded Poppins fonts; no system font installation is required.

## Requirements

- Bluetooth LE (4.0 or higher), enabled on your system.
- SteamVR Base Station 2.0.
- A current stable Rust toolchain and Cargo.
- Platform-native build tools: MSVC Build Tools and the Windows SDK on Windows,
  or Xcode Command Line Tools on macOS. Linux additionally requires BlueZ/D-Bus,
  a Vulkan-capable graphics driver, and GPUI's native development dependencies.

## Build and run

```sh
cargo run --locked
cargo build --release --locked
```

The release executable is `target/release/steamvr-lighthouse-manager.exe` on
Windows or `target/release/steamvr-lighthouse-manager` on Linux and macOS.
The application version is defined in `Cargo.toml`.

Run the Rust tests with:

```sh
cargo test --locked --bin steamvr-lighthouse-manager
```

### Linux desktop launcher

After building a release:

```sh
install -Dm755 target/release/steamvr-lighthouse-manager ~/.local/bin/steamvr-lighthouse-manager
install -Dm644 resources/steamvr-lighthouse-manager.desktop ~/.local/share/applications/steamvr-lighthouse-manager.desktop
install -Dm644 steamvr-lighthouse-manager.png ~/.local/share/icons/hicolor/256x256/apps/steamvr-lighthouse-manager.png
```

Ensure `~/.local/bin` is on your desktop session's `PATH`.

## Usage

Launch the application with Bluetooth enabled. Discovered stations appear even
when no stations have been saved. Use the station controls to change power state,
channel, or nickname, and the group controls to organize stations.

Configuration is stored at
`{user-config-dir}/Alumi/Base Station Manager/config.json`.
After service discovery completes, the station's Bluetooth address and channel
are saved automatically. The `services discovered in ... ms` log includes the
initialization time, channel, and power state.

### Waiting for SteamVR (Windows)

Enable **Manage Base Station Power** in Settings. The app checks `vrserver.exe`
once per second on a background thread and shows **Waiting** until SteamVR starts,
then **Active**. SteamVR is recognized through its Steam configuration, OpenVR
registration, or a running `vrserver.exe`, including custom installation paths.

Each station's **Wake up** and **Power off** switches control whether it wakes
when SteamVR starts and sleeps when SteamVR exits. Stations that finish connecting
later follow the current SteamVR state too. Launching the manager with SteamVR
closed puts eligible stations to sleep while waiting. Disabling management leaves
manual controls in charge; unchanged process observations do not override manual
power changes. A process-monitoring failure never counts as a SteamVR exit.

### System tray (Windows)

With **Allow tray** enabled, the close button and Alt+F4 hide the window without
stopping Bluetooth or SteamVR monitoring. Click the tray icon to restore the
window, or right-click for **Show**, **Wake up**, **Sleep**, and **Quit**.
Wake up and Sleep apply to all discovered stations; Quit exits the application
even when closing normally would hide it.

The window also hides when managed SteamVR becomes active. Restoring it manually
keeps it visible until you close it or SteamVR becomes active again. Disabling
Allow tray removes the icon and restores a hidden window; closing then exits.
If tray initialization fails, closing exits rather than hiding an inaccessible
window. Tray controls are disabled on unsupported platforms.

### Settings

The author, designer, and source-code links in Settings open in your default browser.
Entry descriptions use regular-weight Poppins; titles retain their existing emphasis.
Station nickname and group-name fields focus on click, support keyboard selection/editing,
and keep text and selection vertically centered within the input.
An unset station nickname displays its Bluetooth device name as a gray placeholder.
Typing replaces the placeholder with white text; clearing restores it. The white
caret blinks while focused. Caret redraws and selection changes do not save settings.

### Per-user updates (Windows x64)

Updates are checked in the background at startup. Open **Settings → Updates** to
check again or choose `main`/`beta`. Switching branches only checks. A single
button shows **Check** normally and **Update** when a newer version is available.
Clicking Update downloads the release, verifies its size, SHA-256 checksum, and
x64 executable format, then hands off to a native replacement helper.

The helper inherits the current Windows account and token. It waits for the app
to exit, replaces the executable, and restarts it without an installer, UAC prompt,
or machine-wide changes. Settings remain in the same user configuration folder.
The executable's folder must be writable by that account; protected locations
report an error instead of asking for administrator access. The original executable
is retained for rollback if replacement or restart fails.

Only the native portable asset and its checksum are eligible:

- `steamvr-lighthouse-manager-windows-x86_64.exe`
- `steamvr-lighthouse-manager-windows-x86_64.exe.sha256`

Legacy Go/Wails installers and executables are ignored. Until a native release
is published for a branch, the UI reports that explicitly and does not offer Update.
GitHub release-publishing access is the updater's trust boundary; checksums verify
download integrity, not an independent publisher signature.

#### Publishing a native release

The Windows release workflow builds the Rust executable and publishes the asset
pair above when a `*-main` or `*-beta` tag is pushed. The version portion must
exactly match `Cargo.toml` (an initial `v` is optional). For example, after setting
the package version to `1.2.1-gpui`, publish `v1.2.1-gpui-main` or
`v1.2.1-gpui-beta`. Beta releases are marked prerelease. The workflow refuses a
version mismatch and does not overwrite existing releases.


### Connection initialization

On Windows, initialization queries only the lighthouse GATT service and its
characteristics, reusing Windows' service cache rather than enumerating unrelated
services and descriptors. Channel and power values are read live, concurrently.
Linux and macOS continue to use btleplug for GATT access.

Bluetooth initialization starts alongside UI state setup. The six-second scan
window still allows nearby stations to advertise, but each station's controls
become usable as soon as its own initialization finishes; commands do not wait
for the scan or other stations to finish.

### Window layout

Views resize smoothly to their measured content after navigation and dropdown
changes, with shared 24-pixel side/bottom padding and 12-pixel section spacing.
Dropdown and other content-height changes use a 220 ms interruptible easing curve.
Navigation resizing shares the page's 300 ms total budget. Changing direction
starts from the current animated height. Tall views scroll instead of extending
below the available display area, and dropdown choices wrap within the window width.

### Transitions

The native UI implements the active transitions from the
[original frontend](https://github.com/DHCPCD9/go-steamvr-lighthouse-manager/tree/main/frontend):

- Wait-mode route fades/slides and delayed breadcrumb enter/exit with measured pop-layout.
- Station entrances stagger on first load and for newly discovered stations.
  Navigating between views takes 300 ms total (exit and entry combined), without
  replaying per-station delays or extending the incoming controls' fades beyond
  the page transition. Retained exits, status-color changes, wait-mode channel
  labels, and conditional control fades remain animated.
- Interruptible hover/press, disabled, border, icon and text-color transitions.
- 100 ms no-bounce toggles; 150 ms dropdown fades/chevrons and 200 ms choice colors.
- Conditional toolbar actions, scanning-only rotation, edit-icon rotation/fades,
  and the update-ready tooltip.
- Settings/updates description and button entrances, footer-link color transitions,
  and title-bar status/error presence.

Exit elements remain mounted until their animations finish and do not accept
actions while leaving. Motion's default easing/spring parameters and explicit
frontend CSS timings are shared across widgets; interrupted animations continue
from their current values.

The Connecting indicator uses the frontend's Tailwind 4 `blue-500` (`#2B7FFF`
in sRGB); its label retains the original muted gray (`#C6C6C6`).

## License

[GPLv3](LICENSE). Poppins is distributed under the
[SIL Open Font License](assets/fonts/OFL.txt).

### Contact
 - VRChat: [Alumi Shinamo](https://vrchat.com/home/user/usr_98358fea-8df8-4c08-ab4f-db555ae4e959)
 - X: [@AlumiShinamo](https://x.com/AlumiShinamo)
 - Discord: @dhcpcd

### Sponsor
 - USDT TRC20: TDKHKZEnZihb1grhHiFanBaPv2TF9ADdDF
 - BTC: bc1qk6hynnya9c5dexluhhcgukq598eunffw5yntpa
 - LTC: MNEJjjZU1gmr67snvs8XB4FmFwfwAm952a
 - ETH Mainnet: 0x67EFB56Fa96374571Af0c29CD38De89D42b20E83
