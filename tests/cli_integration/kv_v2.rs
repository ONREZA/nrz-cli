use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    body::Bytes,
    http::{HeaderMap, Method, StatusCode, Uri},
    routing::any,
};
use serde_json::{Value, json};

use super::{nrz, stdout_json, support};

const STORE: &str = "00000000-0000-0000-0000-000000000001";
const BRANCH: &str = "00000000-0000-0000-0000-000000000002";
const FORK: &str = "00000000-0000-0000-0000-000000000003";

type Calls = Arc<Mutex<Vec<(String, String, Value)>>>;

fn branch(id: &str, state: &str) -> Value {
    json!({"id":id,"storeId":STORE,"name":"main","region":"ru-1","state":state,
        "batchWindowMs":100,"createdAt":"2026-11-05T00:00:00Z",
        "configurationEffectiveAt":"2026-11-05T00:00:00Z"})
}

fn store() -> Value {
    json!({"id":STORE,"workspaceId":FORK,"mainBranchId":BRANCH,"name":"test",
        "createdAt":"2026-11-05T00:00:00Z"})
}

fn fixture(response: Value, status: StatusCode) -> (String, Calls) {
    let calls: Calls = Default::default();
    let observed = calls.clone();
    let app = Router::new().fallback(any(
        move |method: Method, uri: Uri, headers: HeaderMap, body: Bytes| {
            let calls = observed.clone();
            let response = response.clone();
            async move {
                assert_eq!(headers["X-API-Key"], "kv-test-token");
                calls.lock().unwrap().push((
                    method.to_string(),
                    uri.to_string(),
                    if body.is_empty() {
                        Value::Null
                    } else {
                        serde_json::from_slice(&body).unwrap()
                    },
                ));
                (status, Json(response))
            }
        },
    ));
    (support::api_mock::spawn(app), calls)
}

fn run(api: &str, args: &[&str], mode: &str) -> std::process::Output {
    nrz()
        .env("NRZ_API_URL", api)
        .env_remove("NRZ_TOKEN")
        .env_remove("NRZ_WORKSPACE")
        .env("NRZ_HUMAN", "false")
        .args([mode, "--token", "kv-test-token", "kv-v2"])
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn kv_v2_metadata_commands_preserve_paths_pagination_identity_and_output() {
    let cases = [
        (
            vec!["regions"],
            "GET",
            "/v1/kv-v2/regions",
            json!({"regions":[{"id":"ru-1"}]}),
            Value::Null,
        ),
        (
            vec!["regions", "--store", STORE],
            "GET",
            "/v1/kv-v2/regions?storeId=00000000-0000-0000-0000-000000000001",
            json!({"regions":[{"id":"ru-1"}]}),
            Value::Null,
        ),
        (
            vec!["stores", "list", "--limit", "7", "--after", STORE],
            "GET",
            "/v1/kv-v2/stores?after=00000000-0000-0000-0000-000000000001&limit=7",
            json!({"stores":[store()],"next":null}),
            Value::Null,
        ),
        (
            vec!["stores", "info", STORE],
            "GET",
            "/v1/kv-v2/stores/00000000-0000-0000-0000-000000000001",
            store(),
            Value::Null,
        ),
        (
            vec![
                "stores",
                "create",
                "test",
                "--region",
                "ru-1",
                "--id",
                STORE,
                "--main-branch-id",
                BRANCH,
            ],
            "POST",
            "/v1/kv-v2/stores",
            json!({"store":store(),"branch":branch(BRANCH,"READY")}),
            json!({"id":STORE,"mainBranchId":BRANCH,"name":"test","region":"ru-1","batchWindowMs":100}),
        ),
        (
            vec!["stores", "delete", STORE, "--force"],
            "DELETE",
            "/v1/kv-v2/stores/00000000-0000-0000-0000-000000000001",
            json!({"success":true}),
            Value::Null,
        ),
        (
            vec!["branches", "--store", STORE, "list", "--limit", "3"],
            "GET",
            "/v1/kv-v2/stores/00000000-0000-0000-0000-000000000001/branches?limit=3",
            json!({"branches":[branch(BRANCH,"READY")],"next":null}),
            Value::Null,
        ),
        (
            vec!["branches", "--store", STORE, "info", BRANCH],
            "GET",
            "/v1/kv-v2/stores/00000000-0000-0000-0000-000000000001/branches/00000000-0000-0000-0000-000000000002",
            branch(BRANCH, "READY"),
            Value::Null,
        ),
        (
            vec![
                "branches", "--store", STORE, "create", "main", "--id", BRANCH, "--region", "ru-1",
            ],
            "POST",
            "/v1/kv-v2/stores/00000000-0000-0000-0000-000000000001/branches",
            branch(BRANCH, "READY"),
            json!({"id":BRANCH,"name":"main","region":"ru-1","batchWindowMs":100}),
        ),
        (
            vec!["branches", "--store", STORE, "delete", BRANCH, "--force"],
            "DELETE",
            "/v1/kv-v2/stores/00000000-0000-0000-0000-000000000001/branches/00000000-0000-0000-0000-000000000002",
            json!({"success":true}),
            Value::Null,
        ),
        (
            vec!["branches", "--store", STORE, "batch-window", BRANCH, "250"],
            "PATCH",
            "/v1/kv-v2/stores/00000000-0000-0000-0000-000000000001/branches/00000000-0000-0000-0000-000000000002/batch-window",
            branch(BRANCH, "READY"),
            json!({"batchWindowMs":250}),
        ),
    ];
    for (args, method, path, response, body) in cases {
        for mode in ["--json", "--human"] {
            let (api, calls) = fixture(response.clone(), StatusCode::OK);
            let output = run(&api, &args, mode);
            assert!(output.status.success(), "{args:?}: {output:?}");
            assert_eq!(
                *calls.lock().unwrap(),
                vec![(method.into(), path.into(), body.clone())]
            );
            if mode == "--json" {
                assert_eq!(stdout_json(&output), response);
                assert!(output.stderr.is_empty(), "{output:?}");
            } else {
                assert!(output.stdout.is_empty());
                assert_eq!(
                    serde_json::from_slice::<Value>(&output.stderr).unwrap(),
                    response
                );
            }
        }
    }
}

#[test]
fn kv_v2_binary_data_and_durability_use_the_generated_wire_contract() {
    let cases = [
        (
            vec!["get", "a/שלום"],
            json!({"operation":"GET","key":"YS/Xqdec15XXnQ=="}),
            json!({"status":"VALUE","value":"AP8="}),
        ),
        (
            vec!["get", "AP8=", "--base64"],
            json!({"operation":"GET","key":"AP8="}),
            json!({"status":"VALUE","value":null}),
        ),
        (
            vec!["put", "AP8=", "/wA=", "--base64"],
            json!({"operation":"PUT","key":"AP8=","value":"/wA=","durability":"BUFFERED"}),
            json!({"status":"WRITTEN","durability":"BUFFERED"}),
        ),
        (
            vec!["put", "key", "value", "--durable"],
            json!({"operation":"PUT","key":"a2V5","value":"dmFsdWU=","durability":"EXPLICIT_DURABLE"}),
            json!({"status":"WRITTEN","durability":"EXPLICIT_DURABLE"}),
        ),
        (
            vec!["delete", "key"],
            json!({"operation":"DELETE","key":"a2V5","durability":"BUFFERED"}),
            json!({"status":"WRITTEN","durability":"BUFFERED"}),
        ),
        (
            vec!["delete", "key", "--durable"],
            json!({"operation":"DELETE","key":"a2V5","durability":"EXPLICIT_DURABLE"}),
            json!({"status":"WRITTEN","durability":"EXPLICIT_DURABLE"}),
        ),
        (
            vec![
                "keys",
                "--prefix",
                "AP8=",
                "--after",
                "AP8A",
                "--base64",
                "--limit",
                "1024",
                "--max-bytes",
                "4096",
            ],
            json!({"operation":"SCAN_KEYS","prefix":"AP8=","after":"AP8A","limit":1024,"maxBytes":4096}),
            json!({"status":"KEYS","keys":["AP8A"],"next":"AP8A"}),
        ),
        (
            vec!["keys"],
            json!({"operation":"SCAN_KEYS","prefix":"","after":null,"limit":100,"maxBytes":1048576}),
            json!({"status":"KEYS","keys":[],"next":null}),
        ),
        (
            vec!["flush"],
            json!({"operation":"FLUSH"}),
            json!({"status":"FLUSHED"}),
        ),
        (
            vec!["stats"],
            json!({"operation":"STATS"}),
            json!({"status":"STATS","stats":{"logicalBytes":2,"keys":1,"customerBatches":1,"writeOperations":1,"writeBytes":2,"readOperations":0,"bufferedOperations":0,"bufferedBytes":0}}),
        ),
    ];
    for (command, request, response) in cases {
        let (api, calls) = fixture(response.clone(), StatusCode::OK);
        let mut args = vec!["data", "--store", STORE, "--branch", BRANCH];
        args.extend(command);
        let output = run(&api, &args, "--json");
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert_eq!(stdout_json(&output), response);
        assert_eq!(
            *calls.lock().unwrap(),
            vec![(
                "POST".into(),
                format!("/v1/kv-v2/stores/{STORE}/branches/{BRANCH}/request"),
                request
            )]
        );
    }
}

#[test]
fn kv_v2_fork_accepts_pending_202_and_preserves_stable_identity() {
    for status in [StatusCode::OK, StatusCode::ACCEPTED] {
        let (api, calls) = fixture(branch(FORK, "PENDING"), status);
        let output = run(
            &api,
            &[
                "branches", "--store", STORE, "fork", BRANCH, "copy", "--id", FORK,
            ],
            "--json",
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(stdout_json(&output), branch(FORK, "PENDING"));
        assert_eq!(
            *calls.lock().unwrap(),
            vec![(
                "POST".into(),
                format!("/v1/kv-v2/stores/{STORE}/branches/{BRANCH}/fork"),
                json!({"id":FORK,"name":"copy"})
            )]
        );
    }
}

#[test]
fn kv_v2_protocol_failures_are_errors_and_mutations_are_never_replayed() {
    for code in ["REJECTED", "BUSY", "OUTCOME_UNKNOWN"] {
        let (api, calls) = fixture(json!({"status":"ERROR","code":code}), StatusCode::OK);
        let output = run(
            &api,
            &[
                "data",
                "--store",
                STORE,
                "--branch",
                BRANCH,
                "put",
                "key",
                "preserve-value",
            ],
            "--json",
        );
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(stdout_json(&output)["code"], code);
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].2["value"], "cHJlc2VydmUtdmFsdWU=");
        if code == "OUTCOME_UNKNOWN" {
            assert!(
                stdout_json(&output)["error"]
                    .as_str()
                    .unwrap()
                    .contains("may have taken effect")
            );
        }
    }
}

#[test]
fn kv_v2_lost_write_response_is_unknown_and_never_replayed() {
    use std::io::{BufRead, BufReader, Read};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    for operation in ["put", "delete", "flush", "get"] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let api = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let observed = calls.clone();
        let (done, stop) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(30);
            while stop.try_recv().is_err() && Instant::now() < deadline {
                match listener.accept() {
                    Ok((socket, _)) => {
                        socket
                            .set_read_timeout(Some(Duration::from_secs(5)))
                            .unwrap();
                        let mut request = BufReader::new(socket);
                        let mut length = 0;
                        loop {
                            let mut line = String::new();
                            assert!(request.read_line(&mut line).unwrap() > 0);
                            if line == "\r\n" {
                                break;
                            }
                            if let Some(value) =
                                line.to_ascii_lowercase().strip_prefix("content-length:")
                            {
                                length = value.trim().parse::<usize>().unwrap();
                            }
                        }
                        let mut body = vec![0; length];
                        request.read_exact(&mut body).unwrap();
                        observed
                            .lock()
                            .unwrap()
                            .push(serde_json::from_slice::<Value>(&body).unwrap());
                        // The server accepted the complete operation but its response was lost.
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(1))
                    }
                    Err(error) => panic!("{error}"),
                }
            }
        });
        let mut command = vec!["data", "--store", STORE, "--branch", BRANCH, operation];
        if operation != "flush" {
            command.push("key");
        }
        if operation == "put" {
            command.push("value");
        }
        let output = run(&api, &command, "--json");
        done.send(()).unwrap();
        server.join().unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(calls.lock().unwrap().len(), 1);
        let error = stdout_json(&output);
        if operation == "get" {
            assert!(error.get("code").is_none(), "{error}");
        } else {
            assert_eq!(error["code"], "OUTCOME_UNKNOWN");
            assert!(
                error["error"]
                    .as_str()
                    .unwrap()
                    .contains("may have taken effect")
            );
        }
    }
}

#[test]
fn kv_v2_proxy_timeout_after_acceptance_is_unknown_without_replay() {
    for (status, response) in [
        (
            StatusCode::GATEWAY_TIMEOUT,
            json!({"message":"upstream timeout"}),
        ),
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"code":"INTERNAL_ERROR","message":"response failed after acceptance"}),
        ),
        (
            StatusCode::REQUEST_TIMEOUT,
            json!({"message":"request timeout"}),
        ),
        (
            StatusCode::BAD_REQUEST,
            json!({"message":"unstructured proxy failure"}),
        ),
    ] {
        let (api, calls) = fixture(response, status);
        let output = run(
            &api,
            &[
                "data", "--store", STORE, "--branch", BRANCH, "put", "key", "value",
            ],
            "--json",
        );
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(stdout_json(&output)["code"], "OUTCOME_UNKNOWN");
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].2,
            json!({"operation":"PUT","key":"a2V5","value":"dmFsdWU=","durability":"BUFFERED"})
        );
    }
}

#[test]
fn kv_v2_api_refusals_keep_the_server_error() {
    let (api, calls) = fixture(
        json!({"code":"FORBIDDEN","message":"KV write access denied"}),
        StatusCode::FORBIDDEN,
    );
    let output = run(
        &api,
        &[
            "data", "--store", STORE, "--branch", BRANCH, "put", "key", "value",
        ],
        "--json",
    );
    assert_eq!(output.status.code(), Some(1));
    let error = stdout_json(&output);
    assert!(error["code"].is_null());
    assert!(error["error"].as_str().unwrap().contains("403 FORBIDDEN"));
    assert!(
        error["error"]
            .as_str()
            .unwrap()
            .contains("KV write access denied")
    );
    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[test]
fn kv_v2_failed_creation_reports_generated_ids_and_never_replays() {
    for command in [
        vec!["stores", "create", "test", "--region", "ru-1"],
        vec![
            "branches", "--store", STORE, "create", "test", "--region", "ru-1",
        ],
        vec!["branches", "--store", STORE, "fork", BRANCH, "copy"],
    ] {
        let (api, calls) = fixture(
            json!({"code":"SERVICE_UNAVAILABLE","message":"unknown outcome"}),
            StatusCode::SERVICE_UNAVAILABLE,
        );
        let output = run(&api, &command, "--json");
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let message = stdout_json(&output)["error"].as_str().unwrap().to_owned();
        let id = calls[0].2["id"].as_str().unwrap();
        uuid::Uuid::parse_str(id).unwrap();
        assert!(message.contains(id), "{message}");
        if command[0] == "stores" {
            assert!(
                message.contains(calls[0].2["mainBranchId"].as_str().unwrap()),
                "{message}"
            );
        }
    }
}

#[test]
fn kv_v2_invalid_parameters_and_unconfirmed_retirement_do_not_reach_transport() {
    let (api, calls) = fixture(json!({"success":true}), StatusCode::OK);
    for command in [
        vec!["stores", "delete", STORE],
        vec!["branches", "list"],
        vec!["data", "get", "key"],
        vec!["branches", "--store", STORE, "delete", BRANCH],
        vec![
            "data", "--store", STORE, "--branch", BRANCH, "get", "AA", "--base64",
        ],
        vec![
            "data", "--store", STORE, "--branch", BRANCH, "get", "AB==", "--base64",
        ],
        vec!["stores", "create", "", "--region", "ru-1"],
        vec!["stores", "create", "test", "--region", "invalid region"],
    ] {
        let output = run(&api, &command, "--json");
        assert_eq!(output.status.code(), Some(1), "{command:?}: {output:?}");
        assert!(stdout_json(&output)["error"].is_string(), "{output:?}");
    }
    for command in [
        vec![
            "data", "--store", STORE, "--branch", BRANCH, "keys", "--limit", "1025",
        ],
        vec![
            "data",
            "--store",
            STORE,
            "--branch",
            BRANCH,
            "keys",
            "--max-bytes",
            "0",
        ],
        vec!["stores", "list", "--limit", "101"],
        vec!["branches", "--store", STORE, "batch-window", BRANCH, "0"],
    ] {
        let output = run(&api, &command, "--json");
        assert_eq!(output.status.code(), Some(2), "{command:?}: {output:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("error: invalid value"));
    }
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn kv_v2_workspace_flag_uses_existing_workspace_credentials() {
    let (api, calls) = fixture(json!({"regions":[]}), StatusCode::OK);
    let config = tempfile::tempdir().unwrap();
    std::fs::create_dir(config.path().join("nrz")).unwrap();
    std::fs::write(
        config.path().join("nrz/config.json"),
        json!({"workspaces":{"kv":{"token":"kv-test-token","name":"KV"}},"default_workspace":null})
            .to_string(),
    )
    .unwrap();
    let output = nrz()
        .env("NRZ_API_URL", api)
        .env("XDG_CONFIG_HOME", config.path())
        .env_remove("NRZ_TOKEN")
        .env("NRZ_HUMAN", "false")
        .args(["--json", "--workspace", "kv", "kv-v2", "regions"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(stdout_json(&output), json!({"regions":[]}));
    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[test]
fn kv_v2_data_scope_flags_are_accepted_after_the_operation() {
    let (api, calls) = fixture(json!({"status":"VALUE","value":null}), StatusCode::OK);
    let output = run(
        &api,
        &["data", "get", "key", "--store", STORE, "--branch", BRANCH],
        "--json",
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        calls.lock().unwrap()[0].2,
        json!({"operation":"GET","key":"a2V5"})
    );
}

#[test]
fn kv_v2_human_output_sanitizes_remote_terminal_controls() {
    let mut response = store();
    response["name"] = json!("a\u{009b}31m\u{001b}[31m");
    let (api, _) = fixture(response.clone(), StatusCode::OK);
    let machine = run(&api, &["stores", "info", STORE], "--json");
    assert!(machine.status.success(), "{machine:?}");
    assert_eq!(stdout_json(&machine), response);
    let human = run(&api, &["stores", "info", STORE], "--human");
    assert!(human.status.success(), "{human:?}");
    assert!(human.stdout.is_empty());
    let text = String::from_utf8(human.stderr).unwrap();
    assert!(!text.contains(['\u{009b}', '\u{001b}']), "{text}");
    assert!(text.contains("a31m\\u001b[31m"), "{text}");
}
