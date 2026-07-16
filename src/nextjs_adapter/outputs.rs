use super::*;

impl AdapterOutputs {
    pub(super) fn route_handler_outputs(
        &self,
    ) -> impl Iterator<Item = (&'static str, &RuntimeOutput)> {
        self.pages_api
            .iter()
            .map(|output| ("PAGES_API", output))
            .chain(self.app_routes.iter().map(|output| ("APP_ROUTE", output)))
            .filter(|(_, output)| !output.is_route_handler_internal_artifact())
    }

    pub(super) fn route_handler_platform_report(&self) -> serde_json::Value {
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

    pub(super) fn runtime_outputs(&self) -> impl Iterator<Item = &RuntimeOutput> {
        self.pages
            .iter()
            .chain(self.pages_api.iter())
            .chain(self.app_pages.iter())
            .chain(self.app_routes.iter())
            .chain(self.middleware.iter())
    }

    pub(super) fn edge_runtime_output_count(&self) -> usize {
        self.runtime_outputs()
            .filter(|output| output.is_edge_runtime())
            .count()
    }

    pub(super) fn named_runtime_output_count(&self) -> usize {
        self.runtime_outputs()
            .filter(|output| output.pathname.is_some())
            .count()
    }

    pub(super) fn typed_runtime_output_count(&self) -> usize {
        self.runtime_outputs()
            .filter(|output| output.output_type.is_some())
            .count()
    }

    pub(super) fn file_runtime_output_count(&self) -> usize {
        self.runtime_outputs()
            .filter(|output| output.file_path.is_some())
            .count()
    }

    pub(super) fn immutable_static_file_count(&self) -> usize {
        self.static_files
            .iter()
            .filter(|output| output.immutable_hash.is_some())
            .count()
    }

    pub(super) fn ppr_prerender_count(&self) -> usize {
        self.prerenders
            .iter()
            .filter(|output| output.is_ppr())
            .count()
    }

    pub(super) fn isr_prerender_count(&self) -> usize {
        self.prerenders
            .iter()
            .filter(|output| output.is_isr())
            .count()
    }

    pub(super) fn prerender_fallback_file_count(&self) -> usize {
        self.prerenders
            .iter()
            .filter(|output| output.has_fallback_file())
            .count()
    }
}

impl RuntimeOutput {
    pub(super) fn is_edge_runtime(&self) -> bool {
        self.runtime.as_deref() == Some("edge") || self.edge_runtime.is_some()
    }

    pub(super) fn has_assets(&self) -> bool {
        !self.assets.is_empty() || !self.wasm_assets.is_empty()
    }

    pub(super) fn has_functions_v1_shape(&self) -> bool {
        self.runtime.as_deref() == Some("nodejs")
            && !self.is_edge_runtime()
            && !self.has_assets()
            && self.file_path.is_some()
    }

    pub(super) fn is_route_handler_internal_artifact(&self) -> bool {
        self.pathname
            .as_deref()
            .is_some_and(next_cache_internal_artifact_pathname)
    }

    pub(super) fn route_handler_report(&self, kind: &'static str) -> serde_json::Value {
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

    pub(super) fn route_handler_status(&self) -> &'static str {
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

    pub(super) fn route_handler_functions_reason(&self, status: &str) -> &'static str {
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
    pub(super) fn static_file_mapping(
        &self,
        project_dir: &Path,
    ) -> anyhow::Result<StaticFileMapping> {
        let target = pathname_to_relative_archive_path(&self.pathname)?;
        let source = canonical_next_output_source(project_dir, &self.file_path, "static file")?;

        Ok(StaticFileMapping { source, target })
    }
}

impl PrerenderOutput {
    pub(super) fn platform_route_report(
        &self,
        descriptor: &AdapterDescriptor,
    ) -> serde_json::Value {
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

    pub(super) fn platform_route_status(&self, descriptor: &AdapterDescriptor) -> &'static str {
        if self.is_ppr() {
            return "compute_fallback_ppr";
        }
        if self.is_isr() {
            return "compute_fallback_isr";
        }
        if let Some(pathname) = self.static_html_pathname() {
            if descriptor.static_prerender_has_routing_conflict(pathname) {
                return "compute_fallback_routing";
            }
            if descriptor.static_prerender_pathname_safe(pathname) {
                return "static_layer";
            }
            return "compute_fallback_middleware";
        }
        if self.has_fallback_file() {
            return "compute_fallback_non_static_fallback";
        }
        "compute_fallback"
    }

    pub(super) fn next_cache_route_report(
        &self,
        descriptor: &AdapterDescriptor,
    ) -> Option<serde_json::Value> {
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

    pub(super) fn is_next_cache_internal_artifact(&self) -> bool {
        self.pathname
            .as_deref()
            .is_some_and(next_cache_internal_artifact_pathname)
    }

    pub(super) fn next_cache_status(&self, descriptor: &AdapterDescriptor) -> &'static str {
        if self.is_ppr() {
            return "blocked_ppr_runtime";
        }
        let Some(pathname) = self.pathname.as_deref() else {
            return "blocked_missing_pathname";
        };
        if !static_prerender_pathname_supported(pathname) {
            return "blocked_unsupported_pathname";
        }
        if descriptor.pathname_has_compute_routing_effect(pathname) {
            return "blocked_by_routing";
        }
        if !descriptor.pathname_safe_for_prerender_middleware(pathname) {
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

    pub(super) fn next_cache_reason(&self, status: &str) -> serde_json::Value {
        let reason = match status {
            "edge_cache_candidate" => return serde_json::Value::Null,
            "blocked_ppr_runtime" => {
                "PPR requires cached shell bytes, postponedState, streaming resume, and Next runtime invocation"
            }
            "blocked_missing_pathname" => "Next.js prerender output has no pathname",
            "blocked_unsupported_pathname" => {
                "Next.js prerender pathname is not safe for edge cache routing"
            }
            "blocked_by_routing" => "Next.js compute-only routing may change this route",
            "blocked_by_middleware" => "Next.js middleware may run before this route",
            "blocked_missing_fallback_file" => {
                "Next.js prerender output has no fallback file to seed cache"
            }
            "blocked_non_html_fallback" => "Next.js fallback is not an HTML response",
            _ => "Next.js prerender revalidate metadata is not understood by nrz",
        };
        serde_json::Value::String(reason.to_string())
    }

    pub(super) fn middleware_safe_for_next_cache(&self, descriptor: &AdapterDescriptor) -> bool {
        self.pathname.as_deref().is_some_and(|pathname| {
            static_prerender_pathname_supported(pathname)
                && descriptor.pathname_safe_for_prerender_middleware(pathname)
        })
    }

    pub(super) fn platform_kind(&self) -> &'static str {
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

    pub(super) fn platform_route_reason(&self, status: &str) -> serde_json::Value {
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
                "Next.js routing can change this pathname before the prerendered response"
            }
            "compute_fallback_non_static_fallback" => {
                "fallback output is not a fully-static HTML response with initialRevalidate=false"
            }
            _ => "no fully-static HTML fallback file was found",
        };
        serde_json::Value::String(reason.to_string())
    }

    pub(super) fn static_prerender_mapping(
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

    pub(super) fn static_html_pathname(&self) -> Option<&str> {
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

    pub(super) fn is_ppr(&self) -> bool {
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

    pub(super) fn is_isr(&self) -> bool {
        self.initial_revalidate_seconds().is_some()
    }

    pub(super) fn initial_revalidate_seconds(&self) -> Option<u64> {
        self.fallback
            .as_ref()
            .and_then(|fallback| fallback.initial_revalidate.as_ref())
            .and_then(serde_json::Value::as_u64)
    }

    pub(super) fn has_fallback_file(&self) -> bool {
        self.fallback
            .as_ref()
            .and_then(|fallback| fallback.file_path.as_ref())
            .is_some()
    }
}

pub(super) fn prerender_route_primitive(status: &str) -> &'static str {
    if status == "static_layer" {
        "STATIC layer with fallthrough to COMPUTE"
    } else {
        "COMPUTE layer"
    }
}

impl PrerenderFallback {
    pub(super) fn is_html_response(&self) -> bool {
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

pub(super) fn pathname_to_relative_archive_path(pathname: &str) -> anyhow::Result<String> {
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

pub(super) fn prerender_pathname_to_relative_html_path(pathname: &str) -> anyhow::Result<String> {
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

pub(super) fn static_prerender_pathname_supported(pathname: &str) -> bool {
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

pub(super) fn next_cache_internal_artifact_pathname(pathname: &str) -> bool {
    pathname == "/_not-found"
        || pathname.starts_with("/_global-error")
        || pathname.starts_with("/_next/")
        || pathname.ends_with(".rsc")
        || pathname.contains(".segments/")
}

pub(super) fn canonical_next_output_source(
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
