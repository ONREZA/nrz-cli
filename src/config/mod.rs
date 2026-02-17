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
    pub workspace: Option<String>,
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
///
/// If `project_id` is `None`, the template is created without a project ID
/// (local-only scaffold before platform linking).
pub fn generate_template(
    project_id: Option<&str>,
    project_name: Option<&str>,
    workspace_slug: Option<&str>,
) -> String {
    let mut project_lines = String::from(
        "#:schema https://raw.githubusercontent.com/onreza/nrz-cli/main/onreza.schema.json\n\n[project]\n",
    );
    if let Some(id) = project_id {
        let id = escape_toml_value(id);
        project_lines.push_str(&format!("id = \"{id}\"\n"));
    } else {
        project_lines.push_str("# id = \"\"\n");
    }
    if let Some(name) = project_name {
        let name = escape_toml_value(name);
        project_lines.push_str(&format!("name = \"{name}\"\n"));
    } else {
        project_lines.push_str("# name = \"\"\n");
    }
    if let Some(ws) = workspace_slug {
        let ws = escape_toml_value(ws);
        project_lines.push_str(&format!("workspace = \"{ws}\"\n"));
    }

    format!(
        r#"{project_lines}
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
/// If the file exists, update `[project]` fields in-place preserving other content.
/// If the file doesn't exist, generate from template.
pub fn save_or_update(
    project_dir: &Path,
    project_id: &str,
    project_name: Option<&str>,
    workspace_slug: Option<&str>,
) -> anyhow::Result<()> {
    let path = project_dir.join(CONFIG_FILENAME);

    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        let config: ProjectConfig = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;

        let id_matches = config.project.id.as_deref() == Some(project_id);
        let name_matches = project_name.is_none() || config.project.name.as_deref() == project_name;
        let ws_matches =
            workspace_slug.is_none() || config.project.workspace.as_deref() == workspace_slug;

        if id_matches && name_matches && ws_matches {
            return Ok(());
        }

        // Update fields in-place preserving comments and formatting
        let updated =
            update_project_fields_in_toml(&content, project_id, project_name, workspace_slug);
        std::fs::write(&path, updated)
            .with_context(|| format!("failed to write {}", path.display()))?;
    } else {
        let content = generate_template(Some(project_id), project_name, workspace_slug);
        std::fs::write(&path, content)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    Ok(())
}

/// Resolve project ID from explicit flag, config, or fail.
pub fn resolve_project_id(
    explicit: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<String> {
    if let Some(id) = explicit
        && !id.is_empty()
    {
        return Ok(id.to_string());
    }

    if let Some(id) = &config.project.id
        && !id.is_empty()
    {
        return Ok(id.clone());
    }

    anyhow::bail!(
        "no project specified. Use --project-id, set [project] id in onreza.toml, or run `nrz link` first."
    );
}

/// Update `[project]` fields in-place, preserving the rest of the file content
/// including comments.
fn update_project_fields_in_toml(
    content: &str,
    new_id: &str,
    new_name: Option<&str>,
    new_workspace: Option<&str>,
) -> String {
    let escaped_id = escape_toml_value(new_id);
    let escaped_name = new_name.map(escape_toml_value);
    let escaped_ws = new_workspace.map(escape_toml_value);

    let mut result = String::new();
    let mut in_project_section = false;
    let mut id_replaced = false;
    let mut name_replaced = false;
    let mut workspace_replaced = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect section headers
        if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
            // Before leaving [project] section, insert any missing fields
            if in_project_section {
                if !id_replaced {
                    result.push_str(&format!("id = \"{escaped_id}\"\n"));
                    id_replaced = true;
                }
                if !name_replaced && let Some(ref name) = escaped_name {
                    result.push_str(&format!("name = \"{name}\"\n"));
                    name_replaced = true;
                }
                if !workspace_replaced && let Some(ref ws) = escaped_ws {
                    result.push_str(&format!("workspace = \"{ws}\"\n"));
                    workspace_replaced = true;
                }
            }
            in_project_section = trimmed == "[project]";
        }

        if in_project_section {
            if let Some(replaced) = try_replace_field(trimmed, "id", &escaped_id)
                && !id_replaced
            {
                result.push_str(&replaced);
                result.push('\n');
                id_replaced = true;
                continue;
            }
            if let Some(ref name) = escaped_name
                && let Some(replaced) = try_replace_field(trimmed, "name", name)
                && !name_replaced
            {
                result.push_str(&replaced);
                result.push('\n');
                name_replaced = true;
                continue;
            }
            if let Some(ref ws) = escaped_ws
                && let Some(replaced) = try_replace_field(trimmed, "workspace", ws)
                && !workspace_replaced
            {
                result.push_str(&replaced);
                result.push('\n');
                workspace_replaced = true;
                continue;
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    // Handle end-of-file: if still in [project] section, insert missing fields
    if in_project_section {
        if !id_replaced {
            result.push_str(&format!("id = \"{escaped_id}\"\n"));
            id_replaced = true;
        }
        if !name_replaced && let Some(ref name) = escaped_name {
            result.push_str(&format!("name = \"{name}\"\n"));
        }
        if !workspace_replaced && let Some(ref ws) = escaped_ws {
            result.push_str(&format!("workspace = \"{ws}\"\n"));
        }
    }

    // If we never found [project] section
    if !id_replaced {
        let has_project_section = result.lines().any(|l| l.trim() == "[project]");
        if !has_project_section {
            let mut header = format!("[project]\nid = \"{escaped_id}\"\n");
            if let Some(ref name) = escaped_name {
                header.push_str(&format!("name = \"{name}\"\n"));
            }
            if let Some(ref ws) = escaped_ws {
                header.push_str(&format!("workspace = \"{ws}\"\n"));
            }
            header.push('\n');
            return format!("{header}{result}");
        }
        let mut final_result = String::new();
        for line in result.lines() {
            final_result.push_str(line);
            final_result.push('\n');
            if line.trim() == "[project]" {
                final_result.push_str(&format!("id = \"{escaped_id}\"\n"));
                if let Some(ref name) = escaped_name {
                    final_result.push_str(&format!("name = \"{name}\"\n"));
                }
                if let Some(ref ws) = escaped_ws {
                    final_result.push_str(&format!("workspace = \"{ws}\"\n"));
                }
            }
        }
        return final_result;
    }

    result
}

/// Try to match and replace a `key = "..."` or `# key = "..."` line.
/// Returns `Some(replacement)` if matched (handles both active and commented-out lines).
fn try_replace_field(trimmed: &str, key: &str, value: &str) -> Option<String> {
    // Strip leading `# ` for commented-out lines
    let effective = trimmed
        .strip_prefix('#')
        .map(|s| s.trim_start())
        .unwrap_or(trimmed);

    if !effective.starts_with(key) {
        return None;
    }
    let rest = effective.strip_prefix(key)?;
    let rest = rest.trim_start();
    if rest.starts_with('=') {
        Some(format!("{key} = \"{value}\""))
    } else {
        None
    }
}

/// Escape a string value for safe TOML insertion.
fn escape_toml_value(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
