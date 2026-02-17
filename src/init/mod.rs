//! `nrz init` — initialize an existing project on ONREZA platform.

#[cfg(test)]
mod init_tests;

use std::path::Path;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::cli::InitArgs;
use crate::dev::detect;
use crate::link::project_ref;
use crate::output;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InitOutput {
    project_id: String,
    project_name: String,
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
) -> anyhow::Result<()> {
    let project_dir = Path::new(".")
        .canonicalize()
        .context("failed to resolve current directory")?;

    // Check if already linked
    if let Some(existing) = project_ref::load(&project_dir)? {
        bail!(
            "project already linked to {} ({}). Use `nrz link` to change.",
            existing.project_name,
            existing.project_id
        );
    }

    // Detect framework
    let framework = if args.skip_detection {
        None
    } else {
        match detect::detect_framework(&project_dir) {
            Ok(f) => {
                let name = format!("{:?}", f.name).to_lowercase();
                if !json {
                    output::status(false, "~", format!("Detected framework: {name}"));
                }
                Some(name)
            }
            Err(e) => {
                output::warn(json, format!("Could not detect framework: {e:#}"));
                None
            }
        }
    };

    // Detect package manager
    let package_manager = if args.skip_detection {
        None
    } else {
        Some(detect_package_manager(&project_dir))
    };

    // Resolve project name
    let name = args.name.unwrap_or_else(|| {
        project_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "my-project".into())
    });

    // Create project on platform
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;

    output::status(json, "~", format!("Creating project \"{}\"...", name));

    let body = CreateProjectBody {
        name: name.clone(),
        framework_preset: framework.clone(),
    };

    let resp: CreateProjectResponse = client
        .post("/v1/projects", &body)
        .await
        .context("failed to create project")?;

    // Link
    let pref = project_ref::ProjectRef {
        project_id: resp.id.clone(),
        project_name: name.clone(),
        workspace_slug: None,
    };
    project_ref::save(&project_dir, &pref)?;

    if json {
        output::json_output(&InitOutput {
            project_id: resp.id,
            project_name: name,
            framework,
            package_manager,
        });
    } else {
        output::success(false, format!("Project \"{}\" created and linked", name));
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

fn detect_package_manager(project_dir: &Path) -> String {
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
