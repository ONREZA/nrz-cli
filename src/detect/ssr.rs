//! SSR compatibility analysis for Next.js, Nuxt, SvelteKit, Astro.

use std::path::Path;

use super::types::SsrAnalysis;

/// Analyze SSR features for a given framework.
/// Returns `None` if the framework is not SSR-capable.
pub fn analyze_ssr(project_dir: &Path, framework: &str) -> Option<SsrAnalysis> {
    match framework {
        "nextjs" => Some(analyze_nextjs(project_dir)),
        "nuxt" => Some(analyze_nuxt(project_dir)),
        "sveltekit" => Some(analyze_sveltekit(project_dir)),
        "astro" => Some(analyze_astro(project_dir)),
        _ => None,
    }
}

// ── Next.js ────────────────────────────────────────────────────

fn analyze_nextjs(project_dir: &Path) -> SsrAnalysis {
    let mut features = Vec::new();
    // Next.js defaults to SSR — static only with explicit output: 'export'
    let mut is_static_compatible = false;

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
        let stripped = strip_block_comments(content);
        if contains_value(&stripped, "output", "standalone") {
            features.push("output: 'standalone'".into());
        }
        if contains_value(&stripped, "output", "export") {
            features.push("output: 'export' (static)".into());
            is_static_compatible = true;
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

    // Check for "use server" directives (Server Actions / Server Components)
    let app_dir = project_dir.join("app");
    if app_dir.is_dir() && walk_for_content(&app_dir, "use server") {
        features.push("\"use server\" directives".into());
        is_static_compatible = false;
    }

    // Check for revalidate export (ISR — needs runtime)
    if app_dir.is_dir() && walk_for_content(&app_dir, "export const revalidate") {
        features.push("revalidate (ISR)".into());
        is_static_compatible = false;
    }

    // Check for getStaticProps / getStaticPaths (Pages Router SSG — informational)
    let pages_dir = project_dir.join("pages");
    if pages_dir.is_dir() {
        if walk_for_content(&pages_dir, "getStaticProps") {
            features.push("getStaticProps (SSG)".into());
        }
        if walk_for_content(&pages_dir, "getStaticPaths") {
            features.push("getStaticPaths (SSG)".into());
        }
    }

    // Check for generateStaticParams (App Router SSG — informational)
    if app_dir.is_dir() && walk_for_content(&app_dir, "generateStaticParams") {
        features.push("generateStaticParams (SSG)".into());
    }

    SsrAnalysis {
        is_static_compatible,
        ssr_features: features,
    }
}

// ── Nuxt ───────────────────────────────────────────────────────

fn analyze_nuxt(project_dir: &Path) -> SsrAnalysis {
    let mut features = Vec::new();
    // Nuxt defaults to SSR — static only with ssr: false or preset: 'static'
    let mut is_static_compatible = false;

    let config_content = read_config_file(project_dir, &["nuxt.config.ts", "nuxt.config.js"]);

    if let Some(ref content) = config_content {
        let stripped = strip_block_comments(content);

        // ssr: false means static-only
        if contains_value(&stripped, "ssr", "false") {
            features.push("ssr: false (static)".into());
            is_static_compatible = true;
        }

        // nitro preset: 'static' — use contains_value for accurate matching
        if contains_value(&stripped, "preset", "static") {
            features.push("preset: 'static'".into());
            is_static_compatible = true;
        }

        // routeRules with SSR-specific values
        if stripped.contains("routeRules")
            && contains_any_pattern(&stripped, &["ssr:", "redirect:", "proxy:", "prerender:"])
        {
            features.push("routeRules (hybrid rendering)".into());
            is_static_compatible = false;
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

    // Check for server/middleware/ directory
    if dir_has_files(project_dir, "server/middleware") {
        features.push("server/middleware/".into());
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
    // SvelteKit defaults to SSR — static only with adapter-static
    let mut is_static_compatible = false;

    let config_content = read_config_file(project_dir, &["svelte.config.js"]);

    if let Some(ref content) = config_content {
        let stripped = strip_block_comments(content);

        if stripped.contains("adapter-static") {
            features.push("adapter-static (static)".into());
            is_static_compatible = true;
        }

        // adapter-node or adapter-auto require runtime
        if stripped.contains("adapter-node") {
            features.push("adapter-node (runtime)".into());
        }
        if stripped.contains("adapter-auto") {
            features.push("adapter-auto (runtime)".into());
        }
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

    // Check for +page.server.{ts,js} and +layout.server.{ts,js} (server load functions)
    let routes_dir = project_dir.join("src/routes");
    if routes_dir.is_dir() {
        if walk_for_file(&routes_dir, &["+page.server.ts", "+page.server.js"]) {
            features.push("+page.server (server load)".into());
            is_static_compatible = false;
        }
        if walk_for_file(&routes_dir, &["+layout.server.ts", "+layout.server.js"]) {
            features.push("+layout.server (server load)".into());
            is_static_compatible = false;
        }

        // Check for form actions (export const actions)
        if walk_for_content_with_names(
            &routes_dir,
            "export const actions",
            &["+page.server.ts", "+page.server.js"],
        ) {
            features.push("form actions".into());
            is_static_compatible = false;
        }
    }

    SsrAnalysis {
        is_static_compatible,
        ssr_features: features,
    }
}

// ── Astro ──────────────────────────────────────────────────────

fn analyze_astro(project_dir: &Path) -> SsrAnalysis {
    let mut features = Vec::new();
    // Astro defaults to static — SSR only with output: 'server' or 'hybrid'
    let mut is_static_compatible = true;

    let config_content = read_config_file(
        project_dir,
        &["astro.config.mjs", "astro.config.ts", "astro.config.js"],
    );

    if let Some(ref content) = config_content {
        let stripped = strip_block_comments(content);

        // output: 'server' — full SSR mode
        if contains_value(&stripped, "output", "server") {
            features.push("output: 'server' (SSR)".into());
            is_static_compatible = false;
        }

        // output: 'hybrid' — hybrid rendering (some SSR, some static)
        if contains_value(&stripped, "output", "hybrid") {
            features.push("output: 'hybrid'".into());
            is_static_compatible = false;
        }

        // Check for SSR adapter integrations in config
        if stripped.contains("@astrojs/node")
            || stripped.contains("@astrojs/vercel")
            || stripped.contains("@astrojs/netlify")
            || stripped.contains("@astrojs/cloudflare")
        {
            features.push("SSR adapter integration".into());
            is_static_compatible = false;
        }
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

/// Strip block comments (`/* ... */`) from content, respecting string literals.
/// Line comments (`//`) are handled separately in `contains_value`/`contains_any_pattern`.
/// Note: template literal interpolation (`${...}`) is not fully handled —
/// acceptable since config files rarely use complex template literals.
fn strip_block_comments(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        // Skip string literals (don't interpret /* inside strings)
        if b == b'\'' || b == b'"' || b == b'`' {
            let quote = b;
            result.push(b as char);
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    result.push(bytes[i] as char);
                    result.push(bytes[i + 1] as char);
                    i += 2;
                } else {
                    result.push(bytes[i] as char);
                    i += 1;
                }
            }
            if i < bytes.len() {
                result.push(bytes[i] as char);
                i += 1;
            }
        } else if i + 1 < bytes.len() && b == b'/' && bytes[i + 1] == b'*' {
            // Block comment — skip until */
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < bytes.len() {
                i += 2; // skip */
            }
        } else {
            result.push(b as char);
            i += 1;
        }
    }
    result
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

/// Check if content contains any of the given patterns (non-comment lines only).
fn contains_any_pattern(content: &str, patterns: &[&str]) -> bool {
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with('*') {
            continue;
        }
        for &pattern in patterns {
            if trimmed.contains(pattern) {
                return true;
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

/// Directories to skip during recursive walks.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".output",
    ".astro",
];

fn should_skip_dir(entry: &std::fs::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| SKIP_DIRS.contains(&name))
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
        } else if ft.is_dir() && !should_skip_dir(&entry) && walk_for_file(&entry.path(), names) {
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
        } else if ft.is_dir() && !should_skip_dir(&entry) && walk_for_content(&path, needle) {
            return true;
        }
    }
    false
}

/// Recursively walk looking for specific file names that contain a string.
fn walk_for_content_with_names(dir: &Path, needle: &str, file_names: &[&str]) -> bool {
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
            if let Some(name) = entry.file_name().to_str()
                && file_names.contains(&name)
                && let Ok(content) = std::fs::read_to_string(&path)
                && content.contains(needle)
            {
                return true;
            }
        } else if ft.is_dir()
            && !should_skip_dir(&entry)
            && walk_for_content_with_names(&path, needle, file_names)
        {
            return true;
        }
    }
    false
}
