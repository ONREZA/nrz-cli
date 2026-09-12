use std::collections::VecDeque;
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
use nrz_runtime_artifact::finalize_source_bundle_runtime_graph;
use nrz_source_bundle::{
    SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH, SOURCE_BUNDLE_V1_SCHEMA_VERSION, SourceLogicalManifest,
    SourceLogicalManifestEntryType, SourceLogicalManifestFile, SourceLogicalManifestLayer,
    canonical_source_logical_manifest_json, compute_logical_manifest_sha256, sha256_hex,
};
use nrz_source_publisher::{
    DeploymentPublicationStatus, ObjectUploadRequest, ObjectUploadResult, PreparedSourceBundle,
    PublicationEvent, PublicationObserver, SourceBundleInput, SourcePublicationError,
    SourcePublicationRequest, SourcePublicationTransport, publish_source_bundle,
    publish_source_bundle_upload,
};
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

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

    assert_eq!(published.source_artifact_id, bundle.source_artifact_id());
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
}

struct FakeState {
    prepare_responses: VecDeque<CliPrepareUploadResponse>,
    complete_responses: VecDeque<CliUploadCompleteResponse>,
    put_outcomes: VecDeque<PutOutcome>,
    statuses: VecDeque<DeploymentPublicationStatus>,
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
                prepare_responses: prepare_responses.into(),
                complete_responses: complete_responses.into(),
                put_outcomes: put_outcomes.into(),
                statuses: statuses.into(),
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
        Ok(state.prepare_responses.pop_front().unwrap())
    }

    async fn complete_multipart(
        &self,
        _deployment_id: Uuid,
        _request: &CliMultipartCompleteRequest,
    ) -> Result<CliMultipartCompleteResponse, SourcePublicationError> {
        panic!("single-part fixture must not complete multipart upload")
    }

    async fn complete_upload(
        &self,
        _deployment_id: Uuid,
        request: &CliUploadCompleteRequest,
    ) -> Result<CliUploadCompleteResponse, SourcePublicationError> {
        let mut state = self.state.lock().unwrap();
        state.complete_requests.push(request.clone());
        Ok(state.complete_responses.pop_front().unwrap())
    }

    async fn report_upload_failed(
        &self,
        _deployment_id: Uuid,
        _request: &CliUploadFailedRequest,
    ) -> Result<CliUploadFailedResponse, SourcePublicationError> {
        panic!("recoverable conditional conflict must not report terminal upload failure")
    }

    async fn deployment_status(
        &self,
        _deployment_id: Uuid,
    ) -> Result<DeploymentPublicationStatus, SourcePublicationError> {
        Ok(self.state.lock().unwrap().statuses.pop_front().unwrap())
    }

    async fn put_object(
        &self,
        request: ObjectUploadRequest,
    ) -> Result<ObjectUploadResult, SourcePublicationError> {
        let mut state = self.state.lock().unwrap();
        state.put_requests.push(request);
        match state.put_outcomes.pop_front().unwrap() {
            PutOutcome::Success => Ok(ObjectUploadResult { e_tag: None }),
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
                role: "static".to_string(),
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
    std::fs::write(&path, &archive).unwrap();
    let bundle = PreparedSourceBundle::verify(
        workspace_id,
        SourceBundleInput {
            path,
            source_sha256: sha256_hex(&archive),
            source_size_bytes: archive.len() as u64,
            logical_manifest_sha256: compute_logical_manifest_sha256(&manifest),
        },
    )
    .await
    .unwrap();
    (temp, bundle)
}
