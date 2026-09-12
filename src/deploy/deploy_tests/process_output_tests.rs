use super::*;

// ── manifest → compute type mapping tests ────────────────────
//
// Verifies the contract: primary_compute_target(manifest) → LayerTarget,
// which deploy maps as: Compute→Process, Static→Static.

#[test]
fn manifest_compute_layer_maps_to_process() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [
            {"name": "assets", "target": "STATIC", "directory": "static"},
            {"name": "server", "target": "COMPUTE", "directory": "standalone", "entry": "server.js"}
        ],
        "routes": [{"pattern": "^/.*$", "layer": "server"}]
    }"#,
    )
    .unwrap();

    let target = crate::build::manifest::primary_compute_target(&manifest);
    let compute = match target {
        crate::build::manifest::LayerTarget::Compute => ComputeType::Process,
        crate::build::manifest::LayerTarget::Static => ComputeType::Static,
    };
    assert_eq!(compute, ComputeType::Process);
}

#[test]
fn manifest_static_only_maps_to_static() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [{"name": "site", "target": "STATIC", "directory": "."}],
        "routes": [{"pattern": "^/.*$", "layer": "site"}]
    }"#,
    )
    .unwrap();

    let target = crate::build::manifest::primary_compute_target(&manifest);
    let compute = match target {
        crate::build::manifest::LayerTarget::Compute => ComputeType::Process,
        crate::build::manifest::LayerTarget::Static => ComputeType::Static,
    };
    assert_eq!(compute, ComputeType::Static);
}

// ── validate_process_output tests ────────────────────────────

#[test]
fn validate_nextjs_dot_next_without_standalone_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next");
    fs::create_dir(&output_dir).unwrap();

    let detection = make_detection("nextjs", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("output: 'standalone'"),
        "should mention standalone: {msg}"
    );
}

#[test]
fn validate_nextjs_dot_next_with_standalone_but_missing_server_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next");
    fs::create_dir(&output_dir).unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'standalone'".into()],
    };
    let detection = make_detection("nextjs", Some(ssr));
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Missing file"),
        "should mention missing file: {msg}"
    );
}

#[test]
fn validate_nextjs_standalone_dir_without_server_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next/standalone");
    fs::create_dir_all(&output_dir).unwrap();

    let detection = make_detection("nextjs", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("server.js is missing"),
        "should mention missing server.js: {msg}"
    );
}

#[test]
fn validate_nextjs_standalone_dir_with_server_ok() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next/standalone");
    fs::create_dir_all(&output_dir).unwrap();
    fs::write(output_dir.join("server.js"), "console.log('ok')").unwrap();

    let detection = make_detection("nextjs", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok());
}

#[test]
fn validate_nuxt_without_server_entry_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".output");
    fs::create_dir(&output_dir).unwrap();

    let detection = make_detection("nuxt", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("nuxi build"),
        "should mention nuxi build: {msg}"
    );
}

#[test]
fn validate_nuxt_with_server_entry_ok() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".output");
    fs::create_dir_all(output_dir.join("server")).unwrap();
    fs::write(output_dir.join("server/index.mjs"), "export default {}").unwrap();

    let detection = make_detection("nuxt", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok());
}

#[test]
fn validate_unknown_framework_ok() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();

    let detection = make_detection("vite", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok());
}

#[test]
fn validate_prebuild_process_project_rejects_cloudflare_vite_plugin() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"@cloudflare/vite-plugin":"^1.0.0"}}"#,
    )
    .unwrap();

    let err = validate_prebuild_process_project(dir.path())
        .expect_err("package-level Workers target should fail before build");
    let msg = err.to_string();
    assert!(
        msg.contains("Cloudflare Workers target detected")
            && msg.contains("@cloudflare/vite-plugin"),
        "should mention package-level Cloudflare signal: {msg}"
    );
    let coded = err
        .chain()
        .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
        .expect("prebuild framework rejection must carry a CodedError");
    assert_eq!(coded.code, "FRAMEWORK_UNSUPPORTED");
}

#[test]
fn validate_prebuild_compute_intent_skips_autodetect_process() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"@cloudflare/vite-plugin":"^1.0.0"}}"#,
    )
    .unwrap();

    let explicit_compute = resolve_explicit_compute_type(None, None).unwrap();

    assert_eq!(explicit_compute, None);
    assert!(validate_prebuild_compute_intent(dir.path(), explicit_compute).is_ok());
}

#[test]
fn resolve_deploy_compute_type_prefers_static_manifest_over_autodetect_process() {
    let manifest = crate::build::manifest::generate_static_manifest();
    let detection = make_detection("tanstack-start", None);

    let compute = resolve_deploy_compute_type(None, Some(&manifest), &detection);

    assert_eq!(compute, ComputeType::Static);
}

#[tokio::test]
async fn postbuild_detection_preserves_generated_root_static_html() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();
    let effective =
        nrz::config::EffectiveProjectConfig::from_project_config(dir.path().to_path_buf(), config);

    let stale_detection =
        crate::detect::detect_with_framework_override(dir.path(), effective.framework_override());
    assert_eq!(stale_detection.framework, "other");

    fs::write(dir.path().join("index.html"), "<h1>generated</h1>").unwrap();

    let stale_result = build::run_with_effective_config(
        BuildArgs {
            dir: dir.path().to_string_lossy().into_owned(),
            skip_validation: true,
        },
        true,
        &effective,
        Some(&stale_detection),
        false,
        dir.path(),
    )
    .await;
    assert!(
        stale_result.is_err(),
        "stale prebuild detection should miss generated root static HTML"
    );

    let postbuild_detection =
        crate::detect::detect_with_framework_override(dir.path(), effective.framework_override());
    assert_eq!(postbuild_detection.framework, "static-html");

    let result = build::run_with_effective_config(
        BuildArgs {
            dir: dir.path().to_string_lossy().into_owned(),
            skip_validation: true,
        },
        true,
        &effective,
        Some(&postbuild_detection),
        false,
        dir.path(),
    )
    .await
    .unwrap();

    assert_eq!(result.output_dir, dir.path());
    let manifest = result
        .manifest
        .expect("root static HTML should auto-generate a STATIC manifest");
    assert_eq!(compute_type_from_manifest(&manifest), ComputeType::Static);
    assert_eq!(manifest.layers[0].directory, ".");
}

#[test]
fn validate_cloudflare_vite_plugin_dep_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"@cloudflare/vite-plugin":"^1.0.0"}}"#,
    )
    .unwrap();

    let detection = make_detection("tanstack-start", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Cloudflare Workers target detected")
            && msg.contains("@cloudflare/vite-plugin"),
        "should mention CF Workers + plugin: {msg}"
    );
    assert!(
        msg.contains("--compute static") && msg.contains("nitro"),
        "should offer both escape hatches: {msg}"
    );
    let coded = err
        .chain()
        .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
        .expect("error must carry a CodedError so Builder classifies it as user-fault");
    assert_eq!(coded.code, "FRAMEWORK_UNSUPPORTED");
}

#[test]
fn validate_cloudflare_wrangler_output_bails() {
    // Fallback signal: even without the dep in package.json (e.g. pnpm workspace root),
    // the build emits server/wrangler.json and we must catch it.
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir_all(output_dir.join("server")).unwrap();
    fs::write(output_dir.join("server/wrangler.json"), r#"{"name":"x"}"#).unwrap();

    let detection = make_detection("tanstack-start", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("server/wrangler.json was emitted"),
        "should mention wrangler.json trigger: {msg}"
    );
}

#[test]
fn validate_hydrogen_oxygen_bails_via_mini_oxygen_dep() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"@shopify/mini-oxygen":"^3.0.0","@shopify/hydrogen":"^2026.0.0"}}"#,
    )
    .unwrap();

    let detection = make_detection("hydrogen", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Shopify Oxygen") && msg.contains("@shopify/mini-oxygen"),
        "should mention Oxygen + mini-oxygen: {msg}"
    );
    assert!(
        msg.contains("Express recipe"),
        "should recommend Express recipe: {msg}"
    );
}

#[test]
fn validate_hydrogen_oxygen_bails_via_output_marker() {
    // Fallback: even without the dep signal we catch Oxygen by its marker file.
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir_all(output_dir.join("server")).unwrap();
    fs::write(output_dir.join("server/oxygen.json"), r#"{"version":1}"#).unwrap();

    let detection = make_detection("hydrogen", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("server/oxygen.json was emitted"),
        "should mention oxygen.json trigger: {msg}"
    );
}

#[test]
fn validate_hydrogen_express_recipe_ok() {
    // Express recipe emits build/server/index.js plus server.mjs at project root.
    // No Oxygen signals → validate passes, entry resolution happens downstream.
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("build");
    fs::create_dir_all(output_dir.join("server")).unwrap();
    fs::write(output_dir.join("server/index.js"), "x".repeat(600)).unwrap();
    fs::write(dir.path().join("server.mjs"), "x".repeat(600)).unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"start":"node server.mjs"},"dependencies":{"express":"^4.19.0","@shopify/hydrogen":"^2026.0.0"}}"#,
    )
    .unwrap();

    let detection = make_detection("hydrogen", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok(), "Express recipe should pass: {result:?}");
}

#[test]
fn validate_malformed_package_json_bails_with_parse_error() {
    // Regression: a corrupted package.json used to silently yield "no workers
    // signal" and ship a broken PROCESS deploy. validate_process_output must
    // surface the parse error loudly so the user can fix the manifest.
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();
    fs::write(dir.path().join("package.json"), "not json{").unwrap();

    let detection = make_detection("tanstack-start", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = format!("{:#}", result.unwrap_err());
    assert!(
        msg.contains("failed to parse") && msg.contains("package.json"),
        "should report parse error: {msg}"
    );
}

#[test]
fn validate_tanstack_start_nitro_output_ok() {
    // TanStack Start with Nitro node-server preset: .output/server/index.mjs layout
    // should pass validation (no CF workers signals present).
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".output");
    fs::create_dir_all(output_dir.join("server")).unwrap();
    fs::write(output_dir.join("server/index.mjs"), "export default {}").unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@tanstack/react-start":"^1.0.0"}}"#,
    )
    .unwrap();

    let detection = make_detection("tanstack-start", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok(), "validation should pass: {result:?}");
}
