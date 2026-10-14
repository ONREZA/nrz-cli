use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::cli::deployment_set::DeploymentSetArgs;
use crate::{auth, execution_context, output};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PendingCommand {
    project_id: Uuid,
    environment_id: Uuid,
    environment_selector: String,
    local: BTreeMap<Uuid, PathBuf>,
    all: bool,
    selected_services: Vec<Uuid>,
    request: Value,
}

pub(crate) async fn run(
    args: DeploymentSetArgs,
    json_mode: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<()> {
    let resolved_token = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&resolved_token)?;
    let local = local_checkouts(&args.local)?;
    let key = args.idempotency_key.unwrap_or_else(Uuid::now_v7);
    ensure!(
        key.get_version_num() == 7,
        "--idempotency-key requires UUIDv7"
    );
    let receipt_root = std::env::current_dir()?.join(".onreza/deployment-sets");
    std::fs::create_dir_all(&receipt_root)?;
    let receipt = receipt_root.join(format!("{key}.json"));
    let pending = prepare_command(&args, &client, key, &receipt, local).await?;
    output::status(
        json_mode,
        "~",
        format!("Admitting Service set (retry key {key})"),
        output::Phase::Deploy,
    );
    let admitted = if pending.local.is_empty() {
        client
            .create_deployment_set(pending.request.clone())
            .await
            .with_context(|| {
                format!(
                    "retry --idempotency-key {key}; command receipt {} is preserved",
                    receipt.display()
                )
            })?
    } else {
        client
            .admit_deployment_set(args.project_id, pending.request.clone())
            .await
            .with_context(|| {
                format!(
                    "retry --idempotency-key {key}; command receipt {} is preserved",
                    receipt.display()
                )
            })?
    };
    let result = async {
        let jobs = admitted["deployments"]
            .as_array()
            .context("selected admission omitted jobs")?;
        ensure!(
            !jobs.is_empty() && admitted["operationId"].as_str().is_some(),
            "selected admission omitted its operation"
        );
        let mut admitted_services = jobs
            .iter()
            .map(|job| {
                job["serviceId"]
                    .as_str()
                    .context("job omitted Service identity")?
                    .parse::<Uuid>()
                    .context("invalid admitted Service identity")
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        admitted_services.sort();
        ensure!(
            admitted_services.windows(2).all(|pair| pair[0] != pair[1]),
            "admission repeated a Service"
        );
        ensure!(
            pending.all || admitted_services == pending.selected_services,
            "admission changed the selected Service set"
        );
        ensure!(
            pending
                .local
                .keys()
                .all(|id| admitted_services.contains(id)),
            "admission omitted a mapped local Source"
        );
        for job in jobs {
            let service_id = job["serviceId"]
                .as_str()
                .context("job omitted Service identity")?
                .parse::<Uuid>()?;
            if let Some(path) = pending.local.get(&service_id) {
                if job["status"] != "BUILDING" {
                    continue;
                }
                let deployment_id = job["id"]
                    .as_str()
                    .context("job omitted deployment identity")?
                    .parse::<Uuid>()?;
                let config = nrz::config::ProjectConfig::default();
                let deploy = crate::cli::DeployArgs::try_parse_from([
                    "deploy",
                    path.to_str().context("checkout path is not UTF-8")?,
                ])?;
                super::workflow::publish_admitted(
                    deploy,
                    json_mode,
                    Some(&resolved_token),
                    workspace,
                    &config,
                    deployment_id,
                )
                .await
                .with_context(|| format!("Service {service_id} publication failed"))?;
            }
        }
        let anchor = jobs[0]["id"]
            .as_str()
            .context("selected admission omitted anchor identity")?;
        let state = super::activation::wait_for_activation(
            super::activation::ActivationWait {
                deployment_id: anchor,
                timeout: Duration::from_secs(args.wait_timeout.into()),
            },
            || async { client.deployment_status(anchor).await?.try_into() },
            |_| {},
        )
        .await?;
        if json_mode {
            output::json_output(
                &json!({ "operation": admitted, "status": state.status, "receipt": receipt }),
            );
        } else {
            output::success(
                false,
                format!("Service set {} is active", admitted["operationId"]),
                output::Phase::Deploy,
            );
        }
        Ok(())
    }
    .await;
    result.map_err(|error| with_set_recovery(error, &admitted["operationId"], key, &receipt))
}

fn with_set_recovery(
    mut error: anyhow::Error,
    operation_id: &Value,
    key: Uuid,
    receipt: &Path,
) -> anyhow::Error {
    if error.downcast_ref::<crate::errors::CliError>().is_none() {
        let (code, details) =
            if let Some(api) = error.downcast_ref::<crate::api::StructuredApiError>() {
                (api.code.clone(), api.details.clone())
            } else if let Some(api) = error
                .downcast_ref::<nrz_source_publisher::SourcePublicationError>()
                .and_then(nrz_source_publisher::SourcePublicationError::structured_control_plane)
            {
                (api.code.clone(), api.details.clone())
            } else if let Some(coded) = error.downcast_ref::<output::CodedError>() {
                (coded.code.clone(), None)
            } else {
                ("DEPLOY_SET_FAILED".into(), None)
            };
        let mut diagnostic =
            crate::errors::CliError::new(code, format!("{error:#}")).phase(output::Phase::Deploy);
        diagnostic.details = details;
        error = diagnostic.with_source(error).into_anyhow();
    }
    let context = format!(
        "Service set {operation_id} was admitted; retry --idempotency-key {key}; command receipt {} is preserved",
        receipt.display()
    );
    if let Some(diagnostic) = error.downcast_mut::<crate::errors::CliError>() {
        diagnostic.message = format!("{context}: {}", diagnostic.message);
        let details = diagnostic.details.get_or_insert_with(|| json!({}));
        if !details.is_object() {
            *details = json!({"originalDetails": details.take()});
        }
        if let Some(details) = details.as_object_mut() {
            details.insert("operationId".into(), operation_id.clone());
            details.insert("idempotencyKey".into(), json!(key));
            details.insert("receipt".into(), json!(receipt));
        }
        error
    } else {
        unreachable!("selected-set errors carry a terminal diagnostic")
    }
}

async fn prepare_command(
    args: &DeploymentSetArgs,
    client: &ApiClient,
    key: Uuid,
    receipt: &Path,
    local: BTreeMap<Uuid, PathBuf>,
) -> anyhow::Result<PendingCommand> {
    if receipt.exists() {
        let pending: PendingCommand = serde_json::from_slice(&std::fs::read(receipt)?)?;
        ensure!(
            pending.project_id == args.project_id
                && pending.environment_selector == args.environment
                && pending.local == local,
            "stored admission belongs to another Product, Environment, or checkout mapping"
        );
        let mut selected = args.service_ids.clone();
        selected.sort();
        ensure!(
            pending.all == args.all && (args.all || pending.selected_services == selected),
            "stored admission selects a different Service set"
        );
        ensure!(
            pending.request["idempotencyKey"].as_str() == Some(key.to_string().as_str()),
            "stored admission has another retry key"
        );
        if let Some(generation) = &args.expected_generation {
            ensure!(
                pending.request["expectedGeneration"].as_str() == Some(generation.as_str()),
                "stored admission has another expected generation"
            );
        }
        return Ok(pending);
    }
    let services = client.list_services(args.project_id).await?;
    let mut selected = if args.all {
        services
            .services
            .iter()
            .filter(|item| item.deletion_requested_at.is_none())
            .map(|item| item.service_id)
            .collect::<Vec<_>>()
    } else {
        args.service_ids.clone()
    };
    selected.sort();
    ensure!(!selected.is_empty(), "select at least one active Service");
    ensure!(
        selected.windows(2).all(|pair| pair[0] != pair[1]),
        "select each Service once"
    );
    ensure!(
        local.keys().all(|id| selected.contains(id)),
        "each checkout must belong to the selected Service set"
    );
    for id in &selected {
        let service = services
            .services
            .iter()
            .find(|item| item.service_id == *id && item.deletion_requested_at.is_none())
            .context("selected Service is unavailable")?;
        ensure!(
            service.source_type == "GIT" || local.contains_key(id),
            "local Source for Service {id} requires --local {id}=PATH"
        );
    }
    let context = execution_context::resolve_for_service_mutation(
        client,
        &args.project_id.to_string(),
        Path::new("."),
        Some(&args.environment),
        None,
        Some(&selected[0].to_string()),
    )
    .await?;
    let environment_id = context.environment_id.parse::<Uuid>()?;
    let pending = {
        let serving = client.environment_serving(&context.environment_id).await?;
        let generation = args
            .expected_generation
            .clone()
            .unwrap_or(serving.desired.generation);
        let mut entries = Vec::new();
        for id in &selected {
            if let Some(path) = local.get(id) {
                let context = execution_context::resolve_for_service_mutation(
                    client,
                    &args.project_id.to_string(),
                    path,
                    Some(&args.environment),
                    None,
                    Some(&id.to_string()),
                )
                .await?;
                let branch = git_revision(path, &["rev-parse", "--abbrev-ref", "HEAD"])
                    .or(context.source_ref.clone())
                    .unwrap_or_else(|| "local".into());
                let commit = git_revision(path, &["rev-parse", "HEAD"])
                    .unwrap_or_else(|| Uuid::now_v7().simple().to_string());
                entries.push(json!({ "mode": "LOCAL", "serviceId": id, "branch": branch, "commitSha": commit, "expectedContext": {
                    "workspaceId": context.workspace_id, "productId": context.product_id, "productEnvironmentId": context.product_environment_id,
                    "workspaceEnvironmentId": context.workspace_environment_id, "workspaceBindingGeneration": context.workspace_binding_generation,
                    "serviceId": context.service_id, "sourceId": context.source_id,
                }}));
            } else {
                entries.push(json!({ "mode": "REMOTE", "serviceId": id }));
            }
        }
        let request = if local.is_empty() {
            json!({ "projectId": args.project_id, "environmentId": environment_id, "expectedGeneration": generation, "idempotencyKey": key,
                "selection": if args.all { json!({ "kind": "ALL" }) } else { json!({ "kind": "SERVICES", "services": selected.iter().map(|id| json!({ "serviceId": id })).collect::<Vec<_>>() }) }, "envVars": {} })
        } else {
            json!({ "protocolVersion": execution_context::EXECUTION_CONTEXT_PROTOCOL, "environmentId": environment_id, "expectedGeneration": generation,
                "idempotencyKey": key, "selection": { "kind": if args.all { "ALL" } else { "SERVICES" } }, "services": entries })
        };
        let pending = PendingCommand {
            project_id: args.project_id,
            environment_id,
            environment_selector: args.environment.clone(),
            local,
            all: args.all,
            selected_services: selected.clone(),
            request,
        };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(receipt)?;
        serde_json::to_writer(&mut file, &pending)?;
        file.sync_all()?;
        pending
    };
    Ok(pending)
}

#[cfg(test)]
#[path = "selected_set_tests.rs"]
mod tests;

fn local_checkouts(values: &[String]) -> anyhow::Result<BTreeMap<Uuid, PathBuf>> {
    let mut mappings = BTreeMap::new();
    for value in values {
        let (id, path) = value
            .split_once('=')
            .context("--local requires SERVICE_UUID=PATH")?;
        let id = id.parse::<Uuid>().context("invalid local Service ID")?;
        let path = std::fs::canonicalize(path).context("local checkout is unavailable")?;
        ensure!(path.is_dir(), "local checkout must be a directory");
        ensure!(
            mappings.insert(id, path).is_none(),
            "each local Service may be mapped once"
        );
    }
    Ok(mappings)
}

fn git_revision(root: &Path, arguments: &[&str]) -> Option<String> {
    let mut command = Command::new("git");
    super::ignored_build::remove_git_repository_environment(&mut command);
    let result = command.args(arguments).current_dir(root).output().ok()?;
    result
        .status
        .success()
        .then(|| String::from_utf8_lossy(&result.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
}
