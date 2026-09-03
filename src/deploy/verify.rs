use std::time::Duration;

use anyhow::Context;
use reqwest::header::{HeaderName, HeaderValue, LOCATION};
use serde::{Deserialize, Serialize};

use crate::api::{ApiClient, path_segment};
use crate::errors::CliError;
use crate::output;

const VERIFY_TIMEOUT: Duration = Duration::from_secs(20);
const PRODUCTION_ALIAS_LOOKUP_ATTEMPTS: u8 = 20;
const PRODUCTION_ALIAS_LOOKUP_INTERVAL: Duration = Duration::from_millis(250);

pub(super) struct DeployVerificationRequest<'a> {
    pub(super) api_client: &'a ApiClient,
    pub(super) deployment_id: &'a str,
    pub(super) project_id: &'a str,
    pub(super) url: &'a str,
    pub(super) production: bool,
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

struct VerificationHeader {
    name: HeaderName,
    value: HeaderValue,
}

struct VerificationResponse {
    status_code: u16,
    location: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentUrlsResponse {
    deployment_urls: Vec<DeploymentUrl>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentUrl {
    full_url: String,
    alias_type: String,
}

pub(super) async fn verify_deployment(
    request: DeployVerificationRequest<'_>,
) -> anyhow::Result<DeployVerificationOutput> {
    let path = verification_path(request.health_check);
    let base_url = resolve_verification_base_url(&request).await?;
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
        if needs_preview_bypass(&initial_response) {
            let access = crate::preview::create_preview_access(
                request.api_client,
                request.project_id,
                "nrz deploy --verify".to_string(),
                Some(url.clone()),
                crate::preview::AGENT_PREVIEW_ACCESS_TTL_SECONDS,
            )
            .await
            .context("failed to create temporary preview access for deploy verification")?;
            let name = HeaderName::from_bytes(access.header_name.as_bytes())
                .context("preview access returned an invalid header name")?;
            let value = HeaderValue::from_str(&access.header_value)
                .context("preview access returned an invalid header value")?;
            (
                fetch_verification_url(&url, Some(&VerificationHeader { name, value })).await,
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

async fn resolve_verification_base_url(
    request: &DeployVerificationRequest<'_>,
) -> anyhow::Result<String> {
    if !request.production {
        return Ok(request.url.to_string());
    }

    for attempt in 0..PRODUCTION_ALIAS_LOOKUP_ATTEMPTS {
        let response: DeploymentUrlsResponse = request
            .api_client
            .get(&format!(
                "/v1/deployments/{}",
                path_segment(request.deployment_id)
            ))
            .await
            .context("failed to resolve production deployment URL")?;
        if let Some(url) = production_alias_url(&response.deployment_urls) {
            return Ok(url.to_string());
        }
        if attempt + 1 < PRODUCTION_ALIAS_LOOKUP_ATTEMPTS {
            tokio::time::sleep(PRODUCTION_ALIAS_LOOKUP_INTERVAL).await;
        }
    }

    Ok(request.url.to_string())
}

fn production_alias_url(urls: &[DeploymentUrl]) -> Option<&str> {
    urls.iter()
        .find(|url| url.alias_type == "PRODUCTION_ALIAS")
        .map(|url| url.full_url.as_str())
}

#[cfg(test)]
pub(super) fn production_alias_url_from_response(response: &str) -> anyhow::Result<Option<String>> {
    let response: DeploymentUrlsResponse = serde_json::from_str(response)?;
    Ok(production_alias_url(&response.deployment_urls).map(str::to_string))
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

async fn fetch_verification_url(
    url: &str,
    header: Option<&VerificationHeader>,
) -> anyhow::Result<VerificationResponse> {
    let client = reqwest::Client::builder()
        .timeout(VERIFY_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("failed to create deploy verification HTTP client")?;

    let mut request = client.get(url);
    if let Some(header) = header {
        request = request.header(header.name.clone(), header.value.clone());
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
    !(200..300).contains(&response.status_code)
        && response
            .location
            .as_deref()
            .is_some_and(is_preview_auth_location)
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
