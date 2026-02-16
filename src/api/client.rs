use anyhow::{Context, bail};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Deserialize)]
struct ApiError {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl ApiClient {
    pub fn anonymous() -> anyhow::Result<Self> {
        let base_url = resolve_base_url();
        let mut headers = HeaderMap::new();
        headers.insert(
            "User-Agent",
            HeaderValue::from_str(&format!("nrz-cli/{}", env!("CARGO_PKG_VERSION")))?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to create HTTP client")?;

        Ok(Self { client, base_url })
    }

    pub fn authenticated(token: &str) -> anyhow::Result<Self> {
        let base_url = resolve_base_url();
        let mut headers = HeaderMap::new();
        headers.insert(
            "User-Agent",
            HeaderValue::from_str(&format!("nrz-cli/{}", env!("CARGO_PKG_VERSION")))?,
        );
        headers.insert(
            "X-API-Key",
            HeaderValue::from_str(token).context("invalid token format")?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to create HTTP client")?;

        Ok(Self { client, base_url })
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("request failed: GET {path}"))?;

        check_response(resp).await
    }

    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("request failed: POST {path}"))?;

        check_response(resp).await
    }

    /// POST that returns the raw response without checking status.
    /// Used for device flow polling where 400 with `authorization_pending` is expected.
    pub async fn post_raw<B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> anyhow::Result<reqwest::Response> {
        let url = format!("{}{}", self.base_url, path);
        self.client
            .post(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("request failed: POST {path}"))
    }

    pub async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .post(&url)
            .send()
            .await
            .with_context(|| format!("request failed: POST {path}"))?;

        check_response(resp).await
    }

    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .with_context(|| format!("request failed: DELETE {path}"))?;

        check_response(resp).await
    }

    pub async fn delete_empty(&self, path: &str) -> anyhow::Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .with_context(|| format!("request failed: DELETE {path}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read response: {e}>"));
            if let Ok(api_err) = serde_json::from_str::<ApiError>(&body) {
                let msg = api_err
                    .message
                    .or(api_err.error)
                    .unwrap_or_else(|| format!("HTTP {status}"));
                bail!("API error ({}): {}", status, msg);
            }
            bail!("API error ({}): {}", status, body);
        }

        Ok(())
    }

    pub async fn patch<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .patch(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("request failed: PATCH {path}"))?;

        check_response(resp).await
    }

    /// PUT raw bytes to an absolute URL (for presigned S3 uploads).
    /// Uses a separate client without auth headers to avoid leaking credentials.
    pub async fn put_bytes(
        &self,
        url: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> anyhow::Result<()> {
        let plain_client = reqwest::Client::new();
        let resp = plain_client
            .put(url)
            .header("Content-Type", content_type)
            .body(data)
            .send()
            .await
            .context("S3 upload failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read response: {e}>"));
            bail!("S3 upload failed ({}): {}", status, body);
        }

        Ok(())
    }
}

fn resolve_base_url() -> String {
    std::env::var("NRZ_API_URL").unwrap_or_else(|_| "https://api.onreza.ru".to_string())
}

async fn check_response<T: DeserializeOwned>(resp: reqwest::Response) -> anyhow::Result<T> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp
            .text()
            .await
            .unwrap_or_else(|e| format!("<failed to read response: {e}>"));
        if let Ok(api_err) = serde_json::from_str::<ApiError>(&body) {
            let msg = api_err
                .message
                .or(api_err.error)
                .unwrap_or_else(|| format!("HTTP {status}"));
            bail!("API error ({}): {}", status, msg);
        }
        bail!("API error ({}): {}", status, body);
    }

    let body = resp.text().await.context("failed to read response body")?;
    serde_json::from_str(&body).with_context(|| format!("failed to parse response: {body}"))
}
