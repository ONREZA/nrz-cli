// @generated vendored copy of platform crates/nrz-source-publisher/src/runtime.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use nrz_contract::{DependencyMaterializationManifestV1Wire, RuntimeArtifactGraphV2Wire};
use nrz_runtime_artifact::{
    VerifiedDependencyMaterializationManifest, VerifiedRuntimeArtifactGraph,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::SourcePublicationError;
use crate::publisher::{
    ObjectHeadVerification, ObjectUploadHeaders, ObjectUploadResult, PublicationEvent,
    PublicationObserver, retry_control_plane,
};

const RUNTIME_PREPARE_BUDGET: Duration = Duration::from_secs(30 * 60);
const RUNTIME_COMPLETE_BUDGET: Duration = Duration::from_secs(30 * 60);
const CONDITIONAL_CREATE_MODE: &str = "conditional-create";

pub struct RuntimeDependencyPublicationInput<'a> {
    pub layer_name: &'a str,
    pub mount_point: &'a str,
    pub image_path: &'a Path,
    pub manifest: &'a VerifiedDependencyMaterializationManifest,
}

pub struct RuntimeArtifactPublicationRequest<'a, T, O> {
    pub transport: &'a T,
    pub observer: &'a O,
    pub deployment_id: Uuid,
    pub workspace_id: Uuid,
    pub dependencies: &'a [RuntimeDependencyPublicationInput<'a>],
    pub expected_graph: &'a VerifiedRuntimeArtifactGraph,
}

#[derive(Debug, Clone)]
pub struct PublishedRuntimeArtifacts {
    pub runtime_artifact_graph_digest: String,
    pub runtime_artifact_graph: RuntimeArtifactGraphV2Wire,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePublicationPrepareRequest {
    pub operation_id: Uuid,
    pub deployment_id: Uuid,
    pub workspace_id: Uuid,
    pub dependencies: Vec<RuntimeDependencyPublication>,
}

pub type RuntimePublicationCompleteRequest = RuntimePublicationPrepareRequest;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDependencyPublication {
    pub layer_name: String,
    pub mount_point: String,
    pub manifest: DependencyMaterializationManifestV1Wire,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePublicationPrepareResponse {
    pub deployment_id: Uuid,
    pub runtime_artifact_graph_digest: String,
    pub runtime_artifact_graph: RuntimeArtifactGraphV2Wire,
    pub uploads: Vec<RuntimeDependencyUpload>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeDependencyUpload {
    pub materialization_id: String,
    pub bucket: String,
    pub object_key: String,
    pub presigned_put: RuntimePresignedPut,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePresignedPut {
    pub mode: String,
    pub url: String,
    pub content_length: i64,
    pub sha256: String,
    pub verify_head: Option<RuntimePresignedHead>,
    pub headers: Option<RuntimePresignedPutHeaders>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePresignedHead {
    pub url: String,
    pub content_length: i64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePresignedPutHeaders {
    #[serde(rename = "content-type")]
    pub content_type: String,
    #[serde(rename = "if-none-match")]
    pub if_none_match: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePublicationCompleteResponse {
    pub deployment_id: Uuid,
    pub runtime_artifact_graph_digest: String,
    pub runtime_artifact_graph: RuntimeArtifactGraphV2Wire,
}

#[derive(Debug, Clone)]
pub struct RuntimeFileUploadRequest {
    pub url: String,
    pub path: PathBuf,
    pub content_length: u64,
    pub sha256: String,
    pub headers: ObjectUploadHeaders,
    pub verify_head: Option<ObjectHeadVerification>,
}

pub trait RuntimeArtifactPublicationTransport: Send + Sync {
    fn prepare_runtime_artifacts(
        &self,
        deployment_id: Uuid,
        request: &RuntimePublicationPrepareRequest,
    ) -> impl Future<Output = Result<RuntimePublicationPrepareResponse, SourcePublicationError>> + Send;

    fn complete_runtime_artifacts(
        &self,
        deployment_id: Uuid,
        request: &RuntimePublicationCompleteRequest,
    ) -> impl Future<Output = Result<RuntimePublicationCompleteResponse, SourcePublicationError>> + Send;

    fn put_runtime_file(
        &self,
        request: RuntimeFileUploadRequest,
    ) -> impl Future<Output = Result<ObjectUploadResult, SourcePublicationError>> + Send;
}

pub async fn publish_runtime_artifacts<T, O>(
    request: RuntimeArtifactPublicationRequest<'_, T, O>,
) -> Result<PublishedRuntimeArtifacts, SourcePublicationError>
where
    T: RuntimeArtifactPublicationTransport,
    O: PublicationObserver,
{
    let body = publication_request(&request);
    request.observer.on_event(PublicationEvent::Preparing);
    let prepared = retry_control_plane(
        request.observer,
        "runtime-artifacts-prepare",
        RUNTIME_PREPARE_BUDGET,
        || async {
            request
                .transport
                .prepare_runtime_artifacts(request.deployment_id, &body)
                .await
        },
    )
    .await?;
    verify_response_graph(
        request.deployment_id,
        request.expected_graph,
        prepared.deployment_id,
        &prepared.runtime_artifact_graph_digest,
        &prepared.runtime_artifact_graph,
    )?;
    upload_dependencies(&request, prepared.uploads).await?;

    request
        .observer
        .on_event(PublicationEvent::CompletingUpload);
    let completed = retry_control_plane(
        request.observer,
        "runtime-artifacts-complete",
        RUNTIME_COMPLETE_BUDGET,
        || async {
            request
                .transport
                .complete_runtime_artifacts(request.deployment_id, &body)
                .await
        },
    )
    .await?;
    verify_response_graph(
        request.deployment_id,
        request.expected_graph,
        completed.deployment_id,
        &completed.runtime_artifact_graph_digest,
        &completed.runtime_artifact_graph,
    )?;
    request.observer.on_event(PublicationEvent::DurableVerified);
    Ok(PublishedRuntimeArtifacts {
        runtime_artifact_graph_digest: completed.runtime_artifact_graph_digest,
        runtime_artifact_graph: completed.runtime_artifact_graph,
    })
}

fn publication_request<T, O>(
    request: &RuntimeArtifactPublicationRequest<'_, T, O>,
) -> RuntimePublicationPrepareRequest {
    RuntimePublicationPrepareRequest {
        operation_id: Uuid::now_v7(),
        deployment_id: request.deployment_id,
        workspace_id: request.workspace_id,
        dependencies: request
            .dependencies
            .iter()
            .map(|dependency| RuntimeDependencyPublication {
                layer_name: dependency.layer_name.to_string(),
                mount_point: dependency.mount_point.to_string(),
                manifest: dependency.manifest.wire().clone(),
            })
            .collect(),
    }
}

async fn upload_dependencies<T, O>(
    request: &RuntimeArtifactPublicationRequest<'_, T, O>,
    uploads: Vec<RuntimeDependencyUpload>,
) -> Result<(), SourcePublicationError>
where
    T: RuntimeArtifactPublicationTransport,
    O: PublicationObserver,
{
    let dependencies = request
        .dependencies
        .iter()
        .map(|dependency| (dependency.manifest.materialization_id(), dependency))
        .collect::<HashMap<_, _>>();
    let mut seen = HashSet::with_capacity(uploads.len());
    for upload in uploads {
        if !seen.insert(upload.materialization_id.clone()) {
            return Err(invalid_response(
                "runtime prepare response contains a duplicate upload",
            ));
        }
        let dependency = dependencies
            .get(upload.materialization_id.as_str())
            .ok_or_else(|| {
                invalid_response("runtime prepare response contains an unknown upload")
            })?;
        let wire = dependency.manifest.wire();
        let expected_sha256 = wire
            .blob_descriptor
            .digest
            .as_str()
            .strip_prefix("sha256:")
            .expect("verified dependency digest has a sha256 prefix");
        let expected_size = u64::try_from(wire.blob_descriptor.size)
            .map_err(|_| invalid_response("dependency upload size is invalid"))?;
        let presigned = upload.presigned_put;
        if presigned.mode != CONDITIONAL_CREATE_MODE
            || presigned.sha256 != expected_sha256
            || u64::try_from(presigned.content_length).ok() != Some(expected_size)
        {
            return Err(invalid_response(
                "runtime upload plan conflicts with the verified dependency manifest",
            ));
        }
        let headers = presigned
            .headers
            .ok_or_else(|| invalid_response("runtime upload plan is missing signed headers"))?;
        if headers.if_none_match.as_deref() != Some("*") {
            return Err(invalid_response(
                "runtime upload plan is not an immutable conditional create",
            ));
        }
        let verify_head = presigned
            .verify_head
            .map(|head| {
                let content_length = u64::try_from(head.content_length)
                    .map_err(|_| invalid_response("runtime verifyHead size is invalid"))?;
                if content_length != expected_size || head.sha256 != expected_sha256 {
                    return Err(invalid_response(
                        "runtime verifyHead conflicts with the verified dependency manifest",
                    ));
                }
                Ok(ObjectHeadVerification {
                    url: head.url,
                    content_length,
                    sha256: head.sha256,
                })
            })
            .transpose()?;
        request.observer.on_event(PublicationEvent::Uploading);
        request
            .transport
            .put_runtime_file(RuntimeFileUploadRequest {
                url: presigned.url,
                path: dependency.image_path.to_path_buf(),
                content_length: expected_size,
                sha256: expected_sha256.to_string(),
                headers: ObjectUploadHeaders {
                    content_type: Some(headers.content_type),
                    if_none_match: headers.if_none_match,
                },
                verify_head,
            })
            .await?;
    }
    Ok(())
}

fn verify_response_graph(
    expected_deployment_id: Uuid,
    expected: &VerifiedRuntimeArtifactGraph,
    actual_deployment_id: Uuid,
    actual_digest: &str,
    actual: &RuntimeArtifactGraphV2Wire,
) -> Result<(), SourcePublicationError> {
    if actual_deployment_id != expected_deployment_id
        || actual_digest != expected.graph_digest()
        || actual != expected.wire()
    {
        return Err(invalid_response(
            "runtime publication response differs from the locally verified graph",
        ));
    }
    Ok(())
}

fn invalid_response(message: &str) -> SourcePublicationError {
    SourcePublicationError::InvalidResponse(message.to_string())
}
