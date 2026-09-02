// @generated vendored copy of platform crates/nrz-source-publisher/src/publisher.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

use std::future::Future;
use std::num::NonZeroU64;
use std::time::{Duration, Instant};

use bytes::Bytes;
use nrz_contract::cli_api::{
    OnrezaCliApiV1MultipartCompleteRequestPartsItem as CliMultipartCompletePart,
    OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummary as CliLogicalManifestSummary,
    OnrezaCliApiV1PrepareUploadRequestMultipart as CliPrepareMultipart,
    OnrezaCliApiV1PrepareUploadRequestMultipartPartsItem as CliPrepareMultipartPart,
    OnrezaCliApiV1PrepareUploadRequestSourceUploadRecovery as CliPrepareUploadSourceUploadRecovery,
};
use nrz_contract::{
    CliMultipartCompleteRequest, CliMultipartCompleteResponse, CliPrepareUploadRequest,
    CliPrepareUploadRequiredComplete, CliPrepareUploadResponse, CliUploadCompleteRequest,
    CliUploadCompleteResponse, CliUploadFailedRequest, CliUploadFailedResponse,
    RuntimeArtifactGraphV2Wire,
};
use nrz_runtime_artifact::{
    VerifiedRuntimeArtifactGraph, compute_source_logical_artifact_id, verify_runtime_artifact_graph,
};
use nrz_source_bundle::summarize_logical_manifest;
use uuid::Uuid;

use crate::bundle::{CLI_PROTOCOL_VERSION, SOURCE_BUNDLE_FORMAT};
use crate::{PreparedSourceBundle, SourcePublicationError, StructuredControlPlaneError};

const PREPARE_BUDGET: Duration = Duration::from_secs(10 * 60);
const COMPLETION_BUDGET: Duration = Duration::from_secs(30 * 60);
const FAILURE_REPORT_BUDGET: Duration = Duration::from_secs(5 * 60);
const DURABLE_READBACK_BUDGET: Duration = Duration::from_secs(30 * 60);
const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(500);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(5);
const SOURCE_UPLOAD_PUT_FAILED: &str = "SOURCE_UPLOAD_PUT_FAILED";
const CONDITIONAL_RECOVERY_REASON: &str = "conditional-precondition-failed";
const MAX_UPLOAD_FAILURE_LOG_LENGTH: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationEvent {
    Preparing,
    Uploading,
    RecoveringConditionalConflict,
    CompletingMultipart,
    CompletingUpload,
    AwaitingDurableReadback,
    DurableVerified,
    Waiting { operation: &'static str },
}

pub trait PublicationObserver: Send + Sync {
    fn on_event(&self, event: PublicationEvent);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopPublicationObserver;

impl PublicationObserver for NoopPublicationObserver {
    fn on_event(&self, _event: PublicationEvent) {}
}

#[derive(Debug, Clone)]
pub struct ObjectUploadHeaders {
    pub content_type: Option<String>,
    pub if_none_match: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ObjectHeadVerification {
    pub url: String,
    pub content_length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct ObjectUploadRequest {
    pub url: String,
    pub bytes: Bytes,
    pub sha256: String,
    pub headers: ObjectUploadHeaders,
    pub verify_head: Option<ObjectHeadVerification>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectUploadResult {
    pub e_tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeploymentPublicationStatus {
    pub status: String,
    pub runtime_artifact_graph_digest: Option<String>,
    pub runtime_artifact_graph: Option<RuntimeArtifactGraphV2Wire>,
    pub error: Option<String>,
    pub error_code: Option<String>,
}

pub trait SourcePublicationTransport: Send + Sync {
    fn prepare_upload(
        &self,
        deployment_id: Uuid,
        request: &CliPrepareUploadRequest,
    ) -> impl Future<Output = Result<CliPrepareUploadResponse, SourcePublicationError>> + Send;

    fn complete_multipart(
        &self,
        deployment_id: Uuid,
        request: &CliMultipartCompleteRequest,
    ) -> impl Future<Output = Result<CliMultipartCompleteResponse, SourcePublicationError>> + Send;

    fn complete_upload(
        &self,
        deployment_id: Uuid,
        request: &CliUploadCompleteRequest,
    ) -> impl Future<Output = Result<CliUploadCompleteResponse, SourcePublicationError>> + Send;

    fn report_upload_failed(
        &self,
        deployment_id: Uuid,
        request: &CliUploadFailedRequest,
    ) -> impl Future<Output = Result<CliUploadFailedResponse, SourcePublicationError>> + Send;

    fn deployment_status(
        &self,
        deployment_id: Uuid,
    ) -> impl Future<Output = Result<DeploymentPublicationStatus, SourcePublicationError>> + Send;

    fn put_object(
        &self,
        request: ObjectUploadRequest,
    ) -> impl Future<Output = Result<ObjectUploadResult, SourcePublicationError>> + Send;
}

pub struct SourcePublicationRequest<'a, T, O> {
    pub transport: &'a T,
    pub observer: &'a O,
    pub deployment_id: Uuid,
    pub workspace_id: Uuid,
    pub project_id: Uuid,
    pub deployment_attempt_id: Uuid,
    pub bundle: &'a PreparedSourceBundle,
}

#[derive(Debug, Clone)]
pub struct PublishedSourceBundle {
    pub source_artifact_id: String,
    pub runtime_artifact_graph_digest: String,
    pub runtime_artifact_graph: VerifiedRuntimeArtifactGraph,
    pub deployment_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedSourceUpload {
    pub source_artifact_id: String,
}

pub async fn publish_source_bundle<T, O>(
    request: SourcePublicationRequest<'_, T, O>,
) -> Result<PublishedSourceBundle, SourcePublicationError>
where
    T: SourcePublicationTransport,
    O: PublicationObserver,
{
    publish_source_upload(&request).await?;
    await_durable_readback(&request).await
}

pub async fn publish_source_bundle_upload<T, O>(
    request: SourcePublicationRequest<'_, T, O>,
) -> Result<PublishedSourceUpload, SourcePublicationError>
where
    T: SourcePublicationTransport,
    O: PublicationObserver,
{
    publish_source_upload(&request).await
}

async fn publish_source_upload<T, O>(
    request: &SourcePublicationRequest<'_, T, O>,
) -> Result<PublishedSourceUpload, SourcePublicationError>
where
    T: SourcePublicationTransport,
    O: PublicationObserver,
{
    request.observer.on_event(PublicationEvent::Preparing);
    let body = prepare_request(request, None)?;
    let mut prepared = prepare_with_retry(request, &body).await?;
    let multipart = match upload_source(request, &prepared).await {
        Ok(completion) => completion,
        Err(SourcePublicationError::ConditionalUploadConflict(_)) => {
            request
                .observer
                .on_event(PublicationEvent::RecoveringConditionalConflict);
            let recovery = CliPrepareUploadSourceUploadRecovery {
                failed_upload_session_id: prepared.upload_session_id,
                reason: CONDITIONAL_RECOVERY_REASON.to_string(),
            };
            let recovery_body = prepare_request(request, Some(recovery))?;
            let recovered = prepare_with_retry(request, &recovery_body).await?;
            match upload_source(request, &recovered).await {
                Ok(completion) => {
                    prepared = recovered;
                    completion
                }
                Err(error) => {
                    report_upload_failed(request, &recovered, &error).await;
                    return Err(error);
                }
            }
        }
        Err(error) => {
            report_upload_failed(request, &prepared, &error).await;
            return Err(error);
        }
    };

    if prepared.required_complete
        == CliPrepareUploadRequiredComplete::MultipartCompleteUploadComplete
    {
        request
            .observer
            .on_event(PublicationEvent::CompletingMultipart);
        let completion = multipart.ok_or_else(|| {
            SourcePublicationError::InvalidResponse(
                "multipart completion was required but no parts were uploaded".to_string(),
            )
        })?;
        let complete = multipart_complete_request(request, &prepared, completion)?;
        retry_control_plane(
            request.observer,
            "multipart-complete",
            COMPLETION_BUDGET,
            || async {
                request
                    .transport
                    .complete_multipart(request.deployment_id, &complete)
                    .await
                    .map(|_| ())
            },
        )
        .await?;
    } else if multipart.is_some() {
        return Err(SourcePublicationError::InvalidResponse(
            "multipart upload was performed without multipart completion authority".to_string(),
        ));
    }

    request
        .observer
        .on_event(PublicationEvent::CompletingUpload);
    let upload_complete = upload_complete_request(request, &prepared)?;
    complete_upload_with_retry(request, &upload_complete).await?;
    Ok(PublishedSourceUpload {
        source_artifact_id: request.bundle.source_artifact_id().to_string(),
    })
}

fn prepare_request<T, O>(
    request: &SourcePublicationRequest<'_, T, O>,
    source_upload_recovery: Option<CliPrepareUploadSourceUploadRecovery>,
) -> Result<CliPrepareUploadRequest, SourcePublicationError> {
    let summary = summarize_logical_manifest(request.bundle.manifest());
    let multipart =
        request
            .bundle
            .multipart()
            .map(|multipart| {
                let parts = multipart
                    .parts
                    .iter()
                    .map(|part| {
                        Ok(CliPrepareMultipartPart {
                            part_number: NonZeroU64::new(u64::from(part.part_number)).ok_or_else(
                                || invalid_source("multipart part number must be non-zero"),
                            )?,
                            size_bytes: NonZeroU64::new(part.size_bytes).ok_or_else(|| {
                                invalid_source("multipart part size must be non-zero")
                            })?,
                            sha256: contract_string(&part.sha256, "multipart part sha256")?,
                        })
                    })
                    .collect::<Result<Vec<_>, SourcePublicationError>>()?;
                Ok::<CliPrepareMultipart, SourcePublicationError>(CliPrepareMultipart {
                    part_count: NonZeroU64::new(u64::try_from(parts.len()).map_err(|_| {
                        invalid_source("multipart part count exceeds the wire range")
                    })?)
                    .ok_or_else(|| invalid_source("multipart part count must be non-zero"))?,
                    part_size_bytes: NonZeroU64::new(multipart.part_size_bytes)
                        .ok_or_else(|| invalid_source("multipart part size must be non-zero"))?,
                    parts,
                })
            })
            .transpose()?;
    Ok(CliPrepareUploadRequest {
        deployment_id: request.deployment_id,
        workspace_id: request.workspace_id,
        project_id: request.project_id,
        deployment_attempt_id: request.deployment_attempt_id,
        operation_id: Uuid::now_v7(),
        artifact_format: "SOURCE_BUNDLE_V1".to_string(),
        cli_protocol_version: contract_string(CLI_PROTOCOL_VERSION, "CLI protocol version")?,
        logical_manifest_summary: CliLogicalManifestSummary {
            file_count: i64::from(summary.file_count),
            logical_static_bytes: contract_string(
                &summary.logical_static_bytes.to_string(),
                "logical static bytes",
            )?,
            artifact_size_bytes: contract_string(
                &summary.artifact_size_bytes.to_string(),
                "artifact size bytes",
            )?,
            max_static_file_size_bytes: contract_string(
                &request.bundle.max_static_file_size_bytes().to_string(),
                "max static file size bytes",
            )?,
        },
        logical_manifest_sha256: contract_string(
            request.bundle.logical_manifest_sha256(),
            "logical manifest sha256",
        )?,
        source_format: SOURCE_BUNDLE_FORMAT.to_string(),
        source_sha256: contract_string(request.bundle.source_sha256(), "source sha256")?,
        source_size_bytes: contract_string(
            &request.bundle.source_size_bytes().to_string(),
            "source size bytes",
        )?,
        multipart,
        source_upload_recovery,
    })
}

async fn prepare_with_retry<T, O>(
    request: &SourcePublicationRequest<'_, T, O>,
    body: &CliPrepareUploadRequest,
) -> Result<CliPrepareUploadResponse, SourcePublicationError>
where
    T: SourcePublicationTransport,
    O: PublicationObserver,
{
    let prepared = retry_control_plane(request.observer, "prepare-upload", PREPARE_BUDGET, || {
        request
            .transport
            .prepare_upload(request.deployment_id, body)
    })
    .await?;
    if prepared.source_artifact_id.as_str() != request.bundle.source_artifact_id() {
        return Err(invalid_response(
            "prepare-upload returned a source artifact id that does not match the verified bundle",
        ));
    }
    Ok(prepared)
}

#[derive(Debug)]
struct MultipartCompletion {
    upload_id: String,
    parts: Vec<ObjectPart>,
}

#[derive(Debug)]
struct ObjectPart {
    part_number: u32,
    e_tag: String,
}

async fn upload_source<T, O>(
    request: &SourcePublicationRequest<'_, T, O>,
    prepared: &CliPrepareUploadResponse,
) -> Result<Option<MultipartCompletion>, SourcePublicationError>
where
    T: SourcePublicationTransport,
    O: PublicationObserver,
{
    if prepared.kind != "source-upload" {
        return Err(SourcePublicationError::InvalidResponse(format!(
            "unexpected prepare-upload kind: {}",
            prepared.kind
        )));
    }
    if prepared.fast_path {
        if prepared.presigned_put.is_some() || prepared.multipart.is_some() {
            return Err(SourcePublicationError::InvalidResponse(
                "fast path returned upload targets".to_string(),
            ));
        }
        return Ok(None);
    }
    request.observer.on_event(PublicationEvent::Uploading);
    match (&prepared.presigned_put, &prepared.multipart) {
        (Some(target), None) => {
            let bytes = request.bundle.read_all().await?;
            verify_payload(
                &bytes,
                signed_size(target.content_length, "presignedPut.contentLength")?,
                target.sha256.as_str(),
            )?;
            request
                .transport
                .put_object(ObjectUploadRequest {
                    url: target.url.clone(),
                    bytes,
                    sha256: target.sha256.as_str().to_string(),
                    headers: target.headers.as_ref().map_or(
                        ObjectUploadHeaders {
                            content_type: None,
                            if_none_match: None,
                        },
                        |headers| ObjectUploadHeaders {
                            content_type: Some(headers.content_type.clone()),
                            if_none_match: headers.if_none_match.clone(),
                        },
                    ),
                    verify_head: target
                        .verify_head
                        .as_ref()
                        .map(|head| {
                            Ok::<ObjectHeadVerification, SourcePublicationError>(
                                ObjectHeadVerification {
                                    url: head.url.clone(),
                                    content_length: signed_size(
                                        head.content_length,
                                        "verifyHead.contentLength",
                                    )?,
                                    sha256: head.sha256.as_str().to_string(),
                                },
                            )
                        })
                        .transpose()?,
                })
                .await?;
            Ok(None)
        }
        (None, Some(target)) => {
            let mut parts = Vec::with_capacity(target.chunks.len());
            for chunk in &target.chunks {
                let part_number = u32::try_from(chunk.part_number.get())
                    .map_err(|_| invalid_response("multipart part number exceeds u32"))?;
                let content_length =
                    signed_size(chunk.content_length, "multipart chunk contentLength")?;
                let offset = u64::from(part_number.saturating_sub(1))
                    .checked_mul(target.chunk_size.get())
                    .ok_or_else(|| invalid_response("multipart chunk offset overflow"))?;
                let bytes = request.bundle.read_chunk(offset, content_length).await?;
                verify_payload(&bytes, content_length, chunk.sha256.as_str())?;
                let result = request
                    .transport
                    .put_object(ObjectUploadRequest {
                        url: chunk.url.clone(),
                        bytes,
                        sha256: chunk.sha256.as_str().to_string(),
                        headers: ObjectUploadHeaders {
                            content_type: None,
                            if_none_match: None,
                        },
                        verify_head: None,
                    })
                    .await?;
                let e_tag = result
                    .e_tag
                    .ok_or_else(|| invalid_response("multipart object upload returned no ETag"))?;
                parts.push(ObjectPart { part_number, e_tag });
            }
            Ok(Some(MultipartCompletion {
                upload_id: target.upload_id.as_str().to_string(),
                parts,
            }))
        }
        (None, None) => Err(invalid_response("prepare-upload returned no upload target")),
        (Some(_), Some(_)) => Err(invalid_response(
            "prepare-upload returned both single and multipart targets",
        )),
    }
}

fn multipart_complete_request<T, O>(
    request: &SourcePublicationRequest<'_, T, O>,
    prepared: &CliPrepareUploadResponse,
    completion: MultipartCompletion,
) -> Result<CliMultipartCompleteRequest, SourcePublicationError> {
    let parts = completion
        .parts
        .into_iter()
        .map(|part| {
            Ok(CliMultipartCompletePart {
                part_number: NonZeroU64::new(u64::from(part.part_number))
                    .ok_or_else(|| invalid_response("multipart part number is zero"))?,
                e_tag: contract_string(&part.e_tag, "multipart ETag")?,
            })
        })
        .collect::<Result<Vec<_>, SourcePublicationError>>()?;
    Ok(CliMultipartCompleteRequest {
        deployment_id: request.deployment_id,
        upload_session_id: prepared.upload_session_id,
        deployment_attempt_id: request.deployment_attempt_id,
        operation_id: Uuid::now_v7(),
        artifact_format: "SOURCE_BUNDLE_V1".to_string(),
        source_artifact_id: contract_string(
            prepared.source_artifact_id.as_str(),
            "source artifact id",
        )?,
        upload_id: contract_string(&completion.upload_id, "multipart upload id")?,
        parts,
    })
}

fn upload_complete_request<T, O>(
    request: &SourcePublicationRequest<'_, T, O>,
    prepared: &CliPrepareUploadResponse,
) -> Result<CliUploadCompleteRequest, SourcePublicationError> {
    Ok(CliUploadCompleteRequest {
        deployment_id: request.deployment_id,
        upload_session_id: prepared.upload_session_id,
        deployment_attempt_id: request.deployment_attempt_id,
        operation_id: Uuid::now_v7(),
        artifact_format: "SOURCE_BUNDLE_V1".to_string(),
        source_artifact_id: contract_string(
            prepared.source_artifact_id.as_str(),
            "source artifact id",
        )?,
        source_sha256: contract_string(request.bundle.source_sha256(), "source sha256")?,
        source_size_bytes: contract_string(
            &request.bundle.source_size_bytes().to_string(),
            "source size bytes",
        )?,
        logical_manifest_sha256: contract_string(
            request.bundle.logical_manifest_sha256(),
            "logical manifest sha256",
        )?,
    })
}

async fn complete_upload_with_retry<T, O>(
    request: &SourcePublicationRequest<'_, T, O>,
    body: &CliUploadCompleteRequest,
) -> Result<(), SourcePublicationError>
where
    T: SourcePublicationTransport,
    O: PublicationObserver,
{
    retry_control_plane(
        request.observer,
        "upload-complete",
        COMPLETION_BUDGET,
        || async {
            match request
                .transport
                .complete_upload(request.deployment_id, body)
                .await?
            {
                CliUploadCompleteResponse::SourceUploadCompleted { .. }
                | CliUploadCompleteResponse::SourceFastPathCompleted { .. }
                | CliUploadCompleteResponse::SourceVerifiedAwaitingRuntime { .. }
                | CliUploadCompleteResponse::NoopAlreadyCompleted { .. } => Ok(()),
                CliUploadCompleteResponse::Incomplete { .. } => Err(
                    StructuredControlPlaneError::retryable("SOURCE_UPLOAD_INCOMPLETE"),
                ),
                CliUploadCompleteResponse::Expired { expired_at, .. } => {
                    Err(SourcePublicationError::InvalidResponse(format!(
                        "source upload window expired at {expired_at}"
                    )))
                }
            }
        },
    )
    .await
}

async fn await_durable_readback<T, O>(
    request: &SourcePublicationRequest<'_, T, O>,
) -> Result<PublishedSourceBundle, SourcePublicationError>
where
    T: SourcePublicationTransport,
    O: PublicationObserver,
{
    request
        .observer
        .on_event(PublicationEvent::AwaitingDurableReadback);
    let started = Instant::now();
    let mut delay = INITIAL_RETRY_DELAY;
    loop {
        let status = match request
            .transport
            .deployment_status(request.deployment_id)
            .await
        {
            Ok(status) => status,
            Err(error) if retry_hint(&error).is_some() => {
                if started.elapsed() >= DURABLE_READBACK_BUDGET {
                    return Err(SourcePublicationError::Deadline(format!(
                        "durable runtime graph readback remained unavailable: {error}"
                    )));
                }
                request.observer.on_event(PublicationEvent::Waiting {
                    operation: "durable-readback",
                });
                let remaining = DURABLE_READBACK_BUDGET.saturating_sub(started.elapsed());
                let hinted = retry_hint(&error).flatten();
                let sleep_for = hinted.unwrap_or(delay).max(delay).min(remaining);
                tokio::time::sleep(sleep_for).await;
                delay = delay.saturating_mul(2).min(MAX_RETRY_DELAY);
                continue;
            }
            Err(error) => return Err(error),
        };
        if matches!(
            status.status.to_ascii_uppercase().as_str(),
            "FAILED" | "CANCELLED"
        ) {
            return Err(SourcePublicationError::InvalidResponse(format!(
                "deployment failed while awaiting durable artifact readback: {} ({})",
                status
                    .error
                    .unwrap_or_else(|| "unknown failure".to_string()),
                status.error_code.unwrap_or_else(|| "UNKNOWN".to_string())
            )));
        }
        if let (Some(actual), Some(graph)) = (
            status.runtime_artifact_graph_digest,
            status.runtime_artifact_graph,
        ) {
            let graph = verify_durable_runtime_graph(request.bundle, actual.as_str(), graph)?;
            request.observer.on_event(PublicationEvent::DurableVerified);
            return Ok(PublishedSourceBundle {
                source_artifact_id: request.bundle.source_artifact_id().to_string(),
                runtime_artifact_graph_digest: actual,
                runtime_artifact_graph: graph,
                deployment_status: status.status,
            });
        }
        let elapsed = started.elapsed();
        if elapsed >= DURABLE_READBACK_BUDGET {
            return Err(SourcePublicationError::Deadline(
                "durable runtime graph readback remained unavailable".to_string(),
            ));
        }
        request.observer.on_event(PublicationEvent::Waiting {
            operation: "durable-readback",
        });
        let remaining = DURABLE_READBACK_BUDGET.saturating_sub(elapsed);
        tokio::time::sleep(delay.min(remaining)).await;
        delay = delay.saturating_mul(2).min(MAX_RETRY_DELAY);
    }
}

fn verify_durable_runtime_graph(
    bundle: &PreparedSourceBundle,
    durable_digest: &str,
    wire: RuntimeArtifactGraphV2Wire,
) -> Result<VerifiedRuntimeArtifactGraph, SourcePublicationError> {
    let application_paths = bundle
        .manifest()
        .files
        .iter()
        .filter(|file| file.role != "dependency")
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    let value = serde_json::to_value(wire).map_err(|error| {
        invalid_response(&format!(
            "durable runtime graph serialization failed: {error}"
        ))
    })?;
    let graph = verify_runtime_artifact_graph(value, &application_paths).map_err(|error| {
        invalid_response(&format!(
            "durable runtime graph verification failed: {error}"
        ))
    })?;
    if graph.graph_digest() != durable_digest {
        return Err(invalid_response(
            "durable runtime graph digest does not match deployment status",
        ));
    }

    let application = &graph.wire().application;
    let expected_artifact_id = compute_source_logical_artifact_id(
        bundle.logical_manifest_sha256(),
        bundle.source_sha256(),
    );
    let expected_blob_digest = format!("sha256:{}", bundle.source_sha256());
    let expected_size = i64::try_from(bundle.source_size_bytes()).map_err(|_| {
        invalid_response("verified source bundle size exceeds the runtime graph contract")
    })?;
    if application.artifact_id.as_str() != expected_artifact_id
        || application.manifest_digest.as_str() != bundle.logical_manifest_sha256()
        || application.blob_descriptor.digest.as_str() != expected_blob_digest
        || application.blob_descriptor.size != expected_size
    {
        return Err(invalid_response(
            "durable runtime graph application does not match the verified source bundle",
        ));
    }

    Ok(graph)
}

async fn report_upload_failed<T, O>(
    request: &SourcePublicationRequest<'_, T, O>,
    prepared: &CliPrepareUploadResponse,
    error: &SourcePublicationError,
) where
    T: SourcePublicationTransport,
    O: PublicationObserver,
{
    let Ok(error_code) = SOURCE_UPLOAD_PUT_FAILED.try_into() else {
        return;
    };
    let log = bounded_failure_log(error);
    let Ok(error_log) = log.as_str().try_into() else {
        return;
    };
    let Ok(source_artifact_id) = prepared.source_artifact_id.as_str().try_into() else {
        return;
    };
    let body = CliUploadFailedRequest {
        deployment_id: request.deployment_id,
        upload_session_id: prepared.upload_session_id,
        deployment_attempt_id: request.deployment_attempt_id,
        operation_id: Uuid::now_v7(),
        artifact_format: "SOURCE_BUNDLE_V1".to_string(),
        source_artifact_id,
        error_code,
        error_log,
    };
    let _ = retry_control_plane(
        request.observer,
        "upload-failed",
        FAILURE_REPORT_BUDGET,
        || async {
            request
                .transport
                .report_upload_failed(request.deployment_id, &body)
                .await
                .map(|_| ())
        },
    )
    .await;
}

pub(crate) async fn retry_control_plane<T, O, F, Fut>(
    observer: &O,
    operation: &'static str,
    budget: Duration,
    mut call: F,
) -> Result<T, SourcePublicationError>
where
    O: PublicationObserver,
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, SourcePublicationError>>,
{
    let started = Instant::now();
    let mut delay = INITIAL_RETRY_DELAY;
    loop {
        match call().await {
            Ok(value) => return Ok(value),
            Err(error) if retry_hint(&error).is_some() => {
                let elapsed = started.elapsed();
                if elapsed >= budget {
                    return Err(SourcePublicationError::Deadline(format!(
                        "{operation} remained retryable for {budget:?}: {error}"
                    )));
                }
                observer.on_event(PublicationEvent::Waiting { operation });
                let remaining = budget.saturating_sub(elapsed);
                let hinted = retry_hint(&error).flatten();
                let sleep_for = hinted.unwrap_or(delay).max(delay).min(remaining);
                tokio::time::sleep(sleep_for).await;
                delay = delay.saturating_mul(2).min(MAX_RETRY_DELAY);
            }
            Err(error) => return Err(error),
        }
    }
}

fn retry_hint(error: &SourcePublicationError) -> Option<Option<Duration>> {
    match error {
        SourcePublicationError::AmbiguousTransport(_) => Some(None),
        SourcePublicationError::ControlPlane(error)
            if matches!(
                error.code.as_str(),
                "OPERATION_IN_PROGRESS"
                    | "SERVICE_UNAVAILABLE"
                    | "TOO_MANY_REQUESTS"
                    | "SOURCE_UPLOAD_INCOMPLETE"
            ) =>
        {
            Some(error.retry_after)
        }
        _ => None,
    }
}

impl StructuredControlPlaneError {
    fn retryable(code: &str) -> SourcePublicationError {
        Self {
            status: 409,
            code: code.to_string(),
            message: code.to_string(),
            retry_after: None,
            details: None,
        }
        .into()
    }
}

fn verify_payload(
    bytes: &[u8],
    expected_size: u64,
    expected_sha256: &str,
) -> Result<(), SourcePublicationError> {
    if bytes.len() as u64 != expected_size {
        return Err(invalid_source(
            "source payload size changed after prepare-upload",
        ));
    }
    let actual = nrz_source_bundle::sha256_hex(bytes);
    if actual != expected_sha256 {
        return Err(invalid_source(
            "source payload digest changed after prepare-upload",
        ));
    }
    Ok(())
}

fn signed_size(value: i64, label: &str) -> Result<u64, SourcePublicationError> {
    u64::try_from(value)
        .map_err(|_| invalid_response(&format!("server returned negative {label}: {value}")))
}

fn contract_string<T>(value: &str, label: &str) -> Result<T, SourcePublicationError>
where
    T: TryFrom<String>,
    T::Error: std::fmt::Display,
{
    value
        .to_string()
        .try_into()
        .map_err(|error| invalid_source(&format!("invalid {label}: {error}")))
}

fn invalid_source(message: &str) -> SourcePublicationError {
    SourcePublicationError::InvalidSourceBundle(message.to_string())
}

fn invalid_response(message: &str) -> SourcePublicationError {
    SourcePublicationError::InvalidResponse(message.to_string())
}

fn bounded_failure_log(error: &SourcePublicationError) -> String {
    let mut message = redact_urls(&error.to_string());
    if message.len() > MAX_UPLOAD_FAILURE_LOG_LENGTH {
        let mut end = MAX_UPLOAD_FAILURE_LOG_LENGTH;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
    }
    message
}

fn redact_urls(message: &str) -> String {
    message
        .split_whitespace()
        .map(|token| {
            if token.starts_with("http://") || token.starts_with("https://") {
                token
                    .split_once('?')
                    .map_or_else(|| token.to_string(), |(base, _)| format!("{base}?REDACTED"))
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
