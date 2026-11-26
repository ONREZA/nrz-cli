use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    body::Bytes,
    http::{HeaderMap, Method, StatusCode, Uri},
};
use serde_json::{Value, json};

use super::{nrz, stdout_json, support};

const DB: &str = "11111111-1111-4111-8111-111111111111";
const BRANCH: &str = "22222222-2222-4222-8222-222222222222";
const OP: &str = "019f38da-5555-7000-8000-000000000003";
const KEY: &str = "019f38da-5555-7000-8000-000000000004";
const SECRET: &str = "create-only-secret-must-never-be-printed";

type Calls = Arc<Mutex<Vec<(String, String, Option<String>)>>>;

fn receipt(kind: &str, status: &str) -> Value {
    json!({"operationId": OP, "operationType": kind, "status": status,
        "result": {"databaseId": DB, "branchId": BRANCH},
        "pendingReason": "assignment_limit", "password": SECRET})
}

fn fixture(status: u16, response: Value) -> (String, Calls) {
    fixture_with_observation(
        status,
        response,
        (StatusCode::OK, receipt("database.create", "succeeded")),
    )
}

fn fixture_with_observation(
    status: u16,
    response: Value,
    observation: (StatusCode, Value),
) -> (String, Calls) {
    let calls: Calls = Default::default();
    let observed = calls.clone();
    let app = Router::new().fallback(
        move |method: Method, uri: Uri, headers: HeaderMap, _: Bytes| {
            let calls = observed.clone();
            let response = response.clone();
            let (observation_status, observation_body) = observation.clone();
            async move {
                assert_eq!(headers["X-API-Key"], "db-fixture-token");
                calls.lock().unwrap().push((
                    method.to_string(),
                    uri.to_string(),
                    headers
                        .get("idempotency-key")
                        .map(|key| key.to_str().unwrap().to_owned()),
                ));
                // A pending Create is followed only through its durable receipt.
                if method == Method::GET && status == 202 {
                    (observation_status, Json(observation_body))
                } else {
                    (StatusCode::from_u16(status).unwrap(), Json(response))
                }
            }
        },
    );
    (support::api_mock::spawn(app), calls)
}

fn run(api: &str, args: &[&str], mode: &str) -> std::process::Output {
    let directory = tempfile::tempdir().unwrap();
    let output = nrz()
        .current_dir(directory.path())
        .env("NRZ_API_URL", api)
        .env("XDG_CONFIG_HOME", directory.path())
        .env_remove("NRZ_TOKEN_FILE")
        .env_remove("NRZ_TOKEN")
        .env_remove("NRZ_WORKSPACE")
        .env("NRZ_HUMAN", "false")
        .args([mode, "--token", "db-fixture-token", "db"])
        .args(args)
        .output()
        .unwrap();
    for bytes in [&output.stdout, &output.stderr] {
        assert!(!String::from_utf8_lossy(bytes).contains(SECRET));
        assert!(!String::from_utf8_lossy(bytes).contains("\"password\""));
    }
    output
}

#[test]
fn native_db_commands_emit_safe_receipts_in_both_modes_for_accepted_and_terminal_responses() {
    for (http, status) in [(202, "pending"), (200, "succeeded")] {
        for mode in ["--json", "--human"] {
            for (args, kind, method, path, key) in [
                (
                    vec!["create", "--name", "primary", "--idempotency-key", KEY],
                    "database.create",
                    "POST",
                    "/v1/kaiki/databases/".to_owned(),
                    Some(KEY.to_owned()),
                ),
                (
                    vec!["start", DB],
                    "endpoint.start",
                    "POST",
                    format!("/v1/kaiki/databases/{DB}/start"),
                    None,
                ),
                (
                    vec!["stop", DB],
                    "endpoint.stop",
                    "POST",
                    format!("/v1/kaiki/databases/{DB}/stop"),
                    None,
                ),
                (
                    vec!["delete", DB, "--force"],
                    "database.delete",
                    "DELETE",
                    format!("/v1/kaiki/databases/{DB}"),
                    None,
                ),
                (
                    vec![
                        "branches",
                        "create",
                        "preview",
                        "--database",
                        DB,
                        "--idempotency-key",
                        KEY,
                    ],
                    "branch.create",
                    "POST",
                    format!("/v1/kaiki/databases/{DB}/branches"),
                    Some(KEY.to_owned()),
                ),
                (
                    vec!["branches", "delete", BRANCH, "--database", DB],
                    "branch.delete",
                    "DELETE",
                    format!("/v1/kaiki/databases/{DB}/branches/{BRANCH}"),
                    None,
                ),
            ] {
                let (api, calls) = fixture(http, receipt(kind, status));
                let output = run(&api, &args, mode);
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(*calls.lock().unwrap(), vec![(method.to_owned(), path, key)]);
                if mode == "--json" {
                    let value = stdout_json(&output);
                    assert_eq!(value["operationId"], OP);
                    assert_eq!(value["status"], status);
                    assert_eq!(value["result"]["databaseId"], DB);
                } else {
                    assert!(output.stdout.is_empty());
                    let human = String::from_utf8_lossy(&output.stderr);
                    assert!(human.contains(OP) && human.contains(status));
                }
            }
        }
    }
}

#[test]
fn native_create_wait_and_deleted_database_observation_use_only_receipt_get() {
    let (api, calls) = fixture(202, receipt("database.create", "pending"));
    let output = run(&api, &["create", "--name", "primary", "--wait"], "--json");
    assert!(output.status.success());
    assert_eq!(stdout_json(&output)["status"], "succeeded");
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "POST");
    assert_eq!(
        calls[0]
            .2
            .as_ref()
            .unwrap()
            .parse::<uuid::Uuid>()
            .unwrap()
            .get_version_num(),
        7
    );
    assert_eq!(
        calls[1],
        (
            "GET".to_owned(),
            format!("/v1/kaiki/databases/{DB}/operations/{OP}"),
            None
        )
    );
    drop(calls);

    let (api, calls) = fixture(200, receipt("database.delete", "succeeded"));
    let output = run(
        &api,
        &["operation", OP, "--database", DB, "--wait"],
        "--json",
    );
    assert!(output.status.success());
    assert_eq!(stdout_json(&output)["operationId"], OP);
    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[test]
fn native_create_wait_reports_accepted_receipt_before_failed_observation() {
    for mode in ["--json", "--human"] {
        let (api, calls) = fixture_with_observation(
            202,
            receipt("database.create", "pending"),
            (
                StatusCode::SERVICE_UNAVAILABLE,
                json!({"code": "SERVICE_UNAVAILABLE", "message": "temporarily unavailable"}),
            ),
        );
        let args = ["create", "--wait", "--idempotency-key", KEY];
        let output = run(&api, &args, mode);
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        if mode == "--json" {
            assert!(stdout_json(&output)["error"].is_string());
            assert!(
                stderr.lines().any(|line| {
                    serde_json::from_str::<Value>(line).is_ok_and(|frame| {
                        frame["p"] == "db"
                            && frame["details"] == json!({"operationId": OP, "databaseId": DB})
                    })
                }),
                "{stderr}"
            );
        } else {
            assert!(output.stdout.is_empty());
            assert!(stderr.contains(&format!("nrz db operation {OP} --database {DB} --wait")));
        }
        assert_eq!(
            *calls.lock().unwrap(),
            vec![
                ("POST".into(), "/v1/kaiki/databases/".into(), Some(KEY.into())),
                (
                    "GET".into(),
                    format!("/v1/kaiki/databases/{DB}/operations/{OP}"),
                    None,
                ),
            ]
        );
    }
}

#[test]
fn native_db_denials_and_malformed_receipts_are_single_safe_json_errors() {
    for status in [400, 401, 403, 404, 409, 429] {
        let (api, calls) = fixture(
            status,
            json!({"code":"FORBIDDEN", "message":"command denied"}),
        );
        let output = run(&api, &["start", DB], "--json");
        assert_eq!(output.status.code(), Some(1));
        assert!(
            stdout_json(&output)["error"]
                .as_str()
                .unwrap()
                .contains("command denied")
        );
        assert_eq!(calls.lock().unwrap().len(), 1);
    }
    let (api, calls) = fixture(202, json!({"password": SECRET}));
    let output = run(&api, &["create"], "--json");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout_json(&output)["error"]
            .as_str()
            .unwrap()
            .contains("contract")
    );
    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[test]
fn legacy_database_commands_keep_their_200_output_shapes_and_redact_creation_passwords() {
    let created = serde_json::to_value(nrz_api::Database200Response2Object {
        id: DB.parse().unwrap(),
        db_name: "primary".to_owned(),
        password: SECRET.to_owned(),
        ..Default::default()
    })
    .unwrap();
    let branch = serde_json::to_value(nrz_api::Branch200Response3Object {
        id: BRANCH.to_owned(),
        name: "preview".to_owned(),
        ..Default::default()
    })
    .unwrap();
    for mode in ["--json", "--human"] {
        for (args, response, field, expected) in [
            (vec!["create"], created.clone(), "id", DB),
            (
                vec!["start", DB],
                json!({"operation_id": OP,"operation_type":"start", "resource":{"resource_type":"branch","resource_id":"main"},"status":"pending"}),
                "operation_id",
                OP,
            ),
            (
                vec!["stop", DB],
                json!({"operation_id": OP,"operation_type":"stop", "resource":{"resource_type":"branch","resource_id":"main"},"status":"pending"}),
                "operation_id",
                OP,
            ),
            (
                vec!["delete", DB, "--force"],
                json!({"status":"deleted"}),
                "deleted",
                DB,
            ),
            (
                vec!["branches", "create", "preview", "--database", DB],
                branch.clone(),
                "id",
                BRANCH,
            ),
            (
                vec!["branches", "delete", BRANCH, "--database", DB],
                json!({"status":"deleted"}),
                "deleted",
                BRANCH,
            ),
        ] {
            let (api, calls) = fixture(200, response);
            let output = run(&api, &args, mode);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            if mode == "--json" {
                assert_eq!(stdout_json(&output)[field], expected);
            } else {
                assert!(output.stdout.is_empty());
                assert!(!output.stderr.is_empty());
            }
            assert_eq!(calls.lock().unwrap().len(), 1);
        }
    }
}

#[test]
fn lost_create_and_fork_replies_preserve_the_generated_retry_key_in_structured_stderr() {
    use std::io::{Read, Write};

    for args in [
        vec!["create", "--name", "primary"],
        vec!["branches", "create", "preview", "--database", DB],
    ] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let api = format!("http://{}", listener.local_addr().unwrap());
        let (sent, received) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            loop {
                let mut chunk = [0; 4096];
                let read = socket.read(&mut chunk).unwrap();
                assert_ne!(read, 0);
                request.extend_from_slice(&chunk[..read]);
                if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8(request).unwrap();
            let key = request
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("idempotency-key")
                        .then(|| value.trim().to_owned())
                })
                .unwrap();
            sent.send(key).unwrap();
            // Admit the request, then lose the response midway through its secret-bearing body.
            let partial = format!("{{\"password\":\"{SECRET}\",");
            write!(socket, "HTTP/1.1 202 Accepted\r\nContent-Type: application/json\r\nContent-Length: 4096\r\nConnection: close\r\n\r\n{partial}").unwrap();
        });
        let output = run(&api, &args, "--json");
        assert_eq!(output.status.code(), Some(1));
        assert!(stdout_json(&output)["error"].is_string());
        let key = received
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert_eq!(key.parse::<uuid::Uuid>().unwrap().get_version_num(), 7);
        let progress = String::from_utf8_lossy(&output.stderr);
        assert!(progress.contains(&format!("--idempotency-key {key}")));
        assert!(progress.contains("\"p\":\"db\""));
        server.join().unwrap();
    }
}
