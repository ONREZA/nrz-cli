use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::Context;
use chrono::{SecondsFormat, Utc};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::api::{ApiClient, classify_api_retry};
use crate::cli::DeployArgs;
use crate::output;

const CREATE_TIMEOUT: Duration = Duration::from_secs(10);
const MUTATION_TIMEOUT: Duration = Duration::from_secs(10);
const MUTATION_RETRY_BUDGET: Duration = Duration::from_secs(30);
const UPLOAD_REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const UPLOAD_RETRY_BUDGET: Duration = Duration::from_secs(30);
const UPLOAD_RETRY_INITIAL_DELAY: Duration = Duration::from_millis(250);
const UPLOAD_RETRY_MAX_DELAY: Duration = Duration::from_secs(2);
const UPLOAD_FINISH_TIMEOUT: Duration = Duration::from_secs(35);
const UPLOAD_CHANNEL_CAPACITY: usize = 1_000;
const MAX_EVENTS_PER_BATCH: usize = 100;
const MAX_SESSION_EVENTS: u32 = 50_000;
const MAX_MESSAGE_BYTES: usize = 4_096;
const MAX_SESSION_BYTES: usize = 5 * 1024 * 1024;
const FLUSH_INTERVAL: Duration = Duration::from_secs(2);
const NOTICE_FILE: &str = "build-log-upload-notice-v2";
const TRUNCATION_MARKER: &str = "…[TRUNCATED]";
const REDACTED: &str = "[REDACTED]";
const PLATFORM_SECRET_ENV_KEYS: &[&str] = &["NRZ_TOKEN"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum BuildLogSource {
    LocalCli,
    RemoteBuilder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum BuildLogStream {
    User,
    Debug,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum BuildLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum BuildLogPhase {
    Init,
    Detect,
    Install,
    Build,
    Deploy,
    Upload,
    Activate,
    Complete,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum BuildLogOrigin {
    Cli,
    ChildStdout,
    ChildStderr,
    Builder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BuildLogUploadConfig {
    source: BuildLogSource,
    attempt: u32,
    upload_enabled: bool,
    include_debug: bool,
}

impl BuildLogUploadConfig {
    pub(super) fn from_args(args: &DeployArgs, attempt: u32) -> Self {
        let source = match std::env::var("NRZ_BUILD_LOG_SOURCE") {
            Ok(value) if value.trim().eq_ignore_ascii_case("REMOTE_BUILDER") => {
                BuildLogSource::RemoteBuilder
            }
            _ => BuildLogSource::LocalCli,
        };
        let env_upload_enabled =
            parse_env_toggle(std::env::var("NRZ_LOG_UPLOAD").ok().as_deref()).unwrap_or(true);
        let env_debug = parse_env_toggle(std::env::var("NRZ_LOG_UPLOAD_DEBUG").ok().as_deref())
            .unwrap_or(false);
        Self {
            source,
            attempt,
            upload_enabled: !args.no_log_upload && env_upload_enabled,
            include_debug: source == BuildLogSource::RemoteBuilder
                || args.log_upload_debug
                || env_debug,
        }
    }
}

pub(super) fn parse_env_toggle(value: Option<&str>) -> Option<bool> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionRequest<'a> {
    id: &'a str,
    project_id: &'a str,
    deployment_id: &'a str,
    attempt: u32,
    producer_id: &'a str,
    source: BuildLogSource,
    cli_version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    builder_version: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionResponse {
    session: SessionResponse,
}

#[derive(Debug, Deserialize)]
struct WorkspacePolicyResponse {
    enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    id: String,
    shipping_policy: String,
    next_seq: u32,
    accepted_bytes: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildLogEvent {
    seq: u32,
    timestamp: String,
    stream: BuildLogStream,
    level: BuildLogLevel,
    phase: BuildLogPhase,
    message: String,
    origin: BuildLogOrigin,
}

#[derive(Debug, Serialize)]
struct AppendEventsRequest<'a> {
    events: &'a [BuildLogEvent],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppendEventsResponse {
    next_seq: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FinishRequest {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_phase: Option<BuildLogPhase>,
}

struct EmitterState {
    next_seq: u32,
    accepted_bytes: usize,
    sender: Option<mpsc::Sender<BuildLogEvent>>,
}

#[derive(Clone)]
pub(super) struct ExactValueRedactor {
    pattern: Option<Regex>,
}

const NON_SENSITIVE_EXACT_VALUES: &[&str] =
    &["production", "preview", "development", "true", "false"];

impl ExactValueRedactor {
    pub(super) fn from_materialized_values(configured_values: &[String]) -> anyhow::Result<Self> {
        let mut values = configured_values.to_vec();
        values.extend(
            PLATFORM_SECRET_ENV_KEYS
                .iter()
                .filter_map(|key| match std::env::var(key) {
                    Ok(value) => Some(Ok(value)),
                    Err(std::env::VarError::NotPresent) => None,
                    Err(error @ std::env::VarError::NotUnicode(_)) => Some(Err(error)),
                })
                .collect::<Result<Vec<_>, _>>()
                .context("platform build-log secret is not valid UTF-8")?,
        );

        Self::from_values(values)
    }

    pub(super) fn from_values(values: impl IntoIterator<Item = String>) -> anyhow::Result<Self> {
        let mut unique = HashSet::new();
        let mut values = values
            .into_iter()
            .flat_map(|value| {
                let mut parts = value
                    .lines()
                    .filter(|part| !part.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                if !value.is_empty() && !parts.iter().any(|part| part == &value) {
                    parts.push(value);
                }
                parts
            })
            .filter(|value| !value.is_empty())
            .filter(|value| {
                !NON_SENSITIVE_EXACT_VALUES
                    .iter()
                    .any(|allowed| value.eq_ignore_ascii_case(allowed))
            })
            .filter(|value| unique.insert(value.clone()))
            .collect::<Vec<_>>();
        values.sort_unstable_by_key(|value| std::cmp::Reverse(value.len()));

        let pattern = if values.is_empty() {
            None
        } else {
            let pattern = values
                .iter()
                .map(|value| regex::escape(value))
                .collect::<Vec<_>>()
                .join("|");
            Some(Regex::new(&pattern).context("failed to compile build-log environment redactor")?)
        };

        Ok(Self { pattern })
    }

    fn redact(&self, value: &str) -> String {
        self.pattern.as_ref().map_or_else(
            || value.to_string(),
            |pattern| pattern.replace_all(value, REDACTED).into_owned(),
        )
    }

    pub(super) fn sanitize_json(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::String(value) => {
                serde_json::Value::String(sanitize_message(value, self))
            }
            serde_json::Value::Array(values) => serde_json::Value::Array(
                values
                    .iter()
                    .map(|value| self.sanitize_json(value))
                    .collect(),
            ),
            serde_json::Value::Object(values) => serde_json::Value::Object(
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), self.sanitize_json(value)))
                    .collect(),
            ),
            _ => value.clone(),
        }
    }
}

#[derive(Clone)]
pub(super) struct BuildLogEmitter {
    state: Arc<Mutex<EmitterState>>,
    phase: Arc<Mutex<BuildLogPhase>>,
    dropped: Arc<std::sync::atomic::AtomicU64>,
    redactor: Arc<ExactValueRedactor>,
    include_debug: bool,
    lifecycle_origin: BuildLogOrigin,
}

impl BuildLogEmitter {
    pub(super) fn emit(
        &self,
        stream: BuildLogStream,
        level: BuildLogLevel,
        phase: BuildLogPhase,
        origin: BuildLogOrigin,
        message: &str,
    ) {
        if stream == BuildLogStream::Debug && !self.include_debug {
            return;
        }
        let message = sanitize_message(message, &self.redactor);
        if message.is_empty() {
            return;
        }
        *self
            .phase
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = phase;
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.next_seq >= MAX_SESSION_EVENTS {
            self.record_drop();
            return;
        }
        let Some(sender) = state.sender.as_ref() else {
            return;
        };
        let event = BuildLogEvent {
            seq: state.next_seq,
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            stream,
            level,
            phase,
            message,
            origin,
        };
        let event_bytes = serde_json::to_vec(&event).map_or(MAX_MESSAGE_BYTES, |value| value.len());
        if state.accepted_bytes.saturating_add(event_bytes) > MAX_SESSION_BYTES {
            self.record_drop();
            return;
        }
        match sender.try_send(event) {
            Ok(()) => {
                state.next_seq += 1;
                state.accepted_bytes += event_bytes;
            }
            Err(_) => self.record_drop(),
        }
    }

    pub(super) fn info(&self, phase: BuildLogPhase, message: &str) {
        self.emit(
            BuildLogStream::User,
            BuildLogLevel::Info,
            phase,
            self.lifecycle_origin,
            message,
        );
    }

    pub(super) fn debug(&self, phase: BuildLogPhase, message: &str) {
        self.emit(
            BuildLogStream::Debug,
            BuildLogLevel::Debug,
            phase,
            self.lifecycle_origin,
            message,
        );
    }

    pub(super) fn error(&self, phase: BuildLogPhase, message: &str) {
        self.emit(
            BuildLogStream::User,
            BuildLogLevel::Error,
            phase,
            self.lifecycle_origin,
            message,
        );
    }

    fn record_drop(&self) {
        self.dropped
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn close(&self) {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .sender
            .take();
    }
}

struct UploadOutcome {
    degraded_reason: Option<String>,
}

pub(super) struct BuildLogSession {
    id: String,
    client: ApiClient,
    emitter: Option<BuildLogEmitter>,
    uploader: Option<tokio::task::JoinHandle<UploadOutcome>>,
    phase: Arc<Mutex<BuildLogPhase>>,
    dropped: Arc<std::sync::atomic::AtomicU64>,
    redactor: Arc<ExactValueRedactor>,
    json: bool,
}

pub(super) struct StartBuildLogSession<'a> {
    pub client: &'a ApiClient,
    pub project_id: &'a str,
    pub deployment_id: &'a str,
    pub workspace_id: &'a str,
    pub project_dir: &'a Path,
    pub redactor: ExactValueRedactor,
    pub config: BuildLogUploadConfig,
    pub json: bool,
}

impl BuildLogSession {
    pub(super) async fn start(request: StartBuildLogSession<'_>) -> Option<Self> {
        let StartBuildLogSession {
            client,
            project_id,
            deployment_id,
            workspace_id,
            project_dir,
            redactor,
            config,
            json,
        } = request;
        if !config.upload_enabled {
            return None;
        }
        if config.source == BuildLogSource::LocalCli {
            let policy = tokio::time::timeout(
                CREATE_TIMEOUT,
                client.get::<WorkspacePolicyResponse>("/v1/build-log-sessions/workspace-policy"),
            )
            .await;
            match policy {
                Ok(Ok(policy)) if policy.enabled => {}
                Ok(Ok(_)) => return None,
                Ok(Err(error)) => {
                    output::warn(
                        json,
                        format!(
                            "Build-log upload disabled: workspace policy is unavailable ({error})"
                        ),
                        output::Phase::Deploy,
                    );
                    return None;
                }
                Err(_) => {
                    output::warn(
                        json,
                        "Build-log upload disabled: workspace policy request timed out",
                        output::Phase::Deploy,
                    );
                    return None;
                }
            }
        }

        let redactor = Arc::new(redactor);
        let id = uuid_from_env("NRZ_BUILD_LOG_SESSION_ID").unwrap_or_else(Uuid::now_v7);
        let producer_id = uuid_from_env("NRZ_BUILD_LOG_PRODUCER_ID").unwrap_or(id);
        let id = id.to_string();
        let producer_id = producer_id.to_string();
        let builder_version = std::env::var("NRZ_BUILDER_VERSION").ok();
        let request = CreateSessionRequest {
            id: &id,
            project_id,
            deployment_id,
            attempt: config.attempt,
            producer_id: &producer_id,
            source: config.source,
            cli_version: env!("CARGO_PKG_VERSION"),
            builder_version: builder_version.as_deref(),
        };
        let response = tokio::time::timeout(
            CREATE_TIMEOUT,
            client.post::<_, CreateSessionResponse>("/v1/build-log-sessions", &request),
        )
        .await;
        let response = match response {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                output::warn(
                    json,
                    format!("Build-log upload unavailable: {error}"),
                    output::Phase::Deploy,
                );
                return None;
            }
            Err(_) => {
                output::warn(
                    json,
                    "Build-log upload unavailable: session creation timed out",
                    output::Phase::Deploy,
                );
                return None;
            }
        };

        let phase = Arc::new(Mutex::new(BuildLogPhase::Init));
        let dropped = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let shipping_enabled = response.session.shipping_policy == "ENABLED";
        if !shipping_enabled {
            return None;
        }
        let (emitter, uploader) = if shipping_enabled {
            let (sender, receiver) = mpsc::channel(UPLOAD_CHANNEL_CAPACITY);
            let emitter = BuildLogEmitter {
                state: Arc::new(Mutex::new(EmitterState {
                    next_seq: response.session.next_seq,
                    accepted_bytes: response.session.accepted_bytes,
                    sender: Some(sender),
                })),
                phase: Arc::clone(&phase),
                dropped: Arc::clone(&dropped),
                redactor: Arc::clone(&redactor),
                include_debug: config.include_debug,
                lifecycle_origin: match config.source {
                    BuildLogSource::LocalCli => BuildLogOrigin::Cli,
                    BuildLogSource::RemoteBuilder => BuildLogOrigin::Builder,
                },
            };
            let uploader = tokio::spawn(upload_events(
                client.clone(),
                response.session.id.clone(),
                receiver,
            ));
            emit_upload_notice(project_dir, workspace_id, project_id, config.source, json);
            emitter.debug(BuildLogPhase::Init, "Build log session started");
            (Some(emitter), Some(uploader))
        } else {
            (None, None)
        };

        Some(Self {
            id: response.session.id,
            client: client.clone(),
            emitter,
            uploader,
            phase,
            dropped,
            redactor,
            json,
        })
    }

    pub(super) fn emitter(&self) -> Option<&BuildLogEmitter> {
        self.emitter.as_ref()
    }

    pub(super) async fn finish(&mut self, result: &anyhow::Result<()>) {
        if let Some(emitter) = &self.emitter {
            match result {
                Ok(()) => emitter.info(BuildLogPhase::Complete, "Build artifacts uploaded"),
                Err(error) => emitter.error(BuildLogPhase::Error, &error.to_string()),
            }
            emitter.close();
        }

        let mut degraded_reason = None;
        if let Some(mut uploader) = self.uploader.take() {
            match tokio::time::timeout(UPLOAD_FINISH_TIMEOUT, &mut uploader).await {
                Ok(Ok(outcome)) => degraded_reason = outcome.degraded_reason,
                Ok(Err(error)) => degraded_reason = Some(format!("uploader task failed: {error}")),
                Err(_) => {
                    uploader.abort();
                    degraded_reason = Some("uploader flush timed out".to_string());
                }
            }
        }
        let dropped = self.dropped.load(std::sync::atomic::Ordering::Relaxed);
        if dropped > 0 {
            degraded_reason = Some(format!(
                "{dropped} event(s) dropped by the bounded upload buffer or session limit"
            ));
        }
        if let Some(reason) = degraded_reason {
            output::warn(
                self.json,
                format!("Build-log upload degraded: {reason}"),
                output::Phase::Deploy,
            );
        }

        let error = result.as_ref().err();
        let request = FinishRequest {
            status: if result.is_ok() { "FINISHED" } else { "FAILED" },
            message: error.map(|error| {
                let message = output::reported_terminal_diagnostic(error).map_or_else(
                    || error.to_string(),
                    |diagnostic| diagnostic.message.clone(),
                );
                sanitize_message(&message, &self.redactor)
            }),
            error_code: error.and_then(error_code),
            error_details: error
                .and_then(error_details)
                .map(|details| self.redactor.sanitize_json(&details)),
            failure_phase: error.map(|_| {
                *self
                    .phase
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
            }),
        };
        let path = format!("/v1/build-log-sessions/{}/finish", self.id);
        if finish_session(&self.client, &path, &request).await.is_err() {
            output::warn(
                self.json,
                "Could not finalize build-log session",
                output::Phase::Deploy,
            );
        }
    }
}

async fn finish_session(
    client: &ApiClient,
    path: &str,
    request: &FinishRequest,
) -> anyhow::Result<()> {
    let started = Instant::now();
    let mut delay = UPLOAD_RETRY_INITIAL_DELAY;
    loop {
        let result = tokio::time::timeout(
            MUTATION_TIMEOUT,
            client.post::<_, serde_json::Value>(path, request),
        )
        .await;
        match result {
            Ok(Ok(_)) => return Ok(()),
            Ok(Err(error)) => {
                let Some(retry) = classify_api_retry(&error) else {
                    return Err(error);
                };
                let remaining = MUTATION_RETRY_BUDGET.saturating_sub(started.elapsed());
                if remaining.is_zero() {
                    return Err(error);
                }
                tokio::time::sleep(retry.retry_after.unwrap_or(delay).min(remaining)).await;
                delay = (delay * 2).min(UPLOAD_RETRY_MAX_DELAY);
            }
            Err(error) => {
                let remaining = MUTATION_RETRY_BUDGET.saturating_sub(started.elapsed());
                if remaining.is_zero() {
                    return Err(error.into());
                }
                tokio::time::sleep(delay.min(remaining)).await;
                delay = (delay * 2).min(UPLOAD_RETRY_MAX_DELAY);
            }
        }
    }
}

async fn upload_events(
    client: ApiClient,
    session_id: String,
    mut receiver: mpsc::Receiver<BuildLogEvent>,
) -> UploadOutcome {
    loop {
        let (batch, closed) =
            collect_upload_batch(&mut receiver, MAX_EVENTS_PER_BATCH, FLUSH_INTERVAL).await;
        if !batch.is_empty()
            && let Err(error) = upload_batch(&client, &session_id, &batch).await
        {
            return UploadOutcome {
                degraded_reason: Some(error.to_string()),
            };
        }
        if closed {
            return UploadOutcome {
                degraded_reason: None,
            };
        }
    }
}

pub(super) async fn collect_upload_batch<T>(
    receiver: &mut mpsc::Receiver<T>,
    max_events: usize,
    flush_interval: Duration,
) -> (Vec<T>, bool) {
    let Some(first) = receiver.recv().await else {
        return (Vec::new(), true);
    };
    let mut batch = Vec::with_capacity(max_events);
    batch.push(first);
    let deadline = tokio::time::Instant::now() + flush_interval;

    while batch.len() < max_events {
        match tokio::time::timeout_at(deadline, receiver.recv()).await {
            Ok(Some(event)) => batch.push(event),
            Ok(None) => return (batch, true),
            Err(_) => break,
        }
    }
    (batch, false)
}

async fn upload_batch(
    client: &ApiClient,
    session_id: &str,
    events: &[BuildLogEvent],
) -> anyhow::Result<()> {
    let started = Instant::now();
    let mut delay = UPLOAD_RETRY_INITIAL_DELAY;
    let expected_next_seq = events.last().map_or(0, |event| event.seq + 1);
    let path = format!("/v1/build-log-sessions/{session_id}/events");
    loop {
        let response = tokio::time::timeout(
            UPLOAD_REQUEST_TIMEOUT,
            client.post::<_, AppendEventsResponse>(&path, &AppendEventsRequest { events }),
        )
        .await;
        match response {
            Ok(Ok(response)) if response.next_seq == expected_next_seq => return Ok(()),
            Ok(Ok(response)) => anyhow::bail!(
                "server acknowledged unexpected build-log cursor {} instead of {}",
                response.next_seq,
                expected_next_seq
            ),
            Ok(Err(error)) if classify_api_retry(&error).is_none() => return Err(error),
            Ok(Err(error)) if started.elapsed() >= UPLOAD_RETRY_BUDGET => return Err(error),
            Ok(Err(error)) if started.elapsed() < UPLOAD_RETRY_BUDGET => {
                let retry =
                    classify_api_retry(&error).expect("retryable error was classified above");
                let remaining = UPLOAD_RETRY_BUDGET.saturating_sub(started.elapsed());
                tokio::time::sleep(retry.retry_after.unwrap_or(delay).min(remaining)).await;
                delay = (delay * 2).min(UPLOAD_RETRY_MAX_DELAY);
            }
            Err(_) if started.elapsed() < UPLOAD_RETRY_BUDGET => {
                let remaining = UPLOAD_RETRY_BUDGET.saturating_sub(started.elapsed());
                tokio::time::sleep(delay.min(remaining)).await;
                delay = (delay * 2).min(UPLOAD_RETRY_MAX_DELAY);
            }
            Err(_) => anyhow::bail!("build-log batch upload timed out"),
            Ok(Err(error)) => return Err(error),
        }
    }
}

pub(super) fn error_code(error: &anyhow::Error) -> Option<String> {
    if let Some(diagnostic) = output::reported_terminal_diagnostic(error) {
        return Some(diagnostic.code.clone());
    }
    if let Some(error) = crate::errors::find_cli_error(error) {
        return Some(error.code.clone());
    }
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<output::CodedError>())
        .map(|error| error.code.clone())
}

pub(super) fn error_details(error: &anyhow::Error) -> Option<serde_json::Value> {
    if let Some(diagnostic) = output::reported_terminal_diagnostic(error) {
        return diagnostic.details.clone();
    }
    crate::errors::find_cli_error(error).and_then(|error| error.details.clone())
}

fn uuid_from_env(name: &str) -> Option<Uuid> {
    std::env::var(name)
        .ok()
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
}

fn emit_upload_notice(
    project_dir: &Path,
    workspace_id: &str,
    project_id: &str,
    source: BuildLogSource,
    json: bool,
) {
    if source != BuildLogSource::LocalCli {
        return;
    }
    let state_dir = project_dir.join(".onreza");
    let state_file = state_dir.join(NOTICE_FILE);
    let key = format!("{workspace_id}:{project_id}");
    if std::fs::read_to_string(&state_file)
        .is_ok_and(|contents| contents.lines().any(|line| line == key))
    {
        return;
    }
    output::status(
        json,
        "~",
        "Build logs are uploaded to ONREZA. Values classified as sensitive by ONREZA are masked only in the uploaded copy; local output stays unchanged. Disable with --no-log-upload or NRZ_LOG_UPLOAD=0.",
        output::Phase::Deploy,
    );
    if std::fs::create_dir_all(&state_dir).is_err() {
        return;
    }
    crate::init::add_to_gitignore(project_dir);
    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(state_file)
    {
        Ok(file) => file,
        Err(_) => return,
    };
    let _ = writeln!(file, "{key}");
}

pub(super) fn sanitize_message(message: &str, redactor: &ExactValueRedactor) -> String {
    static BEARER: OnceLock<Regex> = OnceLock::new();
    static ASSIGNMENT: OnceLock<Regex> = OnceLock::new();
    static URL: OnceLock<Regex> = OnceLock::new();
    let bearer = BEARER.get_or_init(|| Regex::new(r#"(?i)\bBearer\s+[^\s"']+"#).unwrap());
    let assignment = ASSIGNMENT.get_or_init(|| {
        Regex::new(r#"(?i)\b([a-z0-9_-]*(?:authorization|cookie|password|passwd|secret|token|api[_-]?key|access[_-]?key|private[_-]?key)[a-z0-9_-]*)\s*[:=]\s*(?:"[^"]*"|'[^']*'|[^\s,;]+)"#).unwrap()
    });
    let url = URL.get_or_init(|| Regex::new(r#"https?://[^\s\"'<>]+"#).unwrap());

    let value = output::terminal_text(message);
    let value = redactor.redact(&value);
    let value = bearer.replace_all(&value, "Bearer [REDACTED]");
    let value = assignment.replace_all(&value, "$1=[REDACTED]");
    let value = url.replace_all(&value, |captures: &Captures<'_>| sanitize_url(&captures[0]));
    truncate_utf8(value.trim().replace(['\r', '\n'], " "), MAX_MESSAGE_BYTES)
}

fn sanitize_url(raw: &str) -> String {
    let Ok(mut url) = url::Url::parse(raw) else {
        return raw.to_string();
    };
    if !url.username().is_empty() || url.password().is_some() {
        let _ = url.set_username("REDACTED");
        let _ = url.set_password(None);
    }
    if url.query().is_some() {
        url.set_query(None);
    }
    url.to_string()
}

pub(super) fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = max_bytes.saturating_sub(TRUNCATION_MARKER.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value.push_str(TRUNCATION_MARKER);
    value
}
