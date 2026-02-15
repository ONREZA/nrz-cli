mod archive;

#[cfg(test)]
mod archive_tests;

use std::path::Path;

use anyhow::{Context, bail};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::build;
use crate::cli::{BuildArgs, DeployArgs};
use crate::link::{self, project_ref};
use crate::output;

#[derive(Debug, Serialize)]
struct CreateDeploymentBody {
    manifest: serde_json::Value,
    production: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit_sha: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateDeploymentResponse {
    id: String,
    #[allow(dead_code)]
    status: String,
    url: String,
    upload_urls: UploadUrls,
}

#[derive(Debug, Deserialize)]
struct UploadUrls {
    artifact: String,
    server_bundle: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeploymentStatus {
    status: String,
    url: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ActivateResponse {
    #[allow(dead_code)]
    status: String,
}

#[derive(Debug, Deserialize)]
struct UploadCompleteResponse {
    #[allow(dead_code)]
    status: String,
}

#[derive(Serialize)]
struct DeployOutput {
    deployment_id: String,
    url: String,
    status: String,
}

pub async fn run(args: DeployArgs, json: bool, token: Option<&str>) -> anyhow::Result<()> {
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;

    let tok = auth::resolve_token(token)
        .ok_or_else(|| anyhow::anyhow!("not logged in. Use --token or run `nrz login`."))?;

    let client = ApiClient::authenticated(&tok)?;

    // Resolve project: --project-id > .onreza/project.json > interactive
    let project = if let Some(pid) = &args.project_id {
        project_ref::ProjectRef {
            project_id: pid.clone(),
            project_name: String::new(),
        }
    } else {
        match project_ref::load(&project_dir)? {
            Some(p) => p,
            None => {
                if json {
                    bail!("no linked project. Use --project-id or run `nrz link` first.");
                }
                output::warn(false, "No linked project. Select one:");
                let pref = link::select_project_interactive(&client).await?;
                project_ref::save(&project_dir, &pref)?;
                output::success(
                    false,
                    format!("Linked to {}", console::style(&pref.project_name).bold()),
                );
                pref
            }
        }
    };

    // Validate build output
    output::status(json, "~", "Validating build output...");
    build::run(
        BuildArgs {
            dir: project_dir.to_string_lossy().into_owned(),
            skip_validation: false,
        },
        json,
    )
    .await?;

    // Read manifest as raw JSON
    let output_dir = detect_output_dir(&project_dir)?;
    let manifest_path = output_dir.join(".onreza/manifest.json");
    let manifest_raw: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?,
    )?;

    // Git info (optional)
    let branch = git_cmd(&["rev-parse", "--abbrev-ref", "HEAD"]);
    let commit_sha = git_cmd(&["rev-parse", "HEAD"]);

    // Create deployment
    output::status(json, "~", "Creating deployment...");
    let body = CreateDeploymentBody {
        manifest: manifest_raw,
        production: args.prod,
        branch,
        commit_sha,
    };
    let deployment: CreateDeploymentResponse = client
        .post(
            &format!("/v1/projects/{}/deployments", project.project_id),
            &body,
        )
        .await
        .context("failed to create deployment")?;

    // Create tar.gz of output dir
    let spinner = make_spinner(json, "Uploading artifact...");
    let archive_data = archive::create_tar_gz(&output_dir).context("failed to create archive")?;
    let archive_size = archive_data.len();

    // Upload artifact
    client
        .put_bytes(
            &deployment.upload_urls.artifact,
            archive_data,
            "application/gzip",
        )
        .await
        .context("failed to upload artifact")?;
    finish_spinner(
        spinner,
        &format!("Artifact uploaded ({})", format_bytes(archive_size)),
    );

    // Upload server bundle if SSR
    if let Some(server_url) = &deployment.upload_urls.server_bundle {
        let spinner = make_spinner(json, "Uploading server bundle...");
        let server_entry = manifest_path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("server");
        if server_entry.is_dir() {
            let bundle = archive::create_tar_gz(&server_entry)
                .context("failed to create server bundle archive")?;
            let bundle_size = bundle.len();
            client
                .put_bytes(server_url, bundle, "application/gzip")
                .await
                .context("failed to upload server bundle")?;
            finish_spinner(
                spinner,
                &format!("Server bundle uploaded ({})", format_bytes(bundle_size)),
            );
        } else {
            finish_spinner(spinner, "Server bundle: no server dir found, skipping");
        }
    }

    // Signal upload complete
    let _: UploadCompleteResponse = client
        .post_empty(&format!(
            "/v1/deployments/{}/upload-complete",
            deployment.id
        ))
        .await
        .context("failed to signal upload complete")?;

    // Activate deployment
    let _: ActivateResponse = client
        .post_empty(&format!("/v1/deployments/{}/activate", deployment.id))
        .await
        .context("failed to activate deployment")?;

    // Poll for ready status
    let spinner = make_spinner(json, "Deploying...");
    let poll_interval = std::time::Duration::from_secs(3);
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(120);

    loop {
        tokio::time::sleep(poll_interval).await;

        if tokio::time::Instant::now() >= deadline {
            finish_spinner(spinner, "");
            bail!("deployment timed out after 120s");
        }

        let status: DeploymentStatus = client
            .get(&format!("/v1/deployments/{}/status", deployment.id))
            .await
            .context("failed to check deployment status")?;

        match status.status.as_str() {
            "live" => {
                finish_spinner(spinner, "");
                let url = status.url.as_deref().unwrap_or(&deployment.url);

                if json {
                    output::json_output(&DeployOutput {
                        deployment_id: deployment.id,
                        url: url.to_string(),
                        status: "live".into(),
                    });
                } else {
                    eprintln!();
                    eprintln!(
                        "  {} Deployed to {}",
                        console::style("✓").green().bold(),
                        console::style(url).underlined().bold(),
                    );
                    eprintln!();
                }
                return Ok(());
            }
            "failed" => {
                finish_spinner(spinner, "");
                let msg = status.error.unwrap_or_else(|| "unknown error".into());
                bail!("deployment failed: {msg}");
            }
            _ => continue,
        }
    }
}

fn detect_output_dir(project_dir: &Path) -> anyhow::Result<std::path::PathBuf> {
    for name in ["dist", ".output", "build"] {
        let candidate = project_dir.join(name);
        if candidate.is_dir() && candidate.join(".onreza").is_dir() {
            return Ok(candidate);
        }
    }
    bail!(
        "no output directory found in {}. Run your build first.",
        project_dir.display()
    );
}

fn git_cmd(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn make_spinner(json: bool, msg: &str) -> Option<ProgressBar> {
    if json {
        return None;
    }
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("  {spinner} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    spinner.set_message(msg.to_string());
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    Some(spinner)
}

fn finish_spinner(spinner: Option<ProgressBar>, msg: &str) {
    if let Some(s) = spinner {
        if msg.is_empty() {
            s.finish_and_clear();
        } else {
            s.finish_with_message(msg.to_string());
        }
    }
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
