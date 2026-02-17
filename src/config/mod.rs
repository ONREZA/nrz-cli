//! Project configuration from `onreza.toml`.

#[cfg(test)]
mod config_tests;

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

/// Top-level config loaded from `onreza.toml`.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ProjectConfig {
    pub project: ProjectSection,
    pub dev: DevSection,
    pub build: BuildSection,
    pub deploy: DeploySection,
    pub migrations: MigrationsSection,
    pub db: DbSection,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ProjectSection {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DevSection {
    pub command: Option<String>,
    pub port: Option<u16>,
    pub host: Option<String>,

    pub data_dir: Option<String>,
    pub db_name: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct BuildSection {
    pub output_dirs: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DeploySection {
    pub skip_migrations: Option<bool>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct MigrationsSection {
    pub dir: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DbSection {
    pub default_env: Option<String>,
}

// ── Accessor methods with defaults ──────────────────────────

impl ProjectConfig {
    pub fn dev_port(&self) -> u16 {
        self.dev.port.unwrap_or(4321)
    }

    pub fn dev_host(&self) -> &str {
        self.dev.host.as_deref().unwrap_or("127.0.0.1")
    }

    pub fn data_dir_relative(&self) -> &str {
        self.dev.data_dir.as_deref().unwrap_or(".onreza/data")
    }

    pub fn data_dir_path(&self, project_dir: &Path) -> PathBuf {
        project_dir.join(self.data_dir_relative())
    }

    pub fn db_name(&self) -> &str {
        self.dev.db_name.as_deref().unwrap_or("dev.db")
    }

    pub fn output_dirs(&self) -> Vec<&str> {
        match &self.build.output_dirs {
            Some(dirs) => dirs.iter().map(|s| s.as_str()).collect(),
            None => vec!["dist", ".output", "build"],
        }
    }

    pub fn skip_migrations(&self) -> bool {
        self.deploy.skip_migrations.unwrap_or(false)
    }

    pub fn migrations_dir(&self) -> &str {
        self.migrations.dir.as_deref().unwrap_or("migrations")
    }
}

// ── Load / Save ─────────────────────────────────────────────

const CONFIG_FILENAME: &str = "onreza.toml";

/// Load `onreza.toml` from project directory. Returns `Default` if file not found.
pub fn load(project_dir: &Path) -> anyhow::Result<ProjectConfig> {
    let path = project_dir.join(CONFIG_FILENAME);
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let config: ProjectConfig = toml::from_str(&content)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            Ok(config)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ProjectConfig::default()),
        Err(e) => Err(anyhow::anyhow!("failed to read {}: {e}", path.display())),
    }
}

/// Generate a template `onreza.toml` with commented-out defaults.
pub fn generate_template(project_id: &str) -> String {
    format!(
        r#"[project]
id = "{project_id}"
# name = ""

# [dev]
# command = ""
# port = 4321
# host = "127.0.0.1"

# data_dir = ".onreza/data"
# db_name = "dev.db"

# [build]
# output_dirs = ["dist", ".output", "build"]

# [deploy]
# skip_migrations = false

# [migrations]
# dir = "migrations"

# [db]
# default_env = "development"
"#
    )
}

/// Create or update `onreza.toml`.
///
/// If the file exists, only update `[project] id` preserving other content.
/// If the file doesn't exist, generate from template.
pub fn save_or_update(project_dir: &Path, project_id: &str) -> anyhow::Result<()> {
    let path = project_dir.join(CONFIG_FILENAME);

    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        let config: ProjectConfig = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;

        if config.project.id.as_deref() == Some(project_id) {
            return Ok(());
        }

        // Update id in-place preserving comments and formatting
        let updated = update_project_id_in_toml(&content, project_id);
        std::fs::write(&path, updated)
            .with_context(|| format!("failed to write {}", path.display()))?;
    } else {
        let content = generate_template(project_id);
        std::fs::write(&path, content)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    Ok(())
}

/// Try to update just the `id` field under `[project]` section, preserving
/// the rest of the file content including comments.
fn update_project_id_in_toml(content: &str, new_id: &str) -> String {
    let mut result = String::new();
    let mut in_project_section = false;
    let mut id_replaced = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect section headers
        if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
            in_project_section = trimmed == "[project]";
        }

        if in_project_section && !id_replaced && trimmed.starts_with("id") {
            // Check it's actually `id = "..."` not `id_something`
            if let Some(rest) = trimmed.strip_prefix("id") {
                let rest = rest.trim_start();
                if rest.starts_with('=') {
                    result.push_str(&format!("id = \"{new_id}\""));
                    result.push('\n');
                    id_replaced = true;
                    continue;
                }
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    // If we didn't find an id line to replace, insert it after [project]
    // or prepend [project] section if it doesn't exist
    if !id_replaced {
        let has_project_section = result.lines().any(|l| l.trim() == "[project]");
        if !has_project_section {
            return format!("[project]\nid = \"{new_id}\"\n\n{result}");
        }
        let mut final_result = String::new();
        for line in result.lines() {
            final_result.push_str(line);
            final_result.push('\n');
            if line.trim() == "[project]" {
                final_result.push_str(&format!("id = \"{new_id}\"\n"));
            }
        }
        return final_result;
    }

    result
}
