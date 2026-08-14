// This remains one single-domain contract suite: the fixtures exercise the
// same adapter descriptor across routing, middleware, cache, and static output
// reports, so splitting them would duplicate setup and obscure coverage.
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
fn adapter_normalizes_remote_patterns_without_widening_next_semantics() {
    let script = format!(
        r#"
const adapter = require({adapter_path});
const normalize = adapter.__test.buildRemoteImageSources;
const loader = new Function(adapter.__test.imageLoaderSource({{}}).replace('export default ', '') + '\nreturn onrezaImageLoader')();
const result = {{
  omittedSearch: normalize({{ remotePatterns: [{{ protocol: 'https', hostname: 'cdn.example.com', pathname: '/images/**' }}] }}),
  exactDefaultPort: normalize({{ remotePatterns: [{{ protocol: 'https', hostname: 'cdn.example.com', port: '', pathname: '/images/**' }}] }}),
  exactSearch: normalize({{ remotePatterns: [new URL('https://cdn.example.com/images/**?v=1')] }}),
  unicode: normalize({{ remotePatterns: [new URL('https://Изображения.РФ/**')] }}),
  canonicalObjectHostname: normalize({{ remotePatterns: [{{ protocol: 'https', hostname: 'cdn.example.com', port: '', pathname: '/**' }}] }}),
  uppercaseObjectHostname: normalize({{ remotePatterns: [{{ protocol: 'https', hostname: 'CDN.EXAMPLE.COM', port: '', pathname: '/**' }}] }}),
  paddedObjectHostname: normalize({{ remotePatterns: [{{ protocol: 'https', hostname: ' cdn.example.com ', port: '', pathname: '/**' }}] }}),
  legacyDomains: normalize({{ domains: ['cdn.example.com'] }}),
  encodedTraversal: normalize({{ remotePatterns: [{{ protocol: 'https', hostname: 'cdn.example.com', pathname: '/safe/%2e%2e/**' }}] }}),
  insecure: normalize({{ remotePatterns: [{{ protocol: 'http', hostname: 'cdn.example.com', pathname: '/**' }}] }}),
  localIpDecision: adapter.__test.imageOptimizerDecision({{ images: {{ dangerouslyAllowLocalIP: true }} }}),
  redirectDecision: adapter.__test.imageOptimizerDecision({{ images: {{ maximumRedirects: 0 }} }}),
  defaultQualitiesDecision: adapter.__test.imageOptimizerDecision({{ images: {{ qualities: [75] }} }}),
  customQualitiesDecision: adapter.__test.imageOptimizerDecision({{ images: {{ qualities: [60, 75] }} }}),
  upperHttpsLoader: loader({{ src: 'HTTPS://cdn.example.com/image.png', width: 640 }}),
}};
process.stdout.write(JSON.stringify(result));
"#,
        adapter_path = serde_json::to_string(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets/next-adapter/onreza-next-adapter.cjs")
                .display()
                .to_string()
        )
        .unwrap()
    );
    let output = std::process::Command::new("node")
        .args(["-e", &script])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(result["omittedSearch"].is_null());
    assert!(result["exactDefaultPort"][0].get("search").is_none());
    assert_eq!(result["exactSearch"][0]["search"], "?v=1");
    assert_eq!(
        result["unicode"][0]["hostname"],
        "xn--80abndcff3bev4o.xn--p1ai"
    );
    assert_eq!(
        result["canonicalObjectHostname"][0]["hostname"],
        "cdn.example.com"
    );
    assert!(result["uppercaseObjectHostname"].is_null());
    assert!(result["paddedObjectHostname"].is_null());
    assert!(result["legacyDomains"].is_null());
    assert!(result["encodedTraversal"].is_null());
    assert!(result["insecure"].is_null());
    assert_eq!(result["localIpDecision"]["status"], "compute_fallback");
    assert_eq!(result["redirectDecision"]["status"], "compute_fallback");
    assert_eq!(
        result["defaultQualitiesDecision"]["status"],
        "onreza_optimizer"
    );
    assert_eq!(
        result["customQualitiesDecision"]["status"],
        "compute_fallback"
    );
    assert_eq!(
        result["upperHttpsLoader"],
        "/_onreza/image?url=HTTPS%3A%2F%2Fcdn.example.com%2Fimage.png&w=640&q=75"
    );
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
        "Next.js routing can change this pathname before the prerendered response"
    );
}

#[test]
fn static_prerender_mapping_uses_trailing_slash_served_path_for_routing_guard() {
    let dir = tempfile::tempdir().unwrap();
    let docs = dir.path().join(".next/server/app/index.html");
    std::fs::create_dir_all(docs.parent().unwrap()).unwrap();
    std::fs::write(&docs, "<main>docs</main>").unwrap();

    let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
        "version": 1,
        "adapter": { "name": "@onreza/nrz-next-adapter" },
        "config": { "trailingSlash": true },
        "routing": {
            "beforeMiddleware": [{
                "source": "/:notfile((?!\\.well-known(?:/.*)?)(?:[^/]+/)*[^/\\.]+)",
                "sourceRegex": "^(?:\\/((?!\\.well-known(?:\\/.*)?)(?:[^/]+\\/)*[^/\\.]+))$",
                "headers": { "Location": "/$1/" },
                "status": 308,
                "priority": true
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

    assert_eq!(mappings.len(), 1);
    assert_eq!(mappings[0].pathname, "/docs/");
}

#[test]
fn static_outputs_do_not_shadow_compute_only_next_routing() {
    let dir = tempfile::tempdir().unwrap();
    let asset = dir.path().join("public/late.txt");
    let prerender = dir.path().join(".next/server/app/late.html");
    std::fs::create_dir_all(asset.parent().unwrap()).unwrap();
    std::fs::create_dir_all(prerender.parent().unwrap()).unwrap();
    std::fs::write(&asset, "asset").unwrap();
    std::fs::write(&prerender, "<main>late</main>").unwrap();

    let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
        "version": 1,
        "adapter": { "name": "@onreza/nrz-next-adapter" },
        "routing": {
            "afterFiles": [
                { "source": "/late.txt", "destination": "/dynamic-asset" },
                { "source": "/late", "destination": "/dynamic-page" }
            ]
        },
        "outputs": {
            "staticFiles": [{
                "pathname": "/late.txt",
                "filePath": asset
            }],
            "prerenders": [{
                "type": "PRERENDER",
                "pathname": "/late",
                "fallback": {
                    "filePath": prerender,
                    "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                    "initialRevalidate": false
                }
            }]
        }
    }))
    .unwrap();

    assert!(
        descriptor
            .static_file_mappings_for_static_layer(dir.path())
            .unwrap()
            .is_empty()
    );
    assert!(
        descriptor
            .static_prerender_mappings_for_static_layer(dir.path())
            .unwrap()
            .is_empty()
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
    let parsed: nrz_contract::EdgeRuleSetAuthoring = serde_json::from_value(rules.clone()).unwrap();

    assert_eq!(parsed.rules.len(), 2);
    assert_eq!(rules["rules"][0]["action"]["type"], "redirect");
    assert_eq!(rules["rules"][0]["action"]["target"], "/new");
    assert_eq!(rules["rules"][0]["action"]["ifNoFile"], false);
    assert_eq!(rules["rules"][1]["condition"]["path"]["type"], "glob");
    assert_eq!(
        rules["rules"][1]["condition"]["path"]["value"],
        "/docs/{slug}"
    );
    assert_eq!(rules["rules"][1]["action"]["target"], "/help/{slug}");
    assert_eq!(rules["rules"][1]["action"]["ifNoFile"], false);

    let summary = descriptor.compatibility_summary();
    assert_eq!(
        summary["platform"]["routing"]["status"],
        "partial_edge_rules"
    );
    assert_eq!(summary["platform"]["routing"]["edgeRulesGenerated"], 2);
    assert_eq!(summary["platform"]["routing"]["edgeRulesUnsupported"], 2);
}

#[test]
fn generated_remote_image_sources_read_normalized_adapter_hint() {
    let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
        "version": 1,
        "adapter": { "name": "@onreza/nrz-next-adapter" },
        "config": {
            "images": {
                "loader": "custom",
                "loaderFile": "./.onreza/cache/next-adapter/onreza-image-loader.mjs",
            }
        },
        "deploymentHints": {
            "imageOptimizer": {
                "status": "onreza_optimizer",
                "remoteImageSources": [
                    {
                        "id": "next.images.domain.0",
                        "protocol": "https",
                        "hostname": "legacy.example.com",
                        "pathname": "/**"
                    },
                    {
                        "id": "next.images.remote-pattern.0",
                        "protocol": "https",
                        "hostname": "*.assets.example.com",
                        "pathname": "/tenants/*/**"
                    },
                    {
                        "id": "next.images.remote-pattern.1",
                        "protocol": "https",
                        "hostname": "media.example.net",
                        "pathname": "/account/**",
                        "search": "?version=1"
                    },
                    {
                        "id": "next.images.remote-pattern.2",
                        "protocol": "https",
                        "hostname": "static.example.org",
                        "pathname": "/public/**",
                        "search": ""
                    }
                ]
            }
        }
    }))
    .unwrap();

    let image_sources = descriptor.generated_remote_image_sources();
    let edge_rules = serde_json::json!({
        "schemaVersion": "EDGE_RULE_SET_V1",
        "imageSources": image_sources,
        "rules": [],
    });
    crate::functions::validate_edge_rules_value(
        "Next.js adapter generated remote image sources",
        &edge_rules,
    )
    .unwrap();

    assert_eq!(edge_rules["imageSources"].as_array().unwrap().len(), 4);
    assert_eq!(edge_rules["imageSources"][0]["id"], "next.images.domain.0");
    assert_eq!(edge_rules["imageSources"][0]["pathname"], "/**");
    assert_eq!(
        edge_rules["imageSources"][1]["hostname"],
        "*.assets.example.com"
    );
    assert!(edge_rules["imageSources"][1].get("search").is_none());
    assert_eq!(edge_rules["imageSources"][2]["search"], "?version=1");
    assert_eq!(edge_rules["imageSources"][3]["search"], "");
}

#[test]
fn generated_remote_image_sources_do_not_widen_unsupported_config() {
    let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
        "version": 1,
        "adapter": { "name": "@onreza/nrz-next-adapter" },
        "config": {
            "images": {
                "loader": "custom",
                "loaderFile": "./.onreza/cache/next-adapter/onreza-image-loader.mjs",
            }
        },
        "deploymentHints": {
            "imageOptimizer": {
                "status": "compute_fallback",
                "remoteImageSources": [{
                    "id": "spoofed",
                    "protocol": "https",
                    "hostname": "images.example.com",
                    "pathname": "/**"
                }]
            }
        }
    }))
    .unwrap();

    assert!(descriptor.generated_remote_image_sources().is_empty());
}

#[test]
fn generated_edge_rules_keep_after_files_rewrites_in_next_compute() {
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

    assert!(descriptor.generated_edge_rules().is_none());
    assert_eq!(
        descriptor.edge_rule_lowering_counts(),
        AdapterEdgeRuleLoweringCounts {
            generated: 0,
            unsupported: 1,
        }
    );
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
    let parsed: nrz_contract::EdgeRuleSetAuthoring = serde_json::from_value(rules.clone()).unwrap();

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
    let parsed: nrz_contract::EdgeRuleSetAuthoring = serde_json::from_value(rules.clone()).unwrap();

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
    let parsed: nrz_contract::EdgeRuleSetAuthoring = serde_json::from_value(rules.clone()).unwrap();

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
fn generated_edge_rules_keep_plain_http_rewrites_in_next_compute() {
    let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
        "version": 1,
        "adapter": { "name": "@onreza/nrz-next-adapter" },
        "routing": {
            "beforeFiles": [{
                "source": "/api/:path*",
                "destination": "http://212.67.15.151:8000/:path*"
            }]
        }
    }))
    .unwrap();

    assert!(descriptor.generated_edge_rules().is_none());
    assert_eq!(
        descriptor.edge_rule_lowering_counts(),
        AdapterEdgeRuleLoweringCounts {
            generated: 0,
            unsupported: 1,
        }
    );
    assert_eq!(
        descriptor.compatibility_summary()["platform"]["routing"]["status"],
        "pending_edge_rules"
    );
}

#[test]
fn generated_edge_rules_keep_invalid_internal_targets_in_next_compute() {
    let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
        "version": 1,
        "adapter": { "name": "@onreza/nrz-next-adapter" },
        "routing": {
            "beforeFiles": [{
                "source": "/proxy",
                "destination": "/internal?url=http://origin.example.test"
            }]
        }
    }))
    .unwrap();

    assert!(descriptor.generated_edge_rules().is_none());
    assert_eq!(
        descriptor.edge_rule_lowering_counts(),
        AdapterEdgeRuleLoweringCounts {
            generated: 0,
            unsupported: 1,
        }
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
    let parsed: nrz_contract::EdgeRuleSetAuthoring = serde_json::from_value(rules.clone()).unwrap();

    assert_eq!(parsed.rules.len(), 1);
    assert_eq!(rules["rules"][0]["condition"]["path"]["type"], "exact");
    assert_eq!(rules["rules"][0]["condition"]["path"]["value"], "/legacy");
    assert_eq!(rules["rules"][0]["action"]["target"], "/modern");
    assert_eq!(
        descriptor.edge_rule_lowering_counts(),
        AdapterEdgeRuleLoweringCounts {
            generated: 1,
            unsupported: 1,
        }
    );
}

#[test]
fn generated_edge_rules_keep_before_files_in_compute_when_middleware_exists() {
    let descriptor: AdapterDescriptor = serde_json::from_value(serde_json::json!({
        "version": 1,
        "adapter": { "name": "@onreza/nrz-next-adapter" },
        "routing": {
            "beforeFiles": [{
                "source": "/docs/:slug",
                "destination": "/help/:slug"
            }]
        },
        "outputs": {
            "middleware": {
                "type": "MIDDLEWARE",
                "runtime": "nodejs",
                "config": { "matchers": [] }
            }
        }
    }))
    .unwrap();

    assert!(descriptor.generated_edge_rules().is_none());
    assert_eq!(descriptor.edge_rule_lowering_counts().unsupported, 1);
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
