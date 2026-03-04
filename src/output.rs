use std::io::{BufRead, Write};

use serde::Serialize;

/// CLI operation phase for structured JSON logging.
///
/// Used in `--json` mode to tag each log line with the active operation,
/// allowing LLM consumers to filter and route output by phase.
#[derive(Clone, Copy, Debug)]
pub enum Phase {
    Auth,
    Build,
    Db,
    Deploy,
    Detect,
    Domains,
    Env,
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

/// Print error as JSON to stdout.
pub fn json_error(err: &anyhow::Error) {
    let msg = format!("{err:#}");
    match serde_json::to_string(&serde_json::json!({ "error": msg })) {
        Ok(s) => println!("{s}"),
        Err(e) => {
            eprintln!("internal error: failed to serialize error: {e}");
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
