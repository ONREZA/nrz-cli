use super::*;

#[cfg(unix)]
#[test]
fn node_process_runtime_artifact_prefers_workspace_root_for_hoisted_app_symlink() {
    let workspace = tempdir().unwrap();
    let app = workspace.path().join("apps/api");
    fs::create_dir_all(app.join("dist/src")).unwrap();
    fs::write(
        app.join("package.json"),
        r#"{
            "dependencies": {
                "@nestjs/core": "10.0.0"
            }
        }"#,
    )
    .unwrap();
    fs::write(app.join("dist/src/main.js"), "require('@nestjs/core')").unwrap();

    fs::create_dir_all(workspace.path().join("node_modules/@nestjs/core")).unwrap();
    fs::write(
        workspace.path().join("node_modules/@nestjs/core/index.js"),
        "module.exports = {}",
    )
    .unwrap();
    fs::create_dir_all(app.join("node_modules/@nestjs")).unwrap();
    std::os::unix::fs::symlink(
        "../../../../node_modules/@nestjs/core",
        app.join("node_modules/@nestjs/core"),
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(&app, None);
    assert_eq!(detection.framework, "nestjs");
    let manifest = build_manifest::generate_compute_manifest("src/main.js");

    let artifact = resolve_runtime_artifact(
        workspace.path(),
        &app,
        app.join("dist"),
        manifest,
        &detection,
        true,
    )
    .unwrap();

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

    let deployable = prepare_deploy_files(&artifact.manifest, scanned, &detection, true).unwrap();
    let plan = source_bundle_v1::build_source_bundle_plan(
        &artifact.root_dir,
        &artifact.manifest,
        &deployable,
    )
    .unwrap();
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

    fs::create_dir_all(workspace.path().join("node_modules/@nestjs/core")).unwrap();
    fs::write(
        workspace.path().join("node_modules/@nestjs/core/index.js"),
        "module.exports = {}",
    )
    .unwrap();
    fs::create_dir_all(app.join("node_modules/local-only")).unwrap();
    fs::write(
        app.join("node_modules/local-only/index.js"),
        "module.exports = {}",
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(&app, None);
    assert_eq!(detection.framework, "nestjs");
    let manifest = build_manifest::generate_compute_manifest("src/main.js");

    let artifact = resolve_runtime_artifact(
        workspace.path(),
        &app,
        app.join("dist"),
        manifest,
        &detection,
        true,
    )
    .unwrap();

    assert_eq!(artifact.root_dir, workspace.path());
    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let breakdown = artifact.scan.file_breakdown(&scanned);
    assert_eq!(breakdown.total, scanned.len());
    assert_eq!(breakdown.workspace_packages, 0);
    let paths = scanned
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"node_modules/@nestjs/core/index.js"));
    assert!(paths.contains(&"apps/api/node_modules/local-only/index.js"));

    let deployable = prepare_deploy_files(&artifact.manifest, scanned, &detection, true).unwrap();
    let plan = source_bundle_v1::build_source_bundle_plan(
        &artifact.root_dir,
        &artifact.manifest,
        &deployable,
    )
    .unwrap();
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

    fs::create_dir_all(workspace.path().join("node_modules/@nestjs/core")).unwrap();
    fs::write(
        workspace.path().join("node_modules/@nestjs/core/index.js"),
        "module.exports = {}",
    )
    .unwrap();
    fs::create_dir_all(workspace.path().join("node_modules/@scope")).unwrap();
    std::os::unix::fs::symlink(
        "../../packages/group/shared",
        workspace.path().join("node_modules/@scope/shared"),
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(&app, None);
    assert_eq!(detection.framework, "nestjs");
    let manifest = build_manifest::generate_compute_manifest("src/main.js");

    let artifact = resolve_runtime_artifact(
        workspace.path(),
        &app,
        app.join("dist"),
        manifest,
        &detection,
        true,
    )
    .unwrap();

    assert_eq!(artifact.root_dir, workspace.path());
    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let breakdown = artifact.scan.file_breakdown(&scanned);
    assert_eq!(breakdown.total, scanned.len());
    assert!(breakdown.workspace_packages > 0);
    let paths = scanned
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"node_modules/@scope/shared"));
    assert!(paths.contains(&"packages/group/shared/index.js"));

    let deployable = prepare_deploy_files(&artifact.manifest, scanned, &detection, true).unwrap();
    let plan = source_bundle_v1::build_source_bundle_plan(
        &artifact.root_dir,
        &artifact.manifest,
        &deployable,
    )
    .unwrap();
    let shared_runtime = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "packages/group/shared/index.js")
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
    let artifact = resolve_runtime_artifact(
        project.path(),
        project.path(),
        project.path().join("dist"),
        manifest,
        &detection,
        true,
    )
    .unwrap();

    let error = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("does not resolve to a declared workspace package root"),
        "{error}"
    );
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
    fs::create_dir_all(workspace.path().join("node_modules/express")).unwrap();
    fs::write(
        workspace.path().join("node_modules/express/index.js"),
        "module.exports = {}",
    )
    .unwrap();
    std::os::unix::fs::symlink(
        "../../apps/api/.env",
        workspace.path().join("node_modules/express/leak"),
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(&app, None);
    let manifest = build_manifest::generate_compute_manifest("server.js");
    let artifact = resolve_runtime_artifact(
        workspace.path(),
        &app,
        app.join("dist"),
        manifest,
        &detection,
        true,
    )
    .unwrap();

    let error = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("does not resolve to a declared workspace package root"),
        "{error}"
    );
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
