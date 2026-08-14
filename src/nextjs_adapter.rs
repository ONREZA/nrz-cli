// The parent intentionally keeps the adapter descriptor contract, lifecycle
// entry points, and compatibility aggregation together. Routing and output
// mechanics live in focused submodules so this contract remains navigable
// without scattering its cross-domain report assembly.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use regex::Regex;
use serde::Deserialize;

mod outputs;
mod routing;

use routing::*;

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
                output
                    .static_html_pathname()
                    .is_some_and(|pathname| self.static_prerender_pathname_safe(pathname))
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
        ) && !self
            .routing
            .has_compute_effect_for_path(pathname, self.has_middleware())
    }

    pub(crate) fn pathname_safe_for_public_layer(&self, pathname: &str) -> bool {
        self.pathname_safe_for_static_layer(
            pathname,
            &self.deployment_hints.middleware.public_files,
        ) && !self
            .routing
            .has_compute_effect_for_path(pathname, self.has_middleware())
    }

    fn pathname_safe_for_prerender_layer(&self, pathname: &str) -> bool {
        self.pathname_safe_for_prerender_middleware(pathname)
            && !self.pathname_has_compute_routing_effect(pathname)
    }

    fn pathname_safe_for_prerender_middleware(&self, pathname: &str) -> bool {
        !self.has_middleware() || !self.middleware_may_match_path(pathname)
    }

    fn pathname_has_compute_routing_effect(&self, pathname: &str) -> bool {
        self.routing
            .has_compute_effect_for_path(pathname, self.has_middleware())
    }

    fn static_prerender_pathname_safe(&self, pathname: &str) -> bool {
        let pathname = self.static_prerender_served_pathname(pathname);
        self.pathname_safe_for_prerender_layer(&pathname)
            && !self.routing.has_exact_redirect_for_pathname(&pathname)
    }

    fn static_prerender_has_routing_conflict(&self, pathname: &str) -> bool {
        let pathname = self.static_prerender_served_pathname(pathname);
        self.pathname_has_compute_routing_effect(&pathname)
            || self.routing.has_exact_redirect_for_pathname(&pathname)
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
                    .is_some_and(|pathname| self.static_prerender_pathname_safe(pathname))
            })
            .count();
        let middleware = self.has_middleware();
        let routing_guarded_static_files = self
            .outputs
            .static_files
            .iter()
            .any(|file| self.pathname_has_compute_routing_effect(&file.pathname));
        let routing_guarded_prerenders = self.outputs.prerenders.iter().any(|output| {
            output
                .static_html_pathname()
                .is_some_and(|pathname| self.static_prerender_has_routing_conflict(pathname))
        });
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
                    "status": if safe_static_files == counts.static_files && !middleware {
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
                    "reason": static_files_status_reason(middleware, routing_guarded_static_files, safe_static_files, counts.static_files),
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
                    "reason": prerender_status_reason(counts.prerenders, static_prerenders, isr_prerenders, ppr_prerenders, middleware, routing_guarded_prerenders),
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

    pub(crate) fn generated_remote_image_sources(&self) -> Vec<serde_json::Value> {
        if !self.image_config_uses_onreza_optimizer() {
            return Vec::new();
        }
        let Some(hint) = self
            .deployment_hints
            .image_optimizer
            .as_ref()
            .and_then(serde_json::Value::as_object)
        else {
            return Vec::new();
        };
        if hint.get("status").and_then(serde_json::Value::as_str) != Some("onreza_optimizer") {
            return Vec::new();
        }
        hint.get("remoteImageSources")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default()
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
            if route
                .to_edge_rule(bucket, index, self.has_middleware())
                .is_some()
            {
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
            .filter_map(|(bucket, index, route)| {
                route.to_edge_rule(bucket, index, self.has_middleware())
            })
            .collect()
    }
}

fn static_files_status_reason(
    middleware: bool,
    routing_guarded: bool,
    safe_static_files: usize,
    total_static_files: usize,
) -> serde_json::Value {
    if safe_static_files == total_static_files && !middleware {
        return serde_json::Value::Null;
    }
    if routing_guarded && middleware {
        return serde_json::Value::String(
            "Next.js routing or middleware can change some static/public asset responses"
                .to_string(),
        );
    }
    if routing_guarded {
        return serde_json::Value::String(
            "Next.js compute-only routing can change some static/public asset responses"
                .to_string(),
        );
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
    routing_guarded: bool,
) -> serde_json::Value {
    if total == 0 {
        return serde_json::Value::Null;
    }
    if static_prerenders == 0 && routing_guarded {
        return serde_json::Value::String(
            "Next.js compute-only routing can change prerendered responses".to_string(),
        );
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

fn normalize_next_source_regex_for_matching(source_regex: &str) -> String {
    normalize_next_source_regex_for_rust(source_regex).replace(r"(?!\.well-known(?:/.*)?)", "")
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
#[path = "nextjs_adapter_tests.rs"]
mod tests;
