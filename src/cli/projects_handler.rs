use std::io::{BufRead, IsTerminal, Write};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::output;
use nrz::config;

use super::projects::{ProjectsArgs, ProjectsCommand};

// --- List ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectsResponse {
    projects: Vec<ProjectSummary>,
    total: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSummary {
    id: String,
    name: String,
    display_name: Option<String>,
    framework_preset: Option<String>,
    updated_at: Option<String>,
}

// --- Create ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateProjectBody {
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) git_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) framework_preset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) install_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) install_command_source: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) build_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) build_command_source: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output_directory_source: Option<&'static str>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateProjectResponse {
    id: String,
    name: String,
    source_type: Option<String>,
    git_url: Option<String>,
    branch: Option<String>,
    created_at: Option<String>,
    message: Option<String>,
}

// --- Info ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectDetailResponse {
    id: String,
    name: String,
    display_name: Option<String>,
    source_type: Option<String>,
    git_url: Option<String>,
    branch: Option<String>,
    framework_preset: Option<String>,
    install_command: Option<String>,
    build_command: Option<String>,
    output_directory: Option<String>,
    root_directory: Option<String>,
    node_version: Option<String>,
    package_manager: Option<String>,
    auto_deploy_enabled: Option<bool>,
    created_at: Option<String>,
    updated_at: Option<String>,
    #[serde(default)]
    deployments: Vec<serde_json::Value>,
    #[serde(default, rename = "_count")]
    count: Option<serde_json::Value>,
}

// --- Update ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProjectResponse {
    id: String,
    message: Option<String>,
}

// --- Delete ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteProjectResponse {
    id: String,
    message: Option<String>,
}

pub async fn run(
    args: ProjectsArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;

    match args.command {
        ProjectsCommand::List { limit } => list(&client, limit, json).await,
        ProjectsCommand::Create {
            name,
            display_name,
            git_url,
            branch,
            framework,
            install_command,
            build_command,
            output_directory,
            link,
        } => {
            create(
                &client,
                json,
                name,
                display_name,
                git_url,
                branch,
                framework,
                install_command,
                build_command,
                output_directory,
                link,
            )
            .await
        }
        ProjectsCommand::Info { id } => info(&client, &id, json).await,
        ProjectsCommand::Update {
            id,
            display_name,
            git_url,
            branch,
            framework,
            install_command,
            build_command,
            output_directory,
            root_directory,
            node_version,
        } => {
            update(
                &client,
                &id,
                json,
                display_name,
                git_url,
                branch,
                framework,
                install_command,
                build_command,
                output_directory,
                root_directory,
                node_version,
            )
            .await
        }
        ProjectsCommand::Delete { id, force } => delete(&client, &id, force, json).await,
    }
}

async fn list(client: &ApiClient, limit: u32, json: bool) -> anyhow::Result<()> {
    let resp: ProjectsResponse = client
        .get(&format!("/v1/projects?limit={limit}"))
        .await
        .context("failed to fetch projects")?;

    if json {
        output::json_output(&resp);
    } else if resp.projects.is_empty() {
        eprintln!("  No projects found.");
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
            let display_name = p.display_name.as_deref().unwrap_or(&p.name);
            eprintln!("  {:<30} {:<15} {}", display_name, framework, updated);
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

#[allow(clippy::too_many_arguments)]
async fn create(
    client: &ApiClient,
    json: bool,
    name: String,
    display_name: Option<String>,
    git_url: Option<String>,
    branch: Option<String>,
    framework: Option<String>,
    install_command: Option<String>,
    build_command: Option<String>,
    output_directory: Option<String>,
    link: bool,
) -> anyhow::Result<()> {
    let body = CreateProjectBody {
        name: name.clone(),
        display_name: display_name.clone(),
        git_url,
        branch,
        framework_preset: framework,
        install_command_source: install_command.as_ref().map(|_| "USER"),
        install_command,
        build_command_source: build_command.as_ref().map(|_| "USER"),
        build_command,
        output_directory_source: output_directory.as_ref().map(|_| "USER"),
        output_directory,
    };

    let resp: CreateProjectResponse = client
        .post("/v1/projects", &body)
        .await
        .context("failed to create project")?;

    let linked = if link {
        let cwd = std::env::current_dir().context("failed to get current directory")?;
        let display = display_name.as_deref().unwrap_or(&name);
        config::save_or_update(&cwd, &resp.id, Some(display), None)?;
        crate::init::add_to_gitignore(&cwd);
        true
    } else {
        false
    };

    if json {
        let mut out = serde_json::to_value(&resp).context("failed to serialize response")?;
        if linked {
            out["linked"] = serde_json::Value::Bool(true);
        }
        output::json_output(&out);
    } else {
        if linked {
            output::success(
                false,
                "Linked to current directory",
                output::Phase::Projects,
            );
        }
        output::success(
            false,
            format!(
                "Created project {} ({})",
                console::style(&name).bold(),
                &resp.id,
            ),
            output::Phase::Projects,
        );
    }

    Ok(())
}

async fn info(client: &ApiClient, id: &str, json: bool) -> anyhow::Result<()> {
    let resp: ProjectDetailResponse = client
        .get(&format!("/v1/projects/{id}"))
        .await
        .with_context(|| format!("failed to fetch project {id}"))?;

    if json {
        output::json_output(&resp);
    } else {
        eprintln!();
        let display = resp.display_name.as_deref().unwrap_or(&resp.name);
        eprintln!("  {}", console::style(display).bold());
        eprintln!("  {}", "-".repeat(40));
        eprintln!("  {:<20} {}", console::style("ID").dim(), resp.id);
        eprintln!("  {:<20} {}", console::style("Name").dim(), resp.name);
        if let Some(ref st) = resp.source_type {
            eprintln!("  {:<20} {}", console::style("Source").dim(), st);
        }
        if let Some(ref url) = resp.git_url {
            eprintln!("  {:<20} {}", console::style("Git URL").dim(), url);
        }
        if let Some(ref b) = resp.branch {
            eprintln!("  {:<20} {}", console::style("Branch").dim(), b);
        }
        if let Some(ref f) = resp.framework_preset {
            eprintln!("  {:<20} {}", console::style("Framework").dim(), f);
        }
        if let Some(ref nv) = resp.node_version {
            eprintln!("  {:<20} {}", console::style("Node").dim(), nv);
        }
        if let Some(ref pm) = resp.package_manager {
            eprintln!("  {:<20} {}", console::style("Pkg Manager").dim(), pm);
        }
        if let Some(ref c) = resp.created_at {
            eprintln!("  {:<20} {}", console::style("Created").dim(), c);
        }
        if let Some(ref u) = resp.updated_at {
            eprintln!("  {:<20} {}", console::style("Updated").dim(), u);
        }
        eprintln!();
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn update(
    client: &ApiClient,
    id: &str,
    json: bool,
    display_name: Option<String>,
    git_url: Option<String>,
    branch: Option<String>,
    framework: Option<String>,
    install_command: Option<String>,
    build_command: Option<String>,
    output_directory: Option<String>,
    root_directory: Option<String>,
    node_version: Option<String>,
) -> anyhow::Result<()> {
    let mut body = serde_json::Map::new();

    macro_rules! set_field {
        ($field:expr, $key:expr) => {
            if let Some(val) = $field {
                body.insert($key.to_string(), serde_json::Value::String(val));
            }
        };
    }

    set_field!(display_name, "displayName");
    set_field!(git_url, "gitUrl");
    set_field!(branch, "branch");
    set_field!(framework, "frameworkPreset");
    set_field!(install_command, "installCommand");
    set_field!(build_command, "buildCommand");
    set_field!(output_directory, "outputDirectory");
    set_field!(root_directory, "rootDirectory");
    set_field!(node_version, "nodeVersion");

    if body.is_empty() {
        bail!("no fields to update. Specify at least one --flag.");
    }

    let body = serde_json::Value::Object(body);

    let resp: UpdateProjectResponse = client
        .patch(&format!("/v1/projects/{id}"), &body)
        .await
        .with_context(|| format!("failed to update project {id}"))?;

    if json {
        output::json_output(&resp);
    } else {
        output::success(
            false,
            format!("Updated project {}", console::style(id).bold()),
            output::Phase::Projects,
        );
    }

    Ok(())
}

async fn delete(client: &ApiClient, id: &str, force: bool, json: bool) -> anyhow::Result<()> {
    if !force {
        if json || !std::io::stdin().is_terminal() {
            bail!("--force is required in non-interactive mode");
        }

        eprint!(
            "  {} ",
            console::style(format!("Type project ID ({id}) to confirm deletion:")).bold(),
        );
        std::io::stderr().flush()?;

        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;

        if line.trim() != id {
            bail!("confirmation did not match, aborting.");
        }
    }

    let resp: DeleteProjectResponse = client
        .delete(&format!("/v1/projects/{id}"))
        .await
        .with_context(|| format!("failed to delete project {id}"))?;

    if json {
        output::json_output(&resp);
    } else {
        output::success(
            false,
            format!("Deleted project {}", console::style(id).bold()),
            output::Phase::Projects,
        );
    }

    Ok(())
}
