use std::fs;
use std::io::Cursor;
#[cfg(unix)]
use std::path::PathBuf;

use tempfile::tempdir;

use crate::deploy::hash::sha256_hex;
use crate::deploy::scan_dir;

use super::source_bundle_v1::*;
use super::*;

fn static_manifest() -> crate::build::manifest::Manifest {
    serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            { "name": "static", "target": "STATIC", "directory": "." }
        ],
        "routes": []
    }))
    .unwrap()
}

fn compute_manifest() -> crate::build::manifest::Manifest {
    serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            { "name": "server", "target": "COMPUTE", "directory": ".", "entry": "server.js" }
        ],
        "routes": []
    }))
    .unwrap()
}

#[test]
fn source_bundle_retains_prebuild_launcher_args_and_version_witness() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("server.js"), "console.log('ok')").unwrap();
    let mut manifest = compute_manifest();
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        family: nrz_source_bundle::ApplicationRuntimeFamily::Bun,
        entry: Some("server.js".into()),
        args: vec!["--port".into(), "8080".into()],
    };
    crate::deploy::apply_application_runtime_manifest(
        &mut manifest,
        Some(&declaration),
        Some("bun-1.4.2"),
        "other",
    )
    .unwrap();
    let plan =
        build_source_bundle_plan(dir.path(), &manifest, &scan_dir(dir.path()).unwrap()).unwrap();
    let config = plan.logical_manifest.layers[0]
        .runtime_config
        .as_ref()
        .unwrap();
    assert_eq!(
        config["applicationRuntime"],
        serde_json::json!({"family":"BUN","args":["--port","8080"]})
    );
    assert_eq!(config["buildRuntimeVersion"], "bun-1.4.2");
    let mut contradictory = declaration;
    contradictory.family = nrz_source_bundle::ApplicationRuntimeFamily::Node;
    assert!(
        crate::deploy::apply_application_runtime_manifest(
            &mut manifest,
            Some(&contradictory),
            Some("node-24"),
            "other",
        )
        .is_err()
    );
}

#[tokio::test]
async fn source_bundle_plan_is_deterministic_and_uses_identity_file_hashes() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("b.txt"), b"b").unwrap();
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let first = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();
    let second = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    assert_eq!(first.source_sha256, second.source_sha256);
    assert_eq!(
        first.logical_manifest_sha256,
        second.logical_manifest_sha256
    );
    assert_eq!(first.logical_manifest.files[0].path, "a.txt");
    assert_eq!(first.logical_manifest.files[1].path, "b.txt");
    assert_eq!(first.logical_manifest.files[0].sha256, sha256_hex(b"a"));
    assert_eq!(
        first.logical_manifest.files[0].role,
        SourceLogicalManifestFileRole::Static
    );
    let compressed = tokio::fs::read(first.source_path()).await.unwrap();
    assert_eq!(sha256_hex(&compressed), first.source_sha256);
    assert_eq!(compressed.len() as u64, first.source_size_bytes);
}

#[test]
fn source_bundle_assigns_dependency_ownership_only_for_trusted_materialization() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("node_modules/pkg")).unwrap();
    fs::write(dir.path().join("server.js"), b"require('pkg')").unwrap();
    fs::write(
        dir.path().join("node_modules/pkg/index.js"),
        b"module.exports = 1",
    )
    .unwrap();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan_with_scan(
        dir.path(),
        &compute_manifest(),
        &files,
        &RuntimeArtifactScan::NodeRuntimeRoot,
        RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();
    let embedded = build_source_bundle_plan_with_scan(
        dir.path(),
        &compute_manifest(),
        &files,
        &RuntimeArtifactScan::NodeRuntimeRoot,
        RuntimeDependencyPackaging::Embedded,
        None,
    )
    .unwrap();

    let server = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "server.js")
        .unwrap();
    let dependency = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/pkg/index.js")
        .unwrap();
    assert_eq!(server.role, SourceLogicalManifestFileRole::Compute);
    assert_eq!(dependency.role, SourceLogicalManifestFileRole::Dependency);
    assert_eq!(dependency.layer_name.as_deref(), Some("server"));
    let embedded_dependency = embedded
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/pkg/index.js")
        .unwrap();
    assert_eq!(
        embedded_dependency.role,
        SourceLogicalManifestFileRole::Compute
    );
    assert_eq!(embedded_dependency.layer_name.as_deref(), Some("server"));
}

#[tokio::test]
async fn python_bundle_separates_site_packages_and_declares_runtime_family() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".onreza/python/3.14/site-packages/orjson")).unwrap();
    fs::write(dir.path().join("main.py"), b"import orjson").unwrap();
    fs::write(
        dir.path()
            .join(".onreza/python/3.14/site-packages/orjson/__init__.py"),
        b"loads = lambda value: value",
    )
    .unwrap();
    let mut manifest: crate::build::manifest::Manifest =
        serde_json::from_value(serde_json::json!({
            "version": 1,
            "layers": [
                { "name": "server", "target": "COMPUTE", "directory": ".", "entry": "main.py" }
            ],
            "routes": []
        }))
        .unwrap();
    let detection = crate::detect::detect_with_framework_override(dir.path(), None);
    let effective = nrz::config::EffectiveProjectConfig::from_project_config(
        dir.path().to_owned(),
        nrz::config::ProjectConfig::default(),
    );
    let target = crate::deploy::validate_application_runtime_before_build(
        detection.metadata.application_runtime.as_ref(),
        &detection.metadata.runtime,
        &effective,
        false,
        &[],
    )
    .await
    .unwrap();
    crate::deploy::apply_application_runtime_manifest(
        &mut manifest,
        detection.metadata.application_runtime.as_ref(),
        target.as_deref(),
        &detection.framework,
    )
    .unwrap();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan_with_scan(
        dir.path(),
        &manifest,
        &files,
        &RuntimeArtifactScan::PythonRuntimeRoot,
        RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();

    let source = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "main.py")
        .unwrap();
    let dependency = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path.ends_with("orjson/__init__.py"))
        .unwrap();
    assert_eq!(source.role, SourceLogicalManifestFileRole::Compute);
    assert_eq!(dependency.role, SourceLogicalManifestFileRole::Dependency);
    assert_eq!(
        plan.logical_manifest.layers[0].runtime_config,
        Some(serde_json::json!({"runtimeFamily": "PYTHON", "buildRuntimeVersion": "python-3.14"}))
    );
}

#[tokio::test]
async fn python_bundle_without_dependencies_freezes_provided_build_manifest_target() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.py"), b"print('hello')").unwrap();
    fs::create_dir(dir.path().join(".onreza")).unwrap();
    fs::write(
        dir.path().join(".onreza/manifest.json"),
        serde_json::to_vec(&crate::build::manifest::generate_compute_manifest(
            "main.py",
        ))
        .unwrap(),
    )
    .unwrap();
    let detection = crate::detect::detect_with_framework_override(dir.path(), None);
    let effective = nrz::config::EffectiveProjectConfig::from_project_config(
        dir.path().to_owned(),
        nrz::config::ProjectConfig::default(),
    );
    let build = crate::build::run_with_effective_config(
        crate::cli::BuildArgs {
            dir: dir.path().to_string_lossy().into_owned(),
            skip_validation: false,
        },
        true,
        &effective,
        Some(&detection),
        false,
        dir.path(),
    )
    .await
    .unwrap();
    let manifest = build.manifest.unwrap();
    let plan = build_source_bundle_plan_with_scan(
        dir.path(),
        &manifest,
        &scan_dir(dir.path()).unwrap(),
        &RuntimeArtifactScan::PythonRuntimeRoot,
        RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();
    let config = plan.logical_manifest.layers[0]
        .runtime_config
        .as_ref()
        .unwrap();
    assert_eq!(config["runtimeFamily"], "PYTHON");
    assert_eq!(config["buildRuntimeVersion"], "python-3.14");
    assert_eq!(detection.metadata.runtime.version.as_deref(), Some("3.14"));
    assert!(
        plan.logical_manifest
            .files
            .iter()
            .all(|file| file.role != SourceLogicalManifestFileRole::Dependency)
    );
    let source: nrz_source_bundle::SourceLogicalManifest =
        serde_json::from_value(serde_json::to_value(&plan.logical_manifest).unwrap()).unwrap();
    let graph = nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_layer_targets(
        &plan.logical_manifest_sha256,
        &plan.source_sha256,
        plan.source_size_bytes,
        &source,
        &[],
        &std::collections::HashMap::from([("server".into(), "python-3.14".into())]),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(graph.wire()).unwrap()["runtimeLayers"][0]["launch"]["profile"],
        "CPYTHON_3_14"
    );
}

#[tokio::test]
async fn authored_node_manifest_preserves_standalone_declaration_and_uses_admitted_selection() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("index.html"), b"<html></html>").unwrap();
    fs::write(directory.path().join("server.js"), b"console.log('hello')").unwrap();
    fs::create_dir(directory.path().join(".onreza")).unwrap();
    let detection = crate::detect::detect_with_framework_override(directory.path(), None);
    assert!(detection.metadata.application_runtime.is_none());
    assert_eq!(
        detection.metadata.runtime.runtime_type,
        crate::detect::types::RuntimeType::Static
    );
    for admitted in [false, true] {
        for witness in [None, Some("node-24"), Some("node-22")] {
            let mut manifest = crate::build::manifest::generate_compute_manifest("server.js");
            manifest.layers[0].runtime = Some(crate::build::manifest::RuntimeConfig {
                application_runtime: Some(nrz_source_bundle::ApplicationRuntimeIntent {
                    family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
                    args: vec![],
                }),
                build_runtime_version: witness.map(str::to_owned),
                ..Default::default()
            });
            fs::write(
                directory.path().join(".onreza/manifest.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            let mut config = nrz::config::ProjectConfig::default();
            config.build.output_dirs = Some(vec![".".into()]);
            let mut effective = nrz::config::EffectiveProjectConfig::from_project_config(
                directory.path().to_owned(),
                config,
            );
            if admitted {
                effective.bind_admitted_node_version("NODE_24").unwrap();
            }
            let result = crate::build::run_with_effective_config(
                crate::cli::BuildArgs {
                    dir: directory.path().to_string_lossy().into_owned(),
                    skip_validation: false,
                },
                true,
                &effective,
                Some(&detection),
                false,
                directory.path(),
            )
            .await;
            if admitted && witness == Some("node-22") {
                assert!(result.is_err());
                continue;
            }
            let built = result.unwrap().manifest.unwrap();
            let plan = build_source_bundle_plan_with_scan(
                directory.path(),
                &built,
                &scan_dir(directory.path()).unwrap(),
                &RuntimeArtifactScan::NodeRuntimeRoot,
                RuntimeDependencyPackaging::TrustedMaterialization,
                None,
            )
            .unwrap();
            let config = plan.logical_manifest.layers[0]
                .runtime_config
                .as_ref()
                .unwrap();
            assert_eq!(config["applicationRuntime"]["family"], "NODE");
            if admitted {
                assert_eq!(config["buildRuntimeVersion"], "node-24");
            } else {
                assert_eq!(
                    config
                        .get("buildRuntimeVersion")
                        .and_then(serde_json::Value::as_str),
                    witness
                );
            }
        }
    }
    for family in [None, Some(nrz_source_bundle::ApplicationRuntimeFamily::Bun)] {
        let mut manifest = crate::build::manifest::generate_compute_manifest("server.js");
        manifest.layers[0].runtime = family.map(|family| crate::build::manifest::RuntimeConfig {
            application_runtime: Some(nrz_source_bundle::ApplicationRuntimeIntent {
                family,
                args: vec![],
            }),
            ..Default::default()
        });
        fs::write(
            directory.path().join(".onreza/manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let mut config = nrz::config::ProjectConfig::default();
        config.build.output_dirs = Some(vec![".".into()]);
        let effective = nrz::config::EffectiveProjectConfig::from_project_config(
            directory.path().to_owned(),
            config,
        );
        let built = crate::build::run_with_effective_config(
            crate::cli::BuildArgs {
                dir: directory.path().to_string_lossy().into_owned(),
                skip_validation: false,
            },
            true,
            &effective,
            Some(&detection),
            false,
            directory.path(),
        )
        .await
        .unwrap()
        .manifest
        .unwrap();
        let source = build_source_bundle_plan_with_scan(
            directory.path(),
            &built,
            &scan_dir(directory.path()).unwrap(),
            &RuntimeArtifactScan::NodeRuntimeRoot,
            RuntimeDependencyPackaging::TrustedMaterialization,
            None,
        )
        .unwrap();
        assert_eq!(
            source.logical_manifest.layers[0]
                .runtime_config
                .as_ref()
                .unwrap()["buildRuntimeVersion"],
            "bun-1.4.2"
        );
    }
}

#[tokio::test]
async fn managed_build_declarations_preserve_frozen_targets_and_static_publication() {
    const CASE: &str = "ONREZA_PYTHON_PRODUCER_SELECTION_TEST";
    let Ok(selected) = std::env::var(CASE) else {
        for selected in [
            "python-3.14",
            "python-3.13",
            "bun-1.4.2",
            "node-24",
            "MISSING",
        ] {
            let mut child = tokio::process::Command::new(std::env::current_exe().unwrap());
            child.args(["--exact", "artifact::source_bundle_v1_tests::managed_build_declarations_preserve_frozen_targets_and_static_publication"]);
            child.env(CASE, selected);
            child.env("PATH", "/nonexistent");
            if selected == "MISSING" {
                child.env_remove("ONREZA_RUNTIME_VERSION");
            } else {
                child.env("ONREZA_RUNTIME_VERSION", selected);
            }
            let result = child.output().await.unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stdout)
            );
        }
        return;
    };
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("main.py"), b"print('hello')").unwrap();
    let detection = crate::detect::detect_with_framework_override(directory.path(), None);
    let mut config = nrz::config::ProjectConfig::default();
    config.build.output_dirs = Some(vec![".".into()]);
    let mut effective = nrz::config::EffectiveProjectConfig::from_project_config(
        directory.path().to_owned(),
        config,
    );
    effective.apply_platform_runner_settings(&nrz::config::ProjectBuildSettings {
        application_runtime: Some(None),
        ..Default::default()
    });
    let result = crate::deploy::validate_application_runtime_before_build(
        None,
        &detection.metadata.runtime,
        &effective,
        true,
        &[],
    )
    .await;
    if selected == "python-3.14" {
        assert_eq!(result.unwrap().as_deref(), Some("python-3.14"));
    } else {
        assert!(result.is_err());
    }
    // Implicit JavaScript keeps the Builder selection without inventing a
    // launcher declaration; static-only publication does not need a runtime.
    fs::write(directory.path().join("index.html"), b"<html></html>").unwrap();
    fs::remove_file(directory.path().join("main.py")).unwrap();
    fs::write(directory.path().join("server.js"), b"console.log('hello')").unwrap();
    let javascript = crate::detect::detect_with_framework_override(directory.path(), None);
    assert_eq!(
        javascript.metadata.runtime.runtime_type,
        crate::detect::types::RuntimeType::Static
    );
    fs::create_dir(directory.path().join(".onreza")).unwrap();
    let manifest_path = directory.path().join(".onreza/manifest.json");
    for (managed, family) in [
        (true, None),
        (
            true,
            Some(nrz_source_bundle::ApplicationRuntimeFamily::Node),
        ),
        (true, Some(nrz_source_bundle::ApplicationRuntimeFamily::Bun)),
        (false, None),
    ] {
        let mut manifest = if managed {
            crate::build::manifest::generate_compute_manifest("server.js")
        } else {
            crate::build::manifest::generate_static_manifest()
        };
        if managed {
            manifest.layers[0].runtime =
                family.map(|family| crate::build::manifest::RuntimeConfig {
                    application_runtime: Some(nrz_source_bundle::ApplicationRuntimeIntent {
                        family,
                        args: vec![],
                    }),
                    ..Default::default()
                });
        }
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let result = crate::build::run_with_effective_config(
            crate::cli::BuildArgs {
                dir: directory.path().to_string_lossy().into_owned(),
                skip_validation: false,
            },
            true,
            &effective,
            Some(&javascript),
            false,
            directory.path(),
        )
        .await;
        let permitted = match family {
            Some(nrz_source_bundle::ApplicationRuntimeFamily::Node) => selected == "node-24",
            Some(nrz_source_bundle::ApplicationRuntimeFamily::Bun) => selected == "bun-1.4.2",
            None => matches!(selected.as_str(), "bun-1.4.2" | "node-24"),
        };
        if managed && !permitted {
            assert!(result.is_err());
            continue;
        }
        let manifest = result.unwrap().manifest.unwrap();
        if managed {
            let runtime = manifest.layers[0].runtime.as_ref().unwrap();
            assert_eq!(
                runtime.build_runtime_version.as_deref(),
                Some(selected.as_str())
            );
            assert_eq!(
                runtime
                    .application_runtime
                    .as_ref()
                    .map(|intent| intent.family),
                family
            );
            let source = build_source_bundle_plan_with_scan(
                directory.path(),
                &manifest,
                &scan_dir(directory.path()).unwrap(),
                &RuntimeArtifactScan::NodeRuntimeRoot,
                RuntimeDependencyPackaging::TrustedMaterialization,
                None,
            )
            .unwrap();
            assert_eq!(
                source.logical_manifest.layers[0]
                    .runtime_config
                    .as_ref()
                    .unwrap()["buildRuntimeVersion"],
                selected
            );
        } else {
            assert!(manifest.layers[0].runtime.is_none());
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_projects_workspace_packages_into_dependency_root() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("dist")).unwrap();
    fs::write(dir.path().join("dist/server.js"), b"require('pkg')").unwrap();
    fs::create_dir_all(dir.path().join("packages/pkg")).unwrap();
    fs::write(
        dir.path().join("packages/pkg/package.json"),
        br#"{"name":"pkg"}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("packages/pkg/bin.js"),
        b"console.log('pkg')",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("node_modules/.bin")).unwrap();
    std::os::unix::fs::symlink("../packages/pkg", dir.path().join("node_modules/pkg")).unwrap();
    std::os::unix::fs::symlink("../pkg/bin.js", dir.path().join("node_modules/.bin/pkg")).unwrap();
    let files = scan_dir(dir.path()).unwrap();
    let scan = RuntimeArtifactScan::Selected {
        roots: vec![
            RuntimeArtifactScanRoot {
                path: "dist".into(),
                kind: RuntimeArtifactScanRootKind::BuildOutput,
            },
            RuntimeArtifactScanRoot {
                path: "node_modules".into(),
                kind: RuntimeArtifactScanRootKind::NodeModules,
            },
        ],
        symlink_roots: vec!["packages/pkg".into()],
    };

    let plan = build_source_bundle_plan_with_scan(
        dir.path(),
        &compute_manifest(),
        &files,
        &scan,
        RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();

    assert!(
        plan.logical_manifest
            .files
            .iter()
            .all(|file| !file.path.starts_with("packages/pkg"))
    );
    let package = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/pkg/package.json")
        .unwrap();
    assert_eq!(package.role, SourceLogicalManifestFileRole::Dependency);
    let bin = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/.bin/pkg")
        .unwrap();
    assert_eq!(bin.link_target.as_deref(), Some("../pkg/bin.js"));

    let compressed = tokio::fs::read(plan.source_path()).await.unwrap();
    let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed)).unwrap();
    let extracted = tempdir().unwrap();
    tar::Archive::new(Cursor::new(tar_bytes))
        .unpack(extracted.path())
        .unwrap();
    assert_eq!(
        fs::read_to_string(extracted.path().join("node_modules/.bin/pkg")).unwrap(),
        "console.log('pkg')"
    );
}

#[tokio::test]
async fn source_bundle_embeds_canonical_logical_manifest_first() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("index.html"), b"hello").unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();
    let compressed = tokio::fs::read(plan.source_path()).await.unwrap();
    let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed)).unwrap();
    let mut archive = tar::Archive::new(Cursor::new(tar_bytes));
    let mut entries = archive.entries().unwrap();
    let mut manifest_entry = entries.next().unwrap().unwrap();
    let mut manifest_body = String::new();
    std::io::Read::read_to_string(&mut manifest_entry, &mut manifest_body).unwrap();

    assert_eq!(
        manifest_entry.path().unwrap().to_string_lossy(),
        ".__onreza/logical-manifest.json"
    );
    assert_eq!(
        sha256_hex(manifest_body.as_bytes()),
        plan.logical_manifest_sha256
    );
}

#[test]
fn source_bundle_treats_header_and_redirect_files_as_static_content() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("_headers"), b"plain user file").unwrap();
    fs::write(dir.path().join("_redirects"), b"plain user file").unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let headers = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "_headers")
        .unwrap();
    let redirects = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "_redirects")
        .unwrap();
    assert_eq!(headers.role, SourceLogicalManifestFileRole::Static);
    assert_eq!(redirects.role, SourceLogicalManifestFileRole::Static);
}

#[test]
fn source_bundle_marks_prerender_files_under_layer_root() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("_prerender")).unwrap();
    fs::write(dir.path().join("_prerender/index.html"), b"<main/>").unwrap();
    fs::create_dir_all(dir.path().join("server")).unwrap();
    fs::write(dir.path().join("server/server.js"), b"// server").unwrap();

    let manifest: crate::build::manifest::Manifest = serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            { "name": "prerendered", "target": "STATIC", "directory": "_prerender" },
            { "name": "server", "target": "COMPUTE", "directory": "server", "entry": "server.js" }
        ],
        "routes": [
            {
                "pattern": "^/.*$",
                "layer": "prerendered",
                "priority": 75,
                "fallthrough": true,
                "fallthroughWhen": [
                    { "type": "header", "name": "rsc", "value": "1" },
                    { "type": "query", "name": "_rsc" }
                ]
            },
            { "pattern": "^/.*$", "layer": "server", "priority": 0 }
        ],
        "prerender": {
            "layer": "prerendered",
            "pages": { "/": { "html": "index.html" } }
        }
    }))
    .unwrap();

    let files = scan_dir(dir.path()).unwrap();
    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let prerender = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "_prerender/index.html")
        .unwrap();
    assert_eq!(prerender.role, SourceLogicalManifestFileRole::Prerender);
    assert_eq!(prerender.layer_name.as_deref(), Some("prerendered"));

    assert_eq!(
        plan.logical_manifest.routes[0].fallthrough_when.as_ref(),
        manifest.routes[0].fallthrough_when.as_ref()
    );
}

#[test]
fn nuxt_public_asset_maps_to_static_layer_root() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("public")).unwrap();
    fs::write(dir.path().join("public/favicon.svg"), b"<svg/>").unwrap();
    fs::create_dir_all(dir.path().join("server")).unwrap();
    fs::write(dir.path().join("server/index.mjs"), b"// server").unwrap();

    let manifest = crate::build::manifest::generate_nuxt_manifest(true);
    crate::build::manifest::validate(&manifest).unwrap();
    crate::build::manifest::verify_files(dir.path(), &manifest).unwrap();

    let files = scan_dir(dir.path()).unwrap();
    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let static_layer = plan
        .logical_manifest
        .layers
        .iter()
        .find(|layer| layer.name == "static-assets")
        .unwrap();
    assert_eq!(
        static_layer.target,
        SourceLogicalManifestLayerTarget::Static
    );
    assert_eq!(static_layer.root_path.as_deref(), Some("public"));

    let favicon = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "public/favicon.svg")
        .unwrap();
    assert_eq!(favicon.role, SourceLogicalManifestFileRole::Static);
    assert_eq!(favicon.layer_name.as_deref(), Some("static-assets"));

    let static_catch_all = plan
        .logical_manifest
        .routes
        .iter()
        .find(|route| route.pattern == "^/.*$" && route.layer_name == "static-assets")
        .unwrap();
    assert_eq!(static_catch_all.priority, Some(50));

    let server_catch_all = plan
        .logical_manifest
        .routes
        .iter()
        .find(|route| route.pattern == "^/.*$" && route.layer_name == "server")
        .unwrap();
    assert_eq!(server_catch_all.priority, Some(0));
}

#[test]
fn source_bundle_plan_rejects_reserved_metadata_namespace() {
    let manifest = static_manifest();

    let root_file = tempdir().unwrap();
    fs::write(root_file.path().join(".__onreza"), b"user").unwrap();
    let files = scan_dir(root_file.path()).unwrap();
    let err = build_source_bundle_plan(root_file.path(), &manifest, &files).unwrap_err();
    assert!(
        err.to_string().contains("reserves metadata namespace"),
        "{err}"
    );

    let manifest_collision = tempdir().unwrap();
    fs::create_dir(manifest_collision.path().join(".__onreza")).unwrap();
    fs::write(
        manifest_collision
            .path()
            .join(".__onreza/logical-manifest.json"),
        b"user",
    )
    .unwrap();
    let files = scan_dir(manifest_collision.path()).unwrap();
    let err = build_source_bundle_plan(manifest_collision.path(), &manifest, &files).unwrap_err();
    assert!(
        err.to_string().contains("reserves metadata namespace"),
        "{err}"
    );
}

#[test]
fn logical_manifest_sha_uses_stable_key_ordering() {
    let left = SourceLogicalManifest {
        schema_version: SOURCE_BUNDLE_SCHEMA_VERSION.to_string(),
        capabilities: vec![],
        files: vec![SourceLogicalManifestFile {
            path: "index.html".into(),
            sha256: "a".repeat(64),
            size: 5,
            entry_type: None,
            link_target: None,
            content_type: Some("text/html; charset=utf-8".into()),
            role: SourceLogicalManifestFileRole::Static,
            layer_name: Some("static".into()),
            executable: false,
        }],
        layers: vec![SourceLogicalManifestLayer {
            name: "static".into(),
            target: SourceLogicalManifestLayerTarget::Static,
            root_path: None,
            entrypoint: None,
            runtime_config: None,
        }],
        routes: vec![],
        entrypoints: vec![],
    };
    let mut right_value = serde_json::json!({
        "routes": [],
        "layers": [{ "target": "STATIC", "name": "static" }],
        "entrypoints": [],
        "files": [{
            "role": "static",
            "size": 5,
            "sha256": "a".repeat(64),
            "path": "index.html",
            "contentType": "text/html; charset=utf-8",
            "layerName": "static"
        }],
        "capabilities": [],
        "schemaVersion": "SOURCE_BUNDLE_V1.0"
    });
    let right: SourceLogicalManifest = serde_json::from_value(right_value.take()).unwrap();

    assert_eq!(
        compute_logical_manifest_sha256(&left).unwrap(),
        compute_logical_manifest_sha256(&right).unwrap()
    );
}

#[test]
fn canonical_logical_manifest_json_matches_source_bundle_v1_golden() {
    let manifest = SourceLogicalManifest {
        schema_version: SOURCE_BUNDLE_SCHEMA_VERSION.to_string(),
        capabilities: vec![],
        files: vec![
            SourceLogicalManifestFile {
                path: "api/handler.js".into(),
                sha256: "b".repeat(64),
                size: 128,
                entry_type: None,
                link_target: None,
                content_type: Some("application/javascript; charset=utf-8".into()),
                role: SourceLogicalManifestFileRole::Compute,
                layer_name: Some("api".into()),
                executable: true,
            },
            SourceLogicalManifestFile {
                path: "index.html".into(),
                sha256: "a".repeat(64),
                size: 5,
                entry_type: None,
                link_target: None,
                content_type: Some("text/html; charset=utf-8".into()),
                role: SourceLogicalManifestFileRole::Static,
                layer_name: Some("static".into()),
                executable: false,
            },
            SourceLogicalManifestFile {
                path: "link.html".into(),
                sha256: sha256_hex(b"index.html"),
                size: 0,
                entry_type: Some(SourceLogicalManifestEntryType::Symlink),
                link_target: Some("index.html".into()),
                content_type: None,
                role: SourceLogicalManifestFileRole::Static,
                layer_name: Some("static".into()),
                executable: false,
            },
        ],
        layers: vec![
            SourceLogicalManifestLayer {
                name: "api".into(),
                target: SourceLogicalManifestLayerTarget::Compute,
                root_path: Some("api".into()),
                entrypoint: Some("api/handler.js".into()),
                runtime_config: Some(serde_json::json!({ "timeoutMs": 10000, "memoryMb": 256 })),
            },
            SourceLogicalManifestLayer {
                name: "static".into(),
                target: SourceLogicalManifestLayerTarget::Static,
                root_path: None,
                entrypoint: None,
                runtime_config: None,
            },
        ],
        routes: vec![SourceLogicalManifestRoute {
            pattern: "/api/*".into(),
            layer_name: "api".into(),
            priority: Some(10),
            methods: Some(vec!["GET".into(), "POST".into()]),
            fallthrough_when: None,
        }],
        entrypoints: vec!["api/handler.js".into()],
    };

    let canonical = canonical_logical_manifest_json(&manifest).unwrap();
    assert_eq!(
        canonical,
        r#"{"capabilities":[],"entrypoints":["api/handler.js"],"files":[{"contentType":"application/javascript; charset=utf-8","executable":true,"layerName":"api","path":"api/handler.js","role":"compute","sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","size":128},{"contentType":"text/html; charset=utf-8","layerName":"static","path":"index.html","role":"static","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","size":5},{"entryType":"symlink","layerName":"static","linkTarget":"index.html","path":"link.html","role":"static","sha256":"0eb547304658805aad788d320f10bf1f292797b5e6d745a3bf617584da017051","size":0}],"layers":[{"entrypoint":"api/handler.js","name":"api","rootPath":"api","runtimeConfig":{"memoryMb":256,"timeoutMs":10000},"target":"COMPUTE"},{"name":"static","target":"STATIC"}],"routes":[{"layerName":"api","methods":["GET","POST"],"pattern":"/api/*","priority":10}],"schemaVersion":"SOURCE_BUNDLE_V1.0"}"#
    );
    assert_eq!(
        compute_logical_manifest_sha256(&manifest).unwrap(),
        "0ad27552cb64fab088d6e4c4b44c2cd83df43dbd2f5ee76add074e6832aad62a"
    );
    assert_eq!(
        compute_logical_manifest_sha256(&manifest).unwrap(),
        sha256_hex(canonical.as_bytes())
    );
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_plan_serializes_hardlinked_files_as_regular_files() {
    let dir = tempdir().unwrap();
    let platform_bin = dir
        .path()
        .join("node_modules/@esbuild/linux-x64/bin/esbuild");
    fs::create_dir_all(platform_bin.parent().unwrap()).unwrap();
    fs::write(&platform_bin, b"esbuild").unwrap();
    let package_bin = dir.path().join("node_modules/esbuild/bin/esbuild");
    fs::create_dir_all(package_bin.parent().unwrap()).unwrap();
    fs::hard_link(&platform_bin, &package_bin).unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let package_manifest_entry = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/esbuild/bin/esbuild")
        .unwrap();
    assert_eq!(package_manifest_entry.entry_type, None);
    assert_eq!(package_manifest_entry.size, b"esbuild".len() as u64);

    let compressed = tokio::fs::read(plan.source_path()).await.unwrap();
    let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed)).unwrap();
    let mut archive = tar::Archive::new(Cursor::new(tar_bytes.as_slice()));
    let mut hardlinked_paths = Vec::new();
    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().into_owned();
        if path == "node_modules/@esbuild/linux-x64/bin/esbuild"
            || path == "node_modules/esbuild/bin/esbuild"
        {
            assert!(entry.header().entry_type().is_file());
            hardlinked_paths.push(path);
        }
    }
    hardlinked_paths.sort();
    assert_eq!(
        hardlinked_paths,
        [
            "node_modules/@esbuild/linux-x64/bin/esbuild",
            "node_modules/esbuild/bin/esbuild"
        ]
    );

    let extracted = tempdir().unwrap();
    tar::Archive::new(Cursor::new(tar_bytes.as_slice()))
        .unpack(extracted.path())
        .unwrap();
    assert_eq!(
        fs::read(extracted.path().join("node_modules/esbuild/bin/esbuild")).unwrap(),
        b"esbuild"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_plan_preserves_safe_relative_symlinks() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("node_modules/.pnpm/pkg")).unwrap();
    fs::write(
        dir.path().join("node_modules/.pnpm/pkg/index.js"),
        b"module.exports = 1",
    )
    .unwrap();
    std::os::unix::fs::symlink(".pnpm/pkg", dir.path().join("node_modules/pkg")).unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let symlink = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/pkg")
        .unwrap();
    assert_eq!(
        symlink.entry_type,
        Some(SourceLogicalManifestEntryType::Symlink)
    );
    assert_eq!(symlink.link_target.as_deref(), Some(".pnpm/pkg"));
    assert_eq!(symlink.size, 0);
    assert_eq!(symlink.sha256, sha256_hex(b".pnpm/pkg"));

    let compressed = tokio::fs::read(plan.source_path()).await.unwrap();
    let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed)).unwrap();
    let mut archive = tar::Archive::new(Cursor::new(tar_bytes));
    let mut saw_symlink = false;
    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        if entry.path().unwrap().to_string_lossy() != "node_modules/pkg" {
            continue;
        }
        assert!(entry.header().entry_type().is_symlink());
        assert_eq!(
            entry.link_name().unwrap().unwrap(),
            PathBuf::from(".pnpm/pkg")
        );
        saw_symlink = true;
    }
    assert!(saw_symlink);
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_plan_accepts_symlink_chain_through_archive_prefix() {
    let dir = tempdir().unwrap();
    let package_dir = dir
        .path()
        .join("node_modules/.pnpm/foo@1.0.0/node_modules/foo");
    fs::create_dir_all(package_dir.join("bin")).unwrap();
    fs::write(package_dir.join("bin/foo.js"), b"console.log('foo')").unwrap();
    fs::create_dir_all(dir.path().join("node_modules/.bin")).unwrap();
    std::os::unix::fs::symlink(
        ".pnpm/foo@1.0.0/node_modules/foo",
        dir.path().join("node_modules/foo"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        "../foo/bin/foo.js",
        dir.path().join("node_modules/.bin/foo"),
    )
    .unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let compressed = tokio::fs::read(plan.source_path()).await.unwrap();
    let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed)).unwrap();
    let extracted = tempdir().unwrap();
    tar::Archive::new(Cursor::new(tar_bytes))
        .unpack(extracted.path())
        .unwrap();
    assert_eq!(
        fs::read_to_string(extracted.path().join("node_modules/.bin/foo")).unwrap(),
        "console.log('foo')"
    );
}

#[cfg(unix)]
#[test]
fn source_bundle_plan_rejects_symlink_to_empty_directory() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("empty")).unwrap();
    std::os::unix::fs::symlink("empty", dir.path().join("empty-link")).unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let err = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap_err();

    assert!(
        err.to_string()
            .contains("target is not included in archive"),
        "{err}"
    );
}

#[cfg(unix)]
#[test]
fn source_bundle_plan_rejects_recursive_directory_symlink() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("nested")).unwrap();
    fs::write(dir.path().join("nested/file.txt"), b"file").unwrap();
    std::os::unix::fs::symlink(".", dir.path().join("nested/loop")).unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let error = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap_err();

    assert!(error.to_string().contains("recursive symlink"), "{error}");
}

#[cfg(unix)]
#[test]
fn source_bundle_archive_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    fs::write(dir.path().join("secret.txt"), b"secret").unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();
    let mode = fs::metadata(plan.source_path())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;

    assert_eq!(mode, 0o600);
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_plan_accepts_symlink_to_directory_with_symlink_only_descendants() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("dir")).unwrap();
    fs::write(dir.path().join("real.txt"), b"real").unwrap();
    std::os::unix::fs::symlink("../real.txt", dir.path().join("dir/link")).unwrap();
    std::os::unix::fs::symlink("dir", dir.path().join("alias")).unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let compressed = tokio::fs::read(plan.source_path()).await.unwrap();
    let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed)).unwrap();
    let extracted = tempdir().unwrap();
    tar::Archive::new(Cursor::new(tar_bytes))
        .unpack(extracted.path())
        .unwrap();
    assert_eq!(
        fs::read_to_string(extracted.path().join("alias/link")).unwrap(),
        "real"
    );
}

#[cfg(unix)]
#[test]
fn source_bundle_plan_rejects_symlink_to_filtered_file() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("cache")).unwrap();
    fs::write(dir.path().join("cache/data.txt"), b"cache").unwrap();
    std::os::unix::fs::symlink("cache/data.txt", dir.path().join("cache-link")).unwrap();
    let manifest = static_manifest();
    let mut files = scan_dir(dir.path()).unwrap();
    files.retain(|file| file.path != "cache/data.txt");

    let err = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap_err();

    assert!(
        err.to_string()
            .contains("target is not included in archive"),
        "{err}"
    );
}

#[cfg(unix)]
#[test]
fn source_bundle_plan_rejects_overlong_symlink_target() {
    let dir = tempdir().unwrap();
    let target = "a".repeat(SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS + 1);
    std::os::unix::fs::symlink(&target, dir.path().join("long-link")).unwrap();
    let manifest = static_manifest();
    let files = vec![FileEntry {
        path: "long-link".into(),
        size: 0,
        content_hash: sha256_hex(target.as_bytes()),
        kind: crate::artifact::ArtifactFileKind::Symlink,
        symlink_resolved_path: None,
    }];

    let err = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap_err();

    assert!(err.to_string().contains("target too long"), "{err}");
}

#[test]
fn source_bundle_plan_rejects_legacy_manifest_middleware() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("middleware")).unwrap();
    let middleware_body = b"export default function middleware() {}";
    fs::write(dir.path().join("middleware/auth.mjs"), middleware_body).unwrap();
    let manifest: crate::build::manifest::Manifest = serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            { "name": "static", "target": "STATIC", "directory": "." }
        ],
        "routes": [],
        "middleware": [{
            "name": "auth",
            "bundlePath": "middleware/auth.mjs",
            "codeHash": "sha256-abc",
            "matchers": ["^/.*$"]
        }]
    }))
    .unwrap();
    let files = scan_dir(dir.path()).unwrap();

    let err = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap_err();

    assert!(
        err.to_string().contains(
            "manifest middleware is no longer supported; declare HTTP function wiring in onreza.rules.toml with a pipeline action"
        ),
        "{err}"
    );
}

#[test]
fn compute_framework_checks_selected_and_authored_runtime_without_fabricating_intent() {
    for (authored_node, target, witness) in [
        (false, Some("node-22"), None),
        (true, None, None),
        (false, None, Some("node-22")),
    ] {
        let mut manifest = compute_manifest();
        if authored_node {
            manifest.layers[0].runtime = Some(crate::build::manifest::RuntimeConfig {
                application_runtime: Some(nrz_source_bundle::ApplicationRuntimeIntent {
                    family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
                    args: vec![],
                }),
                ..Default::default()
            });
        }
        if let Some(witness) = witness {
            manifest.layers[0]
                .runtime
                .get_or_insert_with(Default::default)
                .build_runtime_version = Some(witness.into());
        }
        let error = crate::deploy::apply_application_runtime_manifest(
            &mut manifest,
            None,
            target,
            "elysia",
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("conflicts with framework elysia")
        );
        assert_eq!(
            error
                .downcast_ref::<crate::output::CodedError>()
                .unwrap()
                .code
                .as_str(),
            "APPLICATION_RUNTIME_INVALID"
        );
    }
    let mut manifest = compute_manifest();
    crate::deploy::apply_application_runtime_manifest(
        &mut manifest,
        None,
        Some("bun-1.4.2"),
        "elysia",
    )
    .unwrap();
    assert!(
        manifest.layers[0]
            .runtime
            .as_ref()
            .unwrap()
            .application_runtime
            .is_none()
    );
    let mut contradictory: crate::build::manifest::Manifest = serde_json::from_value(serde_json::json!({
        "version":1,"layers":[{"name":"server","target":"COMPUTE","directory":".","entry":"server.js",
            "runtime":{"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"bun-1.4.2"}}],"routes":[]
    })).unwrap();
    let error =
        crate::deploy::apply_application_runtime_manifest(&mut contradictory, None, None, "other")
            .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("conflicts with admitted runtime target")
    );
    let mut manifest = static_manifest();
    crate::deploy::apply_application_runtime_manifest(
        &mut manifest,
        None,
        Some("node-22"),
        "elysia",
    )
    .unwrap();
    assert!(manifest.layers[0].runtime.is_none());
}
