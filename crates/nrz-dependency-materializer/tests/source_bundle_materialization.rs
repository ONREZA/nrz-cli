use std::fs::{self, File};
use std::io::Cursor;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use nrz_dependency_materializer::{
    DependencyMaterializationKind, DependencyTreeLimits, ErofsToolchain,
    SourceBundleMaterializationPolicy, SourceBundleMaterializationRequest,
    canonicalization_policy_digest, materialize_source_bundle_runtime,
};
use nrz_source_bundle::{
    SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH, SOURCE_BUNDLE_V1_SCHEMA_VERSION, SourceLogicalManifest,
    SourceLogicalManifestEntryType, SourceLogicalManifestFile, SourceLogicalManifestLayer,
    compute_logical_manifest_sha256, sha256_hex,
};
use serde_json::json;
use tempfile::TempDir;

const SERVER_BODY: &[u8] = b"export default { fetch() {} };\n";
const DEPENDENCY_BODY: &[u8] = b"export const dependency = true;\n";

#[test]
fn native_policy_has_no_dependency_images_and_rejects_foreign_dependency_inputs() {
    let temp = TempDir::new().unwrap();
    let toolchain = fake_erofs_toolchain(temp.path());
    let mut manifest = source_manifest();
    manifest.layers[0].runtime_config = Some(json!({
        "applicationRuntime":{"family":"EXECUTABLE","args":["$(id)","two words"]},
        "buildRuntimeVersion":"native-linux-x86_64-glibc"
    }));
    let mut compatibility = compatibility();
    compatibility["runtimeFamily"] = json!("native");
    compatibility["runtimeVersion"] = json!("native-linux-x86_64-glibc");
    let rejected_root = temp.path().join("rejected");
    let make = |manifest: &SourceLogicalManifest, output_root: &Path| {
        materialize_source_bundle_runtime(
            &toolchain,
            SourceBundleMaterializationRequest {
                source_path: &temp.path().join("absent-source.tar.zst"),
                logical_manifest_sha256: &"a".repeat(64),
                source_sha256: &"b".repeat(64),
                source_size_bytes: 1,
                manifest,
                output_root,
                policy: SourceBundleMaterializationPolicy {
                    kind: None,
                    compatibility: compatibility.clone(),
                    tree_limits: tree_limits(),
                    max_total_files: 10,
                    max_total_bytes: 1024,
                },
            },
        )
    };
    assert!(matches!(
        make(&manifest, &rejected_root),
        Err(
            nrz_dependency_materializer::SourceBundleMaterializationError::UnexpectedDependencies { .. }
        )
    ));
    assert!(!rejected_root.exists());
    manifest.files.retain(|file| file.role != "dependency");
    let result = make(&manifest, &temp.path().join("runtime")).unwrap();
    assert!(result.dependencies.is_empty());
    let launch = result.graph.wire().runtime_layers[0]
        .launch
        .as_ref()
        .unwrap();
    assert_eq!(
        launch.profile,
        nrz_runtime_artifact::RuntimeProfile::Executable
    );
    assert_eq!(
        launch
            .args
            .iter()
            .map(|arg| arg.as_str())
            .collect::<Vec<_>>(),
        ["$(id)", "two words"]
    );
}

#[test]
fn materializes_dependency_images_and_exact_runtime_graph_from_one_source_bundle() {
    for (family, version, expected_profile) in [
        ("bun", "1.4.2", "BUN"),
        ("javascript", "node-22", "NODE_22"),
        ("javascript", "node-24", "NODE_24"),
        ("javascript", "node-26", "NODE_26"),
    ] {
        let temp = TempDir::new().unwrap();
        let mut compatibility = compatibility();
        compatibility["runtimeFamily"] = json!(family);
        compatibility["runtimeVersion"] = json!(version);
        let mut manifest = source_manifest();
        manifest.layers.push(SourceLogicalManifestLayer {
            name: "worker".into(),
            target: "COMPUTE".into(),
            root_path: Some("worker".into()),
            entrypoint: Some("worker/main".into()),
            runtime_config: Some(json!({
                "applicationRuntime":{"family":"EXECUTABLE","args":["$(id)","two words"]},
                "buildRuntimeVersion":"native-linux-x86_64-glibc"
            })),
        });
        let mut worker = source_file("worker/main", SERVER_BODY, "compute");
        worker.layer_name = Some("worker".into());
        worker.executable = true;
        manifest.files.push(worker);
        let source_path = temp.path().join("source.tar.zst");
        write_source_bundle(&source_path, &manifest);
        let source_bytes = fs::read(&source_path).unwrap();
        let logical_manifest = serde_json::to_value(&manifest).unwrap();
        let logical_manifest_sha256 = compute_logical_manifest_sha256(&logical_manifest);
        let source_sha256 = sha256_hex(&source_bytes);
        let toolchain = fake_erofs_toolchain(temp.path());
        let output_root = temp.path().join("runtime");

        let result = materialize_source_bundle_runtime(
            &toolchain,
            SourceBundleMaterializationRequest {
                source_path: &source_path,
                logical_manifest_sha256: &logical_manifest_sha256,
                source_sha256: &source_sha256,
                source_size_bytes: source_bytes.len() as u64,
                manifest: &manifest,
                output_root: &output_root,
                policy: SourceBundleMaterializationPolicy {
                    kind: Some(DependencyMaterializationKind::JavaScriptNodeModules),
                    compatibility,
                    tree_limits: tree_limits(),
                    max_total_files: 10,
                    max_total_bytes: 1024,
                },
            },
        )
        .unwrap();

        assert_eq!(result.dependencies.len(), 1);
        let dependency = &result.dependencies[0];
        assert_eq!(dependency.layer_name, "server");
        assert_eq!(dependency.mount_point, "/output/node_modules");
        assert_eq!(
            fs::read(&dependency.image_path).unwrap(),
            b"fake-erofs-v1\n"
        );
        assert_eq!(
            fs::metadata(&dependency.image_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o444
        );
        assert_eq!(dependency.manifest.wire().expanded_file_count, 1);
        assert_eq!(
            dependency.manifest.wire().expanded_bytes,
            DEPENDENCY_BODY.len() as i64
        );
        assert_eq!(
            dependency
                .manifest
                .wire()
                .canonicalization_policy_digest
                .as_str(),
            canonicalization_policy_digest()
        );

        let graph = result.graph.wire();
        assert_eq!(
            graph.runtime_layers[0]
                .launch
                .as_ref()
                .unwrap()
                .profile
                .to_string(),
            expected_profile
        );
        assert_eq!(graph.dependencies.len(), 1);
        assert_eq!(
            graph.dependencies[0].mount_point.as_str(),
            "/output/node_modules"
        );
        assert_eq!(
            graph.dependencies[0].materialization_id.as_str(),
            dependency.manifest.materialization_id()
        );
        assert_eq!(graph.runtime_layers.len(), 2);
        let worker = &graph.runtime_layers[1];
        assert_eq!(worker.layer_name.as_str(), "worker");
        assert!(worker.dependency_materialization_ids.is_empty());
        let launch = worker.launch.as_ref().unwrap();
        assert_eq!(
            launch.profile,
            nrz_runtime_artifact::RuntimeProfile::Executable
        );
        assert_eq!(
            launch
                .args
                .iter()
                .map(|arg| arg.as_str())
                .collect::<Vec<_>>(),
            ["$(id)", "two words"]
        );
        assert_eq!(graph.runtime_layers[0].layer_name.as_str(), "server");
        assert_eq!(graph.runtime_layers[0].entrypoint.as_str(), "server.js");
        assert_eq!(
            graph.runtime_layers[0].dependency_materialization_ids[0].as_str(),
            dependency.manifest.materialization_id()
        );
        assert_eq!(
            graph.application.manifest_digest.as_str(),
            logical_manifest_sha256
        );
        assert_eq!(
            graph.application.blob_descriptor.digest.as_str(),
            format!("sha256:{source_sha256}")
        );
    }
}

#[test]
fn independent_code_only_serving_layers_keep_their_own_frozen_targets() {
    for (build_family, build_target, kind, serving_family, serving_target, profile) in [
        (
            "javascript",
            "node-24",
            Some(DependencyMaterializationKind::JavaScriptNodeModules),
            "PYTHON",
            "python-3.13",
            "CPYTHON_3_13",
        ),
        (
            "python",
            "python-3.12",
            Some(DependencyMaterializationKind::PythonSitePackages),
            "NODE",
            "node-24",
            "NODE_24",
        ),
        (
            "native",
            "native-linux-x86_64-glibc",
            None,
            "NODE",
            "node-22",
            "NODE_22",
        ),
        (
            "python",
            "python-3.12",
            Some(DependencyMaterializationKind::PythonSitePackages),
            "PYTHON",
            "python-3.13",
            "CPYTHON_3_13",
        ),
        (
            "javascript",
            "node-24",
            Some(DependencyMaterializationKind::JavaScriptNodeModules),
            "NODE",
            "node-22",
            "NODE_22",
        ),
    ] {
        let temp = TempDir::new().unwrap();
        let mut manifest = source_manifest();
        manifest.files.retain(|file| file.role != "dependency");
        manifest.layers[0].runtime_config = Some(json!({
            "applicationRuntime":{"family":serving_family,"args":["literal arg"]},
            "buildRuntimeVersion":serving_target
        }));
        let mut compatibility = compatibility();
        compatibility["runtimeFamily"] = json!(build_family);
        compatibility["runtimeVersion"] = json!(build_target);
        let result = materialize_source_bundle_runtime(
            &fake_erofs_toolchain(temp.path()),
            SourceBundleMaterializationRequest {
                source_path: &temp.path().join("unused.tar.zst"),
                logical_manifest_sha256: &"a".repeat(64),
                source_sha256: &"b".repeat(64),
                source_size_bytes: 1,
                manifest: &manifest,
                output_root: &temp.path().join("runtime"),
                policy: SourceBundleMaterializationPolicy {
                    kind,
                    compatibility: compatibility.clone(),
                    tree_limits: tree_limits(),
                    max_total_files: 10,
                    max_total_bytes: 1024,
                },
            },
        )
        .unwrap();
        assert!(result.dependencies.is_empty());
        let layers = &result.graph.wire().runtime_layers;
        assert_eq!(layers.len(), 1);
        let launch = layers[0].launch.as_ref().unwrap();
        assert_eq!(launch.profile.to_string(), profile);
        assert_eq!(launch.args[0].as_str(), "literal arg");

        let root = nrz_source_bundle::PythonMinor::from_target(serving_target)
            .map(|minor| minor.site_packages_root())
            .unwrap_or("node_modules");
        manifest.files.push(source_file(
            &format!("{root}/pkg/index.py"),
            DEPENDENCY_BODY,
            "dependency",
        ));
        let output_root = temp.path().join("rejected-dependencies");
        let result = materialize_source_bundle_runtime(
            &fake_erofs_toolchain(temp.path()),
            SourceBundleMaterializationRequest {
                source_path: &temp.path().join("unused.tar.zst"),
                logical_manifest_sha256: &"a".repeat(64),
                source_sha256: &"b".repeat(64),
                source_size_bytes: 1,
                manifest: &manifest,
                output_root: &output_root,
                policy: SourceBundleMaterializationPolicy {
                    kind,
                    compatibility,
                    tree_limits: tree_limits(),
                    max_total_files: 10,
                    max_total_bytes: 1024,
                },
            },
        );
        let error = result.err().expect("foreign dependencies must be rejected");
        if kind.is_none() {
            assert!(matches!(error, nrz_dependency_materializer::SourceBundleMaterializationError::UnexpectedDependencies { .. }));
        } else if (kind == Some(DependencyMaterializationKind::PythonSitePackages))
            != serving_target.starts_with("python-")
        {
            assert!(matches!(error, nrz_dependency_materializer::SourceBundleMaterializationError::DependencyKindMismatch { .. }));
        } else {
            assert!(
                error
                    .to_string()
                    .contains("sibling dependencies require their own frozen build policy")
            );
        }
        assert!(
            !output_root.exists(),
            "reject foreign dependencies before IO"
        );
    }
}

#[test]
fn primary_dependencies_keep_their_build_target_while_siblings_use_their_frozen_declarations() {
    for (primary_family, primary_version, primary_profile, config, expected_profile) in [
        (
            "bun",
            "1.4.2",
            "BUN",
            json!({"runtimeFamily":"PYTHON","buildRuntimeVersion":"python-3.14"}),
            "CPYTHON_3_14",
        ),
        (
            "bun",
            "1.4.2",
            "BUN",
            json!({"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-24"}),
            "NODE_24",
        ),
        (
            "javascript",
            "node-24",
            "NODE_24",
            json!({"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-22"}),
            "NODE_22",
        ),
    ] {
        let temp = TempDir::new().unwrap();
        let mut manifest = source_manifest();
        let primary_target = if primary_family == "bun" {
            format!("bun-{primary_version}")
        } else {
            primary_version.to_owned()
        };
        manifest.layers[0].runtime_config = Some(json!({
            "applicationRuntime":{"family":if primary_family == "bun" {"BUN"} else {"NODE"},"args":[]},
            "buildRuntimeVersion":primary_target
        }));
        let mut python = manifest.layers[0].clone();
        python.name = "python".into();
        python.root_path = Some("python".into());
        python.entrypoint = Some("python/main.py".into());
        python.runtime_config = Some(config);
        manifest.layers.push(python);
        let mut file = source_file("python/main.py", SERVER_BODY, "compute");
        file.layer_name = Some("python".into());
        manifest.files.push(file);
        let source_path = temp.path().join("mixed-source.tar.zst");
        write_source_bundle(&source_path, &manifest);
        let bytes = fs::read(&source_path).unwrap();
        let sha = sha256_hex(&bytes);
        let manifest_sha =
            compute_logical_manifest_sha256(&serde_json::to_value(&manifest).unwrap());
        let mut primary_compatibility = compatibility();
        primary_compatibility["runtimeFamily"] = json!(primary_family);
        primary_compatibility["runtimeVersion"] = json!(primary_version);
        let result = materialize_source_bundle_runtime(
            &fake_erofs_toolchain(temp.path()),
            SourceBundleMaterializationRequest {
                source_path: &source_path,
                logical_manifest_sha256: &manifest_sha,
                source_sha256: &sha,
                source_size_bytes: bytes.len() as u64,
                manifest: &manifest,
                output_root: &temp.path().join("mixed-runtime"),
                policy: SourceBundleMaterializationPolicy {
                    kind: Some(DependencyMaterializationKind::JavaScriptNodeModules),
                    compatibility: primary_compatibility.clone(),
                    tree_limits: tree_limits(),
                    max_total_files: 10,
                    max_total_bytes: 1024,
                },
            },
        )
        .unwrap();
        assert_eq!(result.dependencies.len(), 1);
        assert_eq!(
            serde_json::to_value(&result.dependencies[0].manifest.wire().compatibility).unwrap()["runtimeVersion"],
            primary_version
        );
        let layers = &result.graph.wire().runtime_layers;
        assert_eq!(layers.len(), 2);
        assert_eq!(
            layers
                .iter()
                .find(|l| l.layer_name.as_str() == "server")
                .unwrap()
                .launch
                .as_ref()
                .unwrap()
                .profile
                .to_string(),
            primary_profile
        );
        let python = layers
            .iter()
            .find(|l| l.layer_name.as_str() == "python")
            .unwrap();
        assert_eq!(
            python.launch.as_ref().unwrap().profile.to_string(),
            expected_profile
        );
        assert!(python.dependency_materialization_ids.is_empty());
        // Without its own target, a same-family layer belongs to the primary policy.
        if primary_family == "javascript" {
            continue;
        }
        manifest.layers[1]
            .runtime_config
            .as_mut()
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("buildRuntimeVersion");
        let missing_root = temp.path().join("missing-sibling-runtime");
        let error = materialize_source_bundle_runtime(
            &fake_erofs_toolchain(temp.path()),
            SourceBundleMaterializationRequest {
                source_path: &temp.path().join("unused-source.tar.zst"),
                logical_manifest_sha256: &manifest_sha,
                source_sha256: &sha,
                source_size_bytes: bytes.len() as u64,
                manifest: &manifest,
                output_root: &missing_root,
                policy: SourceBundleMaterializationPolicy {
                    kind: Some(DependencyMaterializationKind::JavaScriptNodeModules),
                    compatibility: primary_compatibility,
                    tree_limits: tree_limits(),
                    max_total_files: 10,
                    max_total_bytes: 1024,
                },
            },
        )
        .err()
        .unwrap();
        assert!(
            error
                .to_string()
                .contains("frozen build runtime declaration")
        );
        assert!(!missing_root.exists());
    }
}

#[test]
fn a_sibling_declaration_never_allows_a_foreign_dependency_kind() {
    let temp = TempDir::new().unwrap();
    let mut manifest = source_manifest();
    let mut python = manifest.layers[0].clone();
    python.name = "python".into();
    python.root_path = Some("python".into());
    python.entrypoint = Some("python/main.py".into());
    python.runtime_config =
        Some(json!({"runtimeFamily":"PYTHON","buildRuntimeVersion":"python-3.14"}));
    manifest.layers.push(python);
    let mut main = source_file("python/main.py", SERVER_BODY, "compute");
    main.layer_name = Some("python".into());
    manifest.files.push(main);
    let mut dependency = source_file(
        ".onreza/python/3.14/site-packages/pkg/__init__.py",
        DEPENDENCY_BODY,
        "dependency",
    );
    dependency.layer_name = Some("python".into());
    manifest.files.push(dependency);
    let source_path = temp.path().join("foreign-deps.tar.zst");
    write_source_bundle(&source_path, &manifest);
    let bytes = fs::read(&source_path).unwrap();
    let result = materialize_source_bundle_runtime(
        &fake_erofs_toolchain(temp.path()),
        SourceBundleMaterializationRequest {
            source_path: &source_path,
            logical_manifest_sha256: &compute_logical_manifest_sha256(
                &serde_json::to_value(&manifest).unwrap(),
            ),
            source_sha256: &sha256_hex(&bytes),
            source_size_bytes: bytes.len() as u64,
            manifest: &manifest,
            output_root: &temp.path().join("foreign-runtime"),
            policy: SourceBundleMaterializationPolicy {
                kind: Some(DependencyMaterializationKind::JavaScriptNodeModules),
                compatibility: compatibility(),
                tree_limits: tree_limits(),
                max_total_files: 10,
                max_total_bytes: 1024,
            },
        },
    );
    assert!(matches!(
        result,
        Err(
            nrz_dependency_materializer::SourceBundleMaterializationError::DependencyKindMismatch { .. }
        )
    ));
    assert!(!temp.path().join("foreign-runtime").exists());
}

#[test]
fn rejects_compute_runtime_family_that_disagrees_with_build_policy() {
    let temp = TempDir::new().unwrap();
    let mut manifest = source_manifest();
    manifest.files.retain(|file| file.role != "dependency");
    manifest.layers[0].runtime_config = Some(json!({ "runtimeFamily": "PYTHON" }));
    let output_root = temp.path().join("runtime");
    let toolchain = fake_erofs_toolchain(temp.path());

    let error = materialize_source_bundle_runtime(
        &toolchain,
        SourceBundleMaterializationRequest {
            source_path: &temp.path().join("unused-source.tar.zst"),
            logical_manifest_sha256: &"a".repeat(64),
            source_sha256: &"b".repeat(64),
            source_size_bytes: 1,
            manifest: &manifest,
            output_root: &output_root,
            policy: SourceBundleMaterializationPolicy {
                kind: Some(DependencyMaterializationKind::JavaScriptNodeModules),
                compatibility: compatibility(),
                tree_limits: tree_limits(),
                max_total_files: 10,
                max_total_bytes: 1024,
            },
        },
    )
    .err()
    .expect("runtime family mismatch must fail before materialization");

    assert!(
        error
            .to_string()
            .contains("build policy requires JAVASCRIPT")
    );
    assert!(!output_root.exists());
}

#[test]
fn runtime_intent_and_version_mismatch_fail_before_any_materialization_io() {
    for (runtime, expected) in [
        (
            json!({"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"bun-1.4.2"}),
            "application runtime",
        ),
        (
            json!({"applicationRuntime":{"family":"BUN","args":[]}}),
            "requires a frozen build runtime declaration",
        ),
        (
            json!({"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-25"}),
            "application runtime",
        ),
        (
            json!({"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":"python-3.15"}),
            "application runtime",
        ),
        (
            json!({"applicationRuntime":{"family":"EXECUTABLE","args":[]},"buildRuntimeVersion":"native-linux-arm64-glibc"}),
            "qualified build target",
        ),
    ] {
        let temp = TempDir::new().unwrap();
        let mut manifest = source_manifest();
        manifest.files.retain(|file| file.role != "dependency");
        manifest.layers[0].runtime_config = Some(runtime);
        let output_root = temp.path().join("runtime");
        let toolchain = fake_erofs_toolchain(temp.path());
        let mut compatibility = compatibility();
        compatibility["runtimeFamily"] = json!("javascript");
        compatibility["runtimeVersion"] = json!("node-24");
        let result = materialize_source_bundle_runtime(
            &toolchain,
            SourceBundleMaterializationRequest {
                source_path: &temp.path().join("unused.tar.zst"),
                logical_manifest_sha256: &"a".repeat(64),
                source_sha256: &"b".repeat(64),
                source_size_bytes: 1,
                manifest: &manifest,
                output_root: &output_root,
                policy: SourceBundleMaterializationPolicy {
                    kind: Some(DependencyMaterializationKind::JavaScriptNodeModules),
                    compatibility,
                    tree_limits: tree_limits(),
                    max_total_files: 10,
                    max_total_bytes: 1024,
                },
            },
        );
        let error = result.err().unwrap().to_string();
        assert!(error.contains(expected), "{error}");
        assert!(!output_root.exists());
    }
}

#[test]
fn materializes_a_manifest_owned_cross_tree_dependency_symlink() {
    const LINK_TARGET: &str = "../../../node_modules/@prisma/client";
    let temp = TempDir::new().unwrap();
    let mut manifest = source_manifest();
    manifest.files.push(source_file(
        "node_modules/@prisma/client/index.js",
        DEPENDENCY_BODY,
        "dependency",
    ));
    manifest.files.push(SourceLogicalManifestFile {
        path: ".next/node_modules/@prisma/client-generated".to_string(),
        sha256: sha256_hex(LINK_TARGET.as_bytes()),
        size: 0,
        entry_type: SourceLogicalManifestEntryType::Symlink,
        link_target: Some(LINK_TARGET.to_string()),
        content_type: None,
        role: "dependency".to_string(),
        layer_name: Some("server".to_string()),
        executable: false,
    });
    let source_path = temp.path().join("source-cross-tree.tar.zst");
    let encoder =
        zstd::stream::write::Encoder::new(File::create(&source_path).unwrap(), 1).unwrap();
    let mut archive = tar::Builder::new(encoder);
    append_file(
        &mut archive,
        SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH,
        &serde_json::to_vec(&manifest).unwrap(),
    );
    append_file(&mut archive, "server/server.js", SERVER_BODY);
    append_file(&mut archive, "node_modules/pkg/index.js", DEPENDENCY_BODY);
    if manifest
        .files
        .iter()
        .any(|file| file.path == "python/main.py")
    {
        append_file(&mut archive, "python/main.py", SERVER_BODY);
    }
    append_file(
        &mut archive,
        "node_modules/@prisma/client/index.js",
        DEPENDENCY_BODY,
    );
    append_symlink(
        &mut archive,
        ".next/node_modules/@prisma/client-generated",
        LINK_TARGET,
    );
    let encoder = archive.into_inner().unwrap();
    encoder.finish().unwrap();
    let source_bytes = fs::read(&source_path).unwrap();
    let logical_manifest_sha256 =
        compute_logical_manifest_sha256(&serde_json::to_value(&manifest).unwrap());
    let source_sha256 = sha256_hex(&source_bytes);

    let result = materialize_source_bundle_runtime(
        &fake_erofs_toolchain(temp.path()),
        SourceBundleMaterializationRequest {
            source_path: &source_path,
            logical_manifest_sha256: &logical_manifest_sha256,
            source_sha256: &source_sha256,
            source_size_bytes: source_bytes.len() as u64,
            manifest: &manifest,
            output_root: &temp.path().join("runtime-cross-tree"),
            policy: SourceBundleMaterializationPolicy {
                kind: Some(DependencyMaterializationKind::JavaScriptNodeModules),
                compatibility: compatibility(),
                tree_limits: tree_limits(),
                max_total_files: 10,
                max_total_bytes: 1024,
            },
        },
    )
    .unwrap();

    assert_eq!(result.dependencies.len(), 2);
    let root = result
        .dependencies
        .iter()
        .find(|dependency| dependency.mount_point == "/output/node_modules")
        .unwrap();
    let next = result
        .dependencies
        .iter()
        .find(|dependency| dependency.mount_point == "/output/.next/node_modules")
        .unwrap();
    assert_eq!(
        root.manifest.wire().canonicalization_policy_digest.as_str(),
        canonicalization_policy_digest()
    );
    assert_ne!(
        next.manifest.wire().canonicalization_policy_digest.as_str(),
        canonicalization_policy_digest()
    );
    assert_eq!(next.manifest.wire().symlink_count, 1);
    assert_eq!(result.graph.wire().dependencies.len(), 2);
}

fn source_manifest() -> SourceLogicalManifest {
    SourceLogicalManifest {
        schema_version: SOURCE_BUNDLE_V1_SCHEMA_VERSION.to_string(),
        capabilities: Vec::new(),
        files: vec![
            source_file("server/server.js", SERVER_BODY, "compute"),
            source_file("node_modules/pkg/index.js", DEPENDENCY_BODY, "dependency"),
        ],
        layers: vec![SourceLogicalManifestLayer {
            name: "server".to_string(),
            target: "COMPUTE".to_string(),
            root_path: Some("server".to_string()),
            entrypoint: Some("server/server.js".to_string()),
            runtime_config: Some(json!({ "memoryMb": 256 })),
        }],
        routes: Vec::new(),
        entrypoints: Vec::new(),
    }
}

fn source_file(path: &str, body: &[u8], role: &str) -> SourceLogicalManifestFile {
    SourceLogicalManifestFile {
        path: path.to_string(),
        sha256: sha256_hex(body),
        size: body.len() as u64,
        entry_type: SourceLogicalManifestEntryType::File,
        link_target: None,
        content_type: None,
        role: role.to_string(),
        layer_name: Some("server".to_string()),
        executable: false,
    }
}

fn write_source_bundle(path: &Path, manifest: &SourceLogicalManifest) {
    let encoder = zstd::stream::write::Encoder::new(File::create(path).unwrap(), 1).unwrap();
    let mut archive = tar::Builder::new(encoder);
    append_file(
        &mut archive,
        SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH,
        &serde_json::to_vec(manifest).unwrap(),
    );
    for file in &manifest.files {
        append_file(
            &mut archive,
            &file.path,
            if file.role == "dependency" {
                DEPENDENCY_BODY
            } else {
                SERVER_BODY
            },
        );
    }
    let encoder = archive.into_inner().unwrap();
    encoder.finish().unwrap();
}

fn append_file<W: std::io::Write>(archive: &mut tar::Builder<W>, path: &str, body: &[u8]) {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(0o644);
    header.set_size(body.len() as u64);
    header.set_cksum();
    archive
        .append_data(&mut header, path, Cursor::new(body))
        .unwrap();
}

fn append_symlink<W: std::io::Write>(archive: &mut tar::Builder<W>, path: &str, target: &str) {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_mode(0o777);
    header.set_size(0);
    header.set_link_name(target).unwrap();
    header.set_cksum();
    archive
        .append_data(&mut header, path, Cursor::new([]))
        .unwrap();
}

fn fake_erofs_toolchain(root: &Path) -> ErofsToolchain {
    let mkfs = root.join("mkfs.erofs");
    let fsck = root.join("fsck.erofs");
    write_executable(
        &mkfs,
        "#!/bin/sh\nset -eu\nprevious=\ncurrent=\nfor argument in \"$@\"; do\n  previous=$current\n  current=$argument\ndone\nprintf 'fake-erofs-v1\\n' > \"$previous\"\n",
    );
    write_executable(
        &fsck,
        "#!/bin/sh\nset -eu\nlast=\nfor argument in \"$@\"; do\n  last=$argument\ndone\ntest -s \"$last\"\n",
    );
    ErofsToolchain::new_direct_for_tests(mkfs, fsck).unwrap()
}

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn compatibility() -> serde_json::Value {
    json!({
        "runtimeFamily": "bun",
        "runtimeVersion": "1.4.2",
        "os": "linux",
        "architecture": "x86_64",
        "libc": "glibc",
        "abi": "glibc-2.42",
        "packageManager": "bun",
        "packageManagerVersion": "1.4.2",
        "runnerRootfsDigest": format!("sha256:{}", "d".repeat(64)),
        "buildPolicyGeneration": 1,
    })
}

fn tree_limits() -> DependencyTreeLimits {
    DependencyTreeLimits {
        max_files: 10,
        max_expanded_bytes: 1024,
        max_path_bytes: 512,
        max_symlinks: 10,
    }
}

#[test]
fn interpreter_sibling_dependencies_require_their_own_build_policy() {
    for (primary_family, primary_version, sibling_family, sibling_version) in [
        ("javascript", "node-24", "BUN", "bun-1.4.2"),
        ("bun", "1.4.2", "NODE", "node-24"),
        ("javascript", "node-24", "NODE", "node-22"),
        ("javascript", "node-22", "NODE", "node-24"),
        ("python", "python-3.12", "PYTHON", "python-3.13"),
    ] {
        let temp = TempDir::new().unwrap();
        let mut manifest = source_manifest();
        if primary_family == "python" {
            manifest.layers[0].runtime_config = Some(
                json!({"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":primary_version}),
            );
            manifest
                .files
                .iter_mut()
                .filter(|file| file.role == "dependency")
                .for_each(|file| {
                    file.path = file
                        .path
                        .replace("node_modules", ".onreza/python/3.12/site-packages")
                });
        }
        let mut sibling = manifest.layers[0].clone();
        sibling.name = "worker".into();
        sibling.root_path = Some("worker".into());
        sibling.entrypoint = Some("worker/main.js".into());
        sibling.runtime_config = Some(json!({
            "applicationRuntime":{"family":sibling_family,"args":[]},
            "buildRuntimeVersion":sibling_version
        }));
        manifest.layers.push(sibling);
        for (path, body, role) in [
            ("worker/main.js", SERVER_BODY, "compute"),
            (
                if sibling_family == "PYTHON" {
                    ".onreza/python/3.13/site-packages/pkg/index.js"
                } else {
                    "worker/node_modules/pkg/index.js"
                },
                DEPENDENCY_BODY,
                "dependency",
            ),
        ] {
            let mut file = source_file(path, body, role);
            file.layer_name = Some("worker".into());
            manifest.files.push(file);
        }
        let source_path = temp.path().join("source.tar.zst");
        write_source_bundle(&source_path, &manifest);
        let bytes = fs::read(&source_path).unwrap();
        let mut compatibility = compatibility();
        compatibility["runtimeFamily"] = json!(primary_family);
        compatibility["runtimeVersion"] = json!(primary_version);
        let output_root = temp.path().join("runtime");
        let result = materialize_source_bundle_runtime(
            &fake_erofs_toolchain(temp.path()),
            SourceBundleMaterializationRequest {
                source_path: &source_path,
                logical_manifest_sha256: &compute_logical_manifest_sha256(
                    &serde_json::to_value(&manifest).unwrap(),
                ),
                source_sha256: &sha256_hex(&bytes),
                source_size_bytes: bytes.len() as u64,
                manifest: &manifest,
                output_root: &output_root,
                policy: SourceBundleMaterializationPolicy {
                    kind: Some(if primary_family == "python" {
                        DependencyMaterializationKind::PythonSitePackages
                    } else {
                        DependencyMaterializationKind::JavaScriptNodeModules
                    }),
                    compatibility,
                    tree_limits: tree_limits(),
                    max_total_files: 10,
                    max_total_bytes: 1024,
                },
            },
        );
        assert!(
            result
                .err()
                .expect("foreign dependencies must fail admission")
                .to_string()
                .contains("sibling dependencies require their own frozen build policy"),
            "foreign interpreter dependencies cannot inherit primary build provenance"
        );
        assert!(
            !output_root.exists(),
            "reject incompatible policy before extraction/materialization"
        );
    }
}
