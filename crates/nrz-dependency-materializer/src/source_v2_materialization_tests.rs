use super::*;
use crate::DependencyMaterializationKind;
use nrz_runtime_artifact::verify_dependency_materialization_manifest;

#[test]
fn dependency_descriptor_must_match_the_admitted_root_kind_owner_and_build() {
    let manifest = verify_dependency_materialization_manifest(
        serde_json::from_str(include_str!(
            "../../nrz-contract/fixtures/dependency-materialization-v1-bun.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let root = AdmittedDependencyRoot {
        layer_name: "server",
        source_root: "node_modules",
        kind: DependencyMaterializationKind::JavaScriptNodeModules,
        build_runtime_target: "bun-1.4.2",
    };
    let mut dependency = AdmittedRuntimeDependency {
        root,
        materialization: SourceDependencyMaterializationV3 {
            layer_name: "server",
            mount_point: "/output/node_modules",
            manifest: &manifest,
            physical_blob: None,
        },
    };
    assert!(validate_dependency_bindings(std::slice::from_ref(&dependency)).is_ok());
    for mismatch in [
        AdmittedDependencyRoot {
            layer_name: "other",
            ..root
        },
        AdmittedDependencyRoot {
            source_root: "app/node_modules",
            ..root
        },
        AdmittedDependencyRoot {
            kind: DependencyMaterializationKind::PythonSitePackages,
            ..root
        },
        AdmittedDependencyRoot {
            build_runtime_target: "bun-1.4.3",
            ..root
        },
        AdmittedDependencyRoot {
            build_runtime_target: "1.4.2",
            ..root
        },
    ] {
        dependency.root = mismatch;
        assert!(validate_dependency_bindings(std::slice::from_ref(&dependency)).is_err());
    }
}
