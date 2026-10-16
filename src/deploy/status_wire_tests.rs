use super::activation::{ActivationWait, is_activation_observation_error, wait_for_activation};
use crate::test_support::serve_api;
use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use std::time::Duration;

const DEPLOYMENT: &str = "00000000-0000-0000-0000-000000000001";

fn status() -> Value {
    json!({"id":DEPLOYMENT, "status":"failed", "url":null, "production":false,
        "error":"Build failed", "errorCode":"BUILD_FAILED", "errorDetails":null,
        "runtimeArtifactGraphDigest":null, "runtimeArtifactGraph":null,
        "createdAt":"2026-09-12T00:00:00Z", "readyAt":null})
}

#[tokio::test]
async fn generated_status_preserves_failure_authority_despite_unrecognized_diagnostics() {
    let mut failed = status();
    failed["errorDetails"] = json!({"runtimeStartupFailure":{"expectedPort":"unknown"}});
    let mut mismatched = status();
    mismatched["id"] = json!("00000000-0000-0000-0000-000000000002");
    for (body, expected, observation) in [
        (failed, "DEPLOY_FAILED", false),
        (mismatched, "DEPLOY_STATUS_INVALID", true),
        (
            json!({"id":DEPLOYMENT,"status":"live"}),
            "DEPLOY_STATUS_UNAVAILABLE",
            true,
        ),
    ] {
        let app = Router::new().route(
            "/v1/deployments/{id}/status",
            get(move || {
                let body = body.clone();
                async move { Json(body) }
            }),
        );
        let (client, server) = serve_api(app).await;
        let error = wait_for_activation(
            ActivationWait {
                deployment_id: DEPLOYMENT,
                timeout: Duration::from_secs(2),
            },
            || async { client.deployment_status(DEPLOYMENT).await?.try_into() },
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(
            crate::errors::find_cli_error(&error).unwrap().code,
            expected
        );
        assert_eq!(is_activation_observation_error(&error), observation);
        server.abort();
    }
}

#[test]
fn generated_admission_optional_nullable_contract_preserves_old_server_responses() {
    for admission in [None, Some(Value::Null)] {
        let mut wire = status();
        if let Some(admission) = admission {
            wire["admission"] = admission;
        }
        let generated: nrz_api::Status200Response = serde_json::from_value(wire).unwrap();
        let adapted: super::activation::DeploymentStatusResponse = generated.try_into().unwrap();
        assert_eq!(adapted.status, "failed");
        assert!(adapted.admission.is_none());
    }
}

#[test]
fn generated_status_adapter_accepts_server_admission_wire() {
    let admission = json!({
        "startedAt":"2026-10-15T12:00:00Z",
        "lastProgressAt":"2026-10-15T12:00:25Z",
        "waitingMembers":[{
            "assignmentId":"00000000-0000-4000-8000-000000000001",
            "phase":"WAITING_CAPACITY",
            "startedAt":"2026-10-15T12:00:05Z",
            "lastProgressAt":"2026-10-15T12:00:15Z"
        }]
    });
    let mut wire = status();
    wire["admission"] = admission.clone();
    let generated: nrz_api::Status200Response = serde_json::from_value(wire).unwrap();
    let adapted: super::activation::DeploymentStatusResponse = generated.try_into().unwrap();
    assert_eq!(adapted.status, "failed");
    assert_eq!(serde_json::to_value(adapted.admission).unwrap(), admission);
}
