use super::*;

pub(super) async fn prepare_upload_and_complete(
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

    let deployment_uuid = Uuid::parse_str(deployment_id)
        .with_context(|| format!("deployment id is not a valid UUID: {deployment_id}"))?;
    let workspace_uuid = Uuid::parse_str(workspace_id)
        .with_context(|| format!("workspace id is not a valid UUID: {workspace_id}"))?;
    let project_uuid = Uuid::parse_str(project_id)
        .with_context(|| format!("project id is not a valid UUID: {project_id}"))?;
    let attempt_uuid = Uuid::parse_str(deployment_attempt_id).with_context(|| {
        format!("deployment attempt id is not a valid UUID: {deployment_attempt_id}")
    })?;
    let body = build_prepare_upload_request(
        deployment_uuid,
        workspace_uuid,
        project_uuid,
        attempt_uuid,
        plan,
        None,
    )?;

    let mut prepared = prepare_upload_with_retry(client, deployment_uuid, &body, json).await?;

    let multipart_completion =
        match upload_source_object(client, &prepared, plan, json).await {
            Ok(completion) => completion,
            Err(error) if is_conditional_upload_conflict(&error) => {
                output::status(
                    json,
                    "~",
                    "Recovering SOURCE_BUNDLE_V1 source upload...",
                    output::Phase::Deploy,
                );
                let recovery_body = build_prepare_upload_request(
                    deployment_uuid,
                    workspace_uuid,
                    project_uuid,
                    attempt_uuid,
                    plan,
                    Some(CliPrepareUploadSourceUploadRecovery {
                        failed_upload_session_id: prepared.upload_session_id,
                        reason: SOURCE_UPLOAD_RECOVERY_CONDITIONAL_PRECONDITION_FAILED.to_string(),
                    }),
                )?;
                let recovered_prepared =
                    match prepare_upload_with_retry(client, deployment_uuid, &recovery_body, json)
                        .await
                    {
                        Ok(prepared) => prepared,
                        Err(recovery_error) => {
                            report_source_object_upload_failed(
                                client,
                                deployment_id,
                                deployment_attempt_id,
                                &prepared,
                                &recovery_error,
                                json,
                            )
                            .await;
                            return Err(recovery_error);
                        }
                    };
                match upload_source_object(client, &recovered_prepared, plan, json).await {
                    Ok(completion) => {
                        prepared = recovered_prepared;
                        completion
                    }
                    Err(recovery_error) => {
                        report_source_object_upload_failed(
                            client,
                            deployment_id,
                            deployment_attempt_id,
                            &recovered_prepared,
                            &recovery_error,
                            json,
                        )
                        .await;
                        return Err(recovery_error);
                    }
                }
            }
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

    complete_upload_with_retry(
        client,
        deployment_uuid,
        prepared.upload_session_id,
        attempt_uuid,
        json,
        plan,
        prepared.source_artifact_id.as_str(),
    )
    .await?;

    Ok(())
}

pub(super) fn build_prepare_upload_request(
    deployment_uuid: Uuid,
    workspace_uuid: Uuid,
    project_uuid: Uuid,
    deployment_attempt_uuid: Uuid,
    plan: &SourceBundlePlan,
    source_upload_recovery: Option<CliPrepareUploadSourceUploadRecovery>,
) -> anyhow::Result<CliPrepareUploadRequest> {
    let source_size_bytes = plan.source_size_string();
    Ok(CliPrepareUploadRequest {
        deployment_id: deployment_uuid,
        workspace_id: workspace_uuid,
        project_id: project_uuid,
        deployment_attempt_id: deployment_attempt_uuid,
        operation_id: Uuid::now_v7(),
        artifact_format: "SOURCE_BUNDLE_V1".to_string(),
        cli_protocol_version: CLI_PROTOCOL_VERSION
            .try_into()
            .map_err(|e| anyhow::anyhow!("invalid cli_protocol_version: {e}"))?,
        logical_manifest_summary: to_contract_manifest_summary(&plan.logical_manifest_summary)?,
        logical_manifest_sha256: plan
            .logical_manifest_sha256
            .as_str()
            .try_into()
            .map_err(|e| {
                anyhow::anyhow!("invalid logical_manifest_sha256 for prepare-upload: {e}")
            })?,
        source_format: SOURCE_BUNDLE_FORMAT.to_string(),
        source_sha256: plan
            .source_sha256
            .as_str()
            .try_into()
            .map_err(|e| anyhow::anyhow!("invalid source_sha256 for prepare-upload: {e}"))?,
        source_size_bytes: source_size_bytes
            .as_str()
            .try_into()
            .map_err(|e| anyhow::anyhow!("invalid source_size_bytes for prepare-upload: {e}"))?,
        multipart: plan
            .multipart
            .as_ref()
            .map(to_contract_multipart)
            .transpose()?,
        source_upload_recovery,
    })
}

pub(super) fn is_conditional_upload_conflict(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.is::<ConditionalUploadConflict>())
}

pub(super) fn to_contract_manifest_summary(
    summary: &source_bundle_v1::SourceLogicalManifestSummary,
) -> anyhow::Result<CliLogicalManifestSummary> {
    Ok(CliLogicalManifestSummary {
        file_count: i64::try_from(summary.file_count)
            .context("manifest file_count exceeds i64 range")?,
        logical_static_bytes: summary
            .logical_static_bytes
            .as_str()
            .try_into()
            .map_err(|e| anyhow::anyhow!("invalid logicalStaticBytes: {e}"))?,
        artifact_size_bytes: summary
            .artifact_size_bytes
            .as_str()
            .try_into()
            .map_err(|e| anyhow::anyhow!("invalid artifactSizeBytes: {e}"))?,
        max_static_file_size_bytes: summary
            .max_static_file_size_bytes
            .as_str()
            .try_into()
            .map_err(|e| anyhow::anyhow!("invalid maxStaticFileSizeBytes: {e}"))?,
    })
}

pub(super) fn to_contract_multipart(
    multipart: &source_bundle_v1::SourceBundleMultipartDescriptor,
) -> anyhow::Result<CliPrepareMultipart> {
    let parts = multipart
        .parts
        .iter()
        .map(|part| {
            Ok(CliPrepareMultipartPart {
                part_number: NonZeroU64::new(u64::from(part.part_number))
                    .context("multipart part number must be non-zero")?,
                size_bytes: NonZeroU64::new(part.size_bytes)
                    .context("multipart part size must be non-zero")?,
                sha256: part
                    .sha256
                    .as_str()
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("invalid multipart part sha256: {e}"))?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(CliPrepareMultipart {
        part_count: NonZeroU64::new(u64::from(multipart.part_count))
            .context("multipart part count must be non-zero")?,
        part_size_bytes: NonZeroU64::new(multipart.part_size_bytes)
            .context("multipart part size must be non-zero")?,
        parts,
    })
}

// ── Resume deploy flow ───────────────────────────────────────

pub(super) type PrepareUploadResponse = CliPrepareUploadResponse;
pub(super) type RequiredComplete = CliPrepareUploadRequiredComplete;

pub(super) type UploadCompleteResponse = CliUploadCompleteResponse;

pub(super) type UploadFailedResponse = CliUploadFailedResponse;

pub(super) type MultipartCompleteResponse = CliMultipartCompleteResponse;

pub(super) struct SourceMultipartCompletion {
    upload_id: String,
    parts: Vec<CompletedMultipartPart>,
}

/// Map a *non-retryable* prepare-upload error to the value returned to the
/// caller. User-fault plan-limit failures in JSON mode are reported on both
/// channels (stderr frame for the Builder + stdout envelope with `code` for
/// CLI/automation) via [`output::report_terminal_error`], which returns an
/// `AlreadyReportedError` so `main` does not re-emit a code-less envelope.
/// Everything else stays a contextual `anyhow` that `main` renders as the
/// terminal outcome.
pub(super) fn prepare_upload_terminal_error(error: anyhow::Error, json: bool) -> anyhow::Error {
    if json
        && let Some(api_err) = error.downcast_ref::<crate::api::StructuredApiError>()
        && (api_err.code == "LIMIT_EXCEEDED" || api_err.code == "SUBSCRIPTION_REQUIRED")
    {
        return output::report_terminal_error(
            "deploy",
            &api_err.message,
            &api_err.code,
            api_err.details.as_ref(),
        );
    }
    error.context("failed to prepare upload")
}

pub(super) async fn prepare_upload_with_retry(
    client: &ApiClient,
    deployment_id: Uuid,
    body: &CliPrepareUploadRequest,
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
                    return Err(prepare_upload_terminal_error(error, json));
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
pub(super) enum SourceControlPlaneRetryReason {
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
pub(super) struct SourceControlPlaneRetry {
    pub(super) reason: SourceControlPlaneRetryReason,
    pub(super) retry_after: Option<Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceCompletionAttempt {
    Terminal,
    Retry(SourceControlPlaneRetry),
}

pub(super) fn classify_prepare_upload_retry_error(
    error: &anyhow::Error,
) -> Option<SourceControlPlaneRetry> {
    classify_standard_control_plane_retry_error(
        error,
        SourceControlPlaneRetryReason::PrepareUploadInProgress,
    )
}

pub(super) async fn complete_upload_with_retry(
    client: &ApiClient,
    deployment_id: Uuid,
    upload_session_id: Uuid,
    deployment_attempt_id: Uuid,
    json: bool,
    plan: &SourceBundlePlan,
    source_artifact_id: &str,
) -> anyhow::Result<()> {
    let started = Instant::now();
    let mut delay = SOURCE_COMPLETION_INITIAL_RETRY_DELAY;
    let mut attempts = 0u32;
    let source_size_bytes = plan.source_size_string();
    let body =
        CliUploadCompleteRequest {
            deployment_id,
            upload_session_id,
            deployment_attempt_id,
            operation_id: Uuid::now_v7(),
            artifact_format: "SOURCE_BUNDLE_V1".to_string(),
            source_artifact_id: source_artifact_id.try_into().map_err(|e| {
                anyhow::anyhow!("invalid source_artifact_id for upload-complete: {e}")
            })?,
            source_sha256: plan
                .source_sha256
                .as_str()
                .try_into()
                .map_err(|e| anyhow::anyhow!("invalid source_sha256 for upload-complete: {e}"))?,
            source_size_bytes: source_size_bytes.as_str().try_into().map_err(|e| {
                anyhow::anyhow!("invalid source_size_bytes for upload-complete: {e}")
            })?,
            logical_manifest_sha256: plan.logical_manifest_sha256.as_str().try_into().map_err(
                |e| anyhow::anyhow!("invalid logical_manifest_sha256 for upload-complete: {e}"),
            )?,
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

pub(super) async fn post_upload_complete_once(
    client: &ApiClient,
    deployment_id: Uuid,
    body: &CliUploadCompleteRequest,
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

pub(super) fn classify_upload_complete_response(
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

pub(super) fn classify_upload_complete_retry_error(
    error: &anyhow::Error,
) -> Option<SourceControlPlaneRetry> {
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

pub(super) fn classify_multipart_complete_retry_error(
    error: &anyhow::Error,
) -> Option<SourceControlPlaneRetry> {
    classify_standard_control_plane_retry_error(
        error,
        SourceControlPlaneRetryReason::CompletionInProgress,
    )
}

pub(super) fn classify_upload_failed_retry_error(
    error: &anyhow::Error,
) -> Option<SourceControlPlaneRetry> {
    classify_standard_control_plane_retry_error(
        error,
        SourceControlPlaneRetryReason::FailureReportInProgress,
    )
}

pub(super) fn classify_standard_control_plane_retry_error(
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

pub(super) fn classify_control_plane_transport_retry_error(
    error: &anyhow::Error,
) -> Option<SourceControlPlaneRetry> {
    is_ambiguous_control_plane_transport_error(error).then_some(SourceControlPlaneRetry {
        reason: SourceControlPlaneRetryReason::TransportAmbiguous,
        retry_after: None,
    })
}

pub(super) fn is_ambiguous_control_plane_transport_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|err| err.status().is_none() && !err.is_builder() && !err.is_decode())
    })
}

pub(super) fn retry_delay_with_hint(
    retry_after: Option<Duration>,
    fallback: Duration,
    fallback_max: Duration,
    remaining: Duration,
) -> Duration {
    retry_after
        .unwrap_or_else(|| fallback.min(fallback_max))
        .min(remaining)
}

pub(super) fn signed_content_length(value: i64, label: &str) -> anyhow::Result<u64> {
    u64::try_from(value).with_context(|| format!("server returned negative {label}: {value}"))
}

pub(super) fn signed_part_number(value: NonZeroU64, label: &str) -> anyhow::Result<u32> {
    u32::try_from(value.get())
        .with_context(|| format!("server returned {label} outside u32 range: {}", value.get()))
}

pub(super) fn presigned_put_headers(
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

pub(super) fn presigned_head_verify(
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

pub(super) fn presigned_multipart_chunks(
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

pub(super) async fn upload_source_object(
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

pub(super) async fn complete_source_multipart_if_needed(
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

    let deployment_uuid = Uuid::parse_str(deployment_id)
        .with_context(|| format!("deployment id is not a valid UUID: {deployment_id}"))?;
    let attempt_uuid = Uuid::parse_str(deployment_attempt_id).with_context(|| {
        format!("deployment attempt id is not a valid UUID: {deployment_attempt_id}")
    })?;
    let parts = completion
        .parts
        .into_iter()
        .map(|part| {
            Ok(CliMultipartCompletePart {
                part_number: NonZeroU64::new(u64::from(part.part_number))
                    .context("multipart part number must be non-zero")?,
                e_tag: part
                    .e_tag
                    .as_str()
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("invalid multipart ETag: {e}"))?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let body = CliMultipartCompleteRequest {
        deployment_id: deployment_uuid,
        upload_session_id: prepared.upload_session_id,
        deployment_attempt_id: attempt_uuid,
        operation_id: Uuid::now_v7(),
        artifact_format: "SOURCE_BUNDLE_V1".to_string(),
        source_artifact_id: prepared
            .source_artifact_id
            .as_str()
            .try_into()
            .map_err(|e| {
                anyhow::anyhow!("invalid source_artifact_id for multipart-complete: {e}")
            })?,
        upload_id: completion
            .upload_id
            .as_str()
            .try_into()
            .map_err(|e| anyhow::anyhow!("invalid multipart upload id: {e}"))?,
        parts,
    };
    complete_source_multipart_with_retry(client, deployment_uuid, &body, json).await
}

pub(super) async fn complete_source_multipart_with_retry(
    client: &ApiClient,
    deployment_id: Uuid,
    body: &CliMultipartCompleteRequest,
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

pub(super) async fn post_source_multipart_complete_once(
    client: &ApiClient,
    deployment_id: Uuid,
    body: &CliMultipartCompleteRequest,
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

pub(super) async fn report_source_object_upload_failed(
    client: &ApiClient,
    deployment_id: &str,
    deployment_attempt_id: &str,
    prepared: &PrepareUploadResponse,
    error: &anyhow::Error,
    json: bool,
) {
    let body =
        match build_upload_failed_request(deployment_id, deployment_attempt_id, prepared, error) {
            Ok(body) => body,
            Err(build_error) => {
                output::warn(
                    json,
                    format!("Failed to build SOURCE_BUNDLE_V1 upload-failed report: {build_error}"),
                    output::Phase::Deploy,
                );
                return;
            }
        };

    if let Err(report_error) =
        report_source_object_upload_failed_with_retry(client, body.deployment_id, &body, json).await
    {
        output::warn(
            json,
            format!("Failed to mark SOURCE_BUNDLE_V1 upload as failed: {report_error}"),
            output::Phase::Deploy,
        );
    }
}

pub(super) fn build_upload_failed_request(
    deployment_id: &str,
    deployment_attempt_id: &str,
    prepared: &PrepareUploadResponse,
    error: &anyhow::Error,
) -> anyhow::Result<CliUploadFailedRequest> {
    Ok(CliUploadFailedRequest {
        deployment_id: Uuid::parse_str(deployment_id)
            .with_context(|| format!("deployment id is not a valid UUID: {deployment_id}"))?,
        upload_session_id: prepared.upload_session_id,
        deployment_attempt_id: Uuid::parse_str(deployment_attempt_id).with_context(|| {
            format!("deployment attempt id is not a valid UUID: {deployment_attempt_id}")
        })?,
        operation_id: Uuid::now_v7(),
        artifact_format: "SOURCE_BUNDLE_V1".to_string(),
        source_artifact_id: prepared
            .source_artifact_id
            .as_str()
            .try_into()
            .map_err(|e| anyhow::anyhow!("invalid source_artifact_id for upload-failed: {e}"))?,
        error_code: SOURCE_UPLOAD_PUT_FAILED
            .try_into()
            .map_err(|e| anyhow::anyhow!("invalid error_code for upload-failed: {e}"))?,
        error_log: upload_failure_log(error)
            .as_str()
            .try_into()
            .map_err(|e| anyhow::anyhow!("invalid error_log for upload-failed: {e}"))?,
    })
}

pub(super) async fn report_source_object_upload_failed_with_retry(
    client: &ApiClient,
    deployment_id: Uuid,
    body: &CliUploadFailedRequest,
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

pub(super) fn upload_failure_log(error: &anyhow::Error) -> String {
    let message = format!("{error:#}");
    let message = redact_url_credentials(&message);
    truncate_upload_failure_log(&message)
}

pub(super) fn truncate_upload_failure_log(message: &str) -> String {
    if message.len() <= MAX_UPLOAD_FAILURE_LOG_LENGTH {
        return message.to_string();
    }
    let mut end = MAX_UPLOAD_FAILURE_LOG_LENGTH;
    while !message.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    message[..end].to_string()
}

pub(super) fn redact_url_credentials(message: &str) -> String {
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

pub(super) fn find_url_start(message: &str) -> Option<usize> {
    match (message.find("http://"), message.find("https://")) {
        (Some(http), Some(https)) => Some(http.min(https)),
        (Some(http), None) => Some(http),
        (None, Some(https)) => Some(https),
        (None, None) => None,
    }
}

pub(super) fn url_candidate_len(message: &str) -> usize {
    message
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace() || matches!(ch, '"' | '\'' | '`' | '<' | '>'))
        .map(|(idx, _)| idx)
        .unwrap_or(message.len())
}

pub(super) fn redact_url_candidate(candidate: &str) -> String {
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

pub(super) fn split_url_candidate_suffix(candidate: &str) -> (&str, &str) {
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

pub(super) struct SinglePutUpload<'a> {
    pub(super) url: &'a str,
    pub(super) bytes: Bytes,
    pub(super) content_length: u64,
    pub(super) sha256: &'a str,
    pub(super) headers: &'a PresignedPutHeaders,
    pub(super) verify_head: Option<&'a PresignedHeadVerify>,
    pub(super) label: String,
}

pub(super) async fn upload_single_put(
    client: &ApiClient,
    upload: SinglePutUpload<'_>,
) -> anyhow::Result<()> {
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

pub(super) async fn upload_multipart_chunks<F, Fut>(
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

pub(super) fn verify_upload_payload(
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
    let actual_sha = sha256_hex(bytes);
    if actual_sha != sha256 {
        bail!(
            "{label} SHA drifted between prepare-upload and upload (server signed {sha256}, local materialized {actual_sha})"
        );
    }
    Ok(())
}
