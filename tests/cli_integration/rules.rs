use super::{stdout_json, support};
use axum::{Json, Router, body::Bytes, http::HeaderMap, routing::any};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};

const PROJECT: &str = "00000000-0000-0000-0000-000000000001";
const ENVIRONMENT: &str = "00000000-0000-0000-0000-000000000002";
const FILE: &str = "onreza.rules.toml";
const RULES: &str = r#"schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }
[[rules]]
id = "allow-home"
enabled = true
condition = {}
action = { type = "allow" }
"#;

fn authoring() -> Value {
    json!({"schemaVersion":"EDGE_RULE_SET_V1", "source":{"origin":"build"},
        "rules":[{"id":"allow-home", "enabled":true, "condition":{}, "action":{"type":"allow"}}]})
}

fn rules_api(suffix: &str, response: Value) -> (String, Receiver<(String, Value)>) {
    let (sender, receiver) = channel();
    let resolver = nrz_api::Resolve200Response {
        context: nrz_api::Resolve200ResponseContext {
            project_id: PROJECT.parse().unwrap(),
            environment_id: ENVIRONMENT.parse().unwrap(),
            environment_name: "preview".into(),
            selection_source: nrz_api::ResolveRequestBodySelectionSource::Explicit,
            ..Default::default()
        },
        ..Default::default()
    };
    let app = Router::new()
        .route(&format!("/v1/projects/{PROJECT}/execution-context/resolve"), axum::routing::post(
            move |headers: HeaderMap, Json(body): Json<Value>| {
                let resolver = resolver.clone();
                async move {
                    assert_eq!(headers["X-API-Key"], "test-token");
                    assert_eq!(body, json!({"environment":"preview", "sourceRef":null, "selectionSource":"EXPLICIT"}));
                    Json(resolver)
                }
            }
        ))
        .route(&format!("/v1/projects/{PROJECT}/function-activations/environments/{ENVIRONMENT}/{suffix}"), any(
            move |method: axum::http::Method, headers: HeaderMap, body: Bytes| {
                let sender = sender.clone();
                let response = response.clone();
                async move {
                    assert_eq!(headers["X-API-Key"], "test-token");
                    let body = if body.is_empty() { Value::Null } else { serde_json::from_slice(&body).unwrap() };
                    sender.send((method.to_string(), body)).unwrap();
                    Json(response)
                }
            }
        ));
    (support::api_mock::spawn(app), receiver)
}

fn command(
    project: &Path,
    api: &str,
    mode: &str,
    subcommand: &str,
    flags: &[&str],
) -> std::process::Command {
    let mut command = std::process::Command::new(assert_cmd::cargo::cargo_bin!("nrz"));
    command
        .current_dir(project)
        .env("NRZ_API_URL", api)
        .env("NRZ_HUMAN", "false")
        .env("NRZ_TOKEN", "test-token")
        .args([mode, "rules", subcommand])
        .arg(project)
        .args(flags);
    if subcommand != "check" {
        command.args(["--project-id", PROJECT, "--environment", "preview"]);
    }
    command
}

fn active_rules() -> Value {
    serde_json::to_value(nrz_api::ActiveEdgeRulesResponse {
        rule_set: Some(nrz_api::ActiveEdgeRulesResponseRuleSet {
            environment_id: ENVIRONMENT.parse().unwrap(),
            version: 7,
            rules: json!([{"id":"allow-home", "position":0, "enabled":true, "condition":{}, "action":{"type":"allow"}}]),
            image_sources: json!([]),
            checksum: "a".repeat(64),
            ..Default::default()
        }),
        ..Default::default()
    }).unwrap()
}

fn received(requests: &Receiver<(String, Value)>, method: &str) -> Value {
    let (actual, body) = requests
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("rules request missing");
    assert_eq!(actual, method);
    body
}

#[test]
fn check_validates_the_selected_directory_without_authentication() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join(FILE), RULES).unwrap();
    for mode in ["--json", "--human"] {
        let output = command(project.path(), "http://127.0.0.1:1", mode, "check", &[])
            .env_remove("NRZ_TOKEN")
            .env_remove("NRZ_TOKEN_FILE")
            .env_remove("NRZ_WORKSPACE")
            .env("XDG_CONFIG_HOME", project.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        if mode == "--json" {
            assert_eq!(
                stdout_json(&output),
                json!({"path":project.path().join(FILE), "ruleCount":1,
                "imageSourceCount":0, "rules":[{"id":"allow-home", "position":0,"action":"allow","enabled":true}]})
            );
        } else {
            assert!(output.stdout.is_empty());
            let text = String::from_utf8_lossy(&output.stderr);
            assert!(
                text.contains("allow-home")
                    && text.contains("enabled")
                    && text.contains("rules check passed"),
                "{text}"
            );
        }
    }
}

#[test]
fn pull_creates_missing_files_and_requires_force_to_replace_existing_files() {
    let (api, requests) = rules_api("edge-rules", active_rules());
    let project = tempfile::tempdir().unwrap();
    let path = project.path().join(FILE);
    let output = command(project.path(), &api, "--json", "pull", &[])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(received(&requests, "GET").is_null());
    assert_eq!(
        stdout_json(&output),
        json!({"path":path,"environmentId":ENVIRONMENT,
        "ruleCount":1,"imageSourceCount":0,"version":7,"source":"BUILD","checksum":"a".repeat(64)})
    );
    let content = std::fs::read_to_string(&path).unwrap();
    let loaded: Value = toml::from_str(&content).unwrap();
    assert_eq!(loaded["rules"][0]["id"], "allow-home");
    assert!(loaded["rules"][0].get("position").is_none());
    for mode in ["--json", "--human"] {
        std::fs::write(&path, "previous").unwrap();
        let rejected = command(project.path(), &api, mode, "pull", &[])
            .output()
            .unwrap();
        assert_eq!(rejected.status.code(), Some(1));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "previous");
        if mode == "--json" {
            assert_eq!(stdout_json(&rejected)["code"], "FILE_EXISTS");
        } else {
            assert!(String::from_utf8_lossy(&rejected.stderr).contains("pass --force"));
        }
        received(&requests, "GET");
        let forced = command(project.path(), &api, mode, "pull", &["--force"])
            .output()
            .unwrap();
        assert!(forced.status.success(), "{forced:?}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        received(&requests, "GET");
    }
}

#[cfg(unix)]
#[test]
fn pull_refuses_symbolic_links_even_with_force() {
    let (api, requests) = rules_api("edge-rules", active_rules());
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("protected.toml");
    std::fs::write(&target, "outside").unwrap();
    let path = project.path().join(FILE);
    std::os::unix::fs::symlink(&target, &path).unwrap();
    for flags in [&[][..], &["--force"][..]] {
        let output = command(project.path(), &api, "--json", "pull", flags)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(stdout_json(&output)["code"], "UNSAFE_FILE_TARGET");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "outside");
        assert!(
            std::fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        received(&requests, "GET");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn pull_interactive_confirmation_accepts_yes_and_preserves_files_on_no_or_eof() {
    let (api, requests) = rules_api("edge-rules", active_rules());
    let project = tempfile::tempdir().unwrap();
    let path = project.path().join(FILE);
    for answer in ["yes\n", "n\n", "\u{4}"] {
        std::fs::write(&path, "previous").unwrap();
        let mut cmd = command(project.path(), &api, "--human", "pull", &[]);
        let output = super::terminal::output(&mut cmd, answer);
        let text = String::from_utf8_lossy(&output.stderr);
        assert!(text.contains("Overwrite"), "{text}");
        assert_eq!(output.status.success(), answer == "yes\n", "{output:?}");
        if answer == "yes\n" {
            assert!(text.contains("pulled 1 edge rule(s)"), "{text}");
            assert_ne!(std::fs::read_to_string(&path).unwrap(), "previous");
        } else {
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "previous");
        }
        received(&requests, "GET");
    }
}

#[test]
fn publish_sends_authored_rules_and_force_without_other_function_mutations() {
    let response = serde_json::to_value(nrz_api::FunctionsPublishResponse {
        project_id: PROJECT.into(),
        environment_id: ENVIRONMENT.into(),
        deployment_id: uuid::Uuid::from_u128(3),
        edge_rule_set_published: true,
        ..Default::default()
    })
    .unwrap();
    for force in [false, true] {
        let (api, requests) = rules_api("functions/publish", response.clone());
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join(FILE), RULES).unwrap();
        let flags = if force {
            &["--force-rules"][..]
        } else {
            &[][..]
        };
        let output = command(project.path(), &api, "--json", "publish", flags)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            received(&requests, "POST"),
            json!({"origin":"CLI", "edgeRules":authoring(), "edgeRulesForce":force})
        );
        assert_eq!(
            stdout_json(&output),
            json!({"environmentId":ENVIRONMENT,
            "ruleCount":1,"imageSourceCount":0,"result":response})
        );
    }
}

#[test]
fn publish_reports_local_validation_before_rejecting_authentication() {
    let project = tempfile::tempdir().unwrap();
    for (content, expected) in [
        (None, "not found"),
        (Some("invalid = ["), "failed to parse"),
    ] {
        if let Some(content) = content {
            std::fs::write(project.path().join(FILE), content).unwrap();
        }
        let output = command(
            project.path(),
            "http://127.0.0.1:1",
            "--json",
            "publish",
            &[],
        )
        .env("NRZ_TOKEN", "invalid\ntoken")
        .output()
        .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            stdout_json(&output)["error"]
                .as_str()
                .unwrap()
                .contains(expected),
            "{output:?}"
        );
    }
}

#[test]
fn status_sends_valid_missing_and_invalid_local_rules_and_preserves_both_outputs() {
    for state in ["valid", "missing", "invalid"] {
        let response = json!({"environmentId":ENVIRONMENT,"status":"diverged",
            "active":{"present":true,"ruleCount":2,"imageSourceCount":0,"version":7,"source":"UI",
                "checksum":"a".repeat(64),"publishedAt":"2026-10-08T00:00:00Z"},
            "local":if state=="valid" {json!({"present":true,"invalid":false,"ruleCount":1,"imageSourceCount":0,"checksum":"b".repeat(64)})}
                else {json!({"present":false,"invalid":state=="invalid"})}});
        let (api, requests) = rules_api("edge-rules/status", response.clone());
        let project = tempfile::tempdir().unwrap();
        let path = project.path().join(FILE);
        match state {
            "valid" => std::fs::write(&path, RULES).unwrap(),
            "invalid" => std::fs::write(&path, "invalid = [").unwrap(),
            _ => {}
        }
        for mode in ["--json", "--human"] {
            let output = command(project.path(), &api, mode, "status", &[])
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            let body = received(&requests, "POST");
            assert_eq!(
                body,
                match state {
                    "valid" => json!({"edgeRules":authoring()}),
                    "invalid" => json!({"localInvalid":true}),
                    _ => json!({}),
                }
            );
            if mode == "--json" {
                let result = stdout_json(&output);
                for key in ["environmentId", "status", "active", "local"] {
                    assert_eq!(result[key], response[key]);
                }
                assert_eq!(result["localFile"]["path"], json!(path));
                assert_eq!(result["localFile"]["valid"], state == "valid");
                assert_eq!(
                    result["localFile"]["ruleCount"],
                    if state == "valid" {
                        json!(1)
                    } else {
                        Value::Null
                    }
                );
                let error = &result["localFile"]["error"];
                match state {
                    "valid" => assert!(error.is_null()),
                    "missing" => assert_eq!(error, "not found"),
                    _ => assert!(error.as_str().unwrap().contains("failed to parse")),
                }
            } else {
                assert!(output.stdout.is_empty());
                let text = String::from_utf8_lossy(&output.stderr);
                assert!(
                    text.contains("Status: diverged")
                        && text.contains("UI")
                        && text.contains("Local file:"),
                    "{text}"
                );
                for detail in [
                    "Active: 2 rule(s)",
                    "0 image source(s)",
                    "v7",
                    "published 2026-10-08T00:00:00Z",
                    &format!("sha256 {}", "a".repeat(64)),
                ] {
                    assert!(text.contains(detail), "missing {detail}: {text}");
                }
                match state {
                    "valid" => {
                        assert!(text.contains("1 rule(s), valid"), "{text}");
                        assert!(text.contains("Local: 1 rule(s)"), "{text}");
                        assert!(
                            text.contains(&format!("sha256 {}", "b".repeat(64))),
                            "{text}"
                        );
                        assert!(text.contains(&path.display().to_string()), "{text}");
                    }
                    "missing" => assert!(
                        text.contains("not found") && text.contains("Local: absent"),
                        "{text}"
                    ),
                    _ => assert!(
                        text.contains("failed to parse") && text.contains("Local: invalid"),
                        "{text}"
                    ),
                }
            }
        }
    }
}
