use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub access_token: String,
    pub workspace_slug: String,
    pub workspace_name: String,
}

pub fn load() -> Option<Credentials> {
    load_from(&credentials_path())
}

pub fn save(creds: &Credentials) -> anyhow::Result<()> {
    save_to(&credentials_path(), creds)
}

pub fn remove() -> anyhow::Result<()> {
    remove_at(&credentials_path())
}

pub(crate) fn load_from(path: &Path) -> Option<Credentials> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub(crate) fn save_to(path: &Path, creds: &Credentials) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(creds).context("failed to serialize credentials")?;
    std::fs::write(path, &json).with_context(|| format!("failed to write {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("failed to set permissions on {}", path.display()))?;
    }

    Ok(())
}

pub(crate) fn remove_at(path: &Path) -> anyhow::Result<()> {
    if path.exists() {
        std::fs::remove_file(path)
            .with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

fn credentials_path() -> PathBuf {
    config_dir().join("nrz").join("credentials.json")
}

/// Platform-aware config directory:
/// - Linux: $XDG_CONFIG_HOME or ~/.config
/// - macOS: ~/.config (same convention as Linux CLI tools)
/// - Windows: %APPDATA% (e.g. C:\Users\X\AppData\Roaming)
fn config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata);
        }
    }

    #[cfg(not(windows))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg);
        }
    }

    home_dir().join(".config")
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
