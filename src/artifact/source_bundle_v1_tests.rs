use std::fs;
use std::io::Cursor;
#[cfg(unix)]
use std::path::PathBuf;

use tempfile::tempdir;

use crate::deploy::hash::sha256_hex;
use crate::deploy::scan_dir;

use super::*;
use crate::artifact::*;

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

fn provided_manifest_config(
    directory: &std::path::Path,
    manifest: &impl serde::Serialize,
) -> nrz::config::EffectiveProjectConfig {
    fs::write(
        directory.join(".onreza/manifest.json"),
        serde_json::to_vec(manifest).unwrap(),
    )
    .unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    config.build.output_dirs = Some(vec![".".into()]);
    nrz::config::EffectiveProjectConfig::from_project_config(directory.to_owned(), config)
}

// File-backed ELF fixtures exercise the SDK closure verifier without a host compiler.
fn native_elf_fixture(runpath: Option<&str>) -> Vec<u8> {
    let mut bytes = vec![0u8; 1024];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[16..18].copy_from_slice(&(if runpath.is_some() { 2u16 } else { 3u16 }).to_le_bytes());
    bytes[18..20].copy_from_slice(&62u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    if runpath.is_some() {
        bytes[24..32].copy_from_slice(&0x400180u64.to_le_bytes());
    }
    bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
    bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&(if runpath.is_some() { 3u16 } else { 1u16 }).to_le_bytes());
    bytes[64..68].copy_from_slice(&1u32.to_le_bytes()); // PT_LOAD
    bytes[68..72].copy_from_slice(&5u32.to_le_bytes());
    bytes[80..88].copy_from_slice(&0x400000u64.to_le_bytes());
    bytes[96..104].copy_from_slice(&1024u64.to_le_bytes());
    bytes[104..112].copy_from_slice(&1024u64.to_le_bytes());
    if let Some(runpath) = runpath {
        bytes[120..124].copy_from_slice(&2u32.to_le_bytes()); // PT_DYNAMIC
        bytes[128..136].copy_from_slice(&256u64.to_le_bytes());
        bytes[136..144].copy_from_slice(&0x400100u64.to_le_bytes());
        bytes[152..160].copy_from_slice(&80u64.to_le_bytes());
        bytes[160..168].copy_from_slice(&80u64.to_le_bytes());
        for (index, (tag, value)) in [(5u64, 0x400200u64), (10, 128), (1, 1), (29, 32)]
            .iter()
            .enumerate()
        {
            let offset = 256 + index * 16;
            bytes[offset..offset + 8].copy_from_slice(&tag.to_le_bytes());
            bytes[offset + 8..offset + 16].copy_from_slice(&value.to_le_bytes());
        }
        bytes[513..522].copy_from_slice(b"libfoo.so");
        bytes[544..544 + runpath.len()].copy_from_slice(runpath.as_bytes());
        let interpreter = b"/lib64/ld-linux-x86-64.so.2\0";
        bytes[176..180].copy_from_slice(&3u32.to_le_bytes()); // PT_INTERP
        bytes[184..192].copy_from_slice(&640u64.to_le_bytes());
        bytes[192..200].copy_from_slice(&0x400280u64.to_le_bytes());
        bytes[208..216].copy_from_slice(&(interpreter.len() as u64).to_le_bytes());
        bytes[216..224].copy_from_slice(&(interpreter.len() as u64).to_le_bytes());
        bytes[640..640 + interpreter.len()].copy_from_slice(interpreter);
    }
    bytes
}

#[test]
fn native_source_bundle_requires_archived_libraries_owned_by_its_compute_layer() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("lib")).unwrap();
    fs::write(dir.path().join("lib/libfoo.so"), native_elf_fixture(None)).unwrap();
    fs::write(
        dir.path().join("server"),
        native_elf_fixture(Some("$ORIGIN/lib")),
    )
    .unwrap();
    let mut manifest: crate::build::manifest::Manifest = serde_json::from_value(serde_json::json!({"version":1,"layers":[{"name":"native","target":"COMPUTE","directory":".","entry":"server","runtime":{"applicationRuntime":{"family":"EXECUTABLE","args":[]},"buildRuntimeVersion":"native-linux-x86_64-glibc"}}],"routes":[]})).unwrap();
    let files = scan_dir(dir.path()).unwrap();
    build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();
    let excluded = files
        .iter()
        .filter(|file| file.path != "lib/libfoo.so")
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        build_source_bundle_plan(dir.path(), &manifest, &excluded).is_err(),
        "filesystem-present but unarchived library was accepted"
    );
    manifest.layers.insert(
        0,
        serde_json::from_value(
            serde_json::json!({"name":"foreign","target":"STATIC","directory":"lib"}),
        )
        .unwrap(),
    );
    assert!(
        build_source_bundle_plan(dir.path(), &manifest, &files).is_err(),
        "foreign layer library ownership was accepted"
    );

    // A verified closure may live beside the launch cwd within the artifact.
    manifest.layers.remove(0);
    manifest.layers[0].directory = "bin".into();
    fs::create_dir(dir.path().join("bin")).unwrap();
    fs::write(
        dir.path().join("bin/server"),
        native_elf_fixture(Some("$ORIGIN/../lib")),
    )
    .unwrap();
    let files = scan_dir(dir.path()).unwrap();
    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();
    let library = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "lib/libfoo.so")
        .unwrap();
    assert_eq!(library.layer_name.as_deref(), Some("native"));
    assert_eq!(library.role, SourceLogicalManifestFileRole::Compute);
    fs::create_dir(dir.path().join("public")).unwrap();
    fs::write(dir.path().join("public/index.html"), "PUBLIC_ASSET").unwrap();
    let files = scan_dir(dir.path()).unwrap();
    let mut public: crate::build::manifest::Manifest =
        serde_json::from_value(serde_json::to_value(&manifest).unwrap()).unwrap();
    public.layers.insert(
        0,
        serde_json::from_value(serde_json::json!({
            "name":"public", "target":"STATIC", "directory":"public"
        }))
        .unwrap(),
    );
    let public_plan = build_source_bundle_plan(dir.path(), &public, &files).unwrap();
    for (path, role, owner) in [
        (
            "lib/libfoo.so",
            SourceLogicalManifestFileRole::Compute,
            "native",
        ),
        (
            "public/index.html",
            SourceLogicalManifestFileRole::Static,
            "public",
        ),
    ] {
        let file = public_plan
            .logical_manifest
            .files
            .iter()
            .find(|file| file.path == path)
            .unwrap();
        assert_eq!(file.role, role);
        assert_eq!(file.layer_name.as_deref(), Some(owner));
    }
    let excluded = files
        .iter()
        .filter(|file| file.path != "lib/libfoo.so")
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        build_source_bundle_plan(dir.path(), &manifest, &excluded).is_err(),
        "unarchived sibling library was accepted"
    );
    for target in ["STATIC", "COMPUTE"] {
        let mut foreign: crate::build::manifest::Manifest =
            serde_json::from_value(serde_json::to_value(&manifest).unwrap()).unwrap();
        foreign.layers.insert(
            0,
            serde_json::from_value(serde_json::json!({
                "name":"foreign", "target":target, "directory":"lib", "entry":"main.js"
            }))
            .unwrap(),
        );
        assert!(
            build_source_bundle_plan(dir.path(), &foreign, &files).is_err(),
            "foreign {target} sibling ownership was stolen"
        );
    }
    fs::create_dir(dir.path().join("worker")).unwrap();
    fs::copy(
        dir.path().join("bin/server"),
        dir.path().join("worker/server"),
    )
    .unwrap();
    let mut sibling = manifest.layers[0].clone();
    sibling.name = "worker".into();
    sibling.directory = "worker".into();
    manifest.layers.push(sibling);
    let files = scan_dir(dir.path()).unwrap();
    for reverse in [false, true] {
        if reverse {
            manifest.layers.reverse();
        }
        assert!(
            build_source_bundle_plan(dir.path(), &manifest, &files).is_err(),
            "ambiguous native sibling closure was accepted"
        );
    }
}

#[test]
fn source_bundle_retains_prebuild_launcher_args_and_version_witness() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("server.js"), "console.log('ok')").unwrap();
    let mut manifest = compute_manifest();
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        python_version: None,
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
    let mut detection = crate::detect::detect_with_framework_override(dir.path(), None);
    crate::detect::application_runtime::resolve_and_bind_detection(
        &crate::detect::fs::LocalFs::new(dir.path()),
        &mut detection,
    )
    .unwrap();
    let effective = nrz::config::EffectiveProjectConfig::from_project_config(
        dir.path().to_owned(),
        nrz::config::ProjectConfig::default(),
    );
    let target = crate::deploy::validate_application_runtime_before_build(
        detection.metadata.source_build_context.as_ref().unwrap(),
        &effective,
        false,
        false,
        &[],
    )
    .await
    .unwrap();
    crate::deploy::apply_application_runtime_manifest(
        &mut manifest,
        detection.metadata.application_runtime(),
        target.as_deref(),
        &detection.framework,
    )
    .unwrap();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan_with_scan(
        dir.path(),
        &manifest,
        &files,
        &RuntimeArtifactScan::PythonRuntimeRoot(nrz_source_bundle::PythonMinor::default()),
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
        Some(
            serde_json::json!({"runtimeFamily": "PYTHON", "buildRuntimeVersion": "python-3.14", "applicationRuntime":{"family":"PYTHON","args":[]}})
        )
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
        None,
    )
    .await
    .unwrap();
    let manifest = build.manifest.unwrap();
    let plan = build_source_bundle_plan_with_scan(
        dir.path(),
        &manifest,
        &scan_dir(dir.path()).unwrap(),
        &RuntimeArtifactScan::PythonRuntimeRoot(nrz_source_bundle::PythonMinor::default()),
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

#[cfg(unix)]
#[tokio::test]
async fn mixed_python_node_and_native_bundle_preserves_per_layer_runtime_authority() {
    use nrz_source_bundle::{
        ApplicationRuntimeDeclaration, ApplicationRuntimeFamily as Family, PythonMinor,
    };
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.py"), b"print('python')").unwrap();
    fs::write(dir.path().join("server.js"), b"console.log('node')").unwrap();
    fs::copy("/bin/true", dir.path().join("native-server")).unwrap();
    for primary_python in [true, false] {
        let mut manifest: crate::build::manifest::Manifest = serde_json::from_value(serde_json::json!({
            "version":1,"routes":[],"layers":[
                {"name":"python","target":"COMPUTE","directory":".","entry":"main.py",
                 "runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":"python-3.12"}},
                {"name":"node","target":"COMPUTE","directory":".","entry":"server.js",
                 "runtime":{"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-24"}},
                {"name":"native","target":"COMPUTE","directory":".","entry":"native-server",
                 "runtime":{"applicationRuntime":{"family":"EXECUTABLE","args":[]},"buildRuntimeVersion":"native-linux-x86_64-glibc"}}
            ]
        })).unwrap();
        let (family, entry, target, scan) = if primary_python {
            (
                Family::Python,
                "main.py",
                "python-3.12",
                RuntimeArtifactScan::PythonRuntimeRoot(PythonMinor::Python312),
            )
        } else {
            (
                Family::Node,
                "server.js",
                "node-24",
                RuntimeArtifactScan::NodeRuntimeRoot,
            )
        };
        let declaration = ApplicationRuntimeDeclaration {
            family,
            entry: Some(entry.into()),
            args: vec![],
            python_version: primary_python.then_some(PythonMinor::Python312),
        };
        crate::deploy::apply_application_runtime_manifest(
            &mut manifest,
            Some(&declaration),
            Some(target),
            "other",
        )
        .unwrap();
        let plan = build_source_bundle_plan_with_scan(
            dir.path(),
            &manifest,
            &scan_dir(dir.path()).unwrap(),
            &scan,
            RuntimeDependencyPackaging::TrustedMaterialization,
            None,
        )
        .unwrap();
        let logical: nrz_source_bundle::SourceLogicalManifest =
            serde_json::from_value(serde_json::to_value(&plan.logical_manifest).unwrap()).unwrap();
        let verified = crate::test_support::verify_source_bundle(&plan, &logical).await;
        let logical: nrz_source_bundle::SourceLogicalManifest =
            serde_json::from_value(verified.logical_manifest).unwrap();
        let targets = std::collections::HashMap::from([
            ("python".into(), "python-3.12".into()),
            ("node".into(), "node-24".into()),
            ("native".into(), "native-linux-x86_64-glibc".into()),
        ]);
        let graph = nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_layer_targets(
            &plan.logical_manifest_sha256,
            &plan.source_sha256,
            plan.source_size_bytes,
            &logical,
            &[],
            &targets,
        )
        .unwrap();
        nrz_runtime_artifact::verify_source_runtime_graph_dependencies(&logical, &graph).unwrap();
        for (name, profile) in [
            ("python", "CPYTHON_3_12"),
            ("node", "NODE_24"),
            ("native", "EXECUTABLE"),
        ] {
            let layer = graph
                .wire()
                .runtime_layers
                .iter()
                .find(|layer| layer.layer_name.as_str() == name)
                .unwrap();
            assert_eq!(layer.launch.as_ref().unwrap().profile.to_string(), profile);
        }
    }
}

#[tokio::test]
async fn admitted_node_build_preserves_code_only_sibling_target() {
    let directory = tempdir().unwrap();
    fs::create_dir_all(directory.path().join(".onreza")).unwrap();
    fs::create_dir_all(directory.path().join("secondary")).unwrap();
    fs::write(directory.path().join("server.js"), "console.log('primary')").unwrap();
    fs::write(
        directory.path().join("secondary/server.js"),
        "console.log('secondary')",
    )
    .unwrap();
    fs::write(
        directory.path().join("package.json"),
        r#"{"scripts":{"start":"node server.js"}}"#,
    )
    .unwrap();
    let manifest = serde_json::json!({
        "version":1,
        "routes":[{"pattern":"^/.*$","layer":"primary","priority":0},
                  {"pattern":"^/secondary/.*$","layer":"secondary","priority":1}],
        "layers":[
            {"name":"primary","target":"COMPUTE","directory":".","entry":"server.js",
             "runtime":{"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-24"}},
            {"name":"secondary","target":"COMPUTE","directory":"secondary","entry":"server.js",
             "runtime":{"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-22"}}
        ]
    });
    let mut effective = provided_manifest_config(directory.path(), &manifest);
    effective.bind_admitted_node_version("NODE_24").unwrap();
    let built = crate::build::run_with_effective_config(
        crate::cli::BuildArgs {
            dir: directory.path().to_string_lossy().into_owned(),
            skip_validation: false,
        },
        true,
        &effective,
        None,
        false,
        directory.path(),
        None,
    )
    .await
    .unwrap()
    .manifest
    .unwrap();
    let plan = build_source_bundle_plan_with_scan(
        directory.path(),
        &built,
        &scan_dir(directory.path()).unwrap(),
        &RuntimeArtifactScan::NodeRuntimeRoot,
        RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();
    let secondary = plan
        .logical_manifest
        .layers
        .iter()
        .find(|layer| layer.name == "secondary")
        .unwrap();
    assert_eq!(
        secondary.runtime_config.as_ref().unwrap()["buildRuntimeVersion"],
        "node-22"
    );
    assert!(
        plan.logical_manifest
            .files
            .iter()
            .all(|file| file.role != SourceLogicalManifestFileRole::Dependency)
    );
    let primary = plan
        .logical_manifest
        .layers
        .iter()
        .find(|layer| layer.name == "primary")
        .unwrap();
    assert_eq!(
        primary.runtime_config.as_ref().unwrap()["buildRuntimeVersion"],
        "node-24"
    );
    let logical: nrz_source_bundle::SourceLogicalManifest =
        serde_json::from_value(serde_json::to_value(&plan.logical_manifest).unwrap()).unwrap();
    let graph = nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_layer_targets(
        &plan.logical_manifest_sha256,
        &plan.source_sha256,
        plan.source_size_bytes,
        &logical,
        &[],
        &std::collections::HashMap::from([
            ("primary".into(), "node-24".into()),
            ("secondary".into(), "node-22".into()),
        ]),
    )
    .unwrap();
    nrz_runtime_artifact::verify_source_runtime_graph_dependencies(&logical, &graph).unwrap();
    for (name, profile) in [("primary", "NODE_24"), ("secondary", "NODE_22")] {
        let layer = graph
            .wire()
            .runtime_layers
            .iter()
            .find(|layer| layer.layer_name.as_str() == name)
            .unwrap();
        assert_eq!(layer.launch.as_ref().unwrap().profile.to_string(), profile);
    }
}

#[tokio::test]
async fn authored_node_manifest_preserves_standalone_declaration_and_uses_admitted_selection() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("index.html"), b"<html></html>").unwrap();
    fs::write(directory.path().join("server.js"), b"console.log('hello')").unwrap();
    fs::create_dir(directory.path().join(".onreza")).unwrap();
    let detection = crate::detect::detect_with_framework_override(directory.path(), None);
    assert!(detection.metadata.application_runtime().is_none());
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
            let mut effective = provided_manifest_config(directory.path(), &manifest);
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
                None,
            )
            .await;
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
            if admitted && witness.is_none() {
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
        let effective = provided_manifest_config(directory.path(), &manifest);
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
            None,
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

#[test]
fn sibling_target_exemption_rejects_invalid_targets_and_primary_override() {
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
        entry: Some("server.js".into()),
        args: vec![],
        python_version: None,
    };
    for (directory, target) in [
        ("secondary", "node-21"),
        ("secondary", "node-invalid"),
        ("secondary", "bun-1.4.2"),
        (".", "node-22"),
    ] {
        let mut manifest = compute_manifest();
        manifest.layers[0].directory = directory.into();
        manifest.layers[0].runtime = Some(crate::build::manifest::RuntimeConfig {
            application_runtime: Some(declaration.intent()),
            build_runtime_version: Some(target.into()),
            ..Default::default()
        });
        let error = crate::deploy::apply_application_runtime_manifest(
            &mut manifest,
            Some(&declaration),
            Some("node-24"),
            "other",
        )
        .unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<crate::output::CodedError>()
                .unwrap()
                .code
                .as_str(),
            "APPLICATION_RUNTIME_INVALID"
        );
        assert_eq!(
            manifest.layers[0]
                .runtime
                .as_ref()
                .unwrap()
                .build_runtime_version
                .as_deref(),
            Some(target)
        );
    }
}

#[tokio::test]
async fn managed_build_declarations_preserve_frozen_targets_and_static_publication() {
    const CASE: &str = "ONREZA_PYTHON_PRODUCER_SELECTION_TEST";
    let Ok(_selected) = std::env::var(CASE) else {
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
            child.env("ONREZA_BUILD_RUNTIME_VERSION", "python-3.14");
            child.env("ONREZA_BUILD_RUNTIME_FAMILY", "python");
            child.env_remove("ONREZA_BUILD_NODE_MAJOR");
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
    let mut detection = crate::detect::detect_with_framework_override(directory.path(), None);
    crate::detect::application_runtime::resolve_and_bind_detection(
        &crate::detect::fs::LocalFs::new(directory.path()),
        &mut detection,
    )
    .unwrap();
    let context = detection.metadata.source_build_context.clone().unwrap();
    let mut effective = nrz::config::EffectiveProjectConfig::from_project_config(
        directory.path().to_owned(),
        nrz::config::ProjectConfig::default(),
    );
    effective.apply_platform_runner_settings(&nrz::config::ProjectBuildSettings {
        source_build_context: Some(context.clone()),
        ..Default::default()
    });
    let result = crate::deploy::validate_application_runtime_before_build(
        &context,
        &effective,
        true,
        false,
        &[],
    )
    .await;
    assert_eq!(result.unwrap().as_deref(), Some("python-3.14"));
    // The serving target remains Node even when compiler and dependency ABI are Python.
    let node_context = nrz_source_bundle::SourceBuildContext {
        application_runtime: Some(nrz_source_bundle::ApplicationRuntimeDeclaration {
            family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
            python_version: None,
            entry: Some("server.js".into()),
            args: vec![],
        }),
        ..context.clone()
    };
    effective.apply_platform_runner_settings(&nrz::config::ProjectBuildSettings {
        source_build_context: Some(node_context.clone()),
        node_version: Some("NODE_24".into()),
        ..Default::default()
    });
    assert_eq!(
        crate::deploy::validate_application_runtime_before_build(
            &node_context,
            &effective,
            true,
            false,
            &[]
        )
        .await
        .unwrap()
        .as_deref(),
        Some("node-24")
    );
    let mut node_manifest = crate::build::manifest::generate_compute_manifest("server.js");
    crate::deploy::apply_application_runtime_manifest(
        &mut node_manifest,
        node_context.application_runtime.as_ref(),
        Some("node-24"),
        "other",
    )
    .unwrap();
    assert_eq!(
        node_manifest.layers[0]
            .runtime
            .as_ref()
            .unwrap()
            .build_runtime_version
            .as_deref(),
        Some("node-24")
    );

    // A Python compiler does not supply the serving ABI of typed code-only siblings.
    fs::write(directory.path().join("index.html"), b"<html></html>").unwrap();
    fs::write(directory.path().join("server.js"), b"console.log('hello')").unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    config.build.output_dirs = Some(vec![".".into()]);
    config.build.toolchain = Some(nrz_source_bundle::BuildToolchainFamily::Python);
    config.deploy.compute = Some("static".into());
    let mut static_detection =
        crate::detect::detect_with_framework_override(directory.path(), None);
    let static_context = crate::detect::application_runtime::resolve_and_bind_source_build_context(
        &crate::detect::fs::LocalFs::new(directory.path()),
        &mut static_detection,
        &config,
        None,
        None,
    )
    .unwrap();
    assert!(static_context.application_runtime.is_none());
    let mut effective = nrz::config::EffectiveProjectConfig::from_project_config(
        directory.path().to_owned(),
        config,
    );
    effective.apply_platform_runner_settings(&nrz::config::ProjectBuildSettings {
        source_build_context: Some(static_context.clone()),
        ..Default::default()
    });
    assert!(
        crate::deploy::validate_application_runtime_before_build(
            &static_context,
            &effective,
            true,
            false,
            &[]
        )
        .await
        .unwrap()
        .is_none()
    );
    fs::create_dir(directory.path().join(".onreza")).unwrap();
    for (family, target) in [
        (nrz_source_bundle::ApplicationRuntimeFamily::Node, "node-24"),
        (
            nrz_source_bundle::ApplicationRuntimeFamily::Bun,
            "bun-1.4.2",
        ),
    ] {
        let mut manifest = crate::build::manifest::generate_compute_manifest("server.js");
        manifest.layers[0].runtime = Some(crate::build::manifest::RuntimeConfig {
            application_runtime: Some(nrz_source_bundle::ApplicationRuntimeIntent {
                family,
                args: vec![],
            }),
            build_runtime_version: Some(target.into()),
            ..Default::default()
        });
        fs::write(
            directory.path().join(".onreza/manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let built = crate::build::run_with_effective_config(
            crate::cli::BuildArgs {
                dir: directory.path().to_string_lossy().into_owned(),
                skip_validation: false,
            },
            true,
            &effective,
            Some(&static_detection),
            false,
            directory.path(),
            None,
        )
        .await
        .unwrap();
        let manifest = built.manifest.unwrap();
        assert_eq!(
            manifest.layers[0]
                .runtime
                .as_ref()
                .unwrap()
                .build_runtime_version
                .as_deref(),
            Some(target)
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
            target
        );
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

    let extracted = crate::test_support::unpack_source_bundle(&plan);
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
fn source_bundle_retains_static_file_roles_and_content_types() {
    let directory = tempdir().unwrap();
    let expected = [
        ("index.html", Some("text/html; charset=utf-8")),
        ("legacy.htm", Some("text/html; charset=utf-8")),
        ("style.CSS", Some("text/css; charset=utf-8")),
        ("client.js", Some("application/javascript; charset=utf-8")),
        ("module.mjs", Some("application/javascript; charset=utf-8")),
        ("server.cjs", Some("application/javascript; charset=utf-8")),
        ("data.json", Some("application/json; charset=utf-8")),
        ("client.js.map", Some("application/json; charset=utf-8")),
        ("readme.txt", Some("text/plain; charset=utf-8")),
        ("feed.xml", Some("application/xml; charset=utf-8")),
        ("icon.svg", Some("image/svg+xml")),
        ("image.png", Some("image/png")),
        ("image.jpg", Some("image/jpeg")),
        ("image.jpeg", Some("image/jpeg")),
        ("image.gif", Some("image/gif")),
        ("image.webp", Some("image/webp")),
        ("image.avif", Some("image/avif")),
        ("favicon.ico", Some("image/x-icon")),
        ("module.wasm", Some("application/wasm")),
        ("font.woff", Some("font/woff")),
        ("font.woff2", Some("font/woff2")),
        ("font.ttf", Some("font/ttf")),
        ("font.otf", Some("font/otf")),
        ("document.pdf", Some("application/pdf")),
        ("_headers", None),
        ("_redirects", None),
        ("unknown.data", None),
    ];
    for (path, _) in expected {
        fs::write(directory.path().join(path), b"static content").unwrap();
    }
    let plan = static_source_bundle(directory.path());
    assert_eq!(plan.logical_manifest.files.len(), expected.len());
    for (path, content_type) in expected {
        let file = plan
            .logical_manifest
            .files
            .iter()
            .find(|file| file.path == path)
            .unwrap();
        assert_eq!(file.role, SourceLogicalManifestFileRole::Static);
        assert_eq!(file.content_type.as_deref(), content_type, "{path}");
    }
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
            headers: None,
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

    let extracted = crate::test_support::unpack_source_bundle(&plan);
    assert_eq!(
        fs::read_to_string(extracted.path().join("node_modules/.bin/foo")).unwrap(),
        "console.log('foo')"
    );
}

#[cfg(unix)]
#[test]
fn source_bundle_plan_rejects_unarchived_symlink_targets() {
    for target in ["empty", "cache/data.txt"] {
        let directory = tempdir().unwrap();
        fs::create_dir(directory.path().join("empty")).unwrap();
        fs::create_dir(directory.path().join("cache")).unwrap();
        fs::write(directory.path().join("cache/data.txt"), b"cache").unwrap();
        std::os::unix::fs::symlink(target, directory.path().join("alias")).unwrap();
        let mut files = scan_dir(directory.path()).unwrap();
        files.retain(|file| file.path != "cache/data.txt");
        let error =
            build_source_bundle_plan(directory.path(), &static_manifest(), &files).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("target is not included in archive"),
            "{target}: {error}"
        );
    }
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

    let extracted = crate::test_support::unpack_source_bundle(&plan);
    assert_eq!(
        fs::read_to_string(extracted.path().join("alias/link")).unwrap(),
        "real"
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
        symlink_target: None,
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

#[tokio::test]
async fn frozen_compiler_probes_static_and_different_serving_families() {
    const CASE: &str = "ONREZA_COMPILER_PROBE_TEST";
    let output = std::process::Command::new("node")
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    let version = String::from_utf8(output.stdout).unwrap();
    let actual = version
        .trim()
        .strip_prefix('v')
        .unwrap()
        .split('.')
        .next()
        .unwrap();
    assert!(matches!(actual, "22" | "24" | "26"));
    let wrong = if actual == "22" { "24" } else { "22" };
    let Ok(selected) = std::env::var(CASE) else {
        for (family, major) in [("node", actual), ("node", wrong), ("bun", wrong)] {
            let result = tokio::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "artifact::source_bundle_v1_tests::frozen_compiler_probes_static_and_different_serving_families", "--nocapture"])
                .env(CASE, format!("{family}:{major}"))
                .env("ONREZA_BUILD_RUNTIME_FAMILY", "javascript")
                .env("ONREZA_BUILD_RUNTIME_VERSION", if family == "bun" {"bun-1.4.2".into()} else {format!("node-{major}")})
                .env("ONREZA_BUILD_NODE_MAJOR", major)
                .output().await.unwrap();
            assert!(
                result.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
        return;
    };
    let (family, major) = selected.split_once(':').unwrap();
    let directory = tempdir().unwrap();
    let mut effective = nrz::config::EffectiveProjectConfig::from_project_config(
        directory.path().to_owned(),
        nrz::config::ProjectConfig::default(),
    );
    for serving in [
        None,
        Some(nrz_source_bundle::ApplicationRuntimeDeclaration {
            family: nrz_source_bundle::ApplicationRuntimeFamily::Bun,
            python_version: None,
            entry: Some("server.js".into()),
            args: vec![],
        }),
    ] {
        let context = nrz_source_bundle::SourceBuildContext {
            schema_version: 1,
            build_toolchain: nrz_source_bundle::BuildToolchainDeclaration {
                family: if family == "bun" {
                    nrz_source_bundle::BuildToolchainFamily::Bun
                } else {
                    nrz_source_bundle::BuildToolchainFamily::Node
                },
                python_version: None,
            },
            application_runtime: serving,
        };
        effective.apply_platform_runner_settings(&nrz::config::ProjectBuildSettings {
            node_version: Some(format!("NODE_{major}")),
            source_build_context: Some(context.clone()),
            ..Default::default()
        });
        let result = crate::deploy::validate_application_runtime_before_build(
            &context,
            &effective,
            true,
            false,
            &[],
        )
        .await;
        if major == actual {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(
                result
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("does not match selected node-")
            );
        }
    }
}

#[test]
fn source_bundle_writer_rejects_same_size_rewrite() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("index.html"), b"before").unwrap();
    let entries = source_entries(
        directory.path(),
        &scan_dir(directory.path()).unwrap(),
        &RuntimeArtifactScan::All,
    )
    .unwrap();
    fs::write(directory.path().join("index.html"), b"after!").unwrap();
    let archive = directory.path().join("source.tar.zst");
    let error = write_source_bundle(&archive, b"{}", &entries).unwrap_err();
    assert!(
        error.to_string().contains("changed during packaging"),
        "{error}"
    );
    assert!(!archive.exists());
}

#[test]
fn source_bundle_writer_preserves_existing_destination() {
    let directory = tempdir().unwrap();
    let archive = directory.path().join("source.tar.zst");
    fs::write(&archive, b"existing archive").unwrap();
    assert!(write_source_bundle(&archive, b"{}", &[]).is_err());
    assert_eq!(fs::read(&archive).unwrap(), b"existing archive");
}

#[test]
fn source_bundle_rejects_stale_scanned_content_and_size() {
    for replacement in [b"after!".as_slice(), b"larger than before", b"tiny"] {
        let directory = tempdir().unwrap();
        let file = directory.path().join("index.html");
        fs::write(&file, b"before").unwrap();
        let files = scan_dir(directory.path()).unwrap();
        fs::write(&file, replacement).unwrap();
        let error =
            build_source_bundle_plan(directory.path(), &static_manifest(), &files).unwrap_err();
        assert!(
            error.to_string().contains("file changed during packaging"),
            "{error}"
        );
    }
}

#[cfg(unix)]
#[test]
fn source_bundle_rejects_stale_scanned_symlink_identity() {
    for changed_size in [false, true] {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("one.txt"), b"one").unwrap();
        fs::write(directory.path().join("two.txt"), b"two").unwrap();
        let link = directory.path().join("alias");
        std::os::unix::fs::symlink("one.txt", &link).unwrap();
        let mut files = scan_dir(directory.path()).unwrap();
        if changed_size {
            files
                .iter_mut()
                .find(|file| file.path == "alias")
                .unwrap()
                .size = 1;
        } else {
            fs::remove_file(&link).unwrap();
            std::os::unix::fs::symlink("two.txt", &link).unwrap();
        }
        let error =
            build_source_bundle_plan(directory.path(), &static_manifest(), &files).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("symlink changed during packaging"),
            "{error}"
        );
    }
}

#[test]
fn source_bundle_rejects_noncanonical_scanned_paths() {
    let directory = tempdir().unwrap();
    for path in [
        "",
        "/absolute",
        "back\\slash",
        "nul\0name",
        ".",
        "..",
        "a//b",
        "a/./b",
        "a/../b",
    ] {
        let files = [FileEntry {
            path: path.into(),
            size: 0,
            content_hash: sha256_hex(b""),
            kind: crate::artifact::ArtifactFileKind::File,
            symlink_resolved_path: None,
            symlink_target: None,
        }];
        let error =
            build_source_bundle_plan(directory.path(), &static_manifest(), &files).unwrap_err();
        assert!(
            error.to_string().contains("SOURCE_BUNDLE_V1 path"),
            "{path:?}: {error}"
        );
    }
}

#[tokio::test]
async fn source_bundle_archive_preserves_empty_padded_and_large_file_contents() {
    let directory = tempdir().unwrap();
    let expected = [0, 511, 512, 513, 131_073].map(|size| {
        let path = format!("size-{size}.bin");
        let bytes = (0..size)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        fs::write(directory.path().join(&path), &bytes).unwrap();
        (path, bytes)
    });
    let (plan, extracted) = verified_static_bundle(directory.path()).await;
    for (path, bytes) in expected {
        assert_eq!(fs::read(extracted.path().join(&path)).unwrap(), bytes);
        let file = plan
            .logical_manifest
            .files
            .iter()
            .find(|file| file.path == path)
            .unwrap();
        assert_eq!(file.size, bytes.len() as u64);
        assert_eq!(file.sha256, sha256_hex(&bytes));
    }
    let archive_path = plan.source_path().to_owned();
    drop(plan);
    assert!(!archive_path.exists());
}

#[cfg(unix)]
#[test]
fn source_bundle_archive_preserves_executable_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempdir().unwrap();
    for (name, mode) in [
        ("exec", 0o100),
        ("group-exec", 0o010),
        ("other-exec", 0o001),
        ("data", 0o640),
    ] {
        let path = directory.path().join(name);
        fs::write(&path, b"content").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(mode | 0o600)).unwrap();
    }
    let plan = static_source_bundle(directory.path());
    let extracted = crate::test_support::unpack_source_bundle(&plan);
    for file in &plan.logical_manifest.files {
        assert_eq!(file.executable, file.path != "data");
        assert_eq!(
            fs::metadata(extracted.path().join(&file.path))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            if file.path == "data" { 0o644 } else { 0o755 }
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_projects_cross_package_links_in_each_dependency_root() {
    let directory = tempdir().unwrap();
    for path in [
        "packages/a/lib",
        "packages/b",
        "node_modules/.bin",
        "vendor/node_modules/.bin",
    ] {
        fs::create_dir_all(directory.path().join(path)).unwrap();
    }
    fs::write(directory.path().join("server.js"), b"require('a')").unwrap();
    fs::write(
        directory.path().join("packages/a/lib/value.txt"),
        b"internal",
    )
    .unwrap();
    fs::write(
        directory.path().join("packages/b/index.js"),
        b"cross-package",
    )
    .unwrap();
    std::os::unix::fs::symlink(
        "lib/value.txt",
        directory.path().join("packages/a/internal"),
    )
    .unwrap();
    std::os::unix::fs::symlink("../b/index.js", directory.path().join("packages/a/cross")).unwrap();
    for (root, prefix) in [("node_modules", "../"), ("vendor/node_modules", "../../")] {
        for package in ["a", "b"] {
            std::os::unix::fs::symlink(
                format!("{prefix}packages/{package}"),
                directory.path().join(format!("{root}/{package}")),
            )
            .unwrap();
        }
        std::os::unix::fs::symlink(
            format!("../{prefix}packages/a/lib/value.txt"),
            directory.path().join(format!("{root}/.bin/value")),
        )
        .unwrap();
    }
    let scan = RuntimeArtifactScan::Selected {
        roots: vec![
            RuntimeArtifactScanRoot {
                path: ".".into(),
                kind: RuntimeArtifactScanRootKind::BuildOutput,
            },
            RuntimeArtifactScanRoot {
                path: "node_modules".into(),
                kind: RuntimeArtifactScanRootKind::NodeModules,
            },
            RuntimeArtifactScanRoot {
                path: "vendor/node_modules".into(),
                kind: RuntimeArtifactScanRootKind::NodeModules,
            },
        ],
        symlink_roots: vec!["packages/a".into(), "packages/b".into()],
    };
    let plan = build_source_bundle_plan_with_scan(
        directory.path(),
        &compute_manifest(),
        &scan_dir(directory.path()).unwrap(),
        &scan,
        RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();
    let logical = crate::test_support::validated_source_bundle_manifest(&plan);
    crate::test_support::verify_source_bundle(&plan, &logical).await;
    let extracted = crate::test_support::unpack_source_bundle(&plan);
    for root in ["node_modules", "vendor/node_modules"] {
        for (suffix, target, content) in [
            ("a/internal", "lib/value.txt", "internal"),
            ("a/cross", "../b/index.js", "cross-package"),
            (".bin/value", "../a/lib/value.txt", "internal"),
        ] {
            let path = format!("{root}/{suffix}");
            assert_eq!(
                fs::read_to_string(extracted.path().join(&path)).unwrap(),
                content
            );
            let file = plan
                .logical_manifest
                .files
                .iter()
                .find(|file| file.path == path)
                .unwrap();
            assert_eq!(file.link_target.as_deref(), Some(target));
            assert_eq!(file.sha256, sha256_hex(target.as_bytes()));
            assert_eq!(file.role, SourceLogicalManifestFileRole::Dependency);
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_archive_preserves_long_unicode_paths_and_link_targets() {
    let directory = tempdir().unwrap();
    let long_directory = "д".repeat(60);
    let target = format!("{long_directory}/payload.txt");
    fs::create_dir(directory.path().join(&long_directory)).unwrap();
    fs::write(directory.path().join(&target), b"unicode payload").unwrap();
    let alias = format!("{}/alias", "a".repeat(100));
    fs::create_dir(directory.path().join("a".repeat(100))).unwrap();
    std::os::unix::fs::symlink(format!("../{target}"), directory.path().join(&alias)).unwrap();
    let (_plan, extracted) = verified_static_bundle(directory.path()).await;
    assert_eq!(
        fs::read(extracted.path().join(&target)).unwrap(),
        b"unicode payload"
    );
    assert_eq!(
        fs::read(extracted.path().join(&alias)).unwrap(),
        b"unicode payload"
    );
    assert_eq!(
        fs::read_link(extracted.path().join(alias)).unwrap(),
        PathBuf::from(format!("../{target}"))
    );
}

#[test]
fn source_bundle_symlink_limits_count_utf16_units_and_reject_unsafe_targets() {
    let at_limit = "😀".repeat(256);
    validate_source_symlink_target("alias", &at_limit).unwrap();
    assert!(validate_source_symlink_target("alias", &format!("{at_limit}x")).is_err());
    for target in ["", "/absolute", "back\\slash", "nul\0name"] {
        assert!(
            validate_source_symlink_target("alias", target).is_err(),
            "{target:?}"
        );
    }
    validate_source_symlink_target("nested/alias", "./../file.txt").unwrap();
}

#[test]
fn source_bundle_writer_rejects_changed_length_and_removes_partial_destination() {
    for (replacement, reason) in [
        (b"tiny".as_slice(), "truncated"),
        (b"longer than before".as_slice(), "grew"),
    ] {
        let directory = tempdir().unwrap();
        let path = directory.path().join("payload");
        fs::write(&path, b"before").unwrap();
        let entries = source_entries(
            directory.path(),
            &scan_dir(directory.path()).unwrap(),
            &RuntimeArtifactScan::All,
        )
        .unwrap();
        fs::write(&path, replacement).unwrap();
        let archive = directory.path().join("source.tar.zst");
        let error = write_source_bundle(&archive, b"{}", &entries).unwrap_err();
        assert!(error.to_string().contains(reason), "{error}");
        assert!(!archive.exists());
    }
}

#[test]
fn source_bundle_rejects_duplicate_archive_paths() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("file.txt"), b"file").unwrap();
    let mut files = scan_dir(directory.path()).unwrap();
    files.push(files[0].clone());
    let error = build_source_bundle_plan(directory.path(), &static_manifest(), &files).unwrap_err();
    assert!(
        error.to_string().contains("duplicate archive path"),
        "{error}"
    );
}

#[test]
fn native_bundle_does_not_reassign_trusted_dependency_library_custody() {
    let directory = tempdir().unwrap();
    fs::create_dir(directory.path().join("bin")).unwrap();
    fs::create_dir_all(directory.path().join("node_modules/pkg")).unwrap();
    fs::write(
        directory.path().join("node_modules/pkg/libfoo.so"),
        native_elf_fixture(None),
    )
    .unwrap();
    fs::write(
        directory.path().join("bin/server"),
        native_elf_fixture(Some("$ORIGIN/../node_modules/pkg")),
    )
    .unwrap();
    let mut manifest = compute_manifest();
    manifest.layers[0].entry = Some("bin/server".into());
    manifest.layers[0].runtime = Some(serde_json::from_value(serde_json::json!({"applicationRuntime":{"family":"EXECUTABLE","args":[]},"buildRuntimeVersion":"native-linux-x86_64-glibc"})).unwrap());
    let error = build_source_bundle_plan_with_scan(
        directory.path(),
        &manifest,
        &scan_dir(directory.path()).unwrap(),
        &RuntimeArtifactScan::NodeRuntimeRoot,
        RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("not archived by its owning compute layer"),
        "{error}"
    );
}

#[test]
fn source_bundle_assigns_unmatched_files_to_static_fallback_without_prefix_collision() {
    let directory = tempdir().unwrap();
    for path in ["server", "public", "serverish"] {
        fs::create_dir(directory.path().join(path)).unwrap();
    }
    for path in [
        "server/main.js",
        "public/index.html",
        "serverish/file.txt",
        "loose.txt",
    ] {
        fs::write(directory.path().join(path), b"content").unwrap();
    }
    let manifest = serde_json::from_value(serde_json::json!({"version":1,"routes":[],"layers":[
        {"name":"compute","target":"COMPUTE","directory":"server","entry":"main.js"},
        {"name":"static","target":"STATIC","directory":"public"}
    ]}))
    .unwrap();
    let plan = build_source_bundle_plan(
        directory.path(),
        &manifest,
        &scan_dir(directory.path()).unwrap(),
    )
    .unwrap();
    for file in &plan.logical_manifest.files {
        let compute = file.path == "server/main.js";
        assert_eq!(
            file.role,
            if compute {
                SourceLogicalManifestFileRole::Compute
            } else {
                SourceLogicalManifestFileRole::Static
            }
        );
        assert_eq!(
            file.layer_name.as_deref(),
            Some(if compute { "compute" } else { "static" })
        );
    }
}

#[cfg(unix)]
#[test]
fn source_bundle_rejects_symlink_chain_after_scanned_directory_escapes_output() {
    let directory = tempdir().unwrap();
    let outside = tempdir().unwrap();
    fs::create_dir(directory.path().join("bridge")).unwrap();
    fs::write(directory.path().join("bridge/file.txt"), b"unchanged").unwrap();
    fs::write(outside.path().join("file.txt"), b"unchanged").unwrap();
    std::os::unix::fs::symlink("bridge/file.txt", directory.path().join("alias")).unwrap();
    let files = scan_dir(directory.path()).unwrap();
    fs::remove_dir_all(directory.path().join("bridge")).unwrap();
    std::os::unix::fs::symlink(outside.path(), directory.path().join("bridge")).unwrap();
    let error = build_source_bundle_plan(directory.path(), &static_manifest(), &files).unwrap_err();
    assert!(
        error.to_string().contains("symlink escapes build output"),
        "{error}"
    );
}

#[cfg(unix)]
#[test]
fn source_bundle_rejects_filtered_target_beneath_an_archived_symlink_prefix() {
    let directory = tempdir().unwrap();
    fs::create_dir(directory.path().join("package")).unwrap();
    fs::write(directory.path().join("package/keep.txt"), b"kept").unwrap();
    fs::write(directory.path().join("package/filtered.txt"), b"filtered").unwrap();
    std::os::unix::fs::symlink("package", directory.path().join("package-link")).unwrap();
    std::os::unix::fs::symlink("package-link/filtered.txt", directory.path().join("alias"))
        .unwrap();
    let mut files = scan_dir(directory.path()).unwrap();
    files.retain(|file| file.path != "package/filtered.txt");
    let error = build_source_bundle_plan(directory.path(), &static_manifest(), &files).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("target is not included in archive"),
        "{error}"
    );
}

async fn verified_static_bundle(
    directory: &std::path::Path,
) -> (SourceBundlePlan, tempfile::TempDir) {
    let plan = static_source_bundle(directory);
    let logical = crate::test_support::validated_source_bundle_manifest(&plan);
    crate::test_support::verify_source_bundle(&plan, &logical).await;
    let extracted = crate::test_support::unpack_source_bundle(&plan);
    (plan, extracted)
}

#[cfg(unix)]
#[test]
fn source_bundle_archives_pax_only_after_header_field_boundary() {
    let directory = tempdir().unwrap();
    let short = "p".repeat(100);
    let long = "p".repeat(101);
    for (name, alias) in [(&short, "link-100"), (&long, "link-101")] {
        fs::write(directory.path().join(name), b"payload").unwrap();
        std::os::unix::fs::symlink(name, directory.path().join(alias)).unwrap();
    }
    let plan = static_source_bundle(directory.path());
    let decoder =
        zstd::stream::read::Decoder::new(fs::File::open(plan.source_path()).unwrap()).unwrap();
    let mut archive = tar::Archive::new(decoder);
    let mut extensions = Vec::new();
    let mut entry_count = 0;
    for entry in archive.entries().unwrap().raw(true) {
        let mut entry = entry.unwrap();
        assert_eq!(entry.header().uid().unwrap(), 0);
        assert_eq!(entry.header().gid().unwrap(), 0);
        assert_eq!(entry.header().mtime().unwrap(), 0);
        entry_count += 1;
        if entry.header().entry_type().is_pax_local_extensions() {
            let mut body = String::new();
            std::io::Read::read_to_string(&mut entry, &mut body).unwrap();
            extensions.push(body);
        }
    }
    assert_eq!(entry_count, 7);
    assert_eq!(
        extensions,
        [
            format!("115 linkpath={long}\n"),
            format!("111 path={long}\n")
        ]
    );
}

fn static_source_bundle(directory: &std::path::Path) -> SourceBundlePlan {
    build_source_bundle_plan(directory, &static_manifest(), &scan_dir(directory).unwrap()).unwrap()
}

#[cfg(unix)]
#[test]
fn source_bundle_rejects_parent_target_through_an_unarchived_empty_directory() {
    let directory = tempdir().unwrap();
    fs::create_dir(directory.path().join("emptydir")).unwrap();
    fs::write(directory.path().join("real.txt"), b"real").unwrap();
    std::os::unix::fs::symlink("emptydir/../real.txt", directory.path().join("alias")).unwrap();
    let files = scan_dir(directory.path()).unwrap();
    let result = build_source_bundle_plan(directory.path(), &static_manifest(), &files);
    assert!(
        result.is_err(),
        "empty directory omitted from archive made an unresolved physical symlink appear valid"
    );
}

#[cfg(unix)]
#[test]
fn source_bundle_preserves_physical_parent_targets_through_archived_directories() {
    let directory = tempdir().unwrap();
    fs::create_dir_all(directory.path().join("sub/nested")).unwrap();
    fs::write(
        directory.path().join("sub/nested/keep.txt"),
        b"directory witness",
    )
    .unwrap();
    fs::write(directory.path().join("sub/parent.txt"), b"physical parent").unwrap();
    fs::write(directory.path().join("parent.txt"), b"lexical parent").unwrap();
    fs::write(directory.path().join("real.txt"), b"real").unwrap();
    std::os::unix::fs::symlink("sub/nested", directory.path().join("dir-alias")).unwrap();
    std::os::unix::fs::symlink("dir-alias/../parent.txt", directory.path().join("physical"))
        .unwrap();
    std::os::unix::fs::symlink("sub/../real.txt", directory.path().join("relative")).unwrap();
    let plan = static_source_bundle(directory.path());
    let extracted = crate::test_support::unpack_source_bundle(&plan);
    assert_eq!(
        fs::read(extracted.path().join("relative")).unwrap(),
        b"real"
    );
    assert_eq!(
        fs::read(extracted.path().join("physical")).unwrap(),
        b"physical parent"
    );
}

#[cfg(unix)]
#[test]
fn source_bundle_accepts_physical_parents_beyond_the_lexical_alias_depth() {
    let directory = tempdir().unwrap();
    fs::create_dir_all(directory.path().join("dir/deeper/child")).unwrap();
    let records = [
        ("dir/deeper/child/file", "directory witness", false),
        ("dir/real", "physical parent", false),
        ("directory-alias", "dir/deeper/child", true),
        ("alias", "directory-alias/../../real", true),
    ];
    let files = records.map(|(path, body, link)| {
        if link {
            std::os::unix::fs::symlink(body, directory.path().join(path)).unwrap();
        } else {
            fs::write(directory.path().join(path), body).unwrap();
        }
        FileEntry {
            path: path.into(),
            size: if link { 0 } else { body.len() as u64 },
            content_hash: sha256_hex(body.as_bytes()),
            kind: if link {
                ArtifactFileKind::Symlink
            } else {
                ArtifactFileKind::File
            },
            symlink_resolved_path: None,
            symlink_target: None,
        }
    });
    let plan = build_source_bundle_plan(directory.path(), &static_manifest(), &files).unwrap();
    let extracted = crate::test_support::unpack_source_bundle(&plan);
    assert_eq!(
        fs::read(extracted.path().join("alias")).unwrap(),
        b"physical parent"
    );
    assert_eq!(
        fs::read_link(extracted.path().join("alias")).unwrap(),
        PathBuf::from("directory-alias/../../real")
    );
}

#[cfg(unix)]
#[test]
fn source_bundle_rejects_regular_file_parent_swapped_outside_root() {
    for after_planning in [false, true] {
        let directory = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::create_dir(directory.path().join("bridge")).unwrap();
        fs::write(directory.path().join("bridge/file.txt"), b"same bytes").unwrap();
        fs::write(outside.path().join("file.txt"), b"same bytes").unwrap();
        let files = scan_dir(directory.path()).unwrap();
        let planned = after_planning
            .then(|| source_entries(directory.path(), &files, &RuntimeArtifactScan::All).unwrap());
        fs::remove_dir_all(directory.path().join("bridge")).unwrap();
        std::os::unix::fs::symlink(outside.path(), directory.path().join("bridge")).unwrap();
        if let Some(entries) = planned {
            let archive = directory.path().join("source.tar.zst");
            assert!(write_source_bundle(&archive, b"{}", &entries).is_err());
            assert!(!archive.exists(), "rejected input left a partial archive");
        } else {
            let result = build_source_bundle_plan(directory.path(), &static_manifest(), &files);
            assert!(
                result.is_err(),
                "identical bytes outside the source root were accepted after an ancestor replacement"
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn source_bundle_packages_many_files_with_a_small_descriptor_budget() {
    const CHILD: &str = "NRZ_TEST_SOURCE_ROOT_DESCRIPTOR_BUDGET";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "artifact::source_bundle_v1::tests::source_bundle_packages_many_files_with_a_small_descriptor_budget", "--nocapture"])
            .env(CHILD, "1")
            .output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let directory = tempdir().unwrap();
    for index in 0..256 {
        fs::write(
            directory.path().join(format!("file-{index:03}.txt")),
            b"payload",
        )
        .unwrap();
    }
    // The child isolates the process-wide limit from parallel unit tests.
    unsafe {
        let mut limit = std::mem::zeroed::<libc::rlimit>();
        assert_eq!(libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit), 0);
        limit.rlim_cur = limit.rlim_cur.min(64);
        assert_eq!(libc::setrlimit(libc::RLIMIT_NOFILE, &limit), 0);
    }
    let plan = static_source_bundle(directory.path());
    assert_eq!(plan.logical_manifest.files.len(), 256);
    let restored = crate::test_support::unpack_source_bundle(&plan);
    assert_eq!(
        fs::read(restored.path().join("file-255.txt")).unwrap(),
        b"payload"
    );
}
