#[cfg(test)]
mod deploy_tests;

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, bail};
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_FILES_HOBBY: usize = 500;
const MAX_FILES_PAID: usize = 5_000;
const MAX_SIZE_HOBBY: u64 = 100 * 1024 * 1024; // 100 MB
const MAX_SIZE_PAID: u64 = 1024 * 1024 * 1024; // 1 GB

use crate::api::ApiClient;
use crate::auth;
use crate::build;
use crate::cli::{BuildArgs, DeployArgs};
use crate::link;
use crate::migrations;
use crate::output;
use nrz::config::ProjectConfig;

// ── Workspace / plan ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceInfo {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    slug: String,
    subscription: Option<SubscriptionInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubscriptionInfo {
    plan_slug: String,
}

fn is_paid_plan(workspace: &WorkspaceInfo) -> bool {
    workspace
        .subscription
        .as_ref()
        .is_some_and(|s| s.plan_slug == "PRO" || s.plan_slug == "ENTERPRISE")
}

fn plan_limits(workspace: &WorkspaceInfo) -> (usize, u64) {
    if is_paid_plan(workspace) {
        (MAX_FILES_PAID, MAX_SIZE_PAID)
    } else {
        (MAX_FILES_HOBBY, MAX_SIZE_HOBBY)
    }
}

// ── API structs ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileEntry {
    path: String,
    size: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateDeploymentBody {
    manifest: serde_json::Value,
    files: Vec<FileEntry>,
    production: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    migrations: Option<Vec<migrations::Migration>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateDeploymentResponse {
    id: String,
    #[allow(dead_code)]
    status: String,
    url: String,
    #[allow(dead_code)]
    artifact_prefix: String,
    upload_urls: Vec<FileUploadUrl>,
    #[allow(dead_code)]
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileUploadUrl {
    #[allow(dead_code)]
    path: String,
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentStatusResponse {
    #[allow(dead_code)]
    id: String,
    status: String,
    url: Option<String>,
    #[allow(dead_code)]
    production: Option<bool>,
    error: Option<String>,
    #[allow(dead_code)]
    created_at: Option<String>,
    #[allow(dead_code)]
    ready_at: Option<String>,
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

// ── Main deploy flow ─────────────────────────────────────────

pub async fn run(
    args: DeployArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;

    // Verify auth early to avoid wasting time on build if token is invalid
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;

    // Run build step (default: enabled, skip with --skip-build)
    if !args.skip_build
        && let Some(cmd) =
            resolve_build_command(args.build_command.as_deref(), &project_dir, config)
    {
        run_build_step(&cmd, &project_dir, json)?;
    }

    // Fetch workspace info for plan-based limits
    let ws_info: WorkspaceInfo = client
        .get("/v1/workspace")
        .await
        .context("failed to fetch workspace info")?;

    // Resolve project: --project-id > onreza.toml > interactive
    let project_id = if let Some(pid) = &args.project_id {
        pid.clone()
    } else if let Some(id) = &config.project.id {
        id.clone()
    } else {
        if json {
            bail!(
                "no linked project. Use --project-id, set [project] id in onreza.toml, or run `nrz link` first."
            );
        }
        output::warn(false, "No linked project. Select one:");
        let selected = link::select_project_interactive(&client).await?;
        nrz::config::save_or_update(
            &project_dir,
            &selected.project_id,
            Some(&selected.project_name),
            None,
        )?;
        crate::init::add_to_gitignore(&project_dir);
        output::success(
            false,
            format!(
                "Linked to {}",
                console::style(&selected.project_name).bold()
            ),
        );
        selected.project_id
    };

    // Validate build output
    output::status(json, "~", "Validating build output...");
    build::run(
        BuildArgs {
            dir: project_dir.to_string_lossy().into_owned(),
            skip_validation: false,
        },
        json,
        config,
    )
    .await?;

    // Read manifest as raw JSON
    let output_dir = detect_output_dir(&project_dir, &config.output_dirs())?;
    let manifest_path = output_dir.join(".onreza/manifest.json");
    let manifest_raw: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?,
    )?;

    // Scan output directory for flat file list
    output::status(json, "~", "Scanning output directory...");
    let files = scan_files(&output_dir)?;
    if files.is_empty() {
        bail!("output directory is empty: {}", output_dir.display());
    }

    // Validate plan-based limits
    let (max_files, max_size) = plan_limits(&ws_info);
    if files.len() > max_files {
        bail!(
            "Deployment exceeds maximum file count ({} / {}). \
             Consider using blob storage for large assets. \
             For higher limits contact support@onreza.ru",
            files.len(),
            max_files
        );
    }
    let total_size: u64 = files.iter().map(|f| f.size).sum();
    if total_size > max_size {
        let total_mb = total_size / (1024 * 1024);
        let limit_mb = max_size / (1024 * 1024);
        bail!(
            "Deployment artifact size ({total_mb} MB) exceeds the {limit_mb} MB limit \
             for your plan. Use blob storage for large assets. \
             For higher limits contact support@onreza.ru"
        );
    }

    // Git info
    let branch = git_cmd(&["rev-parse", "--abbrev-ref", "HEAD"]);
    let commit_sha = match git_cmd(&["rev-parse", "HEAD"]) {
        Some(sha) => Some(sha),
        None => {
            output::warn(json, "git not available, using synthetic commit SHA");
            Some(synthetic_sha(&files))
        }
    };

    // Validate required env vars from [env] declarations
    if !args.skip_env_check {
        crate::cli::env_handler::validate_env_for_deploy(&client, &project_id, json, config)
            .await?;
    }

    // Sync detection results to API (best-effort, non-blocking)
    let sync_client = client.clone();
    let sync_project_id = project_id.clone();
    let sync_project_dir = project_dir.clone();
    let _sync_handle = tokio::spawn(async move {
        let detection_result = crate::detect::detect(&sync_project_dir);
        crate::detect_sync::sync_detection_to_api(
            &sync_client,
            &sync_project_id,
            &detection_result,
        )
        .await;
    });

    // Detect migrations
    let skip_mig = args.skip_migrations || config.skip_migrations();
    let mig_entries = detect_migrations(&project_dir, json, skip_mig, config.migrations_dir())
        .context("failed to scan migrations")?;

    // Create deployment
    output::status(json, "~", "Creating deployment...");
    let body = CreateDeploymentBody {
        manifest: manifest_raw,
        files,
        production: args.prod,
        branch,
        commit_sha,
        migrations: mig_entries,
    };
    let file_count = body.files.len();

    let deployment: CreateDeploymentResponse = client
        .post(&format!("/v1/projects/{}/deployments", project_id), &body)
        .await
        .context("failed to create deployment")?;

    // Validate server returned correct number of upload URLs
    if deployment.upload_urls.len() != file_count {
        bail!(
            "server returned {} upload URLs, but {} files were sent. \
             This may indicate an API version mismatch.",
            deployment.upload_urls.len(),
            file_count
        );
    }

    // Upload files in parallel (concurrency = 20)
    let spinner = make_spinner(
        json,
        &format!(
            "Uploading {file_count} files ({})...",
            format_bytes(total_size as usize)
        ),
    );

    let uploaded = AtomicUsize::new(0);
    let upload_results: Vec<anyhow::Result<()>> =
        stream::iter(deployment.upload_urls.iter().map(|file_url| {
            let client = &client;
            let output_dir = &output_dir;
            let spinner = &spinner;
            let uploaded = &uploaded;
            async move {
                let file_path = output_dir.join(&file_url.path);
                let data = tokio::fs::read(&file_path)
                    .await
                    .with_context(|| format!("failed to read {}", file_path.display()))?;

                let content_type = guess_content_type(&file_url.path);

                client
                    .put_bytes(&file_url.url, data, content_type)
                    .await
                    .with_context(|| format!("failed to upload {}", file_url.path))?;

                let done = uploaded.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(s) = spinner {
                    s.set_message(format!("[{done}/{file_count}] {}", file_url.path));
                }

                Ok(())
            }
        }))
        .buffer_unordered(20)
        .collect()
        .await;

    // Check for upload errors
    let errors: Vec<_> = upload_results.into_iter().filter_map(|r| r.err()).collect();
    if !errors.is_empty() {
        finish_spinner(spinner, "");
        let error_details: Vec<String> = errors.iter().map(|e| format!("{e:#}")).collect();

        if json {
            output::json_output(&serde_json::json!({
                "error": format!("{} of {file_count} file uploads failed", errors.len()),
                "failedUploads": error_details,
            }));
            std::process::exit(1);
        }

        for detail in &error_details {
            output::warn(false, format!("upload error: {detail}"));
        }
        bail!("{} of {file_count} file uploads failed", errors.len());
    }

    finish_spinner(
        spinner,
        &format!(
            "Uploaded {file_count} files ({})",
            format_bytes(total_size as usize)
        ),
    );

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

        let status: DeploymentStatusResponse = client
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
            "migration_failed" => {
                finish_spinner(spinner, "");
                let msg = status
                    .error
                    .unwrap_or_else(|| "migration failed during deployment".into());
                bail!("migration failed: {msg}");
            }
            "migrating" => {
                if let Some(ref s) = spinner {
                    s.set_message("Applying migrations...");
                }
                continue;
            }
            other => {
                if let Some(ref s) = spinner {
                    s.set_message(format!("Status: {other}..."));
                }
                continue;
            }
        }
    }
}

// ── Build step ───────────────────────────────────────────────

fn resolve_build_command(
    explicit: Option<&str>,
    project_dir: &Path,
    config: &ProjectConfig,
) -> Option<String> {
    if let Some(cmd) = explicit {
        return Some(cmd.to_string());
    }
    if let Some(cmd) = config.build_command() {
        return Some(cmd.to_string());
    }
    // Only auto-detect if package.json exists
    if !project_dir.join("package.json").exists() {
        return None;
    }
    let pm = crate::detect::detect_package_manager_name(project_dir);
    Some(format!("{pm} run build"))
}

fn run_build_step(cmd: &str, project_dir: &Path, json: bool) -> anyhow::Result<()> {
    if cmd.trim().is_empty() {
        anyhow::bail!("empty build command");
    }

    output::status(json, ">", format!("Building: {cmd}"));

    // Run through shell to support env vars, pipes, and paths with spaces
    #[cfg(unix)]
    let status = std::process::Command::new("sh")
        .args(["-c", cmd])
        .current_dir(project_dir)
        .status()
        .with_context(|| format!("failed to start build command: {cmd}"))?;

    #[cfg(windows)]
    let status = std::process::Command::new("cmd")
        .args(["/C", cmd])
        .current_dir(project_dir)
        .status()
        .with_context(|| format!("failed to start build command: {cmd}"))?;

    if !status.success() {
        match status.code() {
            Some(code) => anyhow::bail!("build failed with exit code {code}"),
            None => anyhow::bail!("build process was killed by signal"),
        }
    }

    output::success(json, "Build completed");
    Ok(())
}

// ── Migration detection ──────────────────────────────────────

fn detect_migrations(
    project_dir: &Path,
    json: bool,
    skip: bool,
    migrations_subdir: &str,
) -> anyhow::Result<Option<Vec<migrations::Migration>>> {
    if skip {
        return Ok(None);
    }

    let migrations_dir = project_dir.join(migrations_subdir);
    if !migrations_dir.is_dir() {
        return Ok(None);
    }

    let migs = migrations::scan_migrations_dir(project_dir, migrations_subdir)?;
    if migs.is_empty() {
        return Ok(None);
    }

    output::status(
        json,
        "~",
        format!(
            "Detected {} migration(s), will apply during activation",
            migs.len()
        ),
    );
    Ok(Some(migs))
}

// ── File scanning ────────────────────────────────────────────

fn scan_files(dir: &Path) -> anyhow::Result<Vec<FileEntry>> {
    let mut files = Vec::new();
    scan_dir_recursive(dir, dir, &mut files)?;
    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn scan_dir_recursive(
    base: &Path,
    current: &Path,
    files: &mut Vec<FileEntry>,
) -> anyhow::Result<()> {
    let entries = std::fs::read_dir(current)
        .with_context(|| format!("failed to read directory {}", current.display()))?;

    for entry in entries {
        let entry = entry?;
        let ft = entry.file_type()?;
        let path = entry.path();

        // Skip symlinks to avoid loops and directory traversal
        if ft.is_symlink() {
            continue;
        }

        if ft.is_dir() {
            scan_dir_recursive(base, &path, files)?;
        } else if ft.is_file() {
            let rel = path
                .strip_prefix(base)
                .context("failed to compute relative path")?;
            let rel_str = rel.to_string_lossy().replace('\\', "/");

            files.push(FileEntry {
                path: rel_str,
                size: entry.metadata()?.len(),
            });
        }
    }

    Ok(())
}

// ── Synthetic commit SHA ─────────────────────────────────────

fn synthetic_sha(files: &[FileEntry]) -> String {
    let mut hasher = Sha256::new();
    for f in files {
        hasher.update(format!("{}:{}\n", f.path, f.size).as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

// ── Content-Type guessing ────────────────────────────────────

fn guess_content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" | "htm" => "text/html",
        "js" | "mjs" | "cjs" => "application/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "txt" => "text/plain",
        "xml" => "application/xml",
        "ico" => "image/x-icon",
        "map" => "application/json",
        "webmanifest" => "application/manifest+json",
        _ => "application/octet-stream",
    }
}

// ── Helpers ──────────────────────────────────────────────────

fn detect_output_dir(
    project_dir: &Path,
    output_dirs: &[&str],
) -> anyhow::Result<std::path::PathBuf> {
    for name in output_dirs {
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
