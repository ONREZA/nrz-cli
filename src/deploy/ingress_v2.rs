//! Opt-in external publication through the authenticated, observed ArtifactAgent.
use super::*;
use anyhow::ensure;
use nrz_contract::source_bundle_v2::FileEntry;
use nrz_source_publisher::ingress_v2::{FrozenBlob, TransferProvider, TransferTarget};
use std::io::Write as _;

pub(super) fn enabled() -> anyhow::Result<bool> {
    match std::env::var("NRZ_SOURCE_TRANSFER") {
        Err(std::env::VarError::NotPresent) => Ok(false),
        Ok(value) if value == "V1" => Ok(false),
        Ok(value) if value == "V2" => Ok(true),
        _ => bail!("NRZ_SOURCE_TRANSFER must be V1 or V2"),
    }
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
) -> anyhow::Result<()> {
    let PublicationScope {
        deployment_id,
        workspace_id,
        project_id,
        environment_id,
        attempt,
    } = scope;
    let package_profile = std::env::var("NRZ_SOURCE_PACKAGE_PROFILE")
        .context("Source V2 requires an installed NRZ_SOURCE_PACKAGE_PROFILE")?;
    ensure!(
        !package_profile.is_empty() && package_profile.len() <= 128,
        "invalid Source V2 package profile"
    );
    let frozen = tempfile::Builder::new()
        .prefix("nrz-source-v2-")
        .tempdir()?;
    let tree = frozen.path().join("tree");
    let (bytes, verified) = crate::artifact::source_bundle_v2::freeze_source_bundle(
        &plan.artifact.root,
        &plan.artifact.runtime.manifest,
        &plan.files,
        &plan.artifact.runtime.scan,
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
        "packageProfile":package_profile,
        "manifest":{"algorithm":"BLAKE3","value":verified.manifest_blake3()},
        "manifestSizeBytes":verified.manifest_size_bytes(),
        "fileCount":summary.file_count,
        "logicalStaticBytes":summary.logical_static_bytes,
        "artifactBytes":summary.artifact_size_bytes,
        "maxFileSize":summary.max_static_file_size_bytes,
        "cliEndpointId":provider.endpoint_id(),
    }))?;
    // Once a request is sent, an unavailable reply may hide an active operation.
    // The legacy fail-before-source transition must not supersede its custody.
    *source_admitted_or_unknown = true;
    // Retries are confined to this exact frozen input and live endpoint identity.
    let admitted = admit(client, deployment_id, &body).await?;
    validate_admission(
        &admitted,
        &body,
        workspace_id,
        project_id,
        environment_id,
        attempt,
    )?;
    if admitted.state == nrz_api::Admit200ResponseState::Closed {
        ensure!(
            admitted.terminal_reason.as_deref() == Some("PUBLISHED"),
            "Source V2 operation closed without publication"
        );
        return Ok(());
    }
    let grant = admitted
        .transfer_grant
        .context("receiver transfer grant unavailable")?;
    ensure!(
        !grant.is_empty() && grant.len() <= 8192,
        "invalid bounded transfer grant"
    );
    let addresses = admitted
        .receiver_direct_addresses
        .context("observed direct receiver unavailable")?
        .into_iter()
        .map(|address| address.parse())
        .collect::<Result<Vec<_>, _>>()?;
    let remaining = (admitted.expires_at.min(admitted.observation_valid_until)
        - chrono::Utc::now())
    .to_std()
    .context("receiver admission expired before transfer")?;
    ensure!(
        !remaining.is_zero(),
        "receiver admission expired before transfer"
    );
    provider
        .publish(
            TransferTarget {
                endpoint_id: admitted.receiver_endpoint_id,
                direct_addresses: addresses,
                operation_id,
                session_id: admitted.receiver_session_id,
                manifest_blake3: verified.manifest_blake3().into(),
            },
            grant,
            tokio::time::Instant::now() + remaining,
        )
        .await?;
    // Keep the frozen tree and all its owner handles through actual publication.
    drop((root, frozen));
    Ok(())
}

async fn admit(
    client: &ApiClient,
    deployment: &str,
    body: &nrz_api::AdmitRequestBody,
) -> anyhow::Result<nrz_api::Admit200Response> {
    let start = Instant::now();
    let mut delay = SOURCE_REGISTRATION_INITIAL_RETRY_DELAY;
    loop {
        let remaining = SOURCE_REGISTRATION_RETRY_BUDGET.saturating_sub(start.elapsed());
        ensure!(
            !remaining.is_zero(),
            "Source V2 admission deadline; original operation may remain pending"
        );
        let result = tokio::time::timeout(
            SOURCE_REGISTRATION_REQUEST_TIMEOUT.min(remaining),
            client.admit_static_source_v2(deployment, body.clone()),
        )
        .await;
        match result {
            Ok(Ok(response)) => return Ok(response),
            Ok(Err(error)) => {
                let Some(retry) = classify_api_retry(&error) else {
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

fn digest(input: &str) -> anyhow::Result<[u8; 32]> {
    ensure!(input.len() == 64, "invalid BLAKE3 digest");
    let mut digest = [0u8; 32];
    for (output, pair) in digest.iter_mut().zip(input.as_bytes().as_chunks::<2>().0) {
        let pair = std::str::from_utf8(pair)?;
        *output = u8::from_str_radix(pair, 16)?;
    }
    Ok(digest)
}
