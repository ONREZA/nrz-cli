use nrz_api::{ProjectRequestBody2, Token200Response, User200Response};
use serde_json::json;

#[test]
fn serving_pending_reason_codes_preserve_the_wire_contract() {
    let mut response = json!({
        "generation": "1", "operationId": null, "releaseId": null,
        "phase": "PENDING", "result": null, "activeEverywhere": false
    });
    let parsed: nrz_api::Serving200ResponseDesired =
        serde_json::from_value(response.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), response);

    response["pendingReason"] = json!(null);
    let parsed: nrz_api::Serving200ResponseDesired =
        serde_json::from_value(response.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), response);

    response["pendingReason"] = json!({
        "operationId": "00000000-0000-0000-0000-000000000001",
        "requestedAt": "2026-10-11T00:00:00Z", "layers": []
    });
    for code in [
        "RELEASE_MATERIALIZATION_PENDING",
        "RUNTIME_APPLICATION_UNQUALIFIED",
        "RUNTIME_POLICY_UNINITIALIZED",
    ] {
        response["pendingReason"]["code"] = json!(code);
        let parsed: nrz_api::Serving200ResponseDesired =
            serde_json::from_value(response.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), response);
    }
    for code in [json!("UNSUPPORTED_PENDING_REASON"), json!(null)] {
        response["pendingReason"]["code"] = code;
        assert!(
            serde_json::from_value::<nrz_api::Serving200ResponseDesired>(response.clone()).is_err()
        );
    }
}

#[test]
fn project_include_files_literals_preserve_string_wire_values() {
    for value in [json!(true), json!(false), json!("true"), json!("false")] {
        let parsed: nrz_api::ProjectRequestBody2IncludeFilesOutsideRoot =
            serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    }
    for value in [json!("unknown"), json!(null)] {
        assert!(
            serde_json::from_value::<nrz_api::ProjectRequestBody2IncludeFilesOutsideRoot>(value)
                .is_err()
        );
    }
}

#[test]
fn compute_config_response_matches_current_server_fields() {
    for health_check_path in [json!(null), json!("/health")] {
        let response = json!({
            "enabled": true,
            "config": {
                "computeMode": "RUNTIME", "processCpuLimitMillis": 1000,
                "idleTimeoutSeconds": 15, "healthCheckPath": health_check_path
            },
            "cpuHardCapMillis": 9000, "effectiveCpuLimitMillis": 1000
        });
        let parsed: nrz_api::ComputeConfig200Response =
            serde_json::from_value(response.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), response);
        let mut missing = response;
        missing["config"]
            .as_object_mut()
            .unwrap()
            .remove("healthCheckPath");
        assert!(serde_json::from_value::<nrz_api::ComputeConfig200Response>(missing).is_err());
    }
}

#[test]
fn historical_runtime_graph_readback_preserves_node_launch_profiles() {
    let digest = "a".repeat(64);
    for major in [22, 24, 26] {
        let graph = json!({
                "schemaVersion": "RUNTIME_ARTIFACT_GRAPH_V2.0",
                "application": {
                    "artifactId": digest, "manifestDigest": digest,
                    "blobDescriptor": {
                        "mediaType": "application/vnd.onreza.source-bundle.tar+zstd.v1",
                        "digest": format!("sha256:{digest}"), "size": 1024
                    }
                },
                "dependencies": [],
                "runtimeLayers": [{
                    "layerName": "server", "applicationRoot": "server",
                    "dependencyMaterializationIds": [], "entrypoint": "server.js",
                    "launch": {"profile": format!("NODE_{major}"), "args": [], "cwd": "."},
                    "runtimeConfig": {"runtimeFamily": "JAVASCRIPT"}
                }],
                "graphDigest": digest
        });
        let parsed: nrz_api::Status200ResponseRuntimeArtifactGraph =
            serde_json::from_value(graph.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), graph);
    }
}

#[test]
fn project_patch_preserves_omission_null_and_value() {
    let mut body: ProjectRequestBody2 = serde_json::from_value(json!({})).unwrap();
    assert_eq!(serde_json::to_value(&body).unwrap(), json!({}));
    body.git_url = Some(None);
    assert_eq!(
        serde_json::to_value(&body).unwrap(),
        json!({"gitUrl": null})
    );
    body.git_url = Some(Some("https://github.com/example/project".to_string()));
    assert_eq!(
        serde_json::to_value(&body).unwrap(),
        json!({"gitUrl": "https://github.com/example/project"})
    );
}

#[test]
fn token_defaults_do_not_hide_missing_required_fields() {
    for body in [json!({}), json!({"access_token": "secret"})] {
        assert!(serde_json::from_value::<Token200Response>(body).is_err());
    }
}

#[test]
fn user_response_nullability_matches_the_server_schema() {
    let valid = json!({
        "id": "00000000-0000-0000-0000-000000000001", "email": "test@example.com",
        "name": "Test", "username": null
    });
    let user: User200Response = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(serde_json::to_value(user).unwrap(), valid);
    let mut missing = valid;
    missing.as_object_mut().unwrap().remove("username");
    assert!(serde_json::from_value::<User200Response>(missing).is_err());
}

#[test]
fn deployment_diagnostics_preserve_recursive_json_nulls() {
    let response = json!({
        "id":"00000000-0000-0000-0000-000000000001", "status":"failed", "url":null,
        "production":false, "error":"failed", "errorCode":"BUILD_FAILED",
        "errorDetails":{"runtimeStartupFailure":{"lastError":null}, "values":[null, "text"]},
        "runtimeArtifactGraphDigest":null, "runtimeArtifactGraph":null,
        "createdAt":"2026-09-12T00:00:00Z", "readyAt":null
    });
    let parsed: nrz_api::Status200Response = serde_json::from_value(response.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), response);
}

#[test]
fn function_invocation_preserves_runtime_extensions_and_recursive_json_values() {
    let response = json!({
        "invocation": { "invocationId": "request-1", "ok": false,
            "protocolVersion": "onreza-functions-poc/v2",
            "timings": { "totalMs": 1.0, "runtimeExtra": null },
            "response": { "status": 500, "setCookie": ["session=value"] },
            "error": { "cause": null }, "logs": [{ "message": "failed", "properties": { "value": null } }] },
        "debugTrace": { "stages": [], "serverTiming": null },
        "revision": { "id": "revision-1", "functionId": "function-1", "sourceSnapshotId": "snapshot-1" }
    });
    let parsed: nrz_api::FunctionTestInvokeResponse =
        serde_json::from_value(response.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), response);
    assert!(
        serde_json::from_value::<nrz_api::FunctionTestInvokeResponse>(json!({
            "invocation": { "ok":true }, "debugTrace":null, "revision":{}
        }))
        .is_err()
    );
}

#[test]
fn invocation_headers_are_fixed_pairs_of_strings() {
    let request: nrz_api::TestInvokeRequestBody = serde_json::from_value(json!({
        "headers":[["accept","application/json"]]
    }))
    .unwrap();
    assert_eq!(
        request.headers.unwrap(),
        vec![("accept".to_owned(), "application/json".to_owned())]
    );
    for pair in [
        json!([]),
        json!(["name"]),
        json!(["name", "value", "extra"]),
        json!(["name", 2]),
    ] {
        assert!(
            serde_json::from_value::<nrz_api::TestInvokeRequestBody>(json!({"headers":[pair]}))
                .is_err()
        );
    }
}
