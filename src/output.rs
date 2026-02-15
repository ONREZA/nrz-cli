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
