use super::*;
use axum::{
    Json, Router,
    extract::Path,
    routing::{delete, get, post, put},
};
use serde_json::Value;

const PROJECT_ID: &str = "00000000-0000-0000-0000-000000000001";
const ENVIRONMENT_ID: &str = "00000000-0000-0000-0000-000000000002";

const SERVICE_ID: &str = "00000000-0000-0000-0000-000000000006";
const WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000004";
const SOURCE_ID: &str = "00000000-0000-0000-0000-000000000007";
const CONFIG_PATH: &str = "/v1/projects/{id}/services/{service}/config";

fn config_context(environment: Option<&str>) -> Value {
    json!({
        "workspaceId": WORKSPACE_ID, "productId": PROJECT_ID, "serviceId": SERVICE_ID,
        "sourceId": SOURCE_ID, "workspaceBindingGeneration":"1",
        "environment": environment.map(|id| json!({
            "workspaceId": WORKSPACE_ID, "productId": PROJECT_ID, "productEnvironmentId": id,
            "workspaceEnvironmentId":"00000000-0000-0000-0000-000000000005", "workspaceBindingGeneration":"1"
        }))
    })
}

fn env_command(
    api: &str,
    root: &std::path::Path,
    mode: &str,
    args: &[&str],
) -> assert_cmd::Command {
    let mut command = nrz();
    command
        .current_dir(root)
        .env("NRZ_API_URL", api)
        .env("NO_COLOR", "1")
        .args([
            mode,
            "--token",
            "test-token",
            "env",
            "--project-id",
            PROJECT_ID,
            "--service",
            SERVICE_ID,
        ])
        .args(args);
    command
}

#[test]
fn env_list_reports_service_defaults_and_masks_secrets_in_both_output_modes() {
    for empty in [false, true] {
        let mut effective = Vec::new();
        if !empty {
            for (key, value, is_secret, id) in [
                (
                    "PUBLIC",
                    "literal value",
                    false,
                    "00000000-0000-0000-0000-000000000003",
                ),
                (
                    "PRIVATE",
                    "DO_NOT_PRINT_SECRET",
                    true,
                    "00000000-0000-0000-0000-000000000004",
                ),
            ] {
                effective.push(json!({
                    "id":id, "key":key, "value":value, "isSecret":is_secret, "note":"metadata",
                    "environmentId":null, "workspaceEnvVarId":null, "scopeType":"ALL",
                    "environmentIds":[], "environmentTypes":[], "previewBranch":null,
                    "updatedAt":"2026-10-08T12:01:00Z", "inherited":false
                }));
            }
        }
        let app = Router::new().route(CONFIG_PATH, get(move |Path((product, service)):Path<(String, String)>| {
            let effective = effective.clone();
            async move {
                assert_eq!((product.as_str(), service.as_str()), (PROJECT_ID, SERVICE_ID));
                Json(json!({"context":config_context(None),"variables":effective,"effective":effective}))
            }
        }));
        let api = support::api_mock::spawn(app);
        let root = tempfile::tempdir().unwrap();
        for mode in ["--json", "--human"] {
            let output = env_command(&api, root.path(), mode, &["list"])
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            assert!(!String::from_utf8_lossy(&output.stdout).contains("DO_NOT_PRINT_SECRET"));
            assert!(!String::from_utf8_lossy(&output.stderr).contains("DO_NOT_PRINT_SECRET"));
            if mode == "--json" {
                let reply = stdout_json(&output);
                assert_eq!(reply["total"], if empty { 0 } else { 2 });
                assert_eq!(reply["serviceId"], SERVICE_ID);
                assert!(reply["environmentId"].is_null());
                assert_eq!(
                    reply["envVars"].as_array().unwrap().len(),
                    if empty { 0 } else { 2 }
                );
                if !empty {
                    let public = &reply["envVars"][0];
                    assert_eq!(public["key"], "PUBLIC");
                    assert_eq!(public["value"], "literal value");
                    assert_eq!(public["note"], "metadata");
                    assert_eq!(public["scopeType"], "ALL");
                    assert_eq!(reply["envVars"][1]["key"], "PRIVATE");
                    assert_eq!(reply["envVars"][1]["isSecret"], true);
                    assert!(reply["envVars"][1]["value"].is_null());
                }
                assert!(output.stderr.is_empty());
            } else {
                assert!(output.stdout.is_empty());
                let text = String::from_utf8(output.stderr).unwrap();
                if empty {
                    assert!(text.contains("No environment variables"));
                } else {
                    for value in ["PUBLIC", "literal value", "PRIVATE", "*****"] {
                        assert!(text.contains(value), "missing {value}: {text}");
                    }
                }
            }
        }
    }
}

#[test]
fn env_set_and_delete_address_the_default_of_one_service() {
    let app = Router::new()
        .route(CONFIG_PATH, put(|Path((product, service)):Path<(String,String)>, Json(body):Json<Value>| async move {
            assert_eq!((product.as_str(), service.as_str()), (PROJECT_ID, SERVICE_ID));
            assert_eq!(body,json!({"environmentId":null,"variables":[{"key":"PUBLIC","value":"literal value","isSecret":false,"note":"metadata"}],"replaceScope":true,"changeCategory":true,"confirmed":true}));
            Json(json!({"context":config_context(None),"message":"Updated"}))
        }))
        .route("/v1/projects/{id}/services/{service}/config/{key}",delete(|Path((product, service, key)):Path<(String,String,String)>, Json(body):Json<Value>| async move {
            assert_eq!((product.as_str(), service.as_str(), key.as_str()), (PROJECT_ID,SERVICE_ID,"PUBLIC"));
            assert_eq!(body,json!({"environmentId":null,"confirmed":true}));
            Json(json!({"context":config_context(None),"message":"Deleted"}))
        }));
    let api = support::api_mock::spawn(app);
    let root = tempfile::tempdir().unwrap();
    for mode in ["--json", "--human"] {
        for args in [
            &[
                "set",
                "PUBLIC",
                "--value",
                "literal value",
                "--plain",
                "--note",
                "metadata",
                "--all",
                "--replace-scope",
                "--change-category",
                "--yes",
            ][..],
            &["delete", "PUBLIC", "--all", "--yes"][..],
        ] {
            let output = env_command(&api, root.path(), mode, args).output().unwrap();
            assert!(output.status.success(), "{output:?}");
            if mode == "--json" {
                let reply = stdout_json(&output);
                assert_eq!(reply["key"], "PUBLIC");
                assert_eq!(reply["serviceId"], SERVICE_ID);
                assert!(reply["environmentId"].is_null());
                if args[0] == "delete" {
                    assert_eq!(reply["deleted"], true);
                }
                assert!(output.stderr.is_empty());
            } else {
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8(output.stderr).unwrap().contains("PUBLIC"));
            }
        }
    }
}

#[test]
fn env_scope_category_changes_and_deletion_require_explicit_confirmation() {
    let root = tempfile::tempdir().unwrap();
    for flag in ["--replace-scope", "--change-category"] {
        let output = env_command(
            "http://127.0.0.1:1",
            root.path(),
            "--json",
            &[
                "set", "PUBLIC", "--value", "literal", "--plain", "--all", flag,
            ],
        )
        .output()
        .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            stdout_json(&output)["error"]
                .as_str()
                .unwrap()
                .contains("confirmation required"),
            "{output:?}"
        );
    }
    {
        let args = ["delete", "PUBLIC", "--all"];
        let output = env_command("http://127.0.0.1:1", root.path(), "--json", &args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            stdout_json(&output)["error"]
                .as_str()
                .unwrap()
                .contains("confirmation required"),
            "{output:?}"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn env_scope_change_respects_terminal_confirmation() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let requests = Arc::new(AtomicUsize::new(0));
    let sent = requests.clone();
    let app = Router::new().route(
        CONFIG_PATH,
        put(
            move |Path((product, service)): Path<(String, String)>, Json(body): Json<Value>| {
                let sent = sent.clone();
                async move {
                    assert_eq!(
                        (product.as_str(), service.as_str()),
                        (PROJECT_ID, SERVICE_ID)
                    );
                    assert_eq!(body["replaceScope"], true);
                    assert_eq!(body["confirmed"], true);
                    sent.fetch_add(1, Ordering::SeqCst);
                    Json(json!({"context":config_context(None),"message":"Updated"}))
                }
            },
        ),
    );
    let api = support::api_mock::spawn(app);
    let root = tempfile::tempdir().unwrap();
    for (answer, accepted) in [("  YeS  \n", true), ("no\n", false)] {
        let mut command = std::process::Command::new(nrz().get_program());
        command
            .current_dir(root.path())
            .env("NRZ_API_URL", &api)
            .env("NO_COLOR", "1")
            .args([
                "--human",
                "--token",
                "test-token",
                "env",
                "--project-id",
                PROJECT_ID,
                "--service",
                SERVICE_ID,
                "set",
                "PUBLIC",
                "--value",
                "literal",
                "--plain",
                "--all",
                "--replace-scope",
            ]);
        let output = super::terminal::output(&mut command, answer);
        assert_eq!(output.status.success(), accepted, "{output:?}");
        assert!(output.stdout.is_empty());
        let text = String::from_utf8(output.stderr).unwrap();
        assert!(text.contains("PUBLIC"), "{text}");
        assert!(text.contains("[y/N]"), "{text}");
        assert!(
            text.contains(if accepted {
                "PUBLIC"
            } else {
                "operation cancelled"
            }),
            "{text}"
        );
    }
    assert_eq!(requests.load(Ordering::SeqCst), 1);
}

fn selected_environment_api(variables: Value) -> Router {
    let context = json!({"workspaceId":"00000000-0000-0000-0000-000000000004", "workspaceSlug":"workspace",
        "productId":PROJECT_ID, "productEnvironmentId":ENVIRONMENT_ID,
        "workspaceEnvironmentId":"00000000-0000-0000-0000-000000000005", "workspaceBindingGeneration":"1",
        "serviceId":"00000000-0000-0000-0000-000000000006", "sourceId":"00000000-0000-0000-0000-000000000007",
        "projectId":PROJECT_ID, "projectName":"project", "environmentId":ENVIRONMENT_ID,
        "environmentName":"Production", "environmentType":"PRODUCTION", "sourceRef":null, "selectionSource":"EXPLICIT"});
    let resolved = context.clone();
    Router::new()
        .route("/v1/projects/{id}/execution-context/resolve", post(move |Path(id):Path<String>,Json(body):Json<Value>| {
            let context = resolved.clone();
            async move {
                assert_eq!(id,PROJECT_ID);
                assert_eq!(body,json!({"environment":"Production", "serviceId":SERVICE_ID, "sourceRef":null, "selectionSource":"EXPLICIT"}));
                Json(json!({"protocolVersion":"execution-context-v3", "context":context}))
            }
        }))
        .route("/v1/projects/{id}/execution-context/materialize", post(move |Json(body):Json<Value>| {
            let context = context.clone();
            let variables = variables.clone();
            async move {
                assert_eq!(body,json!({"environmentId":ENVIRONMENT_ID, "serviceId":"00000000-0000-0000-0000-000000000006", "sourceRef":null, "purpose":"EXEC", "selectionSource":"EXPLICIT"}));
                Json(json!({"protocolVersion":"execution-context-v3", "context":context, "variables":variables, "secretKeys":[],
                    "snapshot":{"fingerprint":format!("v1:{}","a".repeat(64)), "resolvedAt":"2026-10-08T12:00:00Z", "source":"DESIRED_STATE", "deploymentId":null}}))
            }
        }))
}

#[test]
fn env_stdin_secret_preserves_one_newline_and_resolves_selected_write_scope() {
    let app = selected_environment_api(json!({})).route(
        CONFIG_PATH,
        put(
            |Path((product, service)): Path<(String, String)>, Json(body): Json<Value>| async move {
                assert_eq!(
                    (product.as_str(), service.as_str()),
                    (PROJECT_ID, SERVICE_ID)
                );
                assert_eq!(body["environmentId"], ENVIRONMENT_ID);
                assert_eq!(body["variables"][0]["key"], "PRIVATE");
                assert_eq!(body["variables"][0]["value"], "secret\n");
                assert_eq!(body["variables"][0]["isSecret"], true);
                assert_eq!(body["confirmed"], false);
                Json(json!({"context":config_context(Some(ENVIRONMENT_ID)),"message":"Updated"}))
            },
        ),
    );
    let api = support::api_mock::spawn(app);
    let root = tempfile::tempdir().unwrap();
    for mode in ["--json", "--human"] {
        let output = env_command(
            &api,
            root.path(),
            mode,
            &[
                "set",
                "PRIVATE",
                "--secret",
                "--stdin",
                "--environment",
                "Production",
            ],
        )
        .write_stdin("secret\n\n")
        .output()
        .unwrap();
        assert!(output.status.success(), "{output:?}");
        if mode == "--json" {
            let reply = stdout_json(&output);
            assert_eq!(reply["key"], "PRIVATE");
            assert_eq!(reply["environmentId"], ENVIRONMENT_ID);
            assert!(!String::from_utf8_lossy(&output.stdout).contains("secret"));
            assert!(output.stderr.is_empty());
        } else {
            assert!(output.stdout.is_empty());
            let text = String::from_utf8(output.stderr).unwrap();
            assert!(text.contains("PRIVATE"), "{text}");
            assert!(!text.contains("secret"), "{text}");
        }
    }
}

#[test]
fn env_validation_reports_required_and_optional_declarations_separately() {
    for (present, expected_missing, expected_present) in [
        (true, json!([]), json!(["OPTIONAL", "REQUIRED"])),
        (
            false,
            json!([{"key":"REQUIRED", "visibility":"sensitive"}]),
            json!(["OPTIONAL"]),
        ),
    ] {
        let variables = if present {
            json!({"REQUIRED":"secret", "OPTIONAL":"plain"})
        } else {
            json!({"OPTIONAL":"plain"})
        };
        let api = support::api_mock::spawn(selected_environment_api(variables));
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("onreza.toml"), "[env.declarations]\nREQUIRED = { visibility = \"sensitive\", required = true }\nOPTIONAL = { visibility = \"plain\", required = false }\n").unwrap();
        for mode in ["--json", "--human"] {
            let output = env_command(
                &api,
                root.path(),
                mode,
                &["validate", "--environment", "Production"],
            )
            .output()
            .unwrap();
            assert_eq!(
                output.status.code(),
                Some(if present { 0 } else { 1 }),
                "{output:?}"
            );
            if mode == "--json" {
                assert_eq!(
                    stdout_json(&output),
                    json!({"valid":present,"missing":expected_missing,"present":expected_present})
                );
                assert!(output.stderr.is_empty());
            } else {
                assert!(output.stdout.is_empty());
                let text = String::from_utf8(output.stderr).unwrap();
                if present {
                    assert!(
                        text.contains("All 1 required variable(s) are set"),
                        "{text}"
                    );
                } else {
                    for value in [
                        "REQUIRED (sensitive)",
                        "--secret --stdin",
                        "1 required environment variable(s) missing",
                    ] {
                        assert!(text.contains(value), "missing {value}: {text}");
                    }
                    assert!(!text.contains("OPTIONAL"), "{text}");
                }
            }
        }
    }
}
