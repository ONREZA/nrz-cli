use std::path::{Component, Path};

use anyhow::{Context, bail};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{CachedRuntime, RUNTIME_PROTOCOL_VERSION, process::RuntimeProcess};

const INSPECTOR_PATH: &str = ".nrz-inspector.mjs";
const INSPECTOR_SOURCE: &str = include_str!("../../assets/functions-inspector.mjs");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FunctionInspection {
    pub(crate) config: Value,
    pub(crate) handlers: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionFailure {
    message: String,
}

pub(crate) async fn inspect(
    runtime: &CachedRuntime,
    entrypoint: &str,
    content: &str,
) -> anyhow::Result<FunctionInspection> {
    let relative = Path::new(entrypoint);
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        || entrypoint.contains('\\')
        || entrypoint == INSPECTOR_PATH
    {
        bail!("invalid Functions snapshot path '{entrypoint}'");
    }
    let snapshot = tempfile::tempdir().context("failed to create Functions source snapshot")?;
    let entrypoint_path = snapshot.path().join(relative);
    tokio::fs::create_dir_all(
        entrypoint_path
            .parent()
            .context("snapshot file has no parent")?,
    )
    .await?;
    tokio::fs::write(&entrypoint_path, content).await?;
    tokio::fs::write(snapshot.path().join(INSPECTOR_PATH), INSPECTOR_SOURCE).await?;
    let entrypoint_url = url::Url::from_file_path(&entrypoint_path)
        .map_err(|_| anyhow::anyhow!("Functions snapshot has no absolute file URL"))?;
    let mut process = RuntimeProcess::start(runtime, snapshot.path(), INSPECTOR_PATH).await?;
    let invocation_id = uuid::Uuid::now_v7().to_string();
    let locals = process.invoke(json!({
        "protocolVersion": RUNTIME_PROTOCOL_VERSION,
        "invocationId": invocation_id,
        "invocationDepth": 0,
        "runtimeReleaseId": runtime.runtime_release_id,
        "poolKey": {
            "workspaceId": "local-inspection", "workspaceBindingGeneration": 1,
            "workspaceBindingEffectiveAtUs": 1, "deploymentId": "local-inspection",
            "functionRevisionId": "local-inspection", "sourceSnapshotHash": "local-inspection",
            "artifactVersion": "local-inspection", "logicalManifestSha256": "local-inspection",
            "functionName": "local-inspection", "rootPath": ".", "envHash": "local-inspection",
            "runtimeConfigHash": "local-inspection", "runtimeEnvelopeHash": "local-inspection"
        },
        "bundleRoot": snapshot.path(), "entrypoint": INSPECTOR_PATH,
        "env": {}, "event": {"type": "manual", "event": {"entrypointUrl": entrypoint_url.as_str()}},
        "limits": {"wallTimeoutMs": 5000, "maxLogEntries": 8, "maxLogBytes": 4096},
        "debug": {"bindings": {
            "fetch": {"mode": "disabled", "mocks": []},
            "kv": {"mode": "disabled", "entries": []}, "queue": {"mode": "disabled"}
        }}
    })).await?;
    process.shutdown().await?;
    if let Some(failure) = locals.get("inspectionError") {
        if locals.get("inspection").is_some() {
            bail!("Functions runtime returned conflicting inspection results");
        }
        let failure: InspectionFailure = serde_json::from_value(failure.clone())
            .context("Functions runtime returned an invalid inspection error")?;
        return Err(crate::output::coded_error(
            "INVALID_CONFIG",
            failure.message,
        ));
    }
    let inspection = serde_json::from_value::<FunctionInspection>(
        locals
            .get("inspection")
            .cloned()
            .context("Functions runtime omitted inspection")?,
    )
    .context("Functions runtime returned an invalid inspection")?;
    if !inspection.config.is_object() || inspection.handlers.is_empty() {
        bail!("Functions runtime returned an incomplete inspection");
    }
    Ok(inspection)
}

#[cfg(test)]
#[path = "inspection_tests.rs"]
mod tests;
