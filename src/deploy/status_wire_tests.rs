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

async fn terminal_wire_failure(wire: Value) -> anyhow::Error {
    let generated: nrz_api::Status200Response = serde_json::from_value(wire).unwrap();
    let mut adapted = Some(generated.try_into().unwrap());
    wait_for_activation(
        ActivationWait {
            deployment_id: DEPLOYMENT,
            timeout: Duration::from_secs(2),
        },
        || {
            std::future::ready(Ok(adapted
                .take()
                .expect("terminal failure must not poll again")))
        },
        |_| {},
    )
    .await
    .unwrap_err()
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

#[tokio::test]
async fn preparation_limit_wire_preserves_exact_facts_in_human_and_json_failure() {
    let required = u128::MAX.to_string();
    let details = json!({
        "retryable":false,"profileId":"edge-standard-v1",
        "runtimeStartupFailure":{
            "code":"platform_fault","kind":"platform","message":"Preparation limit exceeded",
            "retryAfterSeconds":60,
            "preparationLimit":{"metric":"SOURCE_ARCHIVE_BYTES","required":required,"limit":"1048576"}
        }
    });
    let mut wire = status();
    wire["errorCode"] = json!("RUNTIME_PREPARATION_LIMIT");
    wire["errorDetails"] = details.clone();
    let error = terminal_wire_failure(wire).await;
    let cli = crate::errors::find_cli_error(&error).unwrap();
    let human = error.to_string();
    assert!(human.contains("SOURCE_ARCHIVE_BYTES"), "{human}");
    assert!(human.contains(&required), "{human}");
    assert!(human.contains("1048576"), "{human}");
    assert!(human.contains("retryable: false"), "{human}");
    assert!(!human.contains("retry after:"), "{human}");
    let output = serde_json::to_value(cli.json()).unwrap();
    assert_eq!(output["details"]["errorCode"], "RUNTIME_PREPARATION_LIMIT");
    assert_eq!(output["details"]["errorDetails"]["retryable"], false);
    assert_eq!(
        output["details"]["errorDetails"]["profileId"],
        "edge-standard-v1"
    );
    assert_eq!(
        output["details"]["errorDetails"]["runtimeStartupFailure"]["kind"],
        "platform"
    );
    assert_eq!(
        output["details"]["errorDetails"]["runtimeStartupFailure"]["preparationLimit"],
        details["runtimeStartupFailure"]["preparationLimit"]
    );
    assert_eq!(
        output["details"]["errorDetails"]["runtimeStartupFailure"]["retryAfterSeconds"],
        60
    );
}

#[test]
fn generated_runtime_numeric_diagnostics_preserve_only_exact_unsigned_values() {
    for (number, valid_u64) in [
        (json!(60), true),
        (json!(0), true),
        (json!(9_007_199_254_740_991_u64), true),
        (json!(1.5), false),
        (json!(-1), false),
        (json!(9_007_199_254_740_992_u64), false),
        (json!(u64::MAX), false),
        (Value::from(f64::NAN), false),
        (json!("60"), false),
    ] {
        let mut wire = status();
        wire["errorDetails"] = json!({"runtimeStartupFailure":{
            "code":"platform_fault","message":"Base failure",
            "expectedPort":number,"detectedPorts":[3000,65536,1.5,-1,null],
            "timeoutSeconds":number,"attempts":number,"retryAfterSeconds":number
        }});
        let generated: nrz_api::Status200Response = serde_json::from_value(wire).unwrap();
        let adapted: super::activation::DeploymentStatusResponse = generated.try_into().unwrap();
        let human = super::activation::format_deployment_failure("Build failed", &adapted);
        assert!(human.contains("Base failure"), "{human}");
        assert!(human.contains("reason: platform_fault"), "{human}");
        let runtime = adapted
            .error_details
            .unwrap()
            .runtime_startup_failure
            .unwrap();
        assert_eq!(runtime.timeout_seconds.is_some(), valid_u64, "{number}");
        assert_eq!(runtime.retry_after_seconds.is_some(), valid_u64, "{number}");
        assert_eq!(
            runtime.expected_port.is_some(),
            valid_u64
                && number
                    .as_u64()
                    .is_some_and(|value| value <= u16::MAX.into()),
            "{number}"
        );
        assert_eq!(
            runtime.attempts.is_some(),
            valid_u64
                && number
                    .as_u64()
                    .is_some_and(|value| value <= u32::MAX.into()),
            "{number}"
        );
        assert_eq!(runtime.detected_ports, vec![3000]);
    }
}

#[test]
fn malformed_optional_preparation_limit_keeps_base_runtime_failure() {
    for required in [
        json!("01"),
        json!("-1"),
        json!("1.0"),
        json!("340282366920938463463374607431768211456"),
        json!(1),
    ] {
        let mut wire = status();
        wire["errorDetails"] = json!({"retryable":false,"runtimeStartupFailure":{
            "code":"platform_fault","message":"Preparation limit exceeded",
            "preparationLimit":{"metric":"SOURCE_ARCHIVE_BYTES","required":required,"limit":"1024"}
        }});
        let generated: nrz_api::Status200Response = serde_json::from_value(wire).unwrap();
        let adapted = generated.try_into().unwrap();
        let human = super::activation::format_deployment_failure("Build failed", &adapted);
        assert!(human.contains("Preparation limit exceeded"), "{human}");
        assert!(human.contains("reason: platform_fault"), "{human}");
        assert!(!human.contains("SOURCE_ARCHIVE_BYTES"), "{human}");
    }
}

#[tokio::test]
async fn authority_revocation_wire_preserves_exact_proof_and_ignores_invalid_optional_head() {
    let head = json!({"kind":"ENVIRONMENT_POLICY","id":DEPLOYMENT,
        "generation":"18446744073709551615","headSha256":"ab".repeat(32)});
    let mut cases = vec![(head.clone(), true)];
    for (field, value) in [
        ("generation", json!("18446744073709551616")),
        ("generation", json!("01")),
        ("generation", json!("0")),
        ("generation", json!(9007199254740993_u64)),
        ("id", json!("ABCDEFAB-0000-0000-0000-000000000001")),
        ("kind", json!("UNKNOWN")),
        ("headSha256", json!("AB".repeat(32))),
    ] {
        let mut invalid = head.clone();
        invalid[field] = value;
        cases.push((invalid, false));
    }
    for (proof, valid) in cases {
        let mut wire = status();
        wire["errorCode"] = json!("EDGE_ROLLOUT_REVOKED");
        wire["errorDetails"] = json!({"runtimeStartupFailure":{
            "code":"admission_rejected","kind":"admission_rejected","message":"Runtime authority was revoked","authorityHead":proof
        }});
        let error = terminal_wire_failure(wire).await;
        assert!(error.to_string().contains("Runtime authority was revoked"));
        assert!(error.to_string().contains("reason: admission_rejected"));
        let output =
            serde_json::to_value(crate::errors::find_cli_error(&error).unwrap().json()).unwrap();
        let actual = &output["details"]["errorDetails"]["runtimeStartupFailure"]["authorityHead"];
        assert_eq!(output["details"]["errorCode"], "EDGE_ROLLOUT_REVOKED");
        if valid {
            assert_eq!(*actual, head);
        } else {
            assert!(actual.is_null(), "{actual}");
        }
    }
}

#[test]
fn preparation_limit_diagnostics_validate_wire_facts_without_client_policy() {
    use super::runtime_diagnostics::PreparationLimit;
    let valid = json!({"metric":"FUTURE_METRIC_2","required":"1","limit":"0"});
    assert!(serde_json::from_value::<PreparationLimit>(valid.clone()).is_ok());
    for (field, value) in [
        ("metric", json!("a")),
        ("metric", json!("A\n")),
        ("metric", json!("A".repeat(65))),
        ("required", json!("")),
        ("required", json!("+1")),
        ("required", json!("1e2")),
        ("required", json!("0")),
        ("limit", json!("1")),
        ("limit", json!("2")),
        ("limit", json!("00")),
        ("limit", json!(false)),
    ] {
        let mut invalid = valid.clone();
        invalid[field] = value;
        assert!(
            serde_json::from_value::<PreparationLimit>(invalid.clone()).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn malformed_optional_profile_and_retryability_keep_base_failure() {
    for profile in [
        json!(""),
        json!("-profile"),
        json!("a".repeat(65)),
        json!("a\n"),
        json!(3),
    ] {
        let mut wire = status();
        wire["errorDetails"] = json!({"profileId":profile,"retryable":"false","runtimeStartupFailure":{
            "code":"platform_fault","message":"Base failure"
        }});
        let generated: nrz_api::Status200Response = serde_json::from_value(wire).unwrap();
        let adapted: super::activation::DeploymentStatusResponse = generated.try_into().unwrap();
        let human = super::activation::format_deployment_failure("Build failed", &adapted);
        assert!(human.contains("Base failure"), "{human}");
        assert!(human.contains("reason: platform_fault"), "{human}");
        let details = adapted.error_details.unwrap();
        assert!(details.profile_id.is_none());
        assert!(details.retryable.is_none());
    }
}
