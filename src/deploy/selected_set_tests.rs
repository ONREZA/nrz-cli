use super::*;
use axum::{Router, http::StatusCode, routing::get};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::test]
async fn saved_set_retry_does_not_resolve_changed_service_membership() {
    let directory = tempfile::tempdir().unwrap();
    let receipt = directory.path().join("command.json");
    let project_id = Uuid::now_v7();
    let key = Uuid::now_v7();
    let pending = PendingCommand {
        project_id,
        environment_id: Uuid::now_v7(),
        environment_selector: "production".into(),
        all: true,
        selected_services: vec![Uuid::now_v7()],
        local: BTreeMap::new(),
        request: json!({"idempotencyKey": key, "expectedGeneration": "7", "selection": {"kind": "ALL"}}),
    };
    std::fs::write(&receipt, serde_json::to_vec(&pending).unwrap()).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let app = Router::new().route(
        "/v1/projects/{id}/services",
        get(move || {
            observed.fetch_add(1, Ordering::SeqCst);
            async { StatusCode::NOT_FOUND }
        }),
    );
    let (client, server) = crate::test_support::serve_api(app).await;
    let mut args = DeploymentSetArgs {
        project_id,
        environment: "production".into(),
        all: true,
        service_ids: vec![],
        local: vec![],
        idempotency_key: Some(key),
        expected_generation: None,
        wait_timeout: 1,
    };
    let result = prepare_command(&args, &client, key, &receipt, BTreeMap::new()).await;
    server.abort();
    assert_eq!(result.unwrap().request, pending.request);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    args.environment = "another".into();
    assert!(
        prepare_command(&args, &client, key, &receipt, BTreeMap::new())
            .await
            .is_err()
    );
}

#[tokio::test(start_paused = true)]
async fn set_observation_errors_preserve_recovery_in_terminal_json() {
    use crate::deploy::activation::{ActivationWait, wait_for_activation};
    use crate::errors::find_cli_error;

    let operation_id = json!(Uuid::now_v7());
    let key = Uuid::now_v7();
    let receipt = Path::new(".onreza/deployment-sets/command.json");
    for unavailable in [false, true] {
        let error = wait_for_activation(
            ActivationWait {
                deployment_id: "dep-1",
                timeout: Duration::from_secs(1),
            },
            || async {
                if unavailable {
                    anyhow::bail!("status endpoint unavailable");
                }
                Ok(serde_json::from_value(json!({
                    "id": "dep-1",
                    "status": "building",
                    "url": "https://current.example.test",
                }))
                .unwrap())
            },
            |_| {},
        )
        .await
        .unwrap_err();
        let original = serde_json::to_value(find_cli_error(&error).unwrap().json()).unwrap();
        let error = with_set_recovery(error, &operation_id, key, receipt);
        let terminal = serde_json::to_value(find_cli_error(&error).unwrap().json()).unwrap();
        assert_eq!(terminal["code"], original["code"]);
        assert_eq!(terminal["phase"], original["phase"]);
        assert_eq!(terminal["hint"], original["hint"]);
        for (name, value) in original["details"].as_object().unwrap() {
            assert_eq!(&terminal["details"][name], value);
        }
        assert_eq!(terminal["details"]["operationId"], operation_id);
        assert_eq!(terminal["details"]["idempotencyKey"], json!(key));
        assert_eq!(terminal["details"]["receipt"], json!(receipt));
    }
}

#[test]
fn publication_errors_preserve_codes_details_and_sources_for_recovery() {
    let operation = json!(Uuid::now_v7());
    let key = Uuid::now_v7();
    let receipt = Path::new(".onreza/deployment-sets/command.json");
    let api_error = crate::api::StructuredApiError {
        status: reqwest::StatusCode::FORBIDDEN,
        code: "FORBIDDEN".into(),
        message: "snapshot unavailable".into(),
        retry_after_seconds: None,
        details: Some(json!({"permission": "deployments.read"})),
    };
    let publication_error = nrz_source_publisher::StructuredControlPlaneError {
        status: 400,
        code: "VALIDATION_ERROR".into(),
        message: "artifact rejected".into(),
        retry_after: None,
        details: Some(json!(["invalid archive"])),
    };
    for (error, code, original_details) in [
        (
            anyhow::anyhow!("checkout unavailable"),
            "DEPLOY_SET_FAILED",
            None,
        ),
        (
            output::coded_error("INVALID_CONFIG", "invalid checkout config"),
            "INVALID_CONFIG",
            None,
        ),
        (
            anyhow::Error::new(api_error),
            "FORBIDDEN",
            Some(json!({"permission": "deployments.read"})),
        ),
        (
            anyhow::Error::new(nrz_source_publisher::SourcePublicationError::from(
                publication_error,
            )),
            "VALIDATION_ERROR",
            Some(json!(["invalid archive"])),
        ),
    ] {
        let message = format!("{error:#}");
        let error = with_set_recovery(error, &operation, key, receipt);
        let diagnostic = crate::errors::find_cli_error(&error).unwrap();
        let terminal = serde_json::to_value(diagnostic.json()).unwrap();
        assert_eq!(terminal["code"], code);
        assert_eq!(terminal["details"]["operationId"], operation);
        assert_eq!(terminal["details"]["idempotencyKey"], key.to_string());
        assert_eq!(terminal["details"]["receipt"], json!(receipt));
        assert!(terminal["error"].as_str().unwrap().contains(&message));
        assert!(std::error::Error::source(diagnostic).is_some());
        if let Some(original) = original_details {
            if let Some(fields) = original.as_object() {
                for (name, value) in fields {
                    assert_eq!(&terminal["details"][name], value);
                }
            } else {
                assert_eq!(terminal["details"]["originalDetails"], original);
            }
        }
    }
}

#[test]
fn selected_local_publication_defers_limit_errors_until_recovery_is_attached() {
    for code in ["LIMIT_EXCEEDED", "SUBSCRIPTION_REQUIRED"] {
        let operation = json!(Uuid::now_v7());
        let key = Uuid::now_v7();
        let error = nrz_source_publisher::StructuredControlPlaneError {
            status: 403,
            code: code.into(),
            message: "admission limit exceeded".into(),
            retry_after: None,
            details: Some(json!({"limitType": "maxActiveDeployments", "limit": 4})),
        };
        let error = super::super::source_upload::map_publication_error(
            error.into(),
            false,
            &crate::artifact::RuntimeArtifactFileBreakdown::default(),
        );
        assert!(
            error
                .downcast_ref::<output::AlreadyReportedError>()
                .is_none()
        );
        let error = with_set_recovery(error, &operation, key, Path::new("receipt.json"));
        let diagnostic = crate::errors::find_cli_error(&error).unwrap();
        let terminal = serde_json::to_value(diagnostic.json()).unwrap();
        assert_eq!(terminal["code"], code);
        assert_eq!(terminal["details"]["limitType"], "maxActiveDeployments");
        assert_eq!(terminal["details"]["limit"], 4);
        assert_eq!(terminal["details"]["operationId"], operation);
        assert_eq!(terminal["details"]["idempotencyKey"], key.to_string());
    }
}
