use std::collections::VecDeque;
use std::sync::Mutex;

use nrz_runtime_artifact::{
    SourceDependencyMaterialization, VerifiedRuntimeArtifactGraph,
    finalize_source_bundle_runtime_graph_with_dependencies,
    verify_dependency_materialization_manifest,
};
use nrz_source_bundle::{
    SOURCE_BUNDLE_V1_SCHEMA_VERSION, SourceLogicalManifest, SourceLogicalManifestEntryType,
    SourceLogicalManifestFile, SourceLogicalManifestLayer, sha256_hex,
};
use nrz_source_publisher::{
    NoopPublicationObserver, ObjectUploadResult, RuntimeArtifactPublicationRequest,
    RuntimeArtifactPublicationTransport, RuntimeDependencyPublicationInput,
    RuntimeFileUploadRequest, RuntimePublicationCompleteRequest,
    RuntimePublicationCompleteResponse, RuntimePublicationPrepareRequest,
    RuntimePublicationPrepareResponse, SourcePublicationError, publish_runtime_artifacts,
};
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::test]
async fn publishes_only_missing_verified_dependency_images() {
    let temp = TempDir::new().unwrap();
    let image = vec![0x42_u8; 4096];
    let image_path = temp.path().join("dependency.erofs");
    std::fs::write(&image_path, &image).unwrap();
    let image_sha256 = sha256_hex(&image);
    let dependency = dependency_manifest(&image_sha256, image.len() as i64);
    let manifest = source_manifest();
    let graph = finalize_source_bundle_runtime_graph_with_dependencies(
        &"a".repeat(64),
        &"b".repeat(64),
        1024,
        &manifest,
        &[SourceDependencyMaterialization {
            layer_name: "server",
            mount_point: "/output/node_modules",
            manifest: &dependency,
        }],
    )
    .unwrap();
    let deployment_id = Uuid::now_v7();
    let prepare = serde_json::from_value::<RuntimePublicationPrepareResponse>(json!({
        "deploymentId": deployment_id,
        "runtimeArtifactGraphDigest": graph.graph_digest(),
        "runtimeArtifactGraph": graph.wire(),
        "uploads": [{
            "materializationId": dependency.materialization_id(),
            "bucket": "artifacts",
            "objectKey": format!("artifacts/blobs/sha256/{}/{}", &image_sha256[..2], image_sha256),
            "presignedPut": {
                "mode": "conditional-create",
                "url": "https://upload.invalid/dependency",
                "contentLength": image.len(),
                "sha256": image_sha256,
                "verifyHead": {
                    "url": "https://upload.invalid/dependency-head",
                    "contentLength": image.len(),
                    "sha256": image_sha256
                },
                "headers": {
                    "content-type": "application/vnd.onreza.dependency.erofs.v1",
                    "if-none-match": "*"
                }
            }
        }]
    }))
    .unwrap();
    let complete = complete_response(deployment_id, &graph);
    let transport = FakeRuntimeTransport::new(prepare, complete);
    let inputs = [RuntimeDependencyPublicationInput {
        layer_name: "server",
        mount_point: "/output/node_modules",
        image_path: &image_path,
        manifest: &dependency,
    }];

    let published = publish_runtime_artifacts(RuntimeArtifactPublicationRequest {
        transport: &transport,
        observer: &NoopPublicationObserver,
        deployment_id,
        workspace_id: Uuid::now_v7(),
        dependencies: &inputs,
        expected_graph: &graph,
    })
    .await
    .unwrap();

    assert_eq!(
        published.runtime_artifact_graph_digest,
        graph.graph_digest()
    );
    let state = transport.state.lock().unwrap();
    assert_eq!(state.prepare_requests.len(), 1);
    assert_eq!(state.complete_requests.len(), 1);
    assert_eq!(state.uploads.len(), 1);
    assert_eq!(state.uploads[0].path, image_path);
    assert_eq!(state.uploads[0].sha256, image_sha256);
    assert_eq!(state.uploads[0].headers.if_none_match.as_deref(), Some("*"));
}

#[tokio::test]
async fn rejects_a_server_selected_runtime_graph() {
    let manifest = source_manifest_without_dependency();
    let graph = nrz_runtime_artifact::finalize_source_bundle_runtime_graph(
        &"a".repeat(64),
        &"b".repeat(64),
        1024,
        &manifest,
    )
    .unwrap();
    let other = nrz_runtime_artifact::finalize_source_bundle_runtime_graph(
        &"a".repeat(64),
        &"c".repeat(64),
        1024,
        &manifest,
    )
    .unwrap();
    let deployment_id = Uuid::now_v7();
    let prepare = serde_json::from_value::<RuntimePublicationPrepareResponse>(json!({
        "deploymentId": deployment_id,
        "runtimeArtifactGraphDigest": other.graph_digest(),
        "runtimeArtifactGraph": other.wire(),
        "uploads": []
    }))
    .unwrap();
    let complete = complete_response(deployment_id, &graph);
    let transport = FakeRuntimeTransport::new(prepare, complete);

    let error = publish_runtime_artifacts(RuntimeArtifactPublicationRequest {
        transport: &transport,
        observer: &NoopPublicationObserver,
        deployment_id,
        workspace_id: Uuid::now_v7(),
        dependencies: &[],
        expected_graph: &graph,
    })
    .await
    .unwrap_err();

    assert!(matches!(error, SourcePublicationError::InvalidResponse(_)));
    assert!(transport.state.lock().unwrap().uploads.is_empty());
}

struct FakeRuntimeState {
    prepare_responses: VecDeque<RuntimePublicationPrepareResponse>,
    complete_responses: VecDeque<RuntimePublicationCompleteResponse>,
    prepare_requests: Vec<RuntimePublicationPrepareRequest>,
    complete_requests: Vec<RuntimePublicationCompleteRequest>,
    uploads: Vec<RuntimeFileUploadRequest>,
}

struct FakeRuntimeTransport {
    state: Mutex<FakeRuntimeState>,
}

impl FakeRuntimeTransport {
    fn new(
        prepare: RuntimePublicationPrepareResponse,
        complete: RuntimePublicationCompleteResponse,
    ) -> Self {
        Self {
            state: Mutex::new(FakeRuntimeState {
                prepare_responses: [prepare].into(),
                complete_responses: [complete].into(),
                prepare_requests: Vec::new(),
                complete_requests: Vec::new(),
                uploads: Vec::new(),
            }),
        }
    }
}

impl RuntimeArtifactPublicationTransport for FakeRuntimeTransport {
    async fn prepare_runtime_artifacts(
        &self,
        _deployment_id: Uuid,
        request: &RuntimePublicationPrepareRequest,
    ) -> Result<RuntimePublicationPrepareResponse, SourcePublicationError> {
        let mut state = self.state.lock().unwrap();
        state.prepare_requests.push(request.clone());
        Ok(state.prepare_responses.pop_front().unwrap())
    }

    async fn complete_runtime_artifacts(
        &self,
        _deployment_id: Uuid,
        request: &RuntimePublicationCompleteRequest,
    ) -> Result<RuntimePublicationCompleteResponse, SourcePublicationError> {
        let mut state = self.state.lock().unwrap();
        state.complete_requests.push(request.clone());
        Ok(state.complete_responses.pop_front().unwrap())
    }

    async fn put_runtime_file(
        &self,
        request: RuntimeFileUploadRequest,
    ) -> Result<ObjectUploadResult, SourcePublicationError> {
        self.state.lock().unwrap().uploads.push(request);
        Ok(ObjectUploadResult { e_tag: None })
    }
}

fn complete_response(
    deployment_id: Uuid,
    graph: &VerifiedRuntimeArtifactGraph,
) -> RuntimePublicationCompleteResponse {
    serde_json::from_value(json!({
        "deploymentId": deployment_id,
        "runtimeArtifactGraphDigest": graph.graph_digest(),
        "runtimeArtifactGraph": graph.wire()
    }))
    .unwrap()
}

fn dependency_manifest(
    image_sha256: &str,
    image_size: i64,
) -> nrz_runtime_artifact::VerifiedDependencyMaterializationManifest {
    verify_dependency_materialization_manifest(json!({
        "schemaVersion": "DEPENDENCY_MATERIALIZATION_V1.0",
        "kind": "JAVASCRIPT_NODE_MODULES",
        "compatibility": {
            "runtimeFamily": "bun",
            "runtimeVersion": "1.4.2",
            "os": "linux",
            "architecture": "x86_64",
            "libc": "glibc",
            "abi": "glibc-2.42",
            "packageManager": "bun",
            "packageManagerVersion": "1.4.2",
            "runnerRootfsDigest": format!("sha256:{}", "d".repeat(64)),
            "buildPolicyGeneration": 1
        },
        "logicalTreeDigest": "a".repeat(64),
        "expandedFileCount": 1,
        "expandedBytes": 42,
        "regularFileCount": 1,
        "symlinkCount": 0,
        "nativeObjectCount": 0,
        "canonicalizationPolicyDigest": format!("sha256:{}", "c".repeat(64)),
        "generatorDigest": format!("sha256:{}", "d".repeat(64)),
        "blobDescriptor": {
            "mediaType": "application/vnd.onreza.dependency.erofs.v1",
            "digest": format!("sha256:{image_sha256}"),
            "size": image_size
        }
    }))
    .unwrap()
}

fn source_manifest() -> SourceLogicalManifest {
    let mut manifest = source_manifest_without_dependency();
    manifest.files.push(SourceLogicalManifestFile {
        path: "node_modules/pkg/index.js".to_string(),
        sha256: "e".repeat(64),
        size: 42,
        content_type: None,
        role: "dependency".to_string(),
        layer_name: Some("server".to_string()),
        entry_type: SourceLogicalManifestEntryType::File,
        link_target: None,
        executable: false,
    });
    manifest
}

fn source_manifest_without_dependency() -> SourceLogicalManifest {
    SourceLogicalManifest {
        schema_version: SOURCE_BUNDLE_V1_SCHEMA_VERSION.to_string(),
        files: vec![SourceLogicalManifestFile {
            path: "server/server.js".to_string(),
            sha256: "f".repeat(64),
            size: 64,
            content_type: None,
            role: "compute".to_string(),
            layer_name: Some("server".to_string()),
            entry_type: SourceLogicalManifestEntryType::File,
            link_target: None,
            executable: false,
        }],
        layers: vec![source_layer()],
        capabilities: Vec::new(),
        routes: Vec::new(),
        entrypoints: Vec::new(),
    }
}

fn source_layer() -> SourceLogicalManifestLayer {
    SourceLogicalManifestLayer {
        name: "server".to_string(),
        target: "COMPUTE".to_string(),
        root_path: Some("server".to_string()),
        entrypoint: Some("server/server.js".to_string()),
        runtime_config: Some(json!({ "memoryMb": 256 })),
    }
}
