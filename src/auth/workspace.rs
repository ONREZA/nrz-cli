use anyhow::bail;

use super::config;

#[derive(Clone)]
pub struct WorkspaceContext {
    pub token: String,
    pub workspace_slug: String,
}

impl std::fmt::Debug for WorkspaceContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceContext")
            .field("token", &"***")
            .field("workspace_slug", &self.workspace_slug)
            .finish()
    }
}

/// Resolve workspace context.
///
/// Priority: --token → --workspace → .onreza/project.json workspace → default_workspace → only workspace → error
pub fn resolve_workspace_context(
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<WorkspaceContext> {
    let cfg = config::load();
    let project_ws = resolve_project_workspace();
    resolve_workspace_context_with_config(token, workspace, &cfg, project_ws.as_deref())
}

pub(crate) fn resolve_workspace_context_with_config(
    token: Option<&str>,
    workspace: Option<&str>,
    cfg: &config::WorkspaceConfig,
    project_workspace: Option<&str>,
) -> anyhow::Result<WorkspaceContext> {
    // Explicit token bypasses workspace resolution
    if let Some(tok) = token {
        return Ok(WorkspaceContext {
            token: tok.to_string(),
            workspace_slug: String::new(),
        });
    }

    // Explicit workspace slug
    if let Some(slug) = workspace {
        return resolve_from_config(cfg, slug);
    }

    // Project-local workspace
    if let Some(slug) = project_workspace
        && cfg.workspaces.contains_key(slug)
    {
        return resolve_from_config(cfg, slug);
    }

    // Default workspace
    if let Some(slug) = &cfg.default_workspace {
        return resolve_from_config(cfg, slug);
    }

    // Only one workspace
    if cfg.workspaces.len() == 1 {
        let (slug, info) = cfg.workspaces.iter().next().unwrap();
        return Ok(WorkspaceContext {
            token: info.token.clone(),
            workspace_slug: slug.clone(),
        });
    }

    bail!("not logged in. Run `nrz login` first.")
}

fn resolve_from_config(
    cfg: &config::WorkspaceConfig,
    slug: &str,
) -> anyhow::Result<WorkspaceContext> {
    match cfg.workspaces.get(slug) {
        Some(info) => Ok(WorkspaceContext {
            token: info.token.clone(),
            workspace_slug: slug.to_string(),
        }),
        None => bail!(
            "workspace '{slug}' not found. Run `nrz workspace list` to see available workspaces."
        ),
    }
}

fn resolve_project_workspace() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let path = cwd.join(".onreza/project.json");
    let content = std::fs::read_to_string(path).ok()?;
    let val: serde_json::Value = serde_json::from_str(&content).ok()?;
    val.get("workspace_slug")?.as_str().map(String::from)
}
