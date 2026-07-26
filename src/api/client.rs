use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use base64::Engine;
use bytes::Bytes;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, HeaderMap, HeaderValue, IF_NONE_MATCH};
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

#[derive(Debug, Clone)]
pub(crate) struct PresignedPutResult {
    pub(crate) e_tag: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "kebab-case")]
pub(crate) struct PresignedPutHeaders {
    pub(crate) content_type: Option<String>,
    pub(crate) if_none_match: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresignedHeadVerify {
    pub(crate) url: String,
    pub(crate) content_length: u64,
    pub(crate) sha256: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ConditionalUploadConflict {
    reason: String,
}

impl ConditionalUploadConflict {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for ConditionalUploadConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "conditional upload target exists but does not match expected source object: {}",
            self.reason
        )
    }
}

impl std::error::Error for ConditionalUploadConflict {}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PresignedHeadVerification {
    Matches,
    Conflicts(String),
}

impl PresignedPutHeaders {
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(crate) fn if_none_match_any() -> Self {
        Self {
            content_type: None,
            if_none_match: Some("*".to_string()),
        }
    }

    fn is_conditional_create(&self) -> bool {
        self.if_none_match.as_deref() == Some("*")
    }

    fn to_header_map(&self) -> anyhow::Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        if let Some(value) = &self.content_type {
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_str(value)
                    .with_context(|| format!("invalid Content-Type header value: {value}"))?,
            );
        }
        if let Some(value) = &self.if_none_match {
            headers.insert(
                IF_NONE_MATCH,
                HeaderValue::from_str(value)
                    .with_context(|| format!("invalid If-None-Match header value: {value}"))?,
            );
        }
        Ok(headers)
    }
}

impl ApiClient {
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

    /// PUT a SOURCE_BUNDLE_V1 object to a server-issued conditioned presigned URL.
    ///
    /// The presign signature binds **both** `Content-Length` and `x-amz-checksum-sha256`
    /// (RFC: `deployment-artifacts/INDEX.md` "No-overwrite invariant"). The CLI must
    /// match exactly:
    /// - `Content-Length` is set automatically by reqwest from `data.len()`.
    /// - `x-amz-checksum-sha256` is the base64 of the SHA-256's raw 32 bytes
    ///   (NOT base64 of the hex string — common foot-gun).
    /// - Additional signed headers from the upload target, such as
    ///   `Content-Type: application/zstd` and `If-None-Match: *` for write-once
    ///   source objects, must be sent verbatim. Legacy PACK blob PUTs pass an
    ///   empty header set, so they remain free of unsigned Content-Type.
    ///
    /// Retries transient failures (429, 408, 5xx, network errors) with exponential
    /// backoff + full jitter, honoring `Retry-After`. Permanent 4xx integrity
    /// rejects (400 BadDigest / 403 SignatureDoesNotMatch) propagate immediately.
    /// `412 Precondition Failed` is accepted only for conditional create PUTs
    /// after a presigned HEAD confirms the existing object size and checksum.
    #[cfg(test)]
    pub(crate) async fn put_blob_with_headers(
        &self,
        url: &str,
        data: Bytes,
        sha256_hex: &str,
        headers: &PresignedPutHeaders,
    ) -> anyhow::Result<()> {
        self.put_blob_capture_with_headers(url, data, sha256_hex, headers)
            .await
            .map(|_| ())
    }

    pub(crate) async fn put_blob_with_headers_and_verify(
        &self,
        url: &str,
        data: Bytes,
        sha256_hex: &str,
        headers: &PresignedPutHeaders,
        verify_head: Option<&PresignedHeadVerify>,
    ) -> anyhow::Result<()> {
        self.put_blob_capture_with_policy_and_headers(
            url,
            data,
            sha256_hex,
            &UploadRetryPolicy::production(),
            headers,
            verify_head,
        )
        .await
        .map(|_| ())
    }

    #[cfg(test)]
    pub(crate) async fn put_blob_with_policy(
        &self,
        url: &str,
        data: Bytes,
        sha256_hex: &str,
        policy: &UploadRetryPolicy,
    ) -> anyhow::Result<()> {
        self.put_blob_capture_with_policy_and_headers(
            url,
            data,
            sha256_hex,
            policy,
            &PresignedPutHeaders::empty(),
            None,
        )
        .await
        .map(|_| ())
    }

    #[cfg(test)]
    pub(crate) async fn put_blob_with_policy_headers_and_verify(
        &self,
        url: &str,
        data: Bytes,
        sha256_hex: &str,
        policy: &UploadRetryPolicy,
        headers: &PresignedPutHeaders,
        verify_head: Option<&PresignedHeadVerify>,
    ) -> anyhow::Result<()> {
        self.put_blob_capture_with_policy_and_headers(
            url,
            data,
            sha256_hex,
            policy,
            headers,
            verify_head,
        )
        .await
        .map(|_| ())
    }

    pub(crate) async fn put_blob_capture(
        &self,
        url: &str,
        data: Bytes,
        sha256_hex: &str,
    ) -> anyhow::Result<PresignedPutResult> {
        self.put_blob_capture_with_headers(url, data, sha256_hex, &PresignedPutHeaders::empty())
            .await
    }

    pub(crate) async fn put_blob_capture_with_headers(
        &self,
        url: &str,
        data: Bytes,
        sha256_hex: &str,
        headers: &PresignedPutHeaders,
    ) -> anyhow::Result<PresignedPutResult> {
        self.put_blob_capture_with_policy_and_headers(
            url,
            data,
            sha256_hex,
            &UploadRetryPolicy::production(),
            headers,
            None,
        )
        .await
    }

    async fn put_blob_capture_with_policy_and_headers(
        &self,
        url: &str,
        data: Bytes,
        sha256_hex: &str,
        policy: &UploadRetryPolicy,
        headers: &PresignedPutHeaders,
        verify_head: Option<&PresignedHeadVerify>,
    ) -> anyhow::Result<PresignedPutResult> {
        let checksum_b64 = sha256_hex_to_base64(sha256_hex)
            .with_context(|| format!("invalid SHA-256 for blob upload: {sha256_hex}"))?;
        let signed_headers = headers.to_header_map()?;

        let started = Instant::now();
        let mut attempt: u32 = 0;
        let mut last_err: Option<anyhow::Error> = None;

        loop {
            attempt += 1;
            let mut request = self
                .upload_client
                .put(url)
                .header("x-amz-checksum-sha256", &checksum_b64);
            for (name, value) in &signed_headers {
                request = request.header(name, value);
            }
            let send_result = request.body(data.clone()).send().await;

            let (reason, retry_after) = match send_result {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let e_tag = resp
                            .headers()
                            .get(ETAG)
                            .and_then(|value| value.to_str().ok())
                            .map(str::to_string);
                        return Ok(PresignedPutResult { e_tag });
                    }
                    if status == reqwest::StatusCode::PRECONDITION_FAILED
                        && headers.is_conditional_create()
                    {
                        let verify_head = verify_head.with_context(|| {
                            "conditional create returned 412 but server did not provide verifyHead"
                        })?;
                        match self
                            .verify_presigned_head(
                                verify_head,
                                &UploadRetryPolicy::head_verification(),
                            )
                            .await?
                        {
                            PresignedHeadVerification::Matches => {
                                return Ok(PresignedPutResult { e_tag: None });
                            }
                            PresignedHeadVerification::Conflicts(reason) => {
                                return Err(anyhow::Error::new(ConditionalUploadConflict::new(
                                    reason,
                                )));
                            }
                        }
                    }
                    let retry_after = parse_retry_after(resp.headers());
                    let body = resp
                        .text()
                        .await
                        .unwrap_or_else(|e| format!("<failed to read response: {e}>"));
                    if !is_transient_status(status) {
                        bail!("{}", explain_s3_failure(status, &body));
                    }
                    (
                        format!("HTTP {}: {}", status, truncate(&body, 200)),
                        retry_after,
                    )
                }
                Err(err) => {
                    let is_transient = is_transient_reqwest_err(&err);
                    let err = err.without_url();
                    if !is_transient {
                        return Err(anyhow::Error::new(err).context("S3 upload failed (permanent)"));
                    }
                    let reason = format!("network error: {err}");
                    last_err = Some(anyhow::Error::new(err));
                    (reason, None)
                }
            };

            let elapsed = started.elapsed();
            let remaining = policy.budget.saturating_sub(elapsed);

            // Server asked us to wait longer than we have — retrying is pointless,
            // fail fast with a clear reason instead of sleeping the rest of the budget.
            if let Some(ra) = retry_after
                && ra > remaining
            {
                return Err(exhaustion_error(
                    attempt,
                    elapsed,
                    format!("{reason}; Retry-After {ra:?} exceeds remaining budget {remaining:?}"),
                    last_err,
                ));
            }

            if attempt >= policy.max_attempts || remaining.is_zero() {
                return Err(exhaustion_error(attempt, elapsed, reason, last_err));
            }

            let delay = next_delay(attempt, retry_after, policy).min(remaining);
            tracing::warn!(
                attempt,
                delay_ms = delay.as_millis() as u64,
                %reason,
                "S3 upload retrying"
            );
            tokio::time::sleep(delay).await;
        }
    }

    async fn verify_presigned_head(
        &self,
        verify: &PresignedHeadVerify,
        policy: &UploadRetryPolicy,
    ) -> anyhow::Result<PresignedHeadVerification> {
        let expected_checksum_b64 = sha256_hex_to_base64(&verify.sha256)
            .with_context(|| format!("invalid SHA-256 for HEAD verification: {}", verify.sha256))?;
        let mut attempt = 0u32;
        let started = Instant::now();

        let resp = loop {
            attempt += 1;
            let result = self.upload_client.head(&verify.url).send().await;
            let (reason, retry_after) = match result {
                Ok(resp) if resp.status().is_success() => break resp,
                Ok(resp) => {
                    let status = resp.status();
                    let retry_after = parse_retry_after(resp.headers());
                    if !is_transient_head_verification_status(status) {
                        bail!(
                            "conditional upload target exists but HEAD verification returned {status}"
                        );
                    }
                    let body = resp
                        .text()
                        .await
                        .unwrap_or_else(|error| format!("<failed to read response: {error}>"));
                    (
                        format!("HEAD HTTP {status}: {}", truncate(&body, 200)),
                        retry_after,
                    )
                }
                Err(error) => {
                    let is_transient = is_transient_reqwest_err(&error);
                    let error = error.without_url();
                    if !is_transient {
                        return Err(anyhow::Error::new(error)
                            .context("failed to HEAD existing conditional upload target"));
                    }
                    (format!("HEAD network error: {error}"), None)
                }
            };

            let elapsed = started.elapsed();
            let remaining = policy.budget.saturating_sub(elapsed);
            if attempt >= policy.max_attempts || remaining.is_zero() {
                bail!(
                    "conditional upload HEAD verification failed after {attempt} attempt(s) in {elapsed:?}: {reason}"
                );
            }
            if let Some(retry_after) = retry_after
                && retry_after > remaining
            {
                bail!(
                    "conditional upload HEAD verification cannot retry: Retry-After {retry_after:?} exceeds remaining budget {remaining:?}"
                );
            }

            let delay = next_delay(attempt, retry_after, policy).min(remaining);
            tracing::warn!(
                attempt,
                delay_ms = delay.as_millis() as u64,
                %reason,
                "conditional upload HEAD verification retrying"
            );
            tokio::time::sleep(delay).await;
        };
        let status = resp.status();
        debug_assert!(status.is_success());

        let Some(content_length) = resp
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
        else {
            return Ok(PresignedHeadVerification::Conflicts(
                "HEAD verification returned no Content-Length".to_string(),
            ));
        };
        if content_length != verify.content_length {
            return Ok(PresignedHeadVerification::Conflicts(format!(
                "size mismatch: expected {}, got {}",
                verify.content_length, content_length
            )));
        }

        let Some(checksum) = resp
            .headers()
            .get("x-amz-checksum-sha256")
            .and_then(|value| value.to_str().ok())
        else {
            return Ok(PresignedHeadVerification::Conflicts(
                "HEAD verification returned no x-amz-checksum-sha256".to_string(),
            ));
        };
        if checksum != expected_checksum_b64 {
            return Ok(PresignedHeadVerification::Conflicts(
                "SHA-256 mismatch".to_string(),
            ));
        }

        Ok(PresignedHeadVerification::Matches)
    }
}

// ── Retry policy (S3 uploads) ────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct UploadRetryPolicy {
    max_attempts: u32,
    budget: Duration,
    base: Duration,
    cap: Duration,
}

impl UploadRetryPolicy {
    pub(crate) const fn production() -> Self {
        Self {
            max_attempts: 8,
            budget: Duration::from_secs(180),
            base: Duration::from_millis(500),
            cap: Duration::from_secs(30),
        }
    }

    const fn head_verification() -> Self {
        Self {
            max_attempts: 4,
            budget: Duration::from_secs(30),
            base: Duration::from_millis(250),
            cap: Duration::from_secs(2),
        }
    }

    #[cfg(test)]
    pub(crate) const fn fast_for_tests() -> Self {
        Self {
            max_attempts: 5,
            budget: Duration::from_secs(10),
            base: Duration::from_millis(10),
            cap: Duration::from_millis(80),
        }
    }

    /// Policy crafted so the budget expires before max_attempts: the exponential
    /// sleep exceeds budget well before the 100-attempt ceiling is reached.
    #[cfg(test)]
    pub(crate) const fn budget_exhaust_for_tests() -> Self {
        Self {
            max_attempts: 100,
            budget: Duration::from_secs(2),
            base: Duration::from_millis(500),
            cap: Duration::from_millis(1_000),
        }
    }

    #[cfg(test)]
    pub(crate) const fn expires_before_head_for_tests() -> Self {
        Self {
            max_attempts: 1,
            budget: Duration::from_millis(1),
            base: Duration::from_millis(1),
            cap: Duration::from_millis(1),
        }
    }

    #[cfg(test)]
    pub(crate) const fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
}

fn is_transient_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::REQUEST_TIMEOUT
        || status.is_server_error()
}

fn is_transient_head_verification_status(status: reqwest::StatusCode) -> bool {
    // A 412 PUT proves the key existed, but an immediately following HEAD may
    // briefly observe a replica/cache that has not caught up yet.
    status == reqwest::StatusCode::NOT_FOUND || is_transient_status(status)
}

fn is_transient_reqwest_err(err: &reqwest::Error) -> bool {
    // No response received (transport-level) => transient. Builder/decode errors
    // are permanent: retrying won't rescue a malformed URL or a truncated response
    // from an otherwise-successful handshake.
    err.status().is_none() && !err.is_builder() && !err.is_decode()
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let raw = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    // S3-compatible providers emit seconds; HTTP-date form is allowed by RFC 7231
    // but not used for rate-limit responses in practice. Non-numeric values fall
    // through to plain exponential backoff.
    raw.trim().parse::<u64>().ok().map(Duration::from_secs)
}

fn next_delay(attempt: u32, retry_after: Option<Duration>, policy: &UploadRetryPolicy) -> Duration {
    // AWS-style "full jitter": uniform in [0, capped_exp). Decorrelates concurrent
    // retries maximally and keeps `policy.cap` as the actual per-sleep ceiling.
    let exp_ms = (policy.base.as_millis() as u64)
        .saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1)));
    let capped_ms = exp_ms.min(policy.cap.as_millis() as u64);
    let backoff = Duration::from_millis(jitter(capped_ms));

    match retry_after {
        Some(ra) => backoff.max(ra),
        None => backoff,
    }
}

/// Decorrelated jitter via SplitMix64 over a process-wide atomic counter.
///
/// `SystemTime::now()` was rejected here because tasks woken from a shared
/// `tokio::time::sleep` after a common 429 read wall-clock within the same µs
/// window, producing correlated modulo results. SplitMix64 guarantees every
/// call gets a distinct mix regardless of wake timing.
fn jitter(max_ms: u64) -> u64 {
    if max_ms == 0 {
        return 0;
    }
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed);
    let mut z = n;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^= z >> 31;
    z % max_ms
}

fn exhaustion_error(
    attempt: u32,
    elapsed: Duration,
    reason: String,
    last_err: Option<anyhow::Error>,
) -> anyhow::Error {
    let msg = format!("S3 upload failed after {attempt} attempt(s) in {elapsed:?}: {reason}");
    match last_err {
        Some(e) => e.context(msg),
        None => anyhow::anyhow!("{msg}"),
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        let mut end = n;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

/// Translate a permanent S3 failure into an actionable CLI message.
///
/// S3 returns failures as XML like `<Error><Code>BadDigest</Code>...</Error>`.
/// The raw body dumped verbatim is opaque to users — the two codes specific to
/// the conditioned-PUT contract (`BadDigest`, `SignatureDoesNotMatch`) get
/// bespoke hints; everything else falls back to the truncated body so we never
/// hide diagnostic information.
pub(crate) fn explain_s3_failure(status: reqwest::StatusCode, body: &str) -> String {
    match parse_s3_error_code(body).as_deref() {
        Some("BadDigest") => format!(
            "S3 rejected the upload with BadDigest ({status}): the body's SHA-256 didn't match \
             the signed `x-amz-checksum-sha256`. The file likely changed between scan and upload, \
             or its content drifted. Rebuild and redeploy."
        ),
        Some("SignatureDoesNotMatch") => format!(
            "S3 rejected the upload with SignatureDoesNotMatch ({status}): Content-Length, \
             Content-Type, If-None-Match, or the SHA-256 header didn't match the presigned \
             signature. Rebuild and redeploy."
        ),
        _ => format!("S3 upload failed ({status}): {}", truncate(body, 200)),
    }
}

/// Hand-rolled extractor for `<Code>...</Code>` from an S3 error XML body.
/// The body is small (<1 KB) and well-shaped; pulling in a real XML parser
/// would be overkill for one tag, and a stray "<Code" inside `<Message>` is
/// not a concern S3 produces in practice.
fn parse_s3_error_code(body: &str) -> Option<String> {
    let start = body.find("<Code>")? + "<Code>".len();
    let end = body[start..].find("</Code>")?;
    Some(body[start..start + end].trim().to_string())
}

/// Convert a 64-char lowercase hex SHA-256 into the base64 form S3 expects in
/// `x-amz-checksum-sha256`.
///
/// The checksum is base64 of the **raw 32 bytes**, not base64 of the hex
/// string. Mixing those up was the entire reason this helper exists in its own
/// function: a wrong encoding would just make every PUT 400 with no clue why.
pub(crate) fn sha256_hex_to_base64(hex: &str) -> anyhow::Result<String> {
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        bail!("expected 64 lowercase hex chars, got {} chars", hex.len());
    }
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .with_context(|| format!("invalid hex byte at offset {}", i * 2))?;
    }
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

fn build_upload_client() -> anyhow::Result<reqwest::Client> {
    // Per-request timeout must be < UploadRetryPolicy::production().budget so that
    // a hung connection gets cut and the retry loop can progress before the overall
    // budget expires. connect_timeout is separate so we fail fast on unreachable hosts.
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
