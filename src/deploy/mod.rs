mod pre_source;
use pre_source::*;
mod source_registration;
use source_registration::*;
#[cfg(test)]
mod publication_wire_tests;
mod wire;
// Shared private types and helpers for deployment commands. Stateful admission
// and publication orchestration lives in workflow; activation owns status reads
// and wait deadlines. Public command entrypoints remain re-exported here.
mod activation;
#[cfg(test)]
mod activation_tests;
#[cfg(test)]
mod status_wire_tests;
mod workflow;
pub use workflow::run;

mod build_logs;
#[cfg(test)]
mod build_logs_tests;
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod bundle;
#[cfg(test)]
mod bundle_tests;
mod command;
mod dependency_scripts;
#[cfg(test)]
mod dependency_scripts_tests;
#[cfg(test)]
mod deploy_tests;
mod edge_handoff;
#[cfg(test)]
mod edge_handoff_tests;
pub(crate) mod hash;
pub(crate) mod health_check;
#[cfg(test)]
mod health_check_tests;
mod ignored_build;
#[cfg(test)]
mod ignored_build_tests;
mod package_manager_toolchain;
#[cfg(test)]
mod package_manager_toolchain_tests;
mod plan;
mod python_toolchain;
#[cfg(test)]
mod python_toolchain_tests;
mod runtime_artifact;
mod scan;
mod source_upload;
mod verify;
#[cfg(test)]
mod verify_tests;

use activation::*;
use build_logs::*;
use command::*;
use health_check::resolve_health_check;
#[cfg(test)]
use health_check::validate_health_path;
use ignored_build::{IgnoredBuildOutcome, IgnoredBuildRequest};
use runtime_artifact::*;
pub(crate) use scan::hash_file_streaming;
#[cfg(test)]
pub(crate) use scan::scan_dir;
use scan::*;
use source_upload::*;

use std::collections::{HashSet, VecDeque};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::api::{ApiClient, classify_api_retry};
use crate::artifact::source_bundle_v1::{
    SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS, SourceBundlePlan, source_bundle_contract_characters,
};
use crate::artifact::{ArtifactRootScope, FileEntry, RuntimeArtifact, RuntimeArtifactScan};
use crate::auth;
use crate::build::manifest as build_manifest;
use crate::cli::DeployArgs;
use crate::deploy::hash::{sha256_finalize_hex, sha256_hex};
use crate::detect::types::{ComputeType, RuntimeType};
use crate::link;
use crate::output;
use nrz::config::{EffectiveProjectConfig, ProjectBuildSettings, ProjectConfig};
use uuid::Uuid;

const SOURCE_REGISTRATION_RETRY_BUDGET: Duration = Duration::from_secs(10 * 60);
const SOURCE_REGISTRATION_REQUEST_TIMEOUT: Duration = Duration::from_secs(370);
const SOURCE_REGISTRATION_INITIAL_RETRY_DELAY: Duration = Duration::from_millis(500);
const SOURCE_REGISTRATION_MAX_RETRY_DELAY: Duration = Duration::from_secs(5);
const PRE_SOURCE_FAILURE_RETRY_BUDGET: Duration = Duration::from_secs(10);
const PRE_SOURCE_FAILURE_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const PRE_SOURCE_FAILURE_INITIAL_RETRY_DELAY: Duration = Duration::from_millis(250);
const PRE_SOURCE_FAILURE_MAX_RETRY_DELAY: Duration = Duration::from_secs(2);
const MAX_PRE_SOURCE_FAILURE_LOG_LENGTH: usize = 4096;
const NEXTJS_ADAPTER_EDGE_RULE_PRODUCER: &str = "nextjs-adapter";
#[derive(Debug)]
struct DeploySymlinkTarget {
    link_target: String,
    resolved_path: String,
}

// ── Project settings from server ─────────────────────────────

#[cfg(test)]
fn authoritative_server_framework_preset(preset: Option<&str>) -> Option<&str> {
    nrz::config::normalize_authoritative_framework(preset)
}

#[cfg(test)]
type ProjectInfo = nrz::config::ProjectBuildSettings;

// ── Deployment state ────────────────────────────────────────

#[derive(Debug)]
struct AdmissionDeployment {
    id: String,
    attempt: u32,
    status: String,
    url: String,
}

#[derive(Debug)]
struct AdmissionResponse {
    context: crate::execution_context::ExecutionContext,
    deployment: AdmissionDeployment,
}

type PreSourceFailureCode = nrz_api::FailBeforeSourceRequestBodyErrorCode;

#[derive(Debug, Serialize)]
struct PreSourceFailureDiagnostic {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkippedDeployOutput<'a> {
    deployment_id: &'a str,
    status: &'static str,
}

#[derive(Debug)]
struct RunnerDeploymentContext {
    id: String,
    attempt: u32,
    status: String,
    url: Option<String>,
}

#[derive(Debug)]
struct RunnerContextResponse {
    context: crate::execution_context::ExecutionContext,
    deployment: RunnerDeploymentContext,
    settings: ProjectBuildSettings,
}

fn require_runner_context_protocol(protocol: &str) -> anyhow::Result<()> {
    if protocol == crate::execution_context::RUNNER_CONTEXT_PROTOCOL {
        return Ok(());
    }
    Err(output::coded_error(
        "CLI_UPDATE_REQUIRED",
        format!(
            "unsupported runner context protocol {protocol}; expected {}",
            crate::execution_context::RUNNER_CONTEXT_PROTOCOL
        ),
    ))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeployOutput {
    deployment_id: String,
    url: String,
    status: String,
    target: DeployTargetOutput,
    preview_protected: bool,
    runtime_artifact_files: crate::artifact::RuntimeArtifactFileBreakdown,
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

/// Discover ONREZA Functions, validate them with the pinned native runtime,
/// and assemble the deployment-owned publish snapshot. Even an empty snapshot
/// is sent so the platform can retire generated adapter config from a previous
/// deployment without touching USER-owned Edge Rules.
async fn build_functions_payload(
    _config: &ProjectConfig,
    project_dir: &Path,
    json: bool,
    edge_rules_force: bool,
) -> anyhow::Result<Option<crate::functions::FunctionPublishPayload>> {
    let mut collected = crate::functions::collect(project_dir)
        .map_err(|error| output::with_default_code(error, "INVALID_CONFIG"))?;
    let user_edge_rules = crate::functions::load_edge_rules(project_dir)
        .map_err(|error| output::with_default_code(error, "INVALID_CONFIG"))?;
    let generated_edge_rule_sets = generated_nextjs_edge_rule_sets(project_dir, json)?;
    let edge_rule_count = user_edge_rules
        .as_ref()
        .map_or(0, crate::functions::edge_rule_count)
        + generated_edge_rule_sets
            .iter()
            .map(|rule_set| crate::functions::edge_rule_count(&rule_set.edge_rules))
            .sum::<usize>();

    let has_visible_config =
        !collected.is_empty() || user_edge_rules.is_some() || !generated_edge_rule_sets.is_empty();

    if !collected.is_empty() {
        let runtime = crate::functions_runtime::preflight(&mut collected).await?;
        output::status(
            json,
            "✓",
            format!(
                "{} loaded {} function(s) for {}",
                runtime.runtime_release_id, runtime.functions_loaded, runtime.target
            ),
            output::Phase::Deploy,
        );
    }
    if has_visible_config {
        output::success(
            json,
            format_function_publish_summary(
                collected.functions.len(),
                collected.source_file_count(),
                edge_rule_count,
            ),
            output::Phase::Deploy,
        );
    }
    Ok(Some(crate::functions::build_payload(
        "DEPLOYMENT",
        &collected,
        user_edge_rules,
        edge_rules_force,
        generated_edge_rule_sets,
    )?))
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
    let mut edge_rules = descriptor.generated_edge_rules().unwrap_or_else(|| {
        serde_json::json!({
            "schemaVersion": "EDGE_RULE_SET_V1",
            "rules": [],
        })
    });
    let image_sources = descriptor.generated_remote_image_sources();
    let image_source_count = image_sources.len();
    edge_rules
        .as_object_mut()
        .expect("generated Next.js Edge Rules are an object")
        .insert(
            "imageSources".to_string(),
            serde_json::Value::Array(image_sources),
        );
    crate::functions::validate_edge_rules_value(
        "Next.js adapter generated Edge Rules",
        &edge_rules,
    )
    .context("Next.js adapter produced an invalid Edge Rules payload")?;
    let rule_count = crate::functions::edge_rule_count(&edge_rules);
    output::status(
        json,
        "~",
        format!(
            "Generated {rule_count} Next.js Edge Rule(s) and {} remote image source(s) from adapter config",
            image_source_count
        ),
        output::Phase::Deploy,
    );
    Ok(vec![crate::functions::GeneratedEdgeRuleSet {
        producer: NEXTJS_ADAPTER_EDGE_RULE_PRODUCER.to_string(),
        version: descriptor.next_version.or(descriptor.adapter.version),
        edge_rules,
    }])
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

// ── Compute config sync ──────────────────────────────────────

fn compute_config_body(health_check_path: Option<String>) -> nrz_api::ComputeConfigRequestBody {
    nrz_api::ComputeConfigRequestBody {
        health_check_path: Some(health_check_path),
        ..Default::default()
    }
}

/// Best-effort sync of compute config (health check path) to the platform.
async fn sync_compute_config(
    client: &ApiClient,
    project_id: &str,
    health_check: &ResolvedHealthCheck,
    json: bool,
) {
    let body = compute_config_body(health_check.path.clone());
    let resp = client.update_compute_config(project_id, body).await;
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
#[derive(Debug, Serialize)]
struct ResumeDeployOutput {
    deployment_id: String,
    status: String,
    runtime_artifact_files: crate::artifact::RuntimeArtifactFileBreakdown,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

fn map_source_registration_error(error: anyhow::Error, json: bool, context: &str) -> anyhow::Error {
    let Some(api_error) = error.downcast_ref::<crate::api::StructuredApiError>() else {
        return error.context(context.to_string());
    };
    if let Some(mapped) = map_edge_rules_diverged_error(api_error, json, context) {
        return mapped;
    }
    let message = format_structured_api_failure(context, api_error);
    if json {
        return output::report_terminal_error(
            "deploy",
            &message,
            &api_error.code,
            api_error.details.as_ref(),
        );
    }
    let mut error =
        crate::errors::CliError::new(&api_error.code, message).phase(output::Phase::Deploy);
    if let Some(details) = api_error.details.clone() {
        error = error.details(details);
    }
    error.into_anyhow()
}

fn format_structured_api_failure(context: &str, error: &crate::api::StructuredApiError) -> String {
    let fields = error
        .details
        .as_ref()
        .and_then(|details| details.get("fields"))
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|field| {
            let field_name = field.get("field")?.as_str()?;
            let message = field.get("message")?.as_str()?;
            Some(format!("{field_name}: {message}"))
        })
        .collect::<Vec<_>>();
    if fields.is_empty() {
        format!("{context}: {}", error.message)
    } else {
        format!("{context}: {}", fields.join("; "))
    }
}

struct ResumeDeployRequest<'a> {
    client: &'a ApiClient,
    deployment_id: &'a str,
    workspace_id: &'a str,
    project_id: &'a str,
    upload_plan: SourceBundlePlan,
    warnings: Vec<String>,
    runtime_artifact_files: crate::artifact::RuntimeArtifactFileBreakdown,
    json: bool,
}

async fn resume_deploy(request: ResumeDeployRequest<'_>) -> anyhow::Result<()> {
    let ResumeDeployRequest {
        client,
        deployment_id,
        workspace_id,
        project_id,
        upload_plan,
        warnings,
        runtime_artifact_files,
        json,
    } = request;

    output::status(
        json,
        "~",
        format!("Resuming deployment {deployment_id}"),
        output::Phase::Deploy,
    );

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

    prepare_upload_and_complete(PrepareUploadAndCompleteRequest {
        client,
        deployment_id,
        workspace_id,
        project_id,
        deployment_attempt_id: &deployment_attempt_id,
        json,
        plan: &upload_plan,
        runtime_artifact_files: &runtime_artifact_files,
    })
    .await?;

    // Output result (no polling in resume mode — builder handles status)
    if json {
        let data = ResumeDeployOutput {
            deployment_id: deployment_id.to_string(),
            status: "upload-complete".into(),
            runtime_artifact_files,
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
