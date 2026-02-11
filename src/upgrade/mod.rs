//! Self-update functionality for nrz CLI

use clap::Parser;

const REPO: &str = "onreza/nrz-cli";

/// Upgrade nrz to the latest version
#[derive(Parser)]
pub struct UpgradeArgs {
    /// Force upgrade even if already on latest version
    #[arg(long)]
    pub force: bool,

    /// Specific version to upgrade to (e.g., v0.1.0)
    #[arg(long)]
    pub version: Option<String>,
}
const GITHUB_API: &str = "https://api.github.com/repos";

#[derive(Debug)]
struct ReleaseInfo {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Debug)]
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

    // Get latest release info
    let release = fetch_latest_release().await?;
    let latest_version = release.tag_name.trim_start_matches('v');

    eprintln!("Latest version: {}", latest_version);

    // Check if upgrade is needed
    let target_version = args.version.as_deref().unwrap_or(latest_version);
    let target_tag = if target_version.starts_with('v') {
        target_version.to_string()
    } else {
        format!("v{}", target_version)
    };

    if !args.force && target_version == current_version {
        eprintln!("✅ Already on the latest version!");
        return Ok(());
    }

    // If specific version requested, fetch that release
    let release = if args.version.is_some() && target_tag != release.tag_name {
        fetch_release(&target_tag).await?
    } else {
        release
    };

    // Detect platform
    let platform = detect_platform();
    eprintln!("Platform: {}", platform);

    // Find matching asset
    let asset_name = format!("nrz-{}", platform);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| anyhow::anyhow!("No binary available for platform: {}", platform))?;

    eprintln!("Downloading {}...", asset.browser_download_url);

    // Download new binary
    let new_binary = download_binary(&asset.browser_download_url).await?;

    // Replace current binary
    replace_binary(&new_binary).await?;

    eprintln!("✅ Successfully upgraded to {}!", target_tag);
    Ok(())
}

/// Detect current platform for binary selection
fn detect_platform() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-x64";

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "macos-x64";

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "macos-arm64";

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "windows-x64.exe";

    #[allow(unreachable_code)]
    {
        panic!("Unsupported platform")
    }
}

/// Fetch latest release from GitHub API
async fn fetch_latest_release() -> anyhow::Result<ReleaseInfo> {
    let url = format!("{}/{}/releases/latest", GITHUB_API, REPO);
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "nrz-cli")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch release info: {}", response.status());
    }

    let json: serde_json::Value = response.json().await?;

    let tag_name = json["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid release response"))?
        .to_string();

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

    Ok(ReleaseInfo { tag_name, assets })
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

    let tag_name = json["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid release response"))?
        .to_string();

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

    Ok(ReleaseInfo { tag_name, assets })
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
    let Ok(current_exe) = std::env::current_exe() else { return };
    let Some(exe_dir) = current_exe.parent() else { return };
    let Some(exe_name) = current_exe.file_stem() else { return };
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
