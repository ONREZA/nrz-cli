mod control_plane;

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use base64::Engine as _;
use bytes::Bytes;
use reqwest::header::{
    CONTENT_LENGTH, CONTENT_TYPE, ETAG, HeaderMap, HeaderValue, IF_NONE_MATCH, RETRY_AFTER,
};
use sha2::{Digest as _, Sha256};
use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _};
use tokio_util::io::ReaderStream;

use crate::SourcePublicationError;
use crate::publisher::{
    DeploymentPublicationStatus, ObjectHeadVerification, ObjectUploadHeaders, ObjectUploadRequest,
    ObjectUploadResult,
};
use crate::runtime::RuntimeFileUploadRequest;

const UPLOAD_MAX_ATTEMPTS: u32 = 8;
const UPLOAD_BUDGET: Duration = Duration::from_secs(180);
const UPLOAD_BASE_DELAY: Duration = Duration::from_millis(500);
const UPLOAD_MAX_DELAY: Duration = Duration::from_secs(30);
const HEAD_MAX_ATTEMPTS: u32 = 4;
const HEAD_BUDGET: Duration = Duration::from_secs(30);
const HEAD_BASE_DELAY: Duration = Duration::from_millis(250);
const HEAD_MAX_DELAY: Duration = Duration::from_secs(2);

enum UploadPayload<'a> {
    Bytes(&'a Bytes),
    File { path: &'a Path, content_length: u64 },
}

#[derive(Clone)]
pub struct HttpSourcePublicationTransport {
    base_url: String,
    api_client: reqwest::Client,
    upload_client: reqwest::Client,
}

impl HttpSourcePublicationTransport {
    pub fn authenticated(
        base_url: impl Into<String>,
        token: &[u8],
        user_agent: &str,
    ) -> Result<Self, SourcePublicationError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "User-Agent",
            HeaderValue::from_str(user_agent).map_err(|error| {
                SourcePublicationError::InvalidResponse(format!("invalid user agent: {error}"))
            })?,
        );
        headers.insert(
            "X-API-Key",
            HeaderValue::from_bytes(token).map_err(|error| {
                SourcePublicationError::InvalidResponse(format!("invalid API token: {error}"))
            })?,
        );
        let api_client = reqwest::Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(client_build_error)?;
        let upload_client = build_upload_client()?;
        Ok(Self::from_clients(base_url, api_client, upload_client))
    }

    #[must_use]
    pub fn from_clients(
        base_url: impl Into<String>,
        api_client: reqwest::Client,
        upload_client: reqwest::Client,
    ) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_client,
            upload_client,
        }
    }

    async fn put_object_inner(
        &self,
        request: ObjectUploadRequest,
    ) -> Result<ObjectUploadResult, SourcePublicationError> {
        self.upload_with_retry(
            &request.url,
            &request.sha256,
            &request.headers,
            request.verify_head.as_ref(),
            UploadPayload::Bytes(&request.bytes),
            "source_bundle",
        )
        .await
    }

    async fn put_file_inner(
        &self,
        request: RuntimeFileUploadRequest,
    ) -> Result<ObjectUploadResult, SourcePublicationError> {
        self.upload_with_retry(
            &request.url,
            &request.sha256,
            &request.headers,
            request.verify_head.as_ref(),
            UploadPayload::File {
                path: &request.path,
                content_length: request.content_length,
            },
            "runtime_artifact",
        )
        .await
    }

    async fn upload_with_retry(
        &self,
        url: &str,
        sha256: &str,
        headers: &ObjectUploadHeaders,
        verify_head: Option<&ObjectHeadVerification>,
        payload: UploadPayload<'_>,
        artifact_kind: &'static str,
    ) -> Result<ObjectUploadResult, SourcePublicationError> {
        let checksum = sha256_hex_to_base64(sha256)?;
        let started = Instant::now();
        let mut attempt = 0_u32;
        loop {
            attempt = attempt.saturating_add(1);
            let mut outbound = self
                .upload_client
                .put(url)
                .header("x-amz-checksum-sha256", &checksum);
            if let Some(content_type) = &headers.content_type {
                outbound = outbound.header(CONTENT_TYPE, content_type);
            }
            if let Some(if_none_match) = &headers.if_none_match {
                outbound = outbound.header(IF_NONE_MATCH, if_none_match);
            }
            outbound = match &payload {
                UploadPayload::Bytes(bytes) => outbound.body((*bytes).clone()),
                UploadPayload::File {
                    path,
                    content_length,
                } => {
                    let file = open_verified_upload_file(path, *content_length, sha256).await?;
                    outbound
                        .header(CONTENT_LENGTH, *content_length)
                        .body(reqwest::Body::wrap_stream(ReaderStream::new(file)))
                }
            };

            let outcome = outbound.send().await;
            let (reason, retry_after) = match outcome {
                Ok(response) if response.status().is_success() => {
                    let e_tag = response
                        .headers()
                        .get(ETAG)
                        .and_then(|value| value.to_str().ok())
                        .map(str::to_string);
                    return Ok(ObjectUploadResult { e_tag });
                }
                Ok(response)
                    if response.status() == reqwest::StatusCode::PRECONDITION_FAILED
                        && headers.if_none_match.as_deref() == Some("*") =>
                {
                    let verify = verify_head.ok_or_else(|| {
                        SourcePublicationError::ObjectUpload(
                            "conditional create returned 412 without verifyHead".to_string(),
                        )
                    })?;
                    match self.verify_existing_object(verify).await? {
                        HeadVerification::Matches => {
                            return Ok(ObjectUploadResult { e_tag: None });
                        }
                        HeadVerification::Conflicts(reason) => {
                            return Err(SourcePublicationError::ConditionalUploadConflict(reason));
                        }
                    }
                }
                Ok(response) => {
                    let status = response.status();
                    let retry_after = parse_retry_after(response.headers());
                    let body = response.text().await.unwrap_or_default();
                    if !is_transient_status(status) {
                        return Err(SourcePublicationError::ObjectUpload(explain_s3_failure(
                            status, &body,
                        )));
                    }
                    (format!("HTTP {status}"), retry_after)
                }
                Err(error) => {
                    if !is_ambiguous_transport(&error) {
                        return Err(SourcePublicationError::ObjectUpload(format!(
                            "permanent transport failure: {}",
                            error.without_url()
                        )));
                    }
                    (format!("transport failure: {}", error.without_url()), None)
                }
            };
            let remaining = UPLOAD_BUDGET.saturating_sub(started.elapsed());
            if attempt >= UPLOAD_MAX_ATTEMPTS || remaining.is_zero() {
                return Err(SourcePublicationError::ObjectUpload(format!(
                    "upload exhausted after {attempt} attempts: {reason}"
                )));
            }
            let delay = retry_delay(attempt, retry_after, UPLOAD_BASE_DELAY, UPLOAD_MAX_DELAY)
                .min(remaining);
            tracing::warn!(
                attempt,
                delay_ms = delay.as_millis() as u64,
                artifact_kind,
                %reason,
                "object upload retrying"
            );
            tokio::time::sleep(delay).await;
        }
    }

    async fn verify_existing_object(
        &self,
        verify: &ObjectHeadVerification,
    ) -> Result<HeadVerification, SourcePublicationError> {
        let expected_checksum = sha256_hex_to_base64(&verify.sha256)?;
        let started = Instant::now();
        let mut attempt = 0_u32;
        let response = loop {
            attempt = attempt.saturating_add(1);
            let outcome = self.upload_client.head(&verify.url).send().await;
            let (reason, retry_after) = match outcome {
                Ok(response) if response.status().is_success() => break response,
                Ok(response) => {
                    let status = response.status();
                    if status != reqwest::StatusCode::NOT_FOUND && !is_transient_status(status) {
                        return Err(SourcePublicationError::ObjectUpload(format!(
                            "conditional HEAD returned {status}"
                        )));
                    }
                    (
                        format!("HEAD HTTP {status}"),
                        parse_retry_after(response.headers()),
                    )
                }
                Err(error) => {
                    if !is_ambiguous_transport(&error) {
                        return Err(SourcePublicationError::ObjectUpload(format!(
                            "permanent HEAD transport failure: {}",
                            error.without_url()
                        )));
                    }
                    (
                        format!("HEAD transport failure: {}", error.without_url()),
                        None,
                    )
                }
            };
            let elapsed = started.elapsed();
            let remaining = HEAD_BUDGET.saturating_sub(elapsed);
            if attempt >= HEAD_MAX_ATTEMPTS || remaining.is_zero() {
                return Err(SourcePublicationError::ObjectUpload(format!(
                    "conditional HEAD exhausted after {attempt} attempts: {reason}"
                )));
            }
            let delay =
                retry_delay(attempt, retry_after, HEAD_BASE_DELAY, HEAD_MAX_DELAY).min(remaining);
            tokio::time::sleep(delay).await;
        };
        let Some(content_length) = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
        else {
            return Ok(HeadVerification::Conflicts(
                "HEAD returned no Content-Length".to_string(),
            ));
        };
        if content_length != verify.content_length {
            return Ok(HeadVerification::Conflicts(format!(
                "size mismatch: expected {}, got {content_length}",
                verify.content_length
            )));
        }
        let Some(checksum) = response
            .headers()
            .get("x-amz-checksum-sha256")
            .and_then(|value| value.to_str().ok())
        else {
            return Ok(HeadVerification::Conflicts(
                "HEAD returned no x-amz-checksum-sha256".to_string(),
            ));
        };
        if checksum != expected_checksum {
            return Ok(HeadVerification::Conflicts("SHA-256 mismatch".to_string()));
        }
        Ok(HeadVerification::Matches)
    }
}

fn build_upload_client() -> Result<reqwest::Client, SourcePublicationError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(client_build_error)
}

async fn open_verified_upload_file(
    path: &std::path::Path,
    expected_size: u64,
    expected_sha256: &str,
) -> Result<tokio::fs::File, SourcePublicationError> {
    let mut options = tokio::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(path)
        .await
        .map_err(|source| SourcePublicationError::Io {
            operation: "open runtime artifact for upload",
            source,
        })?;
    let metadata = file
        .metadata()
        .await
        .map_err(|source| SourcePublicationError::Io {
            operation: "inspect runtime artifact for upload",
            source,
        })?;
    if !metadata.is_file() || metadata.len() != expected_size {
        return Err(SourcePublicationError::InvalidSourceBundle(
            "runtime artifact size changed before upload".to_string(),
        ));
    }
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .await
            .map_err(|source| SourcePublicationError::Io {
                operation: "verify runtime artifact before upload",
                source,
            })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    if hex::encode(hasher.finalize()) != expected_sha256 {
        return Err(SourcePublicationError::InvalidSourceBundle(
            "runtime artifact digest changed before upload".to_string(),
        ));
    }
    file.seek(std::io::SeekFrom::Start(0))
        .await
        .map_err(|source| SourcePublicationError::Io {
            operation: "rewind verified runtime artifact for upload",
            source,
        })?;
    Ok(file)
}

fn client_build_error(error: reqwest::Error) -> SourcePublicationError {
    SourcePublicationError::InvalidResponse(format!("failed to create HTTP client: {error}"))
}

fn transport_error(error: reqwest::Error) -> SourcePublicationError {
    if is_ambiguous_transport(&error) {
        SourcePublicationError::AmbiguousTransport(error.without_url().to_string())
    } else {
        SourcePublicationError::InvalidResponse(error.without_url().to_string())
    }
}

fn is_ambiguous_transport(error: &reqwest::Error) -> bool {
    error.status().is_none() && !error.is_builder() && !error.is_decode()
}

fn sha256_hex_to_base64(value: &str) -> Result<String, SourcePublicationError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(SourcePublicationError::InvalidSourceBundle(
            "SHA-256 must be 64 lowercase hexadecimal characters".to_string(),
        ));
    }
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).map_err(|_| {
            SourcePublicationError::InvalidSourceBundle("SHA-256 is invalid".to_string())
        })?;
    }
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

fn is_transient_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::REQUEST_TIMEOUT
        || status.is_server_error()
}

fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    headers
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

fn retry_delay(
    attempt: u32,
    retry_after: Option<Duration>,
    base: Duration,
    cap: Duration,
) -> Duration {
    let exponent = 2_u64.saturating_pow(attempt.saturating_sub(1));
    let upper = (base.as_millis() as u64)
        .saturating_mul(exponent)
        .min(cap.as_millis() as u64);
    let backoff = Duration::from_millis(jitter(upper));
    retry_after.map_or(backoff, |hint| hint.max(backoff))
}

fn jitter(max_milliseconds: u64) -> u64 {
    if max_milliseconds == 0 {
        return 0;
    }
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let mut value = SEQUENCE.fetch_add(0x9e37_79b9_7f4a_7c15, Ordering::Relaxed);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (value ^ (value >> 31)) % max_milliseconds
}

fn explain_s3_failure(status: reqwest::StatusCode, body: &str) -> String {
    if body.contains("<Code>BadDigest</Code>") {
        return format!("S3 rejected the source checksum ({status})");
    }
    if body.contains("<Code>SignatureDoesNotMatch</Code>") {
        return format!("S3 rejected signed source upload headers ({status})");
    }
    format!("S3 returned a permanent upload error ({status})")
}

enum HeadVerification {
    Matches,
    Conflicts(String),
}
