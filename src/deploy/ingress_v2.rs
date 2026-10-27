//! External publication through the authenticated, observed ArtifactAgent.
use super::*;
use anyhow::ensure;
use nrz_contract::application_package_v1::APPLICATION_EROFS_PROFILE_V1;
use nrz_contract::source_bundle_v2::FileEntry;
use nrz_source_publisher::ingress_v2::{
    FrozenBlob, PublicationOutcome, TransferProvider, TransferTarget,
};
use std::io::Write as _;

pub(super) fn validate_configuration() -> anyhow::Result<()> {
    let transfer = std::env::var_os("NRZ_SOURCE_TRANSFER");
    let profile = std::env::var_os("NRZ_SOURCE_PACKAGE_PROFILE");
    validate_values(transfer.as_deref(), profile.as_deref())
}

fn validate_values(
    transfer: Option<&std::ffi::OsStr>,
    profile: Option<&std::ffi::OsStr>,
) -> anyhow::Result<()> {
    if transfer.is_some_and(|value| value == "V1") {
        return Err(output::coded_error(
            "CLI_UPDATE_REQUIRED",
            "Source V1 deployment writes are no longer supported; remove NRZ_SOURCE_TRANSFER=V1",
        ));
    }
    if transfer.is_some_and(|value| value != "V2") {
        return Err(output::coded_error(
            "INVALID_ARGUMENT",
            "NRZ_SOURCE_TRANSFER must be V2",
        ));
    }
    if profile.is_some_and(|value| value != APPLICATION_EROFS_PROFILE_V1) {
        return Err(output::coded_error(
            "INVALID_ARGUMENT",
            format!("NRZ_SOURCE_PACKAGE_PROFILE must be {APPLICATION_EROFS_PROFILE_V1}"),
        ));
    }
    Ok(())
}

pub(super) struct PublicationScope<'a> {
    pub deployment_id: &'a str,
    pub workspace_id: &'a str,
    pub project_id: &'a str,
    pub environment_id: &'a str,
    pub attempt: u32,
}

pub(super) async fn publish(
    client: &ApiClient,
    plan: &plan::DeployPlan,
    scope: PublicationScope<'_>,
    source_admitted_or_unknown: &mut bool,
    json: bool,
) -> anyhow::Result<()> {
    let PublicationScope {
        deployment_id,
        workspace_id,
        project_id,
        environment_id,
        attempt,
    } = scope;
    let frozen = tempfile::Builder::new()
        .prefix("nrz-source-v2-")
        .tempdir()?;
    let tree = frozen.path().join("tree");
    let (bytes, verified) = plan.freeze_source_bundle(
        crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::Embedded,
        &tree,
    )?;
    let mut manifest = tempfile::tempfile()?;
    manifest.write_all(&bytes)?;
    manifest.sync_all()?;
    let mut blobs = vec![FrozenBlob {
        hash: digest(verified.manifest_blake3())?,
        bytes: verified.manifest_size_bytes(),
        file: manifest,
    }];
    let root = nrz_runtime_artifact::ArtifactRoot::open(&tree)?;
    for entry in &verified.wire().files {
        if let FileEntry::File {
            path,
            blob_digest,
            size,
            ..
        } = entry
        {
            blobs.push(FrozenBlob {
                hash: digest(&blob_digest.value)?,
                bytes: *size,
                file: root.open_file(Path::new(&path.0))?,
            });
        }
    }
    let provider = TransferProvider::bind(blobs).await?;
    let summary = verified.summary();
    let operation_id = Uuid::now_v7();
    let idempotency_key = Uuid::now_v7();
    let body: nrz_api::AdmitRequestBody = serde_json::from_value(serde_json::json!({
        "deploymentId":deployment_id,
        "operationId":operation_id,
        "cliIdempotencyKey":idempotency_key,
        "cliAttemptId":Uuid::now_v7(),
        "protocolVersion":2,
        "packageProfile":APPLICATION_EROFS_PROFILE_V1,
        "manifest":{"algorithm":"BLAKE3","value":verified.manifest_blake3()},
        "manifestSizeBytes":verified.manifest_size_bytes(),
        "fileCount":summary.file_count,
        "logicalStaticBytes":summary.logical_static_bytes,
        "artifactBytes":summary.artifact_size_bytes,
        "maxFileSize":summary.max_static_file_size_bytes,
        "cliEndpointId":provider.endpoint_id(),
    }))?;
    // Retries are confined to this exact frozen input and live endpoint identity.
    let admitted = admit(client, deployment_id, &body, source_admitted_or_unknown)
        .await
        .map_err(|error| map_publication_error(error, json, &plan.artifact.file_breakdown))?;
    validate_admission(
        &admitted,
        &body,
        workspace_id,
        project_id,
        environment_id,
        attempt,
    )?;
    if admitted.state == nrz_api::Admit200ResponseState::Closed {
        require_published(&admitted)?;
        return Ok(());
    }
    let grant = admitted
        .transfer_grant
        .as_ref()
        .context("receiver transfer grant unavailable")?;
    ensure!(
        !grant.is_empty() && grant.len() <= 8192,
        "invalid bounded transfer grant"
    );
    let addresses = admitted
        .receiver_direct_addresses
        .as_ref()
        .context("observed direct receiver unavailable")?
        .iter()
        .map(|address| address.parse())
        .collect::<Result<Vec<_>, _>>()?;
    let now = chrono::Utc::now();
    let remaining = (admitted.expires_at.min(admitted.observation_valid_until) - now)
        .to_std()
        .context("receiver admission expired before transfer")?;
    ensure!(
        !remaining.is_zero(),
        "receiver admission expired before transfer"
    );
    let continuation = (admitted.expires_at - now)
        .to_std()
        .context("source operation expired before transfer")?;
    let instant = tokio::time::Instant::now();
    let deadline = instant + continuation;
    // The detached transfer owner also owns the frozen tree through reconciliation.
    let frozen = std::sync::Arc::new((root, frozen));
    let client = client.clone();
    let receiver = admitted.clone();
    let outcome = provider
        .publish_resume_with_reconciliation(
            TransferTarget {
                endpoint_id: admitted.receiver_endpoint_id.clone(),
                direct_addresses: addresses,
                operation_id,
                session_id: admitted.receiver_session_id,
                manifest_blake3: verified.manifest_blake3().into(),
            },
            grant.clone(),
            instant + remaining,
            deadline,
            move || {
                let (client, body, receiver, frozen) = (
                    client.clone(),
                    body.clone(),
                    receiver.clone(),
                    frozen.clone(),
                );
                async move {
                    let _frozen = frozen;
                    reconcile_before(&client, &body, &receiver, deadline).await
                }
            },
        )
        .await?;
    ensure!(
        outcome == PublicationOutcome::Published,
        "Source V2 publication deadline; original operation may remain pending"
    );
    Ok(())
}

async fn reconcile_before(
    client: &ApiClient,
    body: &nrz_api::AdmitRequestBody,
    original: &nrz_api::Admit200Response,
    deadline: tokio::time::Instant,
) -> anyhow::Result<PublicationOutcome> {
    let mut source_admitted_or_unknown = true;
    let result = admit_before(
        client,
        &body.deployment_id.to_string(),
        body,
        &mut source_admitted_or_unknown,
        deadline,
    )
    .await?;
    ensure!(
        result.operation_id == original.operation_id
            && result.cli_idempotency_key == original.cli_idempotency_key
            && result.input == original.input,
        "authenticated reconciliation changed the exact source operation or scope"
    );
    validate_receiver_custody(&result, original)?;
    if result.state == nrz_api::Admit200ResponseState::Active {
        return Ok(PublicationOutcome::Reconcile);
    }
    require_published(&result)?;
    Ok(PublicationOutcome::Published)
}

async fn admit(
    client: &ApiClient,
    deployment: &str,
    body: &nrz_api::AdmitRequestBody,
    source_admitted_or_unknown: &mut bool,
) -> anyhow::Result<nrz_api::Admit200Response> {
    admit_before(
        client,
        deployment,
        body,
        source_admitted_or_unknown,
        tokio::time::Instant::now() + SOURCE_REGISTRATION_RETRY_BUDGET,
    )
    .await
}

async fn admit_before(
    client: &ApiClient,
    deployment: &str,
    body: &nrz_api::AdmitRequestBody,
    source_admitted_or_unknown: &mut bool,
    deadline: tokio::time::Instant,
) -> anyhow::Result<nrz_api::Admit200Response> {
    let mut delay = SOURCE_REGISTRATION_INITIAL_RETRY_DELAY;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        ensure!(
            !remaining.is_zero(),
            "Source V2 admission deadline; original operation may remain pending"
        );
        // Only a first owner quota refusal proves no admission. An earlier
        // unavailable reply may hide committed custody, even if a retry refuses.
        let previous_outcome_unknown = *source_admitted_or_unknown;
        *source_admitted_or_unknown = true;
        let result = tokio::time::timeout(
            SOURCE_REGISTRATION_REQUEST_TIMEOUT.min(remaining),
            client.admit_static_source_v2(deployment, body.clone()),
        )
        .await;
        match result {
            Ok(Ok(response)) => return Ok(response),
            Ok(Err(error)) => {
                let Some(retry) = classify_api_retry(&error) else {
                    if !previous_outcome_unknown
                        && error
                            .downcast_ref::<crate::api::StructuredApiError>()
                            .is_some_and(|error| {
                                error.status == reqwest::StatusCode::FORBIDDEN
                                    && error.code == "LIMIT_EXCEEDED"
                            })
                    {
                        *source_admitted_or_unknown = false;
                    }
                    return Err(error);
                };
                tokio::time::sleep(retry.retry_after.unwrap_or(delay).min(remaining)).await;
            }
            Err(_) => tokio::time::sleep(delay.min(remaining)).await,
        }
        delay = delay
            .saturating_mul(2)
            .min(SOURCE_REGISTRATION_MAX_RETRY_DELAY);
    }
}

fn validate_admission(
    result: &nrz_api::Admit200Response,
    body: &nrz_api::AdmitRequestBody,
    workspace: &str,
    project: &str,
    environment: &str,
    attempt: u32,
) -> anyhow::Result<()> {
    let mut expected = serde_json::to_value(body)?;
    let object = expected
        .as_object_mut()
        .context("admission body must be an object")?;
    object.remove("operationId");
    object.remove("cliIdempotencyKey");
    object.insert(
        "workspaceId".into(),
        serde_json::to_value(Uuid::parse_str(workspace)?)?,
    );
    object.insert(
        "projectId".into(),
        serde_json::to_value(Uuid::parse_str(project)?)?,
    );
    object.insert(
        "environmentId".into(),
        serde_json::to_value(Uuid::parse_str(environment)?)?,
    );
    // Deployment source registration uses retry_count; publication uses its
    // positive attempt identity, exactly as the authenticated admission owner.
    let publication_attempt = attempt
        .checked_add(1)
        .context("deployment attempt overflow")?;
    object.insert(
        "deploymentAttempt".into(),
        serde_json::json!(publication_attempt),
    );
    ensure!(
        result.operation_id == body.operation_id
            && result.cli_idempotency_key == body.cli_idempotency_key
            && serde_json::to_value(&result.input)? == expected,
        "authenticated admission changed the exact source operation or scope"
    );
    ensure!(
        !result.receiver_session_id.is_nil() && !result.receiver_instance_id.is_nil(),
        "admission lacks receiver custody identity"
    );
    Ok(())
}

fn validate_receiver_custody(
    result: &nrz_api::Admit200Response,
    original: &nrz_api::Admit200Response,
) -> anyhow::Result<()> {
    ensure!(
        result.receiver_session_id == original.receiver_session_id
            && result.receiver_instance_id == original.receiver_instance_id
            && result.receiver_endpoint_id == original.receiver_endpoint_id,
        "authenticated reconciliation changed the original receiver custody"
    );
    Ok(())
}

fn require_published(result: &nrz_api::Admit200Response) -> anyhow::Result<()> {
    ensure!(
        result.state == nrz_api::Admit200ResponseState::Closed
            && result.terminal_reason.as_deref() == Some("PUBLISHED"),
        "Source V2 publication unresolved; original operation may remain pending"
    );
    Ok(())
}

fn digest(input: &str) -> anyhow::Result<[u8; 32]> {
    ensure!(input.len() == 64, "invalid BLAKE3 digest");
    let mut digest = [0u8; 32];
    for (output, pair) in digest.iter_mut().zip(input.as_bytes().as_chunks::<2>().0) {
        let pair = std::str::from_utf8(pair)?;
        *output = u8::from_str_radix(pair, 16)?;
    }
    Ok(digest)
}

#[cfg(test)]
#[path = "ingress_v2_tests.rs"]
mod tests;
