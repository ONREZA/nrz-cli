use std::time::Duration;

use anyhow::Context;
use reqwest::header::{COOKIE, HeaderValue, LOCATION, RETRY_AFTER, SET_COOKIE};
use serde::Serialize;

use crate::api::ApiClient;
use crate::errors::CliError;
use crate::output;

const VERIFY_TIMEOUT: Duration = Duration::from_secs(20);
const PREVIEW_ACCESS_READY_TIMEOUT: Duration = Duration::from_secs(60);

pub(super) struct DeployVerificationRequest<'a> {
    pub(super) api_client: &'a ApiClient,
    pub(super) deployment_id: &'a str,
    pub(super) project_id: &'a str,
    pub(super) url: &'a str,
    pub(super) health_check: Option<&'a super::ResolvedHealthCheck>,
    pub(super) json: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeployVerificationOutput {
    pub(super) status: &'static str,
    pub(super) url: String,
    pub(super) path: String,
    pub(super) status_code: u16,
    pub(super) used_preview_bypass: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) preview_access_revoked: Option<bool>,
}

const PREVIEW_COOKIE_NAME: &str = "__Host-onreza-preview-capability";

pub(super) struct PreviewAccessResponse {
    pub(super) status_code: u16,
    pub(super) cookie: Option<HeaderValue>,
    pub(super) retry_after: Option<Duration>,
}

pub(super) struct VerificationResponse {
    pub(super) status_code: u16,
    pub(super) location: Option<String>,
}

pub(super) async fn verify_deployment(
    request: DeployVerificationRequest<'_>,
) -> anyhow::Result<DeployVerificationOutput> {
    let path = verification_path(request.health_check);
    let base_url = request.url.to_string();
    let url = verification_url(&base_url, &path)?;

    output::status(
        request.json,
        "~",
        format!("Verifying deployment URL: {url}"),
        output::Phase::Deploy,
    );

    let initial_response = fetch_verification_url(&url, None).await.map_err(|error| {
        verify_error(
            format!("failed to verify deployment URL: {error:#}"),
            &url,
            &path,
            None,
            None,
            false,
        )
    })?;

    let (response, access_secret_id, used_preview_bypass) =
        if needs_preview_bypass(&initial_response)
            && super::access::read(request.api_client, request.deployment_id, &base_url)
                .await
                .context("failed to resolve URL protection for deploy verification")?
        {
            let access = crate::preview::create_preview_access(
                request.api_client,
                request.project_id,
                "nrz deploy --verify".to_string(),
                Some(url.clone()),
                crate::preview::AGENT_PREVIEW_ACCESS_TTL_SECONDS,
            )
            .await
            .context("failed to create temporary preview access for deploy verification")?;
            (
                verify_with_preview_access(&url, &access.header_value).await,
                Some(access.secret_id),
                true,
            )
        } else {
            (Ok(initial_response), None, false)
        };

    let revoke_result = if let Some(secret_id) = access_secret_id.as_deref() {
        let result = crate::preview::revoke_preview_access(
            request.api_client,
            request.project_id,
            secret_id,
        )
        .await;
        if let Err(error) = &result {
            output::warn(
                request.json,
                format!("failed to revoke temporary preview access {secret_id}: {error:#}"),
                output::Phase::Deploy,
            );
        }
        Some((secret_id, result))
    } else {
        None
    };

    let response = response.map_err(|error| {
        verify_error(
            format!("failed to verify deployment URL: {error:#}"),
            &url,
            &path,
            None,
            None,
            used_preview_bypass,
        )
    })?;

    validate_response(&url, &path, &response, used_preview_bypass)?;

    if let Some((secret_id, Err(error))) = revoke_result {
        return Err(CliError::new(
            "PREVIEW_ACCESS_REVOKE_FAILED",
            "deployment verification passed, but temporary preview access could not be revoked",
        )
        .phase(output::Phase::Deploy)
        .details(serde_json::json!({
            "projectId": request.project_id,
            "secretId": secret_id,
            "url": url,
        }))
        .hint(format!(
            "Revoke it manually with `nrz preview revoke --project-id {} --secret-id {secret_id}`.\n\n{error:#}",
            request.project_id
        ))
        .into_anyhow());
    }

    output::success(
        request.json,
        format!("Verified deployment URL ({})", response.status_code),
        output::Phase::Deploy,
    );

    Ok(DeployVerificationOutput {
        status: "passed",
        url,
        path,
        status_code: response.status_code,
        used_preview_bypass,
        preview_access_revoked: used_preview_bypass.then_some(true),
    })
}

fn verification_path(health_check: Option<&super::ResolvedHealthCheck>) -> String {
    health_check
        .and_then(|health_check| health_check.path.as_deref())
        .filter(|path| path.starts_with('/'))
        .unwrap_or("/")
        .to_string()
}

pub(super) fn verification_url(base_url: &str, path: &str) -> anyhow::Result<String> {
    let mut url = url::Url::parse(base_url)
        .with_context(|| format!("deployment URL is not valid: {base_url}"))?;
    url.set_path(path);
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

pub(super) async fn wait_for_preview_access<F, Fut>(
    timeout: Duration,
    interval: Duration,
    mut fetch: F,
) -> anyhow::Result<PreviewAccessResponse>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<PreviewAccessResponse>>,
{
    tokio::time::timeout(timeout, async {
        loop {
            let response = fetch().await?;
            if !matches!(response.status_code, 401 | 403 | 429 | 503) {
                return Ok(response);
            }
            tokio::time::sleep(response.retry_after.unwrap_or(interval).max(interval)).await;
        }
    })
    .await
    .context("temporary preview access did not become available before the deadline")?
}

pub(super) async fn verify_with_preview_access(
    url: &str,
    credential: &str,
) -> anyhow::Result<VerificationResponse> {
    let deadline = tokio::time::Instant::now() + PREVIEW_ACCESS_READY_TIMEOUT;
    let client = verification_http_client()?;
    let exchange_url = verification_url(url, "/.onreza/preview/credential")?;
    let response = wait_for_preview_access(
        deadline.saturating_duration_since(tokio::time::Instant::now()),
        Duration::from_secs(1),
        || async {
            let response = client
                .post(&exchange_url)
                .json(&serde_json::json!({"kind":"BYPASS","credential":credential}))
                .send()
                .await
                .context("preview credential exchange failed")?;
            let cookie = response
                .headers()
                .get_all(SET_COOKIE)
                .iter()
                .filter_map(|header| header.to_str().ok())
                .filter(|header| header.len() <= 4096)
                .filter_map(|header| header.split(';').next())
                .find(|cookie| {
                    cookie
                        .strip_prefix(PREVIEW_COOKIE_NAME)
                        .is_some_and(|value| value.starts_with("=P2.") && value.len() > 4)
                })
                .map(HeaderValue::from_str)
                .transpose()?;
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|header| header.to_str().ok())
                .and_then(|value| {
                    value
                        .parse::<u64>()
                        .ok()
                        .map(Duration::from_secs)
                        .or_else(|| {
                            chrono::DateTime::parse_from_rfc2822(value)
                                .ok()
                                .map(|time| {
                                    time.signed_duration_since(chrono::Utc::now())
                                        .to_std()
                                        .unwrap_or(Duration::ZERO)
                                })
                        })
                });
            Ok(PreviewAccessResponse {
                status_code: response.status().as_u16(),
                cookie,
                retry_after,
            })
        },
    )
    .await?;
    anyhow::ensure!(
        response.status_code == 204,
        "preview credential exchange returned HTTP {}",
        response.status_code
    );
    let cookie = response
        .cookie
        .context("preview credential exchange did not return a capability cookie")?;
    // Re-exchanging the bypass on the app GET would consume the initial attempt budget again.
    tokio::time::timeout_at(deadline, fetch_verification_url(url, Some(&cookie)))
        .await
        .context("temporary preview access verification did not finish before the deadline")?
}

fn verification_http_client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(VERIFY_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("failed to create deploy verification HTTP client")
}

async fn fetch_verification_url(
    url: &str,
    cookie: Option<&HeaderValue>,
) -> anyhow::Result<VerificationResponse> {
    let client = verification_http_client()?;
    let mut request = client.get(url);
    if let Some(cookie) = cookie {
        request = request.header(COOKIE, cookie.clone());
    }

    let response = request
        .send()
        .await
        .with_context(|| format!("request failed: GET {url}"))?;
    let status_code = response.status().as_u16();
    let location = response
        .headers()
        .get(LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    Ok(VerificationResponse {
        status_code,
        location,
    })
}

fn validate_response(
    url: &str,
    path: &str,
    response: &VerificationResponse,
    used_preview_bypass: bool,
) -> anyhow::Result<()> {
    if (200..300).contains(&response.status_code) {
        return Ok(());
    }

    let preview_auth_redirect = response
        .location
        .as_deref()
        .is_some_and(is_preview_auth_location);
    let message = if preview_auth_redirect {
        "deployment verification reached preview auth instead of the deployment artifact"
            .to_string()
    } else {
        format!(
            "deployment verification returned HTTP {}",
            response.status_code
        )
    };

    Err(verify_error(
        message,
        url,
        path,
        Some(response.status_code),
        response.location.as_deref(),
        used_preview_bypass,
    ))
}

fn needs_preview_bypass(response: &VerificationResponse) -> bool {
    matches!(response.status_code, 401 | 403)
        || (!(200..300).contains(&response.status_code)
            && response
                .location
                .as_deref()
                .is_some_and(is_preview_auth_location))
}

#[cfg(test)]
pub(super) fn response_needs_preview_bypass(status_code: u16, location: Option<&str>) -> bool {
    needs_preview_bypass(&VerificationResponse {
        status_code,
        location: location.map(str::to_string),
    })
}

fn is_preview_auth_location(location: &str) -> bool {
    location.contains("/preview-auth") || location.contains("preview-auth?")
}

fn verify_error(
    message: String,
    url: &str,
    path: &str,
    status_code: Option<u16>,
    location: Option<&str>,
    used_preview_bypass: bool,
) -> anyhow::Error {
    CliError::new("DEPLOY_VERIFY_FAILED", message)
        .phase(output::Phase::Deploy)
        .details(serde_json::json!({
            "url": url,
            "path": path,
            "statusCode": status_code,
            "location": location,
            "usedPreviewBypass": used_preview_bypass,
        }))
        .hint(
            "For preview deployments, verification uses a temporary `nrz preview access` bypass and revokes it after the check.",
        )
        .into_anyhow()
}
