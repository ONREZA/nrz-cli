use std::path::Path;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectRef {
    pub project_id: String,
    pub project_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_slug: Option<String>,
}

pub fn load(project_dir: &Path) -> anyhow::Result<Option<ProjectRef>> {
    let path = project_dir.join(".onreza/project.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let pref = serde_json::from_str(&content)
                .with_context(|| format!("corrupt project link file: {}", path.display()))?;
            Ok(Some(pref))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(anyhow::anyhow!("failed to read {}: {e}", path.display())),
    }
}

pub fn save(project_dir: &Path, project_ref: &ProjectRef) -> anyhow::Result<()> {
    let dir = project_dir.join(".onreza");
    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let path = dir.join("project.json");
    let json = serde_json::to_string_pretty(project_ref)?;
    std::fs::write(&path, &json).with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}

pub fn resolve_project_id(explicit: Option<&str>) -> anyhow::Result<String> {
    if let Some(id) = explicit {
        return Ok(id.to_string());
    }

    let cwd = std::env::current_dir().context("failed to get current directory")?;
    if let Some(pref) = load(&cwd)? {
        return Ok(pref.project_id);
    }

    bail!("no project specified. Use --project-id or run `nrz link` first.");
}
