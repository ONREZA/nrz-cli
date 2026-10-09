use super::*;

fn manifest_with_python_sibling(
    family: &str,
    entry: &str,
    target: &str,
) -> build_manifest::Manifest {
    serde_json::from_value(serde_json::json!({
        "version":1, "routes":[{"pattern":"^/.*$", "layer":"api"}], "layers":[
            {"name":"api","target":"COMPUTE","directory":".","entry":entry,"runtime":{"applicationRuntime":{"family":family,"args":[]},"buildRuntimeVersion":target}},
            {"name":"side","target":"COMPUTE","directory":"side","entry":"main.py","runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":"python-3.12"}}
        ]
    })).unwrap()
}

#[test]
fn bun_process_archive_resolves_dynamic_and_transitive_dependencies_offline() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{
        "packageManager":"bun@1.4.2",
        "dependencies":{"hono":"4.0.0","runtime-pkg":"1.0.0"}
    }"#,
    )
    .unwrap();
    fs::write(dir.path().join("bun.lock"), "{}").unwrap();
    fs::create_dir_all(dir.path().join("dist")).unwrap();
    fs::write(
        dir.path().join("dist/server.mjs"),
        "const { default: value } = await import('runtime-pkg'); console.log(value);",
    )
    .unwrap();
    for (name, body) in [
        (
            "runtime-pkg",
            "import value from 'transitive-pkg'; export default value;",
        ),
        ("transitive-pkg", "export default 'ARTIFACT_RUNTIME_OK';"),
    ] {
        let package = dir.path().join("node_modules").join(name);
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("package.json"),
            serde_json::to_vec(&serde_json::json!({
                "name":name,"version":"1.0.0","type":"module","main":"index.js"
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(package.join("index.js"), body).unwrap();
    }
    assert_bun_runtime_archive_output(dir.path(), "ARTIFACT_RUNTIME_OK");
}

#[test]
#[ignore = "requires an installed sharp fixture with Bun package metadata and dist/server.mjs"]
fn bun_sharp_encodes_from_relocated_source_archive() {
    let fixture =
        PathBuf::from(std::env::var_os("NRZ_NATIVE_FIXTURE_ROOT").expect("native fixture"));
    assert_bun_runtime_archive_output(&fixture, "SHARP_NATIVE_OK 768");
}

fn assert_bun_runtime_archive_output(root: &Path, expected: &str) {
    let detection = crate::detect::detect_with_framework_override(root, Some("hono"));
    assert_eq!(detection.metadata.runtime.runtime_type, RuntimeType::Bun);
    let artifact = resolve_runtime_artifact(
        root,
        root,
        root.join("dist"),
        build_manifest::generate_compute_manifest("server.mjs"),
        &detection,
        true,
    )
    .unwrap();
    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let files = prepare_deploy_files(&artifact.manifest, scanned, &detection, true).unwrap();
    let plan = source_bundle_v1::build_source_bundle_plan_with_scan(
        &artifact.root_dir,
        &artifact.manifest,
        &files,
        &artifact.scan,
        source_bundle_v1::RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();
    let unpacked = tempdir().unwrap();
    let decoder =
        zstd::stream::read::Decoder::new(fs::File::open(plan.source_path()).unwrap()).unwrap();
    tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
    let entry = &plan.logical_manifest.entrypoints[0];
    // An extracted deployment must resolve its own dependencies even with npm
    // auto-install disabled and without access to the source project's files.
    let output = assert_cmd::Command::new("bun")
        .timeout(std::time::Duration::from_secs(10))
        .arg("--no-install")
        .arg(entry)
        .current_dir(unpacked.path())
        .env("BUN_INSTALL_CACHE_DIR", unpacked.path().join("empty-cache"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), expected);
}

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
    let scanned = scan_runtime_artifact(
        dir.path(),
        &RuntimeArtifactScan::PythonRuntimeRoot(nrz_source_bundle::PythonMinor::default()),
    )
    .unwrap();
    let collection = prepare_artifact_files(
        &manifest,
        scanned,
        &detection,
        crate::artifact::ArtifactRootScope::ProjectRoot,
        &RuntimeArtifactScan::PythonRuntimeRoot(nrz_source_bundle::PythonMinor::default()),
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
        &RuntimeArtifactScan::PythonRuntimeRoot(nrz_source_bundle::PythonMinor::default()),
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
fn python_scan_excludes_local_environments_and_keeps_the_frozen_bootstrap() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".venv/bin")).unwrap();
    fs::create_dir_all(dir.path().join("src/__pycache__")).unwrap();
    fs::create_dir_all(dir.path().join("assets")).unwrap();
    fs::create_dir_all(dir.path().join("build/lib/demo")).unwrap();
    fs::create_dir_all(dir.path().join("build/assets")).unwrap();
    fs::create_dir_all(dir.path().join("src/demo.egg-info")).unwrap();
    fs::create_dir_all(dir.path().join(".onreza/python/3.14/site-packages/demo")).unwrap();
    fs::create_dir_all(
        dir.path()
            .join(".onreza/python/3.14/site-packages/demo.egg-info"),
    )
    .unwrap();
    for (name, content) in [
        (".env", "SECRET=local"),
        (".env.production", "SECRET=local"),
        (".venv/bin/python", "local interpreter"),
        ("src/__pycache__/demo.pyc", "cache"),
        ("src/main.py", "print('ready')"),
        ("assets/data.json", "{}"),
        ("build/lib/demo/__init__.py", "backend duplicate"),
        ("build/assets/logo.svg", "<svg/>"),
        ("src/demo.egg-info/PKG-INFO", "backend metadata"),
        (
            "pyproject.toml",
            "[build-system]\nrequires = ['setuptools']\nbuild-backend = 'setuptools.build_meta'\n[project]\nname = 'demo'\nversion = '1.0'\n",
        ),
        (".onreza/python/launch.py", "print('launch')"),
        (".onreza/python/3.14/site-packages/demo/__init__.py", ""),
        (
            ".onreza/python/3.14/site-packages/demo.egg-info/PKG-INFO",
            "installed metadata",
        ),
    ] {
        fs::write(dir.path().join(name), content).unwrap();
    }
    #[cfg(unix)]
    {
        fs::remove_file(dir.path().join(".venv/bin/python")).unwrap();
        std::os::unix::fs::symlink("/usr/bin/python3", dir.path().join(".venv/bin/python"))
            .unwrap();
    }
    let scanned = scan_runtime_artifact(
        dir.path(),
        &RuntimeArtifactScan::PythonRuntimeRoot(nrz_source_bundle::PythonMinor::default()),
    )
    .unwrap();
    let paths = scanned
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            ".onreza/python/3.14/site-packages/demo.egg-info/PKG-INFO",
            ".onreza/python/3.14/site-packages/demo/__init__.py",
            ".onreza/python/launch.py",
            "assets/data.json",
            "build/assets/logo.svg",
            "pyproject.toml",
            "src/main.py"
        ]
    );
    let mut detection = crate::detect::detect_with_framework_override(dir.path(), None);
    detection.metadata.runtime.runtime_type = RuntimeType::Python;
    let manifest = build_manifest::generate_compute_manifest(".onreza/python/launch.py");
    let deployable = prepare_artifact_files(
        &manifest,
        scanned,
        &detection,
        crate::artifact::ArtifactRootScope::ProjectRoot,
        &RuntimeArtifactScan::PythonRuntimeRoot(nrz_source_bundle::PythonMinor::default()),
        true,
    )
    .deployable_entries();
    assert_eq!(deployable.len(), 7);
    assert!(
        deployable
            .iter()
            .any(|file| file.path == ".onreza/python/launch.py")
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/etc/passwd", dir.path().join("assets/escape")).unwrap();
        assert!(
            scan_runtime_artifact(
                dir.path(),
                &RuntimeArtifactScan::PythonRuntimeRoot(nrz_source_bundle::PythonMinor::default())
            )
            .is_err()
        );
    }
}

#[test]
fn python_published_staged_dependencies_keep_assets_named_like_project_build_outputs() {
    let project = tempdir().unwrap();
    let minor = nrz_source_bundle::PythonMinor::default();
    let package = format!("{}/demo", minor.site_packages_root());
    let asset_names = [
        "node_modules",
        ".venv",
        "venv",
        "__pycache__",
        ".pytest_cache",
        ".mypy_cache",
        ".ruff_cache",
        ".tox",
        ".nox",
    ];
    fs::create_dir_all(project.path().join(&package)).unwrap();
    fs::write(
        project.path().join("main.py"),
        "import demo; print(demo.read())\n",
    )
    .unwrap();
    fs::write(project.path().join("requirements.txt"), "").unwrap();
    fs::write(project.path().join(&package).join("__init__.py"),
        "from pathlib import Path\ndef read():\n return ','.join(sorted(p.read_text() for p in Path(__file__).parent.rglob('payload.txt')))\n",
    ).unwrap();
    for name in asset_names {
        for root in [name.to_string(), format!("{package}/{name}")] {
            fs::create_dir_all(project.path().join(&root)).unwrap();
            fs::write(project.path().join(root).join("payload.txt"), name).unwrap();
        }
    }
    // Virtualenv detection must likewise apply to authored project build state,
    // not to an installed package's data directory.
    fs::write(
        project.path().join(&package).join("venv/pyvenv.cfg"),
        "package data",
    )
    .unwrap();
    let mut detection =
        crate::detect::detect_with_framework_override(project.path(), Some("python"));
    detection.metadata.runtime.runtime_type = RuntimeType::Python;
    let manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [{"name":"compute", "target":"COMPUTE", "directory":".", "entry":"main.py",
            "runtime":{"applicationRuntime":{"family":"PYTHON","args":[]}, "buildRuntimeVersion":minor.target()}}],
        "routes": [{"pattern":"^/.*$", "layer":"compute"}]
    })).unwrap();
    let artifact = resolve_runtime_artifact(
        project.path(),
        project.path(),
        project.path().into(),
        manifest,
        &detection,
        true,
    )
    .unwrap();
    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let files = prepare_artifact_files(
        &artifact.manifest,
        scanned,
        &detection,
        crate::artifact::ArtifactRootScope::ProjectRoot,
        &artifact.scan,
        true,
    )
    .deployable_entries();
    let source = source_bundle_v1::build_source_bundle_plan_with_scan(
        &artifact.root_dir,
        &artifact.manifest,
        &files,
        &artifact.scan,
        source_bundle_v1::RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();
    for name in asset_names {
        assert!(
            !source
                .logical_manifest
                .files
                .iter()
                .any(|file| file.path == format!("{name}/payload.txt"))
        );
        let file = source
            .logical_manifest
            .files
            .iter()
            .find(|file| file.path == format!("{package}/{name}/payload.txt"))
            .unwrap();
        assert_eq!(
            file.role,
            source_bundle_v1::SourceLogicalManifestFileRole::Dependency
        );
    }
    let unpacked = crate::test_support::unpack_source_bundle(&source);
    fs::remove_dir_all(project.path()).unwrap();
    let output = std::process::Command::new("python3")
        .args(["-S", "main.py"])
        .env(
            "PYTHONPATH",
            unpacked.path().join(minor.site_packages_root()),
        )
        .current_dir(unpacked.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut expected = asset_names.to_vec();
    expected.sort();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        expected.join(",")
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
        &RuntimeArtifactScan::All,
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

#[cfg(unix)]
fn producer_audit_static_bundle(root: &Path) -> source_bundle_v1::SourceBundlePlan {
    let manifest = build_manifest::generate_static_manifest();
    let scanned = scan_runtime_artifact(root, &RuntimeArtifactScan::All).unwrap();
    let files = prepare_deploy_files(
        &manifest,
        scanned,
        &make_detection("static-html", None),
        true,
    )
    .unwrap();
    source_bundle_v1::build_source_bundle_plan(root, &manifest, &files).unwrap()
}

#[cfg(unix)]
#[test]
fn producer_audit_preserves_intermediate_build_only_alias() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("payload.json"), b"{\"runtime\":true}").unwrap();
    std::os::unix::fs::symlink("payload.json", dir.path().join("package.json")).unwrap();
    std::os::unix::fs::symlink("../package.json", dir.path().join("assets/pkg")).unwrap();
    let plan = producer_audit_static_bundle(dir.path());
    let restored = crate::test_support::unpack_source_bundle(&plan);
    assert_eq!(
        fs::read(restored.path().join("assets/pkg")).unwrap(),
        b"{\"runtime\":true}"
    );
    assert_eq!(
        fs::read_link(restored.path().join("assets/pkg")).unwrap(),
        std::path::Path::new("../package.json")
    );
}

#[cfg(unix)]
#[test]
fn producer_audit_retains_only_a_witness_for_an_intermediate_pruned_directory() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("assets")).unwrap();
    fs::create_dir_all(dir.path().join("node_modules/unused")).unwrap();
    fs::create_dir_all(dir.path().join("node_modules/other")).unwrap();
    fs::create_dir_all(dir.path().join("node_modules/keep-whole")).unwrap();
    for (name, bytes) in [("first", b"first"), ("other", b"other")] {
        fs::write(dir.path().join("node_modules/keep-whole").join(name), bytes).unwrap();
    }
    fs::write(dir.path().join("node_modules/unused/a-witness"), b"witness").unwrap();
    fs::write(
        dir.path().join("node_modules/unused/z-unneeded"),
        b"unneeded",
    )
    .unwrap();
    fs::write(dir.path().join("node_modules/other/unneeded"), b"unneeded").unwrap();
    fs::write(dir.path().join("payload.json"), b"runtime").unwrap();
    std::os::unix::fs::symlink(
        "../node_modules/unused/../../payload.json",
        dir.path().join("assets/pkg"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        "../node_modules/keep-whole",
        dir.path().join("assets/package"),
    )
    .unwrap();
    let plan = producer_audit_static_bundle(dir.path());
    let restored = crate::test_support::unpack_source_bundle(&plan);
    assert_eq!(
        fs::read(restored.path().join("assets/pkg")).unwrap(),
        b"runtime"
    );
    assert_eq!(
        fs::read(restored.path().join("node_modules/unused/a-witness")).unwrap(),
        b"witness"
    );
    assert!(
        !restored
            .path()
            .join("node_modules/unused/z-unneeded")
            .exists()
    );
    assert!(!restored.path().join("node_modules/other").exists());
    assert_eq!(
        fs::read(restored.path().join("assets/package/first")).unwrap(),
        b"first"
    );
    assert_eq!(
        fs::read(restored.path().join("assets/package/other")).unwrap(),
        b"other"
    );
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
    let mut detection = crate::detect::detect(dir.path());
    crate::detect::application_runtime::resolve_and_bind_detection(
        &crate::detect::fs::LocalFs::new(dir.path()),
        &mut detection,
    )
    .unwrap();
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
fn relocated_python_runtime_preserves_compute_file_ownership() {
    let project = tempdir().unwrap();
    fs::create_dir_all(project.path().join("dist/api")).unwrap();
    fs::write(project.path().join("dist/index.html"), "site").unwrap();
    fs::write(
        project.path().join("dist/api/main.py"),
        "from helper import app",
    )
    .unwrap();
    fs::write(project.path().join("dist/api/helper.py"), "app = None").unwrap();
    let mut detection =
        crate::detect::detect_with_framework_override(project.path(), Some("other"));
    detection.metadata.runtime.runtime_type = RuntimeType::Python;
    detection.metadata.runtime.version =
        Some(nrz_source_bundle::PythonMinor::default().version().into());
    let manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            {"name": "static", "target": "STATIC", "directory": "."},
            {"name": "api", "target": "COMPUTE", "directory": "api", "entry": "main.py", "runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":"python-3.14"}}
        ],
        "routes": [
            {"pattern": "^/api/.*$", "layer": "api"},
            {"pattern": "^/.*$", "layer": "static"}
        ]
    }))
    .unwrap();

    let artifact = resolve_runtime_artifact(
        project.path(),
        project.path(),
        project.path().join("dist"),
        manifest,
        &detection,
        true,
    )
    .unwrap();
    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let classified = prepare_artifact_files(
        &artifact.manifest,
        scanned,
        &detection,
        crate::artifact::ArtifactRootScope::ProjectRoot,
        &artifact.scan,
        true,
    );
    let files = classified.deployable_entries();
    let plan = source_bundle_v1::build_source_bundle_plan_with_scan(
        &artifact.root_dir,
        &artifact.manifest,
        &files,
        &artifact.scan,
        source_bundle_v1::RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();
    for path in ["dist/api/main.py", "dist/api/helper.py"] {
        let file = plan
            .logical_manifest
            .files
            .iter()
            .find(|file| file.path == path)
            .unwrap();
        assert_eq!(
            file.role,
            source_bundle_v1::SourceLogicalManifestFileRole::Compute
        );
        assert_eq!(file.layer_name.as_deref(), Some("api"));
    }
    let html = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "dist/index.html")
        .unwrap();
    assert_eq!(
        html.role,
        source_bundle_v1::SourceLogicalManifestFileRole::Static
    );
    let wire =
        serde_json::from_value(serde_json::to_value(&plan.logical_manifest).unwrap()).unwrap();
    let graph = nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_target(
        &plan.logical_manifest_sha256,
        &plan.source_sha256,
        plan.source_size_bytes,
        &wire,
        &[],
        Some(nrz_source_bundle::PythonMinor::default().target()),
    )
    .unwrap();
    assert_eq!(
        graph.wire().runtime_layers[0]
            .launch
            .as_ref()
            .unwrap()
            .profile,
        nrz_runtime_artifact::RuntimeProfile::Cpython314
    );
}

#[test]
fn python_process_runtime_rejects_missing_installed_dependencies() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.py"), "import orjson").unwrap();
    fs::write(dir.path().join("requirements.txt"), "orjson==3.11.3\n").unwrap();
    let mut detection = crate::detect::detect(dir.path());
    crate::detect::application_runtime::resolve_and_bind_detection(
        &crate::detect::fs::LocalFs::new(dir.path()),
        &mut detection,
    )
    .unwrap();
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
    for runtime_type in [RuntimeType::Node, RuntimeType::Bun] {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), r#"{"main":"server.js"}"#).unwrap();
        fs::write(dir.path().join("server.js"), "require('runtime-pkg')").unwrap();
        fs::create_dir_all(dir.path().join("node_modules/runtime-pkg")).unwrap();
        fs::write(
            dir.path().join("node_modules/runtime-pkg/index.js"),
            "module.exports = true",
        )
        .unwrap();

        let mut detection = make_detection("express", None);
        detection.metadata.runtime.runtime_type = runtime_type;
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

#[test]
fn python_runtime_scan_excludes_a_previous_build_for_another_minor() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.py"), "print('ready')").unwrap();
    for minor in nrz_source_bundle::PythonMinor::ALL {
        fs::create_dir_all(dir.path().join(minor.site_packages_root())).unwrap();
        fs::write(
            dir.path().join(minor.site_packages_root()).join("demo.py"),
            "VALUE=1",
        )
        .unwrap();
    }
    for minor in nrz_source_bundle::PythonMinor::ALL {
        let scanned =
            scan_runtime_artifact(dir.path(), &RuntimeArtifactScan::PythonRuntimeRoot(minor))
                .unwrap();
        let paths = scanned
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>();
        assert!(paths.contains(&"main.py"));
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&format!("{}/demo.py", minor.site_packages_root()).as_str()));
    }
}

#[test]
fn mixed_python_minors_keep_dependencies_with_their_declared_owner() {
    use nrz_source_bundle::{
        BuildToolchainDeclaration, BuildToolchainFamily, PythonMinor, SourceBuildContext,
    };
    let project = tempdir().unwrap();
    fs::create_dir_all(project.path().join("side")).unwrap();
    fs::write(project.path().join("main.py"), "import demo").unwrap();
    fs::write(project.path().join("side/main.py"), "print(42)").unwrap();
    let dependencies = project
        .path()
        .join(PythonMinor::Python314.site_packages_root());
    fs::create_dir_all(&dependencies).unwrap();
    fs::write(dependencies.join("demo.py"), "VALUE = 42").unwrap();
    let mut detection = make_detection("other", None);
    detection.metadata.source_build_context = Some(SourceBuildContext {
        schema_version: 1,
        build_toolchain: BuildToolchainDeclaration {
            family: BuildToolchainFamily::Python,
            python_version: Some(PythonMinor::Python314),
        },
        application_runtime: None,
    });
    let manifest = manifest_with_python_sibling("PYTHON", "main.py", "python-3.14");
    let artifact = project_root_runtime_artifact(project.path(), manifest, &detection);
    let source = runtime_source_bundle_with_checked_entrypoints(&artifact);
    let dependency = source
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path.ends_with("site-packages/demo.py"))
        .unwrap();
    assert_dependency_owner(dependency, "api");
    assert_eq!(
        source.logical_manifest.layers[1]
            .runtime_config
            .as_ref()
            .unwrap()["buildRuntimeVersion"],
        "python-3.12"
    );
    let mut wrong_owner = artifact.manifest;
    wrong_owner.layers[0]
        .runtime
        .as_mut()
        .unwrap()
        .application_runtime
        .as_mut()
        .unwrap()
        .family = nrz_source_bundle::ApplicationRuntimeFamily::Node;
    wrong_owner.layers[0]
        .runtime
        .as_mut()
        .unwrap()
        .build_runtime_version = Some("node-24".into());
    let error = resolve_runtime_artifact(
        project.path(),
        project.path(),
        project.path().to_owned(),
        wrong_owner,
        &detection,
        true,
    )
    .unwrap_err();
    expect_code(&error, "APPLICATION_RUNTIME_INVALID");
    assert!(error.to_string().contains("owning layer 'api'"));
}

#[test]
fn javascript_dependency_closure_survives_a_code_only_python_sibling() {
    let project = tempdir().unwrap();
    fs::create_dir_all(project.path().join("side")).unwrap();
    fs::write(project.path().join("server.js"), "require('demo')").unwrap();
    fs::write(project.path().join("side/main.py"), "print(42)").unwrap();
    fs::create_dir_all(project.path().join("node_modules/demo")).unwrap();
    fs::write(
        project.path().join("node_modules/demo/index.js"),
        "module.exports = 42",
    )
    .unwrap();
    let detection = make_detection("other", None);
    let manifest = manifest_with_python_sibling("NODE", "server.js", "node-24");
    let artifact = project_root_runtime_artifact(project.path(), manifest, &detection);
    let source = runtime_source_bundle_with_checked_entrypoints(&artifact);
    let dependency = source
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/demo/index.js")
        .unwrap();
    assert_dependency_owner(dependency, "api");
    assert!(
        source
            .logical_manifest
            .files
            .iter()
            .any(|file| file.path == "side/main.py")
    );
}

#[test]
fn hoisted_javascript_dependencies_survive_a_code_only_python_sibling() {
    use nrz_source_bundle::{
        ApplicationRuntimeDeclaration, ApplicationRuntimeFamily, BuildToolchainDeclaration,
        BuildToolchainFamily, SourceBuildContext,
    };
    for compiler in [BuildToolchainFamily::Node, BuildToolchainFamily::Python] {
        for (family, target, entry, code) in [
            ("NODE", "node-22", "server.js", "console.log(42)"),
            ("PYTHON", "python-3.12", "main.py", "print(42)"),
        ] {
            let workspace = tempdir().unwrap();
            let project = workspace.path().join("apps/site");
            fs::create_dir_all(workspace.path().join("node_modules/demo")).unwrap();
            fs::write(
                workspace.path().join("node_modules/demo/index.js"),
                "module.exports = 42",
            )
            .unwrap();
            fs::create_dir_all(project.join("dist/api")).unwrap();
            fs::create_dir_all(project.join("dist/side")).unwrap();
            fs::write(
                project.join("package.json"),
                r#"{"dependencies":{"demo":"1.0.0"}}"#,
            )
            .unwrap();
            fs::write(project.join("dist/api/server.js"), "require('demo')").unwrap();
            fs::write(project.join("dist/side").join(entry), code).unwrap();
            fs::write(project.join("requirements.txt"), "build-tool==1.0.0\n").unwrap();
            let python_tools =
                project.join(nrz_source_bundle::PythonMinor::Python314.site_packages_root());
            fs::create_dir_all(&python_tools).unwrap();
            fs::write(python_tools.join("build_tool.py"), "VALUE=1").unwrap();

            let mut detection = make_detection("other", None);
            detection.metadata.source_build_context = Some(SourceBuildContext {
                schema_version: 1,
                build_toolchain: BuildToolchainDeclaration {
                    family: compiler,
                    python_version: (compiler == BuildToolchainFamily::Python)
                        .then_some(nrz_source_bundle::PythonMinor::Python314),
                },
                application_runtime: Some(ApplicationRuntimeDeclaration {
                    family: ApplicationRuntimeFamily::Node,
                    python_version: None,
                    entry: Some("dist/api/server.js".into()),
                    args: vec![],
                }),
            });
            let manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
        "version":1, "routes":[{"pattern":"^/.*$", "layer":"api"}], "layers":[
            {"name":"api","target":"COMPUTE","directory":"api","entry":"server.js","runtime":{"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-24"}},
            {"name":"side","target":"COMPUTE","directory":"side","entry":entry,"runtime":{"applicationRuntime":{"family":family,"args":[]},"buildRuntimeVersion":target}}
        ]
    })).unwrap();

            let artifact = resolve_runtime_artifact(
                workspace.path(),
                &project,
                project.join("dist"),
                manifest,
                &detection,
                true,
            )
            .unwrap();
            assert_eq!(artifact.root_dir, workspace.path());
            let mut overlapping: build_manifest::Manifest =
                serde_json::from_value(serde_json::to_value(&artifact.manifest).unwrap()).unwrap();
            overlapping.layers[0].directory = ".".into();
            overlapping.layers[0].entry = Some("api/server.js".into());
            overlapping.layers[1].directory = ".".into();
            overlapping.layers[1].entry = Some(format!("side/{entry}"));
            let error = resolve_runtime_artifact(
                workspace.path(),
                &project,
                project.join("dist"),
                overlapping,
                &detection,
                true,
            )
            .unwrap_err();
            expect_code(&error, "APPLICATION_RUNTIME_INVALID");

            let source = runtime_source_bundle_with_checked_entrypoints(&artifact);
            let dependency = source
                .logical_manifest
                .files
                .iter()
                .find(|file| file.path == "node_modules/demo/index.js")
                .unwrap();
            assert_dependency_owner(dependency, "api");
            let python = source
                .logical_manifest
                .files
                .iter()
                .find(|file| file.path == format!("apps/site/dist/side/{entry}"))
                .unwrap();
            assert_eq!(python.layer_name.as_deref(), Some("side"));
            assert_eq!(
                python.role,
                source_bundle_v1::SourceLogicalManifestFileRole::Compute
            );
            assert!(
                source
                    .logical_manifest
                    .files
                    .iter()
                    .filter(|file| file.role
                        == source_bundle_v1::SourceLogicalManifestFileRole::Dependency)
                    .all(|file| file.path.contains("node_modules/"))
            );
            let python_layer = source
                .logical_manifest
                .layers
                .iter()
                .find(|layer| layer.name == "side")
                .unwrap();
            assert_eq!(
                python_layer.entrypoint.as_deref(),
                Some(format!("apps/site/dist/side/{entry}").as_str())
            );
            let launch = nrz_runtime_artifact::source_layer_launch_for_target(
                python_layer.runtime_config.as_ref(),
                Some(target),
            )
            .unwrap();
            assert_eq!(
                launch.profile,
                if family == "PYTHON" {
                    nrz_runtime_artifact::RuntimeProfile::Cpython312
                } else {
                    nrz_runtime_artifact::RuntimeProfile::Node22
                }
            );
        }
    }
}
