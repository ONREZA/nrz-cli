use std::path::Path;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::cli::LinkArgs;
use crate::output;
use nrz::config;
use nrz::config::ProjectConfig;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectListResponse {
    projects: Vec<Project>,
    #[allow(dead_code)]
    total: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Project {
    id: String,
    #[allow(dead_code)]
    name: String,
    display_name: String,
}

/// Minimal project info returned from interactive selection or API lookup.
pub struct SelectedProject {
    pub project_id: String,
    pub project_name: String,
}

#[derive(Serialize)]
struct LinkOutput {
    project_id: String,
    project_name: String,
}

pub async fn run(
    args: LinkArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    _config: &ProjectConfig,
) -> anyhow::Result<()> {
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("directory not found: {}", args.dir))?;

    let ctx = auth::workspace::resolve_workspace_context(token, workspace)?;
    let tok = ctx.token;

    let client = ApiClient::authenticated(&tok)?;

    let project = if let Some(pid) = &args.project_id {
        find_project_by_id(&client, pid).await?
    } else if json {
        bail!("--project-id is required in non-interactive mode (--json)");
    } else {
        select_project_interactive(&client).await?
    };

    let ws = if ctx.workspace_slug.is_empty() {
        None
    } else {
        Some(ctx.workspace_slug.as_str())
    };
    config::save_or_update(
        &project_dir,
        &project.project_id,
        Some(&project.project_name),
        ws,
    )?;

    // Ensure .onreza/ is in .gitignore
    crate::init::add_to_gitignore(&project_dir);

    if json {
        output::json_output(&LinkOutput {
            project_id: project.project_id,
            project_name: project.project_name,
        });
    } else {
        output::success(
            false,
            format!("Linked to {}", console::style(&project.project_name).bold()),
            output::Phase::Link,
        );
    }

    Ok(())
}

/// Find project by ID via GET /v1/projects/:id.
pub async fn find_project_by_id(
    client: &ApiClient,
    project_id: &str,
) -> anyhow::Result<SelectedProject> {
    let project: Project = client
        .get(&format!("/v1/projects/{project_id}"))
        .await
        .with_context(|| format!("failed to fetch project {project_id}"))?;

    Ok(SelectedProject {
        project_id: project.id,
        project_name: project.display_name,
    })
}

/// Interactive project selection (human mode only).
pub async fn select_project_interactive(client: &ApiClient) -> anyhow::Result<SelectedProject> {
    let resp: ProjectListResponse = client
        .get("/v1/projects")
        .await
        .context("failed to fetch projects")?;

    if resp.projects.is_empty() {
        bail!("no projects found. Create one with: nrz projects create --name <name>");
    }

    eprintln!();
    for (i, project) in resp.projects.iter().enumerate() {
        eprintln!(
            "  {} {}",
            console::style(format!("{}.", i + 1)).dim(),
            project.display_name,
        );
    }
    eprintln!();

    let choice = crate::output::prompt_choice("Select project", resp.projects.len())?;
    let project = &resp.projects[choice - 1];

    Ok(SelectedProject {
        project_id: project.id.clone(),
        project_name: project.display_name.clone(),
    })
}
