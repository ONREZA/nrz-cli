use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::cli::ProjectsArgs;
use crate::output;

#[derive(Debug, Deserialize, Serialize)]
struct ProjectsResponse {
    projects: Vec<Project>,
    total: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct Project {
    id: String,
    name: String,
    display_name: String,
    framework: Option<String>,
    workspace: ProjectWorkspace,
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ProjectWorkspace {
    id: String,
    slug: String,
    name: String,
}

pub async fn run(
    args: ProjectsArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;

    let client = ApiClient::authenticated(&tok)?;

    let resp: ProjectsResponse = client
        .get(&format!("/v1/user/projects?limit={}", args.limit))
        .await
        .context("failed to fetch projects")?;

    if json {
        output::json_output(&resp);
    } else {
        if resp.projects.is_empty() {
            eprintln!("  No projects found.");
            return Ok(());
        }

        eprintln!();
        eprintln!(
            "  {:<30} {:<15} {:<20} {}",
            console::style("Name").bold(),
            console::style("Framework").bold(),
            console::style("Workspace").bold(),
            console::style("Updated").bold(),
        );
        eprintln!("  {}", "-".repeat(80));

        for p in &resp.projects {
            let framework = p.framework.as_deref().unwrap_or("-");
            let updated = p.updated_at.as_deref().unwrap_or("-");
            eprintln!(
                "  {:<30} {:<15} {:<20} {}",
                p.display_name, framework, p.workspace.name, updated
            );
        }

        eprintln!();
        eprintln!(
            "  {} {} project(s)",
            console::style("Total:").dim(),
            resp.total,
        );
    }

    Ok(())
}
