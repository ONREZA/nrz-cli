use super::*;
use axum::{Json, Router, extract::Path, routing::post};
use serde_json::{Value, json};

const PROJECT: &str = "00000000-0000-0000-0000-000000000001";
const ENVIRONMENT: &str = "00000000-0000-0000-0000-000000000002";
const SECOND_ENVIRONMENT: &str = "00000000-0000-0000-0000-000000000003";

#[test]
fn scope_display_preserves_selected_names_and_legacy_labels() {
    for (label, scope, environments, expected) in [
        ("all", Some("ALL"), json!([]), "ALL"),
        ("missing scope", None, json!([]), "ALL"),
        (
            "selected without targets",
            Some("SELECTED"),
            json!([]),
            "SELECTED",
        ),
        (
            "selected names and ID fallback",
            Some("SELECTED"),
            json!([
                {"id":ENVIRONMENT,"name":"Production"},
                {"id":SECOND_ENVIRONMENT,"name":null}
            ]),
            "Production, 00000000-0000-0000-0000-000000000003",
        ),
        (
            "unknown scope",
            Some("FUTURE_SCOPE"),
            json!([]),
            "FUTURE_SCOPE",
        ),
    ] {
        let mut variable = json!({"key":"TOKEN","isSecret":true,"environments":environments});
        if let Some(scope) = scope {
            variable["scopeType"] = json!(scope);
        }
        let variable: EnvVar = serde_json::from_value(variable).unwrap();
        assert_eq!(format_scope(&variable), expected, "{label}");
    }
}

async fn assert_set_request(request: SetEnvRequest<'_>, expected: Value) {
    let (sent, received) = std::sync::mpsc::channel();
    let response = json!({
        "id":ENVIRONMENT,"key":"TOKEN","scopeType":request.scope_type,
        "isSecret":request.is_secret,"created":true,"message":"Created"
    });
    let app = Router::new().route(
        "/v1/projects/{id}/env",
        post(
            move |Path(project): Path<String>, Json(body): Json<Value>| {
                let sent = sent.clone();
                let response = response.clone();
                async move {
                    sent.send((project, body)).unwrap();
                    Json(response)
                }
            },
        ),
    );
    let (client, server) = crate::test_support::serve_api(app).await;
    set(&client, request).await.unwrap();
    let (project, body) = received.try_recv().expect("set must send an API request");
    assert_eq!(project, PROJECT);
    assert_eq!(body, expected);
    assert!(
        received.try_recv().is_err(),
        "set must send exactly one request"
    );
    server.abort();
}

#[tokio::test]
async fn set_secret_for_all_environments_sends_explicit_false_flags() {
    assert_set_request(
        SetEnvRequest {
            project_id: PROJECT,
            key: "TOKEN",
            value: "secret-value",
            is_secret: true,
            note: None,
            scope_type: "ALL",
            environment_ids: None,
            replace_scope: false,
            change_category: false,
            json: false,
        },
        json!({
            "key":"TOKEN","value":"secret-value","isSecret":true,"scopeType":"ALL",
            "replaceScope":false,"changeCategory":false,"confirmed":true
        }),
    )
    .await;
}

#[tokio::test]
async fn set_plain_for_selected_environments_preserves_ids_note_and_safety_flags() {
    assert_set_request(
        SetEnvRequest {
            project_id: PROJECT,
            key: "TOKEN",
            value: "a&b + /שלום\nline two",
            is_secret: false,
            note: Some("visible metadata"),
            scope_type: "SELECTED",
            environment_ids: Some(vec![ENVIRONMENT.into(), SECOND_ENVIRONMENT.into()]),
            replace_scope: true,
            change_category: true,
            json: false,
        },
        json!({
            "key":"TOKEN","value":"a&b + /שלום\nline two","isSecret":false,
            "note":"visible metadata","scopeType":"SELECTED",
            "environmentIds":["00000000-0000-0000-0000-000000000002","00000000-0000-0000-0000-000000000003"],
            "replaceScope":true,"changeCategory":true,"confirmed":true
        }),
    )
    .await;
}

#[test]
fn safety_prompt_describes_exact_scope_and_category_changes() {
    let selected = vec![ENVIRONMENT.into(), SECOND_ENVIRONMENT.into()];
    for (label, scope, ids, secret, replace_scope, change_category, expected) in [
        (
            "selected scope",
            "SELECTED",
            Some(selected.as_slice()),
            false,
            true,
            false,
            "move the one legacy definition to SELECTED (00000000-0000-0000-0000-000000000002, 00000000-0000-0000-0000-000000000003)",
        ),
        (
            "all scope",
            "ALL",
            None,
            true,
            true,
            false,
            "move the one legacy definition to ALL",
        ),
        (
            "plain category",
            "ALL",
            None,
            false,
            false,
            true,
            "change category to PLAIN",
        ),
        (
            "secret category",
            "ALL",
            None,
            true,
            false,
            true,
            "change category to SECRET",
        ),
        (
            "scope and category",
            "SELECTED",
            Some(selected.as_slice()),
            true,
            true,
            true,
            "move the one legacy definition to SELECTED (00000000-0000-0000-0000-000000000002, 00000000-0000-0000-0000-000000000003) and change category to SECRET",
        ),
    ] {
        assert_eq!(
            describe_safety_change("TOKEN", secret, scope, ids, replace_scope, change_category),
            format!(
                "Update TOKEN: {expected}? This replaces the existing metadata shown by the previous API remediation"
            ),
            "{label}"
        );
    }
}

#[test]
fn noninteractive_confirmation_distinguishes_required_changes_and_explicit_yes() {
    for (label, required, yes, accepted) in [
        ("ordinary change", false, false, true),
        ("ordinary change with yes", false, true, true),
        ("confirmed destructive change", true, true, true),
        ("unconfirmed destructive change", true, false, false),
    ] {
        let result = confirm_safety(required, yes, true, "Update TOKEN?");
        if accepted {
            result.unwrap_or_else(|error| panic!("{label}: {error}"));
        } else {
            assert_eq!(
                result.unwrap_err().to_string(),
                "confirmation required; rerun with --yes",
                "{label}"
            );
        }
    }
    if !std::io::stdin().is_terminal() {
        let error = confirm_safety(true, false, false, "Update TOKEN?").unwrap_err();
        assert_eq!(error.to_string(), "confirmation required; rerun with --yes");
    }
}

#[cfg(unix)]
#[test]
fn env_exec_runs_selected_snapshot_and_preserves_child_failure() {
    let materialized: crate::execution_context::MaterializedExecutionContext = serde_json::from_value(json!({
        "protocolVersion":"execution-context-v2",
        "context":{
            "workspaceId":"workspace-1","workspaceSlug":"workspace",
            "projectId":PROJECT,"projectName":"project",
            "environmentId":ENVIRONMENT,"environmentName":"Preview","environmentType":"PREVIEW",
            "sourceRef":"feature","selectionSource":"EXPLICIT"
        },
        "variables":{
            "ONREZA_TEST_SNAPSHOT_VALUE":"selected snapshot value",
            "NRZ_TOKEN":"private-token","NRZ_WORKSPACE":"private-workspace","NRZ_CUSTOM":"private-custom"
        },
        "secretKeys":[],
        "snapshot":{
            "fingerprint":format!("v1:{}", "a".repeat(64)),
            "resolvedAt":"2026-09-12T00:00:00Z","source":"DESIRED_STATE","deploymentId":null
        }
    })).unwrap();
    let literal_argument = "literal $(touch forbidden) ' \"";
    for exit_code in [0, 7] {
        let output = tempfile::tempdir().unwrap();
        let path = output.path().join("child-output.txt");
        let result = exec_with_environment(
            vec![
                "sh".into(),
                "-c".into(),
                "test -z \"${NRZ_TOKEN+x}${NRZ_WORKSPACE+x}${NRZ_CUSTOM+x}\" || exit 91; printf '%s|%s' \"$ONREZA_TEST_SNAPSHOT_VALUE\" \"$2\" > \"$1\"; exit \"$3\"".into(),
                "env-exec-test".into(),
                path.to_str().unwrap().into(),
                literal_argument.into(),
                exit_code.to_string(),
            ],
            &materialized,
        );
        if exit_code == 0 {
            result.unwrap();
        } else {
            assert!(result.unwrap_err().to_string().contains("exit status: 7"));
        }
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            format!("selected snapshot value|{literal_argument}"),
            "exit_code={exit_code}"
        );
    }
}

#[test]
fn admitted_environment_accepts_present_required_and_absent_optional_declarations() {
    let config: ProjectConfig = toml::from_str(
        "[env.declarations]\nREQUIRED = { visibility = \"sensitive\", required = true }\nOPTIONAL = { visibility = \"plain\", required = false }\n",
    ).unwrap();
    let variables = std::collections::HashMap::from([("REQUIRED".into(), "snapshot-value".into())]);
    validate_materialized_env_for_deploy(&variables, false, &config).unwrap();
}
