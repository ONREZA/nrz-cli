use std::io::{BufRead, Write};

use serde::Serialize;

/// CLI operation phase for structured JSON logging.
///
/// Used in `--json` mode to tag each log line with the active operation,
/// allowing LLM consumers to filter and route output by phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Auth,
    Build,
    Db,
    Dev,
    Deploy,
    Detect,
    Domains,
    Env,
    Functions,
    Init,
    Install,
    Link,
    Projects,
    Rollback,
    Rules,
    Workspace,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Auth => "auth",
            Phase::Build => "build",
            Phase::Db => "db",
            Phase::Dev => "dev",
            Phase::Deploy => "deploy",
            Phase::Detect => "detect",
            Phase::Domains => "domains",
            Phase::Env => "env",
            Phase::Functions => "functions",
            Phase::Init => "init",
            Phase::Install => "install",
            Phase::Link => "link",
            Phase::Projects => "projects",
            Phase::Rollback => "rollback",
            Phase::Rules => "rules",
            Phase::Workspace => "workspace",
        }
    }
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Print structured data as JSON to stdout.
pub fn json_output<T: Serialize>(data: &T) {
    match serde_json::to_string(data) {
        Ok(s) => println!("{s}"),
        Err(e) => {
            eprintln!("internal error: failed to serialize output: {e}");
            std::process::exit(1);
        }
    }
}

/// Emit the terminal outcome envelope for a *failed* command to **stdout** (JSON
/// mode). This is the machine-readable result automation/LLMs read from stdout —
/// the counterpart of [`json_output`] for the success case. It is deliberately
/// separate from the stderr structured-log frame channel ([`log_error_structured`])
/// consumed by JSON log tooling: stdout = terminal result, stderr = log stream.
pub fn terminal_error(message: &str, code: Option<&str>) {
    let mut obj = serde_json::json!({ "error": cap_message(message) });
    if let Some(code) = code {
        obj["code"] = serde_json::Value::String(code.to_owned());
    }
    json_output(&obj);
}

/// Sentinel byte prefixing every structured protocol frame.
///
/// ASCII Record Separator (`0x1E`) never appears in normal build-tool output or
/// JSON text, so a consumer can decide "is this a protocol frame?" with one byte
/// check — instead of trying to JSON-parse every `{`-prefixed line and treating
/// failures as errors. Lines that arrive byte-level-torn (spliced by the log
/// pipeline) lose sentinel alignment and are unambiguously *not* frames.
pub const FRAME_SENTINEL: char = '\u{1e}';

/// Upper bound on a message's **JSON-serialized** length before framing.
///
/// Build tools emit very long single lines (minified bundles, base64 blobs,
/// dependency dumps). Container log pipelines (containerd/CRI, Docker) split a
/// log line at ~16 KiB into partial fragments — a frame larger than that would
/// be torn by the runtime, leaving the consumer a sentinel-prefixed half-frame.
/// We bound the *escaped* size (not raw bytes) because control chars / ANSI
/// sequences common in build output expand up to 6× when JSON-escaped; the
/// remaining headroom under 16 KiB covers the small envelope keys/code/details.
const MAX_SERIALIZED_MESSAGE_BYTES: usize = 12 * 1024;

const TRUNCATION_MARKER: &str = "…[truncated]";

/// Byte length of `s` once JSON-escaped by `serde_json` (excluding the quotes).
/// Mirrors serde_json's escaping exactly so [`cap_message`] can bound the real
/// serialized size without allocating: `"`, `\`, `\n`, `\r`, `\t` → 2 bytes;
/// other control bytes → `\u00XX` (6); everything else (incl. UTF-8) → as-is.
fn json_escaped_len(s: &str) -> usize {
    s.bytes()
        .map(|b| match b {
            b'"' | b'\\' | b'\n' | b'\r' | b'\t' => 2,
            0x00..=0x1f => 6,
            _ => 1,
        })
        .sum()
}

/// Truncate `msg` on a char boundary so its JSON-escaped form (plus a marker)
/// stays within [`MAX_SERIALIZED_MESSAGE_BYTES`]. Borrows when no cut is needed.
pub(crate) fn cap_message(msg: &str) -> std::borrow::Cow<'_, str> {
    if json_escaped_len(msg) <= MAX_SERIALIZED_MESSAGE_BYTES {
        return std::borrow::Cow::Borrowed(msg);
    }
    let budget = MAX_SERIALIZED_MESSAGE_BYTES - json_escaped_len(TRUNCATION_MARKER);
    let mut end = msg.len();
    loop {
        // Shrink ~25% per step (rare path — only oversized lines reach here).
        end = end.saturating_sub(end / 4 + 1);
        while end > 0 && !msg.is_char_boundary(end) {
            end -= 1;
        }
        if end == 0 || json_escaped_len(&msg[..end]) <= budget {
            return std::borrow::Cow::Owned(format!("{}{}", &msg[..end], TRUNCATION_MARKER));
        }
    }
}

pub(crate) fn terminal_text(value: &str) -> String {
    static ANSI: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let ansi = ANSI.get_or_init(|| {
        regex::Regex::new(r"\x1B(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))")
            .expect("terminal escape regex must compile")
    });
    ansi.replace_all(value, "")
        .chars()
        .filter_map(|character| match character {
            '\r' => Some(' '),
            '\n' | '\t' => Some(character),
            _ if character.is_control() || matches!(character as u32, 0x7f..=0x9f) => None,
            _ => Some(character),
        })
        .collect()
}

pub(crate) fn terminal_line(value: &str) -> String {
    terminal_text(value)
        .chars()
        .map(|character| match character {
            '\n' | '\t' => ' ',
            _ => character,
        })
        .collect()
}

/// Render a frame to its exact wire bytes: `<sentinel><json>\n`. Pure — the IO
/// side ([`emit_frame`]) writes the result in a single `write_all`.
pub(crate) fn render_frame(obj: &serde_json::Value) -> String {
    let mut line = String::with_capacity(128);
    line.push(FRAME_SENTINEL);
    line.push_str(&obj.to_string());
    line.push('\n');
    line
}

/// Serialize a structured frame and write it to stderr as a single atomic
/// `write_all`: `<sentinel><json>\n`.
///
/// Two invariants this guarantees and the old `eprintln!("{}", value)` did not:
/// 1. **Whole-frame writes.** `Display` for a `serde_json::Value` writes
///    token-by-token; the container log pipeline can read a partial line and
///    splice it with another stream's bytes, producing torn JSON. One write of
///    a size-capped frame ([`cap_message`]) keeps each frame intact in transit.
/// 2. **One channel.** Every frame goes to stderr — never stdout — so frames
///    from different call sites can't interleave across two merged fds.
fn emit_frame(obj: &serde_json::Value) {
    // A broken stderr pipe must not abort an otherwise-successful build.
    let _ = std::io::stderr()
        .lock()
        .write_all(render_frame(obj).as_bytes());
}

/// Emit one structured JSON progress log line to stderr (JSON mode only).
///
/// Wire format: `<FRAME_SENTINEL>{"s":..,"p":..,"l":..,"m":..}\n`
///
/// - `s`: stream — "user" (user-visible) or "debug" (internal noise)
/// - `p`: phase — value from `Phase::as_str()`
/// - `l`: level — "info" or "warn"
/// - `m`: message text (size-capped via [`cap_message`])
pub fn log_line(stream: &str, level: &str, phase: &str, msg: &str) {
    emit_frame(&serde_json::json!({"s": stream, "p": phase, "l": level, "m": cap_message(msg)}));
}

pub(crate) fn info_structured_frame(
    phase: &str,
    message: &str,
    details: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "s": "user",
        "p": phase,
        "l": "info",
        "m": cap_message(message),
        "details": details,
    })
}

/// Emit a user-visible progress frame with machine-readable details.
pub fn log_info_structured(phase: &str, message: &str, details: &serde_json::Value) {
    emit_frame(&info_structured_frame(phase, message, details));
}

/// Print status message to stderr, or structured JSON log in JSON mode.
pub fn status(json: bool, icon: &str, msg: impl std::fmt::Display, phase: Phase) {
    let msg = msg.to_string();
    if json {
        log_line("user", "info", phase.as_str(), &msg);
    } else {
        eprintln!(
            "  {} {}",
            console::style(icon).cyan().bold(),
            terminal_text(&msg)
        );
    }
}

/// Print success message to stderr, or structured JSON log in JSON mode.
pub fn success(json: bool, msg: impl std::fmt::Display, phase: Phase) {
    let msg = msg.to_string();
    if json {
        log_line("user", "info", phase.as_str(), &msg);
    } else {
        eprintln!(
            "  {} {}",
            console::style("✓").green().bold(),
            terminal_text(&msg)
        );
    }
}

/// Print warning message to stderr, or structured JSON log in JSON mode.
pub fn warn(json: bool, msg: impl std::fmt::Display, phase: Phase) {
    let msg = msg.to_string();
    if json {
        log_line("user", "warn", phase.as_str(), &msg);
    } else {
        eprintln!(
            "  {} {}",
            console::style("!").yellow().bold(),
            terminal_text(&msg)
        );
    }
}

/// Emit a structured error line with error code and optional limit details.
///
/// Wire format: `<FRAME_SENTINEL>{"s":"error","p":..,"l":"error","m":..,"code":..,"details":{..}}\n`
///
/// Used by JSON log consumers to preserve structured error info (for example,
/// LIMIT_EXCEEDED with limitType). Shares the single stderr frame channel with
/// [`log_line`].
pub fn log_error_structured(
    phase: &str,
    message: &str,
    code: &str,
    details: Option<&serde_json::Value>,
) {
    let mut obj = serde_json::json!({
        "s": "error",
        "p": phase,
        "l": "error",
        "m": cap_message(message),
        "code": code,
    });
    if let Some(d) = details {
        obj["details"] = d.clone();
    }
    emit_frame(&obj);
}

/// Typed error carrying a machine-readable code for Builder classification.
///
/// Builder treats deploy failures with a non-empty `code` as user-fault (info
/// severity), and failures without one as platform-fault (error + Sentry).
/// Wrap known user-facing failures in `CodedError` so main.rs can emit the
/// structured error line with `code` set.
///
/// `source` preserves the underlying error chain when we attach a code to an
/// already-raised `anyhow::Error` — downstream tooling (Sentry, tracing) can
/// still see original `io::Error` kinds and nested contexts.
#[derive(Debug)]
pub struct CodedError {
    pub code: String,
    pub message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl CodedError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        let code = code.into();
        debug_assert!(
            !code.is_empty(),
            "CodedError code must be non-empty — builder reads empty code as platform-fault and routes to Sentry"
        );
        Self {
            code,
            message: message.into(),
            source: None,
        }
    }

    /// Like [`Self::new`] but preserves the original error as the cause,
    /// keeping `std::error::Error::source()` intact for downstream consumers.
    pub fn with_source(
        code: impl Into<String>,
        message: impl Into<String>,
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    ) -> Self {
        let code = code.into();
        debug_assert!(!code.is_empty());
        Self {
            code,
            message: message.into(),
            source: Some(source),
        }
    }
}

impl std::fmt::Display for CodedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CodedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// Build an `anyhow::Error` carrying a `CodedError` in its chain.
pub fn coded_error(code: impl Into<String>, message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(CodedError::new(code, message))
}

#[derive(Debug, Clone)]
pub struct TerminalDiagnostic {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct AlreadyReportedError {
    diagnostic: Option<TerminalDiagnostic>,
}

impl AlreadyReportedError {
    pub fn diagnostic(&self) -> Option<&TerminalDiagnostic> {
        self.diagnostic.as_ref()
    }
}

impl std::fmt::Display for AlreadyReportedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.diagnostic {
            Some(diagnostic) => f.write_str(&diagnostic.message),
            None => f.write_str("command output already reported"),
        }
    }
}

impl std::error::Error for AlreadyReportedError {}

pub fn reported_terminal_diagnostic(error: &anyhow::Error) -> Option<&TerminalDiagnostic> {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<AlreadyReportedError>())
        .and_then(AlreadyReportedError::diagnostic)
}

/// Fully emit a terminal coded error on BOTH channels and return an
/// [`AlreadyReportedError`] so `main`'s terminal handler does not re-emit it:
/// the structured frame on stderr (carries `details`) and the terminal
/// envelope on stdout (CLI/automation). Use at JSON-mode call sites that detect a
/// terminal failure with richer details than a bare [`CodedError`] would carry.
pub fn report_terminal_error(
    phase: &str,
    message: &str,
    code: &str,
    details: Option<&serde_json::Value>,
) -> anyhow::Error {
    log_error_structured(phase, message, code, details);
    terminal_error(message, Some(code));
    anyhow::Error::new(AlreadyReportedError {
        diagnostic: Some(TerminalDiagnostic {
            code: code.to_string(),
            message: message.to_string(),
            details: details.cloned(),
        }),
    })
}

/// Attach a default `CodedError(code)` to an error only if the chain doesn't
/// already carry one. The original error becomes the `source()` of the new
/// `CodedError`, so `err.chain()` still walks through the original context
/// layers for diagnostics (Sentry payloads, `{:#}` formatting, etc.).
///
/// A raw `std::io::Error` anywhere in the chain is treated as platform-fault
/// (permission denied, EIO, disk full, TOCTOU between stat and open) and left
/// uncoded — a miscoded user-fault would silence real infrastructure
/// incidents. User-fault I/O checks must bail explicitly with
/// `coded_error(...)` before any `?` on raw I/O.
pub fn with_default_code(err: anyhow::Error, code: impl Into<String>) -> anyhow::Error {
    if err.chain().any(|c| c.is::<CodedError>()) {
        return err;
    }
    if err.chain().any(|c| c.is::<std::io::Error>()) {
        return err;
    }
    let message = format!("{err:#}");
    let source: Box<dyn std::error::Error + Send + Sync + 'static> = err.into();
    anyhow::Error::new(CodedError::with_source(code, message, source))
}

/// Interactive numeric choice prompt. Returns 1-based selection.
/// Bails on EOF (e.g. piped stdin).
pub fn prompt_choice(label: &str, max: usize) -> anyhow::Result<usize> {
    loop {
        eprint!(
            "  {} ",
            console::style(format!("{label} (1-{max}):")).bold(),
        );
        std::io::stderr().flush()?;

        let mut line = String::new();
        let bytes = std::io::stdin().lock().read_line(&mut line)?;
        if bytes == 0 {
            anyhow::bail!("unexpected end of input");
        }
        let trimmed = line.trim();

        if let Ok(n) = trimmed.parse::<usize>()
            && n >= 1
            && n <= max
        {
            return Ok(n);
        }
        eprintln!("  Invalid choice. Enter a number between 1 and {max}.");
    }
}
