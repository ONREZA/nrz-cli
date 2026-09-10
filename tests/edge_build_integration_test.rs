//! Edge runner handoff contract against the real CLI binary.
mod support;
use axum::{
    Json, Router,
    extract::{OriginalUri, State},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::json;
use std::{
    fs,
    sync::{Arc, Mutex},
};
use support::cli::{nrz, stdout_json};

#[derive(Clone)]
struct EdgeBuildApiState {
    deployment_id: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl EdgeBuildApiState {
    fn record(&self, method: Method, uri: &OriginalUri) {
        self.requests
            .lock()
            .expect("request log")
            .push(format!("{method} {}", uri.0.path()));
    }
}

async fn edge_build_runner_context(
    State(state): State<EdgeBuildApiState>,
    uri: OriginalUri,
) -> Json<serde_json::Value> {
    state.record(Method::GET, &uri);
    Json(json!({
        "protocolVersion": "runner-context-v3",
        "context": {
            "workspaceId": "workspace-edge",
            "workspaceSlug": "edge",
            "projectId": "project-edge",
            "projectName": "Edge project",
            "environmentId": "environment-edge",
            "environmentName": "Preview",
            "environmentType": "PREVIEW",
            "sourceRef": "0123456789abcdef0123456789abcdef01234567",
            "selectionSource": "DEPLOYMENT"
        },
        "deployment": {
            "id": state.deployment_id,
            "attempt": 1,
            "status": "BUILDING",
            "url": null
        },
        "settings": {
            "frameworkPreset": null,
            "rootDirectory": ".",
            "gitLfsEnabled": false,
            "packageManager": "NPM",
            "installCommand": null,
            "installCommandSource": "PRESET",
            "buildCommand": null,
            "buildCommandSource": "PRESET",
            "outputDirectory": null,
            "outputDirectorySource": "PRESET",
            "ignoredBuildBehavior": "AUTOMATIC",
            "ignoredBuildFolder": null,
            "ignoredBuildCommand": null
        }
    }))
}

async fn edge_build_materialize(
    State(state): State<EdgeBuildApiState>,
    uri: OriginalUri,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    state.record(Method::POST, &uri);
    assert_eq!(body, json!({ "purpose": "DEPLOY" }));
    Json(json!({
        "protocolVersion": "execution-context-v1",
        "context": {
            "workspaceId": "workspace-edge",
            "workspaceSlug": "edge",
            "projectId": "project-edge",
            "projectName": "Edge project",
            "environmentId": "environment-edge",
            "environmentName": "Preview",
            "environmentType": "PREVIEW",
            "sourceRef": "0123456789abcdef0123456789abcdef01234567",
            "selectionSource": "DEPLOYMENT"
        },
        "variables": {},
        "secretKeys": [],
        "snapshot": {
            "fingerprint": format!("v1:{}", "0".repeat(64)),
            "resolvedAt": "2026-08-29T00:00:00Z",
            "source": "DEPLOYMENT",
            "deploymentId": state.deployment_id
        }
    }))
}

async fn reject_unexpected_edge_build_request(
    State(state): State<EdgeBuildApiState>,
    method: Method,
    uri: OriginalUri,
) -> impl IntoResponse {
    state.record(method, &uri);
    (
        StatusCode::IM_A_TEAPOT,
        Json(json!({ "error": "unexpected Edge build request" })),
    )
}

fn spawn_edge_build_handoff_mock(deployment_id: &str) -> (String, Arc<Mutex<Vec<String>>>) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let state = EdgeBuildApiState {
        deployment_id: deployment_id.to_string(),
        requests: Arc::clone(&requests),
    };
    let app = Router::new()
        .route(
            "/v1/deployments/{deployment_id}/runner-context",
            get(edge_build_runner_context),
        )
        .route(
            "/v1/deployments/{deployment_id}/execution-context/materialize",
            post(edge_build_materialize),
        )
        .fallback(reject_unexpected_edge_build_request)
        .with_state(state);
    (support::api_mock::spawn(app), requests)
}

#[test]
fn edge_build_publishes_local_handoff_without_legacy_source_mutations() {
    let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
    let (api_url, requests) = spawn_edge_build_handoff_mock(deployment_id);
    let project = tempfile::tempdir().unwrap();
    let output_dir = tempfile::tempdir().unwrap();
    fs::write(project.path().join("index.html"), "<h1>Edge build</h1>").unwrap();

    let output = nrz()
        .current_dir(project.path())
        .env("NRZ_API_URL", api_url)
        .env("NRZ_RUNNER", "PLATFORM")
        .env("ONREZA_RUNTIME_OS", "linux")
        .env("ONREZA_RUNTIME_ARCH", "x86_64")
        .env("ONREZA_RUNTIME_LIBC", "glibc")
        .env("NRZ_EDGE_BUILD_HANDOFF", "V1")
        .env("NRZ_LOG_UPLOAD", "0")
        .env("ONREZA_OUTPUT_DIR", output_dir.path())
        .args([
            "--token",
            "runner-token",
            "deploy",
            project.path().to_str().unwrap(),
            "--resume-deployment",
            deployment_id,
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let handoff: nrz_source_bundle::EdgeBuildHandoffV1 =
        serde_json::from_value(stdout_json(&output)).unwrap();
    handoff.validate().unwrap();
    assert_eq!(
        fs::read(output_dir.path().join("edge-build-handoff-v1.json")).unwrap(),
        output.stdout
    );
    assert!(output_dir.path().join("source-bundle-v1.tar.zst").is_file());
    assert_eq!(
        requests.lock().unwrap().as_slice(),
        [
            format!("GET /v1/deployments/{deployment_id}/runner-context"),
            format!("POST /v1/deployments/{deployment_id}/execution-context/materialize"),
        ]
    );
}
