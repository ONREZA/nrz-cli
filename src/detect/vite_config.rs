//! Parse vite.config.{ts,mts,js,mjs} for outDir.
//! Uses string parsing — no regex dependency.

use std::path::Path;

const VITE_CONFIG_FILES: &[&str] = &[
    "vite.config.ts",
    "vite.config.mts",
    "vite.config.js",
    "vite.config.mjs",
];

/// Try to extract `outDir` from vite.config.* files.
/// Returns the outDir value if found (without quotes).
pub fn parse_vite_out_dir(project_dir: &Path) -> Option<String> {
    for file in VITE_CONFIG_FILES {
        let path = project_dir.join(file);
        if let Ok(content) = std::fs::read_to_string(&path)
            && let Some(out_dir) = extract_out_dir(&content)
        {
            return Some(out_dir);
        }
    }
    None
}

/// Check if any vite config file exists.
#[allow(dead_code)]
pub fn has_vite_config(project_dir: &Path) -> bool {
    VITE_CONFIG_FILES
        .iter()
        .any(|f| project_dir.join(f).exists())
}

/// Extract outDir value from config content.
/// Looks for patterns like `outDir: "dist"` or `outDir: 'build'` anywhere in a line.
fn extract_out_dir(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(idx) = line.find("outDir") {
            let after = &line[idx + "outDir".len()..];
            let after = after.trim_start();
            if let Some(after) = after.strip_prefix(':') {
                let after = after.trim();
                if let Some(val) = extract_quoted_string(after) {
                    return Some(val);
                }
            }
        }
    }
    None
}

/// Extract the first quoted string (single or double quotes).
fn extract_quoted_string(s: &str) -> Option<String> {
    let quote_char = if s.starts_with('"') {
        '"'
    } else if s.starts_with('\'') {
        '\''
    } else {
        return None;
    };

    let inner = &s[1..];
    if let Some(end) = inner.find(quote_char) {
        let val = &inner[..end];
        if !val.is_empty() {
            return Some(val.to_string());
        }
    }
    None
}
