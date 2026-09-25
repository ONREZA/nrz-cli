use nrz_runtime_artifact::validate_source_bundle_application_graph;
use nrz_source_bundle::SourceLogicalManifest;
use serde_json::json;

fn manifest(dependency_layer: &str) -> SourceLogicalManifest {
    serde_json::from_value(json!({
        "schemaVersion": "SOURCE_BUNDLE_V1.0",
        "files": [
            {
                "path": "server/main.js",
                "sha256": "c".repeat(64),
                "size": 42,
                "role": "compute",
                "layerName": "server"
            },
            {
                "path": "node_modules/pkg/index.js",
                "sha256": "d".repeat(64),
                "size": 42,
                "role": "dependency",
                "layerName": dependency_layer
            }
        ],
        "layers": [
            {
                "name": "server",
                "target": "COMPUTE",
                "rootPath": "server",
                "entrypoint": "server/main.js"
            }
        ]
    }))
    .unwrap()
}

#[test]
fn preflight_accepts_application_ownership_before_dependencies_are_materialized() {
    validate_source_bundle_application_graph(
        &"a".repeat(64),
        &"b".repeat(64),
        1024,
        &manifest("server"),
    )
    .unwrap();
}

#[test]
fn preflight_rejects_dependency_files_owned_by_an_unknown_compute_layer() {
    let error = validate_source_bundle_application_graph(
        &"a".repeat(64),
        &"b".repeat(64),
        1024,
        &manifest("other"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("unknown compute layer"));
}
