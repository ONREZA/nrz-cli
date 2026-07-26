//! Self-update functionality for nrz CLI

use anyhow::Context;
use clap::{Parser, ValueEnum};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::Duration;

use crate::output;

const REPO: &str = "onreza/nrz-cli";
const GITHUB_RELEASES_PER_PAGE: usize = 100;
const CHECKSUMS_ASSET_NAME: &str = "checksums-sha256.txt";
const MAX_RELEASE_ASSET_BYTES: u64 = 256 * 1024 * 1024;
const MAX_RELEASE_CHECKSUMS_BYTES: u64 = 1024 * 1024;
const MAX_RELEASE_BINARY_BYTES: u64 = 256 * 1024 * 1024;

/// Upgrade nrz to the latest version
#[derive(Parser)]
pub struct UpgradeArgs {
    /// Force upgrade even if already on latest version
    #[arg(long)]
    pub force: bool,

    /// Specific version to upgrade to (e.g., v0.1.0)
    #[arg(long)]
    pub version: Option<String>,

    /// Release channel to follow when --version is not provided
    #[arg(long, value_enum, default_value_t = UpgradeChannel::Stable)]
    pub channel: UpgradeChannel,
}
const GITHUB_API: &str = "https://api.github.com/repos";

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum UpgradeChannel {
    Stable,
    Beta,
}

impl UpgradeChannel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
        }
    }
}

#[derive(Clone, Debug)]
struct ReleaseInfo {
    tag_name: String,
    prerelease: bool,
    draft: bool,
    assets: Vec<Asset>,
}

#[derive(Clone, Debug)]
struct Asset {
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpgradeResult<'a> {
    current_version: &'a str,
    target_version: &'a str,
    release: &'a str,
    platform: &'a str,
    channel: Option<&'a str>,
    updated: bool,
    asset: Option<&'a str>,
}

/// Run the upgrade process
pub async fn run(args: UpgradeArgs, json: bool) -> anyhow::Result<()> {
    // Cleanup old update debris first
    cleanup_old_files();

    let current_version = env!("CARGO_PKG_VERSION");
    progress(json, format!("Current version: {current_version}"));

    // Detect platform
    let platform = detect_platform();
    progress(json, format!("Platform: {platform}"));

    let release = if let Some(target_version) = args.version.as_deref() {
        let target_tag = normalize_tag(target_version);
        fetch_release(&target_tag).await?
    } else {
        fetch_release_for_channel(args.channel, platform).await?
    };

    let target_version = release.tag_name.trim_start_matches('v');
    if args.version.is_some() {
        progress(json, format!("Target version: {target_version}"));
    } else {
        progress(
            json,
            format!(
                "Target version: {} ({})",
                target_version,
                args.channel.as_str()
            ),
        );
    }

    if !args.force && target_version == current_version {
        let message = "✅ Already on the requested version!";
        progress(json, message);
        if json {
            output::json_output(&UpgradeResult {
                current_version,
                target_version,
                release: &release.tag_name,
                platform,
                channel: args.version.is_none().then(|| args.channel.as_str()),
                updated: false,
                asset: None,
            });
        }
        return Ok(());
    }

    // Find matching asset
    let asset_name = format!("nrz-{}.tar.gz", platform);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| anyhow::anyhow!("No binary available for platform: {}", platform))?;
    release
        .assets
        .iter()
        .find(|a| a.name == CHECKSUMS_ASSET_NAME)
        .ok_or_else(|| anyhow::anyhow!("Release {} has no checksum manifest", release.tag_name))?;

    let archive_url = release_asset_url(&release.tag_name, &asset.name);
    let checksums_url = release_asset_url(&release.tag_name, CHECKSUMS_ASSET_NAME);
    progress(
        json,
        format!("Downloading {} ({})...", asset.name, release.tag_name),
    );

    // Download and extract binary from tar.gz
    let client = release_http_client()?;
    let archive_data =
        download_release_asset(&client, &archive_url, &asset.name, MAX_RELEASE_ASSET_BYTES).await?;
    let checksums = download_release_asset(
        &client,
        &checksums_url,
        CHECKSUMS_ASSET_NAME,
        MAX_RELEASE_CHECKSUMS_BYTES,
    )
    .await?;
    verify_release_checksum(&asset.name, &archive_data, &checksums)?;
    progress(json, "✅ Checksum verified");
    let new_binary = extract_binary_from_tar_gz(&archive_data)?;

    // Replace current binary
    replace_binary(&new_binary).await?;

    #[cfg(windows)]
    progress(
        json,
        "⚠️  Please restart your terminal to use the new version",
    );

    progress(
        json,
        format!("✅ Successfully upgraded to {}!", release.tag_name),
    );
    if json {
        output::json_output(&UpgradeResult {
            current_version,
            target_version,
            release: &release.tag_name,
            platform,
            channel: args.version.is_none().then(|| args.channel.as_str()),
            updated: true,
            asset: Some(&asset.name),
        });
    }
    Ok(())
}

fn progress(json: bool, message: impl std::fmt::Display) {
    let message = message.to_string();
    if json {
        output::log_line("user", "info", "upgrade", &message);
    } else {
        eprintln!("{message}");
    }
}

fn normalize_tag(version: &str) -> String {
    if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{}", version)
    }
}

/// Detect current platform for binary selection
fn detect_platform() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-x64";

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "darwin-x64";

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "darwin-arm64";

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "win32-x64";

    #[allow(unreachable_code)]
    {
        panic!("Unsupported platform")
    }
}

/// Fetch one releases page from GitHub API.
async fn fetch_releases_page(
    client: &reqwest::Client,
    page: u32,
) -> anyhow::Result<Vec<ReleaseInfo>> {
    let url = releases_url(page);
    let response = client
        .get(&url)
        .header("User-Agent", "nrz-cli")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch releases: {}", response.status());
    }

    let json: serde_json::Value = response.json().await?;
    parse_releases(&json)
}

fn releases_url(page: u32) -> String {
    format!(
        "{}/{}/releases?per_page={}&page={}",
        GITHUB_API, REPO, GITHUB_RELEASES_PER_PAGE, page
    )
}

fn parse_releases(json: &serde_json::Value) -> anyhow::Result<Vec<ReleaseInfo>> {
    let releases = json
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid releases response"))?;
    releases.iter().map(parse_release_info).collect()
}

fn parse_release_info(json: &serde_json::Value) -> anyhow::Result<ReleaseInfo> {
    let tag_name = json["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid release response"))?
        .to_string();
    let prerelease = json["prerelease"].as_bool().unwrap_or(false);
    let draft = json["draft"].as_bool().unwrap_or(false);

    let assets: Vec<Asset> = json["assets"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid assets response"))?
        .iter()
        .filter_map(|a| {
            Some(Asset {
                name: a["name"].as_str()?.to_string(),
            })
        })
        .collect();

    Ok(ReleaseInfo {
        tag_name,
        prerelease,
        draft,
        assets,
    })
}

async fn fetch_release_for_channel(
    channel: UpgradeChannel,
    platform: &str,
) -> anyhow::Result<ReleaseInfo> {
    let client = release_http_client()?;
    let mut page = 1;

    loop {
        let releases = fetch_releases_page(&client, page).await?;
        if let Some(release) = select_release_for_channel(&releases, channel, platform) {
            return Ok(release.clone());
        }
        if !release_page_may_have_next(&releases) {
            break;
        }
        page += 1;
    }

    anyhow::bail!(
        "No complete {} release available for platform: {}",
        channel.as_str(),
        platform
    )
}

fn release_page_may_have_next(releases: &[ReleaseInfo]) -> bool {
    releases.len() == GITHUB_RELEASES_PER_PAGE
}

fn select_release_for_channel<'a>(
    releases: &'a [ReleaseInfo],
    channel: UpgradeChannel,
    platform: &str,
) -> Option<&'a ReleaseInfo> {
    let asset_name = format!("nrz-{platform}.tar.gz");
    releases.iter().find(|release| {
        release_matches_channel(release, channel)
            && release.assets.iter().any(|asset| asset.name == asset_name)
            && release
                .assets
                .iter()
                .any(|asset| asset.name == CHECKSUMS_ASSET_NAME)
    })
}

fn release_matches_channel(release: &ReleaseInfo, channel: UpgradeChannel) -> bool {
    if release.draft {
        return false;
    }
    if channel == UpgradeChannel::Stable {
        return !release.prerelease;
    }
    if !release.prerelease {
        return false;
    }
    release_prerelease_channel(&release.tag_name) == Some(channel.as_str())
}

fn release_prerelease_channel(tag: &str) -> Option<&str> {
    let prerelease = tag.trim_start_matches('v').split_once('-')?.1;
    Some(prerelease.split('.').next().unwrap_or(prerelease))
}

/// Fetch specific release by tag
async fn fetch_release(tag: &str) -> anyhow::Result<ReleaseInfo> {
    let encoded_tag = utf8_percent_encode(tag, NON_ALPHANUMERIC);
    let url = format!("{}/{}/releases/tags/{}", GITHUB_API, REPO, encoded_tag);
    let client = release_http_client()?;
    let response = client
        .get(&url)
        .header("User-Agent", "nrz-cli")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Release {} not found", tag);
    }

    let json: serde_json::Value = response.json().await?;
    parse_release_info(&json)
}

fn release_asset_url(tag: &str, asset_name: &str) -> String {
    let encoded_tag = utf8_percent_encode(tag, NON_ALPHANUMERIC);
    let encoded_asset = utf8_percent_encode(asset_name, NON_ALPHANUMERIC);
    format!("https://github.com/{REPO}/releases/download/{encoded_tag}/{encoded_asset}")
}

fn verify_release_checksum(
    asset_name: &str,
    archive_data: &[u8],
    checksum_data: &[u8],
) -> anyhow::Result<()> {
    let checksums = std::str::from_utf8(checksum_data)
        .map_err(|_| anyhow::anyhow!("Release checksum manifest is not valid UTF-8"))?;
    let mut matches = checksums.lines().filter_map(|line| {
        let mut fields = line.split_whitespace();
        let digest = fields.next()?;
        let name = fields.next()?.trim_start_matches('*');
        if fields.next().is_some()
            || name != asset_name
            || digest.len() != 64
            || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return None;
        }
        Some(digest)
    });
    let expected = matches
        .next()
        .ok_or_else(|| anyhow::anyhow!("Checksum for {asset_name} not found"))?;
    if matches.next().is_some() {
        anyhow::bail!("Checksum manifest contains duplicate entries for {asset_name}");
    }

    let actual = sha256_hex(archive_data);
    if !actual.eq_ignore_ascii_case(expected) {
        anyhow::bail!(
            "Checksum verification failed for {asset_name}: expected {expected}, got {actual}"
        );
    }
    Ok(())
}

fn sha256_hex(data: &[u8]) -> String {
    use std::fmt::Write;

    Sha256::digest(data)
        .iter()
        .fold(String::with_capacity(64), |mut encoded, byte| {
            write!(encoded, "{byte:02x}").expect("writing to String cannot fail");
            encoded
        })
}

/// Extract binary from tar.gz archive
fn extract_binary_from_tar_gz(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let decoder = GzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);

    let binary_name = if cfg!(windows) { "nrz.exe" } else { "nrz" };

    for entry in archive.entries()? {
        let entry = entry?;
        let path = entry.path()?;
        if path.file_name().and_then(|n| n.to_str()) == Some(binary_name)
            && entry.header().entry_type().is_file()
        {
            if entry.size() > MAX_RELEASE_BINARY_BYTES {
                anyhow::bail!("Binary in release archive exceeds the extraction limit");
            }
            let mut buf = Vec::new();
            entry
                .take(MAX_RELEASE_BINARY_BYTES + 1)
                .read_to_end(&mut buf)?;
            if buf.len() as u64 > MAX_RELEASE_BINARY_BYTES {
                anyhow::bail!("Binary in release archive exceeds the extraction limit");
            }
            return Ok(buf);
        }
    }

    anyhow::bail!("Binary '{}' not found in archive", binary_name)
}

fn release_http_client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::limited(5))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(Into::into)
}

async fn download_release_asset(
    client: &reqwest::Client,
    url: &str,
    asset_name: &str,
    max_bytes: u64,
) -> anyhow::Result<Vec<u8>> {
    let mut response = client
        .get(url)
        .header("User-Agent", "nrz-cli")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to download release asset {asset_name}: {}",
            response.status()
        );
    }
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes)
    {
        anyhow::bail!("Release asset {asset_name} exceeds the download limit");
    }

    let capacity = response.content_length().unwrap_or_default().min(max_bytes) as usize;
    let mut bytes = Vec::with_capacity(capacity);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| anyhow::anyhow!("Failed to read release asset {asset_name}: {error}"))?
    {
        let next_len = bytes
            .len()
            .checked_add(chunk.len())
            .ok_or_else(|| anyhow::anyhow!("Release asset size overflow"))?;
        if next_len as u64 > max_bytes {
            anyhow::bail!("Release asset {asset_name} exceeds the download limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// Replace current binary with new one
async fn replace_binary(new_binary: &[u8]) -> anyhow::Result<()> {
    let current_exe = std::env::current_exe()?;
    replace_binary_at(&current_exe, new_binary)
}

fn write_update_candidate(
    current_exe: &std::path::Path,
    new_binary: &[u8],
) -> anyhow::Result<std::path::PathBuf> {
    use std::io::Write;

    let directory = current_exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("current executable has no parent directory"))?;
    let file_name = current_exe
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("current executable has no file name"))?
        .to_string_lossy();
    let candidate = directory.join(format!(
        ".{file_name}.update-{}.tmp",
        uuid::Uuid::now_v7().simple()
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o700);
    }
    let mut file = options
        .open(&candidate)
        .with_context(|| format!("failed to create update candidate {}", candidate.display()))?;
    if let Err(error) = file.write_all(new_binary).and_then(|_| file.sync_all()) {
        drop(file);
        let _ = std::fs::remove_file(&candidate);
        return Err(error).context("failed to write update candidate");
    }
    Ok(candidate)
}

fn replace_binary_at(current_exe: &std::path::Path, new_binary: &[u8]) -> anyhow::Result<()> {
    let candidate = write_update_candidate(current_exe, new_binary)?;

    #[cfg(windows)]
    {
        // On Windows, we can't replace a running executable
        // So we rename the old one, write the new one, and schedule deletion of old
        let old_exe = current_exe.with_extension("old.exe");

        // Rename current to .old
        if let Err(error) = std::fs::rename(current_exe, &old_exe) {
            let _ = std::fs::remove_file(&candidate);
            return Err(error).context("failed to move current executable aside");
        }

        // Rename new to current
        if let Err(error) = std::fs::rename(&candidate, current_exe) {
            let restore = std::fs::rename(&old_exe, current_exe);
            let _ = std::fs::remove_file(&candidate);
            if let Err(restore_error) = restore {
                anyhow::bail!(
                    "failed to install update ({error}) and failed to restore previous executable ({restore_error})"
                );
            }
            return Err(error).context("failed to install update candidate");
        }

        // Try to delete old (may fail if running, will be cleaned up on next run)
        let _ = std::fs::remove_file(&old_exe);
    }

    #[cfg(not(windows))]
    {
        let install_result = (|| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&candidate)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&candidate, perms)?;
            }
            std::fs::rename(&candidate, current_exe)
        })();
        if let Err(error) = install_result {
            let _ = std::fs::remove_file(&candidate);
            return Err(error).context("failed to install update candidate");
        }
    }

    Ok(())
}

/// Cleanup old update debris from previous runs
fn cleanup_old_files() {
    let Ok(current_exe) = std::env::current_exe() else {
        return;
    };
    let Some(exe_dir) = current_exe.parent() else {
        return;
    };
    let Some(exe_name) = current_exe.file_stem() else {
        return;
    };
    let exe_name = exe_name.to_string_lossy();

    // Cleanup patterns: .old.exe, .new.exe, .tmp
    let patterns: Vec<Box<dyn Fn() -> std::path::PathBuf>> = vec![
        #[cfg(windows)]
        Box::new(|| exe_dir.join(format!("{}.old.exe", exe_name))),
        #[cfg(windows)]
        Box::new(|| exe_dir.join(format!("{}.new.exe", exe_name))),
        #[cfg(not(windows))]
        Box::new(|| exe_dir.join(format!("{}.tmp", exe_name))),
    ];

    for path_fn in patterns {
        let path = path_fn();
        if path.exists() {
            tracing::debug!("Cleaning up old file: {}", path.display());
            let _ = std::fs::remove_file(&path);
        }
    }
}

#[cfg(test)]
mod upgrade_tests;
