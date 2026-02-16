use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::cli::ProjectsArgs;
use crate::output;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectsResponse {
    projects: Vec<Project>,
    total: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Project {
    id: String,
    name: String,
    display_name: String,
    framework_preset: Option<String>,
    updated_at: Option<String>,
}

#[derive(Serialize)]
struct ProjectsOutput {
    projects: Vec<ProjectOutput>,
    total: u64,
}

/// Stable CLI JSON output. Field `framework` maps from API's `frameworkPreset`
/// for backward compatibility with scripts and LLM agents.
#[derive(Serialize)]
struct ProjectOutput {
    id: String,
    name: String,
    display_name: String,
    framework: Option<String>,
    updated_at: Option<String>,
}

impl From<Project> for ProjectOutput {
    fn from(p: Project) -> Self {
        Self {
            id: p.id,
            name: p.name,
            display_name: p.display_name,
            framework: p.framework_preset,
            updated_at: p.updated_at,
        }
    }
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
        .get(&format!("/v1/projects?limit={}", args.limit))
        .await
        .context("failed to fetch projects")?;

    if json {
        output::json_output(&ProjectsOutput {
            projects: resp.projects.into_iter().map(ProjectOutput::from).collect(),
            total: resp.total,
        });
    } else if resp.projects.is_empty() {
        eprintln!("  No projects found.");
        return Ok(());
    } else {
        eprintln!();
        eprintln!(
            "  {:<30} {:<15} {}",
            console::style("Name").bold(),
            console::style("Framework").bold(),
            console::style("Updated").bold(),
        );
        eprintln!("  {}", "-".repeat(60));

        for p in &resp.projects {
            let framework = p.framework_preset.as_deref().unwrap_or("-");
            let updated = p.updated_at.as_deref().unwrap_or("-");
            eprintln!("  {:<30} {:<15} {}", p.display_name, framework, updated);
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
