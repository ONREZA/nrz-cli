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
    Workspace,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Auth => "auth",
            Phase::Build => "build",
            Phase::Db => "db",
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

/// Emit one structured JSON log line to stdout (JSON mode only).
///
/// Format: `{"s":"user|debug","p":"phase","l":"info|warn","m":"message"}`
///
/// - `s`: stream — "user" (user-visible) or "debug" (internal noise)
/// - `p`: phase — value from `Phase::as_str()`
/// - `l`: level — "info" or "warn"
/// - `m`: message text
pub fn log_line(stream: &str, level: &str, phase: &str, msg: &str) {
    println!(
        "{}",
        serde_json::json!({"s": stream, "p": phase, "l": level, "m": msg}),
    );
}

/// Print status message to stderr, or structured JSON log in JSON mode.
pub fn status(json: bool, icon: &str, msg: impl std::fmt::Display, phase: Phase) {
    if json {
        log_line("user", "info", phase.as_str(), &msg.to_string());
    } else {
        eprintln!("  {} {msg}", console::style(icon).cyan().bold());
    }
}

/// Print success message to stderr, or structured JSON log in JSON mode.
pub fn success(json: bool, msg: impl std::fmt::Display, phase: Phase) {
    if json {
        log_line("user", "info", phase.as_str(), &msg.to_string());
    } else {
        eprintln!("  {} {msg}", console::style("✓").green().bold());
    }
}

/// Print warning message to stderr, or structured JSON log in JSON mode.
pub fn warn(json: bool, msg: impl std::fmt::Display, phase: Phase) {
    if json {
        log_line("user", "warn", phase.as_str(), &msg.to_string());
    } else {
        eprintln!("  {} {msg}", console::style("!").yellow().bold());
    }
}

/// Emit a structured error line with error code and optional limit details.
///
/// Format: `{"s":"error","p":"phase","l":"error","m":"message","code":"...","details":{...}}`
///
/// Used by Builder to extract structured error info (e.g., LIMIT_EXCEEDED with limitType)
/// and persist it to the deployment record for frontend upsell dialogs.
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
        "m": message,
        "code": code,
    });
    if let Some(d) = details {
        obj["details"] = d.clone();
    }
    println!("{obj}");
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

#[derive(Debug)]
pub struct AlreadyReportedError;

impl std::fmt::Display for AlreadyReportedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("command output already reported")
    }
}

impl std::error::Error for AlreadyReportedError {}

pub fn already_reported_error() -> anyhow::Error {
    anyhow::Error::new(AlreadyReportedError)
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
