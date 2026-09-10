use std::future::{pending, ready};
use std::time::Duration;

use clap::Parser;
use tokio::time::Instant;

use super::activation::{
    ActivationWait, DeploymentStatusResponse, is_activation_observation_error, wait_for_activation,
};
use crate::cli::{Cli, Command};
use crate::errors::find_cli_error;

fn request(seconds: u64) -> ActivationWait<'static> {
    ActivationWait {
        deployment_id: "dep-1",
        url: "https://initial.example.test",
        timeout: Duration::from_secs(seconds),
    }
}

fn status(value: &str) -> DeploymentStatusResponse {
    serde_json::from_value(serde_json::json!({
        "id": "dep-1",
        "status": value,
        "url": "https://current.example.test",
    }))
    .unwrap()
}

#[test]
fn cli_wait_defaults_to_120_seconds_and_accepts_an_override() {
    for (arguments, expected) in [
        (vec!["nrz", "deploy"], 120),
        (vec!["nrz", "deploy", "--wait-timeout", "600"], 600),
    ] {
        let cli = Cli::try_parse_from(arguments).unwrap();
        let Command::Deploy(args) = cli.command else {
            panic!("expected deploy command");
        };
        assert_eq!(args.wait_timeout, expected);
    }
}

#[test]
fn cli_rejects_invalid_wait_values_and_modes_without_activation_wait() {
    for value in ["0", "-1", "never", "4294967296"] {
        assert!(Cli::try_parse_from(["nrz", "deploy", "--wait-timeout", value]).is_err());
    }
    assert!(Cli::try_parse_from(["nrz", "deploy", "--dry"]).is_ok());
    assert!(Cli::try_parse_from(["nrz", "deploy", "--dry", "--wait-timeout", "600"]).is_err());
    assert!(
        Cli::try_parse_from([
            "nrz",
            "deploy",
            "--resume-deployment",
            "dep-1",
            "--wait-timeout",
            "600"
        ])
        .is_err()
    );
}

#[tokio::test(start_paused = true)]
async fn extended_wait_observes_activation_after_the_old_deadline() {
    let started = Instant::now();
    let result = wait_for_activation(
        request(300),
        || {
            ready(Ok(status(
                if started.elapsed() >= Duration::from_secs(180) {
                    "live"
                } else {
                    "smoke_testing"
                },
            )))
        },
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(result.status, "live");
    assert_eq!(started.elapsed(), Duration::from_secs(180));
}

#[tokio::test(start_paused = true)]
async fn timeout_preserves_latest_status_and_url_in_human_and_json_errors() {
    let started = Instant::now();
    let mut observed = Vec::new();
    let error = wait_for_activation(
        request(120),
        || ready(Ok(status("ingesting"))),
        |value| observed.push(value.to_owned()),
    )
    .await
    .unwrap_err();
    assert_eq!(started.elapsed(), Duration::from_secs(120));
    assert!(!observed.is_empty());
    let cli = find_cli_error(&error).unwrap();
    let json = serde_json::to_value(cli.json()).unwrap();
    assert_eq!(json["code"], "DEPLOY_WAIT_TIMEOUT");
    assert!(is_activation_observation_error(&error));
    assert_eq!(json["details"]["deploymentId"], "dep-1");
    assert_eq!(json["details"]["lastKnownStatus"], "ingesting");
    assert_eq!(json["details"]["url"], "https://current.example.test");
    assert_eq!(json["details"]["waitTimeoutSeconds"], 120);
    assert!(json["details"]["lastStatusReadError"].is_null());
    let message = error.to_string();
    assert!(message.contains("dep-1"));
    assert!(message.contains("ingesting"));
    assert!(message.contains("https://current.example.test"));
    assert!(message.contains("did not cancel"));
    assert!(message.contains("--wait-timeout"));
}

#[tokio::test(start_paused = true)]
async fn deadline_also_bounds_a_hung_status_request() {
    let started = Instant::now();
    let error = wait_for_activation(request(5), pending, |_| {})
        .await
        .unwrap_err();
    assert_eq!(started.elapsed(), Duration::from_secs(5));
    let cli = find_cli_error(&error).unwrap();
    let json = serde_json::to_value(cli.json()).unwrap();
    assert_eq!(json["code"], "DEPLOY_WAIT_TIMEOUT");
    assert!(json["details"]["lastKnownStatus"].is_null());
    assert_eq!(json["details"]["url"], "https://initial.example.test");
}

#[tokio::test(start_paused = true)]
async fn already_live_and_terminal_failures_finish_without_polling_again() {
    let started = Instant::now();
    for (state, code) in [
        ("live", None),
        ("failed", Some("DEPLOY_FAILED")),
        ("cancelled", Some("DEPLOY_CANCELLED")),
        ("stopped", Some("DEPLOY_STOPPED")),
        ("skipped", Some("DEPLOY_SKIPPED")),
    ] {
        let mut calls = 0;
        let result = wait_for_activation(
            request(120),
            || {
                calls += 1;
                ready(Ok(status(state)))
            },
            |_| {},
        )
        .await;
        assert_eq!(calls, 1);
        match code {
            None => assert_eq!(result.unwrap().status, "live"),
            Some(code) => {
                let error = result.unwrap_err();
                assert!(!is_activation_observation_error(&error));
                assert_eq!(find_cli_error(&error).unwrap().code, code);
            }
        }
    }
    assert_eq!(started.elapsed(), Duration::ZERO);
}

fn unavailable(retry_after: u64) -> anyhow::Error {
    crate::api::StructuredApiError {
        status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
        code: "SERVICE_UNAVAILABLE".to_owned(),
        message: "temporarily unavailable".to_owned(),
        retry_after_seconds: Some(retry_after),
        details: None,
    }
    .into()
}

#[tokio::test(start_paused = true)]
async fn transient_status_failure_can_recover_within_the_same_wait() {
    let started = Instant::now();
    let mut calls = 0;
    let result = wait_for_activation(
        request(120),
        || {
            calls += 1;
            ready(if calls == 1 {
                Err(unavailable(7))
            } else {
                Ok(status("live"))
            })
        },
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(result.status, "live");
    assert_eq!(calls, 2);
    assert_eq!(started.elapsed(), Duration::from_secs(7));
}

#[tokio::test(start_paused = true)]
async fn retry_after_cannot_extend_the_user_deadline() {
    let started = Instant::now();
    let error = wait_for_activation(request(5), || ready(Err(unavailable(600))), |_| {})
        .await
        .unwrap_err();
    assert_eq!(started.elapsed(), Duration::from_secs(5));
    let json = serde_json::to_value(find_cli_error(&error).unwrap().json()).unwrap();
    assert!(
        json["details"]["lastStatusReadError"]
            .as_str()
            .unwrap()
            .contains("temporarily unavailable")
    );
}

#[tokio::test(start_paused = true)]
async fn non_retryable_status_error_is_reported_without_waiting() {
    let started = Instant::now();
    let error = wait_for_activation(
        request(120),
        || ready(Err(anyhow::anyhow!("permission denied"))),
        |_| {},
    )
    .await
    .unwrap_err();
    assert_eq!(started.elapsed(), Duration::ZERO);
    assert!(format!("{error:#}").contains("permission denied"));
    assert!(format!("{error:#}").contains("dep-1"));
    assert!(is_activation_observation_error(&error));
    let json = serde_json::to_value(find_cli_error(&error).unwrap().json()).unwrap();
    assert_eq!(json["code"], "DEPLOY_STATUS_UNAVAILABLE");
    assert_eq!(json["details"]["deploymentId"], "dep-1");
    assert_eq!(json["details"]["url"], "https://initial.example.test");
}

#[tokio::test(start_paused = true)]
async fn mismatched_response_cannot_report_success_or_replace_last_known_status() {
    let mut calls = 0;
    let error = wait_for_activation(
        request(120),
        || {
            calls += 1;
            let mut response = status(if calls == 1 { "ingesting" } else { "live" });
            if calls > 1 {
                response.id = "other-deployment".to_owned();
            }
            ready(Ok(response))
        },
        |_| {},
    )
    .await
    .unwrap_err();
    assert_eq!(calls, 2);
    assert!(is_activation_observation_error(&error));
    let json = serde_json::to_value(find_cli_error(&error).unwrap().json()).unwrap();
    assert_eq!(json["code"], "DEPLOY_STATUS_INVALID");
    assert_eq!(json["details"]["deploymentId"], "dep-1");
    assert_eq!(json["details"]["lastKnownStatus"], "ingesting");
    assert_eq!(json["details"]["url"], "https://current.example.test");
}
