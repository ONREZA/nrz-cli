//! SSR compatibility analysis for Next.js, Nuxt, SvelteKit.

use std::path::Path;

use super::types::SsrAnalysis;

/// Analyze SSR features for a given framework.
/// Returns `None` if the framework is not SSR-capable.
pub fn analyze_ssr(project_dir: &Path, framework: &str) -> Option<SsrAnalysis> {
    match framework {
        "nextjs" => Some(analyze_nextjs(project_dir)),
        "nuxt" => Some(analyze_nuxt(project_dir)),
        "sveltekit" => Some(analyze_sveltekit(project_dir)),
        _ => None,
    }
}

// ── Next.js ────────────────────────────────────────────────────

fn analyze_nextjs(project_dir: &Path) -> SsrAnalysis {
    let mut features = Vec::new();
    let mut is_static_compatible = true;

    // Check next.config for output mode
    let config_content = read_config_file(
        project_dir,
        &[
            "next.config.js",
            "next.config.mjs",
            "next.config.ts",
            "next.config.mts",
        ],
    );

    if let Some(ref content) = config_content {
        if contains_value(content, "output", "standalone") {
            features.push("output: 'standalone'".into());
            is_static_compatible = false;
        }
        if contains_value(content, "output", "export") {
            features.push("output: 'export' (static)".into());
        }
    }

    // Check for middleware
    if file_exists_any(project_dir, &["middleware.ts", "middleware.js"]) {
        features.push("middleware".into());
        is_static_compatible = false;
    }

    // Check for API routes (pages/api/)
    if dir_has_files(project_dir, "pages/api") {
        features.push("pages/api/ routes".into());
        is_static_compatible = false;
    }

    // Check for route handlers (app/**/route.{ts,js})
    if has_route_handlers(project_dir) {
        features.push("app/ route handlers".into());
        is_static_compatible = false;
    }

    // Check for getServerSideProps
    if has_gssp(project_dir) {
        features.push("getServerSideProps".into());
        is_static_compatible = false;
    }

    SsrAnalysis {
        is_static_compatible,
        ssr_features: features,
    }
}

// ── Nuxt ───────────────────────────────────────────────────────

fn analyze_nuxt(project_dir: &Path) -> SsrAnalysis {
    let mut features = Vec::new();
    let mut is_static_compatible = true;

    let config_content = read_config_file(project_dir, &["nuxt.config.ts", "nuxt.config.js"]);

    if let Some(ref content) = config_content {
        // ssr: false means static-only
        if contains_value(content, "ssr", "false") {
            features.push("ssr: false (static)".into());
        }
        // nitro preset: 'static'
        if content.contains("'static'") && content.contains("preset") {
            features.push("preset: 'static'".into());
        }
    }

    // Check for server/api/ directory
    if dir_has_files(project_dir, "server/api") {
        features.push("server/api/ routes".into());
        is_static_compatible = false;
    }

    // Check for server/routes/ directory
    if dir_has_files(project_dir, "server/routes") {
        features.push("server/routes/".into());
        is_static_compatible = false;
    }

    SsrAnalysis {
        is_static_compatible,
        ssr_features: features,
    }
}

// ── SvelteKit ──────────────────────────────────────────────────

fn analyze_sveltekit(project_dir: &Path) -> SsrAnalysis {
    let mut features = Vec::new();
    let mut is_static_compatible = true;

    let config_content = read_config_file(project_dir, &["svelte.config.js"]);

    if let Some(ref content) = config_content
        && content.contains("adapter-static")
    {
        features.push("adapter-static (static)".into());
    }

    // Check for +server.{ts,js} files
    if has_sveltekit_server_routes(project_dir) {
        features.push("+server routes".into());
        is_static_compatible = false;
    }

    // Check for hooks.server.{ts,js}
    if file_exists_any(project_dir, &["src/hooks.server.ts", "src/hooks.server.js"]) {
        features.push("hooks.server".into());
        is_static_compatible = false;
    }

    SsrAnalysis {
        is_static_compatible,
        ssr_features: features,
    }
}

// ── Helpers ────────────────────────────────────────────────────

/// Read the first existing config file from a list.
fn read_config_file(project_dir: &Path, candidates: &[&str]) -> Option<String> {
    for name in candidates {
        let path = project_dir.join(name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            return Some(content);
        }
    }
    None
}

/// Check if any file from the list exists.
fn file_exists_any(project_dir: &Path, files: &[&str]) -> bool {
    files.iter().any(|f| project_dir.join(f).exists())
}

/// Check if a directory has any files (non-recursive, just direct children).
fn dir_has_files(project_dir: &Path, subdir: &str) -> bool {
    let dir = project_dir.join(subdir);
    match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .flatten()
            .any(|e| e.file_type().is_ok_and(|ft| ft.is_file())),
        Err(_) => false,
    }
}

/// Simple string check: does the content contain `key: value` (with or without quotes)
/// anywhere in a non-comment line?
fn contains_value(content: &str, key: &str, value: &str) -> bool {
    for line in content.lines() {
        let trimmed = line.trim_start();
        // Skip comment lines
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with('*') {
            continue;
        }
        if let Some(idx) = line.find(key) {
            let after = &line[idx + key.len()..];
            let after = after.trim_start();
            if let Some(after) = after.strip_prefix(':') {
                let after = after.trim();
                // Match quoted values (exact match within quotes)
                if after.starts_with(&format!("'{value}'"))
                    || after.starts_with(&format!("\"{value}\""))
                {
                    return true;
                }
                // Match unquoted value with word boundary check
                if let Some(rest) = after.strip_prefix(value)
                    && (rest.is_empty()
                        || rest.starts_with(',')
                        || rest.starts_with('}')
                        || rest.starts_with(' ')
                        || rest.starts_with('\t'))
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Check for Next.js route handlers (app/**/route.{ts,js}).
fn has_route_handlers(project_dir: &Path) -> bool {
    let app_dir = project_dir.join("app");
    if !app_dir.is_dir() {
        return false;
    }
    walk_for_file(&app_dir, &["route.ts", "route.js"])
}

/// Check for getServerSideProps in pages/ directory.
fn has_gssp(project_dir: &Path) -> bool {
    let pages_dir = project_dir.join("pages");
    if !pages_dir.is_dir() {
        return false;
    }
    walk_for_content(&pages_dir, "getServerSideProps")
}

/// Check for SvelteKit +server.{ts,js} files in src/routes/.
fn has_sveltekit_server_routes(project_dir: &Path) -> bool {
    let routes_dir = project_dir.join("src/routes");
    if !routes_dir.is_dir() {
        return false;
    }
    walk_for_file(&routes_dir, &["+server.ts", "+server.js"])
}

/// Recursively walk a directory looking for files with specific names.
fn walk_for_file(dir: &Path, names: &[&str]) -> bool {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return false,
    };
    for entry in entries.flatten() {
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if ft.is_file() {
            if let Some(name) = entry.file_name().to_str()
                && names.contains(&name)
            {
                return true;
            }
        } else if ft.is_dir() && walk_for_file(&entry.path(), names) {
            return true;
        }
    }
    false
}

/// Recursively walk a directory looking for file content containing a string.
fn walk_for_content(dir: &Path, needle: &str) -> bool {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return false,
    };
    for entry in entries.flatten() {
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let path = entry.path();
        if ft.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str())
                && matches!(ext, "ts" | "tsx" | "js" | "jsx")
                && let Ok(content) = std::fs::read_to_string(&path)
                && content.contains(needle)
            {
                return true;
            }
        } else if ft.is_dir() && walk_for_content(&path, needle) {
            return true;
        }
    }
    false
}
