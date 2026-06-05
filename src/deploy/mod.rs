#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod bundle;
#[cfg(test)]
mod bundle_tests;
#[cfg(test)]
mod deploy_tests;
pub(crate) mod health_check;
#[cfg(test)]
mod health_check_tests;
pub(crate) mod source_bundle_v1;
#[cfg(test)]
mod source_bundle_v1_tests;

use std::io::Read;
use std::num::NonZeroU64;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use bytes::Bytes;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::api::{ApiClient, PresignedHeadVerify, PresignedPutHeaders};
use crate::auth;
use crate::build;
use crate::build::manifest as build_manifest;
use crate::cli::{BuildArgs, DeployArgs};
use crate::deploy::source_bundle_v1::{
    CLI_PROTOCOL_VERSION, CompletedMultipartPart, PresignedSourceMultipartChunk,
    SOURCE_BUNDLE_FORMAT, SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS, SourceBundlePlan,
    build_source_bundle_plan, source_bundle_contract_characters,
};
use crate::detect::types::ComputeType;
use crate::link;
use crate::output;
use nrz::config::{EffectiveProjectConfig, HealthCheckPathConfig, ProjectConfig};
use nrz_contract::{
    CliMultipartCompleteResponse, CliPrepareUploadRequiredComplete, CliPrepareUploadResponse,
    CliPrepareUploadResponseMultipartChunk, CliPrepareUploadResponsePresignedPutHeaders,
    CliPrepareUploadResponsePresignedPutVerifyHead, CliUploadCompleteResponse,
    CliUploadFailedResponse,
};
use url::Url;
use uuid::Uuid;

const SOURCE_COMPLETION_RETRY_BUDGET: Duration = Duration::from_secs(30 * 60);
const SOURCE_COMPLETION_INITIAL_RETRY_DELAY: Duration = Duration::from_millis(500);
const SOURCE_COMPLETION_MAX_RETRY_DELAY: Duration = Duration::from_secs(5);
const PREPARE_UPLOAD_RETRY_BUDGET: Duration = Duration::from_secs(10 * 60);
const PREPARE_UPLOAD_INITIAL_RETRY_DELAY: Duration = Duration::from_millis(500);
const PREPARE_UPLOAD_MAX_RETRY_DELAY: Duration = Duration::from_secs(5);
const UPLOAD_FAILED_RETRY_BUDGET: Duration = Duration::from_secs(5 * 60);
const UPLOAD_FAILED_INITIAL_RETRY_DELAY: Duration = Duration::from_millis(500);
const UPLOAD_FAILED_MAX_RETRY_DELAY: Duration = Duration::from_secs(5);
const MAX_UPLOAD_FAILURE_LOG_LENGTH: usize = 4096;
const REDACTED_URL_COMPONENT: &str = "REDACTED";
const SOURCE_UPLOAD_PUT_FAILED: &str = "SOURCE_UPLOAD_PUT_FAILED";
const PNPM_BUILD_SCRIPT_COMPAT_ENV: [(&str, &str); 2] = [
    ("npm_config_dangerously_allow_all_builds", "true"),
    ("pnpm_config_dangerously_allow_all_builds", "true"),
];

// ── Workspace / plan ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceInfo {
    id: String,
    #[allow(dead_code)]
    slug: String,
}

// ── Project settings from server ─────────────────────────────

#[cfg(test)]
fn authoritative_server_framework_preset(preset: Option<&str>) -> Option<&str> {
    nrz::config::normalize_authoritative_framework(preset)
}

#[cfg(test)]
type ProjectInfo = nrz::config::ProjectBuildSettings;

// ── API structs ──────────────────────────────────────────────

/// Per-file identity entry used by the deployment-create body and by
/// SOURCE_BUNDLE_V1 logical manifest/archive construction.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileEntry {
    path: String,
    size: u64,
    content_hash: String,
}

/// Body for `POST /v1/projects/:id/deployments`.
///
/// `manifest` and `commitSha` are required by the server (`deployments.ts` body schema:
/// `manifest: ManifestSchema`, `commitSha: z.string().min(1)`). We mirror that on the
/// CLI side so a missing value fails Rust type checks instead of producing a cryptic
/// 400 from zod. `manifest` is filled by the build step (or the auto-gen fallbacks
/// in `run`); `commitSha` falls back to `synthetic_sha(&files)` when git isn't available.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateDeploymentBody {
    manifest: serde_json::Value,
    files: Vec<FileEntry>,
    production: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    commit_sha: String,
    /// ONREZA Functions published alongside this deployment. Function source is
    /// DB-backed and intentionally not part of SOURCE_BUNDLE_V1.
    #[serde(skip_serializing_if = "Option::is_none")]
    functions: Option<crate::functions::FunctionPublishPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateDeploymentResponse {
    id: String,
    #[allow(dead_code)]
    status: String,
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
    error_code: Option<String>,
    error_details: Option<DeploymentErrorDetails>,
    #[allow(dead_code)]
    created_at: Option<String>,
    #[allow(dead_code)]
    ready_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentErrorDetails {
    runtime_startup_failure: Option<RuntimeStartupFailureDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeStartupFailureDetails {
    code: Option<String>,
    message: Option<String>,
    check_type: Option<String>,
    health_path: Option<String>,
    expected_port: Option<u16>,
    #[serde(default)]
    detected_ports: Vec<u16>,
    timeout_seconds: Option<u64>,
    attempts: Option<u32>,
    last_error: Option<String>,
    process_entry: Option<String>,
    log_tail: Option<String>,
    retry_after_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
struct DeployOutput {
    deployment_id: String,
    url: String,
    status: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health_check: Option<HealthCheckInfo>,
}

fn format_deployment_failure(error: &str, status: &DeploymentStatusResponse) -> String {
    let Some(details) = status
        .error_details
        .as_ref()
        .and_then(|details| details.runtime_startup_failure.as_ref())
    else {
        return error.to_string();
    };

    let mut lines = Vec::new();
    lines.push(
        details
            .message
            .as_deref()
            .filter(|message| !message.trim().is_empty())
            .unwrap_or(error)
            .to_string(),
    );

    let mut facts = Vec::new();
    if let Some(code) = details.code.as_deref() {
        facts.push(format!("reason: {code}"));
    }
    if let Some(check_type) = details.check_type.as_deref() {
        let check = match (check_type, details.health_path.as_deref()) {
            ("http", Some(path)) => format!("HTTP {path}"),
            ("http", None) => "HTTP".to_string(),
            ("tcp", _) => "TCP".to_string(),
            (other, _) => other.to_string(),
        };
        facts.push(format!("check: {check}"));
    }
    if let Some(port) = details.expected_port {
        facts.push(format!("expected port: {port}"));
    }
    if !details.detected_ports.is_empty() {
        facts.push(format!(
            "detected ports: {}",
            details
                .detected_ports
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(timeout) = details.timeout_seconds {
        facts.push(format!("timeout: {timeout}s"));
    }
    if let Some(attempts) = details.attempts {
        facts.push(format!("attempts: {attempts}"));
    }
    if let Some(entry) = details.process_entry.as_deref() {
        facts.push(format!("entry: {entry}"));
    }
    if let Some(last_error) = details.last_error.as_deref() {
        facts.push(format!("last readiness error: {last_error}"));
    }
    if let Some(retry_after) = details.retry_after_seconds {
        facts.push(format!("retry after: {retry_after}s"));
    }

    if !facts.is_empty() {
        lines.push(format!("Runtime diagnostics: {}", facts.join("; ")));
    }

    if let Some(log_tail) = details
        .log_tail
        .as_deref()
        .filter(|tail| !tail.trim().is_empty())
    {
        lines.push(format!("Recent runtime output:\n{}", log_tail.trim()));
    }

    lines.join("\n")
}

/// JSON output for health check configuration.
///
/// Serializes as `{"mode":"http","path":"/health","source":"config"}`
/// or `{"mode":"tcp","source":"default"}` (no `path` field for TCP).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
enum HealthCheckInfo {
    Http {
        path: String,
        source: HealthCheckSourceTag,
    },
    Tcp {
        source: HealthCheckSourceTag,
    },
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum HealthCheckSourceTag {
    Flag,
    Config,
    Detected,
    Default,
}

/// Resolved health check configuration for a PROCESS deployment.
#[derive(Debug, Clone)]
struct ResolvedHealthCheck {
    /// The HTTP path (e.g. `/health`), or `None` for TCP-only.
    path: Option<String>,
    /// Where the value came from.
    source: HealthCheckSource,
}

#[derive(Debug, Clone, Copy)]
enum HealthCheckSource {
    Flag,
    Config,
    Detected,
    Default,
}

impl HealthCheckSource {
    fn to_tag(self) -> HealthCheckSourceTag {
        match self {
            Self::Flag => HealthCheckSourceTag::Flag,
            Self::Config => HealthCheckSourceTag::Config,
            Self::Detected => HealthCheckSourceTag::Detected,
            Self::Default => HealthCheckSourceTag::Default,
        }
    }
}

impl ResolvedHealthCheck {
    fn to_info(&self) -> HealthCheckInfo {
        match &self.path {
            Some(path) => HealthCheckInfo::Http {
                path: path.clone(),
                source: self.source.to_tag(),
            },
            None => HealthCheckInfo::Tcp {
                source: self.source.to_tag(),
            },
        }
    }
}

// ── Main deploy flow ─────────────────────────────────────────

/// Discover ONREZA Functions, run the local policy preview (fail-fast before
/// upload), and assemble the publish payload. Returns `None` when the project
/// has neither functions nor edge rules.
fn build_functions_payload(
    _config: &ProjectConfig,
    project_dir: &Path,
    json: bool,
) -> anyhow::Result<Option<crate::functions::FunctionPublishPayload>> {
    let collected = crate::functions::collect(project_dir)?;
    let edge_rules = crate::functions::load_edge_rules(project_dir)?;
    let edge_rule_count = edge_rules
        .as_ref()
        .map_or(0, crate::functions::edge_rule_count);

    if collected.is_empty() && edge_rules.is_none() {
        return Ok(None);
    }

    let mut violation_count = 0usize;
    for function in &collected.functions {
        let report = crate::functions::run_policy_preview(&function.entrypoint, &function.sources)?;
        if report.status == nrz_fn_policy::PolicyStatus::Failed {
            violation_count += report.violations.len();
            for violation in &report.violations {
                let location = violation.importer.as_deref().unwrap_or(&report.entrypoint);
                output::warn(
                    json,
                    format!(
                        "{} ({location}): {} — {}",
                        function.name, violation.capability, violation.reason
                    ),
                    output::Phase::Deploy,
                );
            }
        }
    }
    if violation_count > 0 {
        return Err(output::coded_error(
            "ONREZA_FUNCTIONS_POLICY",
            format!("function policy check failed with {violation_count} violation(s)"),
        ));
    }
    output::success(
        json,
        format_function_publish_summary(
            collected.functions.len(),
            collected.source_file_count(),
            edge_rule_count,
        ),
        output::Phase::Deploy,
    );
    Ok(Some(crate::functions::build_payload(
        "DEPLOYMENT",
        &collected,
        edge_rules,
    )))
}

pub async fn run(
    args: DeployArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let root_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;
    let project_context = crate::project_context::resolve(&root_dir, config, args.app.as_deref())?;
    if let Some(app) = &project_context.selected_app {
        output::status(
            json,
            "~",
            format!(
                "Monorepo: deploying app \"{}\" from {}/",
                app.requested, app.path
            ),
            output::Phase::Deploy,
        );
    }
    let project_dir = project_context.project_dir.clone();
    let config = &project_context.config;

    // Verify auth early to avoid wasting time on build if token is invalid
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;
    let resume_deployment_id = args.resume_deployment.as_deref().map(str::trim);
    if let Some(deployment_id) = resume_deployment_id
        && deployment_id.is_empty()
    {
        return Err(output::coded_error(
            "INVALID_ARGUMENT",
            "--resume-deployment requires a non-empty deployment ID".to_string(),
        ));
    }

    // Early-resolve project_id before build/detect so server settings can be
    // imported into the same effective config as local onreza.toml.
    let mut early_project_id = args
        .project_id
        .as_deref()
        .or(config.project.id.as_deref())
        .map(String::from);
    if early_project_id.is_none()
        && let Some(deployment_id) = resume_deployment_id
    {
        early_project_id =
            Some(resolve_project_id_for_resume(&client, deployment_id, None, config).await?);
    }

    // Fetch project settings from server if project_id is known
    let server_settings = if let Some(ref pid) = early_project_id {
        match crate::project_settings::fetch_for_effective_config(&client, pid).await? {
            crate::project_settings::ProjectSettingsFetch::Applied(info) => {
                tracing::info!(
                    ?info.build_command,
                    ?info.build_command_source,
                    ?info.install_command,
                    ?info.install_command_source,
                    ?info.output_directory,
                    ?info.output_directory_source,
                    ?info.framework_preset,
                    "fetched project settings from server"
                );
                Some(info)
            }
            crate::project_settings::ProjectSettingsFetch::TransientFailure { message } => {
                output::warn(
                    json,
                    format!(
                        "Could not fetch project settings: {message}. Using local configuration."
                    ),
                    output::Phase::Deploy,
                );
                None
            }
        }
    } else {
        None
    };

    let mut effective =
        EffectiveProjectConfig::from_project_config(project_dir.clone(), config.clone());
    effective.apply_server_settings(server_settings.as_ref());

    // Explicit compute intent is safe to resolve before build because it comes
    // only from CLI/config. Framework detection stays post-build: generated
    // outputs such as root index.html are part of the detection surface.
    let explicit_compute =
        resolve_explicit_compute_type(args.compute.as_deref(), effective.deploy_compute())?;
    validate_prebuild_compute_intent(&project_dir, explicit_compute)?;

    // Run install step (default: enabled, skip with --skip-install or --skip-build)
    if !args.skip_build && !args.skip_install {
        run_install_step(&project_dir, json, &effective)?;
    }

    // Pre-build env injection for framework compatibility.
    let build_env: Vec<(String, String)> = {
        let mut env = Vec::new();

        // Next.js: inject NEXT_PRIVATE_STANDALONE=1 so users don't have to manually set
        // `output: 'standalone'` in next.config. No-op when user has `output: 'export'`.
        if is_nextjs_project(&project_dir) {
            output::status(
                json,
                "~",
                "Next.js detected, enabling standalone output (NEXT_PRIVATE_STANDALONE=1)",
                output::Phase::Deploy,
            );
            env.push(("NEXT_PRIVATE_STANDALONE".to_string(), "1".to_string()));
        }

        // SvelteKit: adapter-auto checks platform env vars to pick an adapter.
        // GCP_BUILDPACKS makes it choose adapter-node, which produces the build/
        // output we expect. Without this, adapter-auto fails silently on unknown hosts.
        if is_sveltekit_with_adapter_auto(&project_dir) {
            output::status(
                json,
                "~",
                "SvelteKit adapter-auto detected, enabling adapter-node (GCP_BUILDPACKS=1)",
                output::Phase::Deploy,
            );
            env.push(("GCP_BUILDPACKS".to_string(), "1".to_string()));
        }

        env
    };

    // Run build step (default: enabled, skip with --skip-build)
    if !args.skip_build
        && let Some(cmd) =
            resolve_build_command(args.build_command.as_deref(), &project_dir, &effective)
    {
        run_build_step(&cmd, &project_dir, json, &build_env)?;
    }

    // Detect after build so generated static HTML roots, manifests, and output
    // markers participate in framework/output resolution.
    let detection =
        crate::detect::detect_with_framework_override(&project_dir, effective.framework_override());

    // Validate build output
    output::status(
        json,
        "~",
        "Validating build output...",
        output::Phase::Deploy,
    );
    let build_result = build::run_with_effective_config(
        BuildArgs {
            dir: project_dir.to_string_lossy().into_owned(),
            skip_validation: false,
        },
        json,
        &effective,
        Some(&detection),
    )
    .await?;

    let output_dir = build_result.output_dir;
    let loaded_manifest = build_result.manifest;
    let has_manifest = loaded_manifest.is_some();

    // Scan output directory into a flat file list with streaming SHA-256 + size per
    // file. Pre-compression is gone (RFC: EDGE_DYNAMIC_ENCODING) — the edge serves
    // identity bytes and compresses on the fly, so the CLI only ships raw content
    // addressed by sha256. Run in a blocking task: filesystem I/O + CPU-bound hashing.
    output::status(
        json,
        "~",
        "Scanning output directory...",
        output::Phase::Deploy,
    );
    let output_dir_for_scan = output_dir.clone();
    let scanned_files = tokio::task::spawn_blocking(move || scan_dir(&output_dir_for_scan))
        .await
        .context("file scan task failed (panic or runtime shutdown)")??;

    let compute =
        resolve_deploy_compute_type(explicit_compute, loaded_manifest.as_ref(), &detection);
    let mut warnings: Vec<String> = Vec::new();

    // Inform about SSR framework compute mode when auto-detected
    if !has_manifest
        && args.compute.is_none()
        && effective.deploy_compute().is_none()
        && crate::detect::presets::is_ssr_framework(&detection.framework)
    {
        let msg = match compute {
            ComputeType::Process => {
                let mut m = format!("{} deploying as PROCESS (server runtime).", detection.name);
                let hint = framework_static_hint(&detection.framework);
                if !hint.is_empty() {
                    m.push_str(&format!(
                        " For a fully static export, {hint} and redeploy with --compute static."
                    ));
                }
                m
            }
            ComputeType::Static => format!(
                "{} deploying as STATIC. \
                 For server-side rendering, use --compute process.",
                detection.name
            ),
        };
        output::warn(json, &msg, output::Phase::Deploy);
        warnings.push(msg);
    }

    let is_process = compute == ComputeType::Process;

    // manifest_raw starts from build result (may already be Some for STATIC auto-gen)
    let mut manifest_raw: Option<serde_json::Value> = loaded_manifest
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .context("failed to serialize manifest")?;

    // When a manifest is present, process entry comes from the manifest's COMPUTE layer —
    // skip pre-flight validation and auto-detection.
    // When no manifest: find entry and auto-generate COMPUTE manifest.
    if is_process && !has_manifest {
        validate_process_output(&output_dir, &project_dir, &detection)
            .map_err(|e| output::with_default_code(e, "MISSING_PROCESS_ENTRY"))?;
        let (entry, warning) = ensure_process_entry(
            &output_dir,
            &project_dir,
            effective.deploy_entry(),
            &detection,
            json,
        )
        .map_err(|e| output::with_default_code(e, "MISSING_PROCESS_ENTRY"))?;
        if let Some(ref w) = warning {
            output::warn(json, w, output::Phase::Deploy);
            warnings.push(w.clone());
        }
        match entry {
            Some(ref e) => {
                let auto = build_manifest::generate_compute_manifest(e);
                output::status(
                    json,
                    "~",
                    format!("Auto-generated COMPUTE manifest (entry: {e})"),
                    output::Phase::Deploy,
                );
                manifest_raw = Some(
                    serde_json::to_value(&auto)
                        .context("failed to serialize auto-generated manifest")?,
                );
            }
            None => {
                return Err(output::coded_error(
                    "MISSING_PROCESS_ENTRY",
                    format!(
                        "Cannot auto-generate COMPUTE manifest: entry point not detected in {}.\n\n\
                         Create .onreza/manifest.json manually.\n\
                         See: docs.onreza.ru/manifest",
                        output_dir.display()
                    ),
                ));
            }
        }
    }

    let manifest_raw = resolve_manifest_for_compute(compute, manifest_raw)?;
    let manifest_for_planning: build_manifest::Manifest =
        serde_json::from_value(manifest_raw.clone())
            .context("failed to parse resolved deployment manifest")?;
    let files = prepare_deploy_files(&manifest_for_planning, scanned_files, &detection, json)?;
    if files.is_empty() {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!(
                "output directory has no deployable files after framework normalization: {}",
                output_dir.display()
            ),
        ));
    }
    let functions = build_functions_payload(effective.config(), &project_dir, json)?;
    let has_compute_layer = manifest_has_compute_layer(&manifest_for_planning);

    // Resolve health check path (PROCESS only)
    let health_check = if has_compute_layer {
        Some(resolve_health_check(
            args.health_check_path.as_deref(),
            config,
            &project_dir,
            &detection,
            &output_dir,
            json,
        )?)
    } else {
        None
    };

    // SOURCE_BUNDLE_V1 prepare-upload needs the authenticated workspace ID for
    // direct CLI deploy and builder resume. Admission/limits are enforced
    // server-side from the logical manifest plus source archive descriptor.
    let ws_info: WorkspaceInfo = client
        .get("/v1/workspace")
        .await
        .context("failed to fetch workspace info")?;

    // ── Resume mode: builder calls us with an existing deployment ID ──
    if let Some(deployment_id) = &args.resume_deployment {
        let deployment_id = deployment_id.trim();
        let project_id = match early_project_id.clone() {
            Some(project_id) => project_id,
            None => {
                resolve_project_id_for_resume(
                    &client,
                    deployment_id,
                    args.project_id.as_deref(),
                    config,
                )
                .await?
            }
        };
        return resume_deploy(
            &client,
            deployment_id,
            &ws_info.id,
            &project_id,
            functions,
            manifest_raw,
            manifest_for_planning,
            files,
            &output_dir,
            json,
            warnings,
        )
        .await;
    }

    // ── Normal flow continues below ─────────────────────────────────

    // Resolve project: --project-id > onreza.toml > interactive
    let project_id = if let Some(pid) = &args.project_id {
        pid.clone()
    } else if let Some(id) = effective.project_id() {
        id.to_string()
    } else {
        if json {
            bail!(
                "no linked project. Use --project-id, set [project] id in onreza.toml, or run `nrz link` first."
            );
        }
        output::warn(
            false,
            "No linked project. Select one:",
            output::Phase::Deploy,
        );
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
            output::Phase::Deploy,
        );
        selected.project_id
    };

    // Git info
    let branch = git_cmd(&["rev-parse", "--abbrev-ref", "HEAD"]);
    let commit_sha = git_cmd(&["rev-parse", "HEAD"]).unwrap_or_else(|| {
        output::warn(
            json,
            "git not available, using synthetic commit SHA",
            output::Phase::Deploy,
        );
        synthetic_sha(&files)
    });

    // Validate required env vars from [env] declarations
    if !args.skip_env_check {
        crate::cli::env_handler::validate_env_for_deploy(&client, &project_id, json, config)
            .await?;
    }

    // Sync detection results to API (best-effort, non-blocking)
    let sync_client = client.clone();
    let sync_project_id = project_id.clone();
    let _sync = tokio::spawn(async move {
        crate::detect_sync::sync_detection_to_api(&sync_client, &sync_project_id, &detection).await;
    });

    // Sync compute config (health check path) for PROCESS deployments
    if let Some(ref hc) = health_check {
        let hc_client = client.clone();
        let hc_project_id = project_id.clone();
        let hc_clone = hc.clone();
        let _hc = tokio::spawn(async move {
            sync_compute_config(&hc_client, &hc_project_id, &hc_clone, json).await;
        });
    }

    output::status(
        json,
        "~",
        "Creating SOURCE_BUNDLE_V1 archive...",
        output::Phase::Deploy,
    );
    let upload_plan = build_source_bundle_plan(&output_dir, &manifest_for_planning, &files)
        .context("failed to prepare SOURCE_BUNDLE_V1 upload plan")?;
    output::success(
        json,
        format!(
            "SOURCE_BUNDLE_V1 archive created ({}, sha256: {}...)",
            format_u64_bytes(upload_plan.source_size_bytes),
            &upload_plan.source_sha256[..12]
        ),
        output::Phase::Deploy,
    );
    let deployment_attempt_id = Uuid::now_v7().to_string();

    // Create deployment
    output::status(json, "~", "Creating deployment...", output::Phase::Deploy);
    let body = CreateDeploymentBody {
        manifest: manifest_raw,
        files: files.clone(),
        production: args.prod,
        branch,
        commit_sha,
        functions,
    };

    let deployment: CreateDeploymentResponse = client
        .post(&format!("/v1/projects/{}/deployments", project_id), &body)
        .await
        .context("failed to create deployment")?;

    prepare_upload_and_complete(
        &client,
        &deployment.id,
        &ws_info.id,
        &project_id,
        &deployment_attempt_id,
        json,
        &upload_plan,
    )
    .await?;

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
                        warnings,
                        health_check: health_check.as_ref().map(|hc| hc.to_info()),
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
                let msg = status.error.as_deref().unwrap_or("unknown error");
                bail!(
                    "deployment failed: {}",
                    format_deployment_failure(msg, &status)
                );
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

// ── Health check resolution ──────────────────────────────────

/// Resolve health check path for PROCESS deployments.
///
/// Priority: CLI flag > config > autodetect > TCP default.
fn resolve_health_check(
    cli_flag: Option<&str>,
    config: &ProjectConfig,
    project_dir: &Path,
    detection: &crate::detect::types::DetectionResult,
    output_dir: &Path,
    json: bool,
) -> anyhow::Result<ResolvedHealthCheck> {
    // 1. CLI flag
    if let Some(flag) = cli_flag {
        if flag.eq_ignore_ascii_case("none")
            || flag.eq_ignore_ascii_case("false")
            || flag.eq_ignore_ascii_case("tcp")
        {
            output::success(
                json,
                "Health check: TCP (from --health-check-path)",
                output::Phase::Deploy,
            );
            return Ok(ResolvedHealthCheck {
                path: None,
                source: HealthCheckSource::Flag,
            });
        }
        validate_health_path(flag, "--health-check-path")?;
        output::success(
            json,
            format!("Health check: HTTP {flag} (from --health-check-path)"),
            output::Phase::Deploy,
        );
        return Ok(ResolvedHealthCheck {
            path: Some(flag.to_string()),
            source: HealthCheckSource::Flag,
        });
    }

    // 2. Config
    if let Some(hc) = config.health_check_path() {
        match hc {
            HealthCheckPathConfig::Tcp => {
                output::success(
                    json,
                    "Health check: TCP (configured)",
                    output::Phase::Deploy,
                );
                return Ok(ResolvedHealthCheck {
                    path: None,
                    source: HealthCheckSource::Config,
                });
            }
            HealthCheckPathConfig::Http(path) => {
                output::success(
                    json,
                    format!("Health check: HTTP {path} (from config)"),
                    output::Phase::Deploy,
                );
                return Ok(ResolvedHealthCheck {
                    path: Some(path.clone()),
                    source: HealthCheckSource::Config,
                });
            }
        }
    }

    // 3. Autodetect
    if let Some(det) =
        health_check::detect_health_path(project_dir, &detection.framework, output_dir)
    {
        output::success(
            json,
            format!(
                "Found health endpoint: {} (source: {})",
                det.path, det.source_description
            ),
            output::Phase::Deploy,
        );
        return Ok(ResolvedHealthCheck {
            path: Some(det.path),
            source: HealthCheckSource::Detected,
        });
    }

    // 4. Default: TCP
    output::status(
        json,
        "ℹ",
        "No health check endpoint detected. Using TCP readiness check.\n    \
         To add HTTP health check, create a /health endpoint or set\n    \
         deploy.health_check_path in onreza.toml",
        output::Phase::Deploy,
    );
    Ok(ResolvedHealthCheck {
        path: None,
        source: HealthCheckSource::Default,
    })
}

/// Validate an HTTP health check path.
fn validate_health_path(path: &str, source: &str) -> anyhow::Result<()> {
    if !path.starts_with('/') {
        return Err(output::coded_error(
            "INVALID_ARGUMENT",
            format!("{source} must start with '/', got: \"{path}\""),
        ));
    }
    if path.contains("..") {
        return Err(output::coded_error(
            "INVALID_ARGUMENT",
            format!("{source} must not contain '..', got: \"{path}\""),
        ));
    }
    if path.contains('?') || path.contains('#') {
        return Err(output::coded_error(
            "INVALID_ARGUMENT",
            format!("{source} must not contain query or fragment, got: \"{path}\""),
        ));
    }
    Ok(())
}

// ── Compute config sync ──────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ComputeConfigBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    health_check_path: Option<String>,
}

/// Best-effort sync of compute config (health check path) to the platform.
async fn sync_compute_config(
    client: &ApiClient,
    project_id: &str,
    health_check: &ResolvedHealthCheck,
    json: bool,
) {
    let body = ComputeConfigBody {
        health_check_path: health_check.path.clone(),
    };

    let path = format!("/v1/compute-config/{project_id}");
    let resp: Result<serde_json::Value, _> = client.put(&path, &body).await;
    if let Err(e) = resp {
        output::warn(
            json,
            format!("failed to sync compute config: {e}"),
            output::Phase::Deploy,
        );
    }
}

// ── Shared upload step ───────────────────────────────────────

/// Drive the SOURCE_BUNDLE_V1 upload protocol:
/// prepare-upload → source object PUT(s) → multipart-complete? → upload-complete.
#[allow(clippy::too_many_arguments)]
async fn prepare_upload_and_complete(
    client: &ApiClient,
    deployment_id: &str,
    workspace_id: &str,
    project_id: &str,
    deployment_attempt_id: &str,
    json: bool,
    plan: &SourceBundlePlan,
) -> anyhow::Result<()> {
    output::status(
        json,
        "~",
        "Preparing SOURCE_BUNDLE_V1 upload...",
        output::Phase::Deploy,
    );

    let body = PrepareUploadBody {
        deployment_id: deployment_id.to_string(),
        workspace_id: workspace_id.to_string(),
        project_id: project_id.to_string(),
        deployment_attempt_id: deployment_attempt_id.to_string(),
        operation_id: Uuid::now_v7().to_string(),
        artifact_format: "SOURCE_BUNDLE_V1".to_string(),
        cli_protocol_version: CLI_PROTOCOL_VERSION.to_string(),
        logical_manifest_summary: plan.logical_manifest_summary.clone(),
        logical_manifest_sha256: plan.logical_manifest_sha256.clone(),
        source_format: SOURCE_BUNDLE_FORMAT.to_string(),
        source_sha256: plan.source_sha256.clone(),
        source_size_bytes: plan.source_size_string(),
        multipart: plan.multipart.clone(),
    };

    let prepared = prepare_upload_with_retry(client, deployment_id, &body, json).await?;

    let multipart_completion = match upload_source_object(client, &prepared, plan, json).await {
        Ok(completion) => completion,
        Err(error) => {
            report_source_object_upload_failed(
                client,
                deployment_id,
                deployment_attempt_id,
                &prepared,
                &error,
                json,
            )
            .await;
            return Err(error);
        }
    };
    // Completion endpoints are idempotent, but response failures are ambiguous:
    // the server may already have accepted the completion before the CLI hears
    // back. Do not send upload-failed after this point.
    complete_source_multipart_if_needed(
        client,
        deployment_id,
        deployment_attempt_id,
        &prepared,
        multipart_completion,
        json,
    )
    .await?;

    let upload_session_id = prepared.upload_session_id.to_string();
    complete_upload_with_retry(
        client,
        deployment_id,
        &upload_session_id,
        deployment_attempt_id,
        json,
        plan,
        prepared.source_artifact_id.as_str(),
    )
    .await?;

    Ok(())
}

// ── Resume deploy flow ───────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrepareUploadBody {
    deployment_id: String,
    workspace_id: String,
    project_id: String,
    deployment_attempt_id: String,
    operation_id: String,
    artifact_format: String,
    cli_protocol_version: String,
    logical_manifest_summary: source_bundle_v1::SourceLogicalManifestSummary,
    logical_manifest_sha256: String,
    source_format: String,
    source_sha256: String,
    source_size_bytes: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    multipart: Option<source_bundle_v1::SourceBundleMultipartDescriptor>,
}

type PrepareUploadResponse = CliPrepareUploadResponse;
type RequiredComplete = CliPrepareUploadRequiredComplete;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadCompleteBody {
    deployment_id: String,
    upload_session_id: String,
    deployment_attempt_id: String,
    operation_id: String,
    artifact_format: String,
    source_artifact_id: String,
    source_sha256: String,
    source_size_bytes: String,
    logical_manifest_sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadFailedBody {
    deployment_id: String,
    upload_session_id: String,
    deployment_attempt_id: String,
    operation_id: String,
    artifact_format: String,
    source_artifact_id: String,
    error_code: String,
    error_log: String,
}

type UploadCompleteResponse = CliUploadCompleteResponse;

type UploadFailedResponse = CliUploadFailedResponse;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MultipartCompleteBody {
    deployment_id: String,
    upload_session_id: String,
    deployment_attempt_id: String,
    operation_id: String,
    artifact_format: String,
    source_artifact_id: String,
    upload_id: String,
    parts: Vec<CompletedMultipartPart>,
}

type MultipartCompleteResponse = CliMultipartCompleteResponse;

struct SourceMultipartCompletion {
    upload_id: String,
    parts: Vec<CompletedMultipartPart>,
}

async fn prepare_upload_with_retry(
    client: &ApiClient,
    deployment_id: &str,
    body: &PrepareUploadBody,
    json: bool,
) -> anyhow::Result<PrepareUploadResponse> {
    let started = Instant::now();
    let mut delay = PREPARE_UPLOAD_INITIAL_RETRY_DELAY;
    let mut attempts = 0u32;

    loop {
        attempts += 1;
        match client
            .post(
                &format!("/v1/deployments/{deployment_id}/prepare-upload"),
                body,
            )
            .await
        {
            Ok(resp) => return Ok(resp),
            Err(error) => {
                let Some(retry) = classify_prepare_upload_retry_error(&error) else {
                    if json
                        && let Some(api_err) =
                            error.downcast_ref::<crate::api::StructuredApiError>()
                        && (api_err.code == "LIMIT_EXCEEDED"
                            || api_err.code == "SUBSCRIPTION_REQUIRED")
                    {
                        output::log_error_structured(
                            "deploy",
                            &api_err.message,
                            &api_err.code,
                            api_err.details.as_ref(),
                        );
                    }
                    return Err(error.context("failed to prepare upload"));
                };

                let elapsed = started.elapsed();
                if elapsed >= PREPARE_UPLOAD_RETRY_BUDGET {
                    return Err(error.context(format!(
                        "failed to prepare upload after waiting {:?}",
                        PREPARE_UPLOAD_RETRY_BUDGET
                    )));
                }

                if attempts == 1 {
                    let message = match retry.reason {
                        SourceControlPlaneRetryReason::ControlPlaneBackpressure => {
                            "Waiting for artifact ingest capacity...".to_string()
                        }
                        reason => format!("Waiting for prepare-upload ({})...", reason.as_str()),
                    };
                    output::status(json, "~", message, output::Phase::Deploy);
                }

                let remaining = PREPARE_UPLOAD_RETRY_BUDGET.saturating_sub(elapsed);
                let sleep_for = retry_delay_with_hint(
                    retry.retry_after,
                    delay,
                    PREPARE_UPLOAD_MAX_RETRY_DELAY,
                    remaining,
                );
                tokio::time::sleep(sleep_for).await;
                delay = delay.saturating_mul(2).min(PREPARE_UPLOAD_MAX_RETRY_DELAY);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceControlPlaneRetryReason {
    PrepareUploadInProgress,
    S3Visibility,
    OwnerVerifyInProgress,
    CompletionInProgress,
    FailureReportInProgress,
    ControlPlaneBackpressure,
    TransportAmbiguous,
}

impl SourceControlPlaneRetryReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::PrepareUploadInProgress => "prepare-upload is still in progress",
            Self::S3Visibility => "S3 objects are not visible yet",
            Self::OwnerVerifyInProgress => "owner verification is still in progress",
            Self::CompletionInProgress => "source completion is still in progress",
            Self::FailureReportInProgress => "upload failure report is still in progress",
            Self::ControlPlaneBackpressure => "artifact ingest capacity is saturated",
            Self::TransportAmbiguous => "control-plane response was not received",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceControlPlaneRetry {
    reason: SourceControlPlaneRetryReason,
    retry_after: Option<Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceCompletionAttempt {
    Terminal,
    Retry(SourceControlPlaneRetry),
}

fn classify_prepare_upload_retry_error(error: &anyhow::Error) -> Option<SourceControlPlaneRetry> {
    classify_standard_control_plane_retry_error(
        error,
        SourceControlPlaneRetryReason::PrepareUploadInProgress,
    )
}

async fn complete_upload_with_retry(
    client: &ApiClient,
    deployment_id: &str,
    upload_session_id: &str,
    deployment_attempt_id: &str,
    json: bool,
    plan: &SourceBundlePlan,
    source_artifact_id: &str,
) -> anyhow::Result<()> {
    let started = Instant::now();
    let mut delay = SOURCE_COMPLETION_INITIAL_RETRY_DELAY;
    let mut attempts = 0u32;
    let body = UploadCompleteBody {
        deployment_id: deployment_id.to_string(),
        upload_session_id: upload_session_id.to_string(),
        deployment_attempt_id: deployment_attempt_id.to_string(),
        operation_id: Uuid::now_v7().to_string(),
        artifact_format: "SOURCE_BUNDLE_V1".to_string(),
        source_artifact_id: source_artifact_id.to_string(),
        source_sha256: plan.source_sha256.clone(),
        source_size_bytes: plan.source_size_string(),
        logical_manifest_sha256: plan.logical_manifest_sha256.clone(),
    };

    loop {
        attempts += 1;
        match post_upload_complete_once(client, deployment_id, &body).await? {
            SourceCompletionAttempt::Terminal => return Ok(()),
            SourceCompletionAttempt::Retry(retry) => {
                let elapsed = started.elapsed();
                if elapsed >= SOURCE_COMPLETION_RETRY_BUDGET {
                    bail!(
                        "upload-complete did not reach a terminal state after {:?} (last state: {})",
                        SOURCE_COMPLETION_RETRY_BUDGET,
                        retry.reason.as_str()
                    );
                }

                if attempts == 1 {
                    output::status(
                        json,
                        "~",
                        format!("Waiting for upload-complete ({})...", retry.reason.as_str()),
                        output::Phase::Deploy,
                    );
                }

                let remaining = SOURCE_COMPLETION_RETRY_BUDGET.saturating_sub(elapsed);
                let sleep_for = retry_delay_with_hint(
                    retry.retry_after,
                    delay,
                    SOURCE_COMPLETION_MAX_RETRY_DELAY,
                    remaining,
                );
                tokio::time::sleep(sleep_for).await;
                delay = delay
                    .saturating_mul(2)
                    .min(SOURCE_COMPLETION_MAX_RETRY_DELAY);
            }
        }
    }
}

async fn post_upload_complete_once(
    client: &ApiClient,
    deployment_id: &str,
    body: &UploadCompleteBody,
) -> anyhow::Result<SourceCompletionAttempt> {
    match client
        .post::<_, UploadCompleteResponse>(
            &format!("/v1/deployments/{deployment_id}/upload-complete"),
            body,
        )
        .await
    {
        Ok(response) => classify_upload_complete_response(response),
        Err(error) => match classify_upload_complete_retry_error(&error) {
            Some(reason) => Ok(SourceCompletionAttempt::Retry(reason)),
            None => Err(error.context("failed to signal upload complete")),
        },
    }
}

fn classify_upload_complete_response(
    response: UploadCompleteResponse,
) -> anyhow::Result<SourceCompletionAttempt> {
    match response {
        UploadCompleteResponse::SourceUploadCompleted { .. }
        | UploadCompleteResponse::SourceFastPathCompleted { .. }
        | UploadCompleteResponse::SourceVerifiedAwaitingRuntime { .. }
        | UploadCompleteResponse::NoopAlreadyCompleted { .. } => {
            Ok(SourceCompletionAttempt::Terminal)
        }
        UploadCompleteResponse::Incomplete { .. } => {
            Ok(SourceCompletionAttempt::Retry(SourceControlPlaneRetry {
                reason: SourceControlPlaneRetryReason::S3Visibility,
                retry_after: None,
            }))
        }
        UploadCompleteResponse::Expired { expired_at, .. } => {
            bail!("upload window expired at {expired_at}; create a new deployment and upload again")
        }
    }
}

fn classify_upload_complete_retry_error(error: &anyhow::Error) -> Option<SourceControlPlaneRetry> {
    if let Some(api_error) = error.downcast_ref::<crate::api::StructuredApiError>()
        && api_error.code == "VALIDATION_ERROR"
        && api_error
            .message
            .to_ascii_lowercase()
            .contains("upload is incomplete")
    {
        return Some(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::S3Visibility,
            retry_after: None,
        });
    };

    classify_standard_control_plane_retry_error(
        error,
        SourceControlPlaneRetryReason::OwnerVerifyInProgress,
    )
}

fn classify_multipart_complete_retry_error(
    error: &anyhow::Error,
) -> Option<SourceControlPlaneRetry> {
    classify_standard_control_plane_retry_error(
        error,
        SourceControlPlaneRetryReason::CompletionInProgress,
    )
}

fn classify_upload_failed_retry_error(error: &anyhow::Error) -> Option<SourceControlPlaneRetry> {
    classify_standard_control_plane_retry_error(
        error,
        SourceControlPlaneRetryReason::FailureReportInProgress,
    )
}

fn classify_standard_control_plane_retry_error(
    error: &anyhow::Error,
    in_progress_reason: SourceControlPlaneRetryReason,
) -> Option<SourceControlPlaneRetry> {
    let Some(api_error) = error.downcast_ref::<crate::api::StructuredApiError>() else {
        return classify_control_plane_transport_retry_error(error);
    };
    match api_error.code.as_str() {
        "OPERATION_IN_PROGRESS" => Some(SourceControlPlaneRetry {
            reason: in_progress_reason,
            retry_after: None,
        }),
        "SERVICE_UNAVAILABLE" | "TOO_MANY_REQUESTS" => Some(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::ControlPlaneBackpressure,
            retry_after: api_error.retry_after_seconds.map(Duration::from_secs),
        }),
        _ => classify_control_plane_transport_retry_error(error),
    }
}

fn classify_control_plane_transport_retry_error(
    error: &anyhow::Error,
) -> Option<SourceControlPlaneRetry> {
    is_ambiguous_control_plane_transport_error(error).then_some(SourceControlPlaneRetry {
        reason: SourceControlPlaneRetryReason::TransportAmbiguous,
        retry_after: None,
    })
}

fn is_ambiguous_control_plane_transport_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|err| err.status().is_none() && !err.is_builder() && !err.is_decode())
    })
}

fn retry_delay_with_hint(
    retry_after: Option<Duration>,
    fallback: Duration,
    fallback_max: Duration,
    remaining: Duration,
) -> Duration {
    retry_after
        .unwrap_or_else(|| fallback.min(fallback_max))
        .min(remaining)
}

fn signed_content_length(value: i64, label: &str) -> anyhow::Result<u64> {
    u64::try_from(value).with_context(|| format!("server returned negative {label}: {value}"))
}

fn signed_part_number(value: NonZeroU64, label: &str) -> anyhow::Result<u32> {
    u32::try_from(value.get())
        .with_context(|| format!("server returned {label} outside u32 range: {}", value.get()))
}

fn presigned_put_headers(
    headers: Option<&CliPrepareUploadResponsePresignedPutHeaders>,
) -> PresignedPutHeaders {
    match headers {
        Some(headers) => PresignedPutHeaders {
            content_type: Some(headers.content_type.clone()),
            if_none_match: headers.if_none_match.clone(),
        },
        None => PresignedPutHeaders::empty(),
    }
}

fn presigned_head_verify(
    verify_head: Option<&CliPrepareUploadResponsePresignedPutVerifyHead>,
) -> anyhow::Result<Option<PresignedHeadVerify>> {
    verify_head
        .map(|verify_head| {
            Ok(PresignedHeadVerify {
                url: verify_head.url.clone(),
                content_length: signed_content_length(
                    verify_head.content_length,
                    "verifyHead.contentLength",
                )?,
                sha256: verify_head.sha256.as_str().to_string(),
            })
        })
        .transpose()
}

fn presigned_multipart_chunks(
    chunks: &[CliPrepareUploadResponseMultipartChunk],
) -> anyhow::Result<Vec<PresignedSourceMultipartChunk>> {
    chunks
        .iter()
        .map(|chunk| {
            Ok(PresignedSourceMultipartChunk {
                part_number: signed_part_number(chunk.part_number, "multipart partNumber")?,
                url: chunk.url.clone(),
                content_length: signed_content_length(
                    chunk.content_length,
                    "multipart contentLength",
                )?,
                sha256: chunk.sha256.as_str().to_string(),
            })
        })
        .collect()
}

async fn upload_source_object(
    client: &ApiClient,
    prepared: &PrepareUploadResponse,
    plan: &SourceBundlePlan,
    json: bool,
) -> anyhow::Result<Option<SourceMultipartCompletion>> {
    if prepared.kind != "source-upload" {
        bail!(
            "server returned unexpected prepare-upload kind: {}",
            prepared.kind
        );
    }
    if prepared.fast_path {
        if prepared.presigned_put.is_some() || prepared.multipart.is_some() {
            bail!("server returned upload targets for SOURCE_BUNDLE_V1 fast path");
        }
        return Ok(None);
    }

    let spinner = make_spinner(
        json,
        &format!(
            "Uploading SOURCE_BUNDLE_V1 source ({})...",
            format_u64_bytes(plan.source_size_bytes)
        ),
    );

    match (&prepared.presigned_put, &prepared.multipart) {
        (Some(single), None) => {
            if prepared.required_complete != RequiredComplete::UploadComplete {
                bail!("server requested multipart complete without a multipart upload target");
            }
            let bytes = plan.read_all().await?;
            let headers = presigned_put_headers(single.headers.as_ref());
            let verify_head = presigned_head_verify(single.verify_head.as_ref())?;
            upload_single_put(
                client,
                SinglePutUpload {
                    url: &single.url,
                    bytes,
                    content_length: signed_content_length(
                        single.content_length,
                        "presignedPut.contentLength",
                    )?,
                    sha256: single.sha256.as_str(),
                    headers: &headers,
                    verify_head: verify_head.as_ref(),
                    label: "SOURCE_BUNDLE_V1 source object".to_string(),
                },
            )
            .await?;
            finish_spinner(spinner, "Uploaded SOURCE_BUNDLE_V1 source");
            Ok(None)
        }
        (None, Some(multipart)) => {
            if prepared.required_complete != RequiredComplete::MultipartCompleteUploadComplete {
                bail!("server returned multipart target but requiredComplete is upload-complete");
            }
            let chunks = presigned_multipart_chunks(&multipart.chunks)?;
            let parts = upload_multipart_chunks(
                client,
                &chunks,
                multipart.chunk_size.get(),
                "SOURCE_BUNDLE_V1 source object",
                |offset, size| plan.read_chunk(offset, size),
            )
            .await?;
            finish_spinner(spinner, "Uploaded SOURCE_BUNDLE_V1 source");
            Ok(Some(SourceMultipartCompletion {
                upload_id: multipart.upload_id.as_str().to_string(),
                parts,
            }))
        }
        (None, None) => bail!("server did not return a SOURCE_BUNDLE_V1 upload target"),
        (Some(_), Some(_)) => bail!("server returned both single and multipart upload targets"),
    }
}

async fn complete_source_multipart_if_needed(
    client: &ApiClient,
    deployment_id: &str,
    deployment_attempt_id: &str,
    prepared: &PrepareUploadResponse,
    completion: Option<SourceMultipartCompletion>,
    json: bool,
) -> anyhow::Result<()> {
    if prepared.required_complete == RequiredComplete::UploadComplete {
        if completion.is_some() {
            bail!("server did not require multipart-complete but multipart upload was performed");
        }
        return Ok(());
    }
    let completion = completion.context(
        "server required multipart-complete but SOURCE_BUNDLE_V1 multipart upload was not performed",
    )?;

    let body = MultipartCompleteBody {
        deployment_id: deployment_id.to_string(),
        upload_session_id: prepared.upload_session_id.to_string(),
        deployment_attempt_id: deployment_attempt_id.to_string(),
        operation_id: Uuid::now_v7().to_string(),
        artifact_format: "SOURCE_BUNDLE_V1".to_string(),
        source_artifact_id: prepared.source_artifact_id.as_str().to_string(),
        upload_id: completion.upload_id,
        parts: completion.parts,
    };
    complete_source_multipart_with_retry(client, deployment_id, &body, json).await
}

async fn complete_source_multipart_with_retry(
    client: &ApiClient,
    deployment_id: &str,
    body: &MultipartCompleteBody,
    json: bool,
) -> anyhow::Result<()> {
    let started = Instant::now();
    let mut delay = SOURCE_COMPLETION_INITIAL_RETRY_DELAY;
    let mut attempts = 0u32;

    loop {
        attempts += 1;
        match post_source_multipart_complete_once(client, deployment_id, body).await? {
            SourceCompletionAttempt::Terminal => return Ok(()),
            SourceCompletionAttempt::Retry(retry) => {
                let elapsed = started.elapsed();
                if elapsed >= SOURCE_COMPLETION_RETRY_BUDGET {
                    bail!(
                        "multipart-complete did not reach a terminal state after {:?} (last state: {})",
                        SOURCE_COMPLETION_RETRY_BUDGET,
                        retry.reason.as_str()
                    );
                }

                if attempts == 1 {
                    output::status(
                        json,
                        "~",
                        format!(
                            "Waiting for multipart-complete ({})...",
                            retry.reason.as_str()
                        ),
                        output::Phase::Deploy,
                    );
                }

                let remaining = SOURCE_COMPLETION_RETRY_BUDGET.saturating_sub(elapsed);
                let sleep_for = retry_delay_with_hint(
                    retry.retry_after,
                    delay,
                    SOURCE_COMPLETION_MAX_RETRY_DELAY,
                    remaining,
                );
                tokio::time::sleep(sleep_for).await;
                delay = delay
                    .saturating_mul(2)
                    .min(SOURCE_COMPLETION_MAX_RETRY_DELAY);
            }
        }
    }
}

async fn post_source_multipart_complete_once(
    client: &ApiClient,
    deployment_id: &str,
    body: &MultipartCompleteBody,
) -> anyhow::Result<SourceCompletionAttempt> {
    match client
        .post::<_, MultipartCompleteResponse>(
            &format!("/v1/deployments/{deployment_id}/multipart-complete"),
            body,
        )
        .await
    {
        Ok(_) => Ok(SourceCompletionAttempt::Terminal),
        Err(error) => match classify_multipart_complete_retry_error(&error) {
            Some(retry) => Ok(SourceCompletionAttempt::Retry(retry)),
            None => Err(error.context("failed to complete source multipart upload")),
        },
    }
}

async fn report_source_object_upload_failed(
    client: &ApiClient,
    deployment_id: &str,
    deployment_attempt_id: &str,
    prepared: &PrepareUploadResponse,
    error: &anyhow::Error,
    json: bool,
) {
    let body = UploadFailedBody {
        deployment_id: deployment_id.to_string(),
        upload_session_id: prepared.upload_session_id.to_string(),
        deployment_attempt_id: deployment_attempt_id.to_string(),
        operation_id: Uuid::now_v7().to_string(),
        artifact_format: "SOURCE_BUNDLE_V1".to_string(),
        source_artifact_id: prepared.source_artifact_id.as_str().to_string(),
        error_code: SOURCE_UPLOAD_PUT_FAILED.to_string(),
        error_log: upload_failure_log(error),
    };

    if let Err(report_error) =
        report_source_object_upload_failed_with_retry(client, deployment_id, &body, json).await
    {
        output::warn(
            json,
            format!("Failed to mark SOURCE_BUNDLE_V1 upload as failed: {report_error}"),
            output::Phase::Deploy,
        );
    }
}

async fn report_source_object_upload_failed_with_retry(
    client: &ApiClient,
    deployment_id: &str,
    body: &UploadFailedBody,
    json: bool,
) -> anyhow::Result<()> {
    let started = Instant::now();
    let mut delay = UPLOAD_FAILED_INITIAL_RETRY_DELAY;
    let mut attempts = 0u32;

    loop {
        attempts += 1;
        let result = client
            .post::<_, UploadFailedResponse>(
                &format!("/v1/deployments/{deployment_id}/upload-failed"),
                body,
            )
            .await;
        match result {
            Ok(_) => return Ok(()),
            Err(error) => {
                let Some(retry) = classify_upload_failed_retry_error(&error) else {
                    return Err(error.context("failed to report source upload failure"));
                };
                let elapsed = started.elapsed();
                if elapsed >= UPLOAD_FAILED_RETRY_BUDGET {
                    return Err(error.context(format!(
                        "failed to report source upload failure after waiting {:?}",
                        UPLOAD_FAILED_RETRY_BUDGET
                    )));
                }

                if attempts == 1 {
                    output::status(
                        json,
                        "~",
                        format!("Waiting for upload-failed ({})...", retry.reason.as_str()),
                        output::Phase::Deploy,
                    );
                }

                let remaining = UPLOAD_FAILED_RETRY_BUDGET.saturating_sub(elapsed);
                let sleep_for = retry_delay_with_hint(
                    retry.retry_after,
                    delay,
                    UPLOAD_FAILED_MAX_RETRY_DELAY,
                    remaining,
                );
                tokio::time::sleep(sleep_for).await;
                delay = delay.saturating_mul(2).min(UPLOAD_FAILED_MAX_RETRY_DELAY);
            }
        }
    }
}

fn upload_failure_log(error: &anyhow::Error) -> String {
    let message = format!("{error:#}");
    let message = redact_url_credentials(&message);
    truncate_upload_failure_log(&message)
}

fn truncate_upload_failure_log(message: &str) -> String {
    if message.len() <= MAX_UPLOAD_FAILURE_LOG_LENGTH {
        return message.to_string();
    }
    let mut end = MAX_UPLOAD_FAILURE_LOG_LENGTH;
    while !message.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    message[..end].to_string()
}

fn redact_url_credentials(message: &str) -> String {
    let mut output = String::with_capacity(message.len());
    let mut cursor = 0;

    while let Some(start) = find_url_start(&message[cursor..]) {
        let start = cursor + start;
        output.push_str(&message[cursor..start]);

        let candidate_len = url_candidate_len(&message[start..]);
        let candidate = &message[start..start + candidate_len];
        output.push_str(&redact_url_candidate(candidate));
        cursor = start + candidate_len;
    }

    output.push_str(&message[cursor..]);
    output
}

fn find_url_start(message: &str) -> Option<usize> {
    match (message.find("http://"), message.find("https://")) {
        (Some(http), Some(https)) => Some(http.min(https)),
        (Some(http), None) => Some(http),
        (None, Some(https)) => Some(https),
        (None, None) => None,
    }
}

fn url_candidate_len(message: &str) -> usize {
    message
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace() || matches!(ch, '"' | '\'' | '`' | '<' | '>'))
        .map(|(idx, _)| idx)
        .unwrap_or(message.len())
}

fn redact_url_candidate(candidate: &str) -> String {
    let (url_part, suffix) = split_url_candidate_suffix(candidate);
    let Ok(mut url) = Url::parse(url_part) else {
        return candidate.to_string();
    };
    if !matches!(url.scheme(), "http" | "https") {
        return candidate.to_string();
    }

    let mut changed = false;
    if !url.username().is_empty() {
        let _ = url.set_username(REDACTED_URL_COMPONENT);
        changed = true;
    }
    if url.password().is_some() {
        let _ = url.set_password(Some(REDACTED_URL_COMPONENT));
        changed = true;
    }
    if url.query().is_some() {
        url.set_query(Some(REDACTED_URL_COMPONENT));
        changed = true;
    }
    if url.fragment().is_some() {
        url.set_fragment(Some(REDACTED_URL_COMPONENT));
        changed = true;
    }

    if changed {
        format!("{url}{suffix}")
    } else {
        candidate.to_string()
    }
}

fn split_url_candidate_suffix(candidate: &str) -> (&str, &str) {
    let mut end = candidate.len();
    while end > 0 {
        let Some(ch) = candidate[..end].chars().next_back() else {
            break;
        };
        if !matches!(ch, ')' | ']' | '}' | ',' | '.' | ';') {
            break;
        }
        end -= ch.len_utf8();
    }
    (&candidate[..end], &candidate[end..])
}

struct SinglePutUpload<'a> {
    url: &'a str,
    bytes: Bytes,
    content_length: u64,
    sha256: &'a str,
    headers: &'a PresignedPutHeaders,
    verify_head: Option<&'a PresignedHeadVerify>,
    label: String,
}

async fn upload_single_put(client: &ApiClient, upload: SinglePutUpload<'_>) -> anyhow::Result<()> {
    verify_upload_payload(
        &upload.label,
        &upload.bytes,
        upload.content_length,
        upload.sha256,
    )?;
    client
        .put_blob_with_headers_and_verify(
            upload.url,
            upload.bytes,
            upload.sha256,
            upload.headers,
            upload.verify_head,
        )
        .await
        .with_context(|| format!("failed to upload {}", upload.label))?;
    Ok(())
}

async fn upload_multipart_chunks<F, Fut>(
    client: &ApiClient,
    chunks: &[PresignedSourceMultipartChunk],
    chunk_size: u64,
    label: &str,
    mut read_chunk: F,
) -> anyhow::Result<Vec<CompletedMultipartPart>>
where
    F: FnMut(u64, u64) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<Bytes>>,
{
    let mut parts = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let offset = u64::from(chunk.part_number.saturating_sub(1))
            .checked_mul(chunk_size)
            .context("multipart chunk offset overflow")?;
        let bytes = read_chunk(offset, chunk.content_length).await?;
        verify_upload_payload(
            &format!("{label} multipart part {}", chunk.part_number),
            &bytes,
            chunk.content_length,
            &chunk.sha256,
        )?;
        let result = client
            .put_blob_capture(&chunk.url, bytes, &chunk.sha256)
            .await
            .with_context(|| {
                format!(
                    "failed to upload {label} multipart part {}",
                    chunk.part_number
                )
            })?;
        let e_tag = result.e_tag.with_context(|| {
            format!(
                "multipart upload for {label} part {} did not return an ETag",
                chunk.part_number
            )
        })?;
        parts.push(CompletedMultipartPart {
            part_number: chunk.part_number,
            e_tag,
        });
    }
    Ok(parts)
}

fn verify_upload_payload(
    label: &str,
    bytes: &[u8],
    content_length: u64,
    sha256: &str,
) -> anyhow::Result<()> {
    if bytes.len() as u64 != content_length {
        bail!(
            "{label} size drifted between prepare-upload and upload (server signed {} bytes, local materialized {} bytes)",
            content_length,
            bytes.len()
        );
    }
    let actual_sha = format!("{:x}", Sha256::digest(bytes));
    if actual_sha != sha256 {
        bail!(
            "{label} SHA drifted between prepare-upload and upload (server signed {sha256}, local materialized {actual_sha})"
        );
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentProjectLookup {
    project: DeploymentProjectInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentProjectInfo {
    id: String,
}

#[derive(Debug, Serialize)]
struct ResumeDeployOutput {
    deployment_id: String,
    status: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

async fn resolve_project_id_for_resume(
    client: &ApiClient,
    deployment_id: &str,
    explicit_project_id: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<String> {
    if let Some(project_id) = explicit_project_id.filter(|value| !value.trim().is_empty()) {
        return Ok(project_id.to_string());
    }
    if let Some(project_id) = config
        .project
        .id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(project_id.to_string());
    }

    let deployment: DeploymentProjectLookup = client
        .get(&format!("/v1/deployments/{deployment_id}"))
        .await
        .context("failed to resolve project for resumed deployment")?;
    Ok(deployment.project.id)
}

#[allow(clippy::too_many_arguments)]
async fn resume_deploy(
    client: &ApiClient,
    deployment_id: &str,
    workspace_id: &str,
    project_id: &str,
    functions: Option<crate::functions::FunctionPublishPayload>,
    _manifest: serde_json::Value,
    manifest_for_planning: build_manifest::Manifest,
    files: Vec<FileEntry>,
    output_dir: &Path,
    json: bool,
    warnings: Vec<String>,
) -> anyhow::Result<()> {
    output::status(
        json,
        "~",
        format!("Resuming deployment {deployment_id}"),
        output::Phase::Deploy,
    );

    if let Some(functions) = &functions {
        stage_deployment_functions(client, deployment_id, project_id, functions, json).await?;
    }

    output::status(
        json,
        "~",
        "Creating SOURCE_BUNDLE_V1 archive...",
        output::Phase::Deploy,
    );
    let upload_plan = build_source_bundle_plan(output_dir, &manifest_for_planning, &files)
        .context("failed to prepare SOURCE_BUNDLE_V1 upload plan")?;
    let deployment_attempt_id = Uuid::now_v7().to_string();

    prepare_upload_and_complete(
        client,
        deployment_id,
        workspace_id,
        project_id,
        &deployment_attempt_id,
        json,
        &upload_plan,
    )
    .await?;

    // Output result (no polling in resume mode — builder handles status)
    if json {
        let data = ResumeDeployOutput {
            deployment_id: deployment_id.to_string(),
            status: "upload-complete".into(),
            warnings,
        };
        if let Ok(s) = serde_json::to_string(&data) {
            output::log_line("debug", "info", "deploy", &s);
        }
    } else {
        eprintln!();
        eprintln!(
            "  {} Deployment {} upload completed",
            console::style("✓").green().bold(),
            console::style(deployment_id).bold(),
        );
        eprintln!();
    }

    Ok(())
}

async fn stage_deployment_functions(
    client: &ApiClient,
    deployment_id: &str,
    project_id: &str,
    functions: &crate::functions::FunctionPublishPayload,
    json: bool,
) -> anyhow::Result<()> {
    let edge_rule_count = functions
        .edge_rules
        .as_ref()
        .map_or(0, crate::functions::edge_rule_count);
    output::status(
        json,
        "~",
        function_stage_message(functions.functions.len(), edge_rule_count),
        output::Phase::Deploy,
    );
    let stage_result: anyhow::Result<serde_json::Value> = client
        .post(
            &stage_deployment_functions_path(project_id, deployment_id),
            functions,
        )
        .await;
    if let Err(error) = stage_result {
        return Err(map_function_stage_error(error, json));
    }
    output::success(
        json,
        function_stage_success_message(functions.functions.len(), edge_rule_count),
        output::Phase::Deploy,
    );
    Ok(())
}

fn map_function_stage_error(error: anyhow::Error, json: bool) -> anyhow::Error {
    let Some(api_error) = error.downcast_ref::<crate::api::StructuredApiError>() else {
        return error.context("failed to stage ONREZA Functions for deployment");
    };
    if api_error.code != "FUNCTION_PUBLISH_FAILED" {
        return error.context("failed to stage ONREZA Functions for deployment");
    }

    let message = format_function_publish_failure(api_error);
    if json {
        output::log_error_structured(
            "deploy",
            &message,
            &api_error.code,
            api_error.details.as_ref(),
        );
        return output::already_reported_error();
    }
    anyhow::anyhow!(message).context("failed to stage ONREZA Functions for deployment")
}

fn format_function_publish_failure(error: &crate::api::StructuredApiError) -> String {
    let Some(details) = error.details.as_ref() else {
        return error.message.clone();
    };
    let category = details
        .get("category")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("UNKNOWN");
    let message = details
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(error.message.as_str());
    let mut rendered = format!("ONREZA Functions publish failed [{category}]: {message}");
    if let Some(attempt_id) = details.get("attemptId").and_then(serde_json::Value::as_str) {
        rendered.push_str(&format!(" (attempt {attempt_id})"));
    }
    rendered
}

fn stage_deployment_functions_path(project_id: &str, deployment_id: &str) -> String {
    format!(
        "/v1/projects/{project_id}/function-activations/deployments/{deployment_id}/functions/stage"
    )
}

fn format_function_publish_summary(
    function_count: usize,
    source_file_count: usize,
    edge_rule_count: usize,
) -> String {
    if edge_rule_count == 0 {
        return format!("{function_count} function(s), {source_file_count} source file(s) ready");
    }
    format!(
        "{function_count} function(s), {source_file_count} source file(s), {edge_rule_count} edge rule(s) ready"
    )
}

fn function_stage_message(function_count: usize, edge_rule_count: usize) -> &'static str {
    match (function_count > 0, edge_rule_count > 0) {
        (true, true) => "Staging ONREZA Functions and Edge Rules...",
        (false, true) => "Staging Edge Rules...",
        _ => "Staging ONREZA Functions...",
    }
}

fn function_stage_success_message(function_count: usize, edge_rule_count: usize) -> &'static str {
    match (function_count > 0, edge_rule_count > 0) {
        (true, true) => "ONREZA Functions and Edge Rules staged for deployment",
        (false, true) => "Edge Rules staged for deployment",
        _ => "ONREZA Functions staged for deployment",
    }
}

// ── PROCESS validation ───────────────────────────────────────

fn manifest_has_compute_layer(manifest: &build_manifest::Manifest) -> bool {
    manifest
        .layers
        .iter()
        .any(|layer| layer.target == build_manifest::LayerTarget::Compute)
}

/// Resolve a guaranteed-present manifest for the given compute mode.
///
/// Returns `Value` (not `Option<Value>`) because the server requires `manifest`
/// on `POST /v1/projects/:id/deployments` (DEP-326 schema). Three cases:
///
/// - manifest already present → passes through `validate_compute_manifest_contract`
///   (guards compute/manifest combinations) and is returned as-is.
/// - STATIC without manifest → auto-gen via `generate_static_manifest()`. The
///   build step auto-gens this only when detection suggested Static; this branch
///   covers `--compute static` overrides for projects detected as Process.
/// - PROCESS without manifest → `validate_compute_manifest_contract` bails with a
///   user-facing error (PROCESS auto-gen runs earlier in `run()`, so this case
///   here means PROCESS auto-gen failed somewhere upstream).
///
/// Replaces the `manifest_raw.expect(...)` runtime invariant with a typed
/// signature, so missing-manifest bugs surface at type-check time on call-sites.
fn resolve_manifest_for_compute(
    compute: ComputeType,
    manifest_raw: Option<serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
    if let Some(manifest) = manifest_raw {
        validate_compute_manifest_contract(compute, true)?;
        return Ok(manifest);
    }

    if compute == ComputeType::Static {
        let auto = build_manifest::generate_static_manifest();
        return serde_json::to_value(&auto)
            .context("failed to serialize auto-generated STATIC manifest");
    }

    // PROCESS without manifest: defer to validate for the user-facing message.
    validate_compute_manifest_contract(compute, false)?;
    // validate_compute_manifest_contract must return Err for these; reaching here
    // means its contract was changed. Surface as a bug, not a panic.
    bail!(
        "Internal error: validate_compute_manifest_contract accepted {compute:?} without a manifest.\n\
         This is a CLI bug — please report at github.com/onreza/nrz-cli/issues."
    );
}

fn validate_compute_manifest_contract(
    compute: ComputeType,
    has_manifest: bool,
) -> anyhow::Result<()> {
    // Safety net: PROCESS auto-generation should have produced a manifest
    // before this point. Reaching here without one is an unexpected internal state.
    if compute == ComputeType::Process && !has_manifest {
        bail!(
            "Internal error: PROCESS deploy reached validation without a manifest.\n\
             This is unexpected — please report this at github.com/onreza/nrz-cli/issues.\n\n\
             If you see this consistently, work around it by creating .onreza/manifest.json\n\
             manually."
        );
    }

    // When a manifest is present, its layers define the compute targets —
    // any compute type derived from the manifest is valid.

    Ok(())
}

/// Framework-specific hint about switching to static export.
fn framework_static_hint(framework: &str) -> &'static str {
    match framework {
        "nextjs" | "blitzjs" | "payload" => "add `output: 'export'` to next.config",
        "nuxt" => "set `ssr: false` in nuxt.config",
        "sveltekit" => "use `adapter-static` in svelte.config.js",
        "astro" => "remove `output: 'server'` from astro.config",
        "react-router" | "hydrogen" => "set `ssr: false` in react-router.config.ts",
        "tanstack-start" => "set `ssr: false` in app.config.ts",
        "remix" => "set `ssr: false` in the Remix Vite plugin options",
        "solidstart" => "set `ssr: false` in app.config.ts",
        "qwik" => "use the static adaptor in vite.config",
        "analog" => "set `ssr: false` in the Analog plugin options",
        _ => "",
    }
}

/// Pre-flight validation for PROCESS deployments.
///
/// Checks that the output directory is compatible with PROCESS before
/// expensive operations (entry resolution, bundling, upload).
fn validate_process_output(
    output_dir: &Path,
    project_dir: &Path,
    detection: &crate::detect::types::DetectionResult,
) -> anyhow::Result<()> {
    if let Some(msg) = detect_workers_runtime_target(project_dir, output_dir)? {
        return Err(output::coded_error("FRAMEWORK_UNSUPPORTED", msg));
    }

    match detection.framework.as_str() {
        "nextjs" | "blitzjs" | "payload" => {
            let has_standalone = detection
                .metadata
                .ssr_analysis
                .as_ref()
                .is_some_and(|ssr| ssr.has_standalone_output());
            let standalone_server = output_dir.join("server.js");

            if !standalone_server.is_file() {
                if has_standalone {
                    bail!(
                        "Next.js `output: 'standalone'` is configured, but PROCESS output is invalid.\n\n\
                         Missing file: {}/server.js\n\n\
                         Make sure build output points to `.next/standalone` and `next build` completed successfully.",
                        output_dir.display()
                    );
                }

                let dir_name = output_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if dir_name == "standalone" {
                    bail!(
                        "Next.js standalone output directory found, but server.js is missing.\n\n\
                         Expected: {}/server.js\n\n\
                         Make sure `next build` completed successfully and the standalone \
                         output contains server.js.",
                        output_dir.display()
                    );
                }

                bail!(
                    "Next.js PROCESS deployment requires `output: 'standalone'` \
                     in next.config.\n\n\
                     Current output is not a runnable standalone server directory \
                     (missing `{}/server.js`).\n\n\
                     Add to your next.config.{{js,mjs,ts}}:\n\
                     \x20 module.exports = {{ output: 'standalone' }}\n\n\
                     Then rebuild and redeploy. For static export, use --compute static \
                     with `output: 'export'`.",
                    output_dir.display()
                );
            }
        }
        "nuxt" => {
            let server_entry = output_dir.join("server/index.mjs");
            if !server_entry.is_file() {
                bail!(
                    "Nuxt PROCESS deployment expects server/index.mjs in {}.\n\n\
                     This file is created by `npx nuxi build`. If you used \
                     `nuxi generate`, the output is static-only and should be \
                     deployed with --compute static.",
                    output_dir.display()
                );
            }
        }
        _ => {}
    }
    Ok(())
}

/// Pre-build validation for PROCESS deployments.
///
/// This only checks package-level signals because it runs before the build
/// artifact exists. Output marker checks stay in `validate_process_output`.
fn validate_prebuild_process_project(project_dir: &Path) -> anyhow::Result<()> {
    if let Some(msg) = detect_workers_runtime_package_target(project_dir)? {
        return Err(output::coded_error("FRAMEWORK_UNSUPPORTED", msg));
    }

    Ok(())
}

fn validate_prebuild_compute_intent(
    project_dir: &Path,
    explicit_compute: Option<ComputeType>,
) -> anyhow::Result<()> {
    if explicit_compute == Some(ComputeType::Process) {
        validate_prebuild_process_project(project_dir)?;
    }

    Ok(())
}

/// Detect projects targeting a Workers-style runtime (Cloudflare workerd,
/// Shopify Oxygen) whose build output cannot execute on Node/Bun.
///
/// These outputs export an ESM module with a `fetch` handler instead of
/// listening on a port, so PROCESS compute would silently 404 every route.
/// We fail fast with framework-specific guidance on how to switch.
///
/// Signals, in order of preference (most specific first):
/// - `@cloudflare/vite-plugin` in package.json → Cloudflare Workers
/// - `@shopify/mini-oxygen` in package.json → Shopify Oxygen
/// - `server/wrangler.json` in build output → Cloudflare Workers (fallback)
/// - `server/oxygen.json` in build output → Shopify Oxygen (fallback)
fn detect_workers_runtime_target(
    project_dir: &Path,
    output_dir: &Path,
) -> anyhow::Result<Option<String>> {
    if let Some(msg) = detect_workers_runtime_package_target(project_dir)? {
        return Ok(Some(msg));
    }

    detect_workers_runtime_output_target(output_dir)
}

fn detect_workers_runtime_package_target(project_dir: &Path) -> anyhow::Result<Option<String>> {
    // Strict load: an unreadable or malformed package.json is propagated as an
    // error instead of silently yielding "no signal". Otherwise a corrupted
    // manifest would let a Workers bundle ship as PROCESS — the exact failure
    // mode this detector exists to prevent.
    let pkg = crate::detect::package_json::PackageJson::load_strict(project_dir)?;
    let has_cf_plugin = pkg
        .as_ref()
        .is_some_and(|p| p.has_dependency("@cloudflare/vite-plugin"));
    let has_mini_oxygen = pkg
        .as_ref()
        .is_some_and(|p| p.has_dependency("@shopify/mini-oxygen"));

    if !has_cf_plugin && !has_mini_oxygen {
        return Ok(None);
    }

    let (runtime, trigger, remedy) = if has_cf_plugin {
        (
            "Cloudflare Workers",
            "@cloudflare/vite-plugin is in your package.json",
            "Replace @cloudflare/vite-plugin with Nitro in vite.config.ts:\n\
             \x20      import { nitro } from 'nitro/vite'\n\
             \x20      // plugins: [tanstackStart(), nitro(), viteReact()]\n\
             \x20    Nitro's default `node-server` preset emits .output/server/index.mjs, \
             which PROCESS can run directly.",
        )
    } else if has_mini_oxygen {
        (
            "Shopify Oxygen (Cloudflare Workers)",
            "@shopify/mini-oxygen is in your package.json",
            "Apply the Hydrogen Express recipe to switch to a Node runtime:\n\
             \x20    https://github.com/Shopify/hydrogen/tree/main/cookbook/recipes/express\n\
             \x20    It replaces the Oxygen server with Express and emits build/server/index.js \
             plus a server.mjs entry at the project root.",
        )
    } else {
        unreachable!("package-level Workers runtime detector has no matching signal");
    };

    Ok(Some(workers_runtime_message(runtime, trigger, remedy)))
}

fn detect_workers_runtime_output_target(output_dir: &Path) -> anyhow::Result<Option<String>> {
    let has_wrangler_output = output_dir.join("server/wrangler.json").is_file();
    let has_oxygen_output = output_dir.join("server/oxygen.json").is_file();

    if !has_wrangler_output && !has_oxygen_output {
        return Ok(None);
    }

    let (runtime, trigger, remedy) = if has_wrangler_output {
        (
            "Cloudflare Workers",
            "server/wrangler.json was emitted into the build output",
            "Remove the Cloudflare Vite plugin from vite.config.ts and rebuild with \
             a Node-compatible preset (e.g. Nitro's node-server).",
        )
    } else {
        (
            "Shopify Oxygen (Cloudflare Workers)",
            "server/oxygen.json was emitted into the build output",
            "Apply the Hydrogen Express recipe to rebuild for Node:\n\
             \x20    https://github.com/Shopify/hydrogen/tree/main/cookbook/recipes/express",
        )
    };

    Ok(Some(workers_runtime_message(runtime, trigger, remedy)))
}

fn workers_runtime_message(runtime: &str, trigger: &str, remedy: &str) -> String {
    format!(
        "{runtime} target detected ({trigger}).\n\n\
         ONREZA PROCESS compute runs Node/Bun servers, not the Workers runtime (workerd), \
         so this build cannot be deployed as-is.\n\n\
         Pick one:\n\
         \x20 1. Deploy as static (if your app has no server functions):\n\
         \x20    nrz deploy --compute static\n\n\
         \x20 2. Switch to a Node server build.\n\
         \x20    {remedy}"
    )
}

/// Framework-specific diagnostic when entry point resolution fails.
fn framework_process_diagnostic(
    framework: &str,
    detection: &crate::detect::types::DetectionResult,
    output_dir: &Path,
) -> Option<String> {
    match framework {
        "nextjs" | "blitzjs" | "payload" => {
            let has_standalone = detection
                .metadata
                .ssr_analysis
                .as_ref()
                .is_some_and(|ssr| ssr.has_standalone_output());

            if has_standalone {
                Some(format!(
                    "Next.js `output: 'standalone'` is configured, but server.js not found \
                     in {}.\n\n\
                     Make sure `next build` completed successfully.\n\
                     Expected: .next/standalone/server.js",
                    output_dir.display()
                ))
            } else {
                Some(
                    "Next.js PROCESS deployment requires `output: 'standalone'` \
                     in next.config.\n\n\
                     Add to your next.config.{js,mjs,ts}:\n\
                     \x20 module.exports = { output: 'standalone' }\n\n\
                     Then rebuild and redeploy. This creates a self-contained \
                     server at .next/standalone/server.js."
                        .to_string(),
                )
            }
        }
        "nuxt" => Some(
            "Nuxt PROCESS deployment expects server/index.mjs in the .output/ directory.\n\n\
             Make sure you ran `npx nuxi build` (not `nuxi generate`).\n\
             The build should create .output/server/index.mjs."
                .to_string(),
        ),
        "sveltekit" => Some(
            "SvelteKit PROCESS deployment requires adapter-node.\n\n\
             Install it:\n\
             \x20 npm install -D @sveltejs/adapter-node\n\n\
             Update svelte.config.js:\n\
             \x20 import adapter from '@sveltejs/adapter-node';\n\n\
             Rebuild and redeploy."
                .to_string(),
        ),
        "react-router" => Some(format!(
            "React Router PROCESS deployment expects server/index.js in {}.\n\n\
             Make sure you ran `npx react-router build` and the build \
             output contains build/server/index.js.",
            output_dir.display()
        )),
        "remix" => Some(format!(
            "Remix PROCESS deployment expects server/index.js in {}.\n\n\
             Make sure you ran the build command and the output \
             contains build/server/index.js.",
            output_dir.display()
        )),
        "hono" => Some(
            "Hono PROCESS deployment requires a built entry point.\n\n\
             Make sure your build script produces a runnable file in dist/ \
             (e.g. dist/index.js)."
                .to_string(),
        ),
        "elysia" => Some(
            "Elysia PROCESS deployment requires a built entry point.\n\n\
             Make sure your build script produces a runnable file in dist/ \
             (e.g. dist/index.js). Elysia runs on Bun."
                .to_string(),
        ),
        "nestjs" => Some(
            "NestJS PROCESS deployment expects main.js in the dist/ directory.\n\n\
             Make sure you ran `npm run build` (nest build).\n\
             The build should create dist/main.js."
                .to_string(),
        ),
        "fastify" => Some(
            "Fastify PROCESS deployment requires a runnable entry point.\n\n\
             Set \"main\" in package.json to your server file, \
             or add a \"start\" script."
                .to_string(),
        ),
        "adonis" => Some(
            "AdonisJS PROCESS deployment expects bin/server.js in the build/ directory.\n\n\
             Make sure you ran `node ace build`.\n\
             The build should create build/bin/server.js."
                .to_string(),
        ),
        "express" => Some(
            "Express PROCESS deployment requires a runnable entry point.\n\n\
             Set \"main\" in package.json to your server file \
             (e.g. \"main\": \"server.js\"), or add a \"start\" script."
                .to_string(),
        ),
        "koa" => Some(
            "Koa PROCESS deployment requires a runnable entry point.\n\n\
             Set \"main\" in package.json to your server file \
             (e.g. \"main\": \"server.js\"), or add a \"start\" script."
                .to_string(),
        ),
        "h3" => Some(
            "H3 PROCESS deployment requires a runnable entry point.\n\n\
             Make sure your build produces a file in dist/ \
             (e.g. dist/index.mjs), or set \"main\" in package.json."
                .to_string(),
        ),
        "nitro" => Some(
            "Nitro PROCESS deployment expects server/index.mjs in the .output/ directory.\n\n\
             Make sure you ran the build command.\n\
             The build should create .output/server/index.mjs."
                .to_string(),
        ),
        "tanstack-start" => Some(format!(
            "TanStack Start PROCESS deployment expects server/index.mjs in {}.\n\n\
             Make sure you ran `npm run build` and the output \
             contains .output/server/index.mjs (Nitro default preset).\n\n\
             If you see `dist/server/` with a worker-entry-*.js instead, your project is \
             configured for Cloudflare Workers via @cloudflare/vite-plugin, which is not \
             supported by PROCESS. Either remove that plugin and use Nitro's default \
             node-server preset, or deploy with --compute static.",
            output_dir.display()
        )),
        "hydrogen" => Some(format!(
            "Hydrogen PROCESS deployment could not resolve a runnable entry in {}.\n\n\
             Hydrogen has two build layouts:\n\
             \x20 - Oxygen (default): emits dist/server/index.js as a Workers bundle — \
             not executable by PROCESS. Apply the Hydrogen Express recipe \
             (https://github.com/Shopify/hydrogen/tree/main/cookbook/recipes/express) \
             to switch to Node.\n\
             \x20 - Express recipe: emits build/server/index.js plus a server.mjs at the \
             project root; the `start` script runs `node server.mjs`.\n\n\
             If you used the Express recipe, make sure `npm run build` completed and the \
             `start` script or package.json `main` points to server.mjs.",
            output_dir.display()
        )),
        _ => None,
    }
}

/// Frameworks where PROCESS output must be explicit and validated.
///
/// For these, a failed entry-point resolution triggers `framework_process_diagnostic`
/// and `bail!` with actionable guidance. Non-strict frameworks silently fall back
/// to `bun <output_dir>`, which for SSR frameworks is almost always a 404-machine —
/// so every SSR framework we ship support for is listed here explicitly.
fn is_strict_process_framework(framework: &str) -> bool {
    matches!(
        framework,
        "nextjs"
            | "nuxt"
            | "sveltekit"
            | "astro"
            | "remix"
            | "react-router"
            | "solidstart"
            | "qwik"
            | "analog"
            | "blitzjs"
            | "payload"
            | "tanstack-start"
            | "hydrogen"
    )
}

// ── PROCESS entry point ──────────────────────────────────────

fn is_windows_drive_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
}

fn sanitize_config_entry(entry: &str) -> anyhow::Result<String> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        bail!("[deploy] entry in onreza.toml must not be empty");
    }

    let normalized = trimmed.replace('\\', "/");
    let lowered = normalized.to_ascii_lowercase();
    let path = Path::new(&normalized);
    if path.is_absolute() || lowered.starts_with("file:") || is_windows_drive_absolute(&normalized)
    {
        bail!(
            "[deploy] entry must be a relative path within the output directory, got: \"{entry}\""
        );
    }

    let mut cleaned = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(seg) => cleaned.push(seg),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!(
                    "[deploy] entry must be a relative path within the output directory, got: \"{entry}\""
                );
            }
        }
    }

    if cleaned.as_os_str().is_empty() {
        bail!("[deploy] entry in onreza.toml must not be empty");
    }

    Ok(cleaned.to_string_lossy().replace('\\', "/"))
}

/// Resolve and ensure entry point for PROCESS deployments.
///
/// 1. Resolve entry: config `[deploy] entry` > framework auto-detect
/// 2. Validate file existence when entry is resolved
/// 3. If unresolved for non-strict frameworks, fallback to runtime default (`bun <output_dir>`)
fn ensure_process_entry(
    output_dir: &Path,
    project_dir: &Path,
    config_entry: Option<&str>,
    detection: &crate::detect::types::DetectionResult,
    json: bool,
) -> anyhow::Result<(Option<String>, Option<String>)> {
    // 1. Resolve entry point
    let entry = if let Some(e) = config_entry {
        Some(
            sanitize_config_entry(e)
                .map_err(|err| output::with_default_code(err, "INVALID_DEPLOY_ENTRY"))?,
        )
    } else {
        match crate::detect::resolve_entry_point_detailed(
            &detection.framework,
            output_dir,
            project_dir,
        ) {
            crate::detect::EntryPointResolution::Found(resolved) => {
                output::status(
                    json,
                    "~",
                    format!(
                        "Entry point resolved from {:?}: {}",
                        resolved.source, resolved.path
                    ),
                    output::Phase::Deploy,
                );
                Some(resolved.path)
            }
            crate::detect::EntryPointResolution::Ambiguous(candidates) => {
                if is_strict_process_framework(&detection.framework) {
                    return Err(output::coded_error(
                        "ENTRY_POINT_AMBIGUOUS",
                        format!(
                            "Cannot determine entry point for PROCESS deployment: multiple candidates found.\n\n\
                             Candidates in {}:\n\
                             {}\n\n\
                             Set [deploy] entry in onreza.toml to pick one explicitly.",
                            output_dir.display(),
                            candidates
                                .iter()
                                .map(|c| format!("  - {c}"))
                                .collect::<Vec<_>>()
                                .join("\n")
                        ),
                    ));
                }

                let warning = format!(
                    "Entry point auto-detection is ambiguous for PROCESS deployment ({} candidates in {}).\n\
                     Falling back to runtime default (`bun` in output directory).\n\
                     Set [deploy] entry in onreza.toml to make startup explicit.",
                    candidates.len(),
                    output_dir.display()
                );
                return Ok((None, Some(warning)));
            }
            crate::detect::EntryPointResolution::NotFound => {
                if !is_strict_process_framework(&detection.framework) {
                    let warning = format!(
                        "Entry point auto-detection did not find a runnable file in {}.\n\
                         Falling back to runtime default (`bun` in output directory).\n\
                         Set [deploy] entry in onreza.toml to avoid runtime guesswork.",
                        output_dir.display()
                    );
                    return Ok((None, Some(warning)));
                }

                if let Some(diagnostic) =
                    framework_process_diagnostic(&detection.framework, detection, output_dir)
                {
                    bail!("{diagnostic}");
                }
                bail!(
                    "Cannot determine entry point for PROCESS deployment.\n\n\
                     No entry point found in output directory: {}\n\n\
                     Options:\n\
                     \x20 1. Set [deploy] entry = \"server.ts\" in onreza.toml\n\
                     \x20 2. Add \"main\" or \"module\" field to package.json\n\
                     \x20 3. Add a start/serve script with an explicit file path",
                    output_dir.display()
                );
            }
            crate::detect::EntryPointResolution::Error(err) => {
                bail!(
                    "Cannot determine entry point for PROCESS deployment.\n\n\
                     {err}\n\n\
                     Set [deploy] entry in onreza.toml to override auto-detection."
                );
            }
        }
    };

    // 2. Validate file exists and is within output_dir
    if let Some(ref entry) = entry {
        let entry_path = output_dir.join(entry);
        if !entry_path.is_file() {
            return Err(output::coded_error(
                "INVALID_DEPLOY_ENTRY",
                missing_entrypoint_message(entry, output_dir),
            ));
        }
        let canonical_entry = entry_path
            .canonicalize()
            .with_context(|| format!("failed to resolve entry point path: {entry}"))?;
        let canonical_output = output_dir
            .canonicalize()
            .context("failed to resolve output directory path")?;
        if !canonical_entry.starts_with(&canonical_output) {
            return Err(output::coded_error(
                "INVALID_DEPLOY_ENTRY",
                format!("entry point must be inside the output directory, got: \"{entry}\""),
            ));
        }

        output::status(
            json,
            "~",
            format!("Entry point: {entry}"),
            output::Phase::Deploy,
        );
    } else {
        output::status(
            json,
            "~",
            format!(
                "Entry point: <runtime default bun {}>",
                output_dir.display()
            ),
            output::Phase::Deploy,
        );
    }

    Ok((entry, None))
}

const MISPLACED_ENTRYPOINT_MAX_DEPTH: usize = 4;
const MISPLACED_ENTRYPOINT_MAX_MATCHES: usize = 5;

fn missing_entrypoint_message(entry: &str, output_dir: &Path) -> String {
    let mut message = format!(
        "Entry point \"{entry}\" not found in output directory: {}\n\n\
         Make sure the file exists after running your build command.",
        output_dir.display()
    );

    let candidates = find_nested_entrypoint_candidates(output_dir, entry);
    if candidates.is_empty() {
        return message;
    }

    message.push_str("\n\nFound matching entry point file outside the selected output root:");
    for candidate in &candidates {
        message.push_str(&format!("\n  - {candidate}"));
    }

    if let Some(output_hint) = output_directory_hint_for_nested_entry(&candidates[0], entry) {
        message.push_str(&format!(
            "\n\nThis usually means outputDirectory points at {} while the build emits a nested deploy artifact.\n\
             Set [build] output_directory = \"{output_hint}\" and keep [deploy] entry = \"{entry}\".",
            output_dir.display()
        ));
    } else {
        message.push_str(
            "\n\nThis usually means outputDirectory and [deploy] entry describe different roots. \
             Point outputDirectory at the directory that contains the entry point.",
        );
    }

    message
}

fn output_directory_hint_for_nested_entry(candidate: &str, entry: &str) -> Option<String> {
    let candidate = Path::new(candidate);
    let entry_depth = Path::new(entry).components().count();
    let mut output_dir = candidate;
    for _ in 0..entry_depth {
        output_dir = output_dir.parent()?;
    }

    if output_dir.as_os_str().is_empty() {
        return None;
    }

    Some(output_dir.to_string_lossy().replace('\\', "/"))
}

fn find_nested_entrypoint_candidates(output_dir: &Path, entry: &str) -> Vec<String> {
    let entry_path = Path::new(entry);
    let direct_entry = output_dir.join(entry_path);
    let mut matches = Vec::new();
    collect_nested_entrypoint_candidates(
        output_dir,
        output_dir,
        entry_path,
        &direct_entry,
        0,
        &mut matches,
    );
    matches.sort();
    matches.truncate(MISPLACED_ENTRYPOINT_MAX_MATCHES);
    matches
}

fn collect_nested_entrypoint_candidates(
    base: &Path,
    current: &Path,
    entry: &Path,
    direct_entry: &Path,
    depth: usize,
    matches: &mut Vec<String>,
) {
    if matches.len() >= MISPLACED_ENTRYPOINT_MAX_MATCHES || depth >= MISPLACED_ENTRYPOINT_MAX_DEPTH
    {
        return;
    }

    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };

    for entry_result in entries {
        if matches.len() >= MISPLACED_ENTRYPOINT_MAX_MATCHES {
            return;
        }

        let Ok(dir_entry) = entry_result else {
            continue;
        };
        let path = dir_entry.path();
        let Ok(file_type) = dir_entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            if should_skip_misplaced_entrypoint_dir(&path) {
                continue;
            }
            collect_nested_entrypoint_candidates(
                base,
                &path,
                entry,
                direct_entry,
                depth + 1,
                matches,
            );
            continue;
        }

        if file_type.is_file()
            && path != direct_entry
            && path.ends_with(entry)
            && let Ok(rel) = path.strip_prefix(base)
        {
            matches.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn should_skip_misplaced_entrypoint_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("node_modules" | ".git" | ".cache" | ".next" | "target")
    )
}

// ── Build step ───────────────────────────────────────────────

/// Resolve build command. Priority: CLI flag > config > server > auto-detect.
fn resolve_build_command(
    explicit: Option<&str>,
    project_dir: &Path,
    effective: &EffectiveProjectConfig,
) -> Option<String> {
    if let Some(cmd) = explicit {
        return Some(cmd.to_string());
    }
    if let Some(setting) = effective.build_command() {
        return setting.value().map(str::to_string);
    }
    // Only auto-detect if package.json has a "build" script
    let pkg = crate::detect::package_json::PackageJson::load(project_dir)?;
    if !pkg.scripts.contains_key("build") {
        return None;
    }
    let pm = crate::detect::detect_package_manager_name(project_dir);
    Some(format!("{pm} run build"))
}

/// Lightweight pre-build check: does the project use Next.js (directly or via wrapper like Payload v3)?
fn is_nextjs_project(project_dir: &Path) -> bool {
    let Some(pkg) = crate::detect::package_json::PackageJson::load(project_dir) else {
        return false;
    };
    pkg.has_dependency("next")
}

/// Check if the project uses SvelteKit with adapter-auto (needs GCP_BUILDPACKS env injection).
fn is_sveltekit_with_adapter_auto(project_dir: &Path) -> bool {
    let Some(pkg) = crate::detect::package_json::PackageJson::load(project_dir) else {
        return false;
    };
    if !pkg.has_dependency("@sveltejs/kit") {
        return false;
    }
    if pkg.has_dependency("@sveltejs/adapter-node")
        || pkg.has_dependency("@sveltejs/adapter-static")
        || pkg.has_dependency("@sveltejs/adapter-vercel")
        || pkg.has_dependency("@sveltejs/adapter-cloudflare")
        || pkg.has_dependency("@sveltejs/adapter-netlify")
    {
        return false;
    }
    let config_content = ["svelte.config.js", "svelte.config.ts"]
        .iter()
        .map(|n| project_dir.join(n))
        .find(|p| p.is_file())
        .and_then(|p| std::fs::read_to_string(p).ok());
    match config_content {
        Some(content) => content.contains("adapter-auto"),
        None => true,
    }
}

/// Run a shell command, streaming stdout/stderr through structured JSON in JSON mode.
///
/// In JSON mode: pipes stdout/stderr, wraps each line via `output::log_line()`.
/// In non-JSON mode: inherits stdio (unchanged behavior).
///
/// `child_stream` controls the `s` field for child stdout lines ("user" or "debug").
/// Child stderr always goes to "debug" stream with "warn" level.
fn run_command_streaming(
    cmd: &str,
    project_dir: &Path,
    json: bool,
    phase: output::Phase,
    child_stream: &str,
    extra_env: &[(String, String)],
) -> anyhow::Result<()> {
    use std::io::BufRead;

    #[cfg(unix)]
    let (shell, shell_args) = ("sh", ["-c", cmd]);
    #[cfg(windows)]
    let (shell, shell_args) = ("cmd", ["/C", cmd]);

    if !json {
        let status = std::process::Command::new(shell)
            .args(shell_args)
            .current_dir(project_dir)
            .envs(
                extra_env
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str())),
            )
            .status()
            .with_context(|| format!("failed to start command: {cmd}"))?;
        if !status.success() {
            match status.code() {
                Some(code) => {
                    return Err(output::coded_error(
                        format!("{}_EXIT_CODE", phase.as_str().to_uppercase()),
                        format!("{phase} command `{cmd}` failed with exit code {code}"),
                    ));
                }
                None => {
                    return Err(output::coded_error(
                        format!("{}_SIGNAL_KILLED", phase.as_str().to_uppercase()),
                        format!("{phase} process `{cmd}` was killed by signal"),
                    ));
                }
            }
        }
        return Ok(());
    }

    // JSON mode: capture stdout/stderr and emit structured log lines
    let mut child = std::process::Command::new(shell)
        .args(shell_args)
        .current_dir(project_dir)
        .envs(
            extra_env
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        )
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start command: {cmd}"))?;

    let stdout = child
        .stdout
        .take()
        .context("expected piped stdout on child process")?;
    let stderr = child
        .stderr
        .take()
        .context("expected piped stderr on child process")?;

    let phase_out = phase.to_string();
    let stream_out = child_stream.to_string();
    let stdout_handle = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for result in reader.lines() {
            match result {
                Ok(line) if line.is_empty() => {} // skip blank lines
                Ok(line) => output::log_line(&stream_out, "info", &phase_out, &line),
                Err(e) => {
                    output::log_line(
                        "debug",
                        "warn",
                        &phase_out,
                        &format!("[nrz] failed to read stdout: {e}"),
                    );
                    break;
                }
            }
        }
    });

    let phase_err = phase.to_string();
    // Install stderr → "user" stream (errors visible to user), other phases follow child_stream
    let stream_err = if phase == output::Phase::Install {
        "user".to_string()
    } else {
        child_stream.to_string()
    };
    let stderr_handle = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for result in reader.lines() {
            match result {
                Ok(line) if line.is_empty() => {} // skip blank lines
                Ok(line) => output::log_line(&stream_err, "warn", &phase_err, &line),
                Err(e) => {
                    output::log_line(
                        "debug",
                        "warn",
                        &phase_err,
                        &format!("[nrz] failed to read stderr: {e}"),
                    );
                    break;
                }
            }
        }
    });

    let status = child
        .wait()
        .with_context(|| format!("failed to wait for command: {cmd}"))?;
    if let Err(e) = stdout_handle.join() {
        tracing::warn!("stdout reader thread panicked: {e:?}");
    }
    if let Err(e) = stderr_handle.join() {
        tracing::warn!("stderr reader thread panicked: {e:?}");
    }

    if !status.success() {
        match status.code() {
            Some(code) => {
                return Err(output::coded_error(
                    format!("{}_EXIT_CODE", phase.as_str().to_uppercase()),
                    format!("{phase} command failed with exit code {code}"),
                ));
            }
            None => {
                return Err(output::coded_error(
                    format!("{}_SIGNAL_KILLED", phase.as_str().to_uppercase()),
                    format!("{phase} process was killed by signal"),
                ));
            }
        }
    }
    Ok(())
}

fn run_install_step(
    project_dir: &Path,
    json: bool,
    effective: &EffectiveProjectConfig,
) -> anyhow::Result<()> {
    let Some(cmd) = resolve_install_command(project_dir, effective) else {
        return Ok(());
    };
    let (cmd, install_env) = prepare_install_command(&cmd, project_dir, json);

    output::status(
        json,
        ">",
        format!("Installing dependencies: {cmd}"),
        output::Phase::Deploy,
    );
    // Install child output → debug stream (npm noise), nrz markers go through output::status/success
    run_command_streaming(
        &cmd,
        project_dir,
        json,
        output::Phase::Install,
        "debug",
        &install_env,
    )?;
    output::success(json, "Dependencies installed", output::Phase::Deploy);
    Ok(())
}

fn resolve_install_command(
    project_dir: &Path,
    effective: &EffectiveProjectConfig,
) -> Option<String> {
    // Priority: effective config command > auto-detect from package manager.
    // PRESET server commands are filtered out while building EffectiveProjectConfig.
    if let Some(setting) = effective.install_command() {
        return setting.value().map(str::to_string);
    }
    if !project_dir.join("package.json").exists() {
        return None;
    }

    let local_fs = crate::detect::fs::LocalFs::new(project_dir);
    let pkg = crate::detect::package_json::PackageJson::load_from_fs(&local_fs);
    let pm_info = crate::detect::package_manager::detect_package_manager(&local_fs, pkg.as_ref());
    match pm_info {
        Some(info) => {
            Some(crate::detect::package_manager::install_command(info.pm_type).to_string())
        }
        None => Some("npm install".to_string()),
    }
}

fn prepare_install_command(
    cmd: &str,
    project_dir: &Path,
    json: bool,
) -> (String, Vec<(String, String)>) {
    prepare_install_command_with_sandbox(cmd, project_dir, json, running_in_onreza_build_sandbox())
}

fn prepare_install_command_with_sandbox(
    cmd: &str,
    project_dir: &Path,
    json: bool,
    running_in_sandbox: bool,
) -> (String, Vec<(String, String)>) {
    if !should_apply_pnpm_build_scripts_compat(cmd, project_dir, running_in_sandbox) {
        return (cmd.to_string(), Vec::new());
    }

    output::status(
        json,
        "~",
        "pnpm install in build sandbox: allowing dependency build scripts (no project pnpm build policy found)",
        output::Phase::Deploy,
    );

    (cmd.to_string(), pnpm_build_scripts_compat_env())
}

fn pnpm_build_scripts_compat_env() -> Vec<(String, String)> {
    PNPM_BUILD_SCRIPT_COMPAT_ENV
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

fn should_apply_pnpm_build_scripts_compat(
    cmd: &str,
    project_dir: &Path,
    running_in_sandbox: bool,
) -> bool {
    running_in_sandbox
        && is_pnpm_install_command(cmd)
        && !has_explicit_pnpm_build_policy(project_dir)
}

fn running_in_onreza_build_sandbox() -> bool {
    running_in_onreza_build_sandbox_from_env(|key| std::env::var(key).ok())
}

fn running_in_onreza_build_sandbox_from_env(mut env: impl FnMut(&str) -> Option<String>) -> bool {
    env_value_is_truthy(env("NRZ_BUILD_SANDBOX").as_deref())
        || (env_value_is_truthy(env("ONREZA").as_deref())
            && env_value_is_truthy(env("CI").as_deref()))
}

fn env_value_is_truthy(value: Option<&str>) -> bool {
    matches!(
        value.map(str::trim),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

fn is_pnpm_install_command(cmd: &str) -> bool {
    let tokens = shell_command_tokens(cmd);
    let Some(pnpm_index) = tokens.iter().position(|token| is_pnpm_command_token(token)) else {
        return false;
    };

    let mut skip_next = false;
    for token in tokens.iter().skip(pnpm_index + 1) {
        if is_shell_command_separator(token) {
            break;
        }
        if skip_next {
            skip_next = false;
            continue;
        }
        if pnpm_option_takes_value(token) {
            skip_next = !token.contains('=');
            continue;
        }
        if token.starts_with('-') {
            continue;
        }
        return matches!(token.as_str(), "install" | "i");
    }
    false
}

fn shell_command_tokens(cmd: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = cmd.chars().peekable();

    while let Some(ch) = chars.next() {
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '&' | '|' if chars.peek() == Some(&ch) => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                chars.next();
                tokens.push(format!("{ch}{ch}"));
            }
            ';' => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                tokens.push(";".to_string());
            }
            ch if ch.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn is_pnpm_command_token(token: &str) -> bool {
    let command = token
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(token)
        .trim_matches(|ch| ch == '"' || ch == '\'');
    command == "pnpm" || command.starts_with("pnpm@")
}

fn is_shell_command_separator(token: &str) -> bool {
    matches!(token, "&&" | "||" | ";")
}

fn pnpm_option_takes_value(token: &str) -> bool {
    matches!(
        token,
        "-C" | "--dir"
            | "--filter"
            | "--workspace-dir"
            | "--store-dir"
            | "--config"
            | "--package-import-method"
            | "--network-concurrency"
            | "--fetch-retries"
            | "--fetch-retry-factor"
            | "--fetch-retry-mintimeout"
            | "--fetch-retry-maxtimeout"
    ) || token.starts_with("--filter=")
        || token.starts_with("--dir=")
        || token.starts_with("--workspace-dir=")
        || token.starts_with("--store-dir=")
        || token.starts_with("--config.")
}

fn has_explicit_pnpm_build_policy(project_dir: &Path) -> bool {
    for dir in project_dir.ancestors() {
        for file in [
            "pnpm-workspace.yaml",
            "pnpm-workspace.yml",
            ".npmrc",
            ".pnpmrc",
            "package.json",
        ] {
            let path = dir.join(file);
            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };
            if file_contains_pnpm_build_policy(file, &contents) {
                return true;
            }
        }
    }
    false
}

fn file_contains_pnpm_build_policy(file_name: &str, contents: &str) -> bool {
    if matches!(file_name, ".npmrc" | ".pnpmrc") {
        return rc_file_contains_pnpm_build_policy(contents);
    }

    if file_name == "package.json" {
        return package_json_contains_pnpm_build_policy(contents);
    }

    yaml_file_contains_pnpm_build_policy(contents)
}

fn rc_file_contains_pnpm_build_policy(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            return false;
        }
        let Some((key, value)) = parse_rc_config_setting(line) else {
            return false;
        };
        pnpm_build_policy_setting_blocks_compat(key, value)
    })
}

fn yaml_file_contains_pnpm_build_policy(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = line.trim_start();
        if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
            return false;
        }
        let Some((key, value)) = parse_yaml_config_setting(line) else {
            return false;
        };
        pnpm_build_policy_setting_blocks_compat(key, value)
    })
}

fn package_json_contains_pnpm_build_policy(contents: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(contents) else {
        return false;
    };
    value
        .get("pnpm")
        .and_then(|pnpm| pnpm.as_object())
        .is_some_and(|pnpm| {
            pnpm.iter().any(|(key, value)| {
                let value = json_config_scalar(value);
                pnpm_build_policy_setting_blocks_compat(key, value.as_deref())
            })
        })
}

fn parse_rc_config_setting(line: &str) -> Option<(&str, Option<&str>)> {
    if let Some((key, value)) = line.split_once('=') {
        return Some((clean_config_key(key)?, Some(value.trim())));
    }

    let mut parts = line.splitn(2, char::is_whitespace);
    let key = clean_config_key(parts.next()?)?;
    Some((key, parts.next().map(str::trim)))
}

fn parse_yaml_config_setting(line: &str) -> Option<(&str, Option<&str>)> {
    let (key, value) = line.trim_start().split_once(':')?;
    let key = clean_config_key(key)?;
    let value = value.trim();
    Some((key, (!value.is_empty()).then_some(value)))
}

fn clean_config_key(key: &str) -> Option<&str> {
    let key = key.trim().trim_matches(|ch| ch == '"' || ch == '\'').trim();
    (!key.is_empty()).then_some(key)
}

fn json_config_scalar(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Bool(value) => Some(value.to_string()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn pnpm_build_policy_setting_blocks_compat(key: &str, value: Option<&str>) -> bool {
    match normalize_pnpm_build_policy_key(key).as_str() {
        "allowbuilds"
        | "dangerouslyallowallbuilds"
        | "onlybuiltdependencies"
        | "onlybuiltdependenciesfile"
        | "ignoredbuiltdependencies"
        | "neverbuiltdependencies" => true,
        "ignoredepscripts" | "strictdepbuilds" | "ignorescripts" => {
            config_bool_value(value).unwrap_or(true)
        }
        _ => false,
    }
}

fn config_bool_value(value: Option<&str>) -> Option<bool> {
    let value = value?
        .split(['#', ';'])
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches(|ch| ch == '"' || ch == '\'')
        .to_ascii_lowercase();
    match value.as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn normalize_pnpm_build_policy_key(key: &str) -> String {
    let normalized: String = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect();
    normalized
        .strip_prefix("pnpm")
        .unwrap_or(&normalized)
        .to_string()
}

fn run_build_step(
    cmd: &str,
    project_dir: &Path,
    json: bool,
    extra_env: &[(String, String)],
) -> anyhow::Result<()> {
    if cmd.trim().is_empty() {
        return Err(output::coded_error(
            "INVALID_CONFIG",
            "empty build command".to_string(),
        ));
    }

    output::status(json, ">", format!("Building: {cmd}"), output::Phase::Deploy);
    // Build child output → user stream (webpack/vite output is useful)
    run_command_streaming(
        cmd,
        project_dir,
        json,
        output::Phase::Build,
        "user",
        extra_env,
    )?;
    output::success(json, "Build completed", output::Phase::Deploy);
    Ok(())
}

// ── Output scan ──────────────────────────────────────────────

/// Read buffer for streaming SHA-256. Sized to match a single page-cache
/// readahead window — small enough to stay in L2 cache, large enough that the
/// per-file read overhead doesn't dominate hashing throughput on big assets.
const SCAN_HASH_CHUNK_BYTES: usize = 64 * 1024;

/// Recursively scan `dir` and return a sorted list of `FileEntry { path, size, content_hash }`.
///
/// SHA-256 and size are computed **streaming**: the file is read in
/// `SCAN_HASH_CHUNK_BYTES` chunks and fed into the hasher, never buffered into
/// memory. Bytes are re-read from disk at upload time (page cache absorbs the
/// second read on any reasonable build host).
///
/// Safe relative symlinks are preserved as SOURCE_BUNDLE_V1 logical entries.
fn scan_dir(dir: &Path) -> anyhow::Result<Vec<FileEntry>> {
    let mut files = Vec::new();
    let canonical_base = std::fs::canonicalize(dir)
        .with_context(|| format!("failed to canonicalize {}", dir.display()))?;
    scan_dir_recursive(dir, dir, &canonical_base, &mut files)?;
    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn prepare_deploy_files(
    manifest: &build_manifest::Manifest,
    files: Vec<FileEntry>,
    detection: &crate::detect::types::DetectionResult,
    json: bool,
) -> anyhow::Result<Vec<FileEntry>> {
    let original_count = files.len();
    let original_bytes = files.iter().map(|file| file.size).sum::<u64>();
    let mut pruned_count = 0usize;
    let mut pruned_bytes = 0u64;
    let mut deployable = Vec::with_capacity(files.len());

    for file in files {
        if is_framework_build_only_path(manifest, detection, &file.path) {
            pruned_count += 1;
            pruned_bytes = pruned_bytes.saturating_add(file.size);
            continue;
        }
        deployable.push(file);
    }

    if pruned_count > 0 {
        output::status(
            json,
            "~",
            format!(
                "Pruned {pruned_count}/{original_count} build-only artifact(s) from SOURCE_BUNDLE_V1 ({})",
                format_u64_bytes(pruned_bytes)
            ),
            output::Phase::Deploy,
        );
        tracing::info!(
            pruned_count,
            original_count,
            pruned_bytes,
            original_bytes,
            "pruned framework build-only artifacts before SOURCE_BUNDLE_V1 packaging"
        );
    }

    warn_large_deploy_files(json, &deployable);
    Ok(deployable)
}

fn is_framework_build_only_path(
    manifest: &build_manifest::Manifest,
    detection: &crate::detect::types::DetectionResult,
    path: &str,
) -> bool {
    if !manifest_has_compute_layer(manifest) {
        return false;
    }
    if !matches!(
        detection.framework.as_str(),
        "nextjs" | "blitzjs" | "payload"
    ) {
        return false;
    }

    path == ".next/cache" || path.starts_with(".next/cache/") || path.contains("/.next/cache/")
}

fn warn_large_deploy_files(json: bool, files: &[FileEntry]) {
    const LARGE_DEPLOY_FILE_WARNING_BYTES: u64 = 25 * 1024 * 1024;
    let mut large = files
        .iter()
        .filter(|file| file.size > LARGE_DEPLOY_FILE_WARNING_BYTES)
        .collect::<Vec<_>>();
    large.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
    if large.is_empty() {
        return;
    }

    let display = large
        .iter()
        .take(5)
        .map(|file| format!("{} ({})", file.path, format_u64_bytes(file.size)))
        .collect::<Vec<_>>()
        .join(", ");
    output::warn(
        json,
        format!(
            "Large deployment files detected before upload: {display}. \
             Server-side plan limits will be checked during upload preparation."
        ),
        output::Phase::Deploy,
    );
}

fn scan_dir_recursive(
    base: &Path,
    current: &Path,
    canonical_base: &Path,
    files: &mut Vec<FileEntry>,
) -> anyhow::Result<()> {
    let entries = std::fs::read_dir(current)
        .with_context(|| format!("failed to read directory {}", current.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", current.display()))?;
        let path = entry.path();
        let ft = entry
            .file_type()
            .with_context(|| format!("failed to stat {}", path.display()))?;

        if ft.is_symlink() {
            let rel = path
                .strip_prefix(base)
                .context("failed to compute relative path")?
                .to_string_lossy()
                .replace('\\', "/");
            let link_target = read_deploy_symlink_target(&path, &rel, canonical_base)?;
            files.push(FileEntry {
                path: rel,
                size: 0,
                content_hash: format!("{:x}", Sha256::digest(link_target.as_bytes())),
            });
            continue;
        }

        if ft.is_dir() {
            scan_dir_recursive(base, &path, canonical_base, files)?;
        } else if ft.is_file() {
            let rel = path
                .strip_prefix(base)
                .context("failed to compute relative path")?;
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let (size, content_hash) = hash_file_streaming(&path)
                .with_context(|| format!("failed to hash {}", rel_str))?;
            files.push(FileEntry {
                path: rel_str,
                size,
                content_hash,
            });
        }
    }

    Ok(())
}

fn read_deploy_symlink_target(
    path: &Path,
    rel: &str,
    canonical_base: &Path,
) -> anyhow::Result<String> {
    let target = std::fs::read_link(path)
        .with_context(|| format!("failed to read SOURCE_BUNDLE_V1 symlink {}", path.display()))?;
    let target = target.to_str().ok_or_else(|| {
        output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!(
                "SOURCE_BUNDLE_V1 symlink target is not UTF-8: {}",
                path.display()
            ),
        )
    })?;
    validate_deploy_symlink_target(rel, target)?;
    match std::fs::canonicalize(path) {
        Ok(canonical) if canonical.starts_with(canonical_base) => Ok(target.to_string()),
        Ok(canonical) => Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!(
                "SOURCE_BUNDLE_V1 symlink escapes build output: {rel} -> {target} resolved to {}",
                canonical.display()
            ),
        )),
        Err(error) => Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!("SOURCE_BUNDLE_V1 broken symlink in build output: {rel} -> {target} ({error})"),
        )),
    }
}

fn validate_deploy_symlink_target(rel: &str, target: &str) -> anyhow::Result<()> {
    if target.is_empty() || target.contains('\\') || target.contains('\0') {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!("unsafe SOURCE_BUNDLE_V1 symlink target: {rel} -> {target}"),
        ));
    }
    if source_bundle_contract_characters(target) > SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!(
                "SOURCE_BUNDLE_V1 symlink target too long: {rel} -> {target} (max {SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS} characters)"
            ),
        ));
    }
    let target_path = Path::new(target);
    if target_path.is_absolute() {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!("SOURCE_BUNDLE_V1 symlink has absolute target: {rel} -> {target}"),
        ));
    }

    let mut resolved = PathBuf::new();
    if let Some(parent) = Path::new(rel).parent()
        && !parent.as_os_str().is_empty()
    {
        resolved.push(parent);
    }
    for component in target_path.components() {
        match component {
            Component::Normal(part) => resolved.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    return Err(output::coded_error(
                        "INVALID_BUILD_OUTPUT",
                        format!("SOURCE_BUNDLE_V1 symlink escapes build output: {rel} -> {target}"),
                    ));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(output::coded_error(
                    "INVALID_BUILD_OUTPUT",
                    format!("unsafe SOURCE_BUNDLE_V1 symlink target: {rel} -> {target}"),
                ));
            }
        }
    }
    if resolved.as_os_str().is_empty() {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!("unsafe SOURCE_BUNDLE_V1 symlink target: {rel} -> {target}"),
        ));
    }
    Ok(())
}

/// Streaming SHA-256 + size for a single file. Returns `(size, lowercase_hex_sha256)`.
fn hash_file_streaming(path: &Path) -> anyhow::Result<(u64, String)> {
    let mut file =
        std::fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; SCAN_HASH_CHUNK_BYTES];
    let mut size: u64 = 0;
    loop {
        let n = file
            .read(&mut buf)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    Ok((size, format!("{:x}", hasher.finalize())))
}

// ── Synthetic commit SHA ─────────────────────────────────────

/// Stable per-deployment commit SHA derived from the file manifest. Used as a
/// fallback when `git rev-parse HEAD` is unavailable. Includes per-file content
/// hashes so two deploys of byte-identical bundles produce the same synthetic
/// SHA — which is exactly what cross-deploy CAS dedup keys off.
fn synthetic_sha(files: &[FileEntry]) -> String {
    let mut hasher = Sha256::new();
    for f in files {
        hasher.update(f.path.as_bytes());
        hasher.update(b":");
        hasher.update(f.content_hash.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

// ── Compute type resolution ──────────────────────────────────

fn resolve_deploy_compute_type(
    explicit_compute: Option<ComputeType>,
    manifest: Option<&build_manifest::Manifest>,
    detection: &crate::detect::types::DetectionResult,
) -> ComputeType {
    if let Some(explicit) = explicit_compute {
        return explicit;
    }

    if let Some(manifest) = manifest {
        return compute_type_from_manifest(manifest);
    }

    detection.suggested_compute
}

fn resolve_explicit_compute_type(
    cli_flag: Option<&str>,
    config_value: Option<&str>,
) -> anyhow::Result<Option<ComputeType>> {
    if let Some(val) = cli_flag {
        return parse_compute_type(val).map(Some);
    }

    if let Some(val) = config_value {
        return parse_compute_type(val).map(Some);
    }

    Ok(None)
}

fn compute_type_from_manifest(manifest: &build_manifest::Manifest) -> ComputeType {
    match build_manifest::primary_compute_target(manifest) {
        build_manifest::LayerTarget::Compute => ComputeType::Process,
        build_manifest::LayerTarget::Static => ComputeType::Static,
    }
}

fn parse_compute_type(s: &str) -> anyhow::Result<ComputeType> {
    match s.to_lowercase().as_str() {
        "static" => Ok(ComputeType::Static),
        "process" => Ok(ComputeType::Process),
        _ => Err(output::coded_error(
            "INVALID_COMPUTE_TYPE",
            format!("invalid compute type: \"{s}\". Must be one of: static, process"),
        )),
    }
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

fn format_u64_bytes(bytes: u64) -> String {
    format_bytes(usize::try_from(bytes).unwrap_or(usize::MAX))
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
