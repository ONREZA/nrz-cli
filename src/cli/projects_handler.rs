use std::io::{BufRead, IsTerminal, Write};

use anyhow::{Context, bail};

use crate::api::ApiClient;
use crate::auth;
use crate::output;
use nrz::config;

use super::projects::{ProjectsArgs, ProjectsCommand};

mod models;
use models::*;

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
        .projects(limit, 0)
        .await
        .context("failed to fetch projects")?
        .into();

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
            let framework = output::terminal_line(p.framework_preset.as_deref().unwrap_or("-"));
            let updated = output::terminal_line(p.updated_at.as_deref().unwrap_or("-"));
            let display_name = output::terminal_line(p.display_name.as_deref().unwrap_or(&p.name));
            eprintln!("  {display_name:<30} {framework:<15} {updated}");
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
    let body = nrz_api::ProjectRequestBody {
        name: name.clone(),
        display_name: display_name.clone(),
        git_url,
        branch,
        framework_preset: framework,
        install_command_source: install_command
            .as_ref()
            .map(|_| nrz_api::ProjectRequestBodyInstallCommandSource::User),
        install_command: install_command.map(Some),
        build_command_source: build_command
            .as_ref()
            .map(|_| nrz_api::ProjectRequestBodyInstallCommandSource::User),
        build_command: build_command.map(Some),
        output_directory_source: output_directory
            .as_ref()
            .map(|_| nrz_api::ProjectRequestBodyInstallCommandSource::User),
        output_directory,
        ..Default::default()
    };

    let resp: CreateProjectResponse = client
        .create_project(body)
        .await
        .context("failed to create project")?
        .into();

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
                resp.id,
            ),
            output::Phase::Projects,
        );
    }

    Ok(())
}

async fn info(client: &ApiClient, id: &str, json: bool) -> anyhow::Result<()> {
    let resp: ProjectDetailResponse = client
        .project(id)
        .await
        .with_context(|| format!("failed to fetch project {id}"))?
        .into();

    if json {
        output::json_output(&resp);
    } else {
        eprintln!();
        let display = output::terminal_line(resp.display_name.as_deref().unwrap_or(&resp.name));
        eprintln!("  {}", console::style(display).bold());
        eprintln!("  {}", "-".repeat(40));
        eprintln!(
            "  {:<20} {}",
            console::style("ID").dim(),
            output::terminal_line(&resp.id)
        );
        eprintln!(
            "  {:<20} {}",
            console::style("Name").dim(),
            output::terminal_line(&resp.name)
        );
        if let Some(ref st) = resp.source_type {
            eprintln!(
                "  {:<20} {}",
                console::style("Source").dim(),
                output::terminal_line(st)
            );
        }
        if let Some(ref url) = resp.git_url {
            eprintln!(
                "  {:<20} {}",
                console::style("Git URL").dim(),
                output::terminal_line(url)
            );
        }
        if let Some(ref b) = resp.branch {
            eprintln!(
                "  {:<20} {}",
                console::style("Branch").dim(),
                output::terminal_line(b)
            );
        }
        if let Some(ref f) = resp.framework_preset {
            eprintln!(
                "  {:<20} {}",
                console::style("Framework").dim(),
                output::terminal_line(f)
            );
        }
        if let Some(ref nv) = resp.node_version {
            eprintln!(
                "  {:<20} {}",
                console::style("Node").dim(),
                output::terminal_line(nv)
            );
        }
        if let Some(ref pm) = resp.package_manager {
            eprintln!(
                "  {:<20} {}",
                console::style("Pkg Manager").dim(),
                output::terminal_line(pm)
            );
        }
        if let Some(ref c) = resp.created_at {
            eprintln!(
                "  {:<20} {}",
                console::style("Created").dim(),
                output::terminal_line(c)
            );
        }
        if let Some(ref u) = resp.updated_at {
            eprintln!(
                "  {:<20} {}",
                console::style("Updated").dim(),
                output::terminal_line(u)
            );
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
    if [
        &display_name,
        &git_url,
        &branch,
        &framework,
        &install_command,
        &build_command,
        &output_directory,
        &root_directory,
        &node_version,
    ]
    .iter()
    .all(|value| value.is_none())
    {
        bail!("no fields to update. Specify at least one --flag.");
    }
    let body = nrz_api::ProjectRequestBody2 {
        display_name,
        git_url: git_url.map(Some),
        branch,
        framework_preset: framework,
        install_command_source: install_command
            .as_ref()
            .map(|_| nrz_api::ProjectRequestBodyInstallCommandSource::User),
        install_command: install_command.map(Some),
        build_command_source: build_command
            .as_ref()
            .map(|_| nrz_api::ProjectRequestBodyInstallCommandSource::User),
        build_command: build_command.map(Some),
        output_directory_source: output_directory
            .as_ref()
            .map(|_| nrz_api::ProjectRequestBodyInstallCommandSource::User),
        output_directory: output_directory.map(Some),
        root_directory,
        node_version: node_version
            .map(|value| serde_json::from_value(serde_json::Value::String(value)))
            .transpose()
            .context("invalid Node version")?,
        ..Default::default()
    };
    let result = client
        .update_project(id, body)
        .await
        .with_context(|| format!("failed to update project {id}"))?;
    let resp = UpdateProjectResponse {
        id: result.id.to_string(),
        message: Some(result.message),
    };

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
            console::style(format!(
                "Type project ID ({}) to confirm deletion:",
                output::terminal_line(id)
            ))
            .bold(),
        );
        std::io::stderr().flush()?;

        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;

        if line.trim() != id {
            bail!("confirmation did not match, aborting.");
        }
    }

    let result = client
        .delete_project(id)
        .await
        .with_context(|| format!("failed to delete project {id}"))?;
    let resp = DeleteProjectResponse {
        id: result.id.to_string(),
        message: Some(result.message),
    };

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
