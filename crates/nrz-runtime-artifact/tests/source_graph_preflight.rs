use nrz_runtime_artifact::validate_source_bundle_application_graph;
use nrz_source_bundle::SourceLogicalManifest;
use serde_json::json;

#[test]
fn source_launch_and_runtime_config_match_the_materialized_graph() {
    for (family, target, profile, args) in [
        ("BUN", "bun-1.4.2", "BUN", json!([])),
        ("NODE", "node-22", "NODE_22", json!(["--port", "8080"])),
        ("NODE", "node-24", "NODE_24", json!(["--port", "8080"])),
        ("NODE", "node-26", "NODE_26", json!(["--port", "8080"])),
    ] {
        let config = json!({
            "applicationRuntime": {"family": family, "args": args},
            "buildRuntimeVersion": target,
            "readiness": {"protocol": "TCP"}
        });
        let mut source = manifest("server");
        source.files.retain(|file| file.role != "dependency");
        source.layers[0].runtime_config = Some(config.clone());
        let graph = nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_target(
            &"a".repeat(64),
            &"b".repeat(64),
            1024,
            &source,
            &[],
            Some(target),
        )
        .unwrap();
        let layer = &graph.wire().runtime_layers[0];
        let launch =
            nrz_runtime_artifact::source_layer_launch_for_target(Some(&config), Some(target))
                .unwrap();
        assert_eq!(launch, *layer.launch.as_ref().unwrap());
        assert_eq!(launch.profile.to_string(), profile);
        assert_eq!(serde_json::to_value(&launch.args).unwrap(), args);
        assert_eq!(
            serde_json::to_value(&layer.runtime_config).unwrap(),
            json!({})
        );
        assert_eq!(
            nrz_runtime_artifact::source_layer_runtime_config(Some(&config)).unwrap(),
            serde_json::to_value(&layer.runtime_config).unwrap()
        );
        assert!(
            nrz_runtime_artifact::source_layer_launch_for_target(Some(&config), Some("bun-1.4.3"))
                .unwrap_err()
                .to_string()
                .contains("build runtime version")
        );
    }
}

#[test]
fn source_runtime_config_projection_preserves_runtime_limits_and_rejects_non_objects() {
    let config = json!({
        "readiness": {"protocol": "TCP"},
        "isBinaryEntry": false,
        "applicationRuntime": {"family": "BUN", "args": []},
        "buildRuntimeVersion": "bun-1.4.2",
        "runtimeFamily": "JAVASCRIPT",
        "memoryMb": 256,
        "timeoutMs": 30000,
        "maxConcurrency": 5
    });
    assert_eq!(
        nrz_runtime_artifact::source_layer_runtime_config(Some(&config)).unwrap(),
        json!({
            "runtimeFamily": "JAVASCRIPT",
            "memoryMb": 256,
            "timeoutMs": 30000,
            "maxConcurrency": 5
        })
    );
    assert_eq!(
        nrz_runtime_artifact::source_layer_runtime_config(None).unwrap(),
        json!({})
    );
    for config in [json!(null), json!([]), json!("invalid")] {
        assert!(nrz_runtime_artifact::source_layer_runtime_config(Some(&config)).is_err());
    }
}

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
fn preflight_preserves_typed_serving_targets_for_python_native_and_mixed_layers() {
    let mut failures = Vec::new();
    for target in [
        "python-3.12",
        "python-3.13",
        "python-3.14",
        "native-linux-x86_64-glibc",
    ] {
        let python = target.starts_with("python-");
        let mut source = manifest("server");
        source.layers[0].runtime_config = Some(json!({
            "applicationRuntime":{"family":if python { "PYTHON" } else { "EXECUTABLE" },"args":["literal argument"]},
            "buildRuntimeVersion":target,
            "runtimeFamily":if python { "PYTHON" } else { "JAVASCRIPT" },
            "isBinaryEntry":!python
        }));
        assert!(
            nrz_runtime_artifact::source_layer_launch_for_target(
                source.layers[0].runtime_config.as_ref(),
                Some("bun-1.4.2"),
            )
            .is_err(),
            "source witness cannot admit a different final target"
        );
        for mixed in [false, true] {
            if mixed {
                let mut node = source.layers[0].clone();
                node.name = "node".into();
                node.root_path = Some("node".into());
                node.entrypoint = Some("node/main.js".into());
                node.runtime_config = Some(json!({
                    "applicationRuntime":{"family":"NODE","args":[]},
                    "buildRuntimeVersion":"node-24"
                }));
                source.layers.push(node);
                let mut file = source.files[0].clone();
                file.path = "node/main.js".into();
                file.layer_name = Some("node".into());
                source.files.push(file);
            }
            if let Err(error) = validate_source_bundle_application_graph(
                &"a".repeat(64),
                &"b".repeat(64),
                1024,
                &source,
            ) {
                failures.push(format!("{target} mixed={mixed}: {error}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn preflight_validates_typed_targets_without_attesting_the_final_admitted_target() {
    let mut source = manifest("server");
    source.layers[0].runtime_config = Some(json!({"buildRuntimeVersion":"node-22"}));
    validate_source_bundle_application_graph(&"a".repeat(64), &"b".repeat(64), 1024, &source)
        .unwrap();
    // Unresolved Node keeps its target-independent ownership preflight; final
    // materialization still rejects it until a trusted target is supplied.
    source.layers[0].runtime_config = Some(json!({
        "applicationRuntime":{"family":"NODE","args":[]}
    }));
    validate_source_bundle_application_graph(&"a".repeat(64), &"b".repeat(64), 1024, &source)
        .unwrap();
    assert!(
        nrz_runtime_artifact::source_layer_launch_for_target(
            source.layers[0].runtime_config.as_ref(),
            None,
        )
        .is_err()
    );
    for (family, target) in [
        ("PYTHON", "python-3.11"),
        ("EXECUTABLE", "node-24"),
        ("NODE", "python-3.12"),
    ] {
        source.layers[0].runtime_config = Some(json!({
            "applicationRuntime":{"family":family,"args":[]},
            "buildRuntimeVersion":target
        }));
        assert!(
            validate_source_bundle_application_graph(
                &"a".repeat(64),
                &"b".repeat(64),
                1024,
                &source,
            )
            .is_err(),
            "preflight accepted incompatible {family}/{target}"
        );
    }
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
