use serde::{Deserialize, Serialize};
use std::future::Future;
use std::time::Duration;
use tokio::time::{Instant, sleep, timeout_at};

use super::runtime_diagnostics::{
    AuthorityHead, PreparationLimit, deserialize_optional, deserialize_profile_id,
    deserialize_runtime_integer, deserialize_runtime_ports,
};
use crate::api::classify_api_retry;
use crate::errors::{CliError, find_cli_error};
use crate::output::Phase;

const POLL_INTERVAL: Duration = Duration::from_secs(3);

pub(super) fn is_activation_observation_error(error: &anyhow::Error) -> bool {
    find_cli_error(error).is_some_and(|error| {
        matches!(
            error.code.as_str(),
            "DEPLOY_WAIT_TIMEOUT" | "DEPLOY_STATUS_UNAVAILABLE" | "DEPLOY_STATUS_INVALID"
        )
    })
}

pub(super) struct ActivationWait<'a> {
    pub(super) deployment_id: &'a str,
    pub(super) timeout: Duration,
}

/// Observes an existing deployment; never creates, cancels, or fails it.
/// The deadline bounds both polling delays and in-flight status requests.
pub(super) async fn wait_for_activation<F, Fut, O>(
    request: ActivationWait<'_>,
    mut fetch_status: F,
    mut on_status: O,
) -> anyhow::Result<DeploymentStatusResponse>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<DeploymentStatusResponse>>,
    O: FnMut(&str),
{
    let deadline = Instant::now() + request.timeout;
    let mut last_status: Option<DeploymentStatusResponse> = None;
    let mut last_read_error: Option<String> = None;

    while Instant::now() < deadline {
        let mut delay = POLL_INTERVAL;
        match timeout_at(deadline, fetch_status()).await {
            Err(_) => break,
            Ok(Err(error)) => {
                let Some(retry) = classify_api_retry(&error) else {
                    return Err(status_read_error(
                        &request,
                        last_status.as_ref(),
                        "DEPLOY_STATUS_UNAVAILABLE",
                        &error.to_string(),
                    ));
                };
                last_read_error = Some(error.to_string());
                delay = retry
                    .retry_after
                    .unwrap_or(POLL_INTERVAL)
                    .max(POLL_INTERVAL);
            }
            Ok(Ok(status)) => {
                if status.id != request.deployment_id {
                    return Err(status_read_error(
                        &request,
                        last_status.as_ref(),
                        "DEPLOY_STATUS_INVALID",
                        "status response belongs to a different deployment",
                    ));
                }
                on_status(&activation_progress(&status, chrono::Utc::now()));
                let code = match status.status.as_str() {
                    "live" => return Ok(status),
                    "failed" => Some("DEPLOY_FAILED"),
                    "cancelled" => Some("DEPLOY_CANCELLED"),
                    "stopped" => Some("DEPLOY_STOPPED"),
                    "skipped" => Some("DEPLOY_SKIPPED"),
                    _ => None,
                };
                if let Some(code) = code {
                    let reason = status.error.as_deref().unwrap_or(&status.status);
                    return Err(CliError::new(
                        code,
                        format!(
                            "deployment {} {}: {}",
                            request.deployment_id,
                            status.status,
                            format_deployment_failure(reason, &status)
                        ),
                    )
                    .phase(Phase::Deploy)
                    .details(serde_json::json!({
                        "deploymentId": request.deployment_id,
                        "lastKnownStatus": status.status,
                        "url": status.url,
                        "errorCode": status.error_code,
                        "errorDetails": status.error_details,
                        "admission": status.admission,
                    }))
                    .into_anyhow());
                }
                last_status = Some(status);
                last_read_error = None;
            }
        }
        sleep(delay.min(deadline.saturating_duration_since(Instant::now()))).await;
    }

    let status = last_status.as_ref().map(|status| status.status.as_str());
    let url = last_status
        .as_ref()
        .and_then(|status| status.url.as_deref());
    Err(CliError::new(
        "DEPLOY_WAIT_TIMEOUT",
        format!(
            "stopped waiting for deployment {} after {}s; last known status: {}. URL: {}.{} \
             The CLI did not cancel the deployment; it may still complete on the server.",
            request.deployment_id,
            request.timeout.as_secs(),
            status.unwrap_or("unknown"),
            url.unwrap_or("unavailable"),
            admission_progress_suffix(last_status.as_ref()),
        ),
    )
    .phase(Phase::Deploy)
    .details(serde_json::json!({
        "deploymentId": request.deployment_id,
        "lastKnownStatus": status,
        "url": url,
        "waitTimeoutSeconds": request.timeout.as_secs(),
        "lastStatusReadError": last_read_error,
        "admission": last_status.as_ref().and_then(|status|status.admission.as_ref()),
    }))
    .hint("Check this deployment in the dashboard before retrying. Use --wait-timeout SECONDS to wait longer on future deployments.")
    .into_anyhow())
}

fn status_read_error(
    request: &ActivationWait<'_>,
    last_status: Option<&DeploymentStatusResponse>,
    code: &str,
    reason: &str,
) -> anyhow::Error {
    let url = last_status.and_then(|status| status.url.as_deref());
    CliError::new(code, format!(
        "could not read deployment {} status ({}): {}.{} The CLI did not cancel the deployment; check its status in the dashboard.",
        request.deployment_id, url.unwrap_or("unavailable"), reason, admission_progress_suffix(last_status),
    ))
    .phase(Phase::Deploy)
    .details(serde_json::json!({
        "deploymentId": request.deployment_id,
        "lastKnownStatus": last_status.map(|status| status.status.as_str()),
        "url": url,
        "lastStatusReadError": reason,
        "admission": last_status.and_then(|status|status.admission.as_ref()),
    }))
    .into_anyhow()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeploymentStatusResponse {
    pub(super) id: String,
    pub(super) status: String,
    pub(super) url: Option<String>,
    #[allow(dead_code)]
    pub(super) production: Option<bool>,
    pub(super) error: Option<String>,
    pub(super) error_code: Option<String>,
    pub(super) error_details: Option<DeploymentErrorDetails>,
    pub(super) admission: Option<nrz_api::Status200ResponseAdmission>,
    #[allow(dead_code)]
    pub(super) created_at: Option<String>,
    #[allow(dead_code)]
    pub(super) ready_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeploymentErrorDetails {
    pub(super) runtime_startup_failure: Option<RuntimeStartupFailureDetails>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional",
        skip_serializing_if = "Option::is_none"
    )]
    pub(super) retryable: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_profile_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub(super) profile_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RuntimeStartupFailureDetails {
    pub(super) code: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional",
        skip_serializing_if = "Option::is_none"
    )]
    pub(super) kind: Option<String>,
    pub(super) message: Option<String>,
    pub(super) check_type: Option<String>,
    pub(super) health_path: Option<String>,
    #[serde(default, deserialize_with = "deserialize_runtime_integer")]
    pub(super) expected_port: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_runtime_ports")]
    pub(super) detected_ports: Vec<u16>,
    #[serde(default, deserialize_with = "deserialize_runtime_integer")]
    pub(super) timeout_seconds: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_runtime_integer")]
    pub(super) attempts: Option<u32>,
    pub(super) last_error: Option<String>,
    pub(super) process_entry: Option<String>,
    pub(super) log_tail: Option<String>,
    #[serde(default, deserialize_with = "deserialize_runtime_integer")]
    pub(super) retry_after_seconds: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional",
        skip_serializing_if = "Option::is_none"
    )]
    pub(super) preparation_limit: Option<PreparationLimit>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional",
        skip_serializing_if = "Option::is_none"
    )]
    pub(super) authority_head: Option<AuthorityHead>,
}

impl TryFrom<nrz_api::Status200Response> for DeploymentStatusResponse {
    type Error = anyhow::Error;
    fn try_from(value: nrz_api::Status200Response) -> anyhow::Result<Self> {
        let error_details = value
            .error_details
            .map(serde_json::to_value)
            .transpose()?
            .and_then(|value| serde_json::from_value(value).ok());
        Ok(Self {
            id: value.id.to_string(),
            status: value.status,
            url: value.url,
            production: Some(value.production),
            error: value.error,
            error_code: value.error_code,
            error_details,
            admission: value.admission.flatten(),
            created_at: Some(
                value
                    .created_at
                    .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
            ),
            ready_at: value
                .ready_at
                .map(|instant| instant.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)),
        })
    }
}

fn admission_progress_suffix(status: Option<&DeploymentStatusResponse>) -> String {
    status
        .filter(|status| status.admission.is_some())
        .map(|status| {
            format!(
                " Progress: {}.",
                activation_progress(status, chrono::Utc::now())
            )
        })
        .unwrap_or_default()
}

pub(super) fn activation_progress(
    status: &DeploymentStatusResponse,
    now: chrono::DateTime<chrono::Utc>,
) -> String {
    use nrz_api::Status200ResponseAdmissionWaitingMemberPhase as Waiting;
    let Some(admission) = &status.admission else {
        return status.status.clone();
    };
    let age = |instant: chrono::DateTime<chrono::Utc>| {
        now.signed_duration_since(instant).num_seconds().max(0)
    };
    if admission.waiting_members.is_empty() {
        return format!(
            "{}; admission {}s, last progress {}s ago",
            status.status,
            age(admission.started_at),
            age(admission.last_progress_at)
        );
    }
    let mut phases = [0; 4];
    for member in &admission.waiting_members {
        phases[match member.phase {
            Waiting::WaitingPolicy => 0,
            Waiting::WaitingPlacement => 1,
            Waiting::WaitingCapacity => 2,
            Waiting::WaitingRecipient => 3,
        }] += 1;
    }
    let waiting = ["policy", "placement", "capacity", "recipient"]
        .into_iter()
        .zip(phases)
        .filter(|(_, count)| *count > 0)
        .map(|(phase, count)| format!("{phase} ({count})"))
        .collect::<Vec<_>>();
    let phase_age = admission
        .waiting_members
        .iter()
        .map(|member| age(member.started_at))
        .max()
        .unwrap_or_else(|| age(admission.started_at));
    let progress_age = admission
        .waiting_members
        .iter()
        .map(|member| age(member.last_progress_at))
        .max()
        .unwrap_or_else(|| age(admission.last_progress_at));
    format!(
        "{}; waiting for {}; admission {}s, oldest phase {}s, oldest progress {}s ago",
        status.status,
        waiting.join(", "),
        age(admission.started_at),
        phase_age,
        progress_age
    )
}

pub(super) fn format_deployment_failure(error: &str, status: &DeploymentStatusResponse) -> String {
    let Some(details) = status
        .error_details
        .as_ref()
        .and_then(|details| details.runtime_startup_failure.as_ref())
    else {
        return format!("{error}{}", admission_progress_suffix(Some(status)));
    };

    let mut lines = Vec::new();
    lines.push(
        details
            .message
            .as_deref()
            .filter(|message| !message.trim().is_empty())
            .unwrap_or(error)
            .to_string(),
    );

    let mut facts = Vec::new();
    if let Some(code) = details.code.as_deref() {
        facts.push(format!("reason: {code}"));
    }
    if let Some(check_type) = details.check_type.as_deref() {
        let check = match (check_type, details.health_path.as_deref()) {
            ("http", Some(path)) => format!("HTTP {path}"),
            ("http", None) => "HTTP".to_string(),
            ("tcp", _) => "TCP".to_string(),
            (other, _) => other.to_string(),
        };
        facts.push(format!("check: {check}"));
    }
    if let Some(port) = details.expected_port {
        facts.push(format!("expected port: {port}"));
    }
    if !details.detected_ports.is_empty() {
        facts.push(format!(
            "detected ports: {}",
            details
                .detected_ports
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(timeout) = details.timeout_seconds {
        facts.push(format!("timeout: {timeout}s"));
    }
    if let Some(attempts) = details.attempts {
        facts.push(format!("attempts: {attempts}"));
    }
    if let Some(entry) = details.process_entry.as_deref() {
        facts.push(format!("entry: {entry}"));
    }
    if let Some(last_error) = details.last_error.as_deref() {
        facts.push(format!("last readiness error: {last_error}"));
    }
    if let Some(limit) = &details.preparation_limit {
        facts.push(limit.to_string());
    }
    let retryable = status
        .error_details
        .as_ref()
        .and_then(|details| details.retryable);
    if let Some(retryable) = retryable {
        facts.push(format!("retryable: {retryable}"));
    }
    if let Some(retry_after) = details
        .retry_after_seconds
        .filter(|_| retryable != Some(false))
    {
        facts.push(format!("retry after: {retry_after}s"));
    }

    if !facts.is_empty() {
        lines.push(format!("Runtime diagnostics: {}", facts.join("; ")));
    }

    if let Some(log_tail) = details
        .log_tail
        .as_deref()
        .filter(|tail| !tail.trim().is_empty())
    {
        lines.push(format!("Recent runtime output:\n{}", log_tail.trim()));
    }

    format!(
        "{}{}",
        lines.join("\n"),
        admission_progress_suffix(Some(status))
    )
}
