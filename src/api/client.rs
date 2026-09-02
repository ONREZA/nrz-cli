use std::time::Duration;

use anyhow::{Context, bail};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Deserialize)]
struct ApiError {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    code: Option<ApiErrorCode>,
    #[serde(default)]
    #[serde(alias = "retryAfterSeconds")]
    retry_after_seconds: Option<u64>,
    #[serde(default)]
    details: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ApiErrorCode {
    String(String),
    Number(serde_json::Number),
}

impl ApiErrorCode {
    fn into_string(self) -> String {
        match self {
            Self::String(value) => value,
            Self::Number(value) => value.to_string(),
        }
    }
}

/// Structured API error preserving code and details for downstream consumers.
/// Used to emit structured error lines in JSON mode (e.g., LIMIT_EXCEEDED with limit details).
#[derive(Debug)]
pub struct StructuredApiError {
    pub status: reqwest::StatusCode,
    pub code: String,
    pub message: String,
    pub retry_after_seconds: Option<u64>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ApiRetry {
    pub(crate) retry_after: Option<Duration>,
}

pub(crate) fn classify_api_retry(error: &anyhow::Error) -> Option<ApiRetry> {
    if let Some(error) = error.downcast_ref::<StructuredApiError>() {
        let retryable = error.status == reqwest::StatusCode::REQUEST_TIMEOUT
            || error.status == reqwest::StatusCode::TOO_MANY_REQUESTS
            || error.status.is_server_error()
            || matches!(
                error.code.as_str(),
                "OPERATION_IN_PROGRESS" | "SERVICE_UNAVAILABLE" | "TOO_MANY_REQUESTS"
            );
        return retryable.then_some(ApiRetry {
            retry_after: error.retry_after_seconds.map(Duration::from_secs),
        });
    }
    error
        .chain()
        .any(|cause| {
            cause
                .downcast_ref::<reqwest::Error>()
                .is_some_and(|error| error.is_timeout() || error.is_connect() || error.is_request())
        })
        .then_some(ApiRetry { retry_after: None })
}

impl std::fmt::Display for StructuredApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "API error ({} {}): {}",
            self.status.as_u16(),
            self.code,
            self.message
        )
    }
}

impl std::error::Error for StructuredApiError {}

/// Standard API envelope: `{"success":bool,"result":T,"errors":[],"messages":[]}`
#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    success: bool,
    result: Option<T>,
    #[serde(default)]
    errors: Vec<ApiEnvelopeMessage>,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelopeMessage {
    message: Option<String>,
    code: Option<i64>,
}

fn format_envelope_errors(errors: &[ApiEnvelopeMessage]) -> String {
    let msgs: Vec<String> = errors
        .iter()
        .filter_map(|e| match (&e.message, &e.code) {
            (Some(msg), Some(code)) => Some(format!("{msg} (code {code})")),
            (Some(msg), None) => Some(msg.clone()),
            (None, Some(code)) => Some(format!("error code {code}")),
            (None, None) => None,
        })
        .collect();
    if msgs.is_empty() {
        "unknown error".to_string()
    } else {
        msgs.join("; ")
    }
}

#[derive(Clone)]
pub struct ApiClient {
    client: reqwest::Client,
    /// Plain client without auth headers, reused for presigned S3 uploads.
    upload_client: reqwest::Client,
    base_url: String,
}

impl ApiClient {
    pub(crate) fn source_publication_transport(
        &self,
    ) -> nrz_source_publisher::HttpSourcePublicationTransport {
        nrz_source_publisher::HttpSourcePublicationTransport::from_clients(
            self.base_url.clone(),
            self.client.clone(),
            self.upload_client.clone(),
        )
    }

    pub fn anonymous() -> anyhow::Result<Self> {
        let base_url = resolve_base_url();
        let mut headers = HeaderMap::new();
        headers.insert(
            "User-Agent",
            HeaderValue::from_str(&format!("nrz-cli/{}", env!("CARGO_PKG_VERSION")))?,
        );

        let client = build_api_http_client(headers)?;

        let upload_client = build_upload_client()?;

        Ok(Self {
            client,
            upload_client,
            base_url,
        })
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

        let client = build_api_http_client(headers)?;

        let upload_client = build_upload_client()?;

        Ok(Self {
            client,
            upload_client,
            base_url,
        })
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
            let retry_after_seconds =
                parse_retry_after(resp.headers()).map(|duration| duration.as_secs());
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read response: {e}>"));
            return Err(extract_api_error(status, &body, retry_after_seconds));
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

    pub async fn put<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .put(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("request failed: PUT {path}"))?;

        check_response(resp).await
    }
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let raw = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    raw.trim().parse::<u64>().ok().map(Duration::from_secs)
}

fn build_upload_client() -> anyhow::Result<reqwest::Client> {
    // The shared publisher owns retries; keep each request below its total
    // upload budget so a hung connection cannot consume the entire operation.
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .context("failed to create upload HTTP client")
}

pub(super) fn build_api_http_client(headers: HeaderMap) -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("failed to create HTTP client")
}

fn resolve_base_url() -> String {
    std::env::var("NRZ_API_URL").unwrap_or_else(|_| "https://api.onreza.ru".to_string())
}

pub(super) fn extract_api_error(
    status: reqwest::StatusCode,
    body: &str,
    retry_after_seconds: Option<u64>,
) -> anyhow::Error {
    if let Ok(envelope) = serde_json::from_str::<ApiEnvelope<serde_json::Value>>(body) {
        if !envelope.errors.is_empty() {
            return anyhow::anyhow!(
                "API error ({}): {}",
                status,
                format_envelope_errors(&envelope.errors)
            );
        }
        if !envelope.success {
            return anyhow::anyhow!("API error ({}): request failed", status);
        }
    }
    if let Ok(api_err) = serde_json::from_str::<ApiError>(body) {
        let msg = api_err
            .message
            .clone()
            .or(api_err.error.clone())
            .unwrap_or_else(|| format!("HTTP {status}"));

        // Return StructuredApiError when API provides a structured error code
        if let Some(code) = api_err.code {
            return StructuredApiError {
                status,
                code: code.into_string(),
                message: msg,
                retry_after_seconds: api_err.retry_after_seconds.or(retry_after_seconds),
                details: api_err.details,
            }
            .into();
        }

        return anyhow::anyhow!("API error ({}): {}", status, msg);
    }
    anyhow::anyhow!("API error ({}): {}", status, body)
}

async fn check_response<T: DeserializeOwned>(resp: reqwest::Response) -> anyhow::Result<T> {
    let status = resp.status();
    if !status.is_success() {
        let retry_after_seconds =
            parse_retry_after(resp.headers()).map(|duration| duration.as_secs());
        let body = resp
            .text()
            .await
            .unwrap_or_else(|e| format!("<failed to read response: {e}>"));
        return Err(extract_api_error(status, &body, retry_after_seconds));
    }

    let body = resp.text().await.context("failed to read response body")?;

    // Try API envelope format: {"success":true,"result":T,...}
    // Use Value first to detect envelope structure without coupling to T
    if let Ok(envelope) = serde_json::from_str::<ApiEnvelope<serde_json::Value>>(&body) {
        if !envelope.success {
            bail!(
                "API returned success=false: {}",
                format_envelope_errors(&envelope.errors)
            );
        }
        let result_value = envelope
            .result
            .context("API returned success=true but result is null")?;
        return serde_json::from_value(result_value)
            .context("failed to deserialize API response result");
    }

    // Fallback: direct deserialization (for endpoints without envelope)
    serde_json::from_str(&body).with_context(|| format!("failed to parse response: {body}"))
}
