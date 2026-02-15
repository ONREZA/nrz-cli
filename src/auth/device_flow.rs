use anyhow::{Context, bail};
use serde::Deserialize;

use crate::api::ApiClient;

#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    #[allow(dead_code)]
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TokenResponse {
    Success {
        access_token: String,
        #[allow(dead_code)]
        token_type: String,
        workspace_slug: String,
        workspace_name: String,
    },
    Error {
        error: String,
    },
}

pub async fn request_device_code(client: &ApiClient) -> anyhow::Result<DeviceCodeResponse> {
    client
        .post(
            "/v1/device",
            &serde_json::Value::Object(serde_json::Map::new()),
        )
        .await
        .context("failed to request device code")
}

pub async fn poll_for_token(
    client: &ApiClient,
    device_code: &str,
    interval: u64,
    expires_in: u64,
) -> anyhow::Result<TokenResponse> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(expires_in);
    let poll_interval = std::time::Duration::from_secs(interval);

    loop {
        tokio::time::sleep(poll_interval).await;

        if tokio::time::Instant::now() >= deadline {
            bail!("device authorization timed out");
        }

        let body = serde_json::json!({
            "device_code": device_code,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
        });

        let raw_resp = client
            .post_raw("/v1/device/token", &body)
            .await
            .context("failed to poll for token")?;

        let resp_body = raw_resp
            .text()
            .await
            .context("failed to read poll response")?;

        let resp: TokenResponse =
            serde_json::from_str(&resp_body).context("failed to parse poll response")?;

        match &resp {
            TokenResponse::Error { error } if error == "authorization_pending" => continue,
            TokenResponse::Error { error } if error == "slow_down" => {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
            TokenResponse::Error { error } if error == "expired_token" => {
                bail!("device code expired. Please try again.");
            }
            TokenResponse::Error { error } => {
                bail!("authorization failed: {error}");
            }
            TokenResponse::Success { .. } => return Ok(resp),
        }
    }
}
