use super::*;

pub(super) async fn mark_pre_source_skipped(
    client: &ApiClient,
    deployment_id: &str,
    attempt: u32,
    reason: &str,
) -> anyhow::Result<()> {
    let body = nrz_api::SkipBeforeSourceRequestBody {
        protocol_version: crate::execution_context::EXECUTION_CONTEXT_PROTOCOL.to_string(),
        attempt: i64::from(attempt),
        reason: reason.to_string(),
    };
    let started = Instant::now();
    let mut delay = PRE_SOURCE_FAILURE_INITIAL_RETRY_DELAY;
    loop {
        let remaining = PRE_SOURCE_FAILURE_RETRY_BUDGET.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(
            PRE_SOURCE_FAILURE_REQUEST_TIMEOUT.min(remaining),
            client.skip_before_source(deployment_id, body.clone()),
        )
        .await
        {
            Ok(Ok(response))
                if response.accepted
                    && Some(response.id) == Uuid::parse_str(deployment_id).ok() =>
            {
                return Ok(());
            }
            Ok(Ok(_)) => {
                return Err(output::coded_error(
                    "IGNORED_BUILD_SKIP_REJECTED",
                    "deployment state changed before Ignored Build Step could mark it skipped",
                ));
            }
            Ok(Err(error)) => {
                let Some(retry) = classify_api_retry(&error) else {
                    return Err(error.context("failed to mark deployment skipped"));
                };
                let remaining = PRE_SOURCE_FAILURE_RETRY_BUDGET.saturating_sub(started.elapsed());
                if remaining.is_zero() {
                    break;
                }
                tokio::time::sleep(retry.retry_after.unwrap_or(delay).min(remaining)).await;
                delay = (delay * 2).min(PRE_SOURCE_FAILURE_MAX_RETRY_DELAY);
            }
            Err(_) => {
                let remaining = PRE_SOURCE_FAILURE_RETRY_BUDGET.saturating_sub(started.elapsed());
                if remaining.is_zero() {
                    break;
                }
                tokio::time::sleep(delay.min(remaining)).await;
                delay = (delay * 2).min(PRE_SOURCE_FAILURE_MAX_RETRY_DELAY);
            }
        }
    }
    Err(output::coded_error(
        "IGNORED_BUILD_SKIP_REPORT_FAILED",
        "timed out while marking deployment skipped",
    ))
}

pub(super) async fn report_pre_source_failure(
    client: Option<&ApiClient>,
    deployment_id: &str,
    attempt: u32,
    error_code: PreSourceFailureCode,
    error: Option<&anyhow::Error>,
    redactor: Option<&ExactValueRedactor>,
    json: bool,
) {
    let Some(client) = client else {
        return;
    };
    let body = match pre_source_failure_body(
        attempt,
        error_code,
        error.and_then(|error| pre_source_failure_diagnostic(error, redactor)),
    ) {
        Ok(body) => body,
        Err(error) => {
            output::warn(
                json,
                format!("Could not prepare deployment failure report: {error}"),
                output::Phase::Deploy,
            );
            return;
        }
    };
    let started = Instant::now();
    let mut delay = PRE_SOURCE_FAILURE_INITIAL_RETRY_DELAY;
    let mut last_error = None;
    loop {
        let remaining = PRE_SOURCE_FAILURE_RETRY_BUDGET.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(
            PRE_SOURCE_FAILURE_REQUEST_TIMEOUT.min(remaining),
            client.fail_before_source(deployment_id, body.clone()),
        )
        .await
        {
            Ok(Ok(response)) => {
                if Some(response.id) != Uuid::parse_str(deployment_id).ok() || !response.accepted {
                    output::warn(
                        json,
                        "Deployment state changed before failure could be recorded",
                        output::Phase::Deploy,
                    );
                }
                return;
            }
            Ok(Err(error)) => {
                let Some(retry) = classify_api_retry(&error) else {
                    output::warn(
                        json,
                        format!("Could not mark admitted deployment failed: {error}"),
                        output::Phase::Deploy,
                    );
                    return;
                };
                last_error = Some(error.to_string());
                let remaining = PRE_SOURCE_FAILURE_RETRY_BUDGET.saturating_sub(started.elapsed());
                if remaining.is_zero() {
                    break;
                }
                tokio::time::sleep(retry.retry_after.unwrap_or(delay).min(remaining)).await;
            }
            Err(_) => {
                last_error = Some("request timed out".to_string());
                let remaining = PRE_SOURCE_FAILURE_RETRY_BUDGET.saturating_sub(started.elapsed());
                if remaining.is_zero() {
                    break;
                }
                tokio::time::sleep(delay.min(remaining)).await;
            }
        }
        delay = (delay * 2).min(PRE_SOURCE_FAILURE_MAX_RETRY_DELAY);
    }
    output::warn(
        json,
        format!(
            "Could not mark admitted deployment failed after retries: {}",
            last_error.as_deref().unwrap_or("retry budget exhausted")
        ),
        output::Phase::Deploy,
    );
}

pub(super) fn pre_source_failure_diagnostic(
    error: &anyhow::Error,
    redactor: Option<&ExactValueRedactor>,
) -> Option<PreSourceFailureDiagnostic> {
    if let Some(diagnostic) = output::reported_terminal_diagnostic(error) {
        return Some(PreSourceFailureDiagnostic {
            code: diagnostic.code.clone(),
            message: sanitize_pre_source_failure_message(&diagnostic.message, redactor),
            details: sanitize_pre_source_failure_details(diagnostic.details.as_ref(), redactor),
        });
    }
    if let Some(error) = crate::errors::find_cli_error(error) {
        return Some(PreSourceFailureDiagnostic {
            code: error.code.clone(),
            message: sanitize_pre_source_failure_message(&error.to_string(), redactor),
            details: sanitize_pre_source_failure_details(error.details.as_ref(), redactor),
        });
    }
    if let Some(error) = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<output::CodedError>())
    {
        return Some(PreSourceFailureDiagnostic {
            code: error.code.clone(),
            message: sanitize_pre_source_failure_message(&error.message, redactor),
            details: None,
        });
    }
    if let Some(error) = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::api::StructuredApiError>())
    {
        return Some(PreSourceFailureDiagnostic {
            code: error.code.clone(),
            message: sanitize_pre_source_failure_message(&error.message, redactor),
            details: sanitize_pre_source_failure_details(error.details.as_ref(), redactor),
        });
    }
    Some(PreSourceFailureDiagnostic {
        code: "INTERNAL_ERROR".to_string(),
        message: sanitize_pre_source_failure_message(&format!("{error:#}"), redactor),
        details: None,
    })
}

fn sanitize_pre_source_failure_details(
    details: Option<&serde_json::Value>,
    redactor: Option<&ExactValueRedactor>,
) -> Option<serde_json::Value> {
    details.map(|details| {
        redactor.map_or_else(
            || details.clone(),
            |redactor| redactor.sanitize_json(details),
        )
    })
}

fn sanitize_pre_source_failure_message(
    message: &str,
    redactor: Option<&ExactValueRedactor>,
) -> String {
    let fallback_redactor = ExactValueRedactor::from_values(std::iter::empty())
        .expect("empty build-log redactor must compile");
    truncate_utf8(
        sanitize_message(message, redactor.unwrap_or(&fallback_redactor)),
        MAX_PRE_SOURCE_FAILURE_LOG_LENGTH,
    )
}

pub(super) fn pre_source_failure_body(
    attempt: u32,
    error_code: PreSourceFailureCode,
    diagnostic: Option<PreSourceFailureDiagnostic>,
) -> anyhow::Result<nrz_api::FailBeforeSourceRequestBody> {
    Ok(nrz_api::FailBeforeSourceRequestBody {
        protocol_version: crate::execution_context::EXECUTION_CONTEXT_PROTOCOL.to_string(),
        attempt: i64::from(attempt),
        error_code,
        diagnostic: diagnostic
            .map(|value| -> anyhow::Result<_> {
                Ok(nrz_api::FailBeforeSourceRequestBodyDiagnostic {
                    code: value.code,
                    message: value.message,
                    details: value
                        .details
                        .map(|details| {
                            let details = if details.is_object() {
                                details
                            } else {
                                serde_json::json!({"value":details})
                            };
                            serde_json::from_value(details)
                                .context("invalid structured failure details")
                        })
                        .transpose()?,
                })
            })
            .transpose()?,
    })
}
