//! SSR compatibility analysis for Next.js, Nuxt, SvelteKit, Astro,
//! SolidStart, Qwik City, and Analog.

use super::fs::Fs;
use super::types::SsrAnalysis;

/// Analyze SSR features for a given framework.
/// Returns `None` if the framework is not SSR-capable.
pub fn analyze_ssr(fs: &dyn Fs, framework: &str) -> Option<SsrAnalysis> {
    match framework {
        "nextjs" | "blitzjs" | "payload" => Some(analyze_nextjs(fs)),
        "nuxt" => Some(analyze_nuxt(fs)),
        "sveltekit" => Some(analyze_sveltekit(fs)),
        "astro" => Some(analyze_astro(fs)),
        "react-router" | "hydrogen" => Some(analyze_react_router(fs)),
        "remix" => Some(analyze_remix(fs)),
        "tanstack-start" => Some(analyze_tanstack_start(fs)),
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
        let stripped = strip_comments(content);
        if config_value(&stripped, &["output"], None) == ConfigValue::String("standalone") {
            features.push("output: 'standalone'".into());
        }
        if config_value(&stripped, &["output"], None) == ConfigValue::String("export") {
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
    if next_app_walk_for_content(fs, "use server") {
        features.push("\"use server\" directives".into());
        is_static_compatible = false;
    }

    // Check for revalidate export (ISR — needs runtime)
    if next_app_walk_for_content(fs, "export const revalidate") {
        features.push("revalidate (ISR)".into());
        is_static_compatible = false;
    }

    if next_app_walk_for_content(fs, "force-dynamic") {
        features.push("dynamic rendering".into());
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
    if next_app_walk_for_content(fs, "generateStaticParams") {
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
        let stripped = strip_comments(content);

        let ssr = config_value(&stripped, &["ssr"], None);
        let preset = config_value(&stripped, &["nitro", "preset"], None);
        if ssr == ConfigValue::Boolean(false) {
            features.push("ssr: false (static)".into());
        }
        if preset == ConfigValue::String("static") {
            features.push("preset: 'static'".into());
        }
        is_static_compatible = ssr != ConfigValue::Unknown
            && preset != ConfigValue::Unknown
            && (ssr == ConfigValue::Boolean(false) || preset == ConfigValue::String("static"));

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
        let stripped = strip_comments(content);

        match config_adapter(&stripped, &["kit", "adapter"]) {
            Some("@sveltejs/adapter-static") => {
                features.push("adapter-static (static)".into());
                is_static_compatible = true;
            }
            Some("@sveltejs/adapter-node") => {
                features.push("adapter-node (runtime)".into());
            }
            Some("@sveltejs/adapter-auto") => {
                features.push("adapter-auto (runtime)".into());
            }
            _ => {}
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
        let stripped = strip_comments(content);

        match config_value(&stripped, &["output"], None) {
            ConfigValue::Absent | ConfigValue::String("static") => {}
            ConfigValue::String("server") => {
                features.push("output: 'server' (SSR)".into());
                is_static_compatible = false;
            }
            ConfigValue::String("hybrid") => {
                features.push("output: 'hybrid'".into());
                is_static_compatible = false;
            }
            ConfigValue::String(_) | ConfigValue::Boolean(_) | ConfigValue::Unknown => {
                features.push("output: unknown (SSR)".into());
                is_static_compatible = false;
            }
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
        let stripped = strip_comments(content);

        if config_value(&stripped, &["ssr"], None) == ConfigValue::Boolean(false) {
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
        let stripped = strip_comments(content);

        // ssr: false disables server rendering
        if config_value(&stripped, &["ssr"], Some("remix")) == ConfigValue::Boolean(false) {
            features.push("ssr: false (SPA mode)".into());
            is_static_compatible = true;
        }
    }

    // Legacy Remix v1: check remix.config.js only if no vite config exists
    if config_content.is_none() {
        let legacy_config = read_config_file(fs, &["remix.config.js"]);
        if let Some(ref content) = legacy_config {
            let stripped = strip_comments(content);
            if config_value(&stripped, &["ssr"], None) == ConfigValue::Boolean(false) {
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
        let stripped = strip_comments(content);

        if config_value(&stripped, &["ssr"], None) == ConfigValue::Boolean(false) {
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

// ── TanStack Start ─────────────────────────────────────────────

fn analyze_tanstack_start(fs: &dyn Fs) -> SsrAnalysis {
    let mut features = Vec::new();

    if fs.is_dir("src/routes") && walk_for_content(fs, "src/routes", "createServerFn") {
        features.push("server functions".into());
    }

    SsrAnalysis {
        is_static_compatible: false,
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
        let stripped = strip_comments(content);

        // Check for static adapter
        if has_applied_qwik_static_adapter(&stripped) {
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
        let stripped = strip_comments(content);

        if config_value(&stripped, &["ssr"], Some("analog")) == ConfigValue::Boolean(false) {
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
        if fs.exists(name) {
            // An unreadable preferred config is unknown, rather than a missing config.
            return Some(String::new());
        }
    }
    None
}

/// Remove comments without changing quoted text or UTF-8 characters.
fn strip_comments(content: &str) -> String {
    let Some(tokens) = config_tokens(content) else {
        return String::new();
    };
    let mut result = String::with_capacity(content.len());
    let mut end = 0;
    let comment_whitespace = |character: char| {
        if character.is_whitespace() {
            character
        } else {
            ' '
        }
    };
    for token in &tokens {
        result.extend(content[end..token.start].chars().map(comment_whitespace));
        result.push_str(token.text);
        end = token.end;
    }
    result.extend(content[end..].chars().map(comment_whitespace));
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

#[derive(Clone, Copy)]
struct ConfigToken<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConfigValue<'a> {
    Absent,
    Boolean(bool),
    String(&'a str),
    Unknown,
}

/// Tokenize the whole file; quoted text is never interpreted as code.
fn config_tokens(content: &str) -> Option<Vec<ConfigToken<'_>>> {
    let mut characters = content.char_indices().peekable();
    let mut tokens = Vec::new();
    while let Some((start, character)) = characters.next() {
        if character.is_whitespace() {
            continue;
        }
        if character == '/' {
            match characters.peek().map(|(_, character)| *character) {
                Some('/') => {
                    characters.next();
                    for (_, next) in characters.by_ref() {
                        if matches!(next, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                            break;
                        }
                    }
                }
                Some('*') => {
                    characters.next();
                    let mut closed = false;
                    while let Some((_, next)) = characters.next() {
                        if next == '*'
                            && characters
                                .peek()
                                .is_some_and(|(_, character)| *character == '/')
                        {
                            characters.next();
                            closed = true;
                            break;
                        }
                    }
                    if !closed {
                        return None;
                    }
                }
                _ => return None,
            }
            continue;
        }
        if matches!(character, '\'' | '"' | '`') {
            let mut closed = false;
            while let Some((_, next)) = characters.next() {
                if next == '\\' {
                    characters.next()?;
                } else if next == character {
                    closed = true;
                    break;
                } else if character == '`'
                    && next == '$'
                    && characters
                        .peek()
                        .is_some_and(|(_, character)| *character == '{')
                {
                    // Template interpolation requires JavaScript parsing.
                    return None;
                }
            }
            if !closed {
                return None;
            }
        } else if !"{}[]():,.;=<>?!+-*%&|^~".contains(character) {
            while characters.peek().is_some_and(|(_, next)| {
                !next.is_whitespace() && !"{}[]():,.;=<>?!+-*/%&|^~'\"`".contains(*next)
            }) {
                characters.next();
            }
        }
        let end = characters.peek().map_or(content.len(), |(index, _)| *index);
        tokens.push(ConfigToken {
            text: &content[start..end],
            start,
            end,
        });
    }
    Some(tokens)
}

const MAX_CONFIG_NESTING: usize = 128;

fn closing_token(tokens: &[ConfigToken<'_>], start: usize) -> Option<usize> {
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match token.text {
            "(" => stack.push(")"),
            "[" => stack.push("]"),
            "{" => stack.push("}"),
            ")" | "]" | "}" => {
                if stack.pop() != Some(token.text) {
                    return None;
                }
                if stack.is_empty() {
                    return Some(index);
                }
            }
            _ => {}
        }
        if stack.len() > MAX_CONFIG_NESTING {
            return None;
        }
    }
    None
}

fn group_contents<'a, 't>(
    tokens: &'t [ConfigToken<'a>],
    opening: &str,
) -> Option<&'t [ConfigToken<'a>]> {
    let [first, inner @ .., _] = tokens else {
        return None;
    };
    if first.text != opening {
        return None;
    }
    if closing_token(tokens, 0)? != tokens.len() - 1 {
        return None;
    }
    Some(inner)
}

fn is_identifier(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first.is_alphabetic() || matches!(first, '_' | '$'))
        && characters.all(|character| character.is_alphanumeric() || matches!(character, '_' | '$'))
}

fn type_expression_end(tokens: &[ConfigToken<'_>], start: usize, depth: usize) -> Option<usize> {
    let mut end = type_term_end(tokens, start, depth)?;
    for _ in 0..tokens.len() {
        if !tokens
            .get(end)
            .is_some_and(|token| matches!(token.text, "|" | "&"))
        {
            break;
        }
        end = type_term_end(tokens, end + 1, depth)?;
    }
    Some(end)
}

fn type_term_end(tokens: &[ConfigToken<'_>], start: usize, depth: usize) -> Option<usize> {
    if depth >= MAX_CONFIG_NESTING {
        return None;
    }
    let first = tokens.get(start)?.text;
    let mut end = if matches!(first, "readonly" | "keyof" | "typeof") {
        type_term_end(tokens, start + 1, depth + 1)?
    } else if first == "(" {
        let end = type_expression_end(tokens, start + 1, depth + 1)?;
        if tokens.get(end)?.text != ")" {
            return None;
        }
        end + 1
    } else if first == "[" {
        type_list_end(tokens, start + 1, "]", depth + 1)?
    } else if is_identifier(first)
        || first.chars().all(|character| character.is_ascii_digit())
        || matches!(first.as_bytes().first(), Some(b'\'' | b'"'))
    {
        start + 1
    } else {
        return None;
    };
    for _ in 0..tokens.len() {
        match tokens.get(end).map(|token| token.text) {
            Some(".")
                if tokens
                    .get(end + 1)
                    .is_some_and(|token| is_identifier(token.text)) =>
            {
                end += 2
            }
            Some("<") => end = type_list_end(tokens, end + 1, ">", depth + 1)?,
            Some("[") => {
                end += 1;
                if tokens.get(end)?.text != "]" {
                    end = type_expression_end(tokens, end, depth + 1)?;
                }
                if tokens.get(end)?.text != "]" {
                    return None;
                }
                end += 1;
            }
            _ => return Some(end),
        }
    }
    None
}

fn type_list_end(
    tokens: &[ConfigToken<'_>],
    start: usize,
    closing: &str,
    depth: usize,
) -> Option<usize> {
    let mut cursor = tokens.iter().enumerate().skip(start).peekable();
    if cursor.peek()?.1.text == closing {
        cursor.next();
    } else {
        loop {
            let start = cursor.peek()?.0;
            let end = type_expression_end(tokens, start, depth)?;
            let (_, separator) = cursor.find(|(index, _)| *index == end)?;
            match separator.text {
                "," => {}
                token if token == closing => break,
                _ => return None,
            }
        }
    }
    Some(cursor.peek().map_or(tokens.len(), |(index, _)| *index))
}

fn without_type_assertions<'a, 't>(
    mut tokens: &'t [ConfigToken<'a>],
) -> Option<&'t [ConfigToken<'a>]> {
    for _ in 0..MAX_CONFIG_NESTING {
        if let Some(inner) = group_contents(tokens, "(") {
            tokens = inner;
            continue;
        }
        let mut cursor = tokens.iter().enumerate();
        let mut assertion = None;
        while let Some((index, token)) = cursor.next() {
            match token.text {
                "(" | "[" | "{" => {
                    let closing = closing_token(tokens, index)?;
                    cursor.find(|(position, _)| *position == closing)?;
                }
                "as" | "satisfies" => {
                    assertion = Some(index);
                    break;
                }
                _ => {}
            }
        }
        let Some(index) = assertion else {
            return Some(tokens);
        };
        let annotation = &tokens[index + 1..];
        let mut end = type_expression_end(annotation, 0, 0)?;
        for _ in 0..annotation.len() {
            if end >= annotation.len() {
                break;
            }
            if !matches!(annotation[end].text, "as" | "satisfies") {
                return None;
            }
            end = type_expression_end(annotation, end + 1, 0)?;
        }
        tokens = &tokens[..index];
    }
    None
}

fn literal_value<'a>(tokens: &[ConfigToken<'a>]) -> ConfigValue<'a> {
    let Some(tokens) = without_type_assertions(tokens) else {
        return ConfigValue::Unknown;
    };
    let [token] = tokens else {
        return ConfigValue::Unknown;
    };
    if matches!(token.text, "true" | "false") {
        return ConfigValue::Boolean(token.text == "true");
    }
    if matches!(token.text.as_bytes().first(), Some(b'\'' | b'"' | b'`'))
        && !token.text.contains('\\')
    {
        return ConfigValue::String(&token.text[1..token.text.len() - 1]);
    }
    ConfigValue::Unknown
}

fn object_expression<'a, 't>(tokens: &'t [ConfigToken<'a>]) -> Option<&'t [ConfigToken<'a>]> {
    let mut tokens = without_type_assertions(tokens)?;
    if matches!(tokens.first()?.text, "defineConfig" | "defineNuxtConfig") {
        tokens = without_type_assertions(group_contents(&tokens[1..], "(")?)?;
    }
    group_contents(tokens, "{")
}

fn trusted_import_binding(local: &str, imported: &str, source: &str) -> bool {
    match local {
        "defineConfig" => matches!(
            (imported, source),
            (
                "defineConfig",
                "vite" | "astro/config" | "@solidjs/start/config"
            )
        ),
        "defineNuxtConfig" => matches!((imported, source), ("defineNuxtConfig", "nuxt/config")),
        "remix" => matches!((imported, source), ("vitePlugin", "@remix-run/dev")),
        "analog" => matches!((imported, source), ("default", "@analogjs/platform")),
        "qwikCity" => matches!(
            (imported, source),
            ("qwikCity", "@builder.io/qwik-city/vite")
        ),
        "qwikRouter" => matches!((imported, source), ("qwikRouter", "@qwik.dev/router/vite")),
        _ => matches!(
            (imported, source),
            (
                "default",
                "@sveltejs/adapter-static"
                    | "@sveltejs/adapter-node"
                    | "@sveltejs/adapter-auto"
                    | "@builder.io/qwik-city/adaptors/static/vite"
                    | "@qwik.dev/router/adaptors/static/vite"
            )
        ),
    }
}

fn config_import<'a, 't>(
    tokens: &'t [ConfigToken<'a>],
    index: usize,
) -> Option<(&'t [ConfigToken<'a>], &'t ConfigToken<'a>)> {
    let (_, remaining) = tokens.get(index..)?.split_first()?;
    if matches!(
        remaining.first()?.text.as_bytes().first(),
        Some(b'\'' | b'"')
    ) {
        return Some((&[], &remaining[0]));
    }
    let from = remaining.windows(2).position(|pair| {
        matches!(
            (pair[0].text, pair[1].text.as_bytes().first()),
            ("from", Some(b'\'' | b'"'))
        )
    })?;
    let (clause, source) = remaining.split_at(from);
    let [_, source, ..] = source else {
        return None;
    };
    Some((clause, source))
}

fn verified_imports<'a>(
    tokens: &[ConfigToken<'a>],
    names: &[&str],
) -> Result<Vec<(&'a str, &'a str)>, ()> {
    let mut bindings = Vec::new();
    let mut cursor = tokens.iter().enumerate().peekable();
    while let Some((index, token)) = cursor.next() {
        if token.text == "import" && cursor.peek().is_some_and(|(_, token)| token.text != "(") {
            let (clause, source) = config_import(tokens, index).ok_or(())?;
            let source_module = &source.text[1..source.text.len() - 1];
            let mut locals = clause.iter().enumerate().peekable();
            while let Some((local_index, local)) = locals.next() {
                if !names.contains(&local.text)
                    || locals.peek().is_some_and(|(_, token)| token.text == "as")
                {
                    continue;
                }
                let original = if local_index > 0 && clause[local_index - 1].text == "as" {
                    local_index.checked_sub(2).ok_or(())?
                } else {
                    local_index
                };
                if clause.first().is_some_and(|token| token.text == "type")
                    || original > 0 && clause[original - 1].text == "type"
                {
                    return Err(());
                }
                let imported = if local_index == 0 {
                    "default"
                } else {
                    clause[original].text
                };
                if !is_identifier(local.text)
                    || !trusted_import_binding(local.text, imported, source_module)
                {
                    return Err(());
                }
                bindings.push((local.text, source_module));
            }
            cursor
                .find(|(_, token)| token.end == source.end)
                .ok_or(())?;
            continue;
        }
        if names.contains(&token.text) {
            return Err(());
        }
    }
    Ok(bindings)
}

fn has_untrusted_reference(tokens: &[ConfigToken<'_>], names: &[&str]) -> bool {
    verified_imports(tokens, names).is_err()
}

fn exported_object<'a, 't>(
    tokens: &'t [ConfigToken<'a>],
) -> Option<(&'t [ConfigToken<'a>], &'t [ConfigToken<'a>])> {
    let mut exports = Vec::new();
    let mut cursor = tokens.iter().enumerate();
    while let Some((index, token)) = cursor.next() {
        let standalone = index == 0
            || matches!(tokens[index - 1].text, ";" | "}")
            || matches!(
                tokens[index - 1].text.as_bytes().first(),
                Some(b'\'' | b'"')
            );
        if tokens[index].text == "export"
            && tokens
                .get(index + 1)
                .is_some_and(|token| token.text == "default")
        {
            if !standalone {
                return None;
            }
            exports.push((index, index + 2));
        }
        if let [owner, dot, property, assignment, ..] = &tokens[index..]
            && matches!(
                [owner.text, dot.text, property.text, assignment.text],
                ["module", ".", "exports", "="]
            )
        {
            if !standalone {
                return None;
            }
            exports.push((index, index + 4));
        }
        if matches!(token.text, "(" | "[" | "{") {
            let closing = closing_token(tokens, index)?;
            cursor.find(|(position, _)| *position == closing)?;
        }
    }
    let module_references = tokens
        .windows(3)
        .filter(|sequence| {
            matches!(
                [sequence[0].text, sequence[1].text, sequence[2].text],
                ["module", ".", "exports"]
            )
        })
        .count();
    if exports.len() != 1 || module_references > 1 {
        return None;
    }
    let (marker, start) = exports[0];
    let prefix = &tokens[..marker];
    let commonjs = tokens[marker].text == "module";
    let protected = if commonjs {
        &["defineConfig", "defineNuxtConfig", "module"][..]
    } else {
        &["defineConfig", "defineNuxtConfig"][..]
    };
    let imports = verified_imports(prefix, protected).ok()?;
    if commonjs && imports.iter().any(|(local, _)| *local == "module") {
        return None;
    }
    let mut expression = &tokens[start..];
    for _ in 0..expression.len() {
        if expression.last().is_none_or(|token| token.text != ";") {
            break;
        }
        expression = &expression[..expression.len() - 1];
    }
    // ponytail: no JS execution; unresolved configs or nesting above 128 require SSR.
    Some((object_expression(expression)?, prefix))
}

fn object_property<'a, 't>(
    tokens: &'t [ConfigToken<'a>],
    key: &str,
) -> Result<Option<&'t [ConfigToken<'a>]>, ()> {
    let mut found = None;
    let mut index = 0;
    for _ in 0..tokens.len() {
        if index >= tokens.len() {
            break;
        }
        let token = tokens[index];
        if token.text == "[" || token.text.contains('\\') {
            return Err(());
        }
        let name = if matches!(token.text.as_bytes().first(), Some(b'\'' | b'"')) {
            &token.text[1..token.text.len() - 1]
        } else if is_identifier(token.text)
            || token
                .text
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            token.text
        } else {
            return Err(());
        };
        index += 1;
        let has_colon = tokens.get(index).is_some_and(|token| token.text == ":");
        let value_start = if has_colon {
            index += 1;
            index
        } else if index == tokens.len() || tokens[index].text == "," {
            if name == key {
                return Err(());
            }
            index
        } else {
            return Err(());
        };
        for _ in 0..tokens.len() {
            if index >= tokens.len() || tokens[index].text == "," {
                break;
            }
            match tokens[index].text {
                "(" | "[" | "{" => index = closing_token(tokens, index).ok_or(())? + 1,
                ")" | "]" | "}" | ";" => return Err(()),
                _ => index += 1,
            }
        }
        if has_colon && value_start == index {
            return Err(());
        }
        if name == key {
            if found.is_some() || value_start == index {
                return Err(());
            }
            found = Some(&tokens[value_start..index]);
        }
        if tokens.get(index).is_none() {
            break;
        }
        index += 1;
    }
    Ok(found)
}

fn array_expressions<'a, 't>(tokens: &'t [ConfigToken<'a>]) -> Option<Vec<&'t [ConfigToken<'a>]>> {
    let tokens = without_type_assertions(tokens)?;
    let inner = group_contents(tokens, "[")?;
    let mut expressions = Vec::new();
    let mut start = 0;
    let mut cursor = inner.iter().enumerate();
    while let Some((index, token)) = cursor.next() {
        match token.text {
            "." if index == start => return None,
            "(" | "[" | "{" => {
                let closing = closing_token(inner, index)?;
                cursor.find(|(position, _)| *position == closing)?;
            }
            "," => {
                if start == index {
                    return None;
                }
                expressions.push(&inner[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    let tail = &inner[start..];
    if !tail.is_empty() {
        expressions.push(tail);
    }
    Some(expressions)
}

fn call_expression<'a, 't>(
    tokens: &'t [ConfigToken<'a>],
) -> Option<(&'a str, &'t [ConfigToken<'a>])> {
    let tokens = without_type_assertions(tokens)?;
    let (callee, arguments) = tokens.split_first()?;
    Some((callee.text, group_contents(arguments, "(")?))
}

fn imported_call<'a>(tokens: &[ConfigToken<'a>], prefix: &[ConfigToken<'a>]) -> Option<&'a str> {
    let (callee, _) = call_expression(tokens)?;
    let imports = verified_imports(prefix, &[callee]).ok()?;
    let [(_, source)] = imports.as_slice() else {
        return None;
    };
    Some(source)
}

fn object_path<'a, 't>(
    mut object: &'t [ConfigToken<'a>],
    path: &[&str],
) -> Result<Option<&'t [ConfigToken<'a>]>, ()> {
    for (index, key) in path.iter().enumerate() {
        let Some(value) = object_property(object, key)? else {
            return Ok(None);
        };
        if index + 1 == path.len() {
            return Ok(Some(value));
        }
        object = object_expression(value).ok_or(())?;
    }
    Err(())
}

fn config_adapter<'a>(content: &'a str, path: &[&str]) -> Option<&'a str> {
    let tokens = config_tokens(content)?;
    let (object, prefix) = exported_object(&tokens)?;
    imported_call(object_path(object, path).ok()??, prefix)
}

fn has_applied_qwik_static_adapter(content: &str) -> bool {
    let Some(tokens) = config_tokens(content) else {
        return false;
    };
    let Some((object, prefix)) = exported_object(&tokens) else {
        return false;
    };
    let Ok(Some(plugins)) = object_property(object, "plugins") else {
        return false;
    };
    let Some(plugins) = array_expressions(plugins) else {
        return false;
    };
    let mut found = false;
    for plugin in plugins {
        match imported_call(plugin, prefix) {
            Some(
                "@builder.io/qwik-city/adaptors/static/vite"
                | "@qwik.dev/router/adaptors/static/vite",
            ) => {
                if found {
                    return false;
                }
                found = true;
            }
            Some("@builder.io/qwik-city/vite" | "@qwik.dev/router/vite") => {}
            _ => return false,
        }
    }
    found
}

fn config_value<'a>(content: &'a str, path: &[&str], plugin: Option<&str>) -> ConfigValue<'a> {
    let Some(tokens) = config_tokens(content) else {
        return ConfigValue::Unknown;
    };
    let Some((mut object, prefix)) = exported_object(&tokens) else {
        return ConfigValue::Unknown;
    };
    if let Some(plugin) = plugin {
        if has_untrusted_reference(prefix, &[plugin]) {
            return ConfigValue::Unknown;
        }
        let Ok(Some(plugins)) = object_property(object, "plugins") else {
            return ConfigValue::Unknown;
        };
        let Some(plugins) = array_expressions(plugins) else {
            return ConfigValue::Unknown;
        };
        let mut found = None;
        for expression in plugins {
            if expression.first().is_some_and(|token| token.text == plugin) {
                if found.is_some() {
                    return ConfigValue::Unknown;
                }
                let Some((_, arguments)) = call_expression(expression) else {
                    return ConfigValue::Unknown;
                };
                let Some(config) = object_expression(arguments) else {
                    return ConfigValue::Unknown;
                };
                found = Some(config);
            }
        }
        let Some(config) = found else {
            return ConfigValue::Absent;
        };
        object = config;
    }
    match object_path(object, path) {
        Ok(Some(value)) => literal_value(value),
        Ok(None) => ConfigValue::Absent,
        Err(()) => ConfigValue::Unknown,
    }
}

fn contains_any_pattern(content: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| content.contains(pattern))
}

fn next_app_walk_for_content(fs: &dyn Fs, needle: &str) -> bool {
    ["app", "src/app"]
        .iter()
        .any(|dir| fs.is_dir(dir) && walk_for_content(fs, dir, needle))
}

/// Check for Next.js route handlers (app/**/route.{ts,js}, src/app/**/route.{ts,js}).
fn has_route_handlers(fs: &dyn Fs) -> bool {
    ["app", "src/app"]
        .iter()
        .any(|dir| fs.is_dir(dir) && walk_for_file(fs, dir, &["route.ts", "route.js"]))
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
