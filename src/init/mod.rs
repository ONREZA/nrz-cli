//! `nrz init` — initialize project scaffold and optionally create/link on platform.

#[cfg(test)]
mod init_tests;

use std::io::IsTerminal;
use std::path::Path;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::cli::InitArgs;
use crate::link;
use crate::output;
use nrz::config;
use nrz::config::ProjectConfig;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InitOutput {
    project_id: Option<String>,
    project_name: Option<String>,
    framework: Option<String>,
    package_manager: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateProjectBody {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    framework_preset: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateProjectResponse {
    id: String,
    #[allow(dead_code)]
    name: String,
}

pub async fn run(
    args: InitArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let project_dir = Path::new(".")
        .canonicalize()
        .context("failed to resolve current directory")?;

    // Check if already linked
    if let Some(id) = &config.project.id {
        bail!(
            "project already configured in onreza.toml ({}). Use `nrz link` to change.",
            id
        );
    }

    // Phase 1: Detect framework and package manager
    let detection_result = if args.skip_detection {
        None
    } else {
        Some(crate::detect::detect(&project_dir))
    };

    let framework = detection_result.as_ref().and_then(|r| {
        if r.framework == "other" {
            None
        } else {
            Some(r.framework.clone())
        }
    });

    if let Some(ref name) = framework
        && !json
    {
        output::status(
            false,
            "~",
            format!("Detected framework: {name}"),
            output::Phase::Init,
        );
    }

    let package_manager = detection_result
        .as_ref()
        .and_then(|r| r.metadata.package_manager.as_ref())
        .map(|pm| pm.pm_type.as_str().to_string());

    // Phase 2: Local scaffold — create onreza.toml template + .onreza/ + .gitignore
    scaffold_local(&project_dir, json)?;

    // Save detected framework to onreza.toml (best-effort)
    if let Some(ref fw) = framework
        && let Err(e) = config::save_framework(&project_dir, fw)
    {
        output::warn(
            json,
            format!("could not save framework to onreza.toml: {e}"),
            output::Phase::Init,
        );
    }

    // Phase 3: Optionally create or link project on platform
    let (project_id, project_name) = if args.create {
        // --create: create project on platform
        let name = resolve_project_name(&args.name, &project_dir);
        let (id, name) = create_on_platform(token, workspace, &name, &framework, json).await?;
        config::save_or_update(&project_dir, &id, Some(&name), None)?;
        // Sync detection to API (best-effort)
        if let Some(ref result) = detection_result {
            let tok = auth::resolve_token(token, workspace).ok();
            if let Some(ref t) = tok
                && let Ok(client) = ApiClient::authenticated(t)
            {
                crate::detect_sync::sync_detection_to_api(&client, &id, result).await;
            }
        }
        (Some(id), Some(name))
    } else if let Some(pid) = &args.project_id {
        // --project-id: link existing project
        let tok = auth::resolve_token(token, workspace)?;
        let client = ApiClient::authenticated(&tok)?;
        let selected = link::find_project_by_id(&client, pid).await?;
        config::save_or_update(
            &project_dir,
            &selected.project_id,
            Some(&selected.project_name),
            None,
        )?;
        // Sync detection to API (best-effort)
        if let Some(ref result) = detection_result {
            crate::detect_sync::sync_detection_to_api(&client, &selected.project_id, result).await;
        }
        if !json {
            output::success(
                false,
                format!(
                    "Linked to {}",
                    console::style(&selected.project_name).bold()
                ),
                output::Phase::Init,
            );
        }
        (Some(selected.project_id), Some(selected.project_name))
    } else if !json && std::io::stdin().is_terminal() {
        // Interactive wizard
        let result_ids =
            interactive_bootstrap(token, workspace, &args.name, &project_dir, &framework).await?;
        // Sync detection to API (best-effort)
        if let (Some(ref id), _) = result_ids
            && let Some(ref det) = detection_result
        {
            let tok = auth::resolve_token(token, workspace).ok();
            if let Some(ref t) = tok
                && let Ok(client) = ApiClient::authenticated(t)
            {
                crate::detect_sync::sync_detection_to_api(&client, id, det).await;
            }
        }
        result_ids
    } else {
        // JSON / non-interactive: local-only scaffold
        (None, None)
    };

    if json {
        output::json_output(&InitOutput {
            project_id,
            project_name,
            framework,
            package_manager,
        });
    } else if project_id.is_none() {
        output::success(false, "Initialized project scaffold", output::Phase::Init);
        eprintln!();
        eprintln!("  Next steps:");
        eprintln!("    1. Link project: nrz link");
        eprintln!("    2. Build: nrz build");
        eprintln!("    3. Deploy: nrz deploy");
        eprintln!();
    } else {
        let display = project_name.as_deref().unwrap_or("project");
        output::success(
            false,
            format!("Project \"{display}\" created and linked"),
            output::Phase::Init,
        );
        eprintln!();
        eprintln!("  Next steps:");
        eprintln!("    1. Build: nrz build");
        eprintln!("    2. Deploy: nrz deploy");
        eprintln!();
    }

    Ok(())
}

/// Create local scaffold: onreza.toml template, .onreza/ dir, .gitignore entry.
fn scaffold_local(project_dir: &Path, json: bool) -> anyhow::Result<()> {
    let toml_path = project_dir.join("onreza.toml");
    if !toml_path.exists() {
        let content = config::generate_template(None, None, None);
        std::fs::write(&toml_path, content)
            .with_context(|| format!("failed to write {}", toml_path.display()))?;
        if !json {
            output::status(false, "+", "Created onreza.toml", output::Phase::Init);
        }
    }

    let onreza_dir = project_dir.join(".onreza");
    if !onreza_dir.exists() {
        std::fs::create_dir_all(&onreza_dir)
            .with_context(|| format!("failed to create {}", onreza_dir.display()))?;
    }

    add_to_gitignore(project_dir);

    Ok(())
}

/// Interactive wizard: ask user whether to create, link, or skip.
async fn interactive_bootstrap(
    token: Option<&str>,
    workspace: Option<&str>,
    name_arg: &Option<String>,
    project_dir: &Path,
    framework: &Option<String>,
) -> anyhow::Result<(Option<String>, Option<String>)> {
    eprintln!();
    eprintln!(
        "  {} Create project on ONREZA platform?",
        console::style("?").cyan().bold()
    );
    eprintln!("    {} Create new project", console::style("1.").dim());
    eprintln!("    {} Link existing project", console::style("2.").dim());
    eprintln!("    {} Skip (local only)", console::style("3.").dim());
    eprintln!();

    let choice = output::prompt_choice("Select", 3)?;

    match choice {
        1 => {
            let name = resolve_project_name(name_arg, project_dir);
            let (id, name) = create_on_platform(token, workspace, &name, framework, false).await?;
            config::save_or_update(project_dir, &id, Some(&name), None)?;
            Ok((Some(id), Some(name)))
        }
        2 => {
            let tok = auth::resolve_token(token, workspace)?;
            let client = ApiClient::authenticated(&tok)?;
            let selected = link::select_project_interactive(&client).await?;
            config::save_or_update(
                project_dir,
                &selected.project_id,
                Some(&selected.project_name),
                None,
            )?;
            output::success(
                false,
                format!(
                    "Linked to {}",
                    console::style(&selected.project_name).bold()
                ),
                output::Phase::Init,
            );
            Ok((Some(selected.project_id), Some(selected.project_name)))
        }
        _ => Ok((None, None)),
    }
}

/// Create project on platform via API.
async fn create_on_platform(
    token: Option<&str>,
    workspace: Option<&str>,
    name: &str,
    framework: &Option<String>,
    json: bool,
) -> anyhow::Result<(String, String)> {
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;

    output::status(
        json,
        "~",
        format!("Creating project \"{name}\"..."),
        output::Phase::Init,
    );

    let body = CreateProjectBody {
        name: name.to_string(),
        framework_preset: framework.clone(),
    };

    let resp: CreateProjectResponse = client
        .post("/v1/projects", &body)
        .await
        .context("failed to create project")?;

    Ok((resp.id, name.to_string()))
}

fn resolve_project_name(name_arg: &Option<String>, project_dir: &Path) -> String {
    name_arg.clone().unwrap_or_else(|| {
        project_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "my-project".into())
    })
}

pub fn add_to_gitignore(project_dir: &Path) {
    let gitignore = project_dir.join(".gitignore");
    if !gitignore.exists() {
        return;
    }
    let content = match std::fs::read_to_string(&gitignore) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  Warning: could not read .gitignore: {e}");
            return;
        }
    };
    // Check if .onreza/ is already in .gitignore
    if content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == ".onreza/"
            || trimmed == ".onreza"
            || trimmed == "/.onreza/"
            || trimmed == "/.onreza"
    }) {
        return;
    }
    // Append .onreza/ to .gitignore
    let separator = if content.ends_with('\n') { "" } else { "\n" };
    if let Err(e) = std::fs::write(&gitignore, format!("{content}{separator}.onreza/\n")) {
        eprintln!("  Warning: could not update .gitignore: {e}");
    }
}
