pub mod project_ref;

#[cfg(test)]
mod project_ref_tests;

use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::cli::LinkArgs;
use crate::output;

#[derive(Debug, Deserialize)]
struct ProjectListResponse {
    projects: Vec<Project>,
    #[allow(dead_code)]
    total: u64,
}

#[derive(Debug, Deserialize)]
struct Project {
    id: String,
    #[allow(dead_code)]
    name: String,
    display_name: String,
    #[allow(dead_code)]
    framework: Option<String>,
    #[allow(dead_code)]
    workspace: ProjectWorkspace,
}

#[derive(Debug, Deserialize)]
struct ProjectWorkspace {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    slug: String,
    #[allow(dead_code)]
    name: String,
}

#[derive(Serialize)]
struct LinkOutput {
    project_id: String,
    project_name: String,
}

pub async fn run(args: LinkArgs, json: bool, token: Option<&str>) -> anyhow::Result<()> {
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("directory not found: {}", args.dir))?;

    let tok = auth::resolve_token(token)
        .ok_or_else(|| anyhow::anyhow!("not logged in. Run `nrz login` first."))?;

    let client = ApiClient::authenticated(&tok)?;

    let project = if let Some(pid) = &args.project_id {
        find_project_by_id(&client, pid).await?
    } else if let Some(existing) = project_ref::load(&project_dir)? {
        existing
    } else if json {
        bail!("--project-id is required in non-interactive mode (--json)");
    } else {
        select_project_interactive(&client).await?
    };

    project_ref::save(&project_dir, &project)?;

    if json {
        output::json_output(&LinkOutput {
            project_id: project.project_id,
            project_name: project.project_name,
        });
    } else {
        output::success(
            false,
            format!("Linked to {}", console::style(&project.project_name).bold()),
        );
    }

    Ok(())
}

/// Find project by ID from user's project list.
async fn find_project_by_id(
    client: &ApiClient,
    project_id: &str,
) -> anyhow::Result<project_ref::ProjectRef> {
    let resp: ProjectListResponse = client
        .get("/v1/user/projects")
        .await
        .context("failed to fetch projects")?;

    let project = resp
        .projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| anyhow::anyhow!("project not found: {project_id}"))?;

    Ok(project_ref::ProjectRef {
        project_id: project.id.clone(),
        project_name: project.display_name.clone(),
    })
}

/// Interactive project selection (human mode only).
pub async fn select_project_interactive(
    client: &ApiClient,
) -> anyhow::Result<project_ref::ProjectRef> {
    let resp: ProjectListResponse = client
        .get("/v1/user/projects")
        .await
        .context("failed to fetch projects")?;

    if resp.projects.is_empty() {
        bail!("no projects found. Create a project at https://onreza.ru first.");
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

    let choice = prompt_choice(resp.projects.len())?;
    let project = &resp.projects[choice - 1];

    Ok(project_ref::ProjectRef {
        project_id: project.id.clone(),
        project_name: project.display_name.clone(),
    })
}

fn prompt_choice(max: usize) -> anyhow::Result<usize> {
    loop {
        eprint!(
            "  {} ",
            console::style(format!("Select project (1-{max}):")).bold(),
        );
        std::io::stderr().flush()?;

        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        let trimmed = line.trim();

        if let Ok(n) = trimmed.parse::<usize>()
            && n >= 1
            && n <= max
        {
            return Ok(n);
        }
        eprintln!("  Invalid choice. Enter a number between 1 and {max}.");
    }
}
