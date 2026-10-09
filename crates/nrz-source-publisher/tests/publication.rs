use std::collections::VecDeque;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Mutex;

use nrz_api::{
    MultipartComplete200Response as CliMultipartCompleteResponse,
    MultipartCompleteRequestBody as CliMultipartCompleteRequest,
    PrepareUpload200Response as CliPrepareUploadResponse,
    PrepareUploadRequestBody as CliPrepareUploadRequest,
    UploadComplete200Response as CliUploadCompleteResponse,
    UploadCompleteRequestBody as CliUploadCompleteRequest,
    UploadFailed200Response as CliUploadFailedResponse,
    UploadFailedRequestBody as CliUploadFailedRequest,
};
use nrz_runtime_artifact::{finalize_runtime_artifact_graph, finalize_source_bundle_runtime_graph};
use nrz_source_bundle::{
    SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH, SOURCE_BUNDLE_V1_SCHEMA_VERSION,
    SourceBundleVerificationBudget, SourceLogicalManifest, SourceLogicalManifestEntryType,
    SourceLogicalManifestFile, SourceLogicalManifestLayer, canonical_source_logical_manifest_json,
    compute_logical_manifest_sha256, sha256_hex,
};
use nrz_source_publisher::{
    DeploymentPublicationStatus, ObjectUploadRequest, ObjectUploadResult, PreparedSourceBundle,
    PublicationEvent, PublicationObserver, SourceBundleInput, SourcePublicationError,
    SourcePublicationRequest, SourcePublicationTransport, publish_source_bundle,
    publish_source_bundle_upload, source_uses_multipart,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn multipart_admission_uses_the_protocol_byte_threshold() {
    assert!(!source_uses_multipart(268_435_455));
    assert!(source_uses_multipart(268_435_456));
    assert!(source_uses_multipart(268_435_457));
}

#[tokio::test]
async fn rejects_source_growth_after_verification_before_upload() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    std::fs::OpenOptions::new()
        .append(true)
        .open(bundle.path())
        .unwrap()
        .write_all(b"changed")
        .unwrap();
    let transport = FakeTransport::new(
        vec![single_prepare_response(&bundle, Uuid::now_v7(), "grown")],
        vec![],
        vec![],
        vec![],
    );
    let observer = RecordingObserver::default();
    let result = publish_source_bundle_upload(request(
        &transport,
        &observer,
        Uuid::now_v7(),
        workspace_id,
        &bundle,
    ))
    .await;
    assert!(
        matches!(result, Err(SourcePublicationError::InvalidSourceBundle(_))),
        "{result:?}"
    );
    let state = transport.state.lock().unwrap();
    assert!(state.put_requests.is_empty());
    assert!(state.complete_requests.is_empty());
    assert_eq!(state.failed_requests.len(), 1);
}

#[tokio::test]
async fn rejects_upload_metadata_that_rebinds_the_verified_source() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    let replacement = vec![b'x'; bundle.source_size_bytes() as usize];
    std::fs::write(bundle.path(), &replacement).unwrap();
    let upload_session_id = Uuid::now_v7();
    let deployment_id = Uuid::now_v7();
    let mut prepared = single_prepare_response(&bundle, upload_session_id, "replacement");
    let target = prepared.presigned_put.as_mut().unwrap();
    target.sha256 = sha256_hex(&replacement);
    target.verify_head.as_mut().unwrap().sha256 = target.sha256.clone();
    let transport = FakeTransport::new(
        vec![prepared],
        vec![CliUploadCompleteResponse::Object(
            nrz_api::UploadComplete200ResponseObject {
                deployment_id,
                upload_session_id,
                ..Default::default()
            },
        )],
        vec![PutOutcome::Success],
        vec![],
    );
    let observer = RecordingObserver::default();
    let result = publish_source_bundle_upload(request(
        &transport,
        &observer,
        deployment_id,
        workspace_id,
        &bundle,
    ))
    .await;
    assert!(
        matches!(result, Err(SourcePublicationError::InvalidResponse(_))),
        "{result:?}"
    );
    let state = transport.state.lock().unwrap();
    assert!(state.put_requests.is_empty());
    assert!(state.complete_requests.is_empty());
}

#[tokio::test]
async fn rejects_inconsistent_single_upload_targets_before_sending_bytes() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    for (label, pointer, value) in [
        (
            "size",
            "/presignedPut/contentLength",
            json!(bundle.source_size_bytes() + 1),
        ),
        ("mode", "/presignedPut/mode", json!("single")),
        ("hash", "/presignedPut/sha256", json!(sha256_hex(b"other"))),
        (
            "HEAD size",
            "/presignedPut/verifyHead/contentLength",
            json!(0),
        ),
        (
            "HEAD hash",
            "/presignedPut/verifyHead/sha256",
            json!(sha256_hex(b"other")),
        ),
        (
            "completion",
            "/requiredComplete",
            json!("multipart-complete+upload-complete"),
        ),
        ("missing target", "/presignedPut", serde_json::Value::Null),
        ("fast path target", "/fastPath", json!(true)),
        (
            "fast path completion",
            "/requiredComplete",
            json!("multipart-complete+upload-complete"),
        ),
    ] {
        let mut response =
            serde_json::to_value(single_prepare_response(&bundle, Uuid::now_v7(), label)).unwrap();
        *response.pointer_mut(pointer).unwrap() = value.clone();
        if pointer == "/presignedPut/contentLength" {
            *response
                .pointer_mut("/presignedPut/verifyHead/contentLength")
                .unwrap() = value;
        }
        let mut response: CliPrepareUploadResponse = serde_json::from_value(response).unwrap();
        if label == "mode" {
            response.presigned_put.as_mut().unwrap().mode = "multipart".into();
        } else if label == "fast path completion" {
            response.fast_path = true;
            response.presigned_put = None;
        }
        assert_rejected_upload_response(workspace_id, &bundle, response, label).await;
    }
}

async fn assert_rejected_upload_response(
    workspace_id: Uuid,
    bundle: &PreparedSourceBundle,
    response: CliPrepareUploadResponse,
    label: &str,
) {
    let transport = FakeTransport::new(vec![response], vec![], vec![], vec![]);
    let observer = RecordingObserver::default();
    let result = publish_source_bundle_upload(request(
        &transport,
        &observer,
        Uuid::now_v7(),
        workspace_id,
        bundle,
    ))
    .await;
    assert!(
        matches!(result, Err(SourcePublicationError::InvalidResponse(_))),
        "{label}: {result:?}"
    );
    if label == "fast path completion" {
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("fast path returned upload targets or multipart completion")
        );
    }
    let state = transport.state.lock().unwrap();
    assert!(state.put_requests.is_empty(), "{label}");
    assert!(state.complete_requests.is_empty(), "{label}");
}

#[tokio::test]
async fn prepare_summary_includes_prerender_and_config_file_sizes() {
    for role in ["prerender", "config"] {
        let workspace_id = Uuid::now_v7();
        let (_temp, bundle) = prepared_bundle_with_static_role(workspace_id, role, None).await;
        let upload_session_id = Uuid::now_v7();
        let deployment_id = Uuid::now_v7();
        let transport = FakeTransport::new(
            vec![single_prepare_response(
                &bundle,
                upload_session_id,
                "role-summary",
            )],
            vec![CliUploadCompleteResponse::Object(
                nrz_api::UploadComplete200ResponseObject {
                    deployment_id,
                    upload_session_id,
                    ..Default::default()
                },
            )],
            vec![PutOutcome::Success],
            vec![],
        );
        let observer = RecordingObserver::default();
        publish_source_bundle_upload(request(
            &transport,
            &observer,
            deployment_id,
            workspace_id,
            &bundle,
        ))
        .await
        .unwrap();
        let state = transport.state.lock().unwrap();
        assert_eq!(
            state.prepare_requests[0]
                .logical_manifest_summary
                .max_static_file_size_bytes
                .as_str(),
            "6",
            "{role}"
        );
    }
}

#[tokio::test]
async fn publishes_verified_bundle_and_requires_durable_graph_readback() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    let deployment_id = Uuid::now_v7();
    let upload_session_id = Uuid::now_v7();
    let transport = FakeTransport::new(
        vec![single_prepare_response(&bundle, upload_session_id, "first")],
        vec![CliUploadCompleteResponse::Object3(
            nrz_api::UploadComplete200ResponseObject3 {
                deployment_id,
                upload_session_id,
                ..Default::default()
            },
        )],
        vec![PutOutcome::Success],
        vec![durable_status(&bundle)],
    );
    let observer = RecordingObserver::default();

    let published = publish_source_bundle(request(
        &transport,
        &observer,
        deployment_id,
        workspace_id,
        &bundle,
    ))
    .await
    .unwrap();

    let expected_id = sha256_hex(format!(
        "{workspace_id}\0{}\0{}\0SOURCE_BUNDLE_V1.0",
        bundle.logical_manifest_sha256(),
        bundle.source_sha256()
    ));
    assert_eq!(bundle.source_artifact_id(), expected_id);
    assert_eq!(published.source_artifact_id, expected_id);
    assert_eq!(
        published.runtime_artifact_graph_digest,
        published.runtime_artifact_graph.graph_digest()
    );
    let state = transport.state.lock().unwrap();
    assert_eq!(state.prepare_requests.len(), 1);
    assert_eq!(state.put_requests.len(), 1);
    assert_eq!(
        sha256_hex(&state.put_requests[0].bytes),
        bundle.source_sha256()
    );
    assert_eq!(state.complete_requests.len(), 1);
    assert!(state.failed_requests.is_empty());
    assert_eq!(
        state.complete_requests[0].source_artifact_id.as_str(),
        bundle.source_artifact_id()
    );
    assert_eq!(
        observer.events.lock().unwrap().as_slice(),
        [
            PublicationEvent::Preparing,
            PublicationEvent::Uploading,
            PublicationEvent::CompletingUpload,
            PublicationEvent::AwaitingDurableReadback,
            PublicationEvent::DurableVerified,
        ]
    );
}

#[tokio::test]
async fn publishes_source_upload_without_waiting_for_a_runtime_graph() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    let deployment_id = Uuid::now_v7();
    let upload_session_id = Uuid::now_v7();
    let transport = FakeTransport::new(
        vec![single_prepare_response(
            &bundle,
            upload_session_id,
            "source-only",
        )],
        vec![CliUploadCompleteResponse::Object(
            nrz_api::UploadComplete200ResponseObject {
                deployment_id,
                upload_session_id,
                ..Default::default()
            },
        )],
        vec![PutOutcome::Success],
        vec![],
    );
    let observer = RecordingObserver::default();

    let published = publish_source_bundle_upload(request(
        &transport,
        &observer,
        deployment_id,
        workspace_id,
        &bundle,
    ))
    .await
    .unwrap();

    assert_eq!(published.source_artifact_id, bundle.source_artifact_id());
    assert_eq!(
        observer.events.lock().unwrap().as_slice(),
        [
            PublicationEvent::Preparing,
            PublicationEvent::Uploading,
            PublicationEvent::CompletingUpload,
        ]
    );
    let state = transport.state.lock().unwrap();
    assert_eq!(state.complete_requests.len(), 1);
    assert!(state.statuses.is_empty());
}

#[tokio::test]
async fn publishes_all_verified_multipart_bytes_before_both_completions() {
    const CHUNK_SIZE: usize = 16 * 1024 * 1024;
    const SOURCE_SIZE: u64 = 256 * 1024 * 1024 + 4;
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) =
        prepared_bundle_with_static_role(workspace_id, "static", Some(SOURCE_SIZE)).await;
    let wrong_single = FakeTransport::new(
        vec![single_prepare_response(
            &bundle,
            Uuid::now_v7(),
            "wrong-single",
        )],
        vec![],
        vec![],
        vec![],
    );
    let observer = RecordingObserver::default();
    let result = publish_source_bundle_upload(request(
        &wrong_single,
        &observer,
        Uuid::now_v7(),
        workspace_id,
        &bundle,
    ))
    .await;
    assert!(
        matches!(result, Err(SourcePublicationError::InvalidResponse(_))),
        "{result:?}"
    );
    assert!(wrong_single.state.lock().unwrap().put_requests.is_empty());
    let deployment_id = Uuid::now_v7();
    let upload_session_id = Uuid::now_v7();
    let transport = FakeTransport::new(
        vec![single_prepare_response(
            &bundle,
            upload_session_id,
            "multipart",
        )],
        vec![CliUploadCompleteResponse::Object(
            nrz_api::UploadComplete200ResponseObject {
                deployment_id,
                upload_session_id,
                ..Default::default()
            },
        )],
        (0..17).map(|_| PutOutcome::Success).collect(),
        vec![],
    );
    transport.state.lock().unwrap().multipart_mode = true;
    let observer = RecordingObserver::default();
    let publication = request(&transport, &observer, deployment_id, workspace_id, &bundle);
    let attempt_id = publication.deployment_attempt_id;
    let published = publish_source_bundle_upload(publication).await.unwrap();
    assert_eq!(published.source_artifact_id, bundle.source_artifact_id());
    assert_eq!(bundle.source_size_bytes(), SOURCE_SIZE);

    let state = transport.state.lock().unwrap();
    assert_eq!(state.prepare_requests.len(), 1);
    let prepare = &state.prepare_requests[0];
    let inventory = prepare.multipart.as_ref().unwrap();
    assert_eq!(inventory.part_size_bytes, CHUNK_SIZE as i64);
    assert_eq!(inventory.part_count, 17);
    assert_eq!(inventory.parts.len(), 17);
    assert_eq!(state.put_requests.len(), 17);
    let mut source = std::fs::File::open(bundle.path()).unwrap();
    let mut expected_bytes = vec![0_u8; CHUNK_SIZE];
    let mut reconstructed = Sha256::new();
    for (index, part) in inventory.parts.iter().enumerate() {
        let size = if index == 16 { 4 } else { CHUNK_SIZE };
        assert_eq!(part.part_number, (index + 1) as i64);
        assert_eq!(part.size_bytes, size as i64);
        // Targets run backwards; reconstruction follows the provider's canonical part order.
        let upload = &state.put_requests[16 - index];
        assert_eq!(
            upload.url,
            format!("https://objects.invalid/part/{}", index + 1)
        );
        assert_eq!(upload.bytes.len(), size);
        assert_eq!(upload.sha256, part.sha256);
        assert_eq!(sha256_hex(&upload.bytes), part.sha256);
        assert!(upload.verify_head.is_none());
        assert!(upload.headers.content_type.is_none());
        assert!(upload.headers.if_none_match.is_none());
        source.read_exact(&mut expected_bytes[..size]).unwrap();
        assert_eq!(upload.bytes.as_ref(), &expected_bytes[..size]);
        if index > 0 {
            assert_eq!(upload.bytes[0], index as u8);
        }
        reconstructed.update(&upload.bytes);
    }
    assert_eq!(source.stream_position().unwrap(), SOURCE_SIZE);
    assert_eq!(source.read(&mut expected_bytes[..1]).unwrap(), 0);
    assert_eq!(
        hex::encode(reconstructed.finalize()),
        bundle.source_sha256()
    );

    assert_eq!(state.multipart_requests.len(), 1);
    let multipart = &state.multipart_requests[0];
    assert_eq!(multipart.deployment_id, deployment_id);
    assert_eq!(multipart.deployment_attempt_id, attempt_id);
    assert_eq!(multipart.upload_session_id, upload_session_id);
    assert_eq!(
        multipart.source_artifact_id.as_str(),
        bundle.source_artifact_id()
    );
    assert_eq!(multipart.upload_id.as_str(), "fixture-multipart-upload");
    assert_eq!(multipart.parts.len(), 17);
    for (index, part) in multipart.parts.iter().enumerate() {
        let number = 17 - index;
        assert_eq!(part.part_number, number as i64);
        assert_eq!(part.e_tag.as_str(), format!("etag-{number}"));
    }
    assert_eq!(state.complete_requests.len(), 1);
    let complete = &state.complete_requests[0];
    assert_eq!(complete.deployment_id, deployment_id);
    assert_eq!(complete.deployment_attempt_id, attempt_id);
    assert_eq!(complete.upload_session_id, upload_session_id);
    assert_eq!(complete.source_artifact_id, multipart.source_artifact_id);
    assert_eq!(complete.source_sha256.as_str(), bundle.source_sha256());
    assert_eq!(complete.source_size_bytes.as_str(), SOURCE_SIZE.to_string());
    assert_eq!(
        complete.logical_manifest_sha256.as_str(),
        bundle.logical_manifest_sha256()
    );
    assert_eq!(
        observer.events.lock().unwrap().as_slice(),
        [
            PublicationEvent::Preparing,
            PublicationEvent::Uploading,
            PublicationEvent::CompletingMultipart,
            PublicationEvent::CompletingUpload,
        ]
    );
}

#[tokio::test]
async fn recovers_conditional_conflict_through_the_same_prepare_state_machine() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    let deployment_id = Uuid::now_v7();
    let first_session_id = Uuid::now_v7();
    let recovered_session_id = Uuid::now_v7();
    let transport = FakeTransport::new(
        vec![
            single_prepare_response(&bundle, first_session_id, "conflict"),
            single_prepare_response(&bundle, recovered_session_id, "recovered"),
        ],
        vec![CliUploadCompleteResponse::Object(
            nrz_api::UploadComplete200ResponseObject {
                deployment_id,
                upload_session_id: recovered_session_id,
                ..Default::default()
            },
        )],
        vec![PutOutcome::Conflict, PutOutcome::Success],
        vec![durable_status(&bundle)],
    );
    let observer = RecordingObserver::default();

    publish_source_bundle(request(
        &transport,
        &observer,
        deployment_id,
        workspace_id,
        &bundle,
    ))
    .await
    .unwrap();

    let state = transport.state.lock().unwrap();
    assert_eq!(state.prepare_requests.len(), 2);
    let recovery = state.prepare_requests[1]
        .source_upload_recovery
        .as_ref()
        .unwrap();
    assert_eq!(recovery.failed_upload_session_id, first_session_id);
    assert_eq!(recovery.reason, "conditional-precondition-failed");
    assert_eq!(state.put_requests.len(), 2);
}

#[tokio::test]
async fn rejects_a_durable_runtime_graph_for_a_different_source_bundle() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    let deployment_id = Uuid::now_v7();
    let upload_session_id = Uuid::now_v7();
    let other_graph = finalize_source_bundle_runtime_graph(
        bundle.logical_manifest_sha256(),
        &"f".repeat(64),
        bundle.source_size_bytes(),
        bundle.manifest(),
    )
    .unwrap();
    let other_graph_digest = other_graph.graph_digest().to_string();
    let transport = FakeTransport::new(
        vec![fast_path_prepare_response(&bundle, upload_session_id)],
        vec![CliUploadCompleteResponse::Object2(
            nrz_api::UploadComplete200ResponseObject2 {
                deployment_id,
                upload_session_id,
                ..Default::default()
            },
        )],
        vec![],
        vec![DeploymentPublicationStatus {
            status: "SMOKE_TESTING".to_string(),
            runtime_artifact_graph_digest: Some(other_graph_digest),
            runtime_artifact_graph: Some(other_graph.into_wire()),
            error: None,
            error_code: None,
        }],
    );
    let observer = RecordingObserver::default();

    let error = publish_source_bundle(request(
        &transport,
        &observer,
        deployment_id,
        workspace_id,
        &bundle,
    ))
    .await
    .unwrap_err();

    assert!(matches!(error, SourcePublicationError::InvalidResponse(_)));
    assert!(
        error
            .to_string()
            .contains("does not match the verified source bundle")
    );
}

#[tokio::test]
async fn rejects_each_durable_application_identity_mismatch_independently() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    let status = durable_status(&bundle);
    for (pointer, value) in [
        ("/application/artifactId", json!("a".repeat(64))),
        ("/application/manifestDigest", json!("b".repeat(64))),
        (
            "/application/blobDescriptor/digest",
            json!(format!("sha256:{}", "c".repeat(64))),
        ),
        (
            "/application/blobDescriptor/size",
            json!(bundle.source_size_bytes() + 1),
        ),
    ] {
        let mut wire =
            serde_json::to_value(status.runtime_artifact_graph.as_ref().unwrap()).unwrap();
        *wire.pointer_mut(pointer).unwrap() = value;
        // A valid canonical graph isolates source binding from graph-digest validation.
        let graph = finalize_runtime_artifact_graph(wire, &[]).unwrap();
        let transport = completed_fast_path(
            &bundle,
            vec![DeploymentPublicationStatus {
                runtime_artifact_graph_digest: Some(graph.graph_digest().into()),
                runtime_artifact_graph: Some(graph.into_wire()),
                ..status.clone()
            }],
        );
        let observer = RecordingObserver::default();
        let result = publish_source_bundle(request(
            &transport,
            &observer,
            Uuid::now_v7(),
            workspace_id,
            &bundle,
        ))
        .await;
        assert!(
            matches!(result, Err(SourcePublicationError::InvalidResponse(_))),
            "{pointer}: {result:?}"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not match the verified source bundle"),
            "{pointer}"
        );
    }
}

#[tokio::test]
async fn rejects_durable_dependency_mount_over_an_application_file() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    let status = durable_status(&bundle);
    let mut wire = serde_json::to_value(status.runtime_artifact_graph.as_ref().unwrap()).unwrap();
    let id = "d".repeat(64);
    wire["dependencies"] = json!([{
        "materializationId": id,
        "kind": "JAVASCRIPT_NODE_MODULES", "mountPoint": "/output/server",
        "compatibility": {
            "runtimeFamily": "bun", "runtimeVersion": "1.4.2", "os": "linux",
            "architecture": "x86_64", "libc": "glibc", "abi": "glibc-2.42",
            "packageManager": "bun", "packageManagerVersion": "1.4.2",
            "runnerRootfsDigest": format!("sha256:{id}"), "buildPolicyGeneration": 1
        },
        "manifestDigest": id,
        "blobDescriptor": { "mediaType": "application/vnd.onreza.dependency.erofs.v1",
            "digest": format!("sha256:{id}"), "size": 4096 }
    }]);
    wire["runtimeLayers"][0]["dependencyMaterializationIds"] = json!([id]);
    let graph = finalize_runtime_artifact_graph(wire, &[]).unwrap();
    let transport = completed_fast_path(
        &bundle,
        vec![DeploymentPublicationStatus {
            runtime_artifact_graph_digest: Some(graph.graph_digest().into()),
            runtime_artifact_graph: Some(graph.into_wire()),
            ..status
        }],
    );
    let observer = RecordingObserver::default();
    let result = publish_source_bundle(request(
        &transport,
        &observer,
        Uuid::now_v7(),
        workspace_id,
        &bundle,
    ))
    .await;
    assert!(
        matches!(result, Err(SourcePublicationError::InvalidResponse(_))),
        "{result:?}"
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("collides with dependency mount")
    );
}

#[tokio::test]
async fn durable_readback_retries_transient_errors_and_pending_statuses() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    let transport = completed_fast_path(&bundle, vec![pending_status(), durable_status(&bundle)]);
    transport.state.lock().unwrap().statuses.push_front(Err(
        SourcePublicationError::AmbiguousTransport("connection reset".into()),
    ));
    let observer = RecordingObserver::default();
    tokio::time::pause();
    let started = tokio::time::Instant::now();
    let published = publish_source_bundle(request(
        &transport,
        &observer,
        Uuid::now_v7(),
        workspace_id,
        &bundle,
    ))
    .await
    .unwrap();
    assert_eq!(
        published.runtime_artifact_graph_digest,
        durable_status(&bundle)
            .runtime_artifact_graph_digest
            .unwrap()
    );
    assert!(
        (std::time::Duration::from_millis(1500)..=std::time::Duration::from_millis(1505))
            .contains(&started.elapsed())
    );
    assert_eq!(
        observer
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| matches!(
                event,
                PublicationEvent::Waiting {
                    operation: "durable-readback"
                }
            ))
            .count(),
        2
    );
}

#[tokio::test]
async fn durable_readback_preserves_terminal_errors_without_retry() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    let transport = completed_fast_path(&bundle, vec![durable_status(&bundle)]);
    transport.state.lock().unwrap().statuses.push_front(Err(
        SourcePublicationError::InvalidResponse("unauthorized readback".into()),
    ));
    let observer = RecordingObserver::default();
    tokio::time::pause();
    let started = tokio::time::Instant::now();
    let result = publish_source_bundle(request(
        &transport,
        &observer,
        Uuid::now_v7(),
        workspace_id,
        &bundle,
    ))
    .await;
    assert!(
        matches!(result, Err(SourcePublicationError::InvalidResponse(ref message)) if message == "unauthorized readback")
    );
    assert_eq!(started.elapsed(), std::time::Duration::ZERO);
    assert!(
        !observer
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|event| matches!(event, PublicationEvent::Waiting { .. }))
    );
}

#[tokio::test]
async fn durable_readback_bounds_pending_and_unavailable_statuses_at_thirty_minutes() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    tokio::time::pause();
    for unavailable in [false, true] {
        let transport = completed_fast_path(&bundle, vec![]);
        transport.state.lock().unwrap().statuses = (0..365)
            .map(|_| {
                if unavailable {
                    Err(SourcePublicationError::AmbiguousTransport(
                        "unavailable".into(),
                    ))
                } else {
                    Ok(pending_status())
                }
            })
            .collect();
        let observer = RecordingObserver::default();
        let started = tokio::time::Instant::now();
        let result = publish_source_bundle(request(
            &transport,
            &observer,
            Uuid::now_v7(),
            workspace_id,
            &bundle,
        ))
        .await;
        assert!(
            matches!(result, Err(SourcePublicationError::Deadline(_))),
            "{result:?}"
        );
        assert!(
            (std::time::Duration::from_secs(1800)..=std::time::Duration::from_millis(1_800_005))
                .contains(&started.elapsed()),
            "{:?}",
            started.elapsed()
        );
    }
}

#[tokio::test]
async fn prepare_retries_transient_transport_and_server_errors_but_preserves_terminal_rejections() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    tokio::time::pause();
    let transport = completed_fast_path(&bundle, vec![]);
    transport
        .state
        .lock()
        .unwrap()
        .prepare_responses
        .push_front(Err(SourcePublicationError::AmbiguousTransport(
            "connection reset".into(),
        )));
    let observer = RecordingObserver::default();
    let published = publish_source_bundle_upload(request(
        &transport,
        &observer,
        Uuid::now_v7(),
        workspace_id,
        &bundle,
    ))
    .await
    .unwrap();
    assert_eq!(published.source_artifact_id, bundle.source_artifact_id());
    assert_eq!(transport.state.lock().unwrap().prepare_requests.len(), 2);
    for (status, code, retryable) in [
        (503, "HTTP_503", true),
        (400, "VALIDATION_ERROR", false),
        (409, "OPERATION_IN_PROGRESS", true),
        (401, "UNAUTHORIZED", false),
    ] {
        let transport = completed_fast_path(&bundle, vec![]);
        transport
            .state
            .lock()
            .unwrap()
            .prepare_responses
            .push_front(Err(nrz_source_publisher::StructuredControlPlaneError {
                status,
                code: code.into(),
                message: code.into(),
                retry_after: None,
                details: None,
            }
            .into()));
        let started = tokio::time::Instant::now();
        let result = publish_source_bundle_upload(request(
            &transport,
            &observer,
            Uuid::now_v7(),
            workspace_id,
            &bundle,
        ))
        .await;
        if retryable {
            assert!(result.is_ok(), "{status}: {result:?}");
            assert_eq!(transport.state.lock().unwrap().prepare_requests.len(), 2);
            assert!(started.elapsed() >= std::time::Duration::from_millis(500));
        } else {
            assert!(
                matches!(result, Err(SourcePublicationError::ControlPlane(ref error)) if error.status == status && error.code == code)
            );
            assert_eq!(transport.state.lock().unwrap().prepare_requests.len(), 1);
            assert_eq!(started.elapsed(), std::time::Duration::ZERO);
        }
    }
}

#[tokio::test]
async fn reports_a_bounded_utf8_upload_failure_with_redacted_object_urls() {
    let workspace_id = Uuid::now_v7();
    let (_temp, bundle) = prepared_bundle(workspace_id).await;
    let message = format!(
        "https://objects.invalid/upload?token=fixture-secret {}",
        "€".repeat(1500)
    );
    let transport = FakeTransport::new(
        vec![single_prepare_response(&bundle, Uuid::now_v7(), "failed")],
        vec![],
        vec![PutOutcome::Failure(message.clone())],
        vec![],
    );
    let observer = RecordingObserver::default();
    let result = publish_source_bundle_upload(request(
        &transport,
        &observer,
        Uuid::now_v7(),
        workspace_id,
        &bundle,
    ))
    .await;
    assert!(
        matches!(result, Err(SourcePublicationError::ObjectUpload(ref actual)) if actual == &message)
    );
    let state = transport.state.lock().unwrap();
    assert_eq!(state.failed_requests.len(), 1);
    assert!(state.complete_requests.is_empty());
    let report = &state.failed_requests[0];
    assert_eq!(report.error_code, "SOURCE_UPLOAD_PUT_FAILED");
    let mut expected = format!(
        "source object upload failed: https://objects.invalid/upload?REDACTED {}",
        "€".repeat(1500)
    );
    let end = expected
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= 4096)
        .last()
        .unwrap();
    expected.truncate(end);
    assert_eq!(report.error_log, expected);
}

fn pending_status() -> DeploymentPublicationStatus {
    DeploymentPublicationStatus {
        status: "BUILDING".into(),
        runtime_artifact_graph_digest: None,
        runtime_artifact_graph: None,
        error: None,
        error_code: None,
    }
}

fn completed_fast_path(
    bundle: &PreparedSourceBundle,
    statuses: Vec<DeploymentPublicationStatus>,
) -> FakeTransport {
    let upload_session_id = Uuid::now_v7();
    FakeTransport::new(
        vec![fast_path_prepare_response(bundle, upload_session_id)],
        vec![CliUploadCompleteResponse::Object(
            nrz_api::UploadComplete200ResponseObject {
                upload_session_id,
                ..Default::default()
            },
        )],
        vec![],
        statuses,
    )
}

fn durable_status(bundle: &PreparedSourceBundle) -> DeploymentPublicationStatus {
    let graph = finalize_source_bundle_runtime_graph(
        bundle.logical_manifest_sha256(),
        bundle.source_sha256(),
        bundle.source_size_bytes(),
        bundle.manifest(),
    )
    .unwrap();
    DeploymentPublicationStatus {
        status: "SMOKE_TESTING".to_string(),
        runtime_artifact_graph_digest: Some(graph.graph_digest().to_string()),
        runtime_artifact_graph: Some(graph.into_wire()),
        error: None,
        error_code: None,
    }
}

fn request<'a>(
    transport: &'a FakeTransport,
    observer: &'a RecordingObserver,
    deployment_id: Uuid,
    workspace_id: Uuid,
    bundle: &'a PreparedSourceBundle,
) -> SourcePublicationRequest<'a, FakeTransport, RecordingObserver> {
    SourcePublicationRequest {
        transport,
        observer,
        deployment_id,
        workspace_id,
        project_id: Uuid::now_v7(),
        deployment_attempt_id: Uuid::now_v7(),
        bundle,
    }
}

#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<PublicationEvent>>,
}

impl PublicationObserver for RecordingObserver {
    fn on_event(&self, event: PublicationEvent) {
        self.events.lock().unwrap().push(event);
    }
}

enum PutOutcome {
    Success,
    Conflict,
    Failure(String),
}

struct FakeState {
    multipart_mode: bool,
    multipart_requests: Vec<CliMultipartCompleteRequest>,
    prepare_responses: VecDeque<Result<CliPrepareUploadResponse, SourcePublicationError>>,
    complete_responses: VecDeque<CliUploadCompleteResponse>,
    put_outcomes: VecDeque<PutOutcome>,
    statuses: VecDeque<Result<DeploymentPublicationStatus, SourcePublicationError>>,
    status_delay: std::time::Duration,
    failed_requests: Vec<CliUploadFailedRequest>,
    prepare_requests: Vec<CliPrepareUploadRequest>,
    complete_requests: Vec<CliUploadCompleteRequest>,
    put_requests: Vec<ObjectUploadRequest>,
}

struct FakeTransport {
    state: Mutex<FakeState>,
}

impl FakeTransport {
    fn new(
        prepare_responses: Vec<CliPrepareUploadResponse>,
        complete_responses: Vec<CliUploadCompleteResponse>,
        put_outcomes: Vec<PutOutcome>,
        statuses: Vec<DeploymentPublicationStatus>,
    ) -> Self {
        Self {
            state: Mutex::new(FakeState {
                multipart_mode: false,
                multipart_requests: Vec::new(),
                prepare_responses: prepare_responses.into_iter().map(Ok).collect(),
                complete_responses: complete_responses.into(),
                put_outcomes: put_outcomes.into(),
                statuses: statuses.into_iter().map(Ok).collect(),
                status_delay: std::time::Duration::ZERO,
                failed_requests: Vec::new(),
                prepare_requests: Vec::new(),
                complete_requests: Vec::new(),
                put_requests: Vec::new(),
            }),
        }
    }
}

impl SourcePublicationTransport for FakeTransport {
    async fn prepare_upload(
        &self,
        _deployment_id: Uuid,
        request: &CliPrepareUploadRequest,
    ) -> Result<CliPrepareUploadResponse, SourcePublicationError> {
        let mut state = self.state.lock().unwrap();
        state.prepare_requests.push(request.clone());
        let mut response = state.prepare_responses.pop_front().unwrap()?;
        if state.multipart_mode {
            let inventory = request.multipart.as_ref().unwrap();
            response.presigned_put = None;
            response.required_complete =
                nrz_api::PrepareUpload200ResponseRequiredComplete::MultipartCompleteUploadComplete;
            response.multipart = Some(nrz_api::PrepareUpload200ResponseMultipart {
                mode: "multipart".to_string(),
                upload_id: "fixture-multipart-upload".to_string(),
                chunk_size: inventory.part_size_bytes,
                chunks: inventory
                    .parts
                    .iter()
                    .rev()
                    .map(|part| nrz_api::PrepareUpload200ResponseMultipartChunk {
                        part_number: part.part_number,
                        url: format!("https://objects.invalid/part/{}", part.part_number),
                        content_length: part.size_bytes,
                        sha256: part.sha256.clone(),
                    })
                    .collect(),
            });
        }
        Ok(response)
    }

    async fn complete_multipart(
        &self,
        _deployment_id: Uuid,
        request: &CliMultipartCompleteRequest,
    ) -> Result<CliMultipartCompleteResponse, SourcePublicationError> {
        let mut state = self.state.lock().unwrap();
        assert!(
            state.multipart_mode,
            "single-part fixture must not complete multipart upload"
        );
        assert_eq!(state.put_requests.len(), request.parts.len());
        assert!(state.complete_requests.is_empty());
        state.multipart_requests.push(request.clone());
        Ok(CliMultipartCompleteResponse::Object(
            nrz_api::MultipartComplete200ResponseObject {
                deployment_id: request.deployment_id,
                upload_session_id: request.upload_session_id,
                completed_targets: 1,
                ..Default::default()
            },
        ))
    }

    async fn complete_upload(
        &self,
        _deployment_id: Uuid,
        request: &CliUploadCompleteRequest,
    ) -> Result<CliUploadCompleteResponse, SourcePublicationError> {
        let mut state = self.state.lock().unwrap();
        if state.multipart_mode {
            assert_eq!(state.multipart_requests.len(), 1);
        }
        state.complete_requests.push(request.clone());
        Ok(state.complete_responses.pop_front().unwrap())
    }

    async fn report_upload_failed(
        &self,
        _deployment_id: Uuid,
        request: &CliUploadFailedRequest,
    ) -> Result<CliUploadFailedResponse, SourcePublicationError> {
        self.state
            .lock()
            .unwrap()
            .failed_requests
            .push(request.clone());
        Ok(CliUploadFailedResponse::Object(
            nrz_api::UploadFailed200ResponseObject {
                deployment_id: request.deployment_id,
                upload_session_id: request.upload_session_id,
                ..Default::default()
            },
        ))
    }

    async fn deployment_status(
        &self,
        _deployment_id: Uuid,
    ) -> Result<DeploymentPublicationStatus, SourcePublicationError> {
        let delay = self.state.lock().unwrap().status_delay;
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        self.state.lock().unwrap().statuses.pop_front().unwrap()
    }

    async fn put_object(
        &self,
        request: ObjectUploadRequest,
    ) -> Result<ObjectUploadResult, SourcePublicationError> {
        let mut state = self.state.lock().unwrap();
        let e_tag = state
            .multipart_mode
            .then(|| format!("etag-{}", request.url.rsplit('/').next().unwrap()));
        state.put_requests.push(request);
        match state.put_outcomes.pop_front().unwrap() {
            PutOutcome::Success => Ok(ObjectUploadResult { e_tag }),
            PutOutcome::Failure(message) => Err(SourcePublicationError::ObjectUpload(message)),
            PutOutcome::Conflict => Err(SourcePublicationError::ConditionalUploadConflict(
                "fixture conflict".to_string(),
            )),
        }
    }
}

fn single_prepare_response(
    bundle: &PreparedSourceBundle,
    upload_session_id: Uuid,
    key: &str,
) -> CliPrepareUploadResponse {
    serde_json::from_value(json!({
        "bucket": "fixture",
        "expiresAt": "2026-09-01T12:00:00Z",
        "fastPath": false,
        "kind": "source-upload",
        "presignedPut": {
            "contentLength": bundle.source_size_bytes(),
            "headers": {
                "content-type": "application/zstd",
                "if-none-match": "*"
            },
            "mode": "single",
            "sha256": bundle.source_sha256(),
            "url": format!("https://objects.invalid/{key}"),
            "verifyHead": {
                "contentLength": bundle.source_size_bytes(),
                "sha256": bundle.source_sha256(),
                "url": format!("https://objects.invalid/{key}")
            }
        },
        "requiredComplete": "upload-complete",
        "sourceArtifactId": bundle.source_artifact_id(),
        "sourceObjectKey": format!("source/{key}"),
        "uploadSessionId": upload_session_id,
    }))
    .unwrap()
}

fn fast_path_prepare_response(
    bundle: &PreparedSourceBundle,
    upload_session_id: Uuid,
) -> CliPrepareUploadResponse {
    serde_json::from_value(json!({
        "bucket": "fixture",
        "expiresAt": "2026-09-01T12:00:00Z",
        "fastPath": true,
        "kind": "source-upload",
        "requiredComplete": "upload-complete",
        "sourceArtifactId": bundle.source_artifact_id(),
        "sourceObjectKey": "source/existing",
        "uploadSessionId": upload_session_id,
    }))
    .unwrap()
}

async fn prepared_bundle(workspace_id: Uuid) -> (TempDir, PreparedSourceBundle) {
    prepared_bundle_with_static_role(workspace_id, "static", None).await
}

async fn prepared_bundle_with_static_role(
    workspace_id: Uuid,
    role: &str,
    padded_source_size: Option<u64>,
) -> (TempDir, PreparedSourceBundle) {
    let temp = tempfile::tempdir().unwrap();
    let static_body = b"ready\n";
    let compute_body = b"export default { fetch() {} };\n";
    let manifest = serde_json::to_value(SourceLogicalManifest {
        schema_version: SOURCE_BUNDLE_V1_SCHEMA_VERSION.to_string(),
        capabilities: Vec::new(),
        files: vec![
            SourceLogicalManifestFile {
                path: "index.html".to_string(),
                sha256: sha256_hex(static_body),
                size: static_body.len() as u64,
                entry_type: SourceLogicalManifestEntryType::File,
                link_target: None,
                executable: false,
                content_type: Some("text/html".to_string()),
                role: role.to_string(),
                layer_name: Some("static".to_string()),
            },
            SourceLogicalManifestFile {
                path: "server/main.js".to_string(),
                sha256: sha256_hex(compute_body),
                size: compute_body.len() as u64,
                entry_type: SourceLogicalManifestEntryType::File,
                link_target: None,
                executable: false,
                content_type: None,
                role: "compute".to_string(),
                layer_name: Some("compute".to_string()),
            },
        ],
        layers: vec![
            SourceLogicalManifestLayer {
                name: "static".to_string(),
                target: "STATIC".to_string(),
                root_path: Some(".".to_string()),
                entrypoint: None,
                runtime_config: None,
            },
            SourceLogicalManifestLayer {
                name: "compute".to_string(),
                target: "COMPUTE".to_string(),
                root_path: Some("server".to_string()),
                entrypoint: Some("server/main.js".to_string()),
                runtime_config: Some(json!({ "memoryMb": 256 })),
            },
        ],
        routes: Vec::new(),
        entrypoints: Vec::new(),
    })
    .unwrap();
    let manifest_body = canonical_source_logical_manifest_json(&manifest).into_bytes();
    let mut encoder = zstd::Encoder::new(Vec::new(), 1).unwrap();
    {
        let mut archive = tar::Builder::new(&mut encoder);
        for (path, body) in [
            (
                SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH,
                manifest_body.as_slice(),
            ),
            ("index.html", static_body.as_slice()),
            ("server/main.js", compute_body.as_slice()),
        ] {
            let mut header = tar::Header::new_ustar();
            header.set_size(body.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive.append_data(&mut header, path, body).unwrap();
        }
        archive.finish().unwrap();
    }
    let archive = encoder.finish().unwrap();
    let path = temp.path().join("source-bundle-v1.tar.zst");
    let mut source = std::fs::File::create(&path).unwrap();
    source.write_all(&archive).unwrap();
    if let Some(size) = padded_source_size {
        let frame_size = u32::try_from(size - archive.len() as u64 - 8).unwrap();
        // A skippable Zstd frame adds source bytes without changing the verified TAR.
        source.write_all(&0x184D2A50_u32.to_le_bytes()).unwrap();
        source.write_all(&frame_size.to_le_bytes()).unwrap();
        source.set_len(size).unwrap();
        for offset in (16 * 1024 * 1024..size).step_by(16 * 1024 * 1024) {
            source.seek(SeekFrom::Start(offset)).unwrap();
            source
                .write_all(&[(offset / (16 * 1024 * 1024)) as u8])
                .unwrap();
        }
    }
    drop(source);
    let source_size_bytes = std::fs::metadata(&path).unwrap().len();
    let mut source = std::fs::File::open(&path).unwrap();
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = source.read(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let bundle = PreparedSourceBundle::verify(
        workspace_id,
        SourceBundleInput {
            path,
            source_sha256: hex::encode(hasher.finalize()),
            source_size_bytes,
            logical_manifest_sha256: compute_logical_manifest_sha256(&manifest),
            verification_budget: SourceBundleVerificationBudget::from_manifest(
                &serde_json::from_value(manifest.clone()).unwrap(),
            )
            .unwrap(),
        },
    )
    .await
    .unwrap();
    (temp, bundle)
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_fifo_child() {
    let Some(path) = std::env::var_os("NRZ_PUBLISHER_FIFO_CHILD") else {
        return;
    };
    let manifest: SourceLogicalManifest = serde_json::from_value(json!({
        "schemaVersion": SOURCE_BUNDLE_V1_SCHEMA_VERSION,
        "files": [], "layers": [], "routes": [], "entrypoints": [], "capabilities": []
    }))
    .unwrap();
    let result = PreparedSourceBundle::verify(
        Uuid::now_v7(),
        SourceBundleInput {
            path: path.into(),
            source_sha256: sha256_hex(b""),
            source_size_bytes: 0,
            logical_manifest_sha256: compute_logical_manifest_sha256(
                &serde_json::to_value(&manifest).unwrap(),
            ),
            verification_budget: SourceBundleVerificationBudget::from_manifest(&manifest).unwrap(),
        },
    )
    .await;
    assert!(
        matches!(result, Err(SourcePublicationError::InvalidSourceBundle(_))),
        "{result:?}"
    );
}

#[cfg(unix)]
#[test]
fn refuses_a_fifo_without_waiting_for_a_writer() {
    let temp = tempfile::tempdir().unwrap();
    let fifo = temp.path().join("source.fifo");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "source_bundle_fifo_child", "--nocapture"])
        .env("NRZ_PUBLISHER_FIFO_CHILD", fifo)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let started = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "FIFO verification child failed: {status}");
            break;
        }
        if started.elapsed() >= std::time::Duration::from_secs(3) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("source bundle open blocked waiting for a FIFO writer");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[cfg(unix)]
#[tokio::test]
async fn rejects_a_final_symlink_to_a_valid_source_bundle() {
    let workspace_id = Uuid::now_v7();
    let (temp, bundle) = prepared_bundle(workspace_id).await;
    let alias = temp.path().join("alias.tar.zst");
    std::os::unix::fs::symlink(bundle.path(), &alias).unwrap();
    let result = PreparedSourceBundle::verify(
        workspace_id,
        SourceBundleInput {
            path: alias,
            source_sha256: bundle.source_sha256().into(),
            source_size_bytes: bundle.source_size_bytes(),
            logical_manifest_sha256: bundle.logical_manifest_sha256().into(),
            verification_budget: SourceBundleVerificationBudget::from_manifest(bundle.manifest())
                .unwrap(),
        },
    )
    .await;
    assert!(
        matches!(result, Err(SourcePublicationError::Io { .. })),
        "{result:?}"
    );
}
