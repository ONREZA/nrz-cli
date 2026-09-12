use anyhow::bail;
use nrz_api::{MaterializeRequestBodyPurpose, ResolveRequestBodySelectionSource};

use super::{ExecutionContext, MaterializedExecutionContext, MaterializedSnapshot};

pub(crate) fn selection_source(value: &str) -> anyhow::Result<ResolveRequestBodySelectionSource> {
    Ok(match value {
        "EXPLICIT" => ResolveRequestBodySelectionSource::Explicit,
        "PROCESS" => ResolveRequestBodySelectionSource::Process,
        "REPOSITORY" => ResolveRequestBodySelectionSource::Repository,
        "DEPLOYMENT" => ResolveRequestBodySelectionSource::Deployment,
        _ => bail!("invalid execution context selection source: {value}"),
    })
}

pub(super) fn purpose(value: &str) -> anyhow::Result<MaterializeRequestBodyPurpose> {
    Ok(match value {
        "DEPLOY" => MaterializeRequestBodyPurpose::Deploy,
        "DEV" => MaterializeRequestBodyPurpose::Dev,
        "EXEC" => MaterializeRequestBodyPurpose::Exec,
        _ => bail!("invalid execution context purpose: {value}"),
    })
}

impl From<nrz_api::Resolve200ResponseContext> for ExecutionContext {
    fn from(value: nrz_api::Resolve200ResponseContext) -> Self {
        Self {
            workspace_id: value.workspace_id.to_string(),
            workspace_slug: value.workspace_slug,
            project_id: value.project_id.to_string(),
            project_name: value.project_name,
            environment_id: value.environment_id.to_string(),
            environment_name: value.environment_name,
            environment_type: value.environment_type.to_string(),
            source_ref: value.source_ref,
            selection_source: value.selection_source.to_string(),
        }
    }
}

impl From<nrz_api::Materialize200Response> for MaterializedExecutionContext {
    fn from(value: nrz_api::Materialize200Response) -> Self {
        Self {
            protocol_version: value.protocol_version,
            context: value.context.into(),
            variables: value.variables,
            secret_keys: value.secret_keys,
            snapshot: MaterializedSnapshot {
                fingerprint: value.snapshot.fingerprint,
                resolved_at: value
                    .snapshot
                    .resolved_at
                    .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
                source: value.snapshot.source.to_string(),
                deployment_id: value.snapshot.deployment_id.map(|id| id.to_string()),
            },
        }
    }
}
