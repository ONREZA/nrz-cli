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
    let framework = if args.skip_detection {
        None
    } else {
        let detected = detect_framework(&project_dir);
        if let Some(ref name) = detected
            && !json
        {
            output::status(false, "~", format!("Detected framework: {name}"));
        }
        detected
    };

    let package_manager = if args.skip_detection {
        None
    } else {
        Some(detect_package_manager(&project_dir))
    };

    // Phase 2: Local scaffold — create onreza.toml template + .onreza/ + .gitignore
    scaffold_local(&project_dir, json)?;

    // Phase 3: Optionally create or link project on platform
    let (project_id, project_name) = if args.create {
        // --create: create project on platform
        let name = resolve_project_name(&args.name, &project_dir);
        let (id, name) = create_on_platform(token, workspace, &name, &framework, json).await?;
        config::save_or_update(&project_dir, &id, Some(&name), None)?;
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
        if !json {
            output::success(
                false,
                format!(
                    "Linked to {}",
                    console::style(&selected.project_name).bold()
                ),
            );
        }
        (Some(selected.project_id), Some(selected.project_name))
    } else if !json && std::io::stdin().is_terminal() {
        // Interactive wizard
        interactive_bootstrap(token, workspace, &args.name, &project_dir, &framework).await?
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
        output::success(false, "Initialized project scaffold");
        eprintln!();
        eprintln!("  Next steps:");
        eprintln!("    1. Link project: nrz link");
        eprintln!("    2. Install adapter: npm add @onreza/adapter-<framework>");
        eprintln!("    3. Deploy: nrz deploy");
        eprintln!();
    } else {
        let display = project_name.as_deref().unwrap_or("project");
        output::success(false, format!("Project \"{display}\" created and linked"));
        eprintln!();
        eprintln!("  Next steps:");
        if let Some(ref pm) = package_manager {
            eprintln!("    1. Install adapter: {pm} add @onreza/adapter-<framework>");
        } else {
            eprintln!("    1. Install adapter: npm add @onreza/adapter-<framework>");
        }
        eprintln!("    2. Build: nrz build");
        eprintln!("    3. Deploy: nrz deploy");
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
            output::status(false, "+", "Created onreza.toml");
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

    output::status(json, "~", format!("Creating project \"{name}\"..."));

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

pub(crate) fn detect_framework(project_dir: &Path) -> Option<String> {
    let pkg_content = std::fs::read_to_string(project_dir.join("package.json")).ok()?;
    let pkg: serde_json::Value = serde_json::from_str(&pkg_content).ok()?;

    let has_dep = |name: &str| -> bool {
        pkg.get("dependencies").and_then(|d| d.get(name)).is_some()
            || pkg
                .get("devDependencies")
                .and_then(|d| d.get(name))
                .is_some()
    };

    if has_dep("astro") {
        Some("astro".into())
    } else if has_dep("nuxt") {
        Some("nuxt".into())
    } else if has_dep("@sveltejs/kit") {
        Some("sveltekit".into())
    } else if has_dep("nitropack") {
        Some("nitro".into())
    } else {
        None
    }
}

pub(crate) fn detect_package_manager(project_dir: &Path) -> String {
    if project_dir.join("bun.lockb").exists() || project_dir.join("bun.lock").exists() {
        "bun".into()
    } else if project_dir.join("pnpm-lock.yaml").exists() {
        "pnpm".into()
    } else if project_dir.join("yarn.lock").exists() {
        "yarn".into()
    } else {
        "npm".into()
    }
}
