use std::collections::HashMap;

use tempfile::tempdir;

use super::*;

fn context() -> ExecutionContext {
    ExecutionContext {
        workspace_id: "workspace-1".into(),
        workspace_slug: "workspace".into(),
        project_id: "project-1".into(),
        project_name: "project".into(),
        environment_id: "environment-1".into(),
        environment_name: "preview".into(),
        environment_type: "PREVIEW".into(),
        source_ref: Some("feature".into()),
        selection_source: "EXPLICIT".into(),
    }
}

#[test]
fn selection_precedence_is_explicit_then_process_then_saved() {
    let saved = SavedExecutionContext {
        version: SAVED_CONTEXT_VERSION,
        environment_id: "saved".into(),
    };

    assert_eq!(
        select_environment(Some("explicit"), Some("process"), Some(&saved)).unwrap(),
        EnvironmentSelection {
            selector: "explicit".into(),
            source: "EXPLICIT",
        }
    );
    assert_eq!(
        select_environment(None, Some("process"), Some(&saved)).unwrap(),
        EnvironmentSelection {
            selector: "process".into(),
            source: "PROCESS",
        }
    );
    assert_eq!(
        select_environment(None, None, Some(&saved)).unwrap(),
        EnvironmentSelection {
            selector: "saved".into(),
            source: "REPOSITORY",
        }
    );
}

#[test]
fn stale_saved_context_has_a_stable_error() {
    let error = map_repository_context_error(anyhow::Error::new(crate::api::StructuredApiError {
        status: reqwest::StatusCode::NOT_FOUND,
        code: "ENVIRONMENT_NOT_FOUND".into(),
        message: "not found".into(),
        retry_after_seconds: None,
        details: None,
    }));
    let coded = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
        .unwrap();
    assert_eq!(coded.code, "ENVIRONMENT_CONTEXT_STALE");
}

#[test]
fn saved_context_round_trips_inside_ignored_repository_state() {
    let directory = tempdir().unwrap();
    save(directory.path(), &context()).unwrap();

    let saved = load_saved(directory.path()).unwrap().unwrap();
    assert_eq!(saved.environment_id, "environment-1");
    let raw: serde_json::Value = serde_json::from_slice(
        &std::fs::read(directory.path().join(".onreza/environment.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(raw.as_object().unwrap().len(), 2);
    assert_eq!(raw["version"], SAVED_CONTEXT_VERSION);
    assert_eq!(raw["environmentId"], "environment-1");
    assert!(directory.path().join(".onreza/environment.json").is_file());
    assert_eq!(
        std::fs::read_to_string(directory.path().join(".gitignore")).unwrap(),
        ".onreza/\n"
    );
}

#[test]
fn materialized_environment_strips_cli_private_values_and_classifies_exact_secrets() {
    let materialized = MaterializedExecutionContext {
        protocol_version: EXECUTION_CONTEXT_PROTOCOL.into(),
        context: context(),
        variables: HashMap::from([
            ("PUBLIC_MODE".into(), "production".into()),
            ("SECRET_TOKEN".into(), "secret-value".into()),
            ("NRZ_TOKEN".into(), "runner-token".into()),
            ("NRZ_FUTURE_CONTROL".into(), "private".into()),
        ]),
        secret_keys: vec!["SECRET_TOKEN".into()],
        snapshot: MaterializedSnapshot {
            fingerprint: format!("v1:{}", "a".repeat(64)),
            resolved_at: "2026-07-14T00:00:00Z".into(),
            source: "DESIRED_STATE".into(),
            deployment_id: None,
        },
    };

    assert_eq!(secret_values(&materialized), vec!["secret-value"]);
    let environment = execution_environment(&materialized);
    assert_eq!(
        environment,
        HashMap::from([
            ("PUBLIC_MODE".into(), "production".into()),
            ("SECRET_TOKEN".into(), "secret-value".into()),
        ])
    );
}
