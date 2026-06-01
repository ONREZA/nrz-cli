//! Self-update functionality for nrz CLI

use clap::{Parser, ValueEnum};

const REPO: &str = "onreza/nrz-cli";
const GITHUB_RELEASES_PER_PAGE: usize = 100;

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
    Rc,
    Canary,
}

impl UpgradeChannel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Rc => "rc",
            Self::Canary => "canary",
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
    browser_download_url: String,
}

/// Run the upgrade process
pub async fn run(args: UpgradeArgs) -> anyhow::Result<()> {
    // Cleanup old update debris first
    cleanup_old_files();

    let current_version = env!("CARGO_PKG_VERSION");
    eprintln!("Current version: {}", current_version);

    // Detect platform
    let platform = detect_platform();
    eprintln!("Platform: {}", platform);

    let release = if let Some(target_version) = args.version.as_deref() {
        let target_tag = normalize_tag(target_version);
        fetch_release(&target_tag).await?
    } else {
        fetch_release_for_channel(args.channel, platform).await?
    };

    let target_version = release.tag_name.trim_start_matches('v');
    eprintln!(
        "Target version: {} ({})",
        target_version,
        args.channel.as_str()
    );

    if !args.force && target_version == current_version {
        eprintln!("✅ Already on the requested version!");
        return Ok(());
    }

    // Find matching asset
    let asset_name = format!("nrz-{}.tar.gz", platform);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| anyhow::anyhow!("No binary available for platform: {}", platform))?;

    eprintln!("Downloading {}...", asset.browser_download_url);

    // Download and extract binary from tar.gz
    let archive_data = download_binary(&asset.browser_download_url).await?;
    let new_binary = extract_binary_from_tar_gz(&archive_data)?;

    // Replace current binary
    replace_binary(&new_binary).await?;

    eprintln!("✅ Successfully upgraded to {}!", release.tag_name);
    Ok(())
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
                browser_download_url: a["browser_download_url"].as_str()?.to_string(),
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
    let client = reqwest::Client::new();
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
    let url = format!("{}/{}/releases/tags/{}", GITHUB_API, REPO, tag);
    let client = reqwest::Client::new();
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

/// Extract binary from tar.gz archive
fn extract_binary_from_tar_gz(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let decoder = GzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);

    let binary_name = if cfg!(windows) { "nrz.exe" } else { "nrz" };

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if path.file_name().and_then(|n| n.to_str()) == Some(binary_name) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }

    anyhow::bail!("Binary '{}' not found in archive", binary_name)
}

/// Download binary from URL
async fn download_binary(url: &str) -> anyhow::Result<Vec<u8>> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "nrz-cli")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download binary: {}", response.status());
    }

    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

/// Replace current binary with new one
async fn replace_binary(new_binary: &[u8]) -> anyhow::Result<()> {
    let current_exe = std::env::current_exe()?;

    #[cfg(windows)]
    {
        // On Windows, we can't replace a running executable
        // So we rename the old one, write the new one, and schedule deletion of old
        let old_exe = current_exe.with_extension("old.exe");
        let new_exe = current_exe.with_extension("new.exe");

        // Write new binary to temp location
        std::fs::write(&new_exe, new_binary)?;

        // Rename current to .old
        std::fs::rename(&current_exe, &old_exe)?;

        // Rename new to current
        std::fs::rename(&new_exe, &current_exe)?;

        // Try to delete old (may fail if running, will be cleaned up on next run)
        let _ = std::fs::remove_file(&old_exe);

        eprintln!("⚠️  Please restart your terminal to use the new version");
    }

    #[cfg(not(windows))]
    {
        // On Unix, we can replace the binary directly
        // Write to temp file first
        let temp_path = current_exe.with_extension("tmp");
        std::fs::write(&temp_path, new_binary)?;

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&temp_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&temp_path, perms)?;
        }

        // Atomic rename
        std::fs::rename(&temp_path, &current_exe)?;
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
