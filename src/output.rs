use std::io::{BufRead, Write};

use serde::Serialize;

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

/// Print status message to stderr (suppressed in JSON mode).
pub fn status(json: bool, icon: &str, msg: impl std::fmt::Display) {
    if !json {
        eprintln!("  {} {msg}", console::style(icon).cyan().bold());
    }
}

/// Print success message to stderr (suppressed in JSON mode).
pub fn success(json: bool, msg: impl std::fmt::Display) {
    if !json {
        eprintln!("  {} {msg}", console::style("✓").green().bold());
    }
}

/// Print warning message to stderr (suppressed in JSON mode).
pub fn warn(json: bool, msg: impl std::fmt::Display) {
    if !json {
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
