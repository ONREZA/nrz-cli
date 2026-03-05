//! SSR compatibility analysis for Next.js, Nuxt, SvelteKit, Astro,
//! SolidStart, Qwik City, and Analog.

use super::fs::Fs;
use super::types::SsrAnalysis;

/// Analyze SSR features for a given framework.
/// Returns `None` if the framework is not SSR-capable.
pub fn analyze_ssr(fs: &dyn Fs, framework: &str) -> Option<SsrAnalysis> {
    match framework {
        "nextjs" => Some(analyze_nextjs(fs)),
        "nuxt" => Some(analyze_nuxt(fs)),
        "sveltekit" => Some(analyze_sveltekit(fs)),
        "astro" => Some(analyze_astro(fs)),
        "react-router" => Some(analyze_react_router(fs)),
        "remix" => Some(analyze_remix(fs)),
        "solidstart" => Some(analyze_solidstart(fs)),
        "qwik" => Some(analyze_qwik(fs)),
        "analog" => Some(analyze_analog(fs)),
        _ => None,
    }
}

// ── Next.js ────────────────────────────────────────────────────

fn analyze_nextjs(fs: &dyn Fs) -> SsrAnalysis {
    let mut features = Vec::new();
    // Next.js defaults to SSR — static only with explicit output: 'export'
    let mut is_static_compatible = false;

    // Check next.config for output mode
    let config_content = read_config_file(
        fs,
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
    if file_exists_any(fs, &["middleware.ts", "middleware.js"]) {
        features.push("middleware".into());
        is_static_compatible = false;
    }

    // Check for API routes (pages/api/)
    if dir_has_files(fs, "pages/api") {
        features.push("pages/api/ routes".into());
        is_static_compatible = false;
    }

    // Check for route handlers (app/**/route.{ts,js})
    if has_route_handlers(fs) {
        features.push("app/ route handlers".into());
        is_static_compatible = false;
    }

    // Check for getServerSideProps
    if has_gssp(fs) {
        features.push("getServerSideProps".into());
        is_static_compatible = false;
    }

    // Check for "use server" directives (Server Actions / Server Components)
    if fs.is_dir("app") && walk_for_content(fs, "app", "use server") {
        features.push("\"use server\" directives".into());
        is_static_compatible = false;
    }

    // Check for revalidate export (ISR — needs runtime)
    if fs.is_dir("app") && walk_for_content(fs, "app", "export const revalidate") {
        features.push("revalidate (ISR)".into());
        is_static_compatible = false;
    }

    // Check for getStaticProps / getStaticPaths (Pages Router SSG — informational)
    if fs.is_dir("pages") {
        if walk_for_content(fs, "pages", "getStaticProps") {
            features.push("getStaticProps (SSG)".into());
        }
        if walk_for_content(fs, "pages", "getStaticPaths") {
            features.push("getStaticPaths (SSG)".into());
        }
    }

    // Check for generateStaticParams (App Router SSG — informational)
    if fs.is_dir("app") && walk_for_content(fs, "app", "generateStaticParams") {
        features.push("generateStaticParams (SSG)".into());
    }

    SsrAnalysis {
        is_static_compatible,
        ssr_features: features,
    }
}

// ── Nuxt ───────────────────────────────────────────────────────

fn analyze_nuxt(fs: &dyn Fs) -> SsrAnalysis {
    let mut features = Vec::new();
    // Nuxt defaults to SSR — static only with ssr: false or preset: 'static'
    let mut is_static_compatible = false;

    let config_content = read_config_file(fs, &["nuxt.config.ts", "nuxt.config.js"]);

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
    if dir_has_files(fs, "server/api") {
        features.push("server/api/ routes".into());
        is_static_compatible = false;
    }

    // Check for server/routes/ directory
    if dir_has_files(fs, "server/routes") {
        features.push("server/routes/".into());
        is_static_compatible = false;
    }

    // Check for server/middleware/ directory
    if dir_has_files(fs, "server/middleware") {
        features.push("server/middleware/".into());
        is_static_compatible = false;
    }

    SsrAnalysis {
        is_static_compatible,
        ssr_features: features,
    }
}

// ── SvelteKit ──────────────────────────────────────────────────

fn analyze_sveltekit(fs: &dyn Fs) -> SsrAnalysis {
    let mut features = Vec::new();
    // SvelteKit defaults to SSR — static only with adapter-static
    let mut is_static_compatible = false;

    let config_content = read_config_file(fs, &["svelte.config.js", "svelte.config.ts"]);

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
    if has_sveltekit_server_routes(fs) {
        features.push("+server routes".into());
        is_static_compatible = false;
    }

    // Check for hooks.server.{ts,js}
    if file_exists_any(fs, &["src/hooks.server.ts", "src/hooks.server.js"]) {
        features.push("hooks.server".into());
        is_static_compatible = false;
    }

    // Check for +page.server.{ts,js} and +layout.server.{ts,js} (server load functions)
    if fs.is_dir("src/routes") {
        if walk_for_file(fs, "src/routes", &["+page.server.ts", "+page.server.js"]) {
            features.push("+page.server (server load)".into());
            is_static_compatible = false;
        }
        if walk_for_file(
            fs,
            "src/routes",
            &["+layout.server.ts", "+layout.server.js"],
        ) {
            features.push("+layout.server (server load)".into());
            is_static_compatible = false;
        }

        // Check for form actions (export const actions)
        if walk_for_content_with_names(
            fs,
            "src/routes",
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

fn analyze_astro(fs: &dyn Fs) -> SsrAnalysis {
    let mut features = Vec::new();
    // Astro defaults to static — SSR only with output: 'server' or 'hybrid'
    let mut is_static_compatible = true;

    let config_content = read_config_file(
        fs,
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

// ── React Router v7 ────────────────────────────────────────────

fn analyze_react_router(fs: &dyn Fs) -> SsrAnalysis {
    let mut features = Vec::new();
    // React Router v7 defaults to SSR — static only with ssr: false in react-router.config
    let mut is_static_compatible = false;

    // Check react-router.config.ts/js for ssr: false
    let config_content =
        read_config_file(fs, &["react-router.config.ts", "react-router.config.js"]);

    if let Some(ref content) = config_content {
        let stripped = strip_block_comments(content);

        if contains_value(&stripped, "ssr", "false") {
            features.push("ssr: false (SPA mode)".into());
            is_static_compatible = true;
        }
    }

    // Check for loader/action exports in routes (same structure as Remix)
    analyze_route_exports(fs, &mut features, &mut is_static_compatible);

    // Check for entry.server
    if file_exists_any(
        fs,
        &[
            "app/entry.server.tsx",
            "app/entry.server.ts",
            "app/entry.server.jsx",
            "app/entry.server.js",
        ],
    ) {
        features.push("entry.server".into());
    }

    SsrAnalysis {
        is_static_compatible,
        ssr_features: features,
    }
}

// ── Remix ──────────────────────────────────────────────────────

fn analyze_remix(fs: &dyn Fs) -> SsrAnalysis {
    let mut features = Vec::new();
    // Remix defaults to SSR — static only with explicit ssr: false in vite config
    let mut is_static_compatible = false;

    // Remix v2+ uses Vite — check vite.config for ssr: false
    let config_content = read_config_file(
        fs,
        &[
            "vite.config.ts",
            "vite.config.mts",
            "vite.config.js",
            "vite.config.mjs",
        ],
    );

    if let Some(ref content) = config_content {
        let stripped = strip_block_comments(content);

        // ssr: false disables server rendering
        if contains_value(&stripped, "ssr", "false") {
            features.push("ssr: false (SPA mode)".into());
            is_static_compatible = true;
        }
    }

    // Legacy Remix v1: check remix.config.js only if no vite config exists
    if config_content.is_none() {
        let legacy_config = read_config_file(fs, &["remix.config.js"]);
        if let Some(ref content) = legacy_config {
            let stripped = strip_block_comments(content);
            if contains_value(&stripped, "ssr", "false") {
                features.push("ssr: false (legacy remix.config)".into());
                is_static_compatible = true;
            }
        }
    }

    // Check for loader/action exports in routes
    analyze_route_exports(fs, &mut features, &mut is_static_compatible);

    // Check for entry.server
    if file_exists_any(fs, &["app/entry.server.tsx", "app/entry.server.ts"]) {
        features.push("entry.server".into());
    }

    SsrAnalysis {
        is_static_compatible,
        ssr_features: features,
    }
}

/// Shared route analysis for Remix and React Router v7.
/// Both use app/routes/ with exported `loader` and `action` functions.
fn analyze_route_exports(fs: &dyn Fs, features: &mut Vec<String>, is_static_compatible: &mut bool) {
    if !fs.is_dir("app/routes") {
        return;
    }

    if walk_for_exported_symbol(fs, "app/routes", "loader") {
        features.push("route loaders".into());
        *is_static_compatible = false;
    }
    if walk_for_exported_symbol(fs, "app/routes", "action") {
        features.push("route actions".into());
        *is_static_compatible = false;
    }
}

// ── SolidStart ──────────────────────────────────────────────────

fn analyze_solidstart(fs: &dyn Fs) -> SsrAnalysis {
    let mut features = Vec::new();
    // SolidStart defaults to SSR — static only with ssr: false in app.config
    let mut is_static_compatible = false;

    let config_content = read_config_file(fs, &["app.config.ts", "app.config.js"]);

    if let Some(ref content) = config_content {
        let stripped = strip_block_comments(content);

        if contains_value(&stripped, "ssr", "false") {
            features.push("ssr: false (static)".into());
            is_static_compatible = true;
        }
    }

    // Check for API routes (src/routes/api/)
    if dir_has_files(fs, "src/routes/api") {
        features.push("src/routes/api/ routes".into());
        is_static_compatible = false;
    }

    // Check for "use server" directives in src/
    if fs.is_dir("src") && walk_for_content(fs, "src", "use server") {
        features.push("\"use server\" directives".into());
        is_static_compatible = false;
    }

    SsrAnalysis {
        is_static_compatible,
        ssr_features: features,
    }
}

// ── Qwik City ───────────────────────────────────────────────────

fn analyze_qwik(fs: &dyn Fs) -> SsrAnalysis {
    let mut features = Vec::new();
    // Qwik City defaults to SSR — static only with no server-side features
    let mut is_static_compatible = false;

    // Check vite config for SSR-related settings
    let config_content = read_config_file(
        fs,
        &[
            "vite.config.ts",
            "vite.config.mts",
            "vite.config.js",
            "vite.config.mjs",
        ],
    );

    if let Some(ref content) = config_content {
        let stripped = strip_block_comments(content);

        // Check for static adapter
        if stripped.contains("@builder.io/qwik-city/adaptors/static")
            || stripped.contains("@qwik.dev/router/adaptors/static")
        {
            features.push("static adaptor".into());
            is_static_compatible = true;
        }
    }

    // Check for server-side features in routes
    if fs.is_dir("src/routes") {
        // routeLoader$, routeAction$ — server-side data loading
        if walk_for_content(fs, "src/routes", "routeLoader$") {
            features.push("routeLoader$".into());
            is_static_compatible = false;
        }
        if walk_for_content(fs, "src/routes", "routeAction$") {
            features.push("routeAction$".into());
            is_static_compatible = false;
        }
    }

    // Check for server$ functions anywhere in src/
    if fs.is_dir("src") && walk_for_content(fs, "src", "server$") {
        features.push("server$ functions".into());
        is_static_compatible = false;
    }

    SsrAnalysis {
        is_static_compatible,
        ssr_features: features,
    }
}

// ── Analog ──────────────────────────────────────────────────────

fn analyze_analog(fs: &dyn Fs) -> SsrAnalysis {
    let mut features = Vec::new();
    // Analog defaults to SSR — static only with ssr: false or prerender-only config
    let mut is_static_compatible = false;

    // Analog uses vite.config with analog() plugin
    let config_content = read_config_file(
        fs,
        &[
            "vite.config.ts",
            "vite.config.mts",
            "vite.config.js",
            "vite.config.mjs",
        ],
    );

    if let Some(ref content) = config_content {
        let stripped = strip_block_comments(content);

        if contains_value(&stripped, "ssr", "false") {
            features.push("ssr: false (static)".into());
            is_static_compatible = true;
        }
    }

    // Check for server routes (src/server/routes/)
    if dir_has_files(fs, "src/server/routes") {
        features.push("src/server/routes/".into());
        is_static_compatible = false;
    }

    // Check for API routes (src/server/api/ — alternative convention)
    if dir_has_files(fs, "src/server/api") {
        features.push("src/server/api/".into());
        is_static_compatible = false;
    }

    SsrAnalysis {
        is_static_compatible,
        ssr_features: features,
    }
}

// ── Helpers ────────────────────────────────────────────────────

/// Read the first existing config file from a list.
fn read_config_file(fs: &dyn Fs, candidates: &[&str]) -> Option<String> {
    for name in candidates {
        if let Some(content) = fs.read_file(name) {
            return Some(content);
        }
    }
    None
}

/// Strip block comments (`/* ... */`) from content, respecting string literals.
/// Line comments (`//`) are handled separately in `contains_value`/`contains_any_pattern`.
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
fn file_exists_any(fs: &dyn Fs, files: &[&str]) -> bool {
    files.iter().any(|f| fs.exists(f))
}

/// Check if a directory has any files (non-recursive, just direct children).
fn dir_has_files(fs: &dyn Fs, subdir: &str) -> bool {
    if !fs.is_dir(subdir) {
        return false;
    }
    fs.list_dir(subdir).iter().any(|entry| {
        let path = format!("{subdir}/{entry}");
        !fs.is_dir(&path)
    })
}

/// Strip `//` inline comment from a line, respecting string literals.
fn strip_inline_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\'' || b == b'"' || b == b'`' {
            let quote = b;
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 1;
                }
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
        } else if i + 1 < bytes.len() && b == b'/' && bytes[i + 1] == b'/' {
            return &line[..i];
        } else {
            i += 1;
        }
    }
    line
}

/// Simple string check: does the content contain `key: value` (with or without quotes)
/// anywhere in a non-comment line?
fn contains_value(content: &str, key: &str, value: &str) -> bool {
    for line in content.lines() {
        let trimmed = line.trim_start();
        // Skip full-line comments
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with('*') {
            continue;
        }
        // Strip inline comments (// ...) respecting string literals
        let line = strip_inline_comment(line);
        if let Some(idx) = line.find(key) {
            let after = &line[idx + key.len()..];
            let after = after.trim_start();
            if let Some(after) = after.strip_prefix(':') {
                let after = after.trim();
                if match_value_token(after, value) {
                    return true;
                }
                // Check for env variable fallback: process.env.X || 'value' / ?? 'value'
                if let Some(fallback) = find_fallback_value(after)
                    && match_value_token(fallback, value)
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a value token starts at the given position (quoted or unquoted).
fn match_value_token(after: &str, value: &str) -> bool {
    // Match quoted values: 'value', "value", `value`
    if after.starts_with(&format!("'{value}'"))
        || after.starts_with(&format!("\"{value}\""))
        || after.starts_with(&format!("`{value}`"))
    {
        return true;
    }
    // Match unquoted value with word boundary check
    if let Some(rest) = after.strip_prefix(value)
        && (rest.is_empty()
            || rest.starts_with(',')
            || rest.starts_with('}')
            || rest.starts_with(')')
            || rest.starts_with(';')
            || rest.starts_with(' ')
            || rest.starts_with('\t'))
    {
        return true;
    }
    false
}

/// Find the fallback value after `||` or `??` operators.
/// Returns the trimmed text after the operator.
fn find_fallback_value(text: &str) -> Option<&str> {
    let idx = text
        .find("||")
        .map(|i| i + 2)
        .or_else(|| text.find("??").map(|i| i + 2))?;
    let after = text[idx..].trim();
    if after.is_empty() { None } else { Some(after) }
}

/// Check if content contains any of the given patterns (non-comment lines only).
fn contains_any_pattern(content: &str, patterns: &[&str]) -> bool {
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with('*') {
            continue;
        }
        let effective = strip_inline_comment(trimmed);
        for &pattern in patterns {
            if effective.contains(pattern) {
                return true;
            }
        }
    }
    false
}

/// Check for Next.js route handlers (app/**/route.{ts,js}).
fn has_route_handlers(fs: &dyn Fs) -> bool {
    if !fs.is_dir("app") {
        return false;
    }
    walk_for_file(fs, "app", &["route.ts", "route.js"])
}

/// Check for getServerSideProps in pages/ directory.
fn has_gssp(fs: &dyn Fs) -> bool {
    if !fs.is_dir("pages") {
        return false;
    }
    walk_for_content(fs, "pages", "getServerSideProps")
}

/// Check for SvelteKit +server.{ts,js} files in src/routes/.
fn has_sveltekit_server_routes(fs: &dyn Fs) -> bool {
    if !fs.is_dir("src/routes") {
        return false;
    }
    walk_for_file(fs, "src/routes", &["+server.ts", "+server.js"])
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

fn should_skip_dir(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

/// Build a child path from a directory and entry name.
fn child_path(dir: &str, entry: &str) -> String {
    if dir.is_empty() {
        entry.to_string()
    } else {
        format!("{dir}/{entry}")
    }
}

/// Recursively walk a directory looking for files with specific names.
fn walk_for_file(fs: &dyn Fs, dir: &str, names: &[&str]) -> bool {
    for entry in fs.list_dir(dir) {
        let path = child_path(dir, &entry);
        if fs.is_dir(&path) {
            if !should_skip_dir(&entry) && walk_for_file(fs, &path, names) {
                return true;
            }
        } else if names.contains(&entry.as_str()) {
            return true;
        }
    }
    false
}

/// Recursively walk a directory looking for file content containing a string.
fn walk_for_content(fs: &dyn Fs, dir: &str, needle: &str) -> bool {
    for entry in fs.list_dir(dir) {
        let path = child_path(dir, &entry);
        if fs.is_dir(&path) {
            if !should_skip_dir(&entry) && walk_for_content(fs, &path, needle) {
                return true;
            }
        } else if is_code_file(&entry)
            && let Some(content) = fs.read_file(&path)
            && content.contains(needle)
        {
            return true;
        }
    }
    false
}

/// Recursively walk route files looking for exported symbols (e.g. `loader`, `action`).
fn walk_for_exported_symbol(fs: &dyn Fs, dir: &str, symbol: &str) -> bool {
    for entry in fs.list_dir(dir) {
        let path = child_path(dir, &entry);
        if fs.is_dir(&path) {
            if !should_skip_dir(&entry) && walk_for_exported_symbol(fs, &path, symbol) {
                return true;
            }
        } else if is_code_file(&entry)
            && let Some(content) = fs.read_file(&path)
            && file_has_exported_symbol(&content, symbol)
        {
            return true;
        }
    }
    false
}

/// Check if file content has an exported symbol on a non-comment line.
fn file_has_exported_symbol(content: &str, symbol: &str) -> bool {
    for line in content.lines() {
        let trimmed = line.trim();
        // Skip comment lines
        if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
            continue;
        }
        // Must have `export` keyword
        if !trimmed.contains("export") {
            continue;
        }
        // Check for: export function/async function/const/let/var <symbol>
        // or: export { <symbol> ... }
        if contains_word(trimmed, symbol) {
            return true;
        }
    }
    false
}

/// Check if `haystack` contains `word` as a whole word (not a substring).
fn contains_word(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(word) {
        let abs = start + pos;
        let before_ok =
            abs == 0 || !bytes[abs - 1].is_ascii_alphanumeric() && bytes[abs - 1] != b'_';
        let after = abs + word.len();
        let after_ok =
            after >= bytes.len() || !bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_';
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

/// Recursively walk looking for specific file names that contain a string.
fn walk_for_content_with_names(fs: &dyn Fs, dir: &str, needle: &str, file_names: &[&str]) -> bool {
    for entry in fs.list_dir(dir) {
        let path = child_path(dir, &entry);
        if fs.is_dir(&path) {
            if !should_skip_dir(&entry)
                && walk_for_content_with_names(fs, &path, needle, file_names)
            {
                return true;
            }
        } else if file_names.contains(&entry.as_str())
            && let Some(content) = fs.read_file(&path)
            && content.contains(needle)
        {
            return true;
        }
    }
    false
}

fn is_code_file(name: &str) -> bool {
    matches!(name.rsplit('.').next(), Some("ts" | "tsx" | "js" | "jsx"))
}
