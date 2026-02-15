use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub token: String,
    pub name: String,
}

impl std::fmt::Debug for WorkspaceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceInfo")
            .field("token", &"***")
            .field("name", &self.name)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub workspaces: BTreeMap<String, WorkspaceInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_workspace: Option<String>,
}

impl WorkspaceConfig {
    pub fn empty() -> Self {
        Self {
            workspaces: BTreeMap::new(),
            default_workspace: None,
        }
    }

    pub fn add_workspace(&mut self, slug: &str, token: String, name: String) {
        let key = if slug.is_empty() {
            "personal".to_string()
        } else {
            slug.to_string()
        };
        self.workspaces
            .insert(key.clone(), WorkspaceInfo { token, name });
        if self.workspaces.len() == 1 {
            self.default_workspace = Some(key);
        }
    }

    pub fn remove_workspace(&mut self, slug: &str) {
        self.workspaces.remove(slug);
        if self.default_workspace.as_deref() == Some(slug) {
            self.default_workspace = if self.workspaces.len() == 1 {
                self.workspaces.keys().next().cloned()
            } else {
                None
            };
        }
    }
}

pub fn load() -> WorkspaceConfig {
    load_from(&config_path(), &legacy_credentials_path())
}

pub fn save(config: &WorkspaceConfig) -> anyhow::Result<()> {
    save_to(&config_path(), config)
}

pub(crate) fn load_from(config_path: &Path, legacy_path: &Path) -> WorkspaceConfig {
    // Try new config first
    if let Ok(content) = std::fs::read_to_string(config_path)
        && let Ok(config) = serde_json::from_str::<WorkspaceConfig>(&content)
    {
        return config;
    }

    // Try migrating from legacy credentials.json
    if let Ok(content) = std::fs::read_to_string(legacy_path)
        && let Ok(old) = serde_json::from_str::<LegacyCredentials>(&content)
    {
        let mut config = WorkspaceConfig::empty();
        let slug = if old.workspace_slug.is_empty() {
            "personal".to_string()
        } else {
            old.workspace_slug
        };
        config.add_workspace(&slug, old.access_token, old.workspace_name);

        // Save new format and remove old
        if save_to(config_path, &config).is_ok() {
            let _ = std::fs::remove_file(legacy_path);
        }
        return config;
    }

    WorkspaceConfig::empty()
}

pub(crate) fn save_to(path: &Path, config: &WorkspaceConfig) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(config).context("failed to serialize config")?;
    std::fs::write(path, &json).with_context(|| format!("failed to write {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("failed to set permissions on {}", path.display()))?;
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct LegacyCredentials {
    access_token: String,
    workspace_slug: String,
    workspace_name: String,
}

fn config_path() -> PathBuf {
    config_dir().join("nrz").join("config.json")
}

fn legacy_credentials_path() -> PathBuf {
    config_dir().join("nrz").join("credentials.json")
}

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
        .expect("HOME or USERPROFILE environment variable must be set")
}
