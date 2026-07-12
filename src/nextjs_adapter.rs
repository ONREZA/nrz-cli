use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use regex::Regex;
use serde::Deserialize;

const ADAPTER_SOURCE: &str = include_str!("../assets/next-adapter/onreza-next-adapter.cjs");
const ADAPTER_FILE_NAME: &str = "onreza-next-adapter.cjs";
const ADAPTER_OUTPUT_RELATIVE_PATH: &str = ".onreza/next-adapter-output.json";
const ADAPTER_CACHE_RELATIVE_DIR: &str = ".onreza/cache/next-adapter";
const NEXT_CACHE_SUBSTRATE_SCHEMA_VERSION: &str = "NEXT_CACHE_SUBSTRATE_V1";
const ONREZA_IMAGE_OPTIMIZER_PATH: &str = "/_onreza/image";
const ONREZA_IMAGE_LOADER_RELATIVE_PATH: &str =
    "./.onreza/cache/next-adapter/onreza-image-loader.mjs";
const MIN_ADAPTER_VERSION: (u64, u64, u64) = (16, 2, 0);

#[derive(Debug)]
pub(crate) struct BuildAdapter {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterDescriptor {
    pub version: u32,
    pub adapter: AdapterInfo,
    pub next_version: Option<String>,
    pub build_id: Option<String>,
    #[serde(default)]
    pub config: AdapterConfig,
    #[serde(default)]
    pub routing: AdapterRouting,
    #[serde(default)]
    pub outputs: AdapterOutputs,
    #[serde(default)]
    pub deployment_hints: AdapterDeploymentHints,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdapterInfo {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterConfig {
    pub base_path: Option<String>,
    pub i18n: Option<serde_json::Value>,
    pub images: Option<serde_json::Value>,
    pub trailing_slash: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterOutputs {
    #[serde(default)]
    pub pages: Vec<RuntimeOutput>,
    #[serde(default)]
    pub pages_api: Vec<RuntimeOutput>,
    #[serde(default)]
    pub app_pages: Vec<RuntimeOutput>,
    #[serde(default)]
    pub app_routes: Vec<RuntimeOutput>,
    #[serde(default)]
    pub prerenders: Vec<PrerenderOutput>,
    #[serde(default)]
    pub static_files: Vec<StaticFileOutput>,
    pub middleware: Option<RuntimeOutput>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterDeploymentHints {
    pub image_optimizer: Option<serde_json::Value>,
    #[serde(default)]
    pub middleware: AdapterMiddlewareDeploymentHints,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterMiddlewareDeploymentHints {
    #[serde(default)]
    pub static_files: AdapterPathnamePartition,
    #[serde(default)]
    pub public_files: AdapterPathnamePartition,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterPathnamePartition {
    #[serde(default)]
    pub safe_for_static_layer: Vec<String>,
    #[serde(default)]
    pub requires_compute: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterRouting {
    #[serde(default)]
    pub before_middleware: Vec<AdapterRoute>,
    #[serde(default)]
    pub before_files: Vec<AdapterRoute>,
    #[serde(default)]
    pub after_files: Vec<AdapterRoute>,
    #[serde(default)]
    pub dynamic_routes: Vec<AdapterRoute>,
    #[serde(default)]
    pub on_match: Vec<AdapterRoute>,
    #[serde(default)]
    pub fallback: Vec<AdapterRoute>,
    pub should_normalize_next_data: Option<bool>,
    pub rsc: Option<serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterRoute {
    pub source: Option<String>,
    pub source_regex: Option<String>,
    pub destination: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub has: Option<serde_json::Value>,
    pub missing: Option<serde_json::Value>,
    pub status: Option<u16>,
    pub priority: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StaticFileOutput {
    pub file_path: String,
    pub pathname: String,
    pub immutable_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeOutput {
    #[serde(rename = "type")]
    pub output_type: Option<String>,
    pub id: Option<String>,
    pub file_path: Option<String>,
    pub pathname: Option<String>,
    pub source_page: Option<String>,
    pub runtime: Option<String>,
    pub edge_runtime: Option<serde_json::Value>,
    #[serde(default)]
    pub assets: BTreeMap<String, String>,
    #[serde(default)]
    pub wasm_assets: BTreeMap<String, String>,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrerenderOutput {
    pub id: Option<String>,
    pub pathname: Option<String>,
    pub parent_output_id: Option<String>,
    pub group_id: Option<u64>,
    pub parent_fallback_mode: Option<serde_json::Value>,
    pub fallback: Option<PrerenderFallback>,
    pub ppr_chain: Option<serde_json::Value>,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrerenderFallback {
    pub file_path: Option<String>,
    pub initial_status: Option<u16>,
    #[serde(default)]
    pub initial_headers: BTreeMap<String, serde_json::Value>,
    pub initial_expiration: Option<u64>,
    pub initial_revalidate: Option<serde_json::Value>,
    pub postponed_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaticFileMapping {
    pub source: PathBuf,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaticPrerenderMapping {
    pub pathname: String,
    pub source: PathBuf,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdapterOutputCounts {
    pub pages: usize,
    pub pages_api: usize,
    pub app_pages: usize,
    pub app_routes: usize,
    pub prerenders: usize,
    pub static_files: usize,
    pub middleware: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdapterRoutingCounts {
    pub before_middleware: usize,
    pub before_files: usize,
    pub after_files: usize,
    pub dynamic_routes: usize,
    pub on_match: usize,
    pub fallback: usize,
    pub redirects: usize,
    pub rewrites: usize,
    pub header_rules: usize,
    pub priority_rules: usize,
    pub source_rules: usize,
    pub source_regex_rules: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdapterEdgeRuleLoweringCounts {
    pub generated: usize,
    pub unsupported: usize,
}

#[derive(Debug)]
struct NextSourceCapture {
    name: String,
}

pub(crate) fn prepare_build_adapter(project_dir: &Path) -> anyhow::Result<Option<BuildAdapter>> {
    if !supports_next_adapter_api(project_dir) {
        return Ok(None);
    }

    let adapter_dir = project_dir
        .join(ADAPTER_CACHE_RELATIVE_DIR)
        .join(env!("CARGO_PKG_VERSION"));
    std::fs::create_dir_all(&adapter_dir)
        .with_context(|| format!("failed to create {}", adapter_dir.display()))?;

    let adapter_path = adapter_dir.join(ADAPTER_FILE_NAME);
    match std::fs::read_to_string(&adapter_path) {
        Ok(existing) if existing == ADAPTER_SOURCE => {}
        _ => std::fs::write(&adapter_path, ADAPTER_SOURCE)
            .with_context(|| format!("failed to write {}", adapter_path.display()))?,
    }

    Ok(Some(BuildAdapter { path: adapter_path }))
}

pub(crate) fn descriptor_path(project_dir: &Path) -> PathBuf {
    project_dir.join(ADAPTER_OUTPUT_RELATIVE_PATH)
}

pub(crate) fn clear_descriptor(project_dir: &Path) -> anyhow::Result<()> {
    let path = descriptor_path(project_dir);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(anyhow::Error::new(err))
            .with_context(|| format!("failed to remove stale {}", path.display())),
    }
}

pub(crate) fn load_descriptor(project_dir: &Path) -> anyhow::Result<Option<AdapterDescriptor>> {
    let path = descriptor_path(project_dir);
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(anyhow::Error::new(err))
                .with_context(|| format!("failed to read {}", path.display()));
        }
    };

    let descriptor = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(descriptor))
}

impl AdapterDescriptor {
    pub(crate) fn static_file_mappings_for_static_layer(
        &self,
        project_dir: &Path,
    ) -> anyhow::Result<Vec<StaticFileMapping>> {
        self.outputs
            .static_files
            .iter()
            .filter(|file| self.pathname_safe_for_static_file_layer(&file.pathname))
            .map(|file| file.static_file_mapping(project_dir))
            .collect()
    }

    pub(crate) fn static_prerender_mappings_for_static_layer(
        &self,
        project_dir: &Path,
    ) -> anyhow::Result<Vec<StaticPrerenderMapping>> {
        self.outputs
            .prerenders
            .iter()
            .filter(|output| {
                output.static_html_pathname().is_some_and(|pathname| {
                    self.pathname_safe_for_prerender_layer(pathname)
                        && !self.routing.has_exact_redirect_for_pathname(pathname)
                })
            })
            .map(|output| output.static_prerender_mapping(project_dir, self))
            .collect()
    }

    pub(crate) fn output_counts(&self) -> AdapterOutputCounts {
        AdapterOutputCounts {
            pages: self.outputs.pages.len(),
            pages_api: self.outputs.pages_api.len(),
            app_pages: self.outputs.app_pages.len(),
            app_routes: self.outputs.app_routes.len(),
            prerenders: self.outputs.prerenders.len(),
            static_files: self.outputs.static_files.len(),
            middleware: usize::from(self.outputs.middleware.is_some()),
        }
    }

    pub(crate) fn has_middleware(&self) -> bool {
        self.outputs.middleware.is_some()
    }

    pub(crate) fn pathname_safe_for_static_file_layer(&self, pathname: &str) -> bool {
        self.pathname_safe_for_static_layer(
            pathname,
            &self.deployment_hints.middleware.static_files,
        )
    }

    pub(crate) fn pathname_safe_for_public_layer(&self, pathname: &str) -> bool {
        self.pathname_safe_for_static_layer(
            pathname,
            &self.deployment_hints.middleware.public_files,
        )
    }

    fn pathname_safe_for_prerender_layer(&self, pathname: &str) -> bool {
        if !self.has_middleware() {
            return true;
        }
        !self.middleware_may_match_path(pathname)
    }

    fn static_prerender_served_pathname<'a>(&self, pathname: &'a str) -> std::borrow::Cow<'a, str> {
        if self.config.trailing_slash != Some(true)
            || pathname == "/"
            || pathname.ends_with('/')
            || pathname
                .rsplit('/')
                .next()
                .is_some_and(|segment| segment.contains('.'))
        {
            return std::borrow::Cow::Borrowed(pathname);
        }
        std::borrow::Cow::Owned(format!("{pathname}/"))
    }

    fn pathname_safe_for_static_layer(
        &self,
        pathname: &str,
        hints: &AdapterPathnamePartition,
    ) -> bool {
        if !self.has_middleware() {
            return true;
        }
        if hints
            .requires_compute
            .iter()
            .any(|candidate| candidate == pathname)
        {
            return false;
        }
        if hints
            .safe_for_static_layer
            .iter()
            .any(|candidate| candidate == pathname)
        {
            return true;
        }
        !self.middleware_may_match_path(pathname)
    }

    pub(crate) fn middleware_may_match_path(&self, pathname: &str) -> bool {
        let Some(middleware) = &self.outputs.middleware else {
            return false;
        };
        let Some(matchers) = middleware
            .config
            .get("matchers")
            .and_then(|value| value.as_array())
        else {
            return true;
        };
        if matchers.is_empty() {
            return true;
        }

        for matcher in matchers {
            let Some(source_regex) = matcher
                .get("sourceRegex")
                .and_then(serde_json::Value::as_str)
            else {
                return true;
            };
            let source_regex = normalize_next_source_regex_for_rust(source_regex);
            let Ok(regex) = Regex::new(&source_regex) else {
                return true;
            };
            if regex.is_match(pathname) {
                return true;
            }
        }
        false
    }

    pub(crate) fn compatibility_summary(&self) -> serde_json::Value {
        let counts = self.output_counts();
        let routing = self.routing_counts();
        let edge_rule_lowering = self.edge_rule_lowering_counts();
        let edge_runtime_outputs = self.outputs.edge_runtime_output_count();
        let middleware_matchers = self.middleware_matcher_count();
        let ppr_prerenders = self.outputs.ppr_prerender_count();
        let isr_prerenders = self.outputs.isr_prerender_count();
        let immutable_static_files = self.outputs.immutable_static_file_count();
        let named_runtime_outputs = self.outputs.named_runtime_output_count();
        let typed_runtime_outputs = self.outputs.typed_runtime_output_count();
        let file_runtime_outputs = self.outputs.file_runtime_output_count();
        let prerender_fallback_files = self.outputs.prerender_fallback_file_count();
        let route_handler_report = self.outputs.route_handler_platform_report();
        let prerender_routes = self.prerender_route_reports();
        let next_cache_report = self.next_cache_candidate_report();
        let image_optimizer_report = self.image_optimizer_platform_report();
        let static_prerenders = self
            .outputs
            .prerenders
            .iter()
            .filter(|output| {
                output
                    .static_html_pathname()
                    .is_some_and(|pathname| self.pathname_safe_for_prerender_layer(pathname))
            })
            .count();
        let middleware = self.has_middleware();
        let safe_static_files = self
            .outputs
            .static_files
            .iter()
            .filter(|file| self.pathname_safe_for_static_file_layer(&file.pathname))
            .count();
        serde_json::json!({
            "outputs": {
                "pages": counts.pages,
                "pagesApi": counts.pages_api,
                "appPages": counts.app_pages,
                "appRoutes": counts.app_routes,
                "prerenders": counts.prerenders,
                "staticFiles": counts.static_files,
                "immutableStaticFiles": immutable_static_files,
                "middleware": counts.middleware,
                "runtimeOutputsWithPathname": named_runtime_outputs,
                "runtimeOutputsWithType": typed_runtime_outputs,
                "runtimeOutputsWithFilePath": file_runtime_outputs,
            },
            "routing": {
                "beforeMiddleware": routing.before_middleware,
                "beforeFiles": routing.before_files,
                "afterFiles": routing.after_files,
                "dynamicRoutes": routing.dynamic_routes,
                "onMatch": routing.on_match,
                "fallback": routing.fallback,
                "shouldNormalizeNextData": self.routing.should_normalize_next_data.unwrap_or(false),
                "hasRscRouting": self.routing.rsc.is_some(),
                "redirects": routing.redirects,
                "rewrites": routing.rewrites,
                "headerRules": routing.header_rules,
                "priorityRules": routing.priority_rules,
                "sourceRules": routing.source_rules,
                "sourceRegexRules": routing.source_regex_rules,
            },
            "platform": {
                "staticFiles": {
                    "status": if !middleware {
                        "supported"
                    } else if safe_static_files > 0 {
                        "guarded_static_split"
                    } else {
                        "compute_fallback"
                    },
                    "count": counts.static_files,
                    "staticLayerCount": safe_static_files,
                    "immutableCount": immutable_static_files,
                    "primitive": "STATIC layer with fallthrough to COMPUTE",
                    "reason": static_files_status_reason(middleware, safe_static_files, counts.static_files),
                },
                "nodeRuntime": {
                    "status": "supported",
                    "primitive": "COMPUTE layer",
                },
                "prerenders": {
                    "status": prerender_platform_status(counts.prerenders, static_prerenders, isr_prerenders, ppr_prerenders),
                    "count": counts.prerenders,
                    "staticLayerCount": static_prerenders,
                    "isrCount": isr_prerenders,
                    "pprCount": ppr_prerenders,
                    "fallbackFileCount": prerender_fallback_files,
                    "primitive": "STATIC prerender layer with fallthrough to COMPUTE",
                    "reason": prerender_status_reason(counts.prerenders, static_prerenders, isr_prerenders, ppr_prerenders, middleware),
                    "routes": prerender_routes,
                },
                "nextCache": next_cache_report,
                "imageOptimizer": image_optimizer_report,
                "routeHandlers": route_handler_report,
                "middleware": {
                    "status": self.middleware_platform_status(),
                    "count": counts.middleware,
                    "id": self.outputs.middleware.as_ref().and_then(|middleware| middleware.id.as_deref()),
                    "sourcePage": self.outputs.middleware.as_ref().and_then(|middleware| middleware.source_page.as_deref()),
                    "filePath": self.outputs.middleware.as_ref().and_then(|middleware| middleware.file_path.as_deref()),
                    "runtime": self.outputs.middleware.as_ref().and_then(|middleware| middleware.runtime.as_deref()),
                    "kind": self.middleware_kind(),
                    "matcherCount": middleware_matchers,
                    "assetCount": self.outputs.middleware.as_ref().map_or(0, |middleware| middleware.assets.len()),
                    "wasmAssetCount": self.outputs.middleware.as_ref().map_or(0, |middleware| middleware.wasm_assets.len()),
                    "edgeRuntime": self.outputs.middleware.as_ref().is_some_and(|middleware| middleware.is_edge_runtime()),
                    "primitive": "COMPUTE layer",
                    "reason": self.middleware_platform_reason(),
                },
                "routing": {
                    "status": routing_platform_status(&routing, &edge_rule_lowering),
                    "primitive": "Edge Rules",
                    "edgeRulesGenerated": edge_rule_lowering.generated,
                    "edgeRulesUnsupported": edge_rule_lowering.unsupported,
                },
                "edgeRuntime": {
                    "status": if edge_runtime_outputs == 0 { "absent" } else { "compute_fallback" },
                    "count": edge_runtime_outputs,
                    "primitive": "COMPUTE layer",
                    "reason": if edge_runtime_outputs == 0 {
                        serde_json::Value::Null
                    } else {
                        serde_json::Value::String(
                            "Next.js edge runtime outputs are multi-chunk framework artifacts; ONREZA Functions v1 only accepts self-contained function sources".to_string(),
                        )
                    },
                },
            },
            "config": {
                "basePath": self.config.base_path.as_deref().filter(|value| !value.is_empty()),
                "i18n": self.config.i18n.as_ref(),
                "images": self.image_config_summary(),
                "trailingSlash": self.config.trailing_slash,
            },
        })
    }

    pub(crate) fn manifest_compatibility_summary(&self) -> serde_json::Value {
        let summary = self.compatibility_summary();
        serde_json::json!({
            "outputs": compact_summary_object(
                &summary,
                "/outputs",
                &[
                    "pages",
                    "pagesApi",
                    "appPages",
                    "appRoutes",
                    "prerenders",
                    "staticFiles",
                    "immutableStaticFiles",
                    "middleware",
                    "runtimeOutputsWithPathname",
                    "runtimeOutputsWithType",
                    "runtimeOutputsWithFilePath",
                ],
            ),
            "routing": compact_summary_object(
                &summary,
                "/routing",
                &[
                    "beforeMiddleware",
                    "beforeFiles",
                    "afterFiles",
                    "dynamicRoutes",
                    "onMatch",
                    "fallback",
                    "shouldNormalizeNextData",
                    "hasRscRouting",
                    "redirects",
                    "rewrites",
                    "headerRules",
                    "priorityRules",
                    "sourceRules",
                    "sourceRegexRules",
                ],
            ),
            "platform": {
                "staticFiles": compact_summary_object(
                    &summary,
                    "/platform/staticFiles",
                    &["status", "count", "staticLayerCount", "immutableCount", "primitive", "reason"],
                ),
                "nodeRuntime": compact_summary_object(
                    &summary,
                    "/platform/nodeRuntime",
                    &["status", "primitive"],
                ),
                "prerenders": compact_summary_object(
                    &summary,
                    "/platform/prerenders",
                    &[
                        "status",
                        "count",
                        "staticLayerCount",
                        "isrCount",
                        "pprCount",
                        "fallbackFileCount",
                        "primitive",
                        "reason",
                    ],
                ),
                "nextCache": compact_summary_object(
                    &summary,
                    "/platform/nextCache",
                    &[
                        "status",
                        "mode",
                        "schemaVersion",
                        "routeCount",
                        "isrCandidateCount",
                        "isrBlockedCount",
                        "pprBlockedCount",
                        "producer",
                        "nextVersion",
                        "buildId",
                    ],
                ),
                "imageOptimizer": compact_summary_object(
                    &summary,
                    "/platform/imageOptimizer",
                    &["status", "mode", "path", "primitive", "reason"],
                ),
                "routeHandlers": compact_summary_object(
                    &summary,
                    "/platform/routeHandlers",
                    &[
                        "status",
                        "count",
                        "pagesApi",
                        "appRoutes",
                        "nodejs",
                        "edgeRuntime",
                        "assetBacked",
                        "missingFilePath",
                        "selfContainedShape",
                        "primitive",
                        "functionsV1",
                    ],
                ),
                "middleware": compact_summary_object(
                    &summary,
                    "/platform/middleware",
                    &[
                        "status",
                        "count",
                        "kind",
                        "matcherCount",
                        "assetCount",
                        "wasmAssetCount",
                        "edgeRuntime",
                        "primitive",
                        "reason",
                    ],
                ),
                "routing": compact_summary_object(
                    &summary,
                    "/platform/routing",
                    &["status", "primitive", "edgeRulesGenerated", "edgeRulesUnsupported"],
                ),
                "edgeRuntime": compact_summary_object(
                    &summary,
                    "/platform/edgeRuntime",
                    &["status", "count", "primitive", "reason"],
                ),
            },
            "config": compact_summary_object(
                &summary,
                "/config",
                &["basePath", "i18n", "images", "trailingSlash"],
            ),
        })
    }

    pub(crate) fn compatibility_report_line(&self) -> String {
        format_nextjs_adapter_report(&self.compatibility_summary())
    }

    fn image_config_object(&self) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.config
            .images
            .as_ref()
            .and_then(serde_json::Value::as_object)
    }

    fn image_config_str(&self, field: &str) -> Option<&str> {
        self.image_config_object()?
            .get(field)
            .and_then(serde_json::Value::as_str)
    }

    fn image_config_bool(&self, field: &str) -> Option<bool> {
        self.image_config_object()?
            .get(field)
            .and_then(serde_json::Value::as_bool)
    }

    fn image_config_summary(&self) -> serde_json::Value {
        let Some(images) = self.image_config_object() else {
            return serde_json::Value::Null;
        };

        let mut summary = serde_json::Map::new();
        for field in [
            "loader",
            "loaderFile",
            "path",
            "unoptimized",
            "domains",
            "remotePatterns",
            "localPatterns",
            "formats",
        ] {
            if let Some(value) = images.get(field) {
                summary.insert(field.to_string(), value.clone());
            }
        }
        serde_json::Value::Object(summary)
    }

    fn image_config_uses_onreza_optimizer(&self) -> bool {
        self.image_config_str("loader") == Some("custom")
            && self.image_config_str("loaderFile") == Some(ONREZA_IMAGE_LOADER_RELATIVE_PATH)
    }

    fn image_config_path_is_default(&self, path: &str) -> bool {
        let path = path.trim_end_matches('/');
        if path == "/_next/image" {
            return true;
        }
        let Some(base_path) = self.config.base_path.as_deref() else {
            return false;
        };
        if base_path.is_empty() || base_path == "/" {
            return false;
        }
        path == format!("{base_path}/_next/image")
    }

    fn image_optimizer_platform_report(&self) -> serde_json::Value {
        if let Some(hint) = self
            .deployment_hints
            .image_optimizer
            .as_ref()
            .and_then(serde_json::Value::as_object)
        {
            return serde_json::Value::Object(hint.clone());
        }

        if self.image_config_uses_onreza_optimizer() {
            return serde_json::json!({
                "status": "onreza_optimizer",
                "mode": "custom_loader",
                "path": ONREZA_IMAGE_OPTIMIZER_PATH,
                "primitive": "ONREZA image optimizer",
                "reason": serde_json::Value::Null,
            });
        }

        if self.image_config_bool("unoptimized") == Some(true) {
            return serde_json::json!({
                "status": "disabled",
                "primitive": "Next.js image config",
                "reason": "images.unoptimized is enabled",
            });
        }

        let user_configured = self
            .image_config_str("loader")
            .is_some_and(|loader| loader != "default")
            || self
                .image_config_str("loaderFile")
                .is_some_and(|loader_file| !loader_file.is_empty())
            || self
                .image_config_str("path")
                .is_some_and(|path| !self.image_config_path_is_default(path));

        if user_configured {
            return serde_json::json!({
                "status": "user_configured",
                "primitive": "user image config",
                "reason": "images.loader, images.loaderFile, or images.path is user-configured",
            });
        }

        serde_json::json!({
            "status": "compute_fallback",
            "primitive": "COMPUTE layer",
            "reason": "Next.js image optimizer remains on the framework runtime",
        })
    }

    fn prerender_route_reports(&self) -> Vec<serde_json::Value> {
        self.outputs
            .prerenders
            .iter()
            .map(|output| output.platform_route_report(self))
            .collect()
    }

    fn next_cache_candidate_report(&self) -> serde_json::Value {
        let routes = self
            .outputs
            .prerenders
            .iter()
            .filter_map(|output| output.next_cache_route_report(self))
            .collect::<Vec<_>>();
        let isr_candidate_count = routes
            .iter()
            .filter(|route| {
                route.get("status").and_then(serde_json::Value::as_str)
                    == Some("edge_cache_candidate")
            })
            .count();
        let isr_blocked_count = routes
            .iter()
            .filter(|route| {
                route.get("kind").and_then(serde_json::Value::as_str) == Some("isr")
                    && route.get("status").and_then(serde_json::Value::as_str)
                        != Some("edge_cache_candidate")
            })
            .count();
        let ppr_blocked_count = routes
            .iter()
            .filter(|route| route.get("kind").and_then(serde_json::Value::as_str) == Some("ppr"))
            .count();

        serde_json::json!({
            "schemaVersion": NEXT_CACHE_SUBSTRATE_SCHEMA_VERSION,
            "status": next_cache_platform_status(routes.len(), isr_candidate_count),
            "mode": "report_only",
            "producer": "nextjs-adapter",
            "nextVersion": self.next_version.as_deref(),
            "buildId": self.build_id.as_deref(),
            "routeCount": routes.len(),
            "isrCandidateCount": isr_candidate_count,
            "isrBlockedCount": isr_blocked_count,
            "pprBlockedCount": ppr_blocked_count,
            "routes": routes,
        })
    }

    fn middleware_matcher_count(&self) -> usize {
        self.outputs
            .middleware
            .as_ref()
            .and_then(|middleware| {
                middleware
                    .config
                    .get("matchers")
                    .and_then(|value| value.as_array())
            })
            .map_or(0, Vec::len)
    }

    fn middleware_kind(&self) -> &'static str {
        let Some(middleware) = &self.outputs.middleware else {
            return "absent";
        };
        if middleware.runtime.as_deref() == Some("nodejs") {
            return "next_proxy_nodejs";
        }
        if middleware.is_edge_runtime() {
            return "next_middleware_edge";
        }
        "next_middleware_unknown_runtime"
    }

    fn middleware_platform_status(&self) -> &'static str {
        match self.middleware_kind() {
            "absent" => "absent",
            "next_proxy_nodejs" => "compute_fallback_nodejs_proxy",
            "next_middleware_edge" => "compute_fallback_edge_runtime",
            _ => "compute_fallback_unknown_runtime",
        }
    }

    fn middleware_platform_reason(&self) -> serde_json::Value {
        let reason = match self.middleware_kind() {
            "absent" => return serde_json::Value::Null,
            "next_proxy_nodejs" => {
                "Next.js proxy is a Node.js runtime artifact and stays in the standalone COMPUTE server"
            }
            "next_middleware_edge" => {
                "Next.js edge middleware is emitted as a multi-chunk framework bundle; ONREZA Functions v1 only accepts self-contained function sources"
            }
            _ => {
                "Next.js middleware runtime is not recognized by nrz; keeping it in the standalone COMPUTE server"
            }
        };
        serde_json::Value::String(reason.to_string())
    }

    pub(crate) fn routing_counts(&self) -> AdapterRoutingCounts {
        self.routing.counts()
    }

    pub(crate) fn generated_edge_rules(&self) -> Option<serde_json::Value> {
        let rules = self.generated_edge_rule_items();
        if rules.is_empty() {
            return None;
        }
        Some(serde_json::json!({
            "schemaVersion": "EDGE_RULE_SET_V1",
            "rules": rules,
        }))
    }

    pub(crate) fn edge_rule_lowering_counts(&self) -> AdapterEdgeRuleLoweringCounts {
        let mut generated = 0usize;
        let mut unsupported = 0usize;
        for (bucket, index, route) in self.routing.indexed_routes() {
            if !route.has_effect() {
                continue;
            }
            if route.to_edge_rule(bucket, index).is_some() {
                generated += 1;
            } else {
                unsupported += 1;
            }
        }
        AdapterEdgeRuleLoweringCounts {
            generated,
            unsupported,
        }
    }

    fn generated_edge_rule_items(&self) -> Vec<serde_json::Value> {
        self.routing
            .indexed_routes()
            .filter_map(|(bucket, index, route)| route.to_edge_rule(bucket, index))
            .collect()
    }
}

impl AdapterRoutingCounts {
    fn has_effects(&self) -> bool {
        self.redirects > 0 || self.rewrites > 0 || self.header_rules > 0
    }
}

fn routing_platform_status(
    routing: &AdapterRoutingCounts,
    edge_rule_lowering: &AdapterEdgeRuleLoweringCounts,
) -> &'static str {
    if !routing.has_effects() {
        return "absent";
    }
    if edge_rule_lowering.generated == 0 {
        return "pending_edge_rules";
    }
    if edge_rule_lowering.unsupported == 0 {
        return "edge_rules_generated";
    }
    "partial_edge_rules"
}

impl AdapterRouting {
    fn counts(&self) -> AdapterRoutingCounts {
        let routes = self.routes().collect::<Vec<_>>();
        AdapterRoutingCounts {
            before_middleware: self.before_middleware.len(),
            before_files: self.before_files.len(),
            after_files: self.after_files.len(),
            dynamic_routes: self.dynamic_routes.len(),
            on_match: self.on_match.len(),
            fallback: self.fallback.len(),
            redirects: routes.iter().filter(|route| route.is_redirect()).count(),
            rewrites: routes.iter().filter(|route| route.is_rewrite()).count(),
            header_rules: routes
                .iter()
                .filter(|route| route.has_headers() && !route.is_redirect())
                .count(),
            priority_rules: routes.iter().filter(|route| route.is_priority()).count(),
            source_rules: routes.iter().filter(|route| route.source.is_some()).count(),
            source_regex_rules: routes
                .iter()
                .filter(|route| route.source_regex.is_some())
                .count(),
        }
    }

    fn routes(&self) -> impl Iterator<Item = &AdapterRoute> {
        self.before_middleware
            .iter()
            .chain(self.before_files.iter())
            .chain(self.after_files.iter())
            .chain(self.dynamic_routes.iter())
            .chain(self.on_match.iter())
            .chain(self.fallback.iter())
    }

    fn has_exact_redirect_for_pathname(&self, pathname: &str) -> bool {
        self.routes()
            .any(|route| route.is_redirect() && route.source.as_deref() == Some(pathname))
    }

    fn indexed_routes(&self) -> impl Iterator<Item = (&'static str, usize, &AdapterRoute)> {
        self.before_middleware
            .iter()
            .enumerate()
            .map(|(index, route)| ("beforeMiddleware", index, route))
            .chain(
                self.before_files
                    .iter()
                    .enumerate()
                    .map(|(index, route)| ("beforeFiles", index, route)),
            )
            .chain(
                self.after_files
                    .iter()
                    .enumerate()
                    .map(|(index, route)| ("afterFiles", index, route)),
            )
            .chain(
                self.dynamic_routes
                    .iter()
                    .enumerate()
                    .map(|(index, route)| ("dynamicRoutes", index, route)),
            )
            .chain(
                self.on_match
                    .iter()
                    .enumerate()
                    .map(|(index, route)| ("onMatch", index, route)),
            )
            .chain(
                self.fallback
                    .iter()
                    .enumerate()
                    .map(|(index, route)| ("fallback", index, route)),
            )
    }
}

impl AdapterRoute {
    fn to_edge_rule(&self, bucket: &'static str, index: usize) -> Option<serde_json::Value> {
        let (condition_path, captures) = self.edge_rule_path_condition()?;
        let mut condition = self.edge_rule_request_condition()?;
        condition.insert("path".to_string(), condition_path);
        let action = self.edge_rule_action(&captures)?;
        let kind = action.get("type")?.as_str()?;
        Some(serde_json::json!({
            "id": format!("next.{kind}.{bucket}.{index}"),
            "condition": condition,
            "action": action,
        }))
    }

    fn edge_rule_path_condition(&self) -> Option<(serde_json::Value, Vec<NextSourceCapture>)> {
        if let Some(source) = self.source.as_deref() {
            return next_source_to_edge_path_condition(source);
        }

        if let Some(source_regex) = self.source_regex.as_deref() {
            return next_source_regex_to_edge_path_condition(source_regex);
        }

        None
    }

    fn edge_rule_action(&self, captures: &[NextSourceCapture]) -> Option<serde_json::Value> {
        if self.is_redirect() {
            if self.headers.len() != 1 {
                return None;
            }
            let status = self.status?;
            if !matches!(status, 301 | 302 | 307 | 308) {
                return None;
            }
            let target = rewrite_next_target(self.location_header()?, captures)?;
            return Some(serde_json::json!({
                "type": "redirect",
                "target": target,
                "statusCode": status,
            }));
        }

        if self.is_rewrite() {
            if self.has_headers() || self.status.is_some() {
                return None;
            }
            let target = rewrite_next_target(self.destination.as_deref()?, captures)?;
            let external = target.starts_with("http://") || target.starts_with("https://");
            return Some(serde_json::json!({
                "type": "rewrite",
                "target": target,
                "external": external,
            }));
        }

        if self.has_headers() {
            if self.status.is_some() {
                return None;
            }
            return Some(serde_json::json!({
                "type": "set_headers",
                "headers": self.headers,
            }));
        }

        None
    }

    fn has_effect(&self) -> bool {
        self.is_redirect() || self.is_rewrite() || self.has_headers()
    }

    fn edge_rule_request_condition(&self) -> Option<serde_json::Map<String, serde_json::Value>> {
        let mut condition = serde_json::Map::new();
        lower_next_route_conditions(self.has.as_ref(), &mut condition)?;

        if value_is_present(&self.missing) {
            let mut not = serde_json::Map::new();
            lower_next_route_conditions(self.missing.as_ref(), &mut not)?;
            if !not.is_empty() {
                condition.insert("not".to_string(), serde_json::Value::Object(not));
            }
        }

        Some(condition)
    }

    fn is_redirect(&self) -> bool {
        self.status.is_some() && self.location_header().is_some()
    }

    fn is_rewrite(&self) -> bool {
        self.destination.is_some()
    }

    fn has_headers(&self) -> bool {
        !self.headers.is_empty()
    }

    fn is_priority(&self) -> bool {
        self.priority == Some(true)
    }

    fn location_header(&self) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case("location"))
            .map(|(_, value)| value.as_str())
    }
}

fn value_is_present(value: &Option<serde_json::Value>) -> bool {
    match value {
        None | Some(serde_json::Value::Null) => false,
        Some(serde_json::Value::Array(items)) => !items.is_empty(),
        Some(_) => true,
    }
}

fn next_source_to_edge_path_condition(
    source: &str,
) -> Option<(serde_json::Value, Vec<NextSourceCapture>)> {
    if !source.starts_with('/') || source.contains(['?', '#', '(', ')', '$']) {
        return None;
    }

    if !source.contains(':') && !source.contains(['*', '+', '[', ']', '{', '}']) {
        return Some((
            serde_json::json!({ "type": "exact", "value": source }),
            Vec::new(),
        ));
    }

    let trailing_slash = source.len() > 1 && source.ends_with('/');
    let source = if trailing_slash {
        source.trim_end_matches('/')
    } else {
        source
    };
    let mut captures: Vec<NextSourceCapture> = Vec::new();
    let mut glob_segments = Vec::new();
    let parts = source
        .trim_start_matches('/')
        .split('/')
        .collect::<Vec<_>>();
    if parts.iter().any(|part| part.is_empty()) {
        return None;
    }

    for (index, part) in parts.iter().enumerate() {
        if let Some(param) = part.strip_prefix(':') {
            let is_last = index + 1 == parts.len();
            let (name, splat) = parse_next_source_param(param)?;
            if splat && !is_last {
                return None;
            }
            if captures.iter().any(|capture| capture.name == name) {
                return None;
            }
            captures.push(NextSourceCapture {
                name: name.to_string(),
            });
            if splat {
                glob_segments.push(format!("{{{name}...}}"));
            } else {
                glob_segments.push(format!("{{{name}}}"));
            }
            continue;
        }

        if !literal_source_segment_supported(part) {
            return None;
        }
        glob_segments.push((*part).to_string());
    }

    let mut value = format!("/{}", glob_segments.join("/"));
    if trailing_slash {
        value.push('/');
    }
    Some((
        serde_json::json!({
            "type": "glob",
            "value": value,
        }),
        captures,
    ))
}

fn next_source_regex_to_edge_path_condition(
    source_regex: &str,
) -> Option<(serde_json::Value, Vec<NextSourceCapture>)> {
    if next_source_regex_targets_next_static(source_regex) {
        return Some((
            serde_json::json!({
                "type": "glob",
                "value": "/_next/static/{path...}",
            }),
            Vec::new(),
        ));
    }

    let normalized = normalize_next_source_regex_for_rust(source_regex);
    let source = normalized.strip_prefix('^')?.strip_suffix('$')?;
    if source.is_empty() || !source.starts_with('/') {
        return None;
    }
    if source.contains([
        '\\', '^', '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '<', '>',
    ]) {
        return None;
    }

    Some((
        serde_json::json!({ "type": "exact", "value": source }),
        Vec::new(),
    ))
}

fn lower_next_route_conditions(
    value: Option<&serde_json::Value>,
    condition: &mut serde_json::Map<String, serde_json::Value>,
) -> Option<()> {
    let Some(value) = value else {
        return Some(());
    };
    match value {
        serde_json::Value::Null => return Some(()),
        serde_json::Value::Array(items) if items.is_empty() => return Some(()),
        serde_json::Value::Array(items) => {
            for item in items {
                lower_next_route_condition(item, condition)?;
            }
        }
        _ => return None,
    }
    Some(())
}

fn lower_next_route_condition(
    value: &serde_json::Value,
    condition: &mut serde_json::Map<String, serde_json::Value>,
) -> Option<()> {
    let object = value.as_object()?;
    let kind = object.get("type")?.as_str()?;
    let raw_value = object.get("value")?.as_str()?;
    let value = next_route_condition_literal_value(raw_value)?;

    match kind {
        "header" => {
            insert_condition_map_value(condition, "headers", object.get("key")?.as_str()?, value)
        }
        "cookie" => {
            insert_condition_map_value(condition, "cookies", object.get("key")?.as_str()?, value)
        }
        "query" => {
            insert_condition_map_value(condition, "query", object.get("key")?.as_str()?, value)
        }
        "host" => {
            if condition.contains_key("host") || value.is_empty() {
                return None;
            }
            condition.insert(
                "host".to_string(),
                serde_json::Value::String(value.to_string()),
            );
            Some(())
        }
        _ => None,
    }
}

fn insert_condition_map_value(
    condition: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
    key: &str,
    value: &str,
) -> Option<()> {
    if key.is_empty() {
        return None;
    }
    let entry = condition
        .entry(field.to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let map = entry.as_object_mut()?;
    if map.contains_key(key) {
        return None;
    }
    map.insert(
        key.to_string(),
        serde_json::Value::String(value.to_string()),
    );
    Some(())
}

fn next_route_condition_literal_value(value: &str) -> Option<&str> {
    if value.contains([
        '\\', '^', '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '<', '>',
    ]) {
        return None;
    }
    Some(value)
}

fn parse_next_source_param(param: &str) -> Option<(&str, bool)> {
    let (name, splat) = if let Some(name) = param.strip_suffix('*') {
        (name, true)
    } else if let Some(name) = param.strip_suffix('+') {
        (name, true)
    } else {
        (param, false)
    };

    if !valid_capture_name(name) {
        return None;
    }
    Some((name, splat))
}

fn literal_source_segment_supported(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.contains([':', '*', '+', '(', ')', '[', ']', '{', '}', '?', '#', '$'])
}

fn valid_capture_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn rewrite_next_target(target: &str, captures: &[NextSourceCapture]) -> Option<String> {
    let param = Regex::new(r":([A-Za-z][A-Za-z0-9_]*)([*+]?)").ok()?;
    let mut supported = true;
    let rewritten = param.replace_all(target, |captures_match: &regex::Captures<'_>| {
        let name = captures_match.get(1).expect("capture exists").as_str();
        if !captures.iter().any(|capture| capture.name == name) {
            supported = false;
        }
        format!("{{{name}}}")
    });
    if !supported {
        return None;
    }

    let numeric_capture = Regex::new(r"\$(\d+)").ok()?;
    let mut supported = true;
    let rewritten =
        numeric_capture.replace_all(&rewritten, |capture_match: &regex::Captures<'_>| {
            let raw_index = capture_match.get(1).expect("capture exists").as_str();
            let Some(index) = raw_index.parse::<usize>().ok().filter(|index| *index > 0) else {
                supported = false;
                return String::new();
            };
            let Some(capture) = captures.get(index - 1) else {
                supported = false;
                return String::new();
            };
            format!("{{{}}}", capture.name)
        });
    if !supported || rewritten.contains('$') {
        return None;
    }

    Some(rewritten.into_owned())
}

fn static_files_status_reason(
    middleware: bool,
    safe_static_files: usize,
    total_static_files: usize,
) -> serde_json::Value {
    if !middleware {
        return serde_json::Value::Null;
    }
    if safe_static_files == 0 {
        return serde_json::Value::String(
            "Next.js middleware may run before static/public assets".to_string(),
        );
    }
    if safe_static_files == total_static_files {
        return serde_json::Value::String(
            "all adapter static file pathnames are disjoint from Next.js middleware matchers"
                .to_string(),
        );
    }
    serde_json::Value::String(
        "only pathnames disjoint from Next.js middleware matchers are staged into STATIC"
            .to_string(),
    )
}

fn prerender_platform_status(
    total: usize,
    static_prerenders: usize,
    isr_prerenders: usize,
    ppr_prerenders: usize,
) -> &'static str {
    if total == 0 {
        return "absent";
    }
    if static_prerenders == 0 {
        return "compute_fallback";
    }
    if static_prerenders == total && isr_prerenders == 0 && ppr_prerenders == 0 {
        return "supported";
    }
    "partial_static_split"
}

fn prerender_status_reason(
    total: usize,
    static_prerenders: usize,
    isr_prerenders: usize,
    ppr_prerenders: usize,
    middleware: bool,
) -> serde_json::Value {
    if total == 0 {
        return serde_json::Value::Null;
    }
    if static_prerenders == 0 && middleware {
        return serde_json::Value::String(
            "Next.js middleware may run before prerendered pages".to_string(),
        );
    }
    if static_prerenders == 0 {
        return serde_json::Value::String(
            "no fully-static HTML prerender fallback files were found".to_string(),
        );
    }
    if isr_prerenders > 0 || ppr_prerenders > 0 {
        return serde_json::Value::String(
            "fully-static HTML prerenders are staged into STATIC; ISR/PPR stay in the standalone COMPUTE server".to_string(),
        );
    }
    serde_json::Value::Null
}

fn route_handler_platform_status(total: usize) -> &'static str {
    if total == 0 {
        "absent"
    } else {
        "compute_fallback"
    }
}

fn next_cache_platform_status(total: usize, isr_candidate_count: usize) -> &'static str {
    if total == 0 {
        "absent"
    } else if isr_candidate_count > 0 {
        "report_only_candidates"
    } else {
        "report_only_blocked"
    }
}

fn next_source_regex_targets_next_static(source_regex: &str) -> bool {
    let normalized = normalize_next_source_regex_for_rust(source_regex);
    let trimmed = normalized.strip_prefix('^').unwrap_or(&normalized);
    trimmed.starts_with("/_next/static/")
}

fn normalize_next_source_regex_for_rust(source_regex: &str) -> String {
    source_regex.replace(r"\/", "/")
}

fn compact_summary_object(
    source: &serde_json::Value,
    object_path: &str,
    fields: &[&str],
) -> serde_json::Value {
    let Some(source_object) = source
        .pointer(object_path)
        .and_then(serde_json::Value::as_object)
    else {
        return serde_json::Value::Object(serde_json::Map::new());
    };
    let mut compact = serde_json::Map::new();
    for field in fields {
        if let Some(value) = source_object.get(*field) {
            compact.insert((*field).to_string(), value.clone());
        }
    }
    serde_json::Value::Object(compact)
}

pub(crate) fn format_nextjs_adapter_report(summary: &serde_json::Value) -> String {
    let static_status = summary
        .pointer("/platform/staticFiles/status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let static_count = summary
        .pointer("/platform/staticFiles/staticLayerCount")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let prerender_status = summary
        .pointer("/platform/prerenders/status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let prerender_count = summary
        .pointer("/platform/prerenders/staticLayerCount")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let isr_count = summary
        .pointer("/platform/prerenders/isrCount")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let ppr_count = summary
        .pointer("/platform/prerenders/pprCount")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let route_handler_status = summary
        .pointer("/platform/routeHandlers/status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let route_handler_count = summary
        .pointer("/platform/routeHandlers/count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let next_cache_status = summary
        .pointer("/platform/nextCache/status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let next_cache_isr_candidates = summary
        .pointer("/platform/nextCache/isrCandidateCount")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let next_cache_isr_blocked = summary
        .pointer("/platform/nextCache/isrBlockedCount")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let routing_status = summary
        .pointer("/platform/routing/status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let edge_rules = summary
        .pointer("/platform/routing/edgeRulesGenerated")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let unsupported = summary
        .pointer("/platform/routing/edgeRulesUnsupported")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let middleware_status = summary
        .pointer("/platform/middleware/status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let image_optimizer_status = summary
        .pointer("/platform/imageOptimizer/status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let edge_runtime_count = summary
        .pointer("/platform/edgeRuntime/count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    format!(
        "Next.js adapter report: STATIC {static_count} ({static_status}), prerenders {prerender_count} static/{isr_count} ISR/{ppr_count} PPR ({prerender_status}), ISR cache {next_cache_isr_candidates} candidates/{next_cache_isr_blocked} blocked ({next_cache_status}), image optimizer {image_optimizer_status}, route handlers {route_handler_count} ({route_handler_status}), Edge Rules {edge_rules} generated/{unsupported} unsupported ({routing_status}), middleware {middleware_status}, edge runtime outputs {edge_runtime_count}"
    )
}

impl AdapterOutputs {
    fn route_handler_outputs(&self) -> impl Iterator<Item = (&'static str, &RuntimeOutput)> {
        self.pages_api
            .iter()
            .map(|output| ("PAGES_API", output))
            .chain(self.app_routes.iter().map(|output| ("APP_ROUTE", output)))
            .filter(|(_, output)| !output.is_route_handler_internal_artifact())
    }

    fn route_handler_platform_report(&self) -> serde_json::Value {
        let route_handlers = self.route_handler_outputs().collect::<Vec<_>>();
        let count = route_handlers.len();
        let edge_runtime = route_handlers
            .iter()
            .filter(|(_, output)| output.is_edge_runtime())
            .count();
        let nodejs = route_handlers
            .iter()
            .filter(|(_, output)| output.runtime.as_deref() == Some("nodejs"))
            .count();
        let asset_backed = route_handlers
            .iter()
            .filter(|(_, output)| output.has_assets())
            .count();
        let missing_file_path = route_handlers
            .iter()
            .filter(|(_, output)| output.file_path.is_none())
            .count();
        let self_contained_shape = route_handlers
            .iter()
            .filter(|(_, output)| output.has_functions_v1_shape())
            .count();
        let routes = route_handlers
            .iter()
            .map(|(kind, output)| output.route_handler_report(kind))
            .collect::<Vec<_>>();

        serde_json::json!({
            "status": route_handler_platform_status(count),
            "count": count,
            "pagesApi": route_handlers.iter().filter(|(kind, _)| *kind == "PAGES_API").count(),
            "appRoutes": route_handlers.iter().filter(|(kind, _)| *kind == "APP_ROUTE").count(),
            "nodejs": nodejs,
            "edgeRuntime": edge_runtime,
            "assetBacked": asset_backed,
            "missingFilePath": missing_file_path,
            "selfContainedShape": self_contained_shape,
            "primitive": "COMPUTE layer",
            "functionsV1": {
                "status": if count == 0 { "absent" } else { "blocked_framework_bundle_contract" },
                "primitive": "ONREZA Functions",
                "reason": if count == 0 {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::String(
                        "Next.js route handlers are emitted as framework artifacts with Next.js invocation/cache semantics; ONREZA Functions v1 accepts self-contained source files only".to_string(),
                    )
                },
            },
            "routes": routes,
        })
    }

    fn runtime_outputs(&self) -> impl Iterator<Item = &RuntimeOutput> {
        self.pages
            .iter()
            .chain(self.pages_api.iter())
            .chain(self.app_pages.iter())
            .chain(self.app_routes.iter())
            .chain(self.middleware.iter())
    }

    fn edge_runtime_output_count(&self) -> usize {
        self.runtime_outputs()
            .filter(|output| output.is_edge_runtime())
            .count()
    }

    fn named_runtime_output_count(&self) -> usize {
        self.runtime_outputs()
            .filter(|output| output.pathname.is_some())
            .count()
    }

    fn typed_runtime_output_count(&self) -> usize {
        self.runtime_outputs()
            .filter(|output| output.output_type.is_some())
            .count()
    }

    fn file_runtime_output_count(&self) -> usize {
        self.runtime_outputs()
            .filter(|output| output.file_path.is_some())
            .count()
    }

    fn immutable_static_file_count(&self) -> usize {
        self.static_files
            .iter()
            .filter(|output| output.immutable_hash.is_some())
            .count()
    }

    fn ppr_prerender_count(&self) -> usize {
        self.prerenders
            .iter()
            .filter(|output| output.is_ppr())
            .count()
    }

    fn isr_prerender_count(&self) -> usize {
        self.prerenders
            .iter()
            .filter(|output| output.is_isr())
            .count()
    }

    fn prerender_fallback_file_count(&self) -> usize {
        self.prerenders
            .iter()
            .filter(|output| output.has_fallback_file())
            .count()
    }
}

impl RuntimeOutput {
    fn is_edge_runtime(&self) -> bool {
        self.runtime.as_deref() == Some("edge") || self.edge_runtime.is_some()
    }

    fn has_assets(&self) -> bool {
        !self.assets.is_empty() || !self.wasm_assets.is_empty()
    }

    fn has_functions_v1_shape(&self) -> bool {
        self.runtime.as_deref() == Some("nodejs")
            && !self.is_edge_runtime()
            && !self.has_assets()
            && self.file_path.is_some()
    }

    fn is_route_handler_internal_artifact(&self) -> bool {
        self.pathname
            .as_deref()
            .is_some_and(next_cache_internal_artifact_pathname)
    }

    fn route_handler_report(&self, kind: &'static str) -> serde_json::Value {
        let status = self.route_handler_status();
        serde_json::json!({
            "type": self.output_type.as_deref().unwrap_or(kind),
            "id": self.id.as_deref(),
            "pathname": self.pathname.as_deref(),
            "sourcePage": self.source_page.as_deref(),
            "runtime": self.runtime.as_deref(),
            "edgeRuntime": self.is_edge_runtime(),
            "assetCount": self.assets.len(),
            "wasmAssetCount": self.wasm_assets.len(),
            "status": status,
            "primitive": "COMPUTE layer",
            "functionsV1": {
                "status": "not_supported",
                "selfContainedShape": self.has_functions_v1_shape(),
                "reason": self.route_handler_functions_reason(status),
            },
        })
    }

    fn route_handler_status(&self) -> &'static str {
        if self.is_edge_runtime() {
            return "compute_fallback_edge_runtime";
        }
        if self.file_path.is_none() {
            return "compute_fallback_missing_file";
        }
        if self.has_assets() {
            return "compute_fallback_framework_assets";
        }
        "compute_fallback_framework_artifact"
    }

    fn route_handler_functions_reason(&self, status: &str) -> &'static str {
        match status {
            "compute_fallback_edge_runtime" => {
                "Next.js edge route handlers are emitted as edge runtime framework bundles; ONREZA Functions v1 accepts self-contained source files"
            }
            "compute_fallback_missing_file" => {
                "Next.js adapter output has no filePath for this route handler"
            }
            "compute_fallback_framework_assets" => {
                "Next.js route handler depends on traced assets/chunks or wasm; ONREZA Functions v1 cannot publish framework asset graphs"
            }
            _ => {
                "Next.js route handler still requires the Next.js invocation/cache/request adapter protocol; keep it in standalone COMPUTE until a framework bundle contract exists"
            }
        }
    }
}

impl StaticFileOutput {
    fn static_file_mapping(&self, project_dir: &Path) -> anyhow::Result<StaticFileMapping> {
        let target = pathname_to_relative_archive_path(&self.pathname)?;
        let source = canonical_next_output_source(project_dir, &self.file_path, "static file")?;

        Ok(StaticFileMapping { source, target })
    }
}

impl PrerenderOutput {
    fn platform_route_report(&self, descriptor: &AdapterDescriptor) -> serde_json::Value {
        let status = self.platform_route_status(descriptor);
        let fallback = self.fallback.as_ref();
        serde_json::json!({
            "id": self.id.as_deref(),
            "pathname": self.pathname.as_deref(),
            "parentOutputId": self.parent_output_id.as_deref(),
            "groupId": self.group_id,
            "kind": self.platform_kind(),
            "status": status,
            "primitive": prerender_route_primitive(status),
            "reason": self.platform_route_reason(status),
            "hasFallbackFile": self.has_fallback_file(),
            "initialStatus": fallback.and_then(|value| value.initial_status),
            "initialExpiration": fallback.and_then(|value| value.initial_expiration),
            "initialRevalidate": fallback.and_then(|value| value.initial_revalidate.as_ref()),
            "hasPprChain": self.ppr_chain.is_some(),
            "hasPostponedState": fallback.is_some_and(|value| value.postponed_state.is_some()),
            "parentFallbackMode": self.parent_fallback_mode.as_ref(),
            "renderingMode": self.config.get("renderingMode").and_then(serde_json::Value::as_str),
            "partialFallback": self.config.get("partialFallback").and_then(serde_json::Value::as_bool),
            "bypassToken": self.config.get("bypassToken").and_then(serde_json::Value::as_str).is_some(),
            "allowQueryCount": self.config.get("allowQuery").and_then(serde_json::Value::as_array).map_or(0, Vec::len),
            "allowHeaderCount": self.config.get("allowHeader").and_then(serde_json::Value::as_array).map_or(0, Vec::len),
            "bypassForCount": self.config.get("bypassFor").and_then(serde_json::Value::as_array).map_or(0, Vec::len),
        })
    }

    fn platform_route_status(&self, descriptor: &AdapterDescriptor) -> &'static str {
        if self.is_ppr() {
            return "compute_fallback_ppr";
        }
        if self.is_isr() {
            return "compute_fallback_isr";
        }
        if let Some(pathname) = self.static_html_pathname() {
            if descriptor.routing.has_exact_redirect_for_pathname(pathname) {
                return "compute_fallback_routing";
            }
            if descriptor.pathname_safe_for_prerender_layer(pathname) {
                return "static_layer";
            }
            return "compute_fallback_middleware";
        }
        if self.has_fallback_file() {
            return "compute_fallback_non_static_fallback";
        }
        "compute_fallback"
    }

    fn next_cache_route_report(&self, descriptor: &AdapterDescriptor) -> Option<serde_json::Value> {
        if !self.is_isr() && !self.is_ppr() {
            return None;
        }
        if self.is_next_cache_internal_artifact() {
            return None;
        }
        let status = self.next_cache_status(descriptor);
        let fallback = self.fallback.as_ref();
        Some(serde_json::json!({
            "id": self.id.as_deref(),
            "pathname": self.pathname.as_deref(),
            "kind": self.platform_kind(),
            "status": status,
            "reason": self.next_cache_reason(status),
            "initialRevalidateSeconds": self.initial_revalidate_seconds(),
            "initialStatus": fallback.and_then(|value| value.initial_status),
            "initialExpiration": fallback.and_then(|value| value.initial_expiration),
            "fallbackFilePath": fallback.and_then(|value| value.file_path.as_deref()),
            "middlewareSafe": self.middleware_safe_for_next_cache(descriptor),
            "hasPprChain": self.ppr_chain.is_some(),
            "hasPostponedState": fallback.is_some_and(|value| value.postponed_state.is_some()),
            "renderingMode": self.config.get("renderingMode").and_then(serde_json::Value::as_str),
            "partialFallback": self.config.get("partialFallback").and_then(serde_json::Value::as_bool),
        }))
    }

    fn is_next_cache_internal_artifact(&self) -> bool {
        self.pathname
            .as_deref()
            .is_some_and(next_cache_internal_artifact_pathname)
    }

    fn next_cache_status(&self, descriptor: &AdapterDescriptor) -> &'static str {
        if self.is_ppr() {
            return "blocked_ppr_runtime";
        }
        let Some(pathname) = self.pathname.as_deref() else {
            return "blocked_missing_pathname";
        };
        if !static_prerender_pathname_supported(pathname) {
            return "blocked_unsupported_pathname";
        }
        if !descriptor.pathname_safe_for_prerender_layer(pathname) {
            return "blocked_by_middleware";
        }
        let Some(fallback) = self.fallback.as_ref() else {
            return "blocked_missing_fallback_file";
        };
        if fallback.file_path.is_none() {
            return "blocked_missing_fallback_file";
        }
        if !fallback.is_html_response() {
            return "blocked_non_html_fallback";
        }
        if self.initial_revalidate_seconds().is_none() {
            return "blocked_unknown_revalidate";
        }
        "edge_cache_candidate"
    }

    fn next_cache_reason(&self, status: &str) -> serde_json::Value {
        let reason = match status {
            "edge_cache_candidate" => return serde_json::Value::Null,
            "blocked_ppr_runtime" => {
                "PPR requires cached shell bytes, postponedState, streaming resume, and Next runtime invocation"
            }
            "blocked_missing_pathname" => "Next.js prerender output has no pathname",
            "blocked_unsupported_pathname" => {
                "Next.js prerender pathname is not safe for edge cache routing"
            }
            "blocked_by_middleware" => "Next.js middleware may run before this route",
            "blocked_missing_fallback_file" => {
                "Next.js prerender output has no fallback file to seed cache"
            }
            "blocked_non_html_fallback" => "Next.js fallback is not an HTML response",
            _ => "Next.js prerender revalidate metadata is not understood by nrz",
        };
        serde_json::Value::String(reason.to_string())
    }

    fn middleware_safe_for_next_cache(&self, descriptor: &AdapterDescriptor) -> bool {
        self.pathname.as_deref().is_some_and(|pathname| {
            static_prerender_pathname_supported(pathname)
                && descriptor.pathname_safe_for_prerender_layer(pathname)
        })
    }

    fn platform_kind(&self) -> &'static str {
        if self.is_ppr() {
            return "ppr";
        }
        if self.is_isr() {
            return "isr";
        }
        if self.static_html_pathname().is_some() {
            return "static";
        }
        "dynamic"
    }

    fn platform_route_reason(&self, status: &str) -> serde_json::Value {
        let reason = match status {
            "static_layer" => return serde_json::Value::Null,
            "compute_fallback_ppr" => {
                "PPR requires Next.js resume/rendering runtime; the static shell and full render stay in standalone COMPUTE until a cache/resume contract is implemented"
            }
            "compute_fallback_isr" => {
                "ISR regeneration, cache-key, bypass and revalidation semantics stay in standalone COMPUTE until ONREZA cache/regeneration ownership is implemented"
            }
            "compute_fallback_middleware" => {
                "Next.js middleware may run before this prerendered page"
            }
            "compute_fallback_routing" => {
                "Next.js routing redirects this pathname before the prerendered response"
            }
            "compute_fallback_non_static_fallback" => {
                "fallback output is not a fully-static HTML response with initialRevalidate=false"
            }
            _ => "no fully-static HTML fallback file was found",
        };
        serde_json::Value::String(reason.to_string())
    }

    fn static_prerender_mapping(
        &self,
        project_dir: &Path,
        descriptor: &AdapterDescriptor,
    ) -> anyhow::Result<StaticPrerenderMapping> {
        let pathname = self
            .static_html_pathname()
            .context("Next.js prerender output is not a static HTML prerender")?;
        let file_path = self
            .fallback
            .as_ref()
            .and_then(|fallback| fallback.file_path.as_deref())
            .context("Next.js static HTML prerender is missing fallback.filePath")?;
        let source = canonical_next_output_source(project_dir, file_path, "prerender file")?;
        let target = prerender_pathname_to_relative_html_path(pathname)?;
        let served_pathname = descriptor.static_prerender_served_pathname(pathname);

        Ok(StaticPrerenderMapping {
            pathname: served_pathname.into_owned(),
            source,
            target,
        })
    }

    fn static_html_pathname(&self) -> Option<&str> {
        let pathname = self.pathname.as_deref()?;
        if !static_prerender_pathname_supported(pathname) {
            return None;
        }
        if self.is_ppr() || self.is_isr() {
            return None;
        }
        let fallback = self.fallback.as_ref()?;
        if fallback.initial_revalidate.as_ref() != Some(&serde_json::Value::Bool(false)) {
            return None;
        }
        if !fallback.is_html_response() {
            return None;
        }
        Some(pathname)
    }

    fn is_ppr(&self) -> bool {
        if self.ppr_chain.is_some() {
            return true;
        }
        if self
            .config
            .get("renderingMode")
            .and_then(serde_json::Value::as_str)
            == Some("PARTIALLY_STATIC")
        {
            return true;
        }
        if self
            .config
            .get("partialFallback")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        {
            return true;
        }
        self.fallback
            .as_ref()
            .and_then(|fallback| fallback.postponed_state.as_ref())
            .is_some()
    }

    fn is_isr(&self) -> bool {
        self.initial_revalidate_seconds().is_some()
    }

    fn initial_revalidate_seconds(&self) -> Option<u64> {
        self.fallback
            .as_ref()
            .and_then(|fallback| fallback.initial_revalidate.as_ref())
            .and_then(serde_json::Value::as_u64)
    }

    fn has_fallback_file(&self) -> bool {
        self.fallback
            .as_ref()
            .and_then(|fallback| fallback.file_path.as_ref())
            .is_some()
    }
}

fn prerender_route_primitive(status: &str) -> &'static str {
    if status == "static_layer" {
        "STATIC layer with fallthrough to COMPUTE"
    } else {
        "COMPUTE layer"
    }
}

impl PrerenderFallback {
    fn is_html_response(&self) -> bool {
        let content_type = self
            .initial_headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case("content-type"))
            .and_then(|(_, value)| value.as_str());
        if content_type.is_some_and(|value| value.starts_with("text/html")) {
            return true;
        }
        self.file_path
            .as_deref()
            .is_some_and(|path| path.ends_with(".html"))
    }
}

fn pathname_to_relative_archive_path(pathname: &str) -> anyhow::Result<String> {
    let trimmed = pathname.strip_prefix('/').unwrap_or(pathname);
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.contains('\\')
        || trimmed.contains('\0')
    {
        anyhow::bail!("invalid Next.js static pathname: {pathname}");
    }
    for segment in trimmed.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            anyhow::bail!("unsafe Next.js static pathname: {pathname}");
        }
    }
    Ok(trimmed.to_string())
}

fn prerender_pathname_to_relative_html_path(pathname: &str) -> anyhow::Result<String> {
    if !static_prerender_pathname_supported(pathname) {
        anyhow::bail!("invalid Next.js prerender pathname: {pathname}");
    }

    let trimmed = pathname.trim_start_matches('/').trim_end_matches('/');
    if trimmed.is_empty() {
        return Ok("index.html".to_string());
    }
    if trimmed.ends_with(".html") {
        return Ok(trimmed.to_string());
    }
    Ok(format!("{trimmed}/index.html"))
}

fn static_prerender_pathname_supported(pathname: &str) -> bool {
    if !pathname.starts_with('/')
        || pathname.contains(['?', '#', '\\', '\0'])
        || pathname.starts_with("/_")
        || pathname.ends_with(".rsc")
        || pathname.contains(".segments/")
    {
        return false;
    }

    let trimmed = pathname.trim_start_matches('/').trim_end_matches('/');
    if trimmed.is_empty() {
        return true;
    }
    trimmed
        .split('/')
        .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn next_cache_internal_artifact_pathname(pathname: &str) -> bool {
    pathname == "/_not-found"
        || pathname.starts_with("/_global-error")
        || pathname.starts_with("/_next/")
        || pathname.ends_with(".rsc")
        || pathname.contains(".segments/")
}

fn canonical_next_output_source(
    project_dir: &Path,
    file_path: &str,
    kind: &str,
) -> anyhow::Result<PathBuf> {
    let source = PathBuf::from(file_path);
    let source = if source.is_absolute() {
        source
    } else {
        project_dir.join(source)
    };
    let canonical_project = std::fs::canonicalize(project_dir)
        .with_context(|| format!("failed to canonicalize {}", project_dir.display()))?;
    let canonical_source = std::fs::canonicalize(&source)
        .with_context(|| format!("failed to canonicalize {}", source.display()))?;
    if !canonical_source.starts_with(&canonical_project) {
        anyhow::bail!(
            "Next.js {kind} '{}' points outside project root {}",
            source.display(),
            canonical_project.display()
        );
    }
    Ok(canonical_source)
}

fn supports_next_adapter_api(project_dir: &Path) -> bool {
    if let Some(version) = installed_next_version(project_dir) {
        return parse_semver_token(&version)
            .is_some_and(|version| semver_at_least(version, MIN_ADAPTER_VERSION));
    }

    let Some(pkg) = crate::detect::package_json::PackageJson::load(project_dir) else {
        return false;
    };
    let Some(version) = pkg.dependency_version("next") else {
        return false;
    };
    dependency_range_supports_adapter(version)
}

fn installed_next_version(project_dir: &Path) -> Option<String> {
    let contents = std::fs::read_to_string(installed_next_package_json(project_dir)?).ok()?;
    let package: serde_json::Value = serde_json::from_str(&contents).ok()?;
    package
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn installed_next_package_json(project_dir: &Path) -> Option<PathBuf> {
    for dir in project_dir.ancestors() {
        let candidate = dir.join("node_modules/next/package.json");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn dependency_range_supports_adapter(version: &str) -> bool {
    version
        .split("||")
        .filter_map(extract_semver_lower_bound)
        .any(|version| semver_at_least(version, MIN_ADAPTER_VERSION))
}

fn extract_semver_lower_bound(range: &str) -> Option<(u64, u64, u64)> {
    let range = range.trim();
    if range.is_empty() {
        return None;
    }

    for token in range.split_whitespace() {
        if token.starts_with('<') {
            continue;
        }
        if let Some(version) = parse_semver_token(token) {
            return Some(version);
        }
    }

    parse_semver_token(range)
}

fn parse_semver_token(token: &str) -> Option<(u64, u64, u64)> {
    let token = token
        .trim()
        .trim_start_matches('=')
        .trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches(">=")
        .trim_start_matches('>')
        .trim_start_matches('v');

    let mut parts = token.split(|c: char| !(c.is_ascii_digit() || c == '.'));
    let numeric = parts.next()?.trim_matches('.');
    if numeric.is_empty() {
        return None;
    }

    let mut nums = numeric.split('.');
    let major = nums.next()?.parse().ok()?;
    let minor = nums.next().unwrap_or("0").parse().ok()?;
    let patch = nums.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

fn semver_at_least(version: (u64, u64, u64), min: (u64, u64, u64)) -> bool {
    version >= min
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_supported_next_ranges() {
        assert!(dependency_range_supports_adapter("16.2.0"));
        assert!(dependency_range_supports_adapter("^16.2.0"));
        assert!(dependency_range_supports_adapter("~16.2.9"));
        assert!(dependency_range_supports_adapter(">=16.2.0"));
        assert!(dependency_range_supports_adapter("17.0.0"));
        assert!(dependency_range_supports_adapter("15.5.0 || ^16.2.0"));
    }

    #[test]
    fn rejects_old_or_uncertain_next_ranges() {
        assert!(!dependency_range_supports_adapter("15.5.0"));
        assert!(!dependency_range_supports_adapter("^16.1.0"));
        assert!(!dependency_range_supports_adapter("workspace:*"));
        assert!(!dependency_range_supports_adapter("latest"));
    }

    #[test]
    fn materializes_adapter_for_supported_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"next":"^16.2.0"}}"#,
        )
        .unwrap();

        let adapter = prepare_build_adapter(dir.path())
            .unwrap()
            .expect("supported next should materialize adapter");

        assert!(adapter.path.is_file());
        assert_eq!(
            std::fs::read_to_string(adapter.path).unwrap(),
            ADAPTER_SOURCE
        );
    }

    #[test]
    fn skips_adapter_for_old_next_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"next":"15.5.0"}}"#,
        )
        .unwrap();

        assert!(prepare_build_adapter(dir.path()).unwrap().is_none());
    }

    #[test]
    fn installed_next_version_is_adapter_support_source_of_truth() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"next":"^16.0.0"}}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/next")).unwrap();
        std::fs::write(
            dir.path().join("node_modules/next/package.json"),
            r#"{"name":"next","version":"16.2.3"}"#,
        )
        .unwrap();

        assert!(
            prepare_build_adapter(dir.path())
                .unwrap()
                .expect("installed supported next should materialize adapter")
                .path
                .is_file()
        );
    }

    #[test]
    fn installed_next_version_uses_node_resolution_ancestors() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("apps/web");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("package.json"),
            r#"{"dependencies":{"next":"^16.0.0"}}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/next")).unwrap();
        std::fs::write(
            dir.path().join("node_modules/next/package.json"),
            r#"{"name":"next","version":"16.2.3"}"#,
        )
        .unwrap();

        assert!(
            prepare_build_adapter(&app)
                .unwrap()
                .expect("hoisted supported next should materialize adapter")
                .path
                .is_file()
        );
    }

    #[test]
    fn installed_old_next_version_disables_adapter_even_with_supported_range() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"next":"^16.2.0"}}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/next")).unwrap();
        std::fs::write(
            dir.path().join("node_modules/next/package.json"),
            r#"{"name":"next","version":"16.1.9"}"#,
        )
        .unwrap();

        assert!(prepare_build_adapter(dir.path()).unwrap().is_none());
    }

    #[test]
    fn clear_descriptor_removes_stale_adapter_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = descriptor_path(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{}").unwrap();

        clear_descriptor(dir.path()).unwrap();

        assert!(!path.exists());
        clear_descriptor(dir.path()).unwrap();
    }

    #[test]
    fn static_file_mapping_uses_url_path_inside_static_layer() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join(".next/static/chunks/app.js");
        std::fs::create_dir_all(src.parent().unwrap()).unwrap();
        std::fs::write(&src, "// app").unwrap();
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "outputs": {
                "staticFiles": [{
                    "pathname": "/_next/static/chunks/app.js",
                    "filePath": src,
                }]
            }
        }))
        .unwrap();

        let mappings = descriptor
            .static_file_mappings_for_static_layer(dir.path())
            .unwrap();

        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].target, "_next/static/chunks/app.js");
    }

    #[test]
    fn static_file_mapping_accepts_project_relative_file_path() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join(".next/static/chunks/app.js");
        std::fs::create_dir_all(src.parent().unwrap()).unwrap();
        std::fs::write(&src, "// app").unwrap();
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "outputs": {
                "staticFiles": [{
                    "pathname": "/_next/static/chunks/app.js",
                    "filePath": ".next/static/chunks/app.js",
                }]
            }
        }))
        .unwrap();

        let mappings = descriptor
            .static_file_mappings_for_static_layer(dir.path())
            .unwrap();

        assert_eq!(mappings[0].source, std::fs::canonicalize(src).unwrap());
    }

    #[test]
    fn static_prerender_mapping_keeps_only_static_html_pages() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join(".next/server/app/index.html");
        let isr = dir.path().join(".next/server/app/blog/index.html");
        let ppr = dir.path().join(".next/server/app/shop/index.html");
        let rsc = dir.path().join(".next/server/app/index.rsc");
        std::fs::create_dir_all(home.parent().unwrap()).unwrap();
        std::fs::create_dir_all(isr.parent().unwrap()).unwrap();
        std::fs::create_dir_all(ppr.parent().unwrap()).unwrap();
        std::fs::write(&home, "<main>home</main>").unwrap();
        std::fs::write(&isr, "<main>blog</main>").unwrap();
        std::fs::write(&ppr, "<main>shop</main>").unwrap();
        std::fs::write(&rsc, "rsc").unwrap();

        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "outputs": {
                "prerenders": [
                    {
                        "type": "PRERENDER",
                        "pathname": "/",
                        "fallback": {
                            "filePath": home,
                            "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                            "initialRevalidate": false
                        }
                    },
                    {
                        "type": "PRERENDER",
                        "pathname": "/blog",
                        "fallback": {
                            "filePath": isr,
                            "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                            "initialRevalidate": 60
                        }
                    },
                    {
                        "type": "PRERENDER",
                        "id": "app-shop",
                        "pathname": "/shop",
                        "groupId": 7,
                        "pprChain": { "headers": { "next-resume": "1" } },
                        "fallback": {
                            "filePath": ppr,
                            "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                            "initialRevalidate": false,
                            "postponedState": "state"
                        },
                        "config": {
                            "renderingMode": "PARTIALLY_STATIC",
                            "partialFallback": true,
                            "bypassToken": "token",
                            "allowQuery": ["preview"],
                            "allowHeader": ["rsc"],
                            "bypassFor": [{ "type": "header", "key": "x-bypass", "value": "1" }]
                        }
                    },
                    {
                        "type": "PRERENDER",
                        "pathname": "/index.rsc",
                        "fallback": {
                            "filePath": rsc,
                            "initialHeaders": { "content-type": "text/x-component" },
                            "initialRevalidate": false
                        }
                    }
                ]
            }
        }))
        .unwrap();

        let mappings = descriptor
            .static_prerender_mappings_for_static_layer(dir.path())
            .unwrap();

        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].pathname, "/");
        assert_eq!(mappings[0].target, "index.html");
        assert_eq!(mappings[0].source, std::fs::canonicalize(home).unwrap());

        let summary = descriptor.compatibility_summary();
        assert_eq!(
            summary["platform"]["prerenders"]["status"],
            "partial_static_split"
        );
        assert_eq!(summary["platform"]["prerenders"]["staticLayerCount"], 1);
        assert_eq!(summary["platform"]["prerenders"]["isrCount"], 1);
        assert_eq!(summary["platform"]["prerenders"]["pprCount"], 1);
        assert_eq!(
            summary["platform"]["nextCache"]["status"],
            "report_only_candidates"
        );
        assert_eq!(summary["platform"]["nextCache"]["mode"], "report_only");
        assert_eq!(
            summary["platform"]["nextCache"]["schemaVersion"],
            "NEXT_CACHE_SUBSTRATE_V1"
        );
        assert_eq!(summary["platform"]["nextCache"]["isrCandidateCount"], 1);
        assert_eq!(summary["platform"]["nextCache"]["isrBlockedCount"], 0);
        assert_eq!(summary["platform"]["nextCache"]["pprBlockedCount"], 1);
        assert_eq!(
            summary["platform"]["prerenders"]["routes"][0]["kind"],
            "static"
        );
        assert_eq!(
            summary["platform"]["prerenders"]["routes"][0]["status"],
            "static_layer"
        );
        assert_eq!(
            summary["platform"]["prerenders"]["routes"][1]["kind"],
            "isr"
        );
        assert_eq!(
            summary["platform"]["prerenders"]["routes"][1]["status"],
            "compute_fallback_isr"
        );
        assert_eq!(
            summary["platform"]["nextCache"]["routes"][0]["status"],
            "edge_cache_candidate"
        );
        assert_eq!(
            summary["platform"]["nextCache"]["routes"][0]["initialRevalidateSeconds"],
            60
        );
        assert_eq!(
            summary["platform"]["nextCache"]["routes"][0]["middlewareSafe"],
            true
        );
        assert_eq!(
            summary["platform"]["prerenders"]["routes"][2]["kind"],
            "ppr"
        );
        assert_eq!(
            summary["platform"]["prerenders"]["routes"][2]["status"],
            "compute_fallback_ppr"
        );
        assert_eq!(summary["platform"]["prerenders"]["routes"][2]["groupId"], 7);
        assert_eq!(
            summary["platform"]["prerenders"]["routes"][2]["renderingMode"],
            "PARTIALLY_STATIC"
        );
        assert_eq!(
            summary["platform"]["prerenders"]["routes"][2]["bypassForCount"],
            1
        );
        assert_eq!(
            summary["platform"]["nextCache"]["routes"][1]["status"],
            "blocked_ppr_runtime"
        );
        assert!(
            descriptor
                .compatibility_report_line()
                .contains("ISR cache 1 candidates/0 blocked (report_only_candidates)")
        );
    }

    #[test]
    fn next_cache_report_ignores_internal_next_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let blog_html = dir.path().join(".next/server/app/blog/index.html");
        let blog_rsc = dir.path().join(".next/server/app/blog/index.rsc");
        let blog_segment = dir
            .path()
            .join(".next/server/app/blog/index.segments/_head.segment.rsc");
        let data_json = dir.path().join(".next/server/pages/isr.json");
        let ppr_html = dir.path().join(".next/server/app/shop/index.html");
        let ppr_rsc = dir.path().join(".next/server/app/shop/index.rsc");
        let global_error = dir.path().join(".next/server/app/_global-error.html");

        for path in [
            &blog_html,
            &blog_rsc,
            &blog_segment,
            &data_json,
            &ppr_html,
            &ppr_rsc,
            &global_error,
        ] {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        std::fs::write(&blog_html, "<main>blog</main>").unwrap();
        std::fs::write(&blog_rsc, "blog rsc").unwrap();
        std::fs::write(&blog_segment, "blog segment").unwrap();
        std::fs::write(&data_json, "{}").unwrap();
        std::fs::write(&ppr_html, "<main>shop</main>").unwrap();
        std::fs::write(&ppr_rsc, "shop rsc").unwrap();
        std::fs::write(&global_error, "<main>error</main>").unwrap();

        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "outputs": {
                "prerenders": [
                    {
                        "type": "PRERENDER",
                        "pathname": "/blog",
                        "fallback": {
                            "filePath": blog_html,
                            "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                            "initialRevalidate": 60
                        }
                    },
                    {
                        "type": "PRERENDER",
                        "pathname": "/blog.rsc",
                        "fallback": {
                            "filePath": blog_rsc,
                            "initialHeaders": { "content-type": "text/x-component" },
                            "initialRevalidate": 60
                        }
                    },
                    {
                        "type": "PRERENDER",
                        "pathname": "/blog.segments/_head.segment.rsc",
                        "fallback": {
                            "filePath": blog_segment,
                            "initialHeaders": { "content-type": "text/x-component" },
                            "initialRevalidate": 60
                        }
                    },
                    {
                        "type": "PRERENDER",
                        "pathname": "/_next/data/build/isr.json",
                        "fallback": {
                            "filePath": data_json,
                            "initialHeaders": { "content-type": "application/json" },
                            "initialRevalidate": 10
                        }
                    },
                    {
                        "type": "PRERENDER",
                        "pathname": "/shop",
                        "pprChain": { "headers": { "next-resume": "1" } },
                        "fallback": {
                            "filePath": ppr_html,
                            "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                            "initialRevalidate": false,
                            "postponedState": "state"
                        },
                        "config": { "renderingMode": "PARTIALLY_STATIC" }
                    },
                    {
                        "type": "PRERENDER",
                        "pathname": "/shop.rsc",
                        "pprChain": { "headers": { "next-resume": "1" } },
                        "fallback": {
                            "filePath": ppr_rsc,
                            "initialHeaders": { "content-type": "text/x-component" },
                            "initialRevalidate": false,
                            "postponedState": "state"
                        },
                        "config": { "renderingMode": "PARTIALLY_STATIC" }
                    },
                    {
                        "type": "PRERENDER",
                        "pathname": "/_global-error",
                        "pprChain": { "headers": { "next-resume": "1" } },
                        "fallback": {
                            "filePath": global_error,
                            "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                            "initialRevalidate": false
                        },
                        "config": { "renderingMode": "PARTIALLY_STATIC" }
                    }
                ]
            }
        }))
        .unwrap();

        let summary = descriptor.compatibility_summary();
        assert_eq!(summary["platform"]["nextCache"]["routeCount"], 2);
        assert_eq!(summary["platform"]["nextCache"]["isrCandidateCount"], 1);
        assert_eq!(summary["platform"]["nextCache"]["isrBlockedCount"], 0);
        assert_eq!(summary["platform"]["nextCache"]["pprBlockedCount"], 1);
        assert_eq!(
            summary["platform"]["nextCache"]["routes"][0]["pathname"],
            "/blog"
        );
        assert_eq!(
            summary["platform"]["nextCache"]["routes"][1]["pathname"],
            "/shop"
        );
    }

    #[test]
    fn static_prerender_mapping_uses_trailing_slash_canonical_pathname() {
        let dir = tempfile::tempdir().unwrap();
        let docs = dir.path().join(".next/server/app/index.html");
        let favicon = dir.path().join(".next/server/app/favicon.ico.html");
        std::fs::create_dir_all(docs.parent().unwrap()).unwrap();
        std::fs::write(&docs, "<main>docs</main>").unwrap();
        std::fs::write(&favicon, "icon").unwrap();

        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "config": {
                "basePath": "/docs",
                "trailingSlash": true
            },
            "outputs": {
                "prerenders": [
                    {
                        "type": "PRERENDER",
                        "pathname": "/docs",
                        "fallback": {
                            "filePath": docs,
                            "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                            "initialRevalidate": false
                        }
                    },
                    {
                        "type": "PRERENDER",
                        "pathname": "/docs/favicon.ico",
                        "fallback": {
                            "filePath": favicon,
                            "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                            "initialRevalidate": false
                        }
                    }
                ]
            }
        }))
        .unwrap();

        let mappings = descriptor
            .static_prerender_mappings_for_static_layer(dir.path())
            .unwrap();

        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].pathname, "/docs/");
        assert_eq!(mappings[0].target, "docs/index.html");
        assert_eq!(mappings[1].pathname, "/docs/favicon.ico");
    }

    #[test]
    fn static_prerender_mapping_keeps_exact_redirect_conflicts_in_compute() {
        let dir = tempfile::tempdir().unwrap();
        let docs = dir.path().join(".next/server/app/index.html");
        std::fs::create_dir_all(docs.parent().unwrap()).unwrap();
        std::fs::write(&docs, "<main>docs</main>").unwrap();

        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "routing": {
                "beforeMiddleware": [{
                    "source": "/docs",
                    "headers": { "Location": "/docs/" },
                    "status": 308
                }]
            },
            "outputs": {
                "prerenders": [{
                    "type": "PRERENDER",
                    "pathname": "/docs",
                    "fallback": {
                        "filePath": docs,
                        "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                        "initialRevalidate": false
                    }
                }]
            }
        }))
        .unwrap();

        let mappings = descriptor
            .static_prerender_mappings_for_static_layer(dir.path())
            .unwrap();
        let summary = descriptor.compatibility_summary();

        assert!(mappings.is_empty());
        assert_eq!(
            summary["platform"]["prerenders"]["routes"][0]["status"],
            "compute_fallback_routing"
        );
        assert_eq!(
            summary["platform"]["prerenders"]["routes"][0]["reason"],
            "Next.js routing redirects this pathname before the prerendered response"
        );
    }

    #[test]
    fn compatibility_summary_reports_next_config_and_routing_metadata() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "config": {
                "basePath": "/docs",
                "trailingSlash": true,
                "i18n": { "locales": ["en", "ru"], "defaultLocale": "en" }
            },
            "routing": {
                "shouldNormalizeNextData": true,
                "rsc": { "prefetchHeader": "next-router-prefetch" }
            },
            "outputs": {
                "staticFiles": [{
                    "type": "STATIC_FILE",
                    "pathname": "/_next/static/chunks/app.js",
                    "filePath": "/tmp/app.js"
                }]
            }
        }))
        .unwrap();

        let summary = descriptor.compatibility_summary();
        assert_eq!(summary["config"]["basePath"], "/docs");
        assert_eq!(summary["config"]["trailingSlash"], true);
        assert_eq!(summary["config"]["i18n"]["defaultLocale"], "en");
        assert_eq!(summary["routing"]["shouldNormalizeNextData"], true);
        assert_eq!(summary["routing"]["hasRscRouting"], true);
        assert!(
            descriptor
                .compatibility_report_line()
                .contains("Next.js adapter report: STATIC 1")
        );
    }

    #[test]
    fn compatibility_summary_reports_onreza_image_optimizer() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "config": {
                "images": {
                    "loader": "custom",
                    "loaderFile": "./.onreza/cache/next-adapter/onreza-image-loader.mjs",
                    "path": "/_onreza/image"
                }
            },
            "deploymentHints": {
                "imageOptimizer": {
                    "status": "onreza_optimizer",
                    "mode": "custom_loader",
                    "path": "/_onreza/image",
                    "primitive": "ONREZA image optimizer",
                    "reason": null
                }
            }
        }))
        .unwrap();

        let summary = descriptor.compatibility_summary();
        assert_eq!(
            summary["platform"]["imageOptimizer"]["status"],
            "onreza_optimizer"
        );
        assert_eq!(
            summary["platform"]["imageOptimizer"]["primitive"],
            "ONREZA image optimizer"
        );
        assert_eq!(
            summary["config"]["images"]["loaderFile"],
            "./.onreza/cache/next-adapter/onreza-image-loader.mjs"
        );
        assert!(
            descriptor
                .compatibility_report_line()
                .contains("image optimizer onreza_optimizer")
        );
    }

    #[test]
    fn static_prerender_mapping_respects_middleware_matchers() {
        let dir = tempfile::tempdir().unwrap();
        let dashboard = dir.path().join(".next/server/app/dashboard/index.html");
        let reports = dir
            .path()
            .join(".next/server/app/dashboard/reports/index.html");
        std::fs::create_dir_all(dashboard.parent().unwrap()).unwrap();
        std::fs::create_dir_all(reports.parent().unwrap()).unwrap();
        std::fs::write(&dashboard, "<main>dashboard</main>").unwrap();
        std::fs::write(&reports, "<main>reports</main>").unwrap();

        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "outputs": {
                "middleware": {
                    "type": "MIDDLEWARE",
                    "pathname": "/_middleware",
                    "runtime": "edge",
                    "config": {
                        "matchers": [{
                            "source": "/dashboard/:path*",
                            "sourceRegex": "^/dashboard(?:/.*)?$"
                        }]
                    }
                },
                "prerenders": [
                    {
                        "type": "PRERENDER",
                        "pathname": "/dashboard",
                        "fallback": {
                            "filePath": dashboard,
                            "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                            "initialRevalidate": false
                        }
                    },
                    {
                        "type": "PRERENDER",
                        "pathname": "/dashboard/reports",
                        "fallback": {
                            "filePath": reports,
                            "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                            "initialRevalidate": 60
                        }
                    }
                ]
            }
        }))
        .unwrap();

        let mappings = descriptor
            .static_prerender_mappings_for_static_layer(dir.path())
            .unwrap();

        assert!(mappings.is_empty());
        assert_eq!(
            descriptor.compatibility_summary()["platform"]["prerenders"]["status"],
            "compute_fallback"
        );
        let summary = descriptor.compatibility_summary();
        assert_eq!(
            summary["platform"]["nextCache"]["status"],
            "report_only_blocked"
        );
        assert_eq!(summary["platform"]["nextCache"]["isrCandidateCount"], 0);
        assert_eq!(summary["platform"]["nextCache"]["isrBlockedCount"], 1);
        assert_eq!(
            summary["platform"]["nextCache"]["routes"][0]["status"],
            "blocked_by_middleware"
        );
    }

    #[test]
    fn compatibility_summary_reports_routing_and_middleware_compute_fallback() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "routing": {
                "beforeMiddleware": [{
                    "source": "/old",
                    "sourceRegex": "^/old$",
                    "headers": { "location": "/new" },
                    "status": 308,
                    "priority": true
                }],
                "beforeFiles": [{
                    "source": "/docs/:path*",
                    "sourceRegex": "^/docs(?:/(.*))?$",
                    "destination": "/help/:path*"
                }]
            },
            "outputs": {
                "staticFiles": [{
                    "pathname": "/_next/static/chunks/app.js",
                    "filePath": "/tmp/app.js"
                }],
                "middleware": {
                    "type": "MIDDLEWARE",
                    "id": "middleware",
                    "sourcePage": "/",
                    "filePath": "/tmp/middleware-edge.js",
                    "pathname": "/_middleware",
                    "runtime": "edge",
                    "assets": {
                        "server/edge/chunks/runtime.js": "/tmp/runtime.js",
                        "server/edge/chunks/app.js": "/tmp/app.js"
                    },
                    "edgeRuntime": { "entryKey": "middleware" }
                }
            }
        }))
        .unwrap();

        let summary = descriptor.compatibility_summary();

        assert!(descriptor.middleware_may_match_path("/_next/static/chunks/app.js"));
        assert_eq!(summary["routing"]["redirects"], 1);
        assert_eq!(summary["routing"]["rewrites"], 1);
        assert_eq!(summary["routing"]["sourceRegexRules"], 2);
        assert_eq!(summary["platform"]["staticFiles"]["staticLayerCount"], 0);
        assert_eq!(
            summary["platform"]["staticFiles"]["status"],
            "compute_fallback"
        );
        assert_eq!(
            summary["platform"]["middleware"]["status"],
            "compute_fallback_edge_runtime"
        );
        assert_eq!(
            summary["platform"]["middleware"]["kind"],
            "next_middleware_edge"
        );
        assert_eq!(summary["platform"]["middleware"]["assetCount"], 2);
        assert_eq!(summary["platform"]["edgeRuntime"]["count"], 1);
        assert_eq!(
            summary["platform"]["edgeRuntime"]["status"],
            "compute_fallback"
        );
    }

    #[test]
    fn compatibility_summary_classifies_next_proxy_node_runtime() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "outputs": {
                "middleware": {
                    "type": "MIDDLEWARE",
                    "id": "/_middleware",
                    "sourcePage": "middleware",
                    "filePath": "/tmp/middleware.js",
                    "pathname": "/_middleware",
                    "runtime": "nodejs",
                    "assets": {
                        ".next/server/middleware.js": "/tmp/middleware.js"
                    },
                    "config": {
                        "matchers": [{
                            "source": "/private/:path*",
                            "sourceRegex": "^/private(?:/.*)?$"
                        }]
                    }
                }
            }
        }))
        .unwrap();

        let summary = descriptor.compatibility_summary();

        assert_eq!(
            summary["platform"]["middleware"]["status"],
            "compute_fallback_nodejs_proxy"
        );
        assert_eq!(
            summary["platform"]["middleware"]["kind"],
            "next_proxy_nodejs"
        );
        assert_eq!(summary["platform"]["middleware"]["matcherCount"], 1);
        assert_eq!(summary["platform"]["middleware"]["assetCount"], 1);
        assert_eq!(summary["platform"]["edgeRuntime"]["count"], 0);
    }

    #[test]
    fn compatibility_summary_classifies_route_handlers_for_compute_fallback() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "outputs": {
                "pagesApi": [{
                    "type": "PAGES_API",
                    "id": "pages-api-hello",
                    "pathname": "/api/hello",
                    "sourcePage": "pages/api/hello.ts",
                    "filePath": "/tmp/pages-api-hello.js",
                    "runtime": "nodejs",
                    "assets": {
                        ".next/server/chunks/next-runtime.js": "/tmp/next-runtime.js"
                    }
                }],
                "appRoutes": [
                    {
                        "type": "APP_ROUTE",
                        "id": "app-api-ping",
                        "pathname": "/api/ping",
                        "sourcePage": "app/api/ping/route.ts",
                        "filePath": "/tmp/app-api-ping.js",
                        "runtime": "edge",
                        "edgeRuntime": { "entryKey": "app-api-ping" }
                    },
                    {
                        "type": "APP_ROUTE",
                        "id": "app-api-ping-rsc",
                        "pathname": "/api/ping.rsc",
                        "sourcePage": "app/api/ping/route.ts",
                        "filePath": "/tmp/app-api-ping.rsc",
                        "runtime": "nodejs",
                        "assets": {
                            ".next/server/chunks/next-runtime.js": "/tmp/next-runtime.js"
                        }
                    }
                ]
            }
        }))
        .unwrap();

        let summary = descriptor.compatibility_summary();
        assert_eq!(
            summary["platform"]["routeHandlers"]["status"],
            "compute_fallback"
        );
        assert_eq!(summary["platform"]["routeHandlers"]["count"], 2);
        assert_eq!(summary["platform"]["routeHandlers"]["pagesApi"], 1);
        assert_eq!(summary["platform"]["routeHandlers"]["appRoutes"], 1);
        assert_eq!(summary["platform"]["routeHandlers"]["assetBacked"], 1);
        assert_eq!(
            summary["platform"]["routeHandlers"]["functionsV1"]["status"],
            "blocked_framework_bundle_contract"
        );
        assert_eq!(
            summary["platform"]["routeHandlers"]["routes"][0]["status"],
            "compute_fallback_framework_assets"
        );
        assert_eq!(
            summary["platform"]["routeHandlers"]["routes"][1]["status"],
            "compute_fallback_edge_runtime"
        );
        assert!(
            descriptor
                .compatibility_report_line()
                .contains("route handlers 2 (compute_fallback)")
        );
    }

    #[test]
    fn middleware_matcher_allows_disjoint_static_pathnames() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "outputs": {
                "middleware": {
                    "type": "MIDDLEWARE",
                    "pathname": "/_middleware",
                    "runtime": "edge",
                    "config": {
                        "matchers": [{
                            "source": "/private/:path*",
                            "sourceRegex": "^(?:\\/(_next\\/data\\/[^/]{1,}))?\\/private(?:\\/((?:[^\\/#\\?]+?)(?:\\/(?:[^\\/#\\?]+?))*))?(\\.json|\\.rsc)?[\\/#\\?]?$"
                        }]
                    }
                },
                "staticFiles": [{
                    "pathname": "/_next/static/chunks/app.js",
                    "filePath": "/tmp/app.js"
                }]
            }
        }))
        .unwrap();

        let summary = descriptor.compatibility_summary();

        assert!(!descriptor.middleware_may_match_path("/_next/static/chunks/app.js"));
        assert!(descriptor.middleware_may_match_path("/private/dashboard"));
        assert_eq!(
            summary["platform"]["staticFiles"]["status"],
            "guarded_static_split"
        );
        assert_eq!(summary["platform"]["staticFiles"]["staticLayerCount"], 1);
    }

    #[test]
    fn generated_edge_rules_lower_supported_next_routing_subset() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "routing": {
                "beforeMiddleware": [{
                    "source": "/old",
                    "headers": { "Location": "/new" },
                    "status": 308
                }],
                "beforeFiles": [{
                    "source": "/docs/:slug",
                    "destination": "/help/$1"
                }],
                "afterFiles": [{
                    "source": "/headers",
                    "headers": { "x-next": "yes" }
                }],
                "fallback": [{
                    "sourceRegex": "^/regex/(.*)$",
                    "headers": { "x-skip": "yes" }
                }]
            }
        }))
        .unwrap();

        let rules = descriptor
            .generated_edge_rules()
            .expect("supported routes should generate edge rules");
        let parsed: nrz_contract::EdgeRuleSetAuthoring =
            serde_json::from_value(rules.clone()).unwrap();

        assert_eq!(parsed.rules.len(), 3);
        assert_eq!(rules["rules"][0]["action"]["type"], "redirect");
        assert_eq!(rules["rules"][0]["action"]["target"], "/new");
        assert_eq!(rules["rules"][1]["condition"]["path"]["type"], "glob");
        assert_eq!(
            rules["rules"][1]["condition"]["path"]["value"],
            "/docs/{slug}"
        );
        assert_eq!(rules["rules"][1]["action"]["target"], "/help/{slug}");
        assert_eq!(rules["rules"][2]["action"]["type"], "set_headers");

        let summary = descriptor.compatibility_summary();
        assert_eq!(
            summary["platform"]["routing"]["status"],
            "partial_edge_rules"
        );
        assert_eq!(summary["platform"]["routing"]["edgeRulesGenerated"], 3);
        assert_eq!(summary["platform"]["routing"]["edgeRulesUnsupported"], 1);
    }

    #[test]
    fn generated_edge_rules_rewrite_numeric_splat_captures() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "routing": {
                "afterFiles": [{
                    "source": "/docs/:path+",
                    "destination": "/help/$1"
                }]
            }
        }))
        .unwrap();

        let rules = descriptor
            .generated_edge_rules()
            .expect("supported splat rewrite should generate an edge rule");
        let parsed: nrz_contract::EdgeRuleSetAuthoring =
            serde_json::from_value(rules.clone()).unwrap();

        assert_eq!(parsed.rules.len(), 1);
        assert_eq!(
            rules["rules"][0]["condition"]["path"]["value"],
            "/docs/{path...}"
        );
        assert_eq!(rules["rules"][0]["action"]["target"], "/help/{path}");
    }

    #[test]
    fn generated_edge_rules_lower_literal_has_and_missing_conditions() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "routing": {
                "beforeFiles": [{
                    "source": "/docs/:slug",
                    "destination": "/preview/$1",
                    "has": [
                        { "type": "header", "key": "x-route", "value": "docs" },
                        { "type": "query", "key": "preview", "value": "1" },
                        { "type": "cookie", "key": "variant", "value": "a" },
                        { "type": "host", "value": "example.com" }
                    ],
                    "missing": [
                        { "type": "header", "key": "x-skip", "value": "1" }
                    ]
                }]
            }
        }))
        .unwrap();

        let rules = descriptor
            .generated_edge_rules()
            .expect("literal request conditions should generate an edge rule");
        let parsed: nrz_contract::EdgeRuleSetAuthoring =
            serde_json::from_value(rules.clone()).unwrap();

        assert_eq!(parsed.rules.len(), 1);
        assert_eq!(
            rules["rules"][0]["condition"]["path"]["value"],
            "/docs/{slug}"
        );
        assert_eq!(rules["rules"][0]["condition"]["headers"]["x-route"], "docs");
        assert_eq!(rules["rules"][0]["condition"]["query"]["preview"], "1");
        assert_eq!(rules["rules"][0]["condition"]["cookies"]["variant"], "a");
        assert_eq!(rules["rules"][0]["condition"]["host"], "example.com");
        assert_eq!(
            rules["rules"][0]["condition"]["not"]["headers"]["x-skip"],
            "1"
        );
    }

    #[test]
    fn generated_edge_rules_lower_trailing_slash_splat_redirect() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "routing": {
                "beforeFiles": [{
                    "source": "/:path+/",
                    "headers": { "Location": "/$1" },
                    "status": 308
                }]
            }
        }))
        .unwrap();

        let rules = descriptor
            .generated_edge_rules()
            .expect("trailing-slash splat redirect should generate an edge rule");
        let parsed: nrz_contract::EdgeRuleSetAuthoring =
            serde_json::from_value(rules.clone()).unwrap();

        assert_eq!(parsed.rules.len(), 1);
        assert_eq!(
            rules["rules"][0]["condition"]["path"]["value"],
            "/{path...}/"
        );
        assert_eq!(rules["rules"][0]["action"]["target"], "/{path}");
    }

    #[test]
    fn generated_edge_rules_lower_next_static_cache_headers() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "routing": {
                "onMatch": [{
                    "sourceRegex": "/_next/static/(?:[^/]+/pages|pages|chunks|runtime|css|image|media|BUILDID)/.+",
                    "headers": { "cache-control": "public,max-age=31536000,immutable" }
                }]
            }
        }))
        .unwrap();

        let rules = descriptor
            .generated_edge_rules()
            .expect("Next.js static cache header should generate an edge rule");
        let parsed: nrz_contract::EdgeRuleSetAuthoring =
            serde_json::from_value(rules.clone()).unwrap();

        assert_eq!(parsed.rules.len(), 1);
        assert_eq!(rules["rules"][0]["condition"]["path"]["type"], "glob");
        assert_eq!(
            rules["rules"][0]["condition"]["path"]["value"],
            "/_next/static/{path...}"
        );
        assert_eq!(rules["rules"][0]["action"]["type"], "set_headers");
        assert_eq!(
            rules["rules"][0]["action"]["headers"]["cache-control"],
            "public,max-age=31536000,immutable"
        );
    }

    #[test]
    fn generated_edge_rules_lower_exact_source_regex_routes() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "routing": {
                "beforeMiddleware": [{
                    "sourceRegex": "^/legacy$",
                    "headers": { "Location": "/modern" },
                    "status": 308
                }],
                "afterFiles": [{
                    "sourceRegex": "^/security.txt$",
                    "headers": { "x-next": "yes" }
                }]
            }
        }))
        .unwrap();

        let rules = descriptor
            .generated_edge_rules()
            .expect("exact sourceRegex routes should generate edge rules");
        let parsed: nrz_contract::EdgeRuleSetAuthoring =
            serde_json::from_value(rules.clone()).unwrap();

        assert_eq!(parsed.rules.len(), 2);
        assert_eq!(rules["rules"][0]["condition"]["path"]["type"], "exact");
        assert_eq!(rules["rules"][0]["condition"]["path"]["value"], "/legacy");
        assert_eq!(rules["rules"][0]["action"]["target"], "/modern");
        assert_eq!(
            rules["rules"][1]["condition"]["path"]["value"],
            "/security.txt"
        );
        assert_eq!(rules["rules"][1]["action"]["type"], "set_headers");
    }

    #[test]
    fn generated_edge_rules_skip_conditional_or_generic_regex_only_routes() {
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "routing": {
                "beforeFiles": [{
                    "source": "/conditional",
                    "destination": "/target",
                    "has": [{ "type": "header", "key": "x-route" }]
                }],
                "onMatch": [{
                    "sourceRegex": "^/regex/(.*)$",
                    "headers": { "cache-control": "public,max-age=31536000,immutable" }
                }]
            }
        }))
        .unwrap();

        assert!(descriptor.generated_edge_rules().is_none());
        let summary = descriptor.compatibility_summary();
        assert_eq!(
            summary["platform"]["routing"]["status"],
            "pending_edge_rules"
        );
        assert_eq!(summary["platform"]["routing"]["edgeRulesGenerated"], 0);
        assert_eq!(summary["platform"]["routing"]["edgeRulesUnsupported"], 2);
    }

    #[test]
    fn static_file_mapping_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("safe.js");
        std::fs::write(&src, "// safe").unwrap();
        let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
            "version": 1,
            "adapter": { "name": "@onreza/nrz-next-adapter" },
            "outputs": {
                "staticFiles": [{
                    "pathname": "/../safe.js",
                    "filePath": src,
                }]
            }
        }))
        .unwrap();

        let error = descriptor
            .static_file_mappings_for_static_layer(dir.path())
            .unwrap_err();

        assert!(error.to_string().contains("unsafe Next.js static pathname"));
    }
}
