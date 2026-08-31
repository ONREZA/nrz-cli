use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

use crate::api::{ApiClient, path_segment};
use crate::auth;
use crate::cli::execution_context::{ContextArgs, ContextCommand};
use crate::output;
use nrz::config::{self, ProjectConfig};

pub const EXECUTION_CONTEXT_PROTOCOL: &str = "execution-context-v1";
pub const RUNNER_CONTEXT_PROTOCOL: &str = "runner-context-v2";
const SAVED_CONTEXT_VERSION: u8 = 1;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionContext {
    pub workspace_id: String,
    pub workspace_slug: String,
    pub project_id: String,
    pub project_name: String,
    pub environment_id: String,
    pub environment_name: String,
    pub environment_type: String,
    pub source_ref: Option<String>,
    pub selection_source: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveResponse {
    protocol_version: String,
    context: ExecutionContext,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterializedSnapshot {
    pub fingerprint: String,
    pub resolved_at: String,
    pub source: String,
    pub deployment_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterializedExecutionContext {
    pub protocol_version: String,
    pub context: ExecutionContext,
    pub variables: HashMap<String, String>,
    pub secret_keys: Vec<String>,
    pub snapshot: MaterializedSnapshot,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SavedExecutionContext {
    version: u8,
    environment_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolveBody<'a> {
    environment: &'a str,
    source_ref: Option<&'a str>,
    selection_source: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MaterializeBody<'a> {
    environment_id: &'a str,
    source_ref: Option<&'a str>,
    purpose: &'a str,
    selection_source: &'a str,
}

#[derive(Serialize)]
struct DeploymentMaterializeBody<'a> {
    purpose: &'a str,
}

pub async fn run(
    args: ContextArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    match args.command {
        ContextCommand::Show => {
            let saved = load_saved(Path::new(&args.dir))?
                .context("no saved environment context; run `nrz context use <environment>`")?;
            let token = auth::resolve_token(token, workspace)?;
            let client = ApiClient::authenticated(&token)?;
            let project_id = config::resolve_project_id(args.project_id.as_deref(), config)?;
            let resolved =
                resolve_repository_context(&client, &project_id, &saved.environment_id, None)
                    .await?;
            if json {
                output::json_output(&resolved);
            } else {
                report_context_human(&resolved);
            }
            Ok(())
        }
        ContextCommand::Clear => {
            let path = context_path(Path::new(&args.dir));
            if path.exists() {
                std::fs::remove_file(&path)
                    .with_context(|| format!("failed to remove {}", path.display()))?;
            }
            output::success(json, "Execution context cleared", output::Phase::Env);
            Ok(())
        }
        ContextCommand::Use { environment } => {
            let token = auth::resolve_token(token, workspace)?;
            let client = ApiClient::authenticated(&token)?;
            let project_id = config::resolve_project_id(args.project_id.as_deref(), config)?;
            let resolved = resolve(&client, &project_id, &environment, None, "EXPLICIT").await?;
            save(Path::new(&args.dir), &resolved)?;
            if json {
                output::json_output(&resolved);
            } else {
                report_context_human(&resolved);
                output::success(false, "Execution context saved", output::Phase::Env);
            }
            Ok(())
        }
    }
}

pub async fn resolve_for_mutation(
    client: &ApiClient,
    project_id: &str,
    project_dir: &Path,
    explicit: Option<&str>,
    source_ref: Option<&str>,
) -> anyhow::Result<ExecutionContext> {
    let saved = load_saved(project_dir)?;
    let process_environment = std::env::var("NRZ_ENVIRONMENT").ok();
    let selection = select_environment(explicit, process_environment.as_deref(), saved.as_ref())?;

    resolve_selection(client, project_id, source_ref, selection).await
}

pub async fn resolve_optional(
    client: &ApiClient,
    project_id: &str,
    project_dir: &Path,
    explicit: Option<&str>,
    source_ref: Option<&str>,
) -> anyhow::Result<Option<ExecutionContext>> {
    let saved = load_saved(project_dir)?;
    let process_environment = std::env::var("NRZ_ENVIRONMENT").ok();
    let selection =
        match select_environment(explicit, process_environment.as_deref(), saved.as_ref()) {
            Ok(selection) => selection,
            Err(_) => return Ok(None),
        };
    resolve_selection(client, project_id, source_ref, selection)
        .await
        .map(Some)
}

async fn resolve_selection(
    client: &ApiClient,
    project_id: &str,
    source_ref: Option<&str>,
    selection: EnvironmentSelection,
) -> anyhow::Result<ExecutionContext> {
    if selection.source == "REPOSITORY" {
        resolve_repository_context(client, project_id, &selection.selector, source_ref).await
    } else {
        resolve(
            client,
            project_id,
            &selection.selector,
            source_ref,
            selection.source,
        )
        .await
    }
}

async fn resolve_repository_context(
    client: &ApiClient,
    project_id: &str,
    environment_id: &str,
    source_ref: Option<&str>,
) -> anyhow::Result<ExecutionContext> {
    resolve(client, project_id, environment_id, source_ref, "REPOSITORY")
        .await
        .map_err(map_repository_context_error)
}

fn map_repository_context_error(error: anyhow::Error) -> anyhow::Error {
    let stale = error
        .downcast_ref::<crate::api::StructuredApiError>()
        .is_some_and(|api_error| api_error.code == "ENVIRONMENT_NOT_FOUND");
    if stale {
        return output::coded_error(
            "ENVIRONMENT_CONTEXT_STALE",
            "saved environment context is stale or belongs to another project; run `nrz context use <environment>`",
        );
    }
    error
}

pub async fn resolve(
    client: &ApiClient,
    project_id: &str,
    selector: &str,
    source_ref: Option<&str>,
    selection_source: &str,
) -> anyhow::Result<ExecutionContext> {
    let response: ResolveResponse = client
        .post(
            &format!(
                "/v1/projects/{}/execution-context/resolve",
                path_segment(project_id)
            ),
            &ResolveBody {
                environment: selector,
                source_ref,
                selection_source,
            },
        )
        .await
        .context("failed to resolve execution context")?;
    require_protocol(&response.protocol_version)?;
    Ok(response.context)
}

pub async fn materialize_desired(
    client: &ApiClient,
    context: &ExecutionContext,
    source_ref: Option<&str>,
    purpose: &str,
) -> anyhow::Result<MaterializedExecutionContext> {
    let response: MaterializedExecutionContext = client
        .post(
            &format!(
                "/v1/projects/{}/execution-context/materialize",
                path_segment(&context.project_id)
            ),
            &MaterializeBody {
                environment_id: &context.environment_id,
                source_ref,
                purpose,
                selection_source: &context.selection_source,
            },
        )
        .await
        .context("failed to materialize environment configuration")?;
    require_protocol(&response.protocol_version)?;
    validate_snapshot_binding(&response, "DESIRED_STATE", None)?;
    if response.context.project_id != context.project_id
        || response.context.environment_id != context.environment_id
    {
        bail!("ENV_SNAPSHOT_SCOPE_MISMATCH: materialized context changed during resolution");
    }
    Ok(response)
}

pub async fn materialize_deployment(
    client: &ApiClient,
    deployment_id: &str,
    purpose: &str,
) -> anyhow::Result<MaterializedExecutionContext> {
    let response: MaterializedExecutionContext = client
        .post(
            &format!(
                "/v1/deployments/{}/execution-context/materialize",
                path_segment(deployment_id)
            ),
            &DeploymentMaterializeBody { purpose },
        )
        .await
        .context("failed to materialize deployment environment snapshot")?;
    require_protocol(&response.protocol_version)?;
    validate_snapshot_binding(&response, "DEPLOYMENT", Some(deployment_id))?;
    Ok(response)
}

fn validate_snapshot_binding(
    materialized: &MaterializedExecutionContext,
    expected_source: &str,
    expected_deployment_id: Option<&str>,
) -> anyhow::Result<()> {
    let fingerprint = materialized.snapshot.fingerprint.as_str();
    let valid_fingerprint = fingerprint.strip_prefix("v1:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    });
    if !valid_fingerprint || materialized.snapshot.resolved_at.trim().is_empty() {
        bail!("ENV_SNAPSHOT_INVALID: materialized snapshot metadata is invalid");
    }
    if materialized.snapshot.source != expected_source
        || materialized.snapshot.deployment_id.as_deref() != expected_deployment_id
    {
        bail!("ENV_SNAPSHOT_SCOPE_MISMATCH: materialized snapshot binding is invalid");
    }
    Ok(())
}

pub fn execution_environment(
    materialized: &MaterializedExecutionContext,
) -> HashMap<String, String> {
    let mut variables = materialized.variables.clone();
    variables.retain(|key, _| !is_private_cli_environment_key(key));
    variables
}

pub fn secret_values(materialized: &MaterializedExecutionContext) -> Vec<String> {
    let secret_keys = materialized
        .secret_keys
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    materialized
        .variables
        .iter()
        .filter(|(key, value)| secret_keys.contains(key.as_str()) && !value.is_empty())
        .map(|(_, value)| value.clone())
        .collect()
}

fn is_private_cli_environment_key(key: &str) -> bool {
    key.starts_with("NRZ_")
}

pub fn private_cli_environment_keys() -> Vec<std::ffi::OsString> {
    let mut keys = [
        "NRZ_TOKEN",
        "NRZ_TOKEN_FILE",
        "NRZ_API_URL",
        "NRZ_WORKSPACE",
        "NRZ_RUNNER",
        "NRZ_DEPLOYMENT_ID",
        "NRZ_BUILD_LOG_SESSION_ID",
        "NRZ_BUILD_LOG_PRODUCER_ID",
        "NRZ_BUILD_LOG_ATTEMPT",
    ]
    .into_iter()
    .map(std::ffi::OsString::from)
    .collect::<std::collections::BTreeSet<_>>();
    keys.extend(
        std::env::vars_os()
            .map(|(key, _)| key)
            .filter(|key| is_private_cli_environment_key(&key.to_string_lossy())),
    );
    keys.into_iter().collect()
}

pub fn warn_local_dotenv_drift(project_dir: &Path, json: bool) -> anyhow::Result<()> {
    let mut filenames = Vec::new();
    for entry in std::fs::read_dir(project_dir)
        .with_context(|| format!("failed to inspect {}", project_dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(".env") || name.ends_with(".example") {
            continue;
        }
        let ignored = if name == ".env.local" {
            true
        } else {
            let mut command = std::process::Command::new("git");
            command
                .args(["check-ignore", "-q", "--", &name])
                .current_dir(project_dir);
            for key in private_cli_environment_keys() {
                command.env_remove(key);
            }
            command.status().is_ok_and(|status| status.success())
        };
        if ignored {
            filenames.push(name);
        }
    }
    filenames.sort();
    if !filenames.is_empty() {
        output::warn(
            json,
            format!(
                "Local dotenv files may override platform values when read directly by tooling: {}",
                filenames.join(", ")
            ),
            output::Phase::Env,
        );
    }
    Ok(())
}

fn require_protocol(protocol: &str) -> anyhow::Result<()> {
    if protocol == EXECUTION_CONTEXT_PROTOCOL {
        return Ok(());
    }
    bail!(
        "unsupported execution context protocol {protocol}; expected {EXECUTION_CONTEXT_PROTOCOL}"
    )
}

fn context_path(project_dir: &Path) -> PathBuf {
    project_dir.join(".onreza").join("environment.json")
}

#[derive(Debug, PartialEq, Eq)]
struct EnvironmentSelection {
    selector: String,
    source: &'static str,
}

fn select_environment(
    explicit: Option<&str>,
    process_environment: Option<&str>,
    saved: Option<&SavedExecutionContext>,
) -> anyhow::Result<EnvironmentSelection> {
    if let Some(environment) = explicit.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(EnvironmentSelection {
            selector: environment.to_owned(),
            source: "EXPLICIT",
        });
    }
    if let Some(environment) = process_environment
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(EnvironmentSelection {
            selector: environment.to_owned(),
            source: "PROCESS",
        });
    }
    if let Some(saved) = saved {
        return Ok(EnvironmentSelection {
            selector: saved.environment_id.clone(),
            source: "REPOSITORY",
        });
    }
    bail!(
        "CONTEXT_NOT_LINKED: select an environment with --environment, NRZ_ENVIRONMENT, or `nrz context use`"
    )
}

fn load_saved(project_dir: &Path) -> anyhow::Result<Option<SavedExecutionContext>> {
    let path = context_path(project_dir);
    if !path.exists() {
        return Ok(None);
    }
    let saved: SavedExecutionContext = serde_json::from_slice(
        &std::fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", path.display()))?;
    if saved.version != SAVED_CONTEXT_VERSION {
        bail!(
            "unsupported saved execution context version {}",
            saved.version
        );
    }
    Ok(Some(saved))
}

fn save(project_dir: &Path, context: &ExecutionContext) -> anyhow::Result<()> {
    let path = context_path(project_dir);
    let parent = path
        .parent()
        .context("execution context path has no parent")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;
    crate::init::add_to_gitignore(project_dir);
    let saved = SavedExecutionContext {
        version: SAVED_CONTEXT_VERSION,
        environment_id: context.environment_id.clone(),
    };
    std::fs::write(&path, serde_json::to_vec_pretty(&saved)?)
        .with_context(|| format!("failed to write {}", path.display()))
}

fn report_context_human(context: &ExecutionContext) {
    eprintln!(
        "  Workspace:   {} ({})",
        output::terminal_line(&context.workspace_slug),
        output::terminal_line(&context.workspace_id)
    );
    eprintln!(
        "  Project:     {} ({})",
        output::terminal_line(&context.project_name),
        output::terminal_line(&context.project_id)
    );
    eprintln!(
        "  Environment: {} ({}, {})",
        output::terminal_line(&context.environment_name),
        output::terminal_line(&context.environment_id),
        output::terminal_line(&context.environment_type)
    );
    eprintln!(
        "  Source:      {}",
        output::terminal_line(&context.selection_source)
    );
}

#[cfg(test)]
#[path = "execution_context_tests.rs"]
mod tests;
