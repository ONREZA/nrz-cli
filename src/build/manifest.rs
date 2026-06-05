use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Context;
use regex::Regex;
use serde::{Deserialize, Serialize};

// ── Spec limits ───────────────────────────────────────────────
const MAX_LAYERS: usize = 10;
const MAX_ROUTES: usize = 200;
const MAX_NAME_LEN: usize = 64;
const MAX_DIRECTORY_LEN: usize = 256;
const MAX_ENTRY_LEN: usize = 512;
const MAX_META_BYTES: usize = 16_384;
const MAX_REVALIDATE_SECS: u64 = 31_536_000; // 1 year in seconds

const VALID_HTTP_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

/// Custom deserializer for optional runtime integer fields.
/// Provides clear error messages for fractional or negative values
/// instead of serde's default "invalid type: floating point" message.
fn de_u32_positive<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let v: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match v {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(n)) => n
            .as_u64()
            .and_then(|u| u32::try_from(u).ok())
            .map(Some)
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "must be a non-negative integer \
                     (got negative, fractional, or out-of-range value)",
                )
            }),
        Some(_) => Err(serde::de::Error::custom("expected a non-negative integer")),
    }
}

/// Validated deployment manifest (.onreza/manifest.json).
#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub version: u32,
    pub layers: Vec<Layer>,
    pub routes: Vec<Route>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerender: Option<PrerenderConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middleware: Option<Vec<Middleware>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub name: String,
    pub target: LayerTarget,
    pub directory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    #[serde(rename = "export", skip_serializing_if = "Option::is_none")]
    pub export_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<RuntimeConfig>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LayerTarget {
    Static,
    Compute,
}

impl std::fmt::Display for LayerTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static => write!(f, "STATIC"),
            Self::Compute => write!(f, "COMPUTE"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfig {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_u32_positive"
    )]
    pub timeout_ms: Option<u32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_u32_positive"
    )]
    pub memory_mb: Option<u32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_u32_positive"
    )]
    pub max_concurrency: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Route {
    pub pattern: String,
    pub layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revalidate: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub methods: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallthrough: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PrerenderConfig {
    pub layer: String,
    pub pages: std::collections::HashMap<String, PrerenderPage>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PrerenderPage {
    pub html: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Middleware {
    pub name: String,
    pub bundle_path: String,
    pub code_hash: String,
    pub matchers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

/// Returns the primary layer target implied by this manifest's layers.
///
/// Used to bridge the manifest-based model with the legacy `compute_type` API contract.
/// Priority: COMPUTE > STATIC.
pub fn primary_compute_target(manifest: &Manifest) -> LayerTarget {
    if manifest
        .layers
        .iter()
        .any(|l| l.target == LayerTarget::Compute)
    {
        LayerTarget::Compute
    } else {
        LayerTarget::Static
    }
}

fn has_path_traversal(s: &str) -> bool {
    if s.contains('\0') {
        return true;
    }
    let lower = s.to_ascii_lowercase();
    // Any URL-encoded dot (%2e) or double-encoded dot (%252e) is suspicious
    if lower.contains("%2e") || lower.contains("%252e") {
        return true;
    }
    s.replace('\\', "/").split('/').any(|seg| seg == "..")
}

static JS_ONLY_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

/// Returns `true` if `pattern` contains JS-only regex features
/// (lookahead, lookbehind, or backreferences) unsupported by the Rust `regex` crate.
/// These patterns must be rejected to prevent silent runtime failures on the edge server.
fn has_js_only_regex_features(pattern: &str) -> bool {
    JS_ONLY_REGEX
        .get_or_init(|| Regex::new(r"\(\?[=!]|\(\?<[=!]|\\[1-9]").unwrap())
        .is_match(pattern)
}

pub fn load_and_validate(path: &Path) -> anyhow::Result<Manifest> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let manifest: Manifest = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    validate(&manifest)?;
    Ok(manifest)
}

pub fn validate(manifest: &Manifest) -> anyhow::Result<()> {
    if manifest.version != 1 {
        anyhow::bail!(
            "unsupported manifest version: {}. Expected 1.",
            manifest.version
        );
    }

    // ── Layers ────────────────────────────────────────────────

    if manifest.layers.is_empty() {
        anyhow::bail!("at least one layer is required");
    }
    if manifest.layers.len() > MAX_LAYERS {
        anyhow::bail!(
            "too many layers: {} (max {})",
            manifest.layers.len(),
            MAX_LAYERS
        );
    }

    // Duplicate layer names
    let mut seen: HashSet<&str> = HashSet::new();
    for layer in &manifest.layers {
        if !seen.insert(layer.name.as_str()) {
            anyhow::bail!("duplicate layer name: '{}'", layer.name);
        }
    }

    // Per-layer rules
    for layer in &manifest.layers {
        if layer.name.is_empty() {
            anyhow::bail!("layer name must not be empty");
        }
        if layer.name.chars().count() > MAX_NAME_LEN {
            anyhow::bail!(
                "layer name exceeds {} chars: '{}'",
                MAX_NAME_LEN,
                layer.name
            );
        }
        if layer.directory.is_empty() {
            anyhow::bail!("layer '{}' directory must not be empty", layer.name);
        }
        if layer.directory.chars().count() > MAX_DIRECTORY_LEN {
            anyhow::bail!(
                "layer '{}' directory path exceeds {} chars",
                layer.name,
                MAX_DIRECTORY_LEN
            );
        }
        if layer.directory.starts_with('/') || has_path_traversal(&layer.directory) {
            anyhow::bail!(
                "path traversal in layer '{}' directory: '{}'",
                layer.name,
                layer.directory
            );
        }

        match layer.target {
            LayerTarget::Static => {
                if layer.entry.is_some() {
                    anyhow::bail!("STATIC layer '{}' must not have 'entry'", layer.name);
                }
                if layer.export_format.is_some() {
                    anyhow::bail!("STATIC layer '{}' must not have 'export'", layer.name);
                }
                if layer.runtime.is_some() {
                    anyhow::bail!("STATIC layer '{}' must not have 'runtime'", layer.name);
                }
            }
            LayerTarget::Compute => {
                let entry = layer.entry.as_deref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "layer '{}' (target={}) requires 'entry'",
                        layer.name,
                        layer.target
                    )
                })?;
                if entry.is_empty() {
                    anyhow::bail!("layer '{}' entry must not be empty", layer.name);
                }
                if entry.chars().count() > MAX_ENTRY_LEN {
                    anyhow::bail!(
                        "layer '{}' entry path exceeds {} chars",
                        layer.name,
                        MAX_ENTRY_LEN
                    );
                }
                if entry.starts_with('/') || has_path_traversal(entry) {
                    anyhow::bail!(
                        "path traversal in layer '{}' entry: '{}'",
                        layer.name,
                        entry
                    );
                }
                if layer.export_format.is_some() {
                    anyhow::bail!(
                        "{} layer '{}' must not have 'export'",
                        layer.target,
                        layer.name
                    );
                }
            }
        }

        // Runtime config rules
        if let Some(ref rt) = layer.runtime {
            if rt.timeout_ms == Some(0) {
                anyhow::bail!("layer '{}' runtime.timeoutMs must be positive", layer.name);
            }
            if rt.memory_mb == Some(0) {
                anyhow::bail!("layer '{}' runtime.memoryMb must be positive", layer.name);
            }
            if rt.max_concurrency == Some(0) {
                anyhow::bail!(
                    "layer '{}' runtime.maxConcurrency must be positive",
                    layer.name
                );
            }
        }
    }

    // ── Routes ────────────────────────────────────────────────

    if manifest.routes.is_empty() {
        anyhow::bail!("at least one route is required");
    }
    if manifest.routes.len() > MAX_ROUTES {
        anyhow::bail!(
            "too many routes: {} (max {})",
            manifest.routes.len(),
            MAX_ROUTES
        );
    }

    let layer_targets: HashMap<&str, LayerTarget> = manifest
        .layers
        .iter()
        .map(|l| (l.name.as_str(), l.target))
        .collect();

    let mut route_pattern_layers: HashSet<(&str, &str)> = HashSet::new();
    let mut terminal_patterns: HashMap<&str, &str> = HashMap::new();
    for route in &manifest.routes {
        if !route.pattern.starts_with("^/") {
            anyhow::bail!("route pattern must start with '^/': '{}'", route.pattern);
        }
        if route.pattern.chars().count() > 500 {
            let prefix: String = route.pattern.chars().take(50).collect();
            anyhow::bail!("route pattern exceeds 500 chars: '{prefix}...'");
        }
        if has_js_only_regex_features(&route.pattern) {
            anyhow::bail!(
                "route pattern uses JS-only regex features \
                 (lookahead, lookbehind, or backreferences): '{}'",
                route.pattern
            );
        }
        if let Err(e) = Regex::new(&route.pattern) {
            anyhow::bail!("invalid regex in route pattern '{}': {}", route.pattern, e);
        }
        if !route_pattern_layers.insert((route.pattern.as_str(), route.layer.as_str())) {
            anyhow::bail!(
                "duplicate route pattern+layer: '{}' -> '{}'",
                route.pattern,
                route.layer
            );
        }

        let layer_target = match layer_targets.get(route.layer.as_str()).copied() {
            Some(target) => target,
            None => anyhow::bail!("route references unknown layer: '{}'", route.layer),
        };

        if layer_target != LayerTarget::Static
            && let Some(existing_layer) =
                terminal_patterns.insert(route.pattern.as_str(), route.layer.as_str())
        {
            anyhow::bail!(
                "route pattern '{}' has multiple terminal (non-STATIC) layers: '{}' and '{}'. \
                 The second route is unreachable because COMPUTE routes never fallthrough.",
                route.pattern,
                existing_layer,
                route.layer
            );
        }

        if let Some(methods) = &route.methods {
            for method in methods {
                if !VALID_HTTP_METHODS.contains(&method.as_str()) {
                    anyhow::bail!(
                        "invalid HTTP method '{}' in route '{}': must be one of {}",
                        method,
                        route.pattern,
                        VALID_HTTP_METHODS.join(", ")
                    );
                }
            }
        }
        if let Some(revalidate) = route.revalidate {
            if layer_target == LayerTarget::Static {
                anyhow::bail!(
                    "ISR revalidate not applicable to STATIC layer '{}'",
                    route.layer
                );
            }
            if revalidate == 0 {
                anyhow::bail!("ISR revalidate must be positive: {}", revalidate);
            }
            if revalidate > MAX_REVALIDATE_SECS {
                anyhow::bail!(
                    "ISR revalidate exceeds maximum of {} seconds (got {})",
                    MAX_REVALIDATE_SECS,
                    revalidate
                );
            }
        }
    }

    if manifest.middleware.is_some() {
        anyhow::bail!(
            "manifest middleware is no longer supported; use a *.nrz-fn.* entry with export const config and a middleware trigger"
        );
    }

    // ── Meta ──────────────────────────────────────────────────

    if let Some(ref meta) = manifest.meta {
        // serde_json outputs non-ASCII as raw UTF-8 bytes (no \uXXXX escaping),
        // so s.len() (byte count) matches JS `TextEncoder(JSON.stringify(meta)).length`.
        let size = serde_json::to_string(meta)
            .map(|s| s.len())
            .unwrap_or(usize::MAX);
        if size > MAX_META_BYTES {
            anyhow::bail!("meta exceeds {} bytes (got {})", MAX_META_BYTES, size);
        }
    }

    // ── Prerender ─────────────────────────────────────────────

    if let Some(prerender) = &manifest.prerender {
        let prerender_layer_target = match layer_targets.get(prerender.layer.as_str()).copied() {
            Some(target) => target,
            None => anyhow::bail!("prerender references unknown layer: '{}'", prerender.layer),
        };
        if prerender_layer_target != LayerTarget::Static {
            anyhow::bail!("prerender layer '{}' must be STATIC", prerender.layer);
        }
        // Page keys must start with '/' and paths must not contain traversal
        for (page_key, page) in &prerender.pages {
            if !page_key.starts_with('/') {
                anyhow::bail!("prerender page key must start with '/': '{}'", page_key);
            }
            if page.html.starts_with('/') || has_path_traversal(&page.html) {
                anyhow::bail!(
                    "path traversal in prerender page '{}' html: '{}'",
                    page_key,
                    page.html
                );
            }
            if let Some(ref data) = page.data
                && (data.starts_with('/') || has_path_traversal(data))
            {
                anyhow::bail!(
                    "path traversal in prerender page '{}' data: '{}'",
                    page_key,
                    data
                );
            }
        }
    }

    Ok(())
}

/// Auto-generate a minimal STATIC manifest for plain static deploys.
pub fn generate_static_manifest() -> Manifest {
    Manifest {
        version: 1,
        layers: vec![Layer {
            name: "site".to_string(),
            target: LayerTarget::Static,
            directory: ".".to_string(),
            entry: None,
            export_format: None,
            runtime: None,
        }],
        routes: vec![Route {
            pattern: "^/.*$".to_string(),
            layer: "site".to_string(),
            priority: Some(0),
            revalidate: None,
            methods: None,
            headers: None,
            fallthrough: None,
        }],
        prerender: None,
        middleware: None,
        meta: None,
    }
}

/// Auto-generate a minimal COMPUTE manifest for PROCESS deploys.
/// `entry` — resolved entry point relative to output dir (e.g. "server.js").
pub fn generate_compute_manifest(entry: &str) -> Manifest {
    Manifest {
        version: 1,
        layers: vec![Layer {
            name: "server".to_string(),
            target: LayerTarget::Compute,
            directory: ".".to_string(),
            entry: Some(entry.to_string()),
            export_format: None,
            runtime: None,
        }],
        routes: vec![Route {
            pattern: "^/.*$".to_string(),
            layer: "server".to_string(),
            priority: Some(0),
            revalidate: None,
            methods: None,
            headers: None,
            fallthrough: None,
        }],
        prerender: None,
        middleware: None,
        meta: None,
    }
}

/// Auto-generate a Next.js standalone manifest with STATIC + COMPUTE layers.
///
/// - `_static/` layer serves `/_next/static/*` via CDN with correct URL nesting
/// - `public/` layer (optional) serves root-level assets; lower-priority server route handles misses
/// - COMPUTE layer runs `server.js`
///
/// Auto-generate a Next.js standalone manifest for a server entry that may live
/// below the standalone bundle root in monorepos.
///
/// Routes use priority-based routing: static(100) > public(50) > server(0).
pub fn generate_nextjs_standalone_manifest_for_server(
    has_public: bool,
    server_dir: &str,
    server_entry: &str,
) -> Manifest {
    let static_dir = join_manifest_path(server_dir, "_static");
    let public_dir = join_manifest_path(server_dir, "public");

    let mut layers = vec![Layer {
        name: "static-assets".to_string(),
        target: LayerTarget::Static,
        directory: static_dir,
        entry: None,
        export_format: None,
        runtime: None,
    }];

    if has_public {
        layers.push(Layer {
            name: "public-assets".to_string(),
            target: LayerTarget::Static,
            directory: public_dir,
            entry: None,
            export_format: None,
            runtime: None,
        });
    }

    layers.push(Layer {
        name: "server".to_string(),
        target: LayerTarget::Compute,
        directory: ".".to_string(),
        entry: Some(server_entry.to_string()),
        export_format: None,
        runtime: None,
    });

    let mut routes = vec![Route {
        pattern: "^/_next/static/.*$".to_string(),
        layer: "static-assets".to_string(),
        priority: Some(100),
        revalidate: None,
        methods: None,
        headers: None,
        fallthrough: None,
    }];

    if has_public {
        routes.push(Route {
            pattern: "^/.*$".to_string(),
            layer: "public-assets".to_string(),
            priority: Some(50),
            revalidate: None,
            methods: None,
            headers: None,
            fallthrough: None,
        });
    }

    routes.push(Route {
        pattern: "^/.*$".to_string(),
        layer: "server".to_string(),
        priority: Some(0),
        revalidate: None,
        methods: None,
        headers: None,
        fallthrough: None,
    });

    Manifest {
        version: 1,
        layers,
        routes,
        prerender: None,
        middleware: None,
        meta: None,
    }
}

fn join_manifest_path(prefix: &str, path: &str) -> String {
    let prefix = prefix.trim_matches('/');
    let path = path.trim_matches('/');
    if prefix.is_empty() {
        path.to_string()
    } else if path.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{path}")
    }
}

/// Configuration for generating an SSR manifest with optional STATIC + COMPUTE layers.
struct SsrManifestConfig {
    /// Static assets directory (e.g. "client", "public"). `None` to skip the static layer.
    static_dir: Option<&'static str>,
    /// CDN route pattern for hashed/immutable assets (e.g. "^/_nuxt/.*$").
    static_route_pattern: Option<&'static str>,
    /// Server/compute directory (e.g. "server", ".").
    server_dir: &'static str,
    /// Server entry file (e.g. "index.mjs", "index.js").
    server_entry: &'static str,
}

/// Build an SSR manifest from the given configuration.
///
/// Produces a two-layer manifest (STATIC + COMPUTE) when `static_dir` is set,
/// or a single COMPUTE layer otherwise. The catch-all `/*` route always falls
/// through to the server at priority 0.
fn generate_ssr_manifest(config: SsrManifestConfig) -> Manifest {
    let mut layers = Vec::new();
    let mut routes = Vec::new();

    if let (Some(dir), Some(pattern)) = (config.static_dir, config.static_route_pattern) {
        layers.push(Layer {
            name: "static-assets".to_string(),
            target: LayerTarget::Static,
            directory: dir.to_string(),
            entry: None,
            export_format: None,
            runtime: None,
        });
        routes.push(Route {
            pattern: pattern.to_string(),
            layer: "static-assets".to_string(),
            priority: Some(100),
            revalidate: None,
            methods: None,
            headers: None,
            fallthrough: None,
        });
    }

    layers.push(Layer {
        name: "server".to_string(),
        target: LayerTarget::Compute,
        directory: config.server_dir.to_string(),
        entry: Some(config.server_entry.to_string()),
        export_format: None,
        runtime: None,
    });
    routes.push(Route {
        pattern: "^/.*$".to_string(),
        layer: "server".to_string(),
        priority: Some(0),
        revalidate: None,
        methods: None,
        headers: None,
        fallthrough: None,
    });

    Manifest {
        version: 1,
        layers,
        routes,
        prerender: None,
        middleware: None,
        meta: None,
    }
}

/// Nuxt `.output/`: `public/` (static, `_nuxt/*` CDN) + `server/index.mjs` (compute).
pub fn generate_nuxt_manifest(has_public: bool) -> Manifest {
    generate_ssr_manifest(SsrManifestConfig {
        static_dir: if has_public { Some("public") } else { None },
        static_route_pattern: if has_public {
            Some("^/_nuxt/.*$")
        } else {
            None
        },
        server_dir: "server",
        server_entry: "index.mjs",
    })
}

/// SvelteKit `build/`: `client/` (static, `_app/*` CDN) + `./index.js` (compute).
pub fn generate_sveltekit_manifest(has_client: bool) -> Manifest {
    generate_ssr_manifest(SsrManifestConfig {
        static_dir: if has_client { Some("client") } else { None },
        static_route_pattern: if has_client { Some("^/_app/.*$") } else { None },
        server_dir: ".",
        server_entry: "index.js",
    })
}

/// Remix / React Router v7 `build/`: `client/` (static, `assets/*` CDN) + `server/index.js` (compute).
pub fn generate_remix_manifest(has_client: bool) -> Manifest {
    generate_ssr_manifest(SsrManifestConfig {
        static_dir: if has_client { Some("client") } else { None },
        static_route_pattern: if has_client {
            Some("^/assets/.*$")
        } else {
            None
        },
        server_dir: "server",
        server_entry: "index.js",
    })
}

/// Astro SSR `dist/`: `client/` (static, `_astro/*` CDN) + `server/entry.mjs` (compute).
pub fn generate_astro_ssr_manifest(has_client: bool) -> Manifest {
    generate_ssr_manifest(SsrManifestConfig {
        static_dir: if has_client { Some("client") } else { None },
        static_route_pattern: if has_client {
            Some("^/_astro/.*$")
        } else {
            None
        },
        server_dir: "server",
        server_entry: "entry.mjs",
    })
}

pub fn verify_files(output_dir: &Path, manifest: &Manifest) -> anyhow::Result<()> {
    for layer in &manifest.layers {
        let layer_dir = output_dir.join(&layer.directory);
        if !layer_dir.is_dir() {
            anyhow::bail!(
                "layer directory not found: '{}' (layer: '{}')",
                layer.directory,
                layer.name
            );
        }
        if let Some(entry) = &layer.entry {
            let entry_path = layer_dir.join(entry);
            if !entry_path.is_file() {
                anyhow::bail!(
                    "entry not found: '{}/{}' (layer: '{}')",
                    layer.directory,
                    entry,
                    layer.name
                );
            }
        }
    }

    if let Some(prerender) = &manifest.prerender {
        let prerender_layer = manifest
            .layers
            .iter()
            .find(|l| l.name == prerender.layer)
            .ok_or_else(|| {
                anyhow::anyhow!("prerender references unknown layer: '{}'", prerender.layer)
            })?;
        let prerender_dir = output_dir.join(&prerender_layer.directory);
        for (page_path, page) in &prerender.pages {
            let html_file = prerender_dir.join(&page.html);
            if !html_file.is_file() {
                anyhow::bail!(
                    "prerender page '{}' html not found: '{}/{}' (layer: '{}')",
                    page_path,
                    prerender_layer.directory,
                    page.html,
                    prerender.layer
                );
            }
            if let Some(ref data) = page.data {
                let data_file = prerender_dir.join(data);
                if !data_file.is_file() {
                    anyhow::bail!(
                        "prerender page '{}' data not found: '{}/{}' (layer: '{}')",
                        page_path,
                        prerender_layer.directory,
                        data,
                        prerender.layer
                    );
                }
            }
        }
    }

    Ok(())
}
