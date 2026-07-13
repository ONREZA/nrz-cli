// The deploy root keeps shared private types and end-to-end orchestration;
// command, upload, runtime artifact, scanning, and build-log domains live in
// focused sibling modules.
mod build_logs;
#[cfg(test)]
mod build_logs_tests;
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod bundle;
#[cfg(test)]
mod bundle_tests;
mod command;
#[cfg(test)]
mod deploy_tests;
pub(crate) mod hash;
pub(crate) mod health_check;
#[cfg(test)]
mod health_check_tests;
mod plan;
mod runtime_artifact;
mod scan;
mod source_upload;
mod verify;

use build_logs::*;
use command::*;
use health_check::resolve_health_check;
#[cfg(test)]
use health_check::validate_health_path;
use runtime_artifact::*;
pub(crate) use scan::hash_file_streaming;
#[cfg(test)]
pub(crate) use scan::scan_dir;
use scan::*;
use source_upload::*;

use std::collections::{HashSet, VecDeque};
use std::io::Read;
use std::num::NonZeroU64;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use bytes::Bytes;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::api::{ApiClient, ConditionalUploadConflict, PresignedHeadVerify, PresignedPutHeaders};
use crate::artifact::source_bundle_v1::{
    self, CLI_PROTOCOL_VERSION, CompletedMultipartPart, PresignedSourceMultipartChunk,
    SOURCE_BUNDLE_FORMAT, SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS, SourceBundlePlan,
    source_bundle_contract_characters,
};
use crate::artifact::{ArtifactRootScope, FileEntry, RuntimeArtifact, RuntimeArtifactScan};
use crate::auth;
use crate::build::manifest as build_manifest;
use crate::cli::DeployArgs;
use crate::deploy::hash::{sha256_finalize_hex, sha256_hex};
use crate::detect::types::{ComputeType, RuntimeType};
use crate::link;
use crate::output;
use nrz::config::{EffectiveProjectConfig, EnvVisibility, ProjectConfig};
use nrz_contract::cli_api::{
    OnrezaCliApiV1MultipartCompleteRequestPartsItem as CliMultipartCompletePart,
    OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummary as CliLogicalManifestSummary,
    OnrezaCliApiV1PrepareUploadRequestMultipart as CliPrepareMultipart,
    OnrezaCliApiV1PrepareUploadRequestMultipartPartsItem as CliPrepareMultipartPart,
    OnrezaCliApiV1PrepareUploadRequestSourceUploadRecovery as CliPrepareUploadSourceUploadRecovery,
};
use nrz_contract::{
    CliMultipartCompleteRequest, CliMultipartCompleteResponse, CliPrepareUploadRequest,
    CliPrepareUploadRequiredComplete, CliPrepareUploadResponse,
    CliPrepareUploadResponseMultipartChunk, CliPrepareUploadResponsePresignedPutHeaders,
    CliPrepareUploadResponsePresignedPutVerifyHead, CliUploadCompleteRequest,
    CliUploadCompleteResponse, CliUploadFailedRequest, CliUploadFailedResponse,
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
const SOURCE_UPLOAD_RECOVERY_CONDITIONAL_PRECONDITION_FAILED: &str =
    "conditional-precondition-failed";
const NEXTJS_ADAPTER_EDGE_RULE_PRODUCER: &str = "nextjs-adapter";
const PNPM_BUILD_SCRIPT_COMPAT_ENV: [(&str, &str); 2] = [
    ("npm_config_dangerously_allow_all_builds", "true"),
    ("pnpm_config_dangerously_allow_all_builds", "true"),
];

#[derive(Debug)]
struct DeploySymlinkTarget {
    link_target: String,
    resolved_path: String,
}

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
    build_log_protocol: &'static str,
    manifest: serde_json::Value,
    files: Vec<FileEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    production: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    commit_sha: String,
    /// ONREZA Functions published alongside this deployment. Function source is
    /// DB-backed and intentionally not part of SOURCE_BUNDLE_V1.
    #[serde(skip_serializing_if = "Option::is_none")]
    functions: Option<serde_json::Value>,
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
#[serde(rename_all = "camelCase")]
struct DeployOutput {
    deployment_id: String,
    url: String,
    status: String,
    target: DeployTargetOutput,
    preview_protected: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health_check: Option<HealthCheckInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification: Option<verify::DeployVerificationOutput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeployTargetOutput {
    environment: &'static str,
    production: Option<bool>,
}

fn deploy_target_output(production: Option<bool>) -> DeployTargetOutput {
    DeployTargetOutput {
        environment: deploy_target_environment(production),
        production,
    }
}

fn deploy_target_environment(production: Option<bool>) -> &'static str {
    match production {
        Some(true) => "production",
        Some(false) => "preview",
        None => "default",
    }
}

fn deploy_preview_protected(production: Option<bool>) -> bool {
    production != Some(true)
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
    edge_rules_force: bool,
) -> anyhow::Result<Option<crate::functions::FunctionPublishPayload>> {
    let collected = crate::functions::collect(project_dir)?;
    let user_edge_rules = crate::functions::load_edge_rules(project_dir)?;
    let generated_edge_rule_sets = generated_nextjs_edge_rule_sets(project_dir, json)?;
    let edge_rule_count = user_edge_rules
        .as_ref()
        .map_or(0, crate::functions::edge_rule_count)
        + generated_edge_rule_sets
            .iter()
            .map(|rule_set| crate::functions::edge_rule_count(&rule_set.edge_rules))
            .sum::<usize>();

    if collected.is_empty() && user_edge_rules.is_none() && generated_edge_rule_sets.is_empty() {
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
        user_edge_rules,
        edge_rules_force,
        generated_edge_rule_sets,
    )))
}

fn generated_nextjs_edge_rule_sets(
    project_dir: &Path,
    json: bool,
) -> anyhow::Result<Vec<crate::functions::GeneratedEdgeRuleSet>> {
    let Some(descriptor) = crate::nextjs_adapter::load_descriptor(project_dir)? else {
        return Ok(Vec::new());
    };
    if descriptor.version != 1 {
        return Ok(Vec::new());
    }
    output::status(
        json,
        "~",
        descriptor.compatibility_report_line(),
        output::Phase::Deploy,
    );
    let edge_rules = descriptor.generated_edge_rules().unwrap_or_else(|| {
        serde_json::json!({
            "schemaVersion": "EDGE_RULE_SET_V1",
            "rules": [],
        })
    });
    let rule_count = crate::functions::edge_rule_count(&edge_rules);
    output::status(
        json,
        "~",
        format!("Generated {rule_count} Next.js Edge Rule(s) from adapter routing"),
        output::Phase::Deploy,
    );
    Ok(vec![crate::functions::GeneratedEdgeRuleSet {
        producer: NEXTJS_ADAPTER_EDGE_RULE_PRODUCER.to_string(),
        version: descriptor.next_version.or(descriptor.adapter.version),
        edge_rules,
    }])
}

pub async fn run(
    args: DeployArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let mut command_context =
        crate::context::CommandContext::resolve(&args.dir, config, args.app.as_deref(), json)?;
    if let Some(app) = &command_context.selected_app {
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

    // Verify auth early to avoid wasting time on build if token is invalid
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;
    let resume_deployment_id = args.resume_deployment.as_deref().map(str::trim);
    if let Some(deployment_id) = resume_deployment_id
        && deployment_id.is_empty()
    {
        return Err(crate::errors::CliError::new(
            "INVALID_ARGUMENT",
            "--resume-deployment requires a non-empty deployment ID",
        )
        .phase(output::Phase::Deploy)
        .details(serde_json::json!({ "argument": "--resume-deployment" }))
        .hint("Pass a deployment ID or omit --resume-deployment.")
        .into_anyhow());
    }

    // Early-resolve project_id before build/detect so server settings can be
    // imported into the same effective config as local onreza.toml.
    command_context.apply_project_id_override(args.project_id.as_deref())?;
    let mut early_project_id = command_context.effective.project_id().map(str::to_string);
    if early_project_id.is_none()
        && let Some(deployment_id) = resume_deployment_id
    {
        early_project_id = Some(
            resolve_project_id_for_resume(&client, deployment_id, None, &command_context.config)
                .await?,
        );
    }
    if !args.dry && early_project_id.is_none() {
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
            &command_context.project_dir,
            &selected.project_id,
            Some(&selected.project_name),
            None,
        )?;
        crate::init::add_to_gitignore(&command_context.project_dir);
        output::success(
            false,
            format!(
                "Linked to {}",
                console::style(&selected.project_name).bold()
            ),
            output::Phase::Deploy,
        );
        early_project_id = Some(selected.project_id);
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

    command_context.apply_server_settings(server_settings.as_ref());

    // Explicit compute intent is safe to resolve before build because it comes
    // only from CLI/config. Framework detection stays post-build: generated
    // outputs such as root index.html are part of the detection surface.
    let explicit_compute = resolve_explicit_compute_type(
        args.compute.as_deref(),
        command_context.effective.deploy_compute(),
    )?;
    if args.dry {
        let deploy_plan = plan::build(plan::DeployPlanRequest {
            args: &args,
            command: &command_context,
            explicit_compute,
            build_logs: None,
        })
        .await?;
        let source_bundle = deploy_plan.materialize_source_bundle(json)?;
        let explain = deploy_plan.explain(
            &command_context,
            early_project_id.as_deref(),
            &source_bundle,
        );
        emit_deploy_plan_explain(json, &explain)?;
        return Ok(());
    }

    // SOURCE_BUNDLE_V1 prepare-upload needs the authenticated workspace ID for
    // direct CLI deploy and builder resume. Admission/limits are enforced
    // server-side from the logical manifest plus source archive descriptor.
    let ws_info: WorkspaceInfo = client
        .get("/v1/workspace")
        .await
        .context("failed to fetch workspace info")?;

    let project_id = early_project_id
        .clone()
        .context("project must be resolved before build-log session creation")?;
    let build_log_redaction_keys = command_context
        .config
        .env
        .declarations
        .iter()
        .filter(|(_, declaration)| declaration.visibility == EnvVisibility::Sensitive)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let mut build_log_session = BuildLogSession::start(
        &client,
        &project_id,
        &ws_info.id,
        &command_context.project_dir,
        &build_log_redaction_keys,
        BuildLogUploadConfig::from_args(&args),
        json,
    )
    .await;
    if let Some(deployment_id) = resume_deployment_id
        && let Some(session) = build_log_session.as_mut()
    {
        session.attach(deployment_id).await;
    }

    let deploy_result = async {
        let deploy_plan = plan::build(plan::DeployPlanRequest {
            args: &args,
            command: &command_context,
            explicit_compute,
            build_logs: build_log_session
                .as_ref()
                .and_then(BuildLogSession::emitter),
        })
        .await?;
        if let Some(emitter) = build_log_session
            .as_ref()
            .and_then(BuildLogSession::emitter)
        {
            emitter.info(BuildLogPhase::Detect, "Build output detected and validated");
        }

        // ── Resume mode: builder calls us with an existing deployment ID ──
        if let Some(deployment_id) = &args.resume_deployment {
            let deployment_id = deployment_id.trim();
            let upload_plan = deploy_plan.materialize_source_bundle(json)?;
            return resume_deploy(ResumeDeployRequest {
                client: &client,
                deployment_id,
                workspace_id: &ws_info.id,
                project_id: &project_id,
                functions: deploy_plan.functions,
                upload_plan,
                json,
                warnings: deploy_plan.warnings,
            })
            .await;
        }

        // ── Normal flow continues below ─────────────────────────────────

        let deploy_files = deploy_plan.files.clone();
        let deploy_warnings = deploy_plan.warnings.clone();
        let deploy_health_check = deploy_plan.health_check.clone();
        let deploy_production = deploy_plan.production;
        let sync_detection = deploy_plan.artifact.build.detection.clone();

        // Git info
        let branch = git_cmd(&["rev-parse", "--abbrev-ref", "HEAD"]);
        let commit_sha = git_cmd(&["rev-parse", "HEAD"]).unwrap_or_else(|| {
            output::warn(
                json,
                "git not available, using synthetic commit SHA",
                output::Phase::Deploy,
            );
            synthetic_sha(&deploy_files)
        });

        // Validate required env vars from [env] declarations
        if !args.skip_env_check {
            crate::cli::env_handler::validate_env_for_deploy(
                &client,
                &project_id,
                json,
                &command_context.config,
            )
            .await?;
        }

        let upload_plan = deploy_plan.materialize_source_bundle(json)?;
        let deploy_functions = deploy_plan.functions;

        // Sync detection results to API (best-effort, non-blocking)
        let sync_client = client.clone();
        let sync_project_id = project_id.clone();
        let _sync = tokio::spawn(async move {
            crate::detect_sync::sync_detection_to_api(
                &sync_client,
                &sync_project_id,
                &sync_detection,
            )
            .await;
        });

        // Sync compute config (health check path) for PROCESS deployments
        if let Some(ref hc) = deploy_health_check {
            let hc_client = client.clone();
            let hc_project_id = project_id.clone();
            let hc_clone = hc.clone();
            let _hc = tokio::spawn(async move {
                sync_compute_config(&hc_client, &hc_project_id, &hc_clone, json).await;
            });
        }

        output::success(
            json,
            format!(
                "SOURCE_BUNDLE_V1 archive ready ({}, sha256: {}...)",
                format_u64_bytes(upload_plan.source_size_bytes),
                &upload_plan.source_sha256[..12]
            ),
            output::Phase::Deploy,
        );
        let deployment_attempt_id = Uuid::now_v7().to_string();

        // Create deployment
        output::status(json, "~", "Creating deployment...", output::Phase::Deploy);
        let body = CreateDeploymentBody {
            build_log_protocol: build_log_protocol(),
            manifest: deploy_plan.manifest_raw,
            files: deploy_files.clone(),
            production: deploy_production,
            branch,
            commit_sha,
            functions: conform_functions_to_wire_contract(deploy_functions)?,
        };

        let deployment: CreateDeploymentResponse = match client
            .post(&format!("/v1/projects/{}/deployments", project_id), &body)
            .await
        {
            Ok(deployment) => deployment,
            Err(error) => return Err(map_create_deployment_error(error, json)),
        };
        if let Some(session) = build_log_session.as_mut() {
            session.attach(&deployment.id).await;
        }
        if let Some(emitter) = build_log_session
            .as_ref()
            .and_then(BuildLogSession::emitter)
        {
            emitter.info(BuildLogPhase::Upload, "Uploading deployment source bundle");
        }

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
        if let Some(emitter) = build_log_session
            .as_ref()
            .and_then(BuildLogSession::emitter)
        {
            emitter.info(
                BuildLogPhase::Activate,
                "Deployment source uploaded; waiting for activation",
            );
        }

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
                    let target = deploy_target_output(deploy_production);
                    let preview_protected = deploy_preview_protected(deploy_production);
                    let verification = if args.verify {
                        Some(
                            verify::verify_deployment(verify::DeployVerificationRequest {
                                api_client: &client,
                                project_id: &project_id,
                                url,
                                preview_protected,
                                health_check: deploy_health_check.as_ref(),
                                json,
                            })
                            .await?,
                        )
                    } else {
                        None
                    };

                    if json {
                        output::json_output(&DeployOutput {
                            deployment_id: deployment.id,
                            url: url.to_string(),
                            status: "live".into(),
                            target,
                            preview_protected,
                            warnings: deploy_warnings.clone(),
                            health_check: deploy_health_check.as_ref().map(|hc| hc.to_info()),
                            verification,
                        });
                    } else {
                        eprintln!();
                        eprintln!(
                            "  {} Deployed to {}",
                            console::style("✓").green().bold(),
                            console::style(url).underlined().bold(),
                        );
                        if let Some(verification) = &verification {
                            eprintln!(
                                "  {} Verified {} ({})",
                                console::style("✓").green().bold(),
                                console::style(&verification.url).underlined(),
                                verification.status_code
                            );
                        }
                        eprintln!();
                        if preview_protected {
                            crate::preview::print_preview_access_hint(&project_id, Some(url));
                        }
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
    .await;

    if let Some(session) = build_log_session.as_mut() {
        session.finish(&deploy_result).await;
    }
    deploy_result
}

fn emit_deploy_plan_explain(json: bool, explain: &plan::DeployPlanExplain) -> anyhow::Result<()> {
    if json {
        output::json_output(explain);
    } else {
        eprintln!("Deployment plan:");
        eprintln!("{}", serde_json::to_string_pretty(explain)?);
    }
    Ok(())
}

fn resolve_deploy_production_override(prod: bool, env: &[String]) -> anyhow::Result<Option<bool>> {
    let mut selected = None;

    for raw in env {
        let value = raw.trim();
        if value.is_empty() {
            continue;
        }
        let production = match value.to_ascii_uppercase().as_str() {
            "PRODUCTION" | "PROD" => true,
            "PREVIEW" => false,
            "DEVELOPMENT" | "DEV" => {
                anyhow::bail!("nrz deploy supports only --env production or --env preview");
            }
            _ => {
                anyhow::bail!(
                    "nrz deploy does not support arbitrary environment IDs or names in --env yet; use --env production or --env preview"
                );
            }
        };

        if selected.is_some_and(|current| current != production) {
            anyhow::bail!("nrz deploy accepts only one target environment");
        }
        selected = Some(production);
    }

    if prod {
        if selected == Some(false) {
            anyhow::bail!("--prod conflicts with --env preview");
        }
        return Ok(Some(true));
    }

    Ok(selected)
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

struct ResumeDeployRequest<'a> {
    client: &'a ApiClient,
    deployment_id: &'a str,
    workspace_id: &'a str,
    project_id: &'a str,
    functions: Option<crate::functions::FunctionPublishPayload>,
    upload_plan: SourceBundlePlan,
    warnings: Vec<String>,
    json: bool,
}

async fn resume_deploy(request: ResumeDeployRequest<'_>) -> anyhow::Result<()> {
    let ResumeDeployRequest {
        client,
        deployment_id,
        workspace_id,
        project_id,
        functions,
        upload_plan,
        warnings,
        json,
    } = request;

    output::status(
        json,
        "~",
        format!("Resuming deployment {deployment_id}"),
        output::Phase::Deploy,
    );

    if let Some(functions) = &functions {
        stage_deployment_functions(client, deployment_id, project_id, functions, json).await?;
    }

    output::success(
        json,
        format!(
            "SOURCE_BUNDLE_V1 archive ready ({}, sha256: {}...)",
            format_u64_bytes(upload_plan.source_size_bytes),
            &upload_plan.source_sha256[..12]
        ),
        output::Phase::Deploy,
    );
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
        output::json_output(&data);
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
    let edge_rule_count = functions_payload_edge_rule_count(functions);
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

fn functions_payload_edge_rule_count(
    functions: &crate::functions::FunctionPublishPayload,
) -> usize {
    functions
        .edge_rules
        .as_ref()
        .map_or(0, crate::functions::edge_rule_count)
        + functions
            .generated_edge_rule_sets
            .iter()
            .map(|rule_set| crate::functions::edge_rule_count(&rule_set.edge_rules))
            .sum::<usize>()
}

fn map_function_stage_error(error: anyhow::Error, json: bool) -> anyhow::Error {
    let Some(api_error) = error.downcast_ref::<crate::api::StructuredApiError>() else {
        return error.context("failed to stage ONREZA Functions for deployment");
    };
    if let Some(mapped) = map_edge_rules_diverged_error(
        api_error,
        json,
        "failed to stage ONREZA Functions for deployment",
    ) {
        return mapped;
    }
    if api_error.code != "FUNCTION_PUBLISH_FAILED" {
        if json {
            let message = format!("failed to stage ONREZA Functions for deployment: {api_error}");
            return output::report_terminal_error(
                "deploy",
                &message,
                &api_error.code,
                api_error.details.as_ref(),
            );
        }
        return error.context("failed to stage ONREZA Functions for deployment");
    }

    let message = format_function_publish_failure(api_error);
    if json {
        return output::report_terminal_error(
            "deploy",
            &message,
            &api_error.code,
            api_error.details.as_ref(),
        );
    }
    anyhow::anyhow!(message).context("failed to stage ONREZA Functions for deployment")
}

fn map_create_deployment_error(error: anyhow::Error, json: bool) -> anyhow::Error {
    let Some(api_error) = error.downcast_ref::<crate::api::StructuredApiError>() else {
        return error.context("failed to create deployment");
    };
    if let Some(mapped) =
        map_edge_rules_diverged_error(api_error, json, "failed to create deployment")
    {
        return mapped;
    }
    if api_error.code != "FUNCTION_PUBLISH_FAILED" {
        if json {
            let message = format!("failed to create deployment: {api_error}");
            return output::report_terminal_error(
                "deploy",
                &message,
                &api_error.code,
                api_error.details.as_ref(),
            );
        }
        return error.context("failed to create deployment");
    }

    let message = format_function_publish_failure(api_error);
    if json {
        return output::report_terminal_error(
            "deploy",
            &message,
            &api_error.code,
            api_error.details.as_ref(),
        );
    }
    anyhow::anyhow!(message).context("failed to create deployment")
}

fn map_edge_rules_diverged_error(
    error: &crate::api::StructuredApiError,
    json: bool,
    context: &str,
) -> Option<anyhow::Error> {
    if !is_edge_rules_diverged_error(error) {
        return None;
    }
    let message = format_edge_rules_diverged_failure(error);
    if json {
        return Some(output::report_terminal_error(
            "deploy",
            &message,
            "EDGE_RULES_DIVERGED",
            error.details.as_ref(),
        ));
    }
    Some(output::coded_error("EDGE_RULES_DIVERGED", message).context(context.to_string()))
}

fn is_edge_rules_diverged_error(error: &crate::api::StructuredApiError) -> bool {
    if error.code == "EDGE_RULES_DIVERGED" {
        return true;
    }
    error.code == "FUNCTION_PUBLISH_FAILED"
        && error
            .details
            .as_ref()
            .and_then(|details| details.get("errorCode"))
            .and_then(serde_json::Value::as_str)
            == Some("EDGE_RULES_DIVERGED")
}

fn format_edge_rules_diverged_failure(error: &crate::api::StructuredApiError) -> String {
    let message = error
        .details
        .as_ref()
        .and_then(|details| details.get("message"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or(error.message.as_str());
    if message.contains("nrz rules pull") && message.contains("--force-rules") {
        return format!("Edge Rules diverged: {message}");
    }
    format!(
        "Edge Rules diverged: {message}. Run `nrz rules pull` to import dashboard-authored rules, or redeploy with `--force-rules` to replace them."
    )
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

// ── Runtime artifact resolution ───────────────────────────────

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
