use std::time::Duration;

use anyhow::Context;
use reqwest::header::{HeaderName, HeaderValue, LOCATION};
use serde::Serialize;

use crate::api::ApiClient;
use crate::errors::CliError;
use crate::output;

const VERIFY_TIMEOUT: Duration = Duration::from_secs(20);

pub(super) struct DeployVerificationRequest<'a> {
    pub(super) api_client: &'a ApiClient,
    pub(super) project_id: &'a str,
    pub(super) url: &'a str,
    pub(super) preview_protected: bool,
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

pub(super) async fn verify_deployment(
    request: DeployVerificationRequest<'_>,
) -> anyhow::Result<DeployVerificationOutput> {
    let path = verification_path(request.health_check);
    let url = verification_url(request.url, &path)?;

    output::status(
        request.json,
        "~",
        format!("Verifying deployment URL: {url}"),
        output::Phase::Deploy,
    );

    let (header, access_secret_id) = if request.preview_protected {
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
            Some(VerificationHeader { name, value }),
            Some(access.secret_id),
        )
    } else {
        (None, None)
    };

    let response = fetch_verification_url(&url, header.as_ref()).await;
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
            request.preview_protected,
        )
    })?;

    validate_response(&url, &path, &response, request.preview_protected)?;

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
        used_preview_bypass: request.preview_protected,
        preview_access_revoked: request.preview_protected.then_some(true),
    })
}

fn verification_path(health_check: Option<&super::ResolvedHealthCheck>) -> String {
    health_check
        .and_then(|health_check| health_check.path.as_deref())
        .filter(|path| path.starts_with('/'))
        .unwrap_or("/")
        .to_string()
}

fn verification_url(base_url: &str, path: &str) -> anyhow::Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_url_replaces_path_and_query() {
        assert_eq!(
            verification_url("https://example.test/old?x=1", "/health").unwrap(),
            "https://example.test/health"
        );
    }

    #[test]
    fn preview_auth_redirect_is_detected() {
        assert!(is_preview_auth_location(
            "https://app.onreza-stage.ru/preview-auth?projectId=1"
        ));
        assert!(!is_preview_auth_location("https://example.test/login"));
    }
}
