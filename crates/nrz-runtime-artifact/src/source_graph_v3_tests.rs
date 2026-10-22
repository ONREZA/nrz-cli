use nrz_contract::application_package_v1::PackageDescriptor;
use nrz_source_bundle::source_bundle_v2::{
    SourceBundleV2ManifestBudget, verify_source_bundle_v2_manifest,
};
use serde_json::{Value, json};

use super::*;
use crate::application_package_v1::verify_application_package_descriptor;

fn policy() -> ApplicationPackagePolicy {
    ApplicationPackagePolicy {
        canonicalization_profile: "test-profile-v1".into(),
        generator_identity: "test-generator-v1".into(),
    }
}

fn package(role: Role) -> VerifiedApplicationPackageDescriptor {
    let descriptor: PackageDescriptor = serde_json::from_value(json!({
        "schemaVersion":"APPLICATION_PACKAGE_V1.0", "role":role,
        "blob":{"mediaType":crate::application_package_v1::APPLICATION_PACKAGE_MEDIA_TYPE,
            "digest":{"algorithm":"BLAKE3", "value":"a".repeat(64)}, "size":4096},
        "canonicalizationProfile":policy().canonicalization_profile,
        "generatorIdentity":policy().generator_identity, "erofsSize":4096,
        "verity":{"version":1, "hashAlgorithm":"SHA256", "rootHash":"b".repeat(64),
            "salt":"c".repeat(64), "dataBlockSize":4096, "hashBlockSize":4096,
            "dataBlockCount":1, "dataOffset":0, "treeOffset":4096, "treeLength":0}
    }))
    .unwrap();
    verify_application_package_descriptor(descriptor, &policy()).unwrap()
}

fn file(path: &str, role: &str, layer: Option<&str>) -> Value {
    let mut file = json!({"path":path, "entryType":"file", "role":role,
        "blobDigest":{"algorithm":"BLAKE3", "value":"d".repeat(64)},
        "size":1, "executable":false});
    if let Some(layer) = layer {
        file["layerName"] = json!(layer);
    }
    file
}

fn manifest() -> Value {
    json!({
        "schemaVersion":"SOURCE_BUNDLE_V2.0", "capabilities":["COMPUTE_NETWORK_V1"],
        "files":[file("public/index.html", "static", None),
            file("server/main.js", "compute", Some("server"))],
        "layers":[{"name":"public", "target":"STATIC", "rootPath":"public"},
            {"name":"server", "target":"COMPUTE", "rootPath":"server",
                "entrypoint":"server/main.js", "runtimeConfig":{
                    "applicationRuntime":{"family":"BUN", "args":["--test"]},
                    "network":nrz_source_bundle::ComputeNetwork::public_default(),
                    "buildRuntimeVersion":"bun-1.4.2", "timeoutMs":9_007_199_254_740_991u64,
                    "memoryMb":256, "runtimePolicyLineage":"BUN:CURRENT"}}],
        "routes":[{"pattern":"/", "layerName":"server"}],
        "entrypoints":["server/main.js"]
    })
}

fn verified(mut value: Value) -> VerifiedSourceBundleV2Manifest {
    value["files"]
        .as_array_mut()
        .unwrap()
        .sort_by(|left, right| {
            left["path"]
                .as_str()
                .unwrap()
                .cmp(right["path"].as_str().unwrap())
        });
    verify_source_bundle_v2_manifest(
        value,
        SourceBundleV2ManifestBudget {
            max_file_count: 100,
            max_logical_bytes: 100,
            max_static_file_bytes: 100,
        },
    )
    .unwrap()
}

fn targets() -> [LayerBuildTarget<'static>; 1] {
    [LayerBuildTarget {
        layer_name: "server",
        build_runtime_target: "bun-1.4.2",
    }]
}

fn dependency() -> VerifiedDependencyMaterializationManifest {
    crate::verify_dependency_materialization_manifest(
        serde_json::from_str(include_str!(
            "../../nrz-contract/fixtures/dependency-materialization-v1-bun.json"
        ))
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn bounded_projection_preserves_source_owners_and_full_safe_integer_timeout() {
    let source = verified(manifest());
    let static_package = package(Role::Static);
    let compute_package = package(Role::Compute);
    let graph = compile_source_bundle_v2_runtime_graph(
        &source,
        &[&compute_package, &static_package],
        &targets(),
        &[],
        &policy(),
    )
    .unwrap();
    assert_eq!(
        graph.wire().application.manifest.size,
        source.manifest_size_bytes()
    );
    assert_eq!(
        graph.wire().application.manifest.digest.value,
        source.manifest_blake3()
    );
    assert_eq!(graph.wire().application.components[0].role, Role::Static);
    assert!(
        matches!(&graph.wire().layers[0], graph::Layer::Static { name, root_path }
        if name.0 == "public" && root_path.as_ref().unwrap().0 == "public")
    );
    let graph::Layer::Compute {
        entrypoint,
        launch,
        limits,
        network,
        build_runtime_target,
        runtime_policy_lineage,
        ..
    } = &graph.wire().layers[1]
    else {
        panic!("expected compute layer");
    };
    assert_eq!(entrypoint.0, "server/main.js");
    assert_eq!(launch.profile, crate::RuntimeProfile::Bun);
    assert_eq!(launch.args, ["--test"]);
    assert_eq!(limits.timeout_ms, Some(9_007_199_254_740_991));
    assert_eq!(limits.memory_mb, Some(256));
    assert_eq!(build_runtime_target, "bun-1.4.2");
    assert_eq!(runtime_policy_lineage.as_deref(), Some("BUN:CURRENT"));
    assert_eq!(
        network,
        source.wire().layers[1]
            .runtime_config
            .as_ref()
            .unwrap()
            .network
            .as_ref()
            .unwrap()
    );
}

#[test]
fn source_roles_require_exact_nonempty_package_membership() {
    let source = verified(manifest());
    let static_package = package(Role::Static);
    let compute_package = package(Role::Compute);
    for components in [
        vec![],
        vec![&static_package],
        vec![&compute_package],
        vec![&static_package, &static_package],
    ] {
        assert!(
            compile_source_bundle_v2_runtime_graph(
                &source,
                &components,
                &targets(),
                &[],
                &policy(),
            )
            .is_err()
        );
    }
    for role in ["config", "prerender"] {
        let mut value = manifest();
        value["files"][0]["role"] = json!(role);
        value["files"][0]["layerName"] = json!("server");
        let source = verified(value);
        assert!(
            compile_source_bundle_v2_runtime_graph(
                &source,
                &[&static_package, &compute_package],
                &targets(),
                &[],
                &policy(),
            )
            .is_ok()
        );
    }
    let source = verified(
        json!({"schemaVersion":"SOURCE_BUNDLE_V2.0", "capabilities":[],
        "files":[], "layers":[], "routes":[], "entrypoints":[]}),
    );
    assert!(compile_source_bundle_v2_runtime_graph(&source, &[], &[], &[], &policy()).is_ok());
    assert!(
        compile_source_bundle_v2_runtime_graph(&source, &[&static_package], &[], &[], &policy())
            .is_err()
    );
}

#[test]
fn independent_targets_are_required_exactly_once_and_source_witness_cannot_override_them() {
    let source = verified(manifest());
    let static_package = package(Role::Static);
    let compute_package = package(Role::Compute);
    for targets in [
        vec![],
        vec![LayerBuildTarget {
            layer_name: "server",
            build_runtime_target: "node-24",
        }],
        vec![LayerBuildTarget {
            layer_name: "public",
            build_runtime_target: "bun-1.4.2",
        }],
        vec![LayerBuildTarget {
            layer_name: "unknown",
            build_runtime_target: "bun-1.4.2",
        }],
        vec![
            LayerBuildTarget {
                layer_name: "server",
                build_runtime_target: "bun-1.4.2",
            },
            LayerBuildTarget {
                layer_name: "server",
                build_runtime_target: "bun-1.4.2",
            },
        ],
    ] {
        assert!(
            compile_source_bundle_v2_runtime_graph(
                &source,
                &[&static_package, &compute_package],
                &targets,
                &[],
                &policy(),
            )
            .is_err()
        );
    }
}

#[test]
fn dependencies_require_file_owner_coverage_and_cannot_shadow_application_files() {
    let mut value = manifest();
    value["files"].as_array_mut().unwrap().push(file(
        "server/node_modules/pkg.js",
        "dependency",
        Some("server"),
    ));
    let source = verified(value.clone());
    let static_package = package(Role::Static);
    let compute_package = package(Role::Compute);
    let materialization = dependency();
    assert!(
        compile_source_bundle_v2_runtime_graph(
            &source,
            &[&static_package, &compute_package],
            &targets(),
            &[],
            &policy(),
        )
        .is_err()
    );
    for (layer_name, mount_point) in [
        ("server", "/output/other/node_modules"),
        ("unknown", "/output/server/node_modules"),
        ("server", "/output/server"),
    ] {
        assert!(
            compile_source_bundle_v2_runtime_graph(
                &source,
                &[&static_package, &compute_package],
                &targets(),
                &[SourceDependencyMaterializationV3 {
                    layer_name,
                    mount_point,
                    manifest: &materialization,
                    physical_blob: None
                }],
                &policy(),
            )
            .is_err()
        );
    }
    value["files"].as_array_mut().unwrap().push(file(
        "server/node_modules/asset.js",
        "static",
        None,
    ));
    let source = verified(value);
    assert!(
        compile_source_bundle_v2_runtime_graph(
            &source,
            &[&static_package, &compute_package],
            &targets(),
            &[SourceDependencyMaterializationV3 {
                layer_name: "server",
                mount_point: "/output/server/node_modules",
                manifest: &materialization,
                physical_blob: None
            }],
            &policy(),
        )
        .is_err()
    );
}

#[test]
fn shared_dependency_has_one_descriptor_and_exact_deterministic_layer_mounts() {
    let mut value = manifest();
    value["files"].as_array_mut().unwrap().extend([
        file("server/node_modules/pkg.js", "dependency", Some("server")),
        file("worker/main.js", "compute", Some("worker")),
        file("worker/node_modules/pkg.js", "dependency", Some("worker")),
    ]);
    let mut worker = value["layers"][1].clone();
    worker["name"] = json!("worker");
    worker["rootPath"] = json!("worker");
    worker["entrypoint"] = json!("worker/main.js");
    value["layers"].as_array_mut().unwrap().push(worker);
    value["routes"].as_array_mut().unwrap().push(json!({
        "pattern":"/worker", "layerName":"worker"
    }));
    value["entrypoints"]
        .as_array_mut()
        .unwrap()
        .push(json!("worker/main.js"));
    let source = verified(value);
    let static_package = package(Role::Static);
    let compute_package = package(Role::Compute);
    let materialization = dependency();
    let mut dependencies = [
        SourceDependencyMaterializationV3 {
            layer_name: "worker",
            mount_point: "/output/worker/node_modules",
            manifest: &materialization,
            physical_blob: None,
        },
        SourceDependencyMaterializationV3 {
            layer_name: "server",
            mount_point: "/output/server/node_modules",
            manifest: &materialization,
            physical_blob: None,
        },
    ];
    let targets = [
        LayerBuildTarget {
            layer_name: "server",
            build_runtime_target: "bun-1.4.2",
        },
        LayerBuildTarget {
            layer_name: "worker",
            build_runtime_target: "bun-1.4.2",
        },
    ];
    let graph = compile_source_bundle_v2_runtime_graph(
        &source,
        &[&static_package, &compute_package],
        &targets,
        &dependencies,
        &policy(),
    )
    .unwrap();
    assert_eq!(graph.wire().dependencies.len(), 1);
    for (index, mount) in [
        (1, "/output/server/node_modules"),
        (2, "/output/worker/node_modules"),
    ] {
        let graph::Layer::Compute {
            dependency_mounts, ..
        } = &graph.wire().layers[index]
        else {
            panic!("expected compute layer");
        };
        assert_eq!(dependency_mounts.len(), 1);
        assert_eq!(dependency_mounts[0].mount_point.as_str(), mount);
        assert_eq!(
            dependency_mounts[0].materialization_id.as_str(),
            materialization.materialization_id()
        );
    }
    dependencies.reverse();
    let reordered = compile_source_bundle_v2_runtime_graph(
        &source,
        &[&compute_package, &static_package],
        &targets,
        &dependencies,
        &policy(),
    )
    .unwrap();
    assert_eq!(graph.graph_digest(), reordered.graph_digest());
}

#[test]
fn explicit_dependency_physical_identity_preserves_legacy_proof_and_exact_size() {
    let mut value = manifest();
    value["files"].as_array_mut().unwrap().push(file(
        "server/node_modules/pkg.js",
        "dependency",
        Some("server"),
    ));
    let source = verified(value);
    let static_package = package(Role::Static);
    let compute_package = package(Role::Compute);
    let materialization = dependency();
    let mut physical = BlobReference {
        media_type: crate::DEPENDENCY_EROFS_MEDIA_TYPE.into(),
        digest: source::BlobDigest {
            algorithm: source::DigestAlgorithm::Blake3,
            value: "f".repeat(64),
        },
        size: 4096,
    };
    let graph = compile_source_bundle_v2_runtime_graph(
        &source,
        &[&static_package, &compute_package],
        &targets(),
        &[SourceDependencyMaterializationV3 {
            layer_name: "server",
            mount_point: "/output/server/node_modules",
            manifest: &materialization,
            physical_blob: Some(&physical),
        }],
        &policy(),
    )
    .unwrap();
    assert_eq!(
        graph.wire().dependencies[0].physical_blob.as_ref(),
        Some(&physical)
    );
    assert_eq!(
        graph.wire().dependencies[0]
            .legacy
            .materialization_id
            .as_str(),
        materialization.materialization_id()
    );
    assert_eq!(
        graph.wire().dependencies[0]
            .legacy
            .blob_descriptor
            .digest
            .as_str(),
        format!("sha256:{}", "e".repeat(64))
    );
    physical.size += 1;
    assert!(
        compile_source_bundle_v2_runtime_graph(
            &source,
            &[&static_package, &compute_package],
            &targets(),
            &[SourceDependencyMaterializationV3 {
                layer_name: "server",
                mount_point: "/output/server/node_modules",
                manifest: &materialization,
                physical_blob: Some(&physical),
            }],
            &policy(),
        )
        .is_err()
    );
}

#[test]
fn empty_dependency_materialization_does_not_require_a_source_file_placeholder() {
    let source = verified(manifest());
    let static_package = package(Role::Static);
    let compute_package = package(Role::Compute);
    let mut empty = serde_json::to_value(dependency().wire()).unwrap();
    empty["expandedFileCount"] = json!(0);
    empty["regularFileCount"] = json!(0);
    empty["expandedBytes"] = json!(0);
    let materialization = crate::verify_dependency_materialization_manifest(empty).unwrap();
    assert!(
        compile_source_bundle_v2_runtime_graph(
            &source,
            &[&static_package, &compute_package],
            &targets(),
            &[SourceDependencyMaterializationV3 {
                layer_name: "server",
                mount_point: "/output/server/node_modules",
                manifest: &materialization,
                physical_blob: None,
            }],
            &policy(),
        )
        .is_ok()
    );
}
