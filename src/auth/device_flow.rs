use std::time::Duration;

use anyhow::{Context, bail};
use nrz_api::{
    Device200Response, ErrorResponseError, PostV1deviceRequest, PostV1deviceResponse,
    PostV1deviceTokenRequest, PostV1deviceTokenResponse, Token200Response, TokenRequestBody,
};

use crate::api::{ApiClient, client::ensure_success};

pub async fn request_device_code(client: &ApiClient) -> anyhow::Result<Device200Response> {
    let response = client
        .platform()?
        .post_v1device(PostV1deviceRequest {})
        .await
        .context("failed to request device code")?;
    let device = match PostV1deviceRequest::parse_response(ensure_success(response).await?)
        .await
        .map_err(nrz_api::response_error)?
    {
        PostV1deviceResponse::Ok(device) => device,
        _ => bail!("unexpected successful response from the device authorization API"),
    };
    positive_seconds(device.interval, "poll interval")?;
    positive_seconds(device.expires_in, "device code lifetime")?;
    Ok(device)
}

fn positive_seconds(value: i64, field: &str) -> anyhow::Result<Duration> {
    let seconds = u64::try_from(value).with_context(|| format!("invalid {field}"))?;
    if seconds == 0 {
        bail!("invalid {field}: must be positive");
    }
    Ok(Duration::from_secs(seconds))
}

pub async fn poll_for_token(
    client: &ApiClient,
    device_code: &str,
    interval: i64,
    expires_in: i64,
) -> anyhow::Result<Token200Response> {
    let lifetime = positive_seconds(expires_in, "device code lifetime")?;
    let deadline = tokio::time::Instant::now()
        .checked_add(lifetime)
        .context("device code lifetime exceeds the supported range")?;
    let mut poll_interval = positive_seconds(interval, "poll interval")?;
    let platform = client.platform()?;
    loop {
        let next_poll = tokio::time::Instant::now()
            .checked_add(poll_interval)
            .context("poll interval exceeds the supported range")?;
        tokio::time::sleep_until(next_poll.min(deadline)).await;
        if tokio::time::Instant::now() >= deadline {
            bail!("device authorization timed out");
        }
        let request = PostV1deviceTokenRequest {
            body: TokenRequestBody {
                device_code: device_code.to_string(),
                grant_type: Some("urn:ietf:params:oauth:grant-type:device_code".to_string()),
            },
        };
        let poll = async {
            let response = platform
                .post_v1device_token(request)
                .await
                .context("failed to poll for token")?;
            let response = if response.status() == reqwest::StatusCode::BAD_REQUEST {
                response
            } else {
                ensure_success(response).await?
            };
            PostV1deviceTokenRequest::parse_response(response)
                .await
                .map_err(nrz_api::response_error)
        };
        let response = tokio::time::timeout_at(deadline, poll)
            .await
            .context("device authorization timed out")??;
        match response {
            PostV1deviceTokenResponse::Ok(token) => {
                if token.access_token.is_empty() || token.workspace_slug.is_empty() {
                    bail!("device authorization returned an incomplete token response");
                }
                positive_seconds(token.expires_in, "access token lifetime")?;
                return Ok(token);
            }
            PostV1deviceTokenResponse::BadRequest(error) => match error.error {
                ErrorResponseError::AuthorizationPending => {}
                ErrorResponseError::SlowDown => {
                    poll_interval = poll_interval
                        .checked_add(Duration::from_secs(5))
                        .context("poll interval exceeds the supported range")?;
                }
                ErrorResponseError::ExpiredToken => bail!("device code expired. Please try again."),
                ErrorResponseError::InvalidGrant => bail!("authorization failed: invalid_grant"),
            },
            _ => bail!("unexpected response from the device token API"),
        }
    }
}
