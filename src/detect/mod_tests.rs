use super::*;
use types::*;

// ── infer_compute_type ──────────────────────────────────────

#[test]
fn static_runtime_always_static() {
    assert_eq!(
        infer_compute_type(RuntimeType::Static, "static-html", None, None),
        ComputeType::Static
    );
}

#[test]
fn non_ssr_framework_always_static() {
    // Vite, CRA, Gatsby, etc. are not SSR frameworks → STATIC
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "vite", None, None),
        ComputeType::Static
    );
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "cra", None, None),
        ComputeType::Static
    );
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "gatsby", None, None),
        ComputeType::Static
    );
}

#[test]
fn ssr_framework_explicit_static_export_no_adapter_is_static() {
    // Explicitly configured: output: 'export' → has_ssr_features: true, is_static_compatible: true
    let ssr = SsrAnalysis {
        is_static_compatible: true,
        ssr_features: vec!["output: export".into()],
    };
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "nextjs", Some(&ssr), None),
        ComputeType::Static
    );
}

#[test]
fn ssr_framework_clean_project_is_process() {
    // No SSR features detected, but SSR framework defaults to PROCESS
    let ssr = SsrAnalysis {
        is_static_compatible: true,
        ssr_features: vec![],
    };
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "nextjs", Some(&ssr), None),
        ComputeType::Process
    );
}

#[test]
fn ssr_framework_with_adapter_is_isolate() {
    let ssr = SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["middleware".into()],
    };
    let adapter = AdapterInfo {
        adapter_package: "@onreza/adapter-nextjs".into(),
        adapter_version: Some("1.0.0".into()),
    };
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "nextjs", Some(&ssr), Some(&adapter)),
        ComputeType::Isolate
    );
}

#[test]
fn ssr_framework_static_export_with_adapter_is_isolate() {
    let ssr = SsrAnalysis {
        is_static_compatible: true,
        ssr_features: vec![],
    };
    let adapter = AdapterInfo {
        adapter_package: "@onreza/adapter-nextjs".into(),
        adapter_version: None,
    };
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "nextjs", Some(&ssr), Some(&adapter)),
        ComputeType::Isolate
    );
}

#[test]
fn ssr_framework_no_adapter_is_process() {
    let ssr = SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["standalone".into()],
    };
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "nextjs", Some(&ssr), None),
        ComputeType::Process
    );
}

#[test]
fn ssr_framework_no_analysis_no_adapter_is_process() {
    // No SSR analysis available, SSR framework → PROCESS
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "nextjs", None, None),
        ComputeType::Process
    );
}

#[test]
fn ssr_framework_no_analysis_with_adapter_is_isolate() {
    let adapter = AdapterInfo {
        adapter_package: "@onreza/adapter-nuxt".into(),
        adapter_version: None,
    };
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "nuxt", None, Some(&adapter)),
        ComputeType::Isolate
    );
}

#[test]
fn nuxt_without_adapter_is_process() {
    let ssr = SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["server/api".into()],
    };
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "nuxt", Some(&ssr), None),
        ComputeType::Process
    );
}

#[test]
fn sveltekit_with_adapter_is_isolate() {
    let ssr = SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["+server routes".into()],
    };
    let adapter = AdapterInfo {
        adapter_package: "@onreza/adapter-sveltekit".into(),
        adapter_version: Some("0.1.0".into()),
    };
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "sveltekit", Some(&ssr), Some(&adapter)),
        ComputeType::Isolate
    );
}

// ── Full detect() integration tests ─────────────────────────

#[test]
fn detect_nextjs_returns_process_by_default() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"next": "14.0.0", "react": "18.0.0"}}"#,
    )
    .unwrap();

    let result = detect(dir.path());
    assert_eq!(result.framework, "nextjs");
    // Next.js without adapter → PROCESS
    assert_eq!(result.suggested_compute, ComputeType::Process);
}

#[test]
fn detect_nextjs_static_export_is_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"next": "14.0.0", "react": "18.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("next.config.js"),
        "module.exports = { output: 'export' }",
    )
    .unwrap();

    let result = detect(dir.path());
    assert_eq!(result.framework, "nextjs");
    assert_eq!(result.suggested_compute, ComputeType::Static);
}

#[test]
fn detect_nextjs_with_adapter_is_isolate() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"next": "14.0.0", "react": "18.0.0", "@onreza/adapter-nextjs": "1.0.0"}}"#,
    )
    .unwrap();

    let result = detect(dir.path());
    assert_eq!(result.framework, "nextjs");
    assert_eq!(result.suggested_compute, ComputeType::Isolate);
}

#[test]
fn detect_vite_is_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"vite": "5.0.0"}}"#,
    )
    .unwrap();

    let result = detect(dir.path());
    assert_eq!(result.framework, "vite");
    assert_eq!(result.suggested_compute, ComputeType::Static);
}

#[test]
fn detect_astro_is_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"astro": "4.0.0"}}"#,
    )
    .unwrap();

    let result = detect(dir.path());
    assert_eq!(result.framework, "astro");
    assert_eq!(result.suggested_compute, ComputeType::Static);
}

#[test]
fn detect_static_html_is_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("index.html"),
        "<html><body>hello</body></html>",
    )
    .unwrap();

    let result = detect(dir.path());
    assert_eq!(result.framework, "static-html");
    assert_eq!(result.suggested_compute, ComputeType::Static);
}

#[test]
fn detect_unknown_is_static() {
    let dir = tempfile::tempdir().unwrap();
    let result = detect(dir.path());
    assert_eq!(result.framework, "other");
    assert_eq!(result.suggested_compute, ComputeType::Static);
}

#[test]
fn detect_nuxt_with_server_api_is_process() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"nuxt": "3.0.0"}}"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("server/api")).unwrap();
    std::fs::write(
        dir.path().join("server/api/hello.ts"),
        "export default defineEventHandler()",
    )
    .unwrap();

    let result = detect(dir.path());
    assert_eq!(result.framework, "nuxt");
    assert_eq!(result.suggested_compute, ComputeType::Process);
}

#[test]
fn detect_nuxt_with_adapter_is_isolate() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"nuxt": "3.0.0", "@onreza/adapter-nuxt": "0.1.0"}}"#,
    )
    .unwrap();

    let result = detect(dir.path());
    assert_eq!(result.framework, "nuxt");
    assert_eq!(result.suggested_compute, ComputeType::Isolate);
}

// ── Fix 1: non-SSR framework with adapter → ISOLATE ──────────

#[test]
fn non_ssr_framework_with_adapter_is_isolate() {
    // Astro + @onreza/adapter-astro → ISOLATE, not STATIC
    let adapter = AdapterInfo {
        adapter_package: "@onreza/adapter-astro".into(),
        adapter_version: Some("1.0.0".into()),
    };
    assert_eq!(
        infer_compute_type(RuntimeType::Node, "astro", None, Some(&adapter)),
        ComputeType::Isolate
    );
}

#[test]
fn detect_astro_with_adapter_is_isolate() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"astro": "4.0.0", "@onreza/adapter-astro": "1.0.0"}}"#,
    )
    .unwrap();

    let result = detect(dir.path());
    assert_eq!(result.framework, "astro");
    assert_eq!(result.suggested_compute, ComputeType::Isolate);
}

// ── Fix 12: multiple frameworks — highest priority wins ──────

#[test]
fn multiple_frameworks_highest_priority_wins() {
    let dir = tempfile::tempdir().unwrap();
    // next (priority 1) + vite (priority 100) → nextjs wins
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"next": "14.0.0", "vite": "5.0.0", "react": "18.0.0"}}"#,
    )
    .unwrap();

    let result = detect(dir.path());
    assert_eq!(result.framework, "nextjs");
}

#[test]
fn nuxt_wins_over_vue_cli() {
    let dir = tempfile::tempdir().unwrap();
    // nuxt (priority 2) + @vue/cli-service (priority 11) → nuxt wins
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"nuxt": "3.0.0", "@vue/cli-service": "5.0.0"}}"#,
    )
    .unwrap();

    let result = detect(dir.path());
    assert_eq!(result.framework, "nuxt");
}

// ── framework_entry_point ────────────────────────────────────

#[test]
fn framework_entry_point_nextjs() {
    assert_eq!(framework_entry_point("nextjs"), Some("server.js".into()));
}

#[test]
fn framework_entry_point_nuxt() {
    assert_eq!(
        framework_entry_point("nuxt"),
        Some("server/index.mjs".into())
    );
}

#[test]
fn framework_entry_point_sveltekit() {
    assert_eq!(framework_entry_point("sveltekit"), Some("index.js".into()));
}

#[test]
fn framework_entry_point_unknown_returns_none() {
    assert_eq!(framework_entry_point("vite"), None);
    assert_eq!(framework_entry_point("astro"), None);
    assert_eq!(framework_entry_point("other"), None);
}

// ── resolve_entry_point ──────────────────────────────────────

#[test]
fn resolve_entry_point_framework_specific() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("server.js"), "module.exports = {}").unwrap();

    let result = resolve_entry_point("nextjs", dir.path(), dir.path());
    assert_eq!(result, Some("server.js".into()));
}

#[test]
fn resolve_entry_point_framework_file_missing_falls_through() {
    // Next.js detected but server.js doesn't exist → fallback
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.js"), "").unwrap();

    let result = resolve_entry_point("nextjs", dir.path(), dir.path());
    assert_eq!(result, Some("index.js".into()));
}

#[test]
fn resolve_entry_point_package_json_main() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"main": "dist/app.js"}"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("dist")).unwrap();
    std::fs::write(dir.path().join("dist/app.js"), "").unwrap();

    let result = resolve_entry_point("other", dir.path(), dir.path());
    assert_eq!(result, Some("dist/app.js".into()));
}

#[test]
fn resolve_entry_point_package_json_main_with_dot_slash() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"main": "./server.js"}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("server.js"), "").unwrap();

    let result = resolve_entry_point("other", dir.path(), dir.path());
    assert_eq!(result, Some("server.js".into()));
}

#[test]
fn resolve_entry_point_package_json_main_path_traversal_rejected() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"main": "../../etc/passwd"}"#,
    )
    .unwrap();

    let result = resolve_entry_point("other", dir.path(), dir.path());
    // Should not return the traversal path, falls through to fallback
    assert_ne!(result, Some("../../etc/passwd".into()));
}

#[test]
fn resolve_entry_point_project_dir_package_json() {
    // output_dir != project_dir, package.json in project_dir
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("package.json"),
        r#"{"main": "app.js"}"#,
    )
    .unwrap();
    std::fs::write(output.path().join("app.js"), "").unwrap();

    let result = resolve_entry_point("other", output.path(), project.path());
    assert_eq!(result, Some("app.js".into()));
}

#[test]
fn resolve_entry_point_fallback_index_ts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.ts"), "").unwrap();

    let result = resolve_entry_point("other", dir.path(), dir.path());
    assert_eq!(result, Some("index.ts".into()));
}

#[test]
fn resolve_entry_point_fallback_server_js() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("server.js"), "").unwrap();

    let result = resolve_entry_point("other", dir.path(), dir.path());
    assert_eq!(result, Some("server.js".into()));
}

#[test]
fn resolve_entry_point_fallback_src_index() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/index.ts"), "").unwrap();

    let result = resolve_entry_point("other", dir.path(), dir.path());
    assert_eq!(result, Some("src/index.ts".into()));
}

#[test]
fn resolve_entry_point_fallback_priority_order() {
    // index.ts should win over server.js
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.ts"), "").unwrap();
    std::fs::write(dir.path().join("server.js"), "").unwrap();

    let result = resolve_entry_point("other", dir.path(), dir.path());
    assert_eq!(result, Some("index.ts".into()));
}

#[test]
fn resolve_entry_point_empty_dir_returns_none() {
    let dir = tempfile::tempdir().unwrap();

    let result = resolve_entry_point("other", dir.path(), dir.path());
    assert_eq!(result, None);
}

#[test]
fn resolve_entry_point_framework_takes_priority_over_package_json() {
    // Next.js server.js should win over package.json "main"
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"main": "custom.js"}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("server.js"), "").unwrap();
    std::fs::write(dir.path().join("custom.js"), "").unwrap();

    let result = resolve_entry_point("nextjs", dir.path(), dir.path());
    assert_eq!(result, Some("server.js".into()));
}
