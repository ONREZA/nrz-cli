use super::*;
use source_bundle_v1::RuntimeDependencyPackaging;
use source_bundle_v1::RuntimeDependencyPackaging::{Embedded, TrustedMaterialization};

fn install_workspace_test_package(root: &Path, package: &str) {
    let directory = root.join("node_modules").join(package);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("index.js"), "module.exports = {}").unwrap();
}

fn resolve_workspace_runtime(
    root: &Path,
    project: &Path,
    manifest: build_manifest::Manifest,
    detection: &crate::detect::types::DetectionResult,
) -> crate::artifact::RuntimeArtifact {
    resolve_runtime_artifact(
        root,
        project,
        project.join("dist"),
        manifest,
        detection,
        true,
    )
    .unwrap()
}

fn workspace_bundle(
    artifact: &crate::artifact::RuntimeArtifact,
    files: Vec<FileEntry>,
    detection: &crate::detect::types::DetectionResult,
    packaging: RuntimeDependencyPackaging,
) -> source_bundle_v1::SourceBundlePlan {
    let files = crate::artifact::classify_artifact_files(
        &artifact.manifest,
        files,
        detection,
        crate::artifact::ArtifactRootScope::ProjectRoot,
        &artifact.scan,
    )
    .deployable_entries();
    source_bundle_v1::build_source_bundle_plan_with_scan(
        &artifact.root_dir,
        &artifact.manifest,
        &files,
        &artifact.scan,
        packaging,
        None,
    )
    .unwrap()
}

fn scan_node_workspace(
    root: &Path,
    app: &Path,
) -> (
    crate::artifact::RuntimeArtifact,
    crate::detect::types::DetectionResult,
    Vec<FileEntry>,
    crate::artifact::RuntimeArtifactFileBreakdown,
) {
    let detection = crate::detect::detect_with_framework_override(app, None);
    let artifact = resolve_workspace_runtime(
        root,
        app,
        build_manifest::generate_compute_manifest("src/main.js"),
        &detection,
    );
    let files = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let breakdown = artifact.scan.file_breakdown(&files);
    (artifact, detection, files, breakdown)
}

#[cfg(unix)]
fn producer_audit_workspace(
    with_bin: bool,
) -> (
    tempfile::TempDir,
    crate::artifact::RuntimeArtifact,
    crate::detect::types::DetectionResult,
) {
    let workspace = tempdir().unwrap();
    fs::write(
        workspace.path().join("package.json"),
        r#"{"private":true,"workspaces":["apps/*","packages/*"]}"#,
    )
    .unwrap();
    let app = workspace.path().join("apps/api");
    fs::create_dir_all(app.join("dist/src")).unwrap();
    fs::write(
        app.join("package.json"),
        r#"{"dependencies":{"@nestjs/core":"10.0.0","shared":"workspace:*"}}"#,
    )
    .unwrap();
    fs::write(app.join("dist/src/main.js"), b"require('shared')").unwrap();
    let shared = workspace.path().join("packages/shared");
    fs::create_dir_all(&shared).unwrap();
    fs::write(
        shared.join("package.json"),
        r#"{"main":"index.js","bin":"tool.js"}"#,
    )
    .unwrap();
    fs::write(shared.join("index.js"), b"module.exports = 1").unwrap();
    fs::write(shared.join("tool.js"), b"console.log('tool')").unwrap();
    // Install .bin first so the scanner cannot rely on encountering the package alias first.
    fs::create_dir_all(workspace.path().join("node_modules/.bin")).unwrap();
    if with_bin {
        std::os::unix::fs::symlink(
            "../shared/tool.js",
            workspace.path().join("node_modules/.bin/shared"),
        )
        .unwrap();
    }
    std::os::unix::fs::symlink(
        "../packages/shared",
        workspace.path().join("node_modules/shared"),
    )
    .unwrap();
    fs::create_dir_all(workspace.path().join("node_modules/@nestjs/core")).unwrap();
    fs::write(
        workspace.path().join("node_modules/@nestjs/core/index.js"),
        b"module.exports = {}",
    )
    .unwrap();
    let detection = crate::detect::detect_with_framework_override(&app, None);
    let artifact = resolve_workspace_runtime(
        workspace.path(),
        &app,
        build_manifest::generate_compute_manifest("src/main.js"),
        &detection,
    );
    (workspace, artifact, detection)
}

#[cfg(unix)]
#[test]
fn producer_audit_scans_workspace_bin_before_package_alias() {
    let (_workspace, artifact, _) = producer_audit_workspace(true);
    // These selected roots are the same producer scanner, with an explicit traversal order.
    let selected = RuntimeArtifactScan::Selected {
        roots: vec![
            crate::artifact::RuntimeArtifactScanRoot {
                path: "node_modules/.bin".into(),
                kind: crate::artifact::RuntimeArtifactScanRootKind::NodeModules,
            },
            crate::artifact::RuntimeArtifactScanRoot {
                path: "node_modules/shared".into(),
                kind: crate::artifact::RuntimeArtifactScanRootKind::NodeModules,
            },
        ],
        symlink_roots: artifact.scan.symlink_roots().unwrap().to_vec(),
    };
    let files = scan_runtime_artifact(&artifact.root_dir, &selected).unwrap();
    assert!(
        files
            .iter()
            .any(|file| file.path == "packages/shared/tool.js")
    );
    scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
}

#[cfg(unix)]
#[test]
fn producer_audit_projects_dependencies_without_dropping_application_output() {
    let (_workspace, artifact, detection) = producer_audit_workspace(false);
    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let plan = workspace_bundle(&artifact, scanned, &detection, TrustedMaterialization);
    let restored = crate::test_support::unpack_source_bundle(&plan);
    assert_eq!(
        fs::read(restored.path().join("apps/api/dist/src/main.js")).unwrap(),
        b"require('shared')"
    );
    assert_eq!(
        fs::read(restored.path().join("node_modules/shared/index.js")).unwrap(),
        b"module.exports = 1"
    );
    let entry = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "apps/api/dist/src/main.js")
        .unwrap();
    assert_eq!(
        entry.role,
        source_bundle_v1::SourceLogicalManifestFileRole::Compute
    );
    assert_eq!(entry.layer_name.as_deref(), Some("server"));
}

#[test]
fn relocated_compute_entrypoint_keeps_compute_ownership_under_static_output() {
    let project = tempdir().unwrap();
    fs::write(project.path().join("package.json"), r#"{"type":"module"}"#).unwrap();
    fs::create_dir_all(project.path().join("dist/api")).unwrap();
    fs::write(project.path().join("dist/index.html"), "<h1>site</h1>").unwrap();
    fs::write(
        project.path().join("dist/api/main.js"),
        "export default () => 'ok'",
    )
    .unwrap();
    fs::write(
        project.path().join("dist/api/helper.js"),
        "export const secret = 'server-only'",
    )
    .unwrap();
    fs::create_dir_all(project.path().join("dist/api/public")).unwrap();
    fs::write(
        project.path().join("dist/api/public/logo.svg"),
        "<svg></svg>",
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(project.path(), Some("other"));
    let manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            {"name": "static", "target": "STATIC", "directory": "."},
            {"name": "api", "target": "COMPUTE", "directory": "api", "entry": "main.js"},
            {"name": "assets", "target": "STATIC", "directory": "api/public"}
        ],
        "routes": [
            {"pattern": "^/assets/.*$", "layer": "assets"},
            {"pattern": "^/api/.*$", "layer": "api"},
            {"pattern": "^/.*$", "layer": "static"}
        ]
    }))
    .unwrap();
    let artifact = resolve_workspace_runtime(project.path(), project.path(), manifest, &detection);
    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let classified = crate::artifact::classify_artifact_files(
        &artifact.manifest,
        scanned,
        &detection,
        crate::artifact::ArtifactRootScope::ProjectRoot,
        &artifact.scan,
    );
    let entrypoint_file = classified
        .files
        .iter()
        .find(|file| file.path == "dist/api/main.js")
        .unwrap();
    assert_eq!(
        entrypoint_file.role,
        crate::artifact::ArtifactFileRole::Compute
    );
    assert_eq!(entrypoint_file.layer.as_deref(), Some("api"));
    let helper_file = classified
        .files
        .iter()
        .find(|file| file.path == "dist/api/helper.js")
        .unwrap();
    assert_eq!(helper_file.role, crate::artifact::ArtifactFileRole::Compute);
    assert_eq!(helper_file.layer.as_deref(), Some("api"));
    let files = classified.deployable_entries();
    let plan = source_bundle_v1::build_source_bundle_plan_with_scan(
        &artifact.root_dir,
        &artifact.manifest,
        &files,
        &artifact.scan,
        TrustedMaterialization,
        None,
    )
    .unwrap();
    let entrypoint = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "dist/api/main.js")
        .unwrap();
    assert_eq!(
        entrypoint.role,
        source_bundle_v1::SourceLogicalManifestFileRole::Compute
    );
    assert_eq!(entrypoint.layer_name.as_deref(), Some("api"));
    let helper = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "dist/api/helper.js")
        .unwrap();
    assert_eq!(
        helper.role,
        source_bundle_v1::SourceLogicalManifestFileRole::Compute
    );
    assert_eq!(helper.layer_name.as_deref(), Some("api"));
    let logo = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "dist/api/public/logo.svg")
        .unwrap();
    assert_eq!(
        logo.role,
        source_bundle_v1::SourceLogicalManifestFileRole::Static
    );
    assert_eq!(logo.layer_name.as_deref(), Some("assets"));
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
    nrz_runtime_artifact::finalize_source_bundle_runtime_graph(
        &plan.logical_manifest_sha256,
        &plan.source_sha256,
        plan.source_size_bytes,
        &wire,
    )
    .unwrap();
}

#[cfg(unix)]
#[test]
fn node_process_runtime_artifact_prefers_workspace_root_for_hoisted_app_symlink() {
    for runtime_type in [RuntimeType::Node, RuntimeType::Bun] {
        let workspace = tempdir().unwrap();
        let app = workspace.path().join("apps/api");
        fs::create_dir_all(app.join("dist/src")).unwrap();
        fs::write(
            app.join("package.json"),
            r#"{
                "name": "@qualification/api",
                "dependencies": {
                    "@nestjs/core": "10.0.0"
                }
            }"#,
        )
        .unwrap();
        fs::write(
            app.join("dist/src/main.js"),
            "console.log(require('@nestjs/core'))",
        )
        .unwrap();

        fs::create_dir_all(workspace.path().join("node_modules/@nestjs/core")).unwrap();
        fs::write(
            workspace.path().join("node_modules/@nestjs/core/index.js"),
            "module.exports = 'WORKSPACE_ALIAS_OK'",
        )
        .unwrap();
        fs::create_dir_all(app.join("node_modules/@nestjs")).unwrap();
        std::os::unix::fs::symlink(
            "../../../../node_modules/@nestjs/core",
            app.join("node_modules/@nestjs/core"),
        )
        .unwrap();

        fs::write(
            workspace.path().join("package.json"),
            r#"{"private":true,"workspaces":["apps/*"]}"#,
        )
        .unwrap();
        fs::create_dir_all(
            workspace
                .path()
                .join("node_modules/.pnpm/node_modules/@qualification"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            "../../../../apps/api",
            workspace
                .path()
                .join("node_modules/.pnpm/node_modules/@qualification/api"),
        )
        .unwrap();

        let mut detection = crate::detect::detect_with_framework_override(&app, None);
        assert_eq!(detection.framework, "nestjs");
        let manifest = build_manifest::generate_compute_manifest("src/main.js");

        detection.metadata.runtime.runtime_type = runtime_type;
        let artifact = resolve_workspace_runtime(workspace.path(), &app, manifest, &detection);

        assert_eq!(artifact.root_dir, workspace.path());
        let compute_layer = artifact
            .manifest
            .layers
            .iter()
            .find(|layer| layer.target == build_manifest::LayerTarget::Compute)
            .unwrap();
        assert_eq!(compute_layer.directory, ".");
        assert_eq!(
            compute_layer.entry.as_deref(),
            Some("apps/api/dist/src/main.js")
        );

        let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
        let breakdown = artifact.scan.file_breakdown(&scanned);
        assert_eq!(breakdown.total, scanned.len());
        assert!(breakdown.build_output > 0);
        assert!(breakdown.node_modules > 0);
        assert!(breakdown.metadata > 0);
        assert_eq!(breakdown.workspace_packages, 0);
        let structured = serde_json::to_string(&breakdown).unwrap();
        assert!(!structured.contains(&workspace.path().display().to_string()));
        let paths = scanned
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>();
        assert!(paths.contains(&"apps/api/node_modules/@nestjs/core"));
        assert!(paths.contains(&"node_modules/@nestjs/core/index.js"));

        for packaging in [Embedded, TrustedMaterialization] {
            let plan = workspace_bundle(&artifact, scanned.clone(), &detection, packaging);
            let symlink = plan
                .logical_manifest
                .files
                .iter()
                .find(|file| file.path == "apps/api/node_modules/@nestjs/core")
                .unwrap();
            assert_eq!(
                symlink.entry_type,
                Some(source_bundle_v1::SourceLogicalManifestEntryType::Symlink)
            );
            assert_eq!(
                symlink.link_target.as_deref(),
                Some("../../../../node_modules/@nestjs/core")
            );
            let restored = crate::test_support::unpack_source_bundle(&plan);
            let result = assert_cmd::Command::new("node")
                .arg(&plan.logical_manifest.entrypoints[0])
                .current_dir(restored.path())
                .env_remove("NODE_PATH")
                .env_remove("NODE_OPTIONS")
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(
                String::from_utf8_lossy(&result.stdout).trim(),
                "WORKSPACE_ALIAS_OK"
            );
        }
    }
}

#[test]
fn node_process_runtime_artifact_includes_workspace_hoisted_deps_with_app_node_modules() {
    let workspace = tempdir().unwrap();
    let app = workspace.path().join("apps/api");
    fs::create_dir_all(app.join("dist/src")).unwrap();
    fs::write(
        app.join("package.json"),
        r#"{
            "dependencies": {
                "@nestjs/core": "10.0.0",
                "local-only": "1.0.0"
            }
        }"#,
    )
    .unwrap();
    fs::write(
        app.join("dist/src/main.js"),
        "require('@nestjs/core'); require('local-only')",
    )
    .unwrap();

    install_workspace_test_package(workspace.path(), "@nestjs/core");
    install_workspace_test_package(&app, "local-only");

    let (artifact, detection, scanned, breakdown) = scan_node_workspace(workspace.path(), &app);
    assert_eq!(detection.framework, "nestjs");
    assert_eq!(artifact.root_dir, workspace.path());
    assert_eq!(breakdown.total, scanned.len());
    assert_eq!(breakdown.workspace_packages, 0);
    let paths = scanned
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"node_modules/@nestjs/core/index.js"));
    assert!(paths.contains(&"apps/api/node_modules/local-only/index.js"));

    let plan = workspace_bundle(&artifact, scanned, &detection, Embedded);
    assert_eq!(
        plan.logical_manifest.entrypoints,
        vec!["apps/api/dist/src/main.js"]
    );
}

#[cfg(unix)]
#[test]
fn node_process_runtime_artifact_includes_workspace_package_symlink_targets() {
    let workspace = tempdir().unwrap();
    let app = workspace.path().join("apps/api");
    let shared = workspace.path().join("packages/group/shared");
    fs::write(
        workspace.path().join("package.json"),
        r#"{"private":true,"workspaces":["apps/*","packages/**"]}"#,
    )
    .unwrap();
    fs::create_dir_all(app.join("dist/src")).unwrap();
    fs::create_dir_all(&shared).unwrap();
    fs::write(
        app.join("package.json"),
        r#"{
            "dependencies": {
                "@nestjs/core": "10.0.0",
                "@scope/shared": "workspace:*"
            }
        }"#,
    )
    .unwrap();
    fs::write(
        app.join("dist/src/main.js"),
        "require('@nestjs/core'); require('@scope/shared')",
    )
    .unwrap();
    fs::write(shared.join("package.json"), r#"{"main":"index.js"}"#).unwrap();
    fs::write(shared.join("index.js"), "module.exports = {}").unwrap();

    install_workspace_test_package(workspace.path(), "@nestjs/core");
    fs::create_dir_all(workspace.path().join("node_modules/@scope")).unwrap();
    std::os::unix::fs::symlink(
        "../../packages/group/shared",
        workspace.path().join("node_modules/@scope/shared"),
    )
    .unwrap();

    let (artifact, detection, scanned, breakdown) = scan_node_workspace(workspace.path(), &app);
    assert_eq!(detection.framework, "nestjs");
    assert_eq!(artifact.root_dir, workspace.path());
    assert_eq!(breakdown.total, scanned.len());
    assert!(breakdown.workspace_packages > 0);
    let paths = scanned
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"node_modules/@scope/shared"));
    assert!(paths.contains(&"packages/group/shared/index.js"));

    let plan = workspace_bundle(&artifact, scanned, &detection, Embedded);
    let shared_runtime = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/@scope/shared/index.js")
        .unwrap();
    assert_eq!(
        shared_runtime.role,
        source_bundle_v1::SourceLogicalManifestFileRole::Compute
    );
}

#[cfg(unix)]
#[test]
fn node_process_runtime_artifact_rejects_unlisted_symlink_target() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("package.json"),
        r#"{"dependencies":{"express":"1.0.0"}}"#,
    )
    .unwrap();
    fs::create_dir_all(project.path().join("dist")).unwrap();
    fs::write(project.path().join("dist/server.js"), "require('express')").unwrap();
    fs::create_dir_all(project.path().join("node_modules/express")).unwrap();
    fs::write(
        project.path().join("node_modules/express/index.js"),
        "module.exports = {}",
    )
    .unwrap();
    fs::write(project.path().join(".env"), "SECRET=value").unwrap();
    std::os::unix::fs::symlink(
        "../../.env",
        project.path().join("node_modules/express/leak"),
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(project.path(), None);
    let manifest = build_manifest::generate_compute_manifest("server.js");
    let artifact = resolve_workspace_runtime(project.path(), project.path(), manifest, &detection);

    let error = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap_err();
    expect_code(&error, "INVALID_BUILD_OUTPUT");
}

#[cfg(unix)]
#[test]
fn node_process_runtime_artifact_rejects_workspace_package_file_symlink_target() {
    let workspace = tempdir().unwrap();
    let app = workspace.path().join("apps/api");
    fs::write(
        workspace.path().join("package.json"),
        r#"{"private":true,"workspaces":["apps/*"]}"#,
    )
    .unwrap();
    fs::create_dir_all(app.join("dist")).unwrap();
    fs::write(
        app.join("package.json"),
        r#"{"dependencies":{"express":"1.0.0"}}"#,
    )
    .unwrap();
    fs::write(app.join("dist/server.js"), "require('express')").unwrap();
    fs::write(app.join(".env"), "SECRET=value").unwrap();
    install_workspace_test_package(workspace.path(), "express");
    std::os::unix::fs::symlink(
        "../../apps/api/.env",
        workspace.path().join("node_modules/express/leak"),
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(&app, None);
    let manifest = build_manifest::generate_compute_manifest("server.js");
    let artifact = resolve_workspace_runtime(workspace.path(), &app, manifest, &detection);

    let error = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap_err();
    expect_code(&error, "INVALID_BUILD_OUTPUT");
}

#[test]
fn node_process_runtime_artifact_falls_back_when_build_output_outside_project() {
    let project = tempdir().unwrap();
    let external_output = tempdir().unwrap();
    fs::write(
        project.path().join("package.json"),
        r#"{
            "dependencies": {
                "@nestjs/core": "10.0.0"
            }
        }"#,
    )
    .unwrap();
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { NestFactory } from '@nestjs/core';",
    )
    .unwrap();
    fs::create_dir_all(project.path().join("node_modules/@nestjs/core")).unwrap();
    fs::write(
        project.path().join("node_modules/@nestjs/core/index.js"),
        "module.exports = {}",
    )
    .unwrap();
    fs::create_dir_all(external_output.path().join("src")).unwrap();
    fs::write(
        external_output.path().join("src/main.js"),
        "require('@nestjs/core')",
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(project.path(), None);
    assert_eq!(detection.framework, "nestjs");
    let manifest = build_manifest::generate_compute_manifest("src/main.js");

    let artifact = resolve_runtime_artifact(
        project.path(),
        project.path(),
        external_output.path().to_path_buf(),
        manifest,
        &detection,
        true,
    )
    .unwrap();

    // Build output lives outside the project, so relocation can't apply: the
    // deploy degrades to scanning the build output as-is instead of erroring.
    assert_eq!(artifact.root_dir, external_output.path());
    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    assert_eq!(
        scanned
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        vec!["src/main.js"]
    );
    assert_eq!(artifact.scan.file_breakdown(&scanned).build_output, 1);
    let compute_layer = artifact
        .manifest
        .layers
        .iter()
        .find(|layer| layer.target == build_manifest::LayerTarget::Compute)
        .unwrap();
    assert_eq!(compute_layer.directory, ".");
    assert_eq!(compute_layer.entry.as_deref(), Some("src/main.js"));
}
