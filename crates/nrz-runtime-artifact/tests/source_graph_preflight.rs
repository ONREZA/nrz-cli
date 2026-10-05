use nrz_runtime_artifact::validate_source_bundle_application_graph;
use nrz_source_bundle::SourceLogicalManifest;
use serde_json::json;

#[test]
fn declared_bun_runtime_preserves_args_and_rejects_node_target() {
    let mut source = manifest("server");
    source.files.retain(|file| file.role != "dependency");
    source.layers[0].runtime_config = Some(json!({
        "applicationRuntime": {"family":"BUN","args":["--port","8080"]}
    }));
    let graph = nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_target(
        &"a".repeat(64),
        &"b".repeat(64),
        1024,
        &source,
        &[],
        Some("bun-1.4.2"),
    )
    .expect("explicit Bun runtime must survive compilation with its trusted target");
    let launch = graph.wire().runtime_layers[0].launch.as_ref().unwrap();
    assert_eq!(launch.profile.to_string(), "BUN");
    assert_eq!(
        serde_json::to_value(&launch.args).unwrap(),
        json!(["--port", "8080"])
    );
    let result = nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_target(
        &"a".repeat(64),
        &"b".repeat(64),
        1024,
        &source,
        &[],
        Some("node-24"),
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("application runtime")
    );
}

#[test]
fn selected_node_version_is_fenced_even_without_dependencies() {
    let mut source = manifest("server");
    source.files.retain(|file| file.role != "dependency");
    source.layers[0].runtime_config = Some(json!({
        "applicationRuntime":{"family":"NODE","args":[]},
        "buildRuntimeVersion":"node-22"
    }));
    let compile = |target| {
        nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_target(
            &"a".repeat(64),
            &"b".repeat(64),
            1024,
            &source,
            &[],
            Some(target),
        )
    };
    let graph = compile("node-22").unwrap();
    assert_eq!(
        graph.wire().runtime_layers[0]
            .launch
            .as_ref()
            .unwrap()
            .profile
            .to_string(),
        "NODE_22"
    );
    let runtime = serde_json::to_value(&graph.wire().runtime_layers[0].runtime_config).unwrap();
    assert!(runtime.get("applicationRuntime").is_none());
    assert!(runtime.get("buildRuntimeVersion").is_none());
    assert!(
        compile("node-24")
            .unwrap_err()
            .to_string()
            .contains("build runtime version")
    );
}

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
