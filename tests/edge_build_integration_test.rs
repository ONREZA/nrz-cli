//! Edge runner handoff contract against the real CLI binary.
#[path = "edge_build/source_v2.rs"]
mod source_v2;
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
    retry_count: u32,
    requests: Arc<Mutex<Vec<String>>>,
    application_runtime: serde_json::Value,
    build_command: Option<String>,
    install_command: Option<String>,
    build_toolchain: serde_json::Value,
    output_directory: Option<String>,
    ignored_build_behavior: &'static str,
}

impl EdgeBuildApiState {
    fn record(&self, method: Method, uri: &OriginalUri) {
        self.requests
            .lock()
            .expect("request log")
            .push(format!("{method} {}", uri.0.path()));
    }
}

fn edge_build_execution_context() -> serde_json::Value {
    json!({
        "workspaceId": "00000000-0000-0000-0000-000000000001",
        "workspaceSlug": "edge",
        "productId": "00000000-0000-0000-0000-000000000002",
        "productEnvironmentId": "00000000-0000-0000-0000-000000000003",
        "workspaceEnvironmentId": "00000000-0000-0000-0000-000000000004",
        "workspaceBindingGeneration": "1",
        "serviceId": "00000000-0000-0000-0000-000000000005",
        "sourceId": "00000000-0000-0000-0000-000000000006",
        "projectId": "00000000-0000-0000-0000-000000000002",
        "projectName": "Edge project",
        "environmentId": "00000000-0000-0000-0000-000000000003",
        "environmentName": "Preview",
        "environmentType": "PREVIEW",
        "sourceRef": "0123456789abcdef0123456789abcdef01234567",
        "selectionSource": "DEPLOYMENT"
    })
}

async fn edge_build_runner_context(
    State(state): State<EdgeBuildApiState>,
    uri: OriginalUri,
) -> Json<serde_json::Value> {
    state.record(Method::GET, &uri);
    Json(json!({
        "protocolVersion": "runner-context-v6",
        "context": edge_build_execution_context(),
        "deployment": {
            "id": state.deployment_id,
            "attempt": state.retry_count,
            "status": "BUILDING",
            "branch": "main",
            "commitSha": "0123456789abcdef0123456789abcdef01234567",
            "url": null
        },
        "settings": {
            "nodeVersion": "NODE_24",
            "sourceBuildContext": {"schemaVersion":1,"buildToolchain":state.build_toolchain,"applicationRuntime": state.application_runtime},
            "frameworkPreset": null,
            "rootDirectory": ".",
            "gitLfsEnabled": false,
            "packageManager": "NPM",
            "installCommand": state.install_command,
            "installCommandSource": if state.build_command.is_some() || !state.application_runtime.is_null() {"USER"} else {"PRESET"},
            "buildCommand": state.build_command,
            "buildCommandSource": if state.build_command.is_some() {"USER"} else {"PRESET"},
            "outputDirectory": state.output_directory,
            "outputDirectorySource": if state.build_command.is_some() {"USER"} else {"PRESET"},
            "ignoredBuildBehavior": state.ignored_build_behavior,
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
        "protocolVersion": "execution-context-v3",
        "context": edge_build_execution_context(),
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

async fn edge_build_source_v2_metadata(
    State(state): State<EdgeBuildApiState>,
    uri: OriginalUri,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    state.record(Method::POST, &uri);
    let body: nrz_api::MetadatumRequestBody = serde_json::from_value(body).unwrap();
    assert_eq!(body.protocol_version, "execution-context-v3");
    assert_eq!(body.attempt, i64::from(state.retry_count));
    assert_eq!(
        body.operation_id,
        uuid::Uuid::new_v5(
            &state.deployment_id.parse().unwrap(),
            format!("onreza:deployment-source:{}", state.retry_count).as_bytes()
        )
    );
    assert!(!body.manifest.layers.is_empty());
    let functions = body
        .functions
        .expect("CLI preserves the explicit Functions snapshot");
    assert_eq!(functions.origin, "DEPLOYMENT");
    assert_eq!(functions.functions, Some(Vec::new()));
    assert!(functions.edge_rules.is_none());
    assert!(
        functions
            .generated_edge_rule_sets
            .is_none_or(|sets| sets.is_empty())
    );
    Json(json!({"id":state.deployment_id,"attempt":body.attempt,"registered":true}))
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
    spawn_edge_build_handoff_mock_with_intent(deployment_id, serde_json::Value::Null)
}

fn spawn_edge_build_handoff_mock_with_retry_count(
    deployment_id: &str,
    retry_count: u32,
) -> (String, Arc<Mutex<Vec<String>>>) {
    spawn_edge_build_handoff_mock_with_commands(
        deployment_id,
        serde_json::Value::Null,
        json!({"family":"NODE"}),
        None,
        None,
        None,
        retry_count,
    )
}

fn spawn_edge_build_handoff_mock_with_intent(
    deployment_id: &str,
    application_runtime: serde_json::Value,
) -> (String, Arc<Mutex<Vec<String>>>) {
    spawn_edge_build_handoff_mock_with_build(deployment_id, application_runtime, None)
}

fn spawn_edge_build_handoff_mock_with_build(
    deployment_id: &str,
    application_runtime: serde_json::Value,
    build_command: Option<String>,
) -> (String, Arc<Mutex<Vec<String>>>) {
    let install_command = if build_command
        .as_ref()
        .is_some_and(|command| command.is_empty())
    {
        Some(String::new())
    } else if build_command.is_some() {
        Some("true".into())
    } else if application_runtime.is_null() {
        None
    } else {
        Some("touch installer-started".into())
    };
    spawn_edge_build_handoff_mock_with_commands(
        deployment_id,
        application_runtime,
        json!({"family":"NODE"}),
        install_command,
        build_command,
        None,
        1,
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_edge_build_handoff_mock_with_commands(
    deployment_id: &str,
    application_runtime: serde_json::Value,
    build_toolchain: serde_json::Value,
    install_command: Option<String>,
    build_command: Option<String>,
    output_directory: Option<&str>,
    retry_count: u32,
) -> (String, Arc<Mutex<Vec<String>>>) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let output_directory = output_directory
        .map(str::to_owned)
        .or_else(|| build_command.as_ref().map(|_| ".".into()));
    let state = EdgeBuildApiState {
        deployment_id: deployment_id.to_string(),
        retry_count,
        requests: Arc::clone(&requests),
        application_runtime,
        build_command,
        install_command,
        build_toolchain,
        output_directory,
        ignored_build_behavior: "AUTOMATIC",
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
        .route(
            "/v1/deployments/{deployment_id}/source-v2/metadata",
            post(edge_build_source_v2_metadata),
        )
        .route("/v1/projects/{id}/execution-context/resolve", post(|State(state):State<EdgeBuildApiState>, uri:OriginalUri| async move {
            state.record(Method::POST, &uri);
            Json(json!({"protocolVersion":"execution-context-v3","context":edge_build_execution_context()}))
        }))
        .route("/v1/projects/{id}/deployments/admit", post(|State(state):State<EdgeBuildApiState>, uri:OriginalUri| async move {
            state.record(Method::POST, &uri);
            Json(json!({"protocolVersion":"execution-context-v3","context":edge_build_execution_context(),
                "deployment":{"id":state.deployment_id,"attempt":state.retry_count,"status":"BUILDING","url":null},
                "snapshot":{"fingerprint":format!("v1:{}","0".repeat(64)),"resolvedAt":"2026-10-22T00:00:00Z"}}))
        }))
        .route("/v1/deployments/{id}/static-source-v2/admit", post(|State(state):State<EdgeBuildApiState>, uri:OriginalUri, Json(body):Json<serde_json::Value>| async move {
            state.record(Method::POST, &uri);
            assert_eq!(body["packageProfile"], nrz_contract::application_package_v1::APPLICATION_EROFS_PROFILE_V1);
            Json(closed_source_v2_admission(body, state.retry_count))
        }))
        .route("/v1/deployments/{id}/status", get(|State(state):State<EdgeBuildApiState>, uri:OriginalUri| async move {
            state.record(Method::GET, &uri);
            Json(json!({"id":state.deployment_id,"status":"live","url":null,"production":false,
                "error":null,"errorCode":null,"errorDetails":null,"runtimeArtifactGraphDigest":null,"runtimeArtifactGraph":null,
                "createdAt":"2026-10-22T00:00:00Z","readyAt":null}))
        }))
        .fallback(reject_unexpected_edge_build_request)
        .with_state(state);
    (support::api_mock::spawn(app), requests)
}

fn edge_build_command(
    project: &std::path::Path,
    output: &std::path::Path,
    api_url: &str,
    deployment_id: &str,
    json_mode: bool,
) -> assert_cmd::Command {
    let mut command = nrz();
    if json_mode {
        command.arg("--json");
    }
    command
        .current_dir(project)
        .env("NRZ_API_URL", api_url)
        .env("NRZ_RUNNER", "PLATFORM")
        .env("ONREZA_BUILD_RUNTIME_FAMILY", "javascript")
        .env("ONREZA_BUILD_RUNTIME_VERSION", "node-24")
        .env_remove("ONREZA_BUILD_NODE_MAJOR")
        .env("ONREZA_RUNTIME_OS", "linux")
        .env("ONREZA_RUNTIME_ARCH", "x86_64")
        .env("ONREZA_RUNTIME_LIBC", "glibc")
        .env("NRZ_EDGE_BUILD_HANDOFF", "V2")
        .env(
            "NRZ_EDGE_BUILD_COMMAND_ID",
            "77777777-7777-4777-8777-777777777777",
        )
        .env("NRZ_EDGE_BUILD_COMMAND_DIGEST", "a".repeat(64))
        .env(
            "NRZ_EDGE_BUILD_BUILD_ATTEMPT_ID",
            "11111111-1111-4111-8111-111111111111",
        )
        .env("NRZ_EDGE_BUILD_ATTEMPT", "2")
        .env("NRZ_EDGE_BUILD_SOURCE_WRITER_GENERATION", "9")
        .env(
            "NRZ_EDGE_BUILD_SOURCE_ENVIRONMENT_ID",
            edge_build_execution_context()["productEnvironmentId"]
                .as_str()
                .unwrap(),
        )
        .env_remove("NRZ_SOURCE_TRANSFER")
        .env_remove("NRZ_SOURCE_PACKAGE_PROFILE")
        .env("NRZ_LOG_UPLOAD", "0")
        .env("ONREZA_OUTPUT_DIR", output)
        .args([
            "--token",
            "runner-token",
            "deploy",
            project.to_str().unwrap(),
            "--resume-deployment",
            deployment_id,
        ]);
    command
}

fn successful_handoff(
    output: &std::process::Output,
    scenario: &str,
) -> nrz_source_bundle::EdgeBuildHandoffV2 {
    assert!(
        output.status.success(),
        "{scenario} stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let handoff: nrz_source_bundle::EdgeBuildHandoffV2 =
        serde_json::from_value(stdout_json(output)).unwrap();
    handoff.validate().unwrap();
    handoff
}

#[derive(serde::Deserialize)]
struct SourceV2Manifest {
    layers: Vec<nrz_source_bundle::SourceLogicalManifestLayer>,
    files: Vec<SourceV2File>,
}

#[derive(serde::Deserialize)]
struct SourceV2File {
    path: String,
    role: String,
}

fn source_bundle_manifest(
    output: &std::path::Path,
    cli_output: &std::process::Output,
    scenario: &str,
) -> SourceV2Manifest {
    let handoff = successful_handoff(cli_output, scenario);
    let bytes = fs::read(output.join(&handoff.source_manifest.path)).unwrap();
    assert_eq!(
        blake3::hash(&bytes).to_hex().as_str(),
        handoff.source_manifest.blake3
    );
    serde_json::from_slice(&bytes).unwrap()
}

fn closed_source_v2_admission(mut body: serde_json::Value, retry_count: u32) -> serde_json::Value {
    let operation_id = body["operationId"].clone();
    let idempotency_key = body["cliIdempotencyKey"].clone();
    let input = body.as_object_mut().unwrap();
    input.remove("operationId");
    input.remove("cliIdempotencyKey");
    let context = edge_build_execution_context();
    for name in ["workspaceId", "projectId", "environmentId"] {
        input.insert(name.into(), context[name].clone());
    }
    input.insert("deploymentAttempt".into(), json!(retry_count + 1));
    let identity = "77777777-7777-4777-8777-777777777777";
    json!({
        "operationId":operation_id,"cliIdempotencyKey":idempotency_key,"state":"CLOSED","input":body,
        "inputDigest":"a".repeat(64),"writerGeneration":"9","installationId":identity,
        "assignmentId":identity,"assignmentGeneration":"1","nodeIncarnationId":identity,
        "nodeId":"edge-test","nodeBootId":identity,"identityGeneration":"1",
        "receiverInstanceId":identity,"receiverSessionId":identity,"receiverEndpointId":"b".repeat(64),
        "profileDigest":"c".repeat(64),"observationId":identity,"observedAt":"2026-10-22T00:00:00Z",
        "observationValidUntil":"2099-01-01T00:00:00Z","expiresAt":"2099-01-01T00:00:00Z",
        "reservedBusinessBytes":0,"terminalReason":"PUBLISHED","terminalAt":"2026-10-22T00:01:00Z"
    })
}

#[test]
fn local_sets_publish_before_activation_and_keep_one_terminal_json() {
    for scenario in ["complete", "register-error", "prepare-error", "skip"] {
        let project = tempfile::tempdir().unwrap();
        fs::write(project.path().join("index.html"), "<h1>Local set</h1>").unwrap();
        let context = edge_build_execution_context();
        let deployment = uuid::Uuid::now_v7();
        let operation = uuid::Uuid::now_v7();
        let key = uuid::Uuid::now_v7();
        let service = context["serviceId"].as_str().unwrap();
        let receipt = project
            .path()
            .join(format!(".onreza/deployment-sets/{key}.json"));
        fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        let expected_context: serde_json::Map<String, serde_json::Value> = [
            "workspaceId",
            "productId",
            "productEnvironmentId",
            "workspaceEnvironmentId",
            "workspaceBindingGeneration",
            "serviceId",
            "sourceId",
        ]
        .into_iter()
        .map(|name| (name.to_string(), context[name].clone()))
        .collect();
        fs::write(&receipt, json!({
            "projectId":context["projectId"], "environmentId":context["environmentId"],
            "environmentSelector":"preview", "local":{service:project.path()}, "all":false,
            "selectedServices":[service], "request":{
                "protocolVersion":"execution-context-v3", "environmentId":context["environmentId"],
                "expectedGeneration":"7", "idempotencyKey":key, "selection":{"kind":"SERVICES"},
                "services":[{"mode":"LOCAL", "serviceId":service, "branch":"main",
                    "commitSha":"0123456789abcdef0123456789abcdef01234567", "expectedContext":expected_context}]
            }
        }).to_string()).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let state = EdgeBuildApiState {
            deployment_id: deployment.to_string(),
            retry_count: 1,
            requests: Arc::clone(&requests),
            application_runtime: serde_json::Value::Null,
            build_command: Some(String::new()),
            install_command: Some(String::new()),
            build_toolchain: json!({"family":"NODE"}),
            output_directory: Some(".".into()),
            ignored_build_behavior: if scenario == "skip" {
                "NEVER"
            } else {
                "AUTOMATIC"
            },
        };
        let admitted = json!({"operationId":operation,"generation":"8","status":"PENDING","replayed":true,
            "deployments":[{"id":deployment,"serviceId":service,"status":"BUILDING"}]});
        let app = Router::new()
            .route("/v1/projects/{id}/deployments/admit-set", post(move || {
                let admitted = admitted.clone();
                async move { Json(admitted) }
            }))
            .route("/v1/deployments/{id}/runner-context", get(edge_build_runner_context))
            .route("/v1/deployments/{id}/execution-context/materialize", post(edge_build_materialize))
            .route("/v1/deployments/{id}/source-v2/metadata", post(move |State(state):State<EdgeBuildApiState>, uri:OriginalUri, Json(body):Json<serde_json::Value>| async move {
                state.record(Method::POST, &uri);
                assert_eq!(body["attempt"],1);
                assert_eq!(body["manifest"]["version"].as_f64(),Some(2.0));
                if scenario == "register-error" {
                    (StatusCode::BAD_REQUEST,Json(json!({"code":"EDGE_RULES_DIVERGED","message":"pull reviewed rules", "details":{"field":"edgeRules.remote-7"}}))).into_response()
                } else {
                    Json(json!({"id":deployment,"attempt":1,"registered":true})).into_response()
                }
            }))
            .route("/v1/deployments/{id}/static-source-v2/admit", post(move |State(state):State<EdgeBuildApiState>, uri:OriginalUri, Json(body):Json<serde_json::Value>| async move {
                state.record(Method::POST, &uri);
                assert_eq!(body["packageProfile"], nrz_contract::application_package_v1::APPLICATION_EROFS_PROFILE_V1);
                if scenario == "prepare-error" {
                    (StatusCode::FORBIDDEN,Json(json!({"code":"LIMIT_EXCEEDED","message":"source limit", "details":{"limitType":"maxDeploymentFiles","current":2,"limit":1,"remaining":0}}))).into_response()
                } else {
                    Json(closed_source_v2_admission(body, state.retry_count)).into_response()
                }
            }))
            .route("/v1/deployments/{id}/execution-context/fail-before-source", post(move |State(state):State<EdgeBuildApiState>, uri:OriginalUri, Json(body):Json<serde_json::Value>| async move {
                state.record(Method::POST, &uri);
                assert_eq!(body["attempt"], 1);
                if scenario == "prepare-error" {
                    assert_eq!(body["diagnostic"]["code"], "LIMIT_EXCEEDED");
                    assert_eq!(body["diagnostic"]["details"]["limitType"], "maxDeploymentFiles");
                }
                Json(json!({"id":deployment,"accepted":true}))
            }))
            .route("/v1/deployments/{id}/execution-context/skip-before-source", post(move |State(state):State<EdgeBuildApiState>, uri:OriginalUri| async move {
                state.record(Method::POST, &uri);
                Json(json!({"id":deployment,"accepted":true}))
            }))
            .route("/v1/deployments/{id}/status", get(move |State(state):State<EdgeBuildApiState>, uri:OriginalUri| {
                state.record(Method::GET,&uri);
                async move {
                Json(json!({"id":deployment,"status":if scenario == "skip" {"skipped"} else {"live"},
                    "url":null,"production":false,"error":null,"errorCode":null,"errorDetails":null,
                    "runtimeArtifactGraphDigest":null,"runtimeArtifactGraph":null,"createdAt":"2026-10-14T00:00:00Z","readyAt":null}))
            }}))
            .fallback(reject_unexpected_edge_build_request)
            .with_state(state);
        let api = support::api_mock::spawn(app);
        let output = nrz()
            .timeout(std::time::Duration::from_secs(20))
            .current_dir(project.path())
            .env("NRZ_API_URL", api)
            .env("NRZ_LOG_UPLOAD", "0")
            .env_remove("NRZ_SOURCE_TRANSFER")
            .env_remove("NRZ_SOURCE_PACKAGE_PROFILE")
            .env_remove("NRZ_RUNNER")
            .env_remove("NRZ_EDGE_BUILD_HANDOFF")
            .env_remove("ONREZA_OUTPUT_DIR")
            .args([
                "--json",
                "--token",
                "test-token",
                "deploy-set",
                "--project-id",
                context["projectId"].as_str().unwrap(),
                "--environment",
                "preview",
                "--service-id",
                service,
                "--local",
                &format!("{service}={}", project.path().display()),
                "--idempotency-key",
                &key.to_string(),
                "--wait-timeout",
                "1",
            ])
            .output()
            .unwrap();
        let terminal = stdout_json(&output);
        assert_eq!(
            output.status.success(),
            scenario == "complete",
            "{scenario}: {terminal}, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let calls = requests.lock().unwrap();
        let called = |path: &str| {
            calls
                .iter()
                .any(|call| call == &format!("POST /v1/deployments/{deployment}/{path}"))
        };
        if scenario == "complete" {
            assert_eq!(terminal["status"], "live");
            assert_eq!(terminal["operation"]["operationId"], operation.to_string());
            assert!(called("source-v2/metadata") && called("static-source-v2/admit"));
            assert!(!called("source") && !called("prepare-upload") && !called("upload-complete"));
            let sequence = [
                format!("POST /v1/deployments/{deployment}/source-v2/metadata"),
                format!("POST /v1/deployments/{deployment}/static-source-v2/admit"),
                format!("GET /v1/deployments/{deployment}/status"),
            ];
            let positions =
                sequence.map(|request| calls.iter().position(|call| call == &request).unwrap());
            assert!(
                positions.windows(2).all(|pair| pair[0] < pair[1]),
                "publication order: {calls:?}"
            );
        } else {
            assert_eq!(terminal["details"]["operationId"], operation.to_string());
            assert_eq!(terminal["details"]["idempotencyKey"], key.to_string());
            assert_eq!(
                terminal["details"]["receipt"],
                receipt.to_string_lossy().as_ref()
            );
            match scenario {
                "register-error" => {
                    assert_eq!(terminal["code"], "EDGE_RULES_DIVERGED");
                    assert_eq!(terminal["details"]["field"], "edgeRules.remote-7");
                    assert!(called("source-v2/metadata"));
                    assert!(!called("static-source-v2/admit"));
                }
                "prepare-error" => {
                    assert_eq!(terminal["code"], "LIMIT_EXCEEDED");
                    assert_eq!(terminal["details"]["limit"].as_f64(), Some(1.0));
                    assert!(
                        terminal["details"]["runtimeArtifactFiles"]["total"]
                            .as_u64()
                            .unwrap()
                            > 0
                    );
                    assert!(called("static-source-v2/admit"));
                    assert!(!called("upload-complete"));
                    assert!(
                        called("execution-context/fail-before-source"),
                        "definite quota refusal must terminalize the unaccepted deployment: {calls:?}"
                    );
                }
                "skip" => {
                    assert_eq!(terminal["code"], "DEPLOY_SKIPPED");
                    assert!(called("execution-context/skip-before-source"));
                    assert!(!called("source"));
                }
                _ => unreachable!(),
            }
        }
    }
}

#[test]
fn changed_source_runtime_is_rejected_before_the_installer_executes() {
    let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
    let (api_url, _) = spawn_edge_build_handoff_mock_with_intent(
        deployment_id,
        json!({"family":"NODE","entry":"server.ts","args":[]}),
    );
    let project = tempfile::tempdir().unwrap();
    let output_dir = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("onreza.toml"),
        "[deploy]\nruntime='bun'\nentry='server.ts'\nargs=[]\n",
    )
    .unwrap();
    fs::write(
        project.path().join("server.ts"),
        "Bun.serve({fetch:()=>new Response('ok')})",
    )
    .unwrap();
    let output = edge_build_command(
        project.path(),
        output_dir.path(),
        &api_url,
        deployment_id,
        true,
    )
    .env("ONREZA_RUNTIME_VERSION", "bun-1.4.2")
    .output()
    .unwrap();
    assert!(!output.status.success());
    let result = stdout_json(&output);
    assert_eq!(result["code"], "APPLICATION_RUNTIME_INVALID");
    assert!(
        result["error"]
            .as_str()
            .unwrap()
            .contains("frozen deployment snapshot")
    );
    assert!(!project.path().join("installer-started").exists());
    assert!(!output_dir.path().join("source-bundle-v1.tar.zst").exists());
}

#[test]
fn edge_build_publishes_local_handoff_without_legacy_source_mutations() {
    let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
    let (api_url, requests) = spawn_edge_build_handoff_mock(deployment_id);
    let project = tempfile::tempdir().unwrap();
    let output_dir = tempfile::tempdir().unwrap();
    fs::write(project.path().join("index.html"), "<h1>Edge build</h1>").unwrap();

    let output = edge_build_command(
        project.path(),
        output_dir.path(),
        &api_url,
        deployment_id,
        false,
    )
    .output()
    .unwrap();

    successful_handoff(&output, "static handoff");
    assert_eq!(
        fs::read(output_dir.path().join("edge-build-handoff-v2.json")).unwrap(),
        output.stdout
    );
    assert!(output_dir.path().join("source-manifest-v2.json").is_file());
    assert_eq!(
        requests.lock().unwrap().as_slice(),
        [
            format!("GET /v1/deployments/{deployment_id}/runner-context"),
            format!("POST /v1/deployments/{deployment_id}/execution-context/materialize"),
            format!("POST /v1/deployments/{deployment_id}/source-v2/metadata"),
        ]
    );
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[test]
fn edge_build_handoff_preserves_python_and_native_serving_under_node_compiler() {
    let targets = nrz_source_bundle::PythonMinor::ALL
        .into_iter()
        .map(|minor| (Some(minor), minor.target()))
        .chain([(
            Some(nrz_source_bundle::PythonMinor::Python312),
            "python-3.14".to_string(),
        )])
        .chain([(
            None,
            nrz_runtime_artifact::NATIVE_EXECUTION_TARGET.to_string(),
        )]);
    for (minor, target) in targets {
        let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
        let (family, entry, mut declaration) = if let Some(minor) = minor {
            (
                "PYTHON",
                "main.py",
                json!({"family":"PYTHON", "pythonVersion":minor.version(), "entry":"main.py", "args":["literal argument"]}),
            )
        } else {
            (
                "EXECUTABLE",
                "server",
                json!({"family":"EXECUTABLE", "entry":"server", "args":["literal argument"]}),
            )
        };
        let (api_url, requests) = spawn_edge_build_handoff_mock_with_build(
            deployment_id,
            declaration.clone(),
            Some("true".into()),
        );
        let project = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        let selector = minor.map_or_else(String::new, |minor| {
            format!("python_version='{}'\n", minor.version())
        });
        fs::write(project.path().join("onreza.toml"), format!(
            "[build]\ntoolchain='node'\n[deploy]\nruntime='{}'\n{selector}entry='{entry}'\nargs=['literal argument']\n",
            family.to_ascii_lowercase(),
        )).unwrap();
        if minor.is_some() {
            fs::write(
                project.path().join(entry),
                "print('code-only serving fixture')\n",
            )
            .unwrap();
        } else {
            // Package a real host ELF; this test does not claim runtime execution.
            fs::copy("/usr/bin/true", project.path().join(entry)).unwrap();
        }
        let manifest = admitted_source_manifest(
            project.path(),
            output_dir.path(),
            &api_url,
            deployment_id,
            &target,
        );
        assert_eq!(manifest.layers.len(), 1);
        let layer = &manifest.layers[0];
        assert_eq!(layer.target, "COMPUTE");
        assert_eq!(layer.entrypoint.as_deref(), Some(entry));
        let runtime = layer.runtime_config.as_ref().unwrap();
        assert_eq!(runtime["buildRuntimeVersion"], target);
        assert!(runtime.get("runtimePolicyLineage").is_none());
        declaration.as_object_mut().unwrap().remove("entry");
        declaration.as_object_mut().unwrap().remove("pythonVersion");
        assert_eq!(runtime["applicationRuntime"], declaration);
        assert_eq!(
            nrz_runtime_artifact::source_layer_launch_for_target(Some(runtime), Some(&target))
                .unwrap()
                .profile
                .to_string(),
            nrz_source_bundle::PythonMinor::from_target(&target)
                .map_or_else(|| "EXECUTABLE".to_string(), |minor| minor.profile_name())
        );
        assert!(
            nrz_runtime_artifact::source_layer_launch_for_target(Some(runtime), Some("node-24"))
                .is_err()
        );
        assert_eq!(
            requests.lock().unwrap().as_slice(),
            [
                format!("GET /v1/deployments/{deployment_id}/runner-context"),
                format!("POST /v1/deployments/{deployment_id}/execution-context/materialize"),
                format!("POST /v1/deployments/{deployment_id}/source-v2/metadata"),
            ]
        );
    }
}

#[cfg(unix)]
#[test]
fn build_generated_package_infers_process_without_changing_frozen_runtime() {
    for (frozen_runtime, keep_start, root_html, configured_entry) in [
        (serde_json::Value::Null, true, false, true),
        (serde_json::Value::Null, true, false, false),
        (
            json!({"family":"BUN","entry":"server.js","args":["literal argument"]}),
            true,
            false,
            true,
        ),
        (
            json!({"family":"BUN","entry":"server.js","args":["literal argument"]}),
            false,
            false,
            true,
        ),
        (
            json!({"family":"BUN","entry":"server.js","args":["literal argument"]}),
            false,
            true,
            true,
        ),
    ] {
        let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
        let (api_url, _) = spawn_edge_build_handoff_mock_with_build(
            deployment_id,
            frozen_runtime.clone(),
            Some("mv package_temp.json package.json && npm install --ignore-scripts --no-audit --no-fund".into()),
        );
        let project = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        let config = if !configured_entry {
            ""
        } else if frozen_runtime.is_null() {
            "[deploy]\nentry='server.js'\n"
        } else {
            "[deploy]\nruntime='bun'\nentry='server.js'\nargs=['literal argument']\n"
        };
        fs::write(project.path().join("onreza.toml"), config).unwrap();
        let package = json!({"name":"generated-process", "version":"1.0.0", "private":true,
            "scripts": if keep_start {json!({"start":"node server.js"})} else {json!({})}});
        fs::write(
            project.path().join("package_temp.json"),
            package.to_string(),
        )
        .unwrap();
        if root_html {
            fs::write(
                project.path().join("index.html"),
                "<h1>server-owned content</h1>",
            )
            .unwrap();
        }
        fs::write(project.path().join("server.js"),
            "require('node:http').createServer((_,r)=>r.end('ok')).listen(process.env.PORT || 3000)").unwrap();
        if !frozen_runtime.is_null() {
            fs::write(
                project.path().join("package.json"),
                r#"{"scripts":{"start":"bun server.js"}}"#,
            )
            .unwrap();
        }
        let target = if frozen_runtime.is_null() {
            "node-24"
        } else {
            "bun-1.4.3"
        };
        let manifest = admitted_source_manifest(
            project.path(),
            output_dir.path(),
            &api_url,
            deployment_id,
            target,
        );
        assert_eq!(manifest.layers.len(), 1);
        let layer = &manifest.layers[0];
        assert_eq!(layer.target, "COMPUTE");
        assert_eq!(layer.entrypoint.as_deref(), Some("server.js"));
        let runtime = layer.runtime_config.as_ref().unwrap();
        assert_eq!(runtime["buildRuntimeVersion"], target);
        assert!(runtime.get("runtimePolicyLineage").is_none());
        let expected_intent = if frozen_runtime.is_null() {
            json!({"family":"NODE","args":[]})
        } else {
            json!({"family":"BUN","args":["literal argument"]})
        };
        assert_eq!(runtime["applicationRuntime"], expected_intent);
        let launch =
            nrz_runtime_artifact::source_layer_launch_for_target(Some(runtime), Some(target))
                .unwrap();
        assert_eq!(
            launch.profile.to_string(),
            if frozen_runtime.is_null() {
                "NODE_24"
            } else {
                "BUN"
            }
        );
        assert!(
            manifest
                .files
                .iter()
                .any(|file| file.path == "server.js" && file.role == "compute")
        );
        assert!(manifest.files.iter().all(|file| file.role != "static"));
        assert!(!project.path().join("package_temp.json").exists());
        assert!(project.path().join("package-lock.json").exists());
    }
}

#[cfg(unix)]
#[test]
fn build_generated_elysia_rejects_node_and_keeps_explicit_static_before_source_handoff() {
    for (frozen_runtime, explicit_static) in [
        (
            json!({"family":"NODE","entry":"server.js","args":[]}),
            false,
        ),
        (serde_json::Value::Null, false),
        (serde_json::Value::Null, true),
    ] {
        let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
        let (api_url, _) = spawn_edge_build_handoff_mock_with_build(
            deployment_id,
            frozen_runtime.clone(),
            Some("mv package_temp.json package.json && touch build-completed".into()),
        );
        let project = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join("onreza.toml"),
            if explicit_static {
                "[deploy]\ncompute='static'\n"
            } else if frozen_runtime.is_null() {
                "[deploy]\nentry='server.js'\n"
            } else {
                "[deploy]\nruntime='node'\nentry='server.js'\nargs=[]\n"
            },
        )
        .unwrap();
        fs::write(
            project.path().join("package.json"),
            if frozen_runtime.is_null() {
                r#"{}"#
            } else {
                r#"{"scripts":{"start":"node server.js"}}"#
            },
        )
        .unwrap();
        fs::write(
            project.path().join("package_temp.json"),
            r#"{"dependencies":{"elysia":"^1.4.0"},"scripts":{"start":"node server.js"}}"#,
        )
        .unwrap();
        fs::write(project.path().join("server.js"), "console.log('server')").unwrap();
        let output = edge_build_command(
            project.path(),
            output_dir.path(),
            &api_url,
            deployment_id,
            true,
        )
        .env("ONREZA_RUNTIME_VERSION", "node-24")
        .output()
        .unwrap();
        if explicit_static {
            successful_handoff(&output, "explicit static elysia");
            assert!(output_dir.path().join("source-manifest-v2.json").exists());
            continue;
        }
        assert!(!output.status.success());
        let result = stdout_json(&output);
        assert_eq!(result["code"], "APPLICATION_RUNTIME_INVALID");
        assert!(
            result["error"]
                .as_str()
                .unwrap()
                .contains("conflicts with framework elysia"),
            "{result}"
        );
        assert!(project.path().join("build-completed").exists());
        assert!(!project.path().join("package_temp.json").exists());
        assert!(!output_dir.path().join("source-bundle-v1.tar.zst").exists());
        assert!(
            !output_dir
                .path()
                .join("edge-build-handoff-v2.json")
                .exists()
        );
    }
}

#[test]
fn retained_platform_handoff_preserves_authored_targets_without_new_build_facts() {
    for (family, old_target, admitted_target, minor, declared_target, typed) in [
        ("BUN", "bun-1.4.2", "bun-1.4.3", None, true, true),
        ("NODE", "node-24", "node-26", None, true, true),
        ("NODE", "node-24", "node-26", None, false, true),
        ("NODE", "node-24", "node-26", None, true, false),
        ("NODE", "node-24", "node-26", None, false, false),
        (
            "PYTHON",
            "python-3.12",
            "python-3.14",
            Some("3.12"),
            true,
            true,
        ),
        (
            "PYTHON",
            "python-3.12",
            "python-3.14",
            Some("3.12"),
            false,
            true,
        ),
    ] {
        let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
        let mut declaration = json!({"family":family,"entry":"server.js","args":[]});
        if let Some(minor) = minor {
            declaration["pythonVersion"] = json!(minor);
        }
        if !typed {
            declaration = serde_json::Value::Null;
        }
        let (api_url, _) = spawn_edge_build_handoff_mock_with_build(
            deployment_id,
            declaration,
            Some(String::new()),
        );
        let project = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let minor_config = minor
            .map(|minor| format!("python_version='{minor}'\n"))
            .unwrap_or_default();
        let config = if typed {
            format!(
                "[build]\ntoolchain='node'\n[deploy]\nruntime='{}'\n{minor_config}entry='server.js'\n",
                family.to_ascii_lowercase()
            )
        } else {
            "[build]\ntoolchain='node'\n".into()
        };
        fs::write(project.path().join("onreza.toml"), config).unwrap();
        fs::write(project.path().join("server.js"), "// retained source bytes").unwrap();
        if family != "PYTHON" {
            fs::write(
                project.path().join("package.json"),
                r#"{"name":"retained","dependencies":{"retained-dep":"1.0.0"}}"#,
            )
            .unwrap();
            fs::create_dir_all(project.path().join("node_modules/retained-dep")).unwrap();
            fs::write(
                project
                    .path()
                    .join("node_modules/retained-dep/package.json"),
                r#"{"name":"retained-dep","version":"1.0.0","main":"index.js"}"#,
            )
            .unwrap();
            fs::write(
                project.path().join("node_modules/retained-dep/index.js"),
                "module.exports = 'retained dependency'",
            )
            .unwrap();
        }
        fs::create_dir(project.path().join(".onreza")).unwrap();
        let mut authored_manifest = json!({
            "version":2,"routes":[{"pattern":"^/.*$","layer":"primary"}],"layers":[
                {"name":"primary","target":"COMPUTE","directory":".","entry":"server.js",
                 "runtime":{"applicationRuntime":{"family":family,"args":[]},"buildRuntimeVersion":old_target,"network":fixture_network("PUBLIC")}}
            ]
        });
        if !typed {
            authored_manifest["layers"][0]["runtime"]
                .as_object_mut()
                .unwrap()
                .remove("applicationRuntime");
        }
        if !declared_target {
            authored_manifest["layers"][0]["runtime"]
                .as_object_mut()
                .unwrap()
                .remove("buildRuntimeVersion");
        }
        fs::write(
            project.path().join(".onreza/manifest.json"),
            serde_json::to_vec(&authored_manifest).unwrap(),
        )
        .unwrap();
        let result =
            edge_build_command(project.path(), output.path(), &api_url, deployment_id, true)
                .env("ONREZA_RUNTIME_VERSION", admitted_target)
                .output()
                .unwrap();
        if !typed && !declared_target {
            assert!(!result.status.success());
            let result = stdout_json(&result);
            assert!(
                result["error"]
                    .as_str()
                    .unwrap()
                    .contains("requires an explicit application runtime"),
                "{result}"
            );
            assert!(!output.path().join("source-bundle-v1.tar.zst").exists());
            continue;
        }
        let manifest = source_bundle_manifest(output.path(), &result, old_target);
        let runtime = manifest.layers[0].runtime_config.as_ref().unwrap();
        assert_eq!(
            runtime["buildRuntimeVersion"],
            if declared_target {
                old_target
            } else if family == "NODE" {
                "node"
            } else {
                "python"
            }
        );
        assert!(runtime.get("runtimePolicyLineage").is_none());
        if family != "PYTHON" {
            let dependency = manifest
                .files
                .iter()
                .find(|file| file.path == "node_modules/retained-dep/index.js")
                .expect("retained dependency bytes must be packaged");
            assert_eq!(dependency.role, "compute");
        }
        assert!(!project.path().join("installer-started").exists());
        assert!(!project.path().join(".onreza/python/launch.py").exists());
        assert_eq!(
            fs::read_to_string(project.path().join("server.js")).unwrap(),
            "// retained source bytes"
        );
    }
}

#[test]
fn fresh_hono_dist_handoff_keeps_primary_lineage_through_runtime_relocation() {
    let bun = assert_cmd::Command::new("bun")
        .arg("--version")
        .assert()
        .success();
    let target = format!(
        "bun-{}",
        std::str::from_utf8(&bun.get_output().stdout)
            .unwrap()
            .trim()
    );
    for (explicit_entry, authored) in [(true, false), (true, true), (false, false), (false, true)] {
        let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
        let entry = if authored {
            "app/server.mjs"
        } else {
            "server.mjs"
        };
        let declaration = json!({"family":"BUN","entry":entry,"args":[]});
        let (api_url, _) = spawn_edge_build_handoff_mock_with_commands(
            deployment_id,
            declaration,
            json!({"family":"BUN"}),
            Some("true".into()),
            Some("bun run build".into()),
            Some("dist"),
            1,
        );
        let project = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join("onreza.toml"),
            format!(
                "[build]\ntoolchain='bun'\n[deploy]\nruntime='bun'\n{}",
                if explicit_entry {
                    format!("entry='{entry}'\n")
                } else {
                    String::new()
                }
            ),
        )
        .unwrap();
        let build = if authored {
            "mkdir -p dist/app && cp source.mjs dist/app/server.mjs"
        } else {
            "mkdir -p dist && cp source.mjs dist/server.mjs"
        };
        fs::write(project.path().join("package.json"), serde_json::to_vec(&json!({"packageManager":"bun@1.4.2","dependencies":{"hono":"4.0.0"},"scripts":{"build":build,"start":format!("bun {entry}")}})).unwrap()).unwrap();
        fs::write(project.path().join("bun.lock"), "{}").unwrap();
        fs::create_dir_all(project.path().join("node_modules/hono")).unwrap();
        fs::write(
            project.path().join("node_modules/hono/package.json"),
            r#"{"name":"hono","version":"4.0.0","type":"module","main":"index.js"}"#,
        )
        .unwrap();
        fs::write(
            project.path().join("node_modules/hono/index.js"),
            "export default 1",
        )
        .unwrap();
        fs::write(project.path().join("source.mjs"), "export default 'ready'").unwrap();
        if authored {
            fs::create_dir_all(project.path().join("dist/.onreza")).unwrap();
            fs::create_dir_all(project.path().join("dist/worker")).unwrap();
            fs::write(
                project.path().join("dist/worker/sibling.js"),
                "export default 2",
            )
            .unwrap();
            fs::write(
                project.path().join("dist/.onreza/manifest.json"),
                serde_json::to_vec(&json!({
                    "version":2,"routes":[{"pattern":"^/.*$","layer":"primary"}],"layers":[
                        {"name":"sibling","target":"COMPUTE","directory":"worker","entry":"sibling.js",
                         "runtime":{"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-22","network":fixture_network("PRIVATE")}},
                        {"name":"primary","target":"COMPUTE","directory":"app","entry":"server.mjs",
                         "runtime":{"applicationRuntime":{"family":"BUN","args":[]},"buildRuntimeVersion":"bun-1.4.2","network":fixture_network("PUBLIC")}}
                    ]
                }))
                .unwrap(),
            )
            .unwrap();
        }
        let result =
            edge_build_command(project.path(), output.path(), &api_url, deployment_id, true)
                .env("ONREZA_BUILD_RUNTIME_VERSION", &target)
                .env("ONREZA_BUILD_NODE_MAJOR", "24")
                .env("ONREZA_RUNTIME_VERSION", &target)
                .output()
                .unwrap();
        let manifest = source_bundle_manifest(output.path(), &result, "Hono dist/server.mjs");
        let primary = manifest
            .layers
            .iter()
            .find(|layer| layer.entrypoint.as_deref() == Some(format!("dist/{entry}").as_str()))
            .unwrap();
        let runtime = primary.runtime_config.as_ref().unwrap();
        assert_eq!(runtime["buildRuntimeVersion"], target);
        assert!(runtime.get("runtimePolicyLineage").is_none());
        if authored {
            assert_eq!(primary.name, "primary");
            let sibling = manifest
                .layers
                .iter()
                .find(|layer| layer.name == "sibling")
                .unwrap();
            let runtime = sibling.runtime_config.as_ref().unwrap();
            assert_eq!(runtime["buildRuntimeVersion"], "node-22");
            assert!(runtime.get("runtimePolicyLineage").is_none());
        }
        assert!(
            manifest
                .files
                .iter()
                .any(|file| file.path == format!("dist/{entry}"))
        );
    }
}

#[test]
fn fresh_python_module_handoff_keeps_project_owned_bootstrap_with_dist_output() {
    let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
    let (api_url, _) = spawn_edge_build_handoff_mock_with_commands(
        deployment_id,
        json!({"family":"PYTHON","pythonVersion":"3.13","entry":".onreza/python/launch.py","args":["MODULE","app"]}),
        json!({"family":"PYTHON","pythonVersion":"3.13"}),
        Some("true".into()),
        Some(String::new()),
        Some("dist"),
        1,
    );
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    fs::write(project.path().join("onreza.toml"), "[build]\ntoolchain='python'\npython_version='3.13'\n[deploy]\nruntime='python'\npython_version='3.13'\nmodule='app'\n").unwrap();
    fs::write(
        project.path().join("requirements.txt"),
        "retained-dep==1.0\n",
    )
    .unwrap();
    fs::write(
        project.path().join("app.py"),
        "import retained_dep\nprint(retained_dep.VALUE)\n",
    )
    .unwrap();
    let dependencies = project.path().join(".onreza/python/3.14/site-packages");
    fs::create_dir_all(&dependencies).unwrap();
    fs::write(
        dependencies.join("retained_dep.py"),
        "VALUE = 'BOOTSTRAP_OK'\n",
    )
    .unwrap();
    fs::create_dir(project.path().join("dist")).unwrap();
    fs::write(project.path().join("dist/asset.txt"), "build output").unwrap();
    let result = edge_build_command(project.path(), output.path(), &api_url, deployment_id, true)
        .env("ONREZA_BUILD_RUNTIME_FAMILY", "python")
        .env("ONREZA_BUILD_RUNTIME_VERSION", "python-3.14")
        .env("ONREZA_RUNTIME_VERSION", "python-3.14")
        .output()
        .unwrap();
    let manifest = source_bundle_manifest(output.path(), &result, "project-owned Python bootstrap");
    let primary = &manifest.layers[0];
    assert_eq!(
        primary.entrypoint.as_deref(),
        Some(".onreza/python/launch.py")
    );
    let runtime = primary.runtime_config.as_ref().unwrap();
    assert_eq!(runtime["buildRuntimeVersion"], "python");
    assert!(runtime.get("runtimePolicyLineage").is_none());
    assert!(manifest.files.iter().any(|file| file.path == "app.py"));
    let source_tree = output
        .path()
        .join(nrz_source_bundle::EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY);
    fs::remove_dir_all(project.path()).unwrap();
    let executed = std::process::Command::new("python3")
        .args(["-S", primary.entrypoint.as_deref().unwrap()])
        .args(
            runtime["applicationRuntime"]["args"]
                .as_array()
                .unwrap()
                .iter()
                .map(|argument| argument.as_str().unwrap()),
        )
        .current_dir(&source_tree)
        .output()
        .unwrap();
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&executed.stdout).trim(),
        "BOOTSTRAP_OK"
    );
}

#[test]
fn fresh_python_dependencies_use_resolved_minors_without_rejecting_requested_lines() {
    for (fresh_install, fresh_build) in [(true, false), (false, true), (false, false)] {
        let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
        let (api_url, _) = spawn_edge_build_handoff_mock_with_commands(
            deployment_id,
            json!({"family":"PYTHON","pythonVersion":"3.13","entry":"main.py","args":[]}),
            json!({"family":"PYTHON","pythonVersion":"3.12"}),
            Some(if fresh_install { "true" } else { "" }.into()),
            Some(if fresh_build { "true" } else { "" }.into()),
            None,
            1,
        );
        let project = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        fs::write(project.path().join("onreza.toml"), "[build]\ntoolchain='python'\npython_version='3.12'\n[deploy]\nruntime='python'\npython_version='3.13'\nentry='main.py'\n").unwrap();
        fs::write(project.path().join("main.py"), "print('retained code')").unwrap();
        fs::write(
            project.path().join("requirements.txt"),
            "retained-dep==1.0\n",
        )
        .unwrap();
        let dependency_root = project.path().join(".onreza/python/3.14/site-packages");
        fs::create_dir_all(&dependency_root).unwrap();
        fs::write(dependency_root.join("retained_dep.py"), "VALUE = 1").unwrap();
        fs::write(project.path().join(".onreza/manifest.json"), serde_json::to_vec(&json!({
            "version":2,"routes":[{"pattern":"^/.*$","layer":"server"}],"layers":[
                {"name":"server","target":"COMPUTE","directory":".","entry":"main.py",
                 "runtime":{"network":fixture_network("PUBLIC"),"applicationRuntime":{"family":"PYTHON","args":[]},
                    "buildRuntimeVersion":"python-3.14"}}
            ]
        })).unwrap()).unwrap();
        let old_dependency_root = project.path().join(".onreza/python/3.13/site-packages");
        fs::create_dir_all(&old_dependency_root).unwrap();
        fs::write(old_dependency_root.join("old_dep.py"), "OLD_ABI_BYTES = 1").unwrap();
        let result =
            edge_build_command(project.path(), output.path(), &api_url, deployment_id, true)
                .env("ONREZA_BUILD_RUNTIME_FAMILY", "python")
                .env("ONREZA_BUILD_RUNTIME_VERSION", "python-3.14")
                .env("ONREZA_RUNTIME_VERSION", "python-3.14")
                .output()
                .unwrap();
        let manifest = source_bundle_manifest(output.path(), &result, "python-3.14");
        assert!(
            manifest
                .files
                .iter()
                .all(|file| !file.path.ends_with("old_dep.py"))
        );
        let runtime = manifest.layers[0].runtime_config.as_ref().unwrap();
        assert_eq!(runtime["buildRuntimeVersion"], "python-3.14");
        assert!(runtime.get("runtimePolicyLineage").is_none());
        assert!(
            manifest
                .files
                .iter()
                .any(|file| file.path.ends_with("retained_dep.py")
                    && file.role
                        == if fresh_install || fresh_build {
                            "dependency"
                        } else {
                            "compute"
                        })
        );
    }
}

fn admitted_source_manifest(
    project: &std::path::Path,
    output: &std::path::Path,
    api_url: &str,
    deployment_id: &str,
    target: &str,
) -> SourceV2Manifest {
    let result = edge_build_command(project, output, api_url, deployment_id, true)
        .env("ONREZA_RUNTIME_VERSION", target)
        .output()
        .unwrap();
    source_bundle_manifest(output, &result, target)
}

fn fixture_network(exposure: &str) -> serde_json::Value {
    json!({"schemaVersion":"COMPUTE_NETWORK_V1","exposure":exposure,
        "ports":[{"name":"http","protocol":"http","port":3000,"binding":"primary"}],
        "readiness":{"protocol":"tcp","port":"http"},"allowFrom":[]})
}
