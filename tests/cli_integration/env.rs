use super::*;
use axum::{
    Json, Router,
    extract::Path,
    routing::{delete, get, post},
};
use serde_json::Value;

const PROJECT_ID: &str = "00000000-0000-0000-0000-000000000001";
const ENVIRONMENT_ID: &str = "00000000-0000-0000-0000-000000000002";

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
        ])
        .args(args);
    command
}

#[test]
fn env_list_preserves_json_fields_and_masks_secrets_in_human_output() {
    for empty in [false, true] {
        let variables = if empty {
            vec![]
        } else {
            vec![
                json!({"id":"00000000-0000-0000-0000-000000000003", "key":"PUBLIC", "value":"literal value", "isSecret":false, "note":"metadata",
                "scopeType":"ALL", "previewBranch":null, "environments":[],
                "createdAt":"2026-10-08T12:00:00Z", "updatedAt":"2026-10-08T12:01:00Z"}),
                json!({"id":"00000000-0000-0000-0000-000000000004", "key":"PRIVATE", "value":"DO_NOT_PRINT_SECRET", "isSecret":true, "note":null,
                "scopeType":"SELECTED", "previewBranch":"main",
                "environments":[{"id":ENVIRONMENT_ID, "name":"Production", "type":"PRODUCTION"}],
                "createdAt":"2026-10-08T12:02:00Z", "updatedAt":"2026-10-08T12:03:00Z"}),
            ]
        };
        let response = json!({"envVars":variables, "total":if empty {0} else {42}});
        let mut expected = response.clone();
        if !empty {
            for variable in expected["envVars"].as_array_mut().unwrap() {
                variable.as_object_mut().unwrap().remove("id");
            }
            expected["envVars"][1]["environments"][0]
                .as_object_mut()
                .unwrap()
                .remove("type");
        }
        let app = Router::new().route(
            "/v1/projects/{id}/env",
            get(move |Path(id): Path<String>| {
                let response = response.clone();
                async move {
                    assert_eq!(id, PROJECT_ID);
                    Json(response)
                }
            }),
        );
        let api = support::api_mock::spawn(app);
        let root = tempfile::tempdir().unwrap();
        for mode in ["--json", "--human"] {
            let output = env_command(&api, root.path(), mode, &["list"])
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            if mode == "--json" {
                assert_eq!(stdout_json(&output), expected);
                assert!(output.stderr.is_empty());
            } else {
                assert!(output.stdout.is_empty());
                let text = String::from_utf8(output.stderr).unwrap();
                if empty {
                    assert_eq!(text, "  No environment variables found.\n");
                } else {
                    for value in [
                        "PUBLIC",
                        "literal value",
                        "ALL",
                        "PRIVATE",
                        "*****",
                        "Production",
                    ] {
                        assert!(text.contains(value), "missing {value}: {text}");
                    }
                    assert!(!text.contains("DO_NOT_PRINT_SECRET"), "{text}");
                }
            }
        }
    }
}

#[test]
fn env_set_and_delete_send_confirmed_requests_and_report_results() {
    let set_response = json!({"id":ENVIRONMENT_ID, "key":"PUBLIC", "scopeType":"ALL",
        "isSecret":false, "created":true, "message":"Created"});
    let response = set_response.clone();
    let app = Router::new()
        .route(
            "/v1/projects/{id}/env",
            post(move |Path(id): Path<String>, Json(body): Json<Value>| {
                let response = response.clone();
                async move {
                    assert_eq!(id, PROJECT_ID);
                    assert_eq!(
                        body,
                        json!({"key":"PUBLIC", "value":"literal value", "isSecret":false,
                    "note":"metadata", "scopeType":"ALL", "replaceScope":true,
                    "changeCategory":true, "confirmed":true})
                    );
                    Json(response)
                }
            }),
        )
        .route(
            "/v1/projects/{id}/env/{key}",
            delete(
                |Path((id, key)): Path<(String, String)>,
                 axum::extract::Query(query): axum::extract::Query<
                    std::collections::HashMap<String, String>,
                >| async move {
                    assert_eq!(id, PROJECT_ID);
                    assert_eq!(key, "PUBLIC");
                    assert_eq!(
                        query,
                        std::collections::HashMap::from([("confirmed".into(), "true".into())])
                    );
                    Json(json!({"key":"PUBLIC", "deleted":true, "message":"Deleted"}))
                },
            ),
        );
    let api = support::api_mock::spawn(app);
    let root = tempfile::tempdir().unwrap();
    for mode in ["--json", "--human"] {
        for (args, expected, human) in [
            (
                vec![
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
                ],
                set_response.clone(),
                "Created PUBLIC",
            ),
            (
                vec!["delete", "PUBLIC", "--all", "--yes"],
                json!({"key":"PUBLIC", "deleted":true}),
                "Deleted PUBLIC",
            ),
        ] {
            let output = env_command(&api, root.path(), mode, &args)
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            if mode == "--json" {
                assert_eq!(stdout_json(&output), expected);
                assert!(output.stderr.is_empty());
            } else {
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8(output.stderr).unwrap().contains(human));
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
    for (args, message) in [
        (vec!["delete", "PUBLIC", "--yes"], "explicit --all"),
        (vec!["delete", "PUBLIC", "--all"], "confirmation required"),
    ] {
        let output = env_command("http://127.0.0.1:1", root.path(), "--json", &args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            stdout_json(&output)["error"]
                .as_str()
                .unwrap()
                .contains(message),
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
        "/v1/projects/{id}/env",
        post(move |Path(id): Path<String>, Json(body): Json<Value>| {
            let sent = sent.clone();
            async move {
                assert_eq!(id, PROJECT_ID);
                assert_eq!(body["replaceScope"], true);
                assert_eq!(body["confirmed"], true);
                sent.fetch_add(1, Ordering::SeqCst);
                Json(
                    json!({"id":ENVIRONMENT_ID, "key":"PUBLIC", "scopeType":"ALL",
                    "isSecret":false, "created":false, "message":"Updated"}),
                )
            }
        }),
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
        assert!(
            text.contains("move the one legacy definition to ALL"),
            "{text}"
        );
        assert!(text.contains("[y/N]"), "{text}");
        assert!(
            text.contains(if accepted {
                "Updated PUBLIC"
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
        "projectId":PROJECT_ID, "projectName":"project", "environmentId":ENVIRONMENT_ID,
        "environmentName":"Production", "environmentType":"PRODUCTION", "sourceRef":null, "selectionSource":"EXPLICIT"});
    let resolved = context.clone();
    Router::new()
        .route("/v1/projects/{id}/execution-context/resolve", post(move |Path(id):Path<String>,Json(body):Json<Value>| {
            let context = resolved.clone();
            async move {
                assert_eq!(id,PROJECT_ID);
                assert_eq!(body,json!({"environment":"Production", "sourceRef":null, "selectionSource":"EXPLICIT"}));
                Json(json!({"protocolVersion":"execution-context-v2", "context":context}))
            }
        }))
        .route("/v1/projects/{id}/execution-context/materialize", post(move |Json(body):Json<Value>| {
            let context = context.clone();
            let variables = variables.clone();
            async move {
                assert_eq!(body,json!({"environmentId":ENVIRONMENT_ID, "sourceRef":null, "purpose":"EXEC", "selectionSource":"EXPLICIT"}));
                Json(json!({"protocolVersion":"execution-context-v2", "context":context, "variables":variables, "secretKeys":[],
                    "snapshot":{"fingerprint":format!("v1:{}","a".repeat(64)), "resolvedAt":"2026-10-08T12:00:00Z", "source":"DESIRED_STATE", "deploymentId":null}}))
            }
        }))
}

#[test]
fn env_stdin_secret_preserves_one_newline_and_resolves_selected_write_scope() {
    let response = json!({"id":ENVIRONMENT_ID, "key":"PRIVATE", "scopeType":"SELECTED",
        "isSecret":true, "created":false, "message":"Updated"});
    let reply = response.clone();
    let app = selected_environment_api(json!({})).route(
        "/v1/projects/{id}/env",
        post(move |Path(id): Path<String>, Json(body): Json<Value>| {
            let response = reply.clone();
            async move {
                assert_eq!(id, PROJECT_ID);
                assert_eq!(
                    body,
                    json!({"key":"PRIVATE", "value":"secret\n", "isSecret":true,
                "scopeType":"SELECTED", "environmentIds":[ENVIRONMENT_ID],
                "replaceScope":false, "changeCategory":false, "confirmed":true})
                );
                Json(response)
            }
        }),
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
            assert_eq!(stdout_json(&output), response);
            assert!(output.stderr.is_empty());
        } else {
            assert!(output.stdout.is_empty());
            let text = String::from_utf8(output.stderr).unwrap();
            assert!(text.contains("Updated PRIVATE"), "{text}");
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
