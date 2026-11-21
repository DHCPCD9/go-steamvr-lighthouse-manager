//! Native portable updates from this repository's GitHub releases.

use anyhow::{Result, anyhow};

#[derive(Clone, Debug)]
pub enum Status {
    Unsupported,
    Idle,
    Checking,
    NoRelease,
    UpToDate,
    Available { version: String },
    Downloading { received: u64, total: u64 },
    Verifying,
    Preparing,
    Restarting,
    Error(String),
}

impl Status {
    pub fn busy(&self) -> bool {
        matches!(
            self,
            Self::Checking
                | Self::Downloading { .. }
                | Self::Verifying
                | Self::Preparing
                | Self::Restarting
        )
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Status(Status),
    ReadyToRestart,
}

pub enum Command {
    Check { branch: String },
    Install { branch: String },
}

pub struct Handle {
    pub events: flume::Receiver<Event>,
    commands: flume::Sender<Command>,
}

impl Handle {
    /// Never block the UI if another command is already queued.
    pub fn send(&self, command: Command) -> Result<()> {
        self.commands
            .try_send(command)
            .map_err(|error| anyhow!("Cannot queue update command: {error}"))
    }
}

pub fn is_supported() -> bool {
    cfg!(all(target_os = "windows", target_arch = "x86_64"))
}

pub fn start() -> Option<Handle> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        let (commands, receiver) = flume::bounded(1);
        let (sender, events) = flume::unbounded();
        let failure_sender = sender.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("native-updater".into())
            .spawn(move || worker::run(receiver, sender))
        {
            let _ = failure_sender.send(Event::Status(Status::Error(format!(
                "Cannot start update worker: {error}"
            ))));
        }
        Some(Handle { events, commands })
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
    {
        None
    }
}

#[cfg(all(target_os = "windows", any(target_arch = "x86_64", test)))]
mod worker {
    use super::*;
    use anyhow::{Context, bail, ensure};
    use reqwest::{
        Url,
        blocking::{Client, Response},
        header::{ACCEPT, ACCEPT_ENCODING, LOCATION},
        redirect::Policy,
    };
    use semver::Version;
    use serde::Deserialize;
    use sha2::{Digest, Sha256};
    use std::{
        fs::File,
        io::{Read, Seek, SeekFrom, Write},
        path::Path,
        time::Duration,
    };

    #[cfg(test)]
    const REPOSITORY: &str = "DHCPCD9/go-steamvr-lighthouse-manager";
    const API: &str =
        "https://api.github.com/repos/DHCPCD9/go-steamvr-lighthouse-manager/releases";
    const BINARY: &str = "steamvr-lighthouse-manager-windows-x86_64.exe";
    const CHECKSUM: &str = "steamvr-lighthouse-manager-windows-x86_64.exe.sha256";
    const MAX_BINARY: u64 = 512 * 1024 * 1024;
    const MAX_CHECKSUM: u64 = 4096;
    const MAX_METADATA: u64 = 8 * 1024 * 1024;
    const PAGE_SIZE: usize = 100;
    const MAX_PAGES: usize = 20;

    #[derive(Debug, Deserialize)]
    struct Release {
        tag_name: String,
        draft: bool,
        assets: Vec<Asset>,
    }

    #[derive(Clone, Debug, Deserialize)]
    struct Asset {
        name: String,
        size: u64,
        browser_download_url: String,
    }

    #[derive(Debug)]
    struct Candidate {
        version: Version,
        tag: String,
        binary: Asset,
        checksum: Asset,
    }

    #[derive(Debug)]
    enum Selection {
        NoRelease,
        UpToDate,
        Available(Candidate),
    }

    fn current_version(value: &str) -> Result<Version> {
        let mut version = Version::parse(value)
            .context("The running application's version is invalid")?;
        version.build = semver::BuildMetadata::EMPTY;
        Ok(version)
    }

    fn select(releases: Vec<Release>, branch: &str, current: &str) -> Result<Selection> {
        let suffix = match branch {
            "main" => "-main",
            "beta" => "-beta",
            _ => bail!("Unknown update channel: {branch}"),
        };
        let current = current_version(current)?;
        let mut newest: Option<Candidate> = None;
        for release in releases {
            if release.draft {
                continue;
            }
            let Some(version_text) = release.tag_name.strip_suffix(suffix) else {
                continue;
            };
            let mut binary = None;
            let mut checksum = None;
            for asset in release.assets {
                let slot = match asset.name.as_str() {
                    BINARY => &mut binary,
                    CHECKSUM => &mut checksum,
                    _ => continue,
                };
                ensure!(
                    slot.replace(asset).is_none(),
                    "Native release {} contains duplicate native artifacts",
                    release.tag_name
                );
            }
            // Existing Go/Wails releases do not have this exact native asset pair.
            if binary.is_none() && checksum.is_none() {
                continue;
            }
            ensure!(
                binary.is_some() && checksum.is_some(),
                "Native release {} must contain exactly one executable and checksum",
                release.tag_name
            );
            let mut version = Version::parse(version_text.strip_prefix('v').unwrap_or(version_text))
                .with_context(|| format!("Native release has an invalid version: {}", release.tag_name))?;
            version.build = semver::BuildMetadata::EMPTY;
            let binary = binary.unwrap();
            let checksum = checksum.unwrap();
            ensure!(
                (1..=MAX_BINARY).contains(&binary.size),
                "Native executable has an invalid or excessive advertised size"
            );
            ensure!(
                (1..=MAX_CHECKSUM).contains(&checksum.size),
                "Release checksum has an invalid or excessive advertised size"
            );
            validate_asset_url(&binary.browser_download_url, &release.tag_name, BINARY)?;
            validate_asset_url(&checksum.browser_download_url, &release.tag_name, CHECKSUM)?;
            if newest.as_ref().is_none_or(|candidate| version > candidate.version) {
                newest = Some(Candidate {
                    version,
                    tag: release.tag_name,
                    binary,
                    checksum,
                });
            }
        }
        Ok(match newest {
            None => Selection::NoRelease,
            Some(candidate) if candidate.version <= current => Selection::UpToDate,
            Some(candidate) => Selection::Available(candidate),
        })
    }

    fn secure_url(url: &Url) -> Result<()> {
        ensure!(
            url.scheme() == "https"
                && url.username().is_empty()
                && url.password().is_none()
                && url.port_or_known_default() == Some(443)
                && url.fragment().is_none(),
            "Update URL must use HTTPS without credentials, custom ports, or fragments"
        );
        Ok(())
    }

    fn validate_asset_url(value: &str, tag: &str, name: &str) -> Result<()> {
        let url = Url::parse(value).context("Invalid release asset URL")?;
        secure_url(&url)?;
        let mut expected = Url::parse("https://github.com")?;
        expected
            .path_segments_mut()
            .map_err(|_| anyhow!("Cannot construct release asset URL"))?
            .extend(["DHCPCD9", "go-steamvr-lighthouse-manager", "releases", "download", tag, name]);
        ensure!(
            url == expected,
            "Release asset URL does not identify this repository's expected native artifact"
        );
        Ok(())
    }

    fn validate_request_url(url: &Url, initial: bool) -> Result<()> {
        secure_url(url)?;
        let allowed = match url.host_str() {
            Some("api.github.com") => {
                url.path() == "/repos/DHCPCD9/go-steamvr-lighthouse-manager/releases"
            }
            Some("github.com") => url.path().starts_with("/DHCPCD9/go-steamvr-lighthouse-manager/releases/download/"),
            Some("release-assets.githubusercontent.com" | "objects.githubusercontent.com") => !initial,
            _ => false,
        };
        ensure!(allowed, "Update URL or redirect is outside the trusted GitHub release hosts");
        Ok(())
    }

    struct Network {
        client: Client,
        // Only unit tests can route already-validated GitHub requests to a local fixture.
        #[cfg(test)]
        fixture: Option<Url>,
    }

    impl Network {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        fn production() -> Result<Self> {
            Ok(Self {
                client: Client::builder()
                    .https_only(true)
                    .redirect(Policy::none())
                    .connect_timeout(Duration::from_secs(15))
                    .timeout(Duration::from_secs(600))
                    .user_agent(concat!("steamvr-lighthouse-manager/", env!("CARGO_PKG_VERSION")))
                    .build()
                    .context("Cannot initialize secure update networking")?,
                #[cfg(test)]
                fixture: None,
            })
        }

        fn get(&self, mut url: Url, metadata: bool) -> Result<Response> {
            validate_request_url(&url, true)?;
            for redirects in 0..=5 {
                let request_url = {
                    #[cfg(test)]
                    if let Some(origin) = &self.fixture {
                        let mut local = origin.clone();
                        local.set_path(url.path());
                        local.set_query(url.query());
                        local
                    } else {
                        url.clone()
                    }
                    #[cfg(not(test))]
                    url.clone()
                };
                let response = self.client.get(request_url)
                    .header(ACCEPT, if metadata { "application/vnd.github+json" } else { "application/octet-stream" })
                    .header(ACCEPT_ENCODING, "identity")
                    .timeout(Duration::from_secs(if metadata { 30 } else { 600 }))
                    .send()
                    .context("Cannot contact GitHub for the update; check your connection")?;
                if response.status().is_redirection() {
                    ensure!(redirects < 5, "Too many redirects while downloading an update");
                    let location = response.headers().get(LOCATION)
                        .context("GitHub returned a redirect without a destination")?
                        .to_str().context("GitHub returned an invalid redirect destination")?;
                    url = url.join(location).context("GitHub returned an invalid redirect URL")?;
                    validate_request_url(&url, false)?;
                    continue;
                }
                let status = response.status();
                ensure!(
                    status.is_success(),
                    "GitHub update request failed (HTTP {}); check connectivity and GitHub API rate limits",
                    status.as_u16()
                );
                return Ok(response);
            }
            unreachable!("redirect loop always returns or errors")
        }

        fn releases(&self) -> Result<Vec<Release>> {
            let mut releases = Vec::new();
            for page in 1..=MAX_PAGES {
                let mut url = Url::parse(API)?;
                url.query_pairs_mut()
                    .append_pair("per_page", &PAGE_SIZE.to_string())
                    .append_pair("page", &page.to_string());
                let response = self.get(url, true)?;
                let data = bounded_body(response, MAX_METADATA, None)
                    .context("Cannot read GitHub release metadata")?;
                let mut batch: Vec<Release> = serde_json::from_slice(&data)
                    .context("GitHub returned malformed release metadata")?;
                let last = batch.len() < PAGE_SIZE;
                releases.append(&mut batch);
                if last {
                    return Ok(releases);
                }
            }
            bail!("GitHub release history exceeds the safe metadata limit; cannot select an update reliably")
        }

        fn checksum(&self, candidate: &Candidate) -> Result<[u8; 32]> {
            let response = self.get(Url::parse(&candidate.checksum.browser_download_url)?, false)?;
            let bytes = bounded_body(response, MAX_CHECKSUM, Some(candidate.checksum.size))
                .context("Cannot download the release checksum")?;
            parse_checksum(&bytes)
        }

        fn download(
            &self,
            candidate: &Candidate,
            destination: &Path,
            events: &flume::Sender<Event>,
        ) -> Result<[u8; 32]> {
            let response = self.get(Url::parse(&candidate.binary.browser_download_url)?, false)?;
            check_content_length(&response, MAX_BINARY, Some(candidate.binary.size))?;
            let mut output = std::fs::OpenOptions::new()
                .write(true).create_new(true).open(destination)
                .context("Cannot create the staged update executable; check folder write permissions")?;
            let hash = copy_payload(response, &mut output, candidate.binary.size, |received, total| {
                emit(events, Status::Downloading { received, total })
            })?;
            output.sync_all().context("Cannot flush the downloaded update to disk")?;
            Ok(hash)
        }

        fn stage(
            &self,
            candidate: &Candidate,
            destination: &Path,
            events: &flume::Sender<Event>,
        ) -> Result<[u8; 32]> {
            let expected = self.checksum(candidate)?;
            let actual = self.download(candidate, destination, events)?;
            emit(events, Status::Verifying)?;
            verify_payload(destination, actual, expected)?;
            Ok(expected)
        }
    }

    fn check_content_length(response: &Response, limit: u64, expected: Option<u64>) -> Result<()> {
        if let Some(length) = response.content_length() {
            ensure!(length <= limit, "Update response exceeds the safe size limit");
            if let Some(expected) = expected {
                ensure!(length == expected, "Update response size differs from release metadata");
            }
        }
        Ok(())
    }

    fn bounded_body(mut response: Response, limit: u64, expected: Option<u64>) -> Result<Vec<u8>> {
        check_content_length(&response, limit, expected)?;
        let capacity = expected.or_else(|| response.content_length()).unwrap_or(0).min(limit);
        let mut bytes = Vec::with_capacity(capacity as usize);
        response.by_ref().take(limit + 1).read_to_end(&mut bytes)
            .context("Update response was interrupted")?;
        ensure!(bytes.len() as u64 <= limit, "Update response exceeds the safe size limit");
        if let Some(expected) = expected {
            ensure!(bytes.len() as u64 == expected, "Update response size differs from release metadata");
        }
        Ok(bytes)
    }

    fn parse_checksum(bytes: &[u8]) -> Result<[u8; 32]> {
        let text = std::str::from_utf8(bytes).context("Release checksum is not UTF-8 text")?;
        let mut fields = text.split_whitespace();
        let hash = fields.next().context("Release checksum is empty")?;
        ensure!(hash.len() == 64 && hash.is_ascii(), "Release checksum must contain a SHA-256 hex digest");
        if let Some(name) = fields.next() {
            ensure!(name.strip_prefix('*').unwrap_or(name) == BINARY, "Release checksum names a different executable");
        }
        ensure!(fields.next().is_none(), "Release checksum contains unexpected extra data");
        let mut digest = [0_u8; 32];
        for (index, byte) in digest.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hash[index * 2..index * 2 + 2], 16)
                .context("Release checksum contains invalid hexadecimal digits")?;
        }
        Ok(digest)
    }

    fn copy_payload(
        mut input: impl Read,
        output: &mut impl Write,
        expected: u64,
        mut progress: impl FnMut(u64, u64) -> Result<()>,
    ) -> Result<[u8; 32]> {
        ensure!((1..=MAX_BINARY).contains(&expected), "Invalid advertised executable size");
        progress(0, expected)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        let mut received = 0_u64;
        let interval = (expected / 100).max(256 * 1024);
        let mut next_progress = interval;
        loop {
            let count = input.read(&mut buffer).context("Update download was interrupted")?;
            if count == 0 {
                break;
            }
            received += count as u64;
            ensure!(received <= expected, "Downloaded executable exceeds its advertised size");
            output.write_all(&buffer[..count]).context("Cannot write the staged executable; check free disk space")?;
            hasher.update(&buffer[..count]);
            if received >= next_progress && received < expected {
                progress(received, expected)?;
                next_progress = received.saturating_add(interval);
            }
        }
        ensure!(received == expected, "Downloaded executable is truncated");
        progress(received, expected)?;
        Ok(hasher.finalize().into())
    }

    fn verify_payload(path: &Path, actual: [u8; 32], expected: [u8; 32]) -> Result<()> {
        ensure!(actual == expected, "Downloaded executable failed SHA-256 verification; nothing was installed");
        let mut file = File::open(path).context("Cannot reopen the downloaded executable")?;
        let length = file.metadata()?.len();
        let mut dos = [0_u8; 64];
        file.read_exact(&mut dos).context("Downloaded executable has a truncated DOS header")?;
        ensure!(&dos[..2] == b"MZ", "Downloaded file is not a Windows executable");
        let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap()) as u64;
        ensure!(offset >= 64 && offset + 24 <= length, "Downloaded executable has an invalid PE offset");
        file.seek(SeekFrom::Start(offset))?;
        let mut coff = [0_u8; 24];
        file.read_exact(&mut coff)?;
        let sections = u16::from_le_bytes([coff[6], coff[7]]) as u64;
        let optional_size = u16::from_le_bytes([coff[20], coff[21]]) as u64;
        let characteristics = u16::from_le_bytes([coff[22], coff[23]]);
        ensure!(
            &coff[..4] == b"PE\0\0"
                && u16::from_le_bytes([coff[4], coff[5]]) == 0x8664
                && sections > 0
                && optional_size >= 112
                && offset + 24 + optional_size + sections * 40 <= length
                && characteristics & 0x0002 != 0
                && characteristics & 0x2000 == 0,
            "Downloaded file is not an x64 Windows application"
        );
        let mut optional = [0_u8; 70];
        file.read_exact(&mut optional)?;
        ensure!(
            u16::from_le_bytes([optional[0], optional[1]]) == 0x020b
                && matches!(u16::from_le_bytes([optional[68], optional[69]]), 2 | 3),
            "Downloaded executable has an invalid x64 Windows application header"
        );
        Ok(())
    }

    fn emit(events: &flume::Sender<Event>, status: Status) -> Result<()> {
        events.send(Event::Status(status)).map_err(|_| anyhow!("Update UI disconnected"))
    }

    fn selection_status(selection: &Selection) -> Status {
        match selection {
            Selection::NoRelease => Status::NoRelease,
            Selection::UpToDate => Status::UpToDate,
            Selection::Available(candidate) => Status::Available { version: candidate.tag.clone() },
        }
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    pub(super) fn run(commands: flume::Receiver<Command>, events: flume::Sender<Event>) {
        let network = Network::production();
        while let Ok(command) = commands.recv() {
            let result: Result<bool> = (|| {
                emit(&events, Status::Checking)?;
                let network = network.as_ref().map_err(|error| anyhow!("{error:#}"))?;
                let (branch, install) = match command {
                    Command::Check { branch } => (branch, false),
                    Command::Install { branch } => (branch, true),
                };
                ensure!(matches!(branch.as_str(), "main" | "beta"), "Unknown update channel: {branch}");
                let selection = select(network.releases()?, &branch, env!("CARGO_PKG_VERSION"))?;
                if !install || !matches!(&selection, Selection::Available(_)) {
                    emit(&events, selection_status(&selection))?;
                    return Ok(false);
                }
                let Selection::Available(candidate) = selection else {
                    unreachable!("available selection checked above")
                };
                emit(&events, Status::Preparing)?;
                let staging = super::super::update_apply::prepare()?;
                let expected = network.stage(&candidate, staging.payload_path(), &events)?;
                emit(&events, Status::Preparing)?;
                super::super::update_apply::handoff(staging, expected)?;
                emit(&events, Status::Restarting)?;
                events.send(Event::ReadyToRestart).map_err(|_| anyhow!("Update UI disconnected"))?;
                Ok(true)
            })();
            match result {
                Ok(true) => break,
                Ok(false) => {}
                Err(error) => {
                    if emit(&events, Status::Error(format!("{error:#}"))).is_err() {
                        break;
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{
            io::Cursor,
            net::TcpListener,
            path::PathBuf,
            sync::{Arc, atomic::{AtomicBool, Ordering}},
            thread::{self, JoinHandle},
        };

        fn asset(tag: &str, name: &str, size: u64) -> Asset {
            Asset {
                name: name.into(),
                size,
                browser_download_url: format!("https://github.com/{REPOSITORY}/releases/download/{tag}/{name}"),
            }
        }

        fn native(tag: &str) -> Release {
            Release {
                tag_name: tag.into(),
                draft: false,
                assets: vec![asset(tag, BINARY, 512), asset(tag, CHECKSUM, 64)],
            }
        }

        fn available(releases: Vec<Release>, channel: &str) -> Candidate {
            match select(releases, channel, "1.2.0-gpui").unwrap() {
                Selection::Available(candidate) => candidate,
                other => panic!("Expected an available release, got {other:?}"),
            }
        }

        #[test]
        fn chooses_newest_native_semver_in_the_requested_channel() {
            let releases = vec![
                native("v1.9.0-main"),
                native("1.10.0-main"),
                native("v9.0.0-beta"),
                native("1.3.0-main"),
            ];
            assert_eq!(available(releases, "main").tag, "1.10.0-main");
            assert_eq!(
                available(vec![native("v9.0.0-main"), native("v1.4.0-beta")], "beta").tag,
                "v1.4.0-beta"
            );
        }

        #[test]
        fn legacy_drafts_and_unrecognized_channels_never_become_updates() {
            let mut legacy = native("v99.0.0-main");
            legacy.assets[0].name = "steamvr-lighthouse-manager-amd64-installer.exe".into();
            legacy.assets.remove(1);
            let mut draft = native("v99.0.0-main");
            draft.draft = true;
            let selected = select(vec![legacy, draft, native("v9.0.0-beta")], "main", "1.2.0-gpui").unwrap();
            assert!(matches!(selection_status(&selected), Status::NoRelease));
            assert!(select(vec![native("v2.0.0-main")], "nightly", "1.2.0-gpui").is_err());
        }

        #[test]
        fn equal_or_older_native_versions_are_up_to_date_not_installable() {
            for tag in ["v1.2.0-main", "1.1.9-main", "v1.2.0+build.99-main"] {
                let selected = select(vec![native(tag)], "main", "1.2.0").unwrap();
                assert!(matches!(selection_status(&selected), Status::UpToDate));
            }
            assert!(matches!(
                select(vec![], "main", "1.2.0-gpui").unwrap(),
                Selection::NoRelease
            ));
        }

        #[test]
        fn prerelease_build_can_upgrade_to_stable_without_reinstalling_itself() {
            assert!(matches!(
                select(vec![native("v1.2.0-main")], "main", "1.2.0-gpui").unwrap(),
                Selection::Available(_)
            ));
            assert!(matches!(
                select(vec![native("v1.2.0-gpui-main")], "main", "1.2.0-gpui").unwrap(),
                Selection::UpToDate
            ));
        }

        #[test]
        fn malformed_native_releases_are_errors_not_no_release() {
            let mut missing = native("v2.0.0-main");
            missing.assets.pop();
            let mut duplicate = native("v2.0.0-main");
            duplicate.assets.push(duplicate.assets[0].clone());
            let mut oversized = native("v2.0.0-main");
            oversized.assets[0].size = MAX_BINARY + 1;
            let mut wrong_repo = native("v2.0.0-main");
            wrong_repo.assets[0].browser_download_url =
                wrong_repo.assets[0].browser_download_url.replace(REPOSITORY, "attacker/repository");
            for release in [missing, duplicate, oversized, wrong_repo, native("not-semver-main")] {
                assert!(select(vec![release], "main", "1.2.0-gpui").is_err());
            }
        }

        #[test]
        fn only_exact_https_repository_artifacts_and_secure_redirects_are_allowed() {
            let valid = asset("v2.0.0-main", BINARY, 512).browser_download_url;
            validate_asset_url(&valid, "v2.0.0-main", BINARY).unwrap();
            for invalid in [
                valid.replace("https:", "http:"),
                valid.replace("github.com/", "github.com.evil.test/"),
                valid.replace("github.com/", "user@github.com/"),
                valid.replace("github.com/", "github.com:444/"),
                format!("{valid}?unexpected=true"),
                valid.replace("/v2.0.0-main/", "/v9.0.0-main/"),
            ] {
                assert!(validate_asset_url(&invalid, "v2.0.0-main", BINARY).is_err());
            }
            assert!(validate_request_url(&Url::parse("https://release-assets.githubusercontent.com/signed").unwrap(), false).is_ok());
            for url in [
                "http://release-assets.githubusercontent.com/signed",
                "https://evil.test/payload",
                "https://github.com/other/repository/releases/download/v2/payload",
            ] {
                assert!(validate_request_url(&Url::parse(url).unwrap(), false).is_err());
            }
        }

        #[test]
        fn checksum_accepts_standard_formats_and_rejects_ambiguity() {
            let digest = "AB".repeat(32);
            for text in [
                digest.clone(),
                format!("{digest}\n"),
                format!("{digest}  {BINARY}\r\n"),
                format!("{digest} *{BINARY}"),
            ] {
                assert_eq!(parse_checksum(text.as_bytes()).unwrap(), [0xab; 32]);
            }
            for text in [
                String::new(),
                "0".repeat(63),
                "zz".repeat(32),
                format!("{digest} other.exe"),
                format!("{digest} {BINARY}\n{digest}"),
            ] {
                assert!(parse_checksum(text.as_bytes()).is_err());
            }
        }

        #[test]
        fn streaming_rejects_truncation_overflow_and_propagates_io_errors() {
            for (bytes, expected) in [(vec![1, 2], 3), (vec![1, 2, 3], 2)] {
                assert!(copy_payload(Cursor::new(bytes), &mut Vec::new(), expected, |_, _| Ok(())).is_err());
            }
            struct Broken;
            impl Read for Broken {
                fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                    Err(std::io::Error::other("fixture connection dropped"))
                }
            }
            assert!(copy_payload(Broken, &mut Vec::new(), 1, |_, _| Ok(())).is_err());
            assert!(copy_payload(Cursor::new([1]), &mut Vec::new(), 1, |_, _| bail!("cancelled")).is_err());
        }

        #[test]
        fn progress_is_bounded_and_reaches_the_advertised_total() {
            let bytes = vec![7; 4 * 1024 * 1024];
            let mut output = Vec::new();
            let mut progress = Vec::new();
            let digest = copy_payload(Cursor::new(&bytes), &mut output, bytes.len() as u64, |received, total| {
                progress.push((received, total));
                Ok(())
            }).unwrap();
            assert_eq!(output, bytes);
            assert_eq!(digest, <[u8; 32]>::from(Sha256::digest(&bytes)));
            assert_eq!(progress.first(), Some(&(0, bytes.len() as u64)));
            assert_eq!(progress.last(), Some(&(bytes.len() as u64, bytes.len() as u64)));
            assert!(progress.len() <= 102);
            assert!(progress.windows(2).all(|pair| pair[0].0 < pair[1].0));
        }

        struct TempPayload {
            directory: PathBuf,
            path: PathBuf,
        }

        impl TempPayload {
            fn new() -> Self {
                let directory = std::env::temp_dir().join(format!("native-update-test-{}", uuid::Uuid::new_v4()));
                std::fs::create_dir(&directory).unwrap();
                let path = directory.join("payload.exe");
                Self { directory, path }
            }
        }

        impl Drop for TempPayload {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.path);
                let _ = std::fs::remove_dir(&self.directory);
            }
        }

        fn pe() -> Vec<u8> {
            let mut bytes = vec![0; 512];
            bytes[..2].copy_from_slice(b"MZ");
            bytes[60..64].copy_from_slice(&64_u32.to_le_bytes());
            bytes[64..68].copy_from_slice(b"PE\0\0");
            bytes[68..70].copy_from_slice(&0x8664_u16.to_le_bytes());
            bytes[70..72].copy_from_slice(&1_u16.to_le_bytes());
            bytes[84..86].copy_from_slice(&240_u16.to_le_bytes());
            bytes[86..88].copy_from_slice(&0x22_u16.to_le_bytes());
            bytes[88..90].copy_from_slice(&0x20b_u16.to_le_bytes());
            bytes[156..158].copy_from_slice(&2_u16.to_le_bytes());
            bytes
        }

        #[test]
        fn verifies_hash_and_rejects_wrong_architecture_or_non_executable_pe() {
            let payload = TempPayload::new();
            let valid = pe();
            let digest: [u8; 32] = Sha256::digest(&valid).into();
            std::fs::write(&payload.path, &valid).unwrap();
            verify_payload(&payload.path, digest, digest).unwrap();
            assert!(verify_payload(&payload.path, digest, [0; 32]).is_err());
            for offset in [0, 64, 68, 88, 156] {
                let mut invalid = valid.clone();
                invalid[offset] ^= 0xff;
                let hash = Sha256::digest(&invalid).into();
                std::fs::write(&payload.path, invalid).unwrap();
                assert!(verify_payload(&payload.path, hash, hash).is_err());
            }
            let mut dll = valid;
            dll[87] |= 0x20;
            let hash = Sha256::digest(&dll).into();
            std::fs::write(&payload.path, dll).unwrap();
            assert!(verify_payload(&payload.path, hash, hash).is_err());
        }

        struct Fixture {
            network: Network,
            stop: Arc<AtomicBool>,
            server: Option<JoinHandle<()>>,
        }

        impl Fixture {
            fn new(responses: Vec<Vec<u8>>) -> Self {
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                let origin = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
                listener.set_nonblocking(true).unwrap();
                let stop = Arc::new(AtomicBool::new(false));
                let server_stop = Arc::clone(&stop);
                let server = thread::spawn(move || {
                    for response in responses {
                        let mut stream = loop {
                            if server_stop.load(Ordering::Relaxed) {
                                return;
                            }
                            match listener.accept() {
                                Ok((stream, _)) => break stream,
                                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                    thread::sleep(Duration::from_millis(1));
                                }
                                Err(error) => panic!("Fixture accept failed: {error}"),
                            }
                        };
                        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                        stream.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
                        let mut request = Vec::new();
                        while !request.ends_with(b"\r\n\r\n") && request.len() < 16 * 1024 {
                            let mut byte = [0];
                            if stream.read(&mut byte).unwrap_or(0) == 0 {
                                break;
                            }
                            request.push(byte[0]);
                        }
                        let _ = stream.write_all(&response);
                    }
                });
                let network = Network {
                    client: Client::builder().no_proxy().redirect(Policy::none())
                        .connect_timeout(Duration::from_secs(2)).build().unwrap(),
                    fixture: Some(origin),
                };
                Self { network, stop, server: Some(server) }
            }
        }

        impl Drop for Fixture {
            fn drop(&mut self) {
                self.stop.store(true, Ordering::Relaxed);
                if let Some(server) = self.server.take() {
                    let result = server.join();
                    if !thread::panicking() {
                        assert!(result.is_ok(), "HTTP fixture server failed");
                    }
                }
            }
        }

        fn response(status: &str, body: &[u8]) -> Vec<u8> {
            let mut bytes = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).into_bytes();
            bytes.extend_from_slice(body);
            bytes
        }

        #[test]
        fn metadata_http_and_schema_failures_do_not_masquerade_as_no_release() {
            for raw in [
                response("403 Forbidden", b"API rate limit exceeded"),
                response("200 OK", b"not JSON"),
                response("200 OK", br#"[{"tag_name":"v2.0.0-main","draft":false}]"#),
                format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", MAX_METADATA + 1).into_bytes(),
            ] {
                let fixture = Fixture::new(vec![raw]);
                assert!(fixture.network.releases().is_err());
            }
            let fixture = Fixture::new(vec![response("200 OK", b"[]")]);
            let releases = fixture.network.releases().unwrap();
            assert!(matches!(select(releases, "main", "1.2.0-gpui").unwrap(), Selection::NoRelease));
        }

        #[test]
        fn rejects_insecure_redirects_but_follows_github_signed_https_assets() {
            for destination in ["http://github.com/unsafe", "https://evil.test/update.exe"] {
                let raw = format!("HTTP/1.1 302 Found\r\nLocation: {destination}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").into_bytes();
                let fixture = Fixture::new(vec![raw]);
                assert!(fixture.network.get(Url::parse(&asset("v2.0.0-main", BINARY, 1).browser_download_url).unwrap(), false).is_err());
            }
            let redirect = b"HTTP/1.1 302 Found\r\nLocation: https://release-assets.githubusercontent.com/signed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec();
            let fixture = Fixture::new(vec![redirect, response("200 OK", b"payload")]);
            let result = fixture.network.get(Url::parse(&asset("v2.0.0-main", BINARY, 7).browser_download_url).unwrap(), false).unwrap();
            assert_eq!(bounded_body(result, 7, Some(7)).unwrap(), b"payload");
        }

        #[test]
        fn complete_download_verifies_and_reports_progress_without_handoff() {
            let bytes = pe();
            let hash: [u8; 32] = Sha256::digest(&bytes).into();
            let checksum = format!("{:x}", Sha256::digest(&bytes));
            let fixture = Fixture::new(vec![
                response("200 OK", checksum.as_bytes()),
                response("200 OK", &bytes),
            ]);
            let candidate = available(vec![native("v2.0.0-main")], "main");
            let payload = TempPayload::new();
            let (sender, receiver) = flume::unbounded();
            assert_eq!(fixture.network.stage(&candidate, &payload.path, &sender).unwrap(), hash);
            let events: Vec<_> = receiver.drain().collect();
            assert!(matches!(events.first(), Some(Event::Status(Status::Downloading { received: 0, total: 512 }))));
            assert!(matches!(events.last(), Some(Event::Status(Status::Verifying))));
            assert_eq!(std::fs::read(&payload.path).unwrap(), bytes);
        }

        #[test]
        fn downloaded_corruption_wrong_sizes_and_http_errors_never_verify() {
            let valid = pe();
            let hash = format!("{:x}", Sha256::digest(&valid));
            let mut corrupt = valid.clone();
            corrupt[511] ^= 1;
            let mut oversized = valid.clone();
            oversized.push(0);
            let mut no_length = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
            no_length.extend_from_slice(&oversized);
            for payload_response in [
                response("200 OK", &corrupt),
                response("200 OK", &valid[..511]),
                response("200 OK", &oversized),
                response("404 Not Found", b"missing"),
                no_length,
            ] {
                let fixture = Fixture::new(vec![response("200 OK", hash.as_bytes()), payload_response]);
                let candidate = available(vec![native("v2.0.0-main")], "main");
                let payload = TempPayload::new();
                let (sender, _receiver) = flume::unbounded();
                assert!(fixture.network.stage(&candidate, &payload.path, &sender).is_err());
            }
        }

        #[test]
        fn malformed_or_missing_checksums_never_create_an_executable() {
            for raw in [
                response("200 OK", b"not-a-sha256"),
                response("200 OK", &vec![b'z'; 64]),
                response("404 Not Found", b"missing"),
            ] {
                let fixture = Fixture::new(vec![raw]);
                let candidate = available(vec![native("v2.0.0-main")], "main");
                let payload = TempPayload::new();
                let (sender, _receiver) = flume::unbounded();
                assert!(fixture.network.stage(&candidate, &payload.path, &sender).is_err());
                assert!(!payload.path.exists());
            }
        }

        #[test]
        fn even_a_matching_hash_cannot_authorize_non_pe_bytes() {
            let bytes = vec![b'x'; 512];
            let hash = format!("{:x}", Sha256::digest(&bytes));
            let fixture = Fixture::new(vec![
                response("200 OK", hash.as_bytes()),
                response("200 OK", &bytes),
            ]);
            let candidate = available(vec![native("v2.0.0-main")], "main");
            let payload = TempPayload::new();
            let (sender, _receiver) = flume::unbounded();
            let error = fixture.network.stage(&candidate, &payload.path, &sender).unwrap_err();
            assert!(error.to_string().contains("Windows executable"));
        }

        #[test]
        fn download_never_overwrites_an_existing_staging_file() {
            let bytes = pe();
            let fixture = Fixture::new(vec![response("200 OK", &bytes)]);
            let candidate = available(vec![native("v2.0.0-main")], "main");
            let payload = TempPayload::new();
            std::fs::write(&payload.path, b"unrelated data").unwrap();
            let (sender, _receiver) = flume::unbounded();
            assert!(fixture.network.download(&candidate, &payload.path, &sender).is_err());
            assert_eq!(std::fs::read(&payload.path).unwrap(), b"unrelated data");
        }

        #[test]
        fn native_release_selection_includes_later_metadata_pages() {
            let first = serde_json::to_vec(&vec![
                serde_json::json!({"tag_name":"legacy-main", "draft":false, "assets":[]});
                PAGE_SIZE
            ]).unwrap();
            let release = native("v2.1.0-main");
            let assets: Vec<_> = release.assets.iter().map(|asset| serde_json::json!({
                "name": asset.name,
                "size": asset.size,
                "browser_download_url": asset.browser_download_url,
            })).collect();
            let second = serde_json::to_vec(&serde_json::json!([{
                "tag_name": release.tag_name, "draft": false, "assets": assets,
            }])).unwrap();
            let fixture = Fixture::new(vec![
                response("200 OK", &first),
                response("200 OK", &second),
            ]);
            assert_eq!(available(fixture.network.releases().unwrap(), "main").tag, "v2.1.0-main");
        }

        #[test]
        fn endless_redirects_fail_at_a_finite_limit() {
            let redirect = b"HTTP/1.1 302 Found\r\nLocation: https://release-assets.githubusercontent.com/loop\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec();
            let fixture = Fixture::new(vec![redirect; 6]);
            let url = Url::parse(&asset("v2.0.0-main", BINARY, 1).browser_download_url).unwrap();
            assert!(fixture.network.get(url, false).unwrap_err().to_string().contains("Too many redirects"));
        }

        #[test]
        fn busy_statuses_cover_only_in_flight_work() {
            for status in [
                Status::Checking, Status::Downloading { received: 0, total: 1 },
                Status::Verifying, Status::Preparing, Status::Restarting,
            ] {
                assert!(status.busy());
            }
            for status in [
                Status::Unsupported, Status::Idle, Status::NoRelease, Status::UpToDate,
                Status::Available { version: "v2.0.0-main".into() },
                Status::Error("connection refused".into()),
            ] {
                assert!(!status.busy());
            }
        }
    }
}
