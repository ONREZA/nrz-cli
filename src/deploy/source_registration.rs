use super::*;

pub(super) async fn register_deployment_source(
    client: &ApiClient,
    deployment_id: &str,
    attempt: u32,
    manifest: serde_json::Value,
    functions: Option<serde_json::Value>,
    json: bool,
) -> anyhow::Result<()> {
    let namespace = Uuid::parse_str(deployment_id).context("deployment ID is not a valid UUID")?;
    let operation_id = Uuid::new_v5(
        &namespace,
        format!("onreza:deployment-source:{attempt}").as_bytes(),
    );
    let body = source_request_body(attempt, operation_id, manifest, functions)?;
    let started = Instant::now();
    let mut delay = SOURCE_REGISTRATION_INITIAL_RETRY_DELAY;

    loop {
        let remaining = SOURCE_REGISTRATION_RETRY_BUDGET.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            bail!(
                "failed to register admitted deployment source after waiting {:?}",
                SOURCE_REGISTRATION_RETRY_BUDGET
            );
        }
        let response = tokio::time::timeout(
            SOURCE_REGISTRATION_REQUEST_TIMEOUT.min(remaining),
            client.register_source(deployment_id, body.clone()),
        )
        .await;
        match response {
            Ok(Ok(response)) => {
                if response.id != namespace || response.status != "UPLOADING" {
                    bail!("deployment source registration returned an unexpected state");
                }
                return Ok(());
            }
            Ok(Err(error)) => {
                let Some(retry) = classify_api_retry(&error) else {
                    return Err(map_source_registration_error(
                        error,
                        json,
                        "failed to register admitted deployment source",
                    ));
                };
                let remaining = SOURCE_REGISTRATION_RETRY_BUDGET.saturating_sub(started.elapsed());
                if remaining.is_zero() {
                    return Err(map_source_registration_error(
                        error,
                        json,
                        &format!(
                            "failed to register admitted deployment source after waiting {:?}",
                            SOURCE_REGISTRATION_RETRY_BUDGET
                        ),
                    ));
                }
                tokio::time::sleep(retry.retry_after.unwrap_or(delay).min(remaining)).await;
            }
            Err(_) => {
                let remaining = SOURCE_REGISTRATION_RETRY_BUDGET.saturating_sub(started.elapsed());
                if remaining.is_zero() {
                    bail!(
                        "failed to register admitted deployment source after waiting {:?}",
                        SOURCE_REGISTRATION_RETRY_BUDGET
                    );
                }
                tokio::time::sleep(delay.min(remaining)).await;
            }
        }
        delay = (delay * 2).min(SOURCE_REGISTRATION_MAX_RETRY_DELAY);
    }
}

pub(super) fn source_request_body(
    attempt: u32,
    operation_id: Uuid,
    manifest: serde_json::Value,
    functions: Option<serde_json::Value>,
) -> anyhow::Result<nrz_api::SourceRequestBody> {
    Ok(nrz_api::SourceRequestBody {
        protocol_version: crate::execution_context::EXECUTION_CONTEXT_PROTOCOL.to_string(),
        attempt: i64::from(attempt),
        operation_id,
        manifest: serde_json::from_value(manifest)
            .context("manifest does not match the server contract")?,
        functions: functions
            .map(serde_json::from_value)
            .transpose()
            .context("functions do not match the server contract")?,
    })
}
