use super::*;

#[test]
fn prepare_deploy_files_keeps_python_dependencies_and_prunes_platform_metadata() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".onreza/python/3.14/site-packages/orjson")).unwrap();
    fs::write(dir.path().join("main.py"), "import orjson").unwrap();
    fs::write(
        dir.path()
            .join(".onreza/python/3.14/site-packages/orjson/__init__.py"),
        "loads = lambda value: value",
    )
    .unwrap();
    fs::write(dir.path().join(".onreza/manifest.json"), "{}").unwrap();

    let detection = crate::detect::detect_with_framework_override(dir.path(), None);
    let manifest = build_manifest::generate_compute_manifest("main.py");
    let scanned =
        scan_runtime_artifact(dir.path(), &RuntimeArtifactScan::PythonRuntimeRoot).unwrap();
    let collection = prepare_artifact_files(
        &manifest,
        scanned,
        &detection,
        crate::artifact::ArtifactRootScope::ProjectRoot,
        true,
    );
    let deployable = collection.deployable_entries();
    let paths = deployable
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();

    assert_eq!(detection.metadata.runtime.runtime_type, RuntimeType::Python);
    assert!(paths.contains(&"main.py".to_string()));
    assert!(paths.contains(&".onreza/python/3.14/site-packages/orjson/__init__.py".to_string()));
    assert!(!paths.contains(&".onreza/manifest.json".to_string()));
    assert_eq!(collection.summary.deployable_files, 2);
    assert_eq!(collection.summary.pruned_files, 1);

    let plan = source_bundle_v1::build_source_bundle_plan_with_scan(
        dir.path(),
        &manifest,
        &deployable,
        &RuntimeArtifactScan::PythonRuntimeRoot,
        crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::TrustedMaterialization,
        Some(crate::artifact::source_bundle_v1::RuntimeReadinessContract::Http("/healthz")),
    )
    .unwrap();
    assert_eq!(
        plan.logical_manifest.layers[0].runtime_config,
        Some(serde_json::json!({
            "readiness": {"path": "/healthz", "protocol": "HTTP"},
            "runtimeFamily": "PYTHON"
        }))
    );
}

#[test]
fn pnpm_install_preserves_project_build_script_policy() {
    let dir = tempdir().unwrap();

    let (cmd, env) = prepare_install_command("pnpm install", dir.path(), true);

    assert_eq!(cmd, "pnpm install");
    assert!(env.is_empty());
}

#[test]
fn prepare_deploy_files_prunes_next_cache_only() {
    let manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            {"name": "server", "target": "COMPUTE", "directory": ".", "entry": "server.js"}
        ],
        "routes": [
            {"pattern": "^/.*$", "layer": "server"}
        ]
    }))
    .unwrap();
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"next":"15.0.0","react":"19.0.0"}}"#,
    )
    .unwrap();
    let detection = crate::detect::detect_with_framework_override(dir.path(), None);
    let files = vec![
        fe(".next/cache/webpack/client.json", 10, "aa"),
        fe(
            "node_modules/@next/swc-linux-x64-gnu/package.json",
            20,
            "bb",
        ),
        fe("server.js", 30, "cc"),
    ];

    let deployable = prepare_deploy_files(&manifest, files, &detection, true).unwrap();
    let paths = deployable
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        paths,
        vec![
            "node_modules/@next/swc-linux-x64-gnu/package.json",
            "server.js"
        ]
    );
}

#[test]
fn prepare_deploy_files_prunes_root_static_metadata() {
    let manifest = build_manifest::generate_static_manifest();
    let detection = make_detection("static-html", None);
    let files = vec![
        fe("index.html", 10, "aa"),
        fe("assets/app.js", 20, "bb"),
        fe(".onreza/manifest.json", 30, "cc"),
        fe("package.json", 40, "dd"),
        fe("node_modules/pkg/index.js", 50, "ee"),
        fe(".env.local", 60, "ff"),
        fe("onreza.toml", 70, "gg"),
    ];

    let deployable = prepare_deploy_files(&manifest, files, &detection, true).unwrap();
    let paths = deployable
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec!["index.html", "assets/app.js"]);
}

#[test]
fn prepare_deploy_files_keeps_static_build_output_node_modules_assets() {
    let output = tempdir().unwrap();
    fs::create_dir_all(output.path().join("node_modules/pkg")).unwrap();
    fs::write(
        output.path().join("index.html"),
        r#"<script type="module" src="/node_modules/pkg/index.js"></script>"#,
    )
    .unwrap();
    fs::write(
        output.path().join("node_modules/pkg/index.js"),
        "export const ok = true;",
    )
    .unwrap();

    let manifest = build_manifest::generate_static_manifest();
    let detection = make_detection("static-html", None);
    let scanned = scan_runtime_artifact(output.path(), &RuntimeArtifactScan::All).unwrap();
    let deployable = prepare_artifact_files(
        &manifest,
        scanned,
        &detection,
        crate::artifact::ArtifactRootScope::BuildOutput,
        true,
    )
    .deployable_entries();
    let paths = deployable
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();

    assert!(paths.contains(&"index.html"));
    assert!(paths.contains(&"node_modules/pkg/index.js"));
    assert_eq!(
        RuntimeArtifactScan::All.file_breakdown(&deployable),
        crate::artifact::RuntimeArtifactFileBreakdown {
            build_output: 2,
            node_modules: 0,
            python_site_packages: 0,
            metadata: 0,
            workspace_packages: 0,
            other: 0,
            total: 2,
        }
    );
    source_bundle_v1::build_source_bundle_plan(output.path(), &manifest, &deployable).unwrap();
}

#[cfg(unix)]
#[test]
fn prepare_deploy_files_preserves_build_only_target_for_deployable_symlink() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    std::os::unix::fs::symlink("../package.json", dir.path().join("assets/pkg")).unwrap();

    let manifest = build_manifest::generate_static_manifest();
    let detection = make_detection("static-html", None);
    let scanned = scan_runtime_artifact(dir.path(), &RuntimeArtifactScan::All).unwrap();
    let deployable = prepare_deploy_files(&manifest, scanned, &detection, true).unwrap();
    let paths = deployable
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();

    assert!(paths.contains(&"assets/pkg"));
    assert!(paths.contains(&"package.json"));
    source_bundle_v1::build_source_bundle_plan(dir.path(), &manifest, &deployable).unwrap();
}

#[test]
fn prepare_deploy_files_keeps_package_json_for_compute_runtime() {
    let manifest = build_manifest::generate_compute_manifest("server.js");
    let detection = make_detection("express", None);
    let files = vec![
        fe("package.json", 10, "aa"),
        fe(".onreza/manifest.json", 20, "bb"),
        fe("server.js", 30, "cc"),
    ];

    let deployable = prepare_deploy_files(&manifest, files, &detection, true).unwrap();
    let paths = deployable
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec!["package.json", "server.js"]);
}

#[test]
fn python_process_runtime_uses_project_root_with_versioned_dependencies() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".onreza/python/3.14/site-packages/orjson")).unwrap();
    fs::write(dir.path().join("main.py"), "import orjson").unwrap();
    fs::write(dir.path().join("requirements.txt"), "orjson==3.11.3\n").unwrap();
    fs::write(
        dir.path()
            .join(".onreza/python/3.14/site-packages/orjson/__init__.py"),
        "loads = lambda value: value",
    )
    .unwrap();
    let detection = crate::detect::detect(dir.path());
    let manifest = build_manifest::generate_compute_manifest("main.py");

    let artifact = resolve_runtime_artifact(
        dir.path(),
        dir.path(),
        dir.path().to_path_buf(),
        manifest,
        &detection,
        true,
    )
    .unwrap();
    let files = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let breakdown = artifact.scan.file_breakdown(&files);

    assert_eq!(artifact.root_dir, dir.path());
    assert_eq!(breakdown.python_site_packages, 1);
    assert!(files.iter().any(|file| file.path == "main.py"));
}

#[test]
fn python_process_runtime_rejects_missing_installed_dependencies() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.py"), "import orjson").unwrap();
    fs::write(dir.path().join("requirements.txt"), "orjson==3.11.3\n").unwrap();
    let detection = crate::detect::detect(dir.path());
    let manifest = build_manifest::generate_compute_manifest("main.py");

    let error = resolve_runtime_artifact(
        dir.path(),
        dir.path(),
        dir.path().to_path_buf(),
        manifest,
        &detection,
        true,
    )
    .unwrap_err();

    assert!(error.to_string().contains("installed dependencies"));
}

#[test]
fn node_process_runtime_artifact_uses_project_root_for_nestjs() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{
            "main": "dist/src/main.js",
            "dependencies": {
                "@nestjs/core": "10.0.0",
                "rxjs": "7.0.0"
            }
        }"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("dist/src")).unwrap();
    fs::write(
        dir.path().join("dist/src/main.js"),
        "require('@nestjs/core')",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("node_modules/@nestjs/core")).unwrap();
    fs::write(
        dir.path().join("node_modules/@nestjs/core/index.js"),
        "module.exports = {}",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.ts"), "source only").unwrap();

    let detection = crate::detect::detect_with_framework_override(dir.path(), None);
    assert_eq!(detection.framework, "nestjs");
    let manifest = build_manifest::generate_compute_manifest("src/main.js");

    let artifact = resolve_runtime_artifact(
        dir.path(),
        dir.path(),
        dir.path().join("dist"),
        manifest,
        &detection,
        true,
    )
    .unwrap();

    assert_eq!(artifact.root_dir, dir.path());
    let compute_layer = artifact
        .manifest
        .layers
        .iter()
        .find(|layer| layer.target == build_manifest::LayerTarget::Compute)
        .unwrap();
    assert_eq!(compute_layer.directory, ".");
    assert_eq!(compute_layer.entry.as_deref(), Some("dist/src/main.js"));

    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let paths = scanned
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"dist/src/main.js"));
    assert!(paths.contains(&"node_modules/@nestjs/core/index.js"));
    assert!(paths.contains(&"package.json"));
    assert!(!paths.contains(&"src/main.ts"));

    let deployable = prepare_deploy_files(&artifact.manifest, scanned, &detection, true).unwrap();
    let plan = source_bundle_v1::build_source_bundle_plan(
        &artifact.root_dir,
        &artifact.manifest,
        &deployable,
    )
    .unwrap();
    assert_eq!(plan.logical_manifest.entrypoints, vec!["dist/src/main.js"]);
    let nest_runtime = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/@nestjs/core/index.js")
        .unwrap();
    assert_eq!(
        nest_runtime.role,
        source_bundle_v1::SourceLogicalManifestFileRole::Compute
    );
}

#[test]
fn process_root_output_keeps_dependency_categories_distinct() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), r#"{"main":"server.js"}"#).unwrap();
    fs::write(dir.path().join("server.js"), "require('runtime-pkg')").unwrap();
    fs::create_dir_all(dir.path().join("node_modules/runtime-pkg")).unwrap();
    fs::write(
        dir.path().join("node_modules/runtime-pkg/index.js"),
        "module.exports = true",
    )
    .unwrap();

    let detection = make_detection("express", None);
    let artifact = resolve_runtime_artifact(
        dir.path(),
        dir.path(),
        dir.path().to_path_buf(),
        build_manifest::generate_compute_manifest("server.js"),
        &detection,
        true,
    )
    .unwrap();
    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();

    assert_eq!(
        artifact.scan.file_breakdown(&scanned),
        crate::artifact::RuntimeArtifactFileBreakdown {
            build_output: 1,
            node_modules: 1,
            python_site_packages: 0,
            metadata: 1,
            workspace_packages: 0,
            other: 0,
            total: 3,
        }
    );
}

#[test]
fn astro_node_runtime_artifact_includes_project_dependencies() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{
            "dependencies": {
                "@astrojs/internal-helpers": "1.0.0"
            }
        }"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("dist/server/chunks")).unwrap();
    fs::write(
        dir.path().join("dist/server/entry.mjs"),
        "import '@astrojs/internal-helpers/path'",
    )
    .unwrap();
    fs::write(
        dir.path().join("dist/server/chunks/errors.mjs"),
        "import '@astrojs/internal-helpers/path'",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("node_modules/@astrojs/internal-helpers")).unwrap();
    fs::write(
        dir.path()
            .join("node_modules/@astrojs/internal-helpers/path.js"),
        "export const joinPaths = () => {}",
    )
    .unwrap();

    let detection = make_detection(
        "astro",
        Some(crate::detect::types::SsrAnalysis {
            is_static_compatible: false,
            ssr_features: vec!["output: 'server' (SSR)".into()],
        }),
    );
    let manifest = build_manifest::generate_astro_ssr_manifest(false);

    let artifact = resolve_runtime_artifact(
        dir.path(),
        dir.path(),
        dir.path().join("dist"),
        manifest,
        &detection,
        true,
    )
    .unwrap();

    assert_eq!(artifact.root_dir, dir.path());
    let compute_layer = artifact
        .manifest
        .layers
        .iter()
        .find(|layer| layer.target == build_manifest::LayerTarget::Compute)
        .unwrap();
    assert_eq!(compute_layer.directory, ".");
    assert_eq!(
        compute_layer.entry.as_deref(),
        Some("dist/server/entry.mjs")
    );

    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let paths = scanned
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"dist/server/entry.mjs"));
    assert!(paths.contains(&"node_modules/@astrojs/internal-helpers/path.js"));
    assert!(paths.contains(&"package.json"));
}

#[test]
fn adapter_node_runtimes_include_project_dependencies() {
    for (framework, entry, manifest) in [
        (
            "sveltekit",
            "index.js",
            build_manifest::generate_sveltekit_manifest(false),
        ),
        (
            "remix",
            "server/index.js",
            build_manifest::generate_remix_manifest(false),
        ),
        (
            "react-router",
            "server/index.js",
            build_manifest::generate_remix_manifest(false),
        ),
    ] {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"runtime-pkg":"1.0.0"}}"#,
        )
        .unwrap();
        fs::create_dir_all(
            dir.path()
                .join("build")
                .join(Path::new(entry).parent().unwrap()),
        )
        .unwrap();
        fs::write(dir.path().join("build").join(entry), "import 'runtime-pkg'").unwrap();
        fs::create_dir_all(dir.path().join("node_modules/runtime-pkg")).unwrap();
        fs::write(
            dir.path().join("node_modules/runtime-pkg/index.js"),
            "export const runtime = true",
        )
        .unwrap();

        let detection = make_detection(framework, None);
        let artifact = resolve_runtime_artifact(
            dir.path(),
            dir.path(),
            dir.path().join("build"),
            manifest,
            &detection,
            true,
        )
        .unwrap();

        assert_eq!(artifact.root_dir, dir.path(), "framework: {framework}");
        let compute_layer = artifact
            .manifest
            .layers
            .iter()
            .find(|layer| layer.target == build_manifest::LayerTarget::Compute)
            .unwrap();
        assert_eq!(compute_layer.directory, ".", "framework: {framework}");
        assert_eq!(
            compute_layer.entry.as_deref(),
            Some(format!("build/{entry}").as_str()),
            "framework: {framework}"
        );

        let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
        assert_eq!(
            artifact.scan.file_breakdown(&scanned),
            crate::artifact::RuntimeArtifactFileBreakdown {
                build_output: 1,
                node_modules: 1,
                python_site_packages: 0,
                metadata: 1,
                workspace_packages: 0,
                other: 0,
                total: 3,
            },
            "framework: {framework}"
        );
        let paths = scanned
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>();
        assert!(paths.contains(&format!("build/{entry}").as_str()));
        assert!(paths.contains(&"node_modules/runtime-pkg/index.js"));
        assert!(paths.contains(&"package.json"));
    }
}

#[tokio::test]
async fn runtime_artifact_scan_failure_preserves_upload_error_code() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("missing-runtime-artifact");

    let error =
        super::plan::scan_runtime_artifact_for_plan(missing.clone(), RuntimeArtifactScan::All)
            .await
            .expect_err("missing runtime artifact must fail before upload");

    expect_code(&error, "UPLOAD_FAILED");
    assert!(
        error
            .to_string()
            .contains("failed to scan runtime artifact"),
        "{error:#}"
    );
    assert!(error.to_string().contains(&missing.display().to_string()));
}

#[cfg(unix)]
#[tokio::test]
async fn runtime_artifact_scan_preserves_invalid_output_error_code() {
    let dir = tempdir().unwrap();
    std::os::unix::fs::symlink("missing-target", dir.path().join("broken-link")).unwrap();

    let error = super::plan::scan_runtime_artifact_for_plan(
        dir.path().to_path_buf(),
        RuntimeArtifactScan::All,
    )
    .await
    .expect_err("broken runtime symlink must be rejected");

    expect_code(&error, "INVALID_BUILD_OUTPUT");
    assert!(
        error
            .to_string()
            .contains("failed to scan runtime artifact"),
        "{error:#}"
    );
}
