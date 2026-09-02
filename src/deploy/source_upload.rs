use std::collections::HashSet;
use std::sync::Mutex;

use super::*;
use nrz_source_publisher::{
    PreparedSourceBundle, PublicationEvent, PublicationObserver, SourceBundleInput,
    SourcePublicationError, SourcePublicationRequest, publish_source_bundle,
};

pub(super) struct PrepareUploadAndCompleteRequest<'a> {
    pub(super) client: &'a ApiClient,
    pub(super) deployment_id: &'a str,
    pub(super) workspace_id: &'a str,
    pub(super) project_id: &'a str,
    pub(super) deployment_attempt_id: &'a str,
    pub(super) json: bool,
    pub(super) plan: &'a SourceBundlePlan,
    pub(super) runtime_artifact_files: &'a crate::artifact::RuntimeArtifactFileBreakdown,
}

pub(super) async fn prepare_upload_and_complete(
    request: PrepareUploadAndCompleteRequest<'_>,
) -> anyhow::Result<()> {
    let deployment_id = parse_uuid("deployment id", request.deployment_id)?;
    let workspace_id = parse_uuid("workspace id", request.workspace_id)?;
    let project_id = parse_uuid("project id", request.project_id)?;
    let deployment_attempt_id = parse_uuid("deployment attempt id", request.deployment_attempt_id)?;
    let bundle = PreparedSourceBundle::verify(
        workspace_id,
        SourceBundleInput {
            path: request.plan.source_path().to_path_buf(),
            source_sha256: request.plan.source_sha256.clone(),
            source_size_bytes: request.plan.source_size_bytes,
            logical_manifest_sha256: request.plan.logical_manifest_sha256.clone(),
        },
    )
    .await
    .map_err(|error| map_publication_error(error, request.json, request.runtime_artifact_files))?;
    let transport = request.client.source_publication_transport();
    let observer = CliPublicationObserver::new(request.json);

    publish_source_bundle(SourcePublicationRequest {
        transport: &transport,
        observer: &observer,
        deployment_id,
        workspace_id,
        project_id,
        deployment_attempt_id,
        bundle: &bundle,
    })
    .await
    .map_err(|error| map_publication_error(error, request.json, request.runtime_artifact_files))?;
    Ok(())
}

fn parse_uuid(label: &str, value: &str) -> anyhow::Result<Uuid> {
    Uuid::parse_str(value).with_context(|| format!("{label} is not a valid UUID: {value}"))
}

struct CliPublicationObserver {
    json: bool,
    reported_waits: Mutex<HashSet<&'static str>>,
}

impl CliPublicationObserver {
    fn new(json: bool) -> Self {
        Self {
            json,
            reported_waits: Mutex::new(HashSet::new()),
        }
    }

    fn status(&self, message: impl std::fmt::Display) {
        output::status(self.json, "~", message, output::Phase::Deploy);
    }
}

impl PublicationObserver for CliPublicationObserver {
    fn on_event(&self, event: PublicationEvent) {
        match event {
            PublicationEvent::Preparing => self.status("Preparing SOURCE_BUNDLE_V1 upload..."),
            PublicationEvent::Uploading => self.status("Uploading SOURCE_BUNDLE_V1 source..."),
            PublicationEvent::RecoveringConditionalConflict => {
                self.status("Recovering SOURCE_BUNDLE_V1 source upload...");
            }
            PublicationEvent::CompletingMultipart => {
                self.status("Completing SOURCE_BUNDLE_V1 multipart upload...");
            }
            PublicationEvent::CompletingUpload => {
                self.status("Completing SOURCE_BUNDLE_V1 publication...");
            }
            PublicationEvent::AwaitingDurableReadback => {
                self.status("Waiting for durable runtime artifact graph...");
            }
            PublicationEvent::DurableVerified => output::success(
                self.json,
                "Durable runtime artifact graph verified",
                output::Phase::Deploy,
            ),
            PublicationEvent::Waiting { operation } => {
                let Ok(mut reported) = self.reported_waits.lock() else {
                    return;
                };
                if reported.insert(operation) {
                    self.status(format!("Waiting for {operation}..."));
                }
            }
        }
    }
}

pub(super) fn map_publication_error(
    error: SourcePublicationError,
    json: bool,
    runtime_artifact_files: &crate::artifact::RuntimeArtifactFileBreakdown,
) -> anyhow::Error {
    if let Some(api_error) = error.structured_control_plane() {
        let is_file_limit = api_error.code == "LIMIT_EXCEEDED"
            && api_error
                .details
                .as_ref()
                .and_then(|details| details.get("limitType"))
                .and_then(serde_json::Value::as_str)
                == Some("maxDeploymentFiles");
        let details = is_file_limit.then(|| {
            let mut details = api_error
                .details
                .clone()
                .unwrap_or_else(|| serde_json::json!({}));
            if let Some(object) = details.as_object_mut() {
                object.insert(
                    "runtimeArtifactFiles".to_string(),
                    serde_json::json!(runtime_artifact_files),
                );
            }
            details
        });
        let message = if is_file_limit {
            runtime_artifact_file_limit_message(&api_error.message, runtime_artifact_files)
        } else {
            api_error.message.clone()
        };

        if json
            && matches!(
                api_error.code.as_str(),
                "LIMIT_EXCEEDED" | "SUBSCRIPTION_REQUIRED"
            )
        {
            return output::report_terminal_error(
                "deploy",
                &message,
                &api_error.code,
                details.as_ref().or(api_error.details.as_ref()),
            );
        }
        if is_file_limit {
            let mut mapped =
                crate::errors::CliError::new(&api_error.code, message).phase(output::Phase::Deploy);
            if let Some(details) = details {
                mapped = mapped.details(details);
            }
            return mapped.into_anyhow();
        }
    }
    anyhow::Error::new(error).context("failed to publish verified source bundle")
}

fn runtime_artifact_file_limit_message(
    message: &str,
    breakdown: &crate::artifact::RuntimeArtifactFileBreakdown,
) -> String {
    let categories = breakdown
        .categories()
        .map(|(label, count)| format!("{label} {count}"))
        .collect::<Vec<_>>();
    let mut guidance = format!(
        "{message}\n\nRuntime artifact file breakdown: {}; total {}.",
        categories.join(", "),
        breakdown.total
    );
    if breakdown.includes_installed_dependencies() {
        guidance.push_str(
            "\nFor PROCESS/SSR deployments, installed Node.js dependencies are part of the runtime artifact even when Output Directory is build or dist.",
        );
        guidance.push_str(
            "\nReduce the runtime closure by pruning development dependencies after the build (for example, npm prune --omit=dev) or by producing a self-contained server bundle. Use a static adapter only when SSR is not required.",
        );
    } else {
        guidance.push_str(
            "\nReduce generated deployment files or produce a more compact, self-contained artifact before retrying.",
        );
    }
    guidance
}
