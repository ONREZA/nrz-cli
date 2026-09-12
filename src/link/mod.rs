use std::path::Path;

use anyhow::{Context, bail};
use serde::Serialize;

use crate::api::ApiClient;
use crate::auth;
use crate::cli::LinkArgs;
use crate::output;
use nrz::config;
use nrz::config::ProjectConfig;

/// Minimal project info returned from interactive selection or API lookup.
#[derive(Debug)]
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
    let project = client
        .project(project_id)
        .await
        .with_context(|| format!("failed to fetch project {project_id}"))?;

    anyhow::ensure!(
        project.id == project_id.parse::<uuid::Uuid>()?,
        "project response belongs to another project"
    );
    Ok(SelectedProject {
        project_id: project.id.to_string(),
        project_name: project.display_name.unwrap_or(project.name),
    })
}

/// Interactive project selection (human mode only).
pub async fn select_project_interactive(client: &ApiClient) -> anyhow::Result<SelectedProject> {
    let projects = selection_projects(client).await?;
    if projects.is_empty() {
        bail!("no projects found. Create one with: nrz projects create --name <name>");
    }

    eprintln!();
    for (i, project) in projects.iter().enumerate() {
        eprintln!(
            "  {} {}",
            console::style(format!("{}.", i + 1)).dim(),
            output::terminal_line(&project.project_name),
        );
    }
    eprintln!();

    let choice = crate::output::prompt_choice("Select project", projects.len())?;
    Ok(projects
        .into_iter()
        .nth(choice - 1)
        .expect("validated project choice"))
}

async fn selection_projects(client: &ApiClient) -> anyhow::Result<Vec<SelectedProject>> {
    let mut projects = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut offset = 0u32;
    loop {
        let page = client
            .projects(100, offset)
            .await
            .context("failed to fetch projects")?;
        anyhow::ensure!(page.total >= 0, "invalid project count");
        let count = u32::try_from(page.projects.len()).context("project page is too large")?;
        let previous = projects.len();
        for project in page.projects {
            if seen.insert(project.id) {
                projects.push(SelectedProject {
                    project_id: project.id.to_string(),
                    project_name: project.display_name.unwrap_or(project.name),
                });
            }
        }
        offset = offset
            .checked_add(count)
            .context("project pagination exceeded its bound")?;
        if i64::from(offset) >= page.total {
            return Ok(projects);
        }
        anyhow::ensure!(
            projects.len() > previous,
            "project pagination made no progress; retry selection"
        );
    }
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod selection_tests;
