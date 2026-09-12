use super::*;

// ── ensure_process_entry tests ───────────────────────────────

#[test]
fn ensure_process_entry_resolves_module_field() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"name":"app","module":"./server.mjs"}"#,
    )
    .unwrap();
    fs::write(dir.path().join("server.mjs"), "export default {}").unwrap();

    let detection = make_detection("other", None);
    let (entry, warning) =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).unwrap();
    assert_eq!(entry, Some("server.mjs".to_string()));
    assert!(warning.is_none());
}

#[test]
fn ensure_process_entry_ambiguous_candidates_errors_for_non_strict_framework() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("runtime")).unwrap();
    // Both files must exceed the heuristic-scan min-size threshold, otherwise
    // they're classified as ESM stubs and skipped before ambiguity kicks in.
    fs::write(
        dir.path().join("runtime/foo.mjs"),
        format!("console.log('foo') // {}", "x".repeat(600)),
    )
    .unwrap();
    fs::write(
        dir.path().join("runtime/bar.mjs"),
        format!("console.log('bar') // {}", "x".repeat(600)),
    )
    .unwrap();

    let detection = make_detection("other", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    let msg = err.to_string();
    assert!(msg.contains("ambiguous"));
    assert!(msg.contains("[deploy] entry"));
    expect_code(&err, "ENTRY_POINT_AMBIGUOUS");
}

#[test]
fn ensure_process_entry_root_prefers_server_over_main() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("server.js"), "console.log('server')").unwrap();
    fs::write(dir.path().join("main.js"), "console.log('main')").unwrap();

    let detection = make_detection("other", None);
    let (entry, warning) =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).unwrap();
    assert_eq!(entry, Some("server.js".to_string()));
    assert!(warning.is_none());
}

#[test]
fn ensure_process_entry_config_entry_allows_double_dot_in_filename() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("foo..js"), "console.log('ok')").unwrap();

    let detection = make_detection("other", None);
    let (entry, warning) =
        ensure_process_entry(dir.path(), dir.path(), Some("foo..js"), &detection, true).unwrap();
    assert_eq!(entry, Some("foo..js".to_string()));
    assert!(warning.is_none());
    assert!(!dir.path().join("package.json").exists());
}

#[test]
fn ensure_process_entry_config_entry_rejects_parent_traversal() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("server.js"), "console.log('ok')").unwrap();

    let detection = make_detection("other", None);
    let err = ensure_process_entry(
        dir.path(),
        dir.path(),
        Some("../server.js"),
        &detection,
        true,
    )
    .expect_err("parent traversal should fail");
    assert!(
        err.to_string()
            .contains("relative path within the output directory")
    );
}

#[test]
fn ensure_process_entry_config_entry_rejects_shell_command() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("index.js"), "console.log('ok')").unwrap();

    let detection = make_detection("other", None);
    for entry in ["node index.js", "python3.14 main.py"] {
        let err = ensure_process_entry(dir.path(), dir.path(), Some(entry), &detection, true)
            .expect_err("shell command entry should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("not a shell command"),
            "unexpected error: {msg}"
        );
        expect_code(&err, "INVALID_DEPLOY_ENTRY");
    }
}

#[test]
fn ensure_process_entry_not_found_errors_for_non_strict_framework() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("assets/app.css"), "body{}").unwrap();

    let detection = make_detection("other", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    let msg = err.to_string();
    assert!(msg.contains("Cannot determine entry point"));
    assert!(msg.contains("[deploy] entry"));
    assert!(!msg.contains("Falling back to runtime default"));
}

#[test]
fn ensure_process_entry_not_found_is_error_for_strict_framework() {
    let dir = tempdir().unwrap();
    let detection = make_detection("nuxt", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    assert!(
        err.to_string().contains("Nuxt PROCESS deployment expects"),
        "unexpected error: {err:#}"
    );
}

#[test]
fn ensure_process_entry_astro_client_chunks_report_missing_node_output() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir_all(output_dir.join("client/_astro")).unwrap();
    fs::write(
        output_dir.join("client/_astro/app.js"),
        format!("console.log('client') // {}", "x".repeat(600)),
    )
    .unwrap();
    fs::write(
        output_dir.join("client/_astro/runtime.js"),
        format!("console.log('runtime') // {}", "x".repeat(600)),
    )
    .unwrap();

    let detection = make_detection(
        "astro",
        Some(crate::detect::types::SsrAnalysis {
            is_static_compatible: false,
            ssr_features: vec!["SSR adapter integration".into()],
        }),
    );
    let error = ensure_process_entry(&output_dir, dir.path(), None, &detection, true)
        .expect_err("Astro client chunks are not a PROCESS entry point");
    let message = error.to_string();

    expect_code(&error, "MISSING_PROCESS_ENTRY");
    assert!(message.contains("Astro Node adapter"), "{message}");
    assert!(message.contains("dist/server/entry.mjs"), "{message}");
    assert!(!message.contains("client/_astro/app.js"), "{message}");
}

#[test]
fn ensure_process_entry_not_found_is_error_for_hydrogen() {
    // Regression: hydrogen lost its FrameworkHint in this PR; without strict
    // handling, a hydrogen project with no buildable entry would silently fall
    // back to `bun <output>` and 404. Must bail with Hydrogen diagnostic.
    let dir = tempdir().unwrap();
    let detection = make_detection("hydrogen", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    let msg = err.to_string();
    assert!(
        msg.contains("Hydrogen PROCESS") && msg.contains("Express recipe"),
        "expected Hydrogen diagnostic, got: {msg}"
    );
}

#[test]
fn ensure_process_entry_not_found_is_error_for_tanstack_start() {
    let dir = tempdir().unwrap();
    let detection = make_detection("tanstack-start", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    let msg = err.to_string();
    assert!(
        msg.contains("TanStack Start PROCESS") && msg.contains("server/index.mjs"),
        "expected TSS diagnostic, got: {msg}"
    );
}

#[test]
fn is_strict_process_framework_covers_all_ssr() {
    // All SSR frameworks must be strict — falling back to `bun <output>` for
    // a framework we claim to support is the exact silent-404 failure mode
    // this PR was created to eliminate.
    for framework in [
        "nextjs",
        "nuxt",
        "sveltekit",
        "astro",
        "remix",
        "react-router",
        "solidstart",
        "qwik",
        "analog",
        "blitzjs",
        "payload",
        "tanstack-start",
        "hydrogen",
    ] {
        assert!(
            is_strict_process_framework(framework),
            "{framework} must be strict"
        );
    }
    // Unknown / non-SSR frameworks stay non-strict (generic server projects
    // may legitimately want the bun fallback).
    assert!(!is_strict_process_framework("other"));
    assert!(!is_strict_process_framework("vite"));
    assert!(!is_strict_process_framework("hono"));
}

// ── COMPUTE auto-gen bail: entry not found ────────────────────

#[test]
fn ensure_process_entry_not_found_is_the_bail_precondition() {
    // Verify that a project with no runnable files fails before COMPUTE manifest
    // auto-generation instead of advertising a runtime default that deploy cannot
    // actually encode.
    let dir = tempdir().unwrap();
    // Create a "dist" output dir with no .js/.mjs/.cjs files
    fs::create_dir(dir.path().join("dist")).unwrap();
    fs::write(dir.path().join("dist/style.css"), "body{}").unwrap();

    let detection = make_detection("other", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    assert!(
        err.to_string().contains("Cannot determine entry point"),
        "unexpected error: {err:#}"
    );
}

// ── framework_process_diagnostic tests ───────────────────────

#[test]
fn diagnostic_nextjs_no_standalone_suggests_config() {
    let dir = tempdir().unwrap();
    let detection = make_detection("nextjs", None);
    let msg = framework_process_diagnostic("nextjs", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("output: 'standalone'"));
}

#[test]
fn diagnostic_nextjs_standalone_mentions_server_js() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next/standalone");
    fs::create_dir_all(&output_dir).unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'standalone'".into()],
    };
    let detection = make_detection("nextjs", Some(ssr));
    let msg = framework_process_diagnostic("nextjs", &detection, &output_dir);
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("server.js"));
}

#[test]
fn diagnostic_nuxt_mentions_nuxi_build() {
    let dir = tempdir().unwrap();
    let detection = make_detection("nuxt", None);
    let msg = framework_process_diagnostic("nuxt", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("nuxi build"));
}

#[test]
fn diagnostic_sveltekit_mentions_adapter_node() {
    let dir = tempdir().unwrap();
    let detection = make_detection("sveltekit", None);
    let msg = framework_process_diagnostic("sveltekit", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("adapter-node"));
}

#[test]
fn diagnostic_unknown_framework_returns_none() {
    let dir = tempdir().unwrap();
    let detection = make_detection("vite", None);
    let msg = framework_process_diagnostic("vite", &detection, dir.path());
    assert!(msg.is_none());
}

#[test]
fn diagnostic_react_router_mentions_server_index() {
    let dir = tempdir().unwrap();
    let detection = make_detection("react-router", None);
    let msg = framework_process_diagnostic("react-router", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("server/index.js"));
}

#[test]
fn diagnostic_remix_mentions_server_index() {
    let dir = tempdir().unwrap();
    let detection = make_detection("remix", None);
    let msg = framework_process_diagnostic("remix", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("server/index.js"));
}

#[test]
fn diagnostic_hono_mentions_entry_point() {
    let dir = tempdir().unwrap();
    let detection = make_detection("hono", None);
    let msg = framework_process_diagnostic("hono", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("dist/"));
}

#[test]
fn diagnostic_elysia_mentions_bun() {
    let dir = tempdir().unwrap();
    let detection = make_detection("elysia", None);
    let msg = framework_process_diagnostic("elysia", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("Bun"));
}
