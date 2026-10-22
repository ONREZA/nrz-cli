use super::*;
use crate::test_support::serve_api;
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

const DEPLOYMENT: &str = "00000000-0000-0000-0000-000000000001";

fn source_config_json() -> Value {
    json!({"version":2,"layers":[{"name":"static","target":"STATIC","directory":"."}],"routes":[{"pattern":"^/.*","layer":"static"}]})
}

fn assert_source_retries(requests: &[Value]) {
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
    assert_eq!(requests[0]["attempt"], 3);
    assert_eq!(
        requests[0]["operationId"],
        Uuid::new_v5(&DEPLOYMENT.parse().unwrap(), b"onreza:deployment-source:3").to_string()
    );
}

#[tokio::test]
async fn source_registration_preparation_limit_preserves_human_and_json_diagnostics() {
    let required = u128::MAX.to_string();
    let details = json!({"category":"PREPARATION_LIMIT","metric":"SOURCE_ARCHIVE_BYTES",
        "required":required,"limit":"1048576","profileId":"edge-standard-v1"});
    let body = json!({"code":"RUNTIME_PREPARATION_LIMIT","message":"Preparation limit exceeded","details":details});
    let requests = Arc::new(Mutex::new(0));
    let app = Router::new().route(
        "/v1/deployments/{id}/source-v2/metadata",
        post({
            let requests = Arc::clone(&requests);
            move || {
                let body = body.clone();
                let requests = Arc::clone(&requests);
                async move {
                    *requests.lock().unwrap() += 1;
                    (StatusCode::UNPROCESSABLE_ENTITY, Json(body))
                }
            }
        }),
    );
    let (client, server) = serve_api(app).await;
    for json_mode in [false, true] {
        let error = register_deployment_source_v2_metadata(&client, DEPLOYMENT, 3,
            json!({"version":2,"layers":[{"name":"static","target":"STATIC","directory":"."}],"routes":[{"pattern":"^/.*","layer":"static"}]}), None, json_mode
        ).await.unwrap_err();
        let diagnostic = pre_source_failure_diagnostic(&error, None).unwrap();
        assert_eq!(diagnostic.code, "RUNTIME_PREPARATION_LIMIT");
        assert_eq!(diagnostic.details.as_ref(), Some(&details));
        assert!(
            diagnostic.message.contains("SOURCE_ARCHIVE_BYTES"),
            "{}",
            diagnostic.message
        );
        assert!(
            diagnostic.message.contains(&required),
            "{}",
            diagnostic.message
        );
        assert!(
            diagnostic.message.contains("1048576"),
            "{}",
            diagnostic.message
        );
    }
    assert_eq!(
        *requests.lock().unwrap(),
        2,
        "permanent rejection must not retry unchanged input"
    );
    server.abort();
}

#[tokio::test]
async fn source_v2_metadata_retries_the_same_operation_without_legacy_upload() {
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let functions = json!({
        "origin": "DEPLOYMENT",
        "functions": [],
        "edgeRulesForce": false,
        "generatedEdgeRuleSets": [{
            "producer": "nextjs-adapter",
            "version": "16.2.9",
            "edgeRules": {"schemaVersion": "EDGE_RULE_SET_V1", "rules": []},
        }],
    });
    let app = Router::new().route("/v1/deployments/{id}/source-v2/metadata", post(
        |State(requests):State<Arc<Mutex<Vec<Value>>>>, Json(body):Json<Value>| async move {
            let mut received = requests.lock().unwrap();
            received.push(body);
            if received.len() == 1 {
                (StatusCode::TOO_MANY_REQUESTS, [("retry-after","0")], Json(json!({"message":"busy"}))).into_response()
            } else { Json(json!({"id":DEPLOYMENT,"attempt":3,"registered":true})).into_response() }
        })).with_state(Arc::clone(&requests));
    let (client, server) = serve_api(app).await;
    register_deployment_source_v2_metadata(
        &client,
        DEPLOYMENT,
        3,
        source_config_json(),
        Some(functions.clone()),
        true,
    )
    .await
    .unwrap();
    let requests = requests.lock().unwrap();
    assert_source_retries(&requests);
    assert_eq!(requests[0]["functions"], functions);
    server.abort();
}

#[tokio::test]
async fn skipped_build_requires_acceptance_for_the_requested_deployment() {
    for response in [
        json!({"id":DEPLOYMENT,"accepted":false}),
        json!({"id":Uuid::nil(),"accepted":true}),
    ] {
        let app = Router::new().route("/v1/deployments/{id}/execution-context/skip-before-source", post(move |Json(body):Json<Value>| {
            let response = response.clone();
            async move {
                assert_eq!(body, json!({"protocolVersion":"execution-context-v3","attempt":2,"reason":"unchanged"}));
                Json(response)
            }
        }));
        let (client, server) = serve_api(app).await;
        assert!(
            mark_pre_source_skipped(&client, DEPLOYMENT, 2, "unchanged")
                .await
                .unwrap_err()
                .to_string()
                .contains("deployment state changed")
        );
        server.abort();
    }
}

#[tokio::test]
async fn pre_source_failure_uploads_redacted_diagnostics_and_attempt() {
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let app = Router::new().route("/v1/deployments/{id}/execution-context/fail-before-source", post(
        |State(requests):State<Arc<Mutex<Vec<Value>>>>, Json(body):Json<Value>| async move {
            requests.lock().unwrap().push(body);
            Json(json!({"id":DEPLOYMENT,"accepted":true}))
        })).with_state(Arc::clone(&requests));
    let (client, server) = serve_api(app).await;
    let error = crate::errors::CliError::new("BUILD_FAILED", "failed using secret-token")
        .details(json!({"lastError":null,"value":"secret-token"}))
        .into_anyhow();
    let redactor = ExactValueRedactor::from_values(["secret-token".to_string()]).unwrap();
    report_pre_source_failure(
        Some(&client),
        DEPLOYMENT,
        4,
        PreSourceFailureCode::BuildFailed,
        Some(&error),
        Some(&redactor),
        true,
    )
    .await;
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["attempt"], 4);
    assert_eq!(
        requests[0]["diagnostic"]["details"],
        json!({"lastError":null,"value":"[REDACTED]"})
    );
    assert!(!requests[0].to_string().contains("secret-token"));
    server.abort();
}

fn context_json() -> Value {
    json!({"workspaceId":"00000000-0000-0000-0000-000000000002", "workspaceSlug":"workspace",
        "productId":"00000000-0000-0000-0000-000000000003", "productEnvironmentId":"00000000-0000-0000-0000-000000000004",
        "workspaceEnvironmentId":"00000000-0000-0000-0000-000000000005", "workspaceBindingGeneration":"1",
        "serviceId":"00000000-0000-0000-0000-000000000006", "sourceId":"00000000-0000-0000-0000-000000000007",
        "projectId":"00000000-0000-0000-0000-000000000003", "projectName":"project",
        "environmentId":"00000000-0000-0000-0000-000000000004", "environmentName":"Production",
        "environmentType":"PRODUCTION", "sourceRef":null, "selectionSource":"EXPLICIT"})
}

fn expected_context_json() -> Value {
    let context = context_json();
    [
        "workspaceId",
        "productId",
        "productEnvironmentId",
        "workspaceEnvironmentId",
        "workspaceBindingGeneration",
        "serviceId",
        "sourceId",
    ]
    .into_iter()
    .map(|field| (field.to_owned(), context[field].clone()))
    .collect::<serde_json::Map<_, _>>()
    .into()
}

fn admission_request_json() -> Value {
    json!({"protocolVersion":"execution-context-v3", "environmentId":context_json()["environmentId"],
        "selectionSource":"EXPLICIT", "branch":"feature/a", "commitSha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "expectedContext":expected_context_json()})
}

#[tokio::test]
async fn admission_preserves_selection_and_rejects_another_scope() {
    for changed_field in [
        None,
        Some("workspaceId"),
        Some("serviceId"),
        Some("sourceId"),
        Some("productId"),
        Some("productEnvironmentId"),
        Some("workspaceEnvironmentId"),
        Some("workspaceBindingGeneration"),
    ] {
        let app = Router::new().route("/v1/projects/{id}/deployments/admit", post(move |Json(body):Json<Value>| async move {
            assert_eq!(body,admission_request_json());
            let mut context = context_json();
            if let Some(field) = changed_field { context[field] = if field == "workspaceBindingGeneration" { json!("2") } else { json!(Uuid::nil()) }; }
            Json(json!({"protocolVersion":"execution-context-v3","context":context,
                "deployment":{"id":DEPLOYMENT,"attempt":2,"status":"BUILDING","url":null},
                "snapshot":{"fingerprint":format!("v1:{}","a".repeat(64)),"resolvedAt":"2026-09-12T00:00:00Z"}}))
        }));
        let (client, server) = serve_api(app).await;
        let context: crate::execution_context::ExecutionContext =
            serde_json::from_value(context_json()).unwrap();
        let result = wire::admit(
            &client,
            &context.project_id,
            &context,
            "feature/a".into(),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            None,
        )
        .await;
        if changed_field.is_some() {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("another workspace")
            );
        } else {
            let result = result.unwrap();
            assert_eq!(result.deployment.attempt, 2);
            assert_eq!(result.deployment.id, DEPLOYMENT);
        }
        server.abort();
    }
}

#[tokio::test]
async fn admission_context_conflict_preserves_structured_error_without_retry() {
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let app = Router::new().route("/v1/projects/{id}/deployments/admit", post(
        |State(requests): State<Arc<Mutex<Vec<Value>>>>, Json(body): Json<Value>| async move {
            requests.lock().unwrap().push(body);
            (StatusCode::CONFLICT, Json(json!({"code":"CONFLICT", "message":"Execution context changed before admission"})))
        })).with_state(Arc::clone(&requests));
    let (client, server) = serve_api(app).await;
    let context: crate::execution_context::ExecutionContext =
        serde_json::from_value(context_json()).unwrap();
    let error = wire::admit(
        &client,
        &context.project_id,
        &context,
        "feature/a".into(),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        None,
    )
    .await
    .unwrap_err();
    let error = error
        .downcast_ref::<crate::api::StructuredApiError>()
        .unwrap();
    assert_eq!(error.status, StatusCode::CONFLICT);
    assert_eq!(error.code, "CONFLICT");
    assert_eq!(
        requests.lock().unwrap().as_slice(),
        [admission_request_json()]
    );
    server.abort();
}

#[tokio::test]
async fn runner_context_validates_identity_and_protocol_before_applying_settings() {
    for (id, protocol) in [
        (DEPLOYMENT, "runner-context-v6"),
        (DEPLOYMENT, "runner-context-v4"),
        (DEPLOYMENT, "runner-context-v99"),
        ("00000000-0000-0000-0000-000000000009", "runner-context-v6"),
    ] {
        let app = Router::new().route("/v1/deployments/{id}/runner-context", axum::routing::get(move || async move {
            Json(json!({"protocolVersion":protocol,"context":context_json(),
                "deployment":{"id":id,"attempt":5,"status":"BUILDING","url":null,"branch":"main","commitSha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
                "settings":{"nodeVersion":"NODE_24","sourceBuildContext":null,"frameworkPreset":null,"rootDirectory":".","gitLfsEnabled":false,"packageManager":"BUN",
                    "installCommand":null,"installCommandSource":"PRESET","buildCommand":"bun run build","buildCommandSource":"USER",
                    "outputDirectory":null,"outputDirectorySource":"DETECTED","ignoredBuildBehavior":"AUTOMATIC","ignoredBuildFolder":null,"ignoredBuildCommand":null}}))
        }));
        let (client, server) = serve_api(app).await;
        let result = wire::load_runner_context(&client, DEPLOYMENT.parse().unwrap()).await;
        if id != DEPLOYMENT {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("another deployment")
            );
        } else if protocol != "runner-context-v6" {
            assert!(result.unwrap_err().to_string().contains("OpenAPI contract"));
        } else {
            let result = result.unwrap();
            assert_eq!(result.deployment.attempt, 5);
            assert_eq!(
                result.settings.build_command.as_deref(),
                Some("bun run build")
            );
        }
        server.abort();
    }
}

#[tokio::test]
async fn runner_v6_rejects_absent_context_and_accepts_explicit_null() {
    for present in [false, true] {
        let mut settings = json!({"nodeVersion":"NODE_24","frameworkPreset":null,"rootDirectory":".","gitLfsEnabled":false,"packageManager":"NPM",
            "installCommand":null,"installCommandSource":"PRESET","buildCommand":null,"buildCommandSource":"PRESET",
            "outputDirectory":null,"outputDirectorySource":"PRESET","ignoredBuildBehavior":"AUTOMATIC","ignoredBuildFolder":null,"ignoredBuildCommand":null});
        if present {
            settings["sourceBuildContext"] = Value::Null;
        }
        let app = Router::new().route("/v1/deployments/{id}/runner-context", axum::routing::get(move || {
            let settings = settings.clone();
            async move { Json(json!({"protocolVersion":"runner-context-v6","context":context_json(),
                "deployment":{"id":DEPLOYMENT,"attempt":5,"status":"BUILDING","url":null,"branch":"main","commitSha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"settings":settings})) }
        }));
        let (client, server) = serve_api(app).await;
        let result = wire::load_runner_context(&client, DEPLOYMENT.parse().unwrap()).await;
        if present {
            let settings = result.unwrap().settings;
            assert_eq!(settings.source_build_context, None);
            assert_eq!(settings.node_version.as_deref(), Some("NODE_24"));
        } else {
            assert!(result.unwrap_err().to_string().contains("OpenAPI contract"));
        }
        server.abort();
    }
}
