use super::{nrz, stdout_json, support};
use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

const DB: &str = "11111111-1111-4111-8111-111111111111";
const WORKSPACE: &str = "22222222-2222-4222-8222-222222222222";
const OPERATION: &str = "33333333-3333-4333-8333-333333333333";
const LEGACY_DB: &str = "44444444-4444-4444-8444-444444444444";

fn accepted(status: &str) -> Value {
    json!({"id": DB, "workspaceId": WORKSPACE, "operationId": OPERATION, "status": status})
}

fn native_metadata() -> Value {
    json!({
        "providerContour": "PLATFORM_NATIVE", "id": DB, "workspaceId": WORKSPACE,
        "productId": null, "dbName": "primary", "roleName": "app", "pgVersion": 17,
        "cuSize": 0.5, "status": "ACTIVE", "createdAt": "2026-10-24T00:00:00Z",
        "updatedAt": "2026-10-24T00:00:00Z", "projectAttachments": []
    })
}

fn native_detail() -> Value {
    let mut detail = native_metadata();
    detail["plan"] = json!("HOBBY");
    detail["permissions"] = json!(["projects.read"]);
    detail
}

#[test]
fn create_reports_async_identity_and_replays_with_the_supplied_key() {
    let key = "db-create:retry-1";
    let observed = Arc::new(Mutex::new(Vec::new()));
    let calls = observed.clone();
    let app = Router::new().route(
        "/v1/kaiki/databases/",
        post(move |headers: HeaderMap, Json(body): Json<Value>| {
            let mut calls = calls.lock().unwrap();
            calls.push((headers["Idempotency-Key"].to_str().unwrap().to_owned(), body));
            let status = if calls.len() == 1 { "CREATING" } else { "ACTIVE" };
            async move { (StatusCode::ACCEPTED, Json(accepted(status))) }
        }),
    );
    let api = support::api_mock::spawn(app);
    let root = tempfile::tempdir().unwrap();
    for status in ["CREATING", "ACTIVE"] {
        let output = nrz()
            .current_dir(root.path())
            .env("NRZ_API_URL", &api)
            .args(["--json", "--token", "test-token", "db", "create", "--name", "primary", "--idempotency-key", key])
            .output().unwrap();
        assert!(output.status.success(), "{output:?}");
        let mut expected = accepted(status);
        expected["idempotencyKey"] = json!(key);
        assert_eq!(stdout_json(&output), expected);
        assert!(String::from_utf8_lossy(&output.stderr).contains(key));
    }
    let calls = observed.lock().unwrap();
    assert_eq!(calls.len(), 2);
    for (observed_key, body) in calls.iter() {
        assert_eq!(observed_key, key);
        assert_eq!(*body, json!({"dbName": "primary"}));
    }
}

#[test]
fn create_wait_preserves_info_projection_and_admission_identity_without_credentials() {
    let key = "db-create:wait-success";
    let detail = native_detail();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let posts = observed.clone();
    let reads = observed.clone();
    let app = Router::new()
        .route("/v1/kaiki/databases/", post(move |headers: HeaderMap| {
            assert_eq!(headers["Idempotency-Key"], key);
            posts.lock().unwrap().push("POST");
            async { (StatusCode::ACCEPTED, Json(accepted("CREATING"))) }
        }))
        .route(&format!("/v1/kaiki/databases/{DB}"), get(move || {
            reads.lock().unwrap().push("GET");
            let detail = detail.clone();
            async move { Json(detail) }
        }));
    let api = support::api_mock::spawn(app);
    let root = tempfile::tempdir().unwrap();
    let info = nrz().current_dir(root.path()).env("NRZ_API_URL", &api)
        .args(["--json", "--token", "test-token", "db", "info", DB])
        .output().unwrap();
    assert!(info.status.success(), "{info:?}");
    let mut expected = stdout_json(&info);
    assert_eq!(expected, native_detail());
    assert_eq!(expected["id"], DB);
    assert_eq!(expected["dbName"], "primary");
    assert_eq!(expected["status"], "ACTIVE");
    assert_eq!(expected["pgVersion"], 17);
    assert_eq!(expected["cuSize"], 0.5);
    expected["workspaceId"] = json!(WORKSPACE);
    expected["operationId"] = json!(OPERATION);
    expected["idempotencyKey"] = json!(key);
    let output = nrz().current_dir(root.path()).env("NRZ_API_URL", api)
        .args(["--json", "--token", "test-token", "db", "create", "--wait", "--idempotency-key", key])
        .output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let result = stdout_json(&output);
    assert_eq!(result, expected);
    for field in ["password", "encryptedPassword", "encryptedConnUri", "connection_uri"] {
        assert!(result.get(field).is_none(), "credential field leaked: {field}");
    }
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private-"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private-"));
    assert_eq!(*observed.lock().unwrap(), ["GET", "POST", "GET"]);
}

#[test]
fn create_errors_preserve_retry_keys_and_known_admission_without_reposting() {
    for wait in [false, true] {
        let observed = Arc::new(Mutex::new(Vec::new()));
        let calls = observed.clone();
        let app = Router::new()
            .route("/v1/kaiki/databases/", post(move |headers: HeaderMap| {
                calls.lock().unwrap().push(headers["Idempotency-Key"].to_str().unwrap().to_owned());
                async move {
                    if wait {
                        (StatusCode::ACCEPTED, Json(accepted("CREATING")))
                    } else {
                        (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"code": "SERVICE_UNAVAILABLE", "message": "unknown admission", "details": {"reason": "gateway"}})))
                    }
                }
            }))
            .route(&format!("/v1/kaiki/databases/{DB}"), get(|| async {
                (StatusCode::FORBIDDEN, Json(json!({"code": "FORBIDDEN", "message": "status unavailable", "details": {"permission": "projects.read"}})))
            }));
        let api = support::api_mock::spawn(app);
        let root = tempfile::tempdir().unwrap();
        let mut command = nrz();
        command.current_dir(root.path()).env("NRZ_API_URL", api)
            .args(["--json", "--token", "test-token", "db", "create"]);
        if wait { command.arg("--wait"); }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let error = stdout_json(&output);
        let calls = observed.lock().unwrap();
        assert_eq!(calls.len(), 1, "create must send a single POST");
        let key = &calls[0];
        assert_eq!(key.parse::<uuid::Uuid>().unwrap().get_version_num(), 7);
        assert_eq!(error["details"]["idempotencyKey"], *key);
        assert!(String::from_utf8_lossy(&output.stderr).contains(key));
        if wait {
            assert_eq!(error["code"], "FORBIDDEN");
            assert_eq!(error["details"]["permission"], "projects.read");
            assert_eq!(error["details"]["databaseId"], DB);
            assert_eq!(error["details"]["operationId"], OPERATION);
            assert_eq!(error["details"]["status"], "CREATING");
            assert!(error["hint"].as_str().unwrap().contains(DB));
        } else {
            assert_eq!(error["code"], "SERVICE_UNAVAILABLE");
            assert_eq!(error["details"]["reason"], "gateway");
            assert!(error["hint"].as_str().unwrap().contains(key));
        }
    }
}

#[test]
fn mixed_reads_and_name_selection_work_without_creation_availability() {
    let legacy = nrz_api::ManagedDatabaseLegacyListItem {
        id: LEGACY_DB.parse().unwrap(),
        db_name: "legacy".into(),
        status: nrz_api::ManagedDatabaseStatus::Active,
        encrypted_password: Some("private-legacy-password".into()),
        ..Default::default()
    };
    let legacy = serde_json::to_value(legacy).unwrap();
    for creation in [
        json!({"status": "UNAVAILABLE"}),
        json!({"status": "AVAILABLE", "defaultPgVersion": 17, "defaultCuSize": 0.5,
            "profiles": [{"pgVersion": 17, "cuSize": 0.5, "cpuMillicores": 500, "memoryBytes": 536870912}],
            "suspendTimeoutSeconds": {"default": 300, "minimum": 60, "maximum": 3600}}),
    ] {
        let list = json!({"plan": "HOBBY", "creation": creation, "data": [native_metadata(), legacy]});
        let app = Router::new()
            .route("/v1/kaiki/databases/", get(move || {
                let list = list.clone();
                async move { Json(list) }
            }))
            .route(&format!("/v1/kaiki/databases/{DB}"), get(|| async { Json(native_detail()) }));
        let api = support::api_mock::spawn(app);
        let root = tempfile::tempdir().unwrap();
        for mode in ["--json", "--human"] {
            let output = nrz().current_dir(root.path()).env("NRZ_API_URL", &api)
                .args([mode, "--token", "test-token", "db", "list"])
                .output().unwrap();
            assert!(output.status.success(), "{output:?}");
            if mode == "--json" {
                let result = stdout_json(&output);
                assert_eq!(result["creation"], creation);
                assert_eq!(result["data"][0], native_metadata());
                assert_eq!(result["data"][1]["providerContour"], "LEGACY");
                assert_eq!(result["data"][1]["id"], LEGACY_DB);
                assert_eq!(result["data"][1]["autoInjectDbUrl"], Value::Null);
                assert!(result.get("allowedCuSizes").is_none());
                assert!(!String::from_utf8_lossy(&output.stdout).contains("private-"));
            } else {
                let text = String::from_utf8_lossy(&output.stderr);
                assert!(text.contains("primary") && text.contains("legacy"));
                assert!(text.contains(if creation["status"] == "AVAILABLE" {
                    "PG 17 / 0.5 CU"
                } else {
                    "creation is unavailable"
                }));
            }
        }
        let output = nrz().current_dir(root.path()).env("NRZ_API_URL", &api)
            .args(["--json", "--token", "test-token", "db", "info", "primary"])
            .output().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(stdout_json(&output), native_detail());
        let bindings = nrz().current_dir(root.path()).env("NRZ_API_URL", api)
            .args(["--json", "--token", "test-token", "db", "config", DB, "--project-id", WORKSPACE])
            .output().unwrap();
        assert!(bindings.status.success(), "{bindings:?}");
        assert_eq!(stdout_json(&bindings), json!({"bindings": []}));
    }
}

#[test]
fn legacy_info_keeps_its_existing_projection_without_sensitive_provider_fields() {
    let detail = nrz_api::ManagedDatabaseLegacyDetail {
        id: LEGACY_DB.parse().unwrap(),
        db_name: "legacy".into(),
        status: nrz_api::ManagedDatabaseStatus::Active,
        cu_size: 0.5,
        pg_version: 17,
        kaiki_database_id: "private-provider-database".into(),
        encrypted_password: Some("private-encrypted-password".into()),
        encrypted_conn_uri: Some("private-encrypted-uri".into()),
        allowed_cu_sizes: serde_json::from_value(json!([0.5, 1.0])).unwrap(),
        autoscale_max_cu: serde_json::from_value(json!(1.0)).unwrap(),
        kaiki_status: serde_json::from_value(json!("active")).unwrap(),
        ..Default::default()
    };
    let mut detail = serde_json::to_value(detail).unwrap();
    detail["password"] = json!("private-plaintext-password");
    let app = Router::new().route(&format!("/v1/kaiki/databases/{LEGACY_DB}"), get(move || {
        let detail = detail.clone();
        async move { Json(detail) }
    }));
    let api = support::api_mock::spawn(app);
    let root = tempfile::tempdir().unwrap();
    let output = nrz().current_dir(root.path()).env("NRZ_API_URL", api)
        .args(["--json", "--token", "test-token", "db", "info", LEGACY_DB])
        .output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(stdout_json(&output), json!({
        "providerContour": "LEGACY", "id": LEGACY_DB, "productId": null,
        "dbName": "legacy", "status": "ACTIVE", "cuSize": 0.5, "pgVersion": 17,
        "autoInjectDbUrl": null, "envVarName": null, "autoCreatePreviewBranch": null,
        "kaikiStatus": "active", "projectAttachments": [], "allowedCuSizes": [0.5, 1.0],
        "autoscaleMaxCu": 1.0, "plan": "HOBBY"
    }));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private-"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private-"));
}

#[test]
fn empty_human_lists_show_creation_availability_without_unavailable_create_advice() {
    for creation in [
        json!({"status": "UNAVAILABLE"}),
        json!({"status": "AVAILABLE", "defaultPgVersion": 17, "defaultCuSize": 0.5,
            "profiles": [{"pgVersion": 17, "cuSize": 0.5, "cpuMillicores": 500, "memoryBytes": 536870912}],
            "suspendTimeoutSeconds": {"default": 300, "minimum": 60, "maximum": 3600}}),
    ] {
        let available = creation["status"] == "AVAILABLE";
        let app = Router::new().route("/v1/kaiki/databases/", get(move || {
            let creation = creation.clone();
            async move { Json(json!({"plan": "HOBBY", "creation": creation, "data": []})) }
        }));
        let api = support::api_mock::spawn(app);
        let root = tempfile::tempdir().unwrap();
        let output = nrz().current_dir(root.path()).env("NRZ_API_URL", api)
            .args(["--human", "--token", "test-token", "db", "list"])
            .output().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(output.stdout.is_empty());
        let text = String::from_utf8_lossy(&output.stderr);
        assert!(text.contains("Plan: HOBBY") && text.contains("No databases found."));
        assert_eq!(text.contains("Create one with: nrz db create"), available);
        assert!(text.contains(if available {
            "PG 17 / 0.5 CU"
        } else {
            "creation is unavailable"
        }));
    }
}
