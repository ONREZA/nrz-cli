use nrz_runtime_artifact::{
    compile_source_runtime_layer_for_target, finalize_source_bundle_runtime_graph_for_target,
    verify_source_runtime_graph_dependencies, verify_source_runtime_layer_for_target,
};
use nrz_source_bundle::SourceLogicalManifest;
use serde_json::{Value, json};

fn manifest(config: Value) -> SourceLogicalManifest {
    serde_json::from_value(json!({
        "schemaVersion":"SOURCE_BUNDLE_V1.0", "capabilities":[],
        "files":[{"path":"server/main.js", "sha256":"0".repeat(64), "size":1,
            "role":"compute", "layerName":"server"}],
        "layers":[{"name":"server", "target":"COMPUTE", "rootPath":"server",
            "entrypoint":"server/main.js", "runtimeConfig":config}],
        "routes":[], "entrypoints":[]
    }))
    .unwrap()
}

#[test]
fn native_source_labels_cannot_compile_to_a_contradictory_runtime_graph() {
    for typed in [true, false] {
        for family in ["JAVASCRIPT", "PYTHON"] {
            let mut config = json!({"isBinaryEntry":true,"runtimeFamily":family});
            let target = typed.then_some("native-linux-x86_64-glibc");
            if typed {
                config["applicationRuntime"] = json!({"family":"EXECUTABLE","args":[]});
                config["buildRuntimeVersion"] = json!(target.unwrap());
            }
            let source = manifest(config);
            let compiled = compile_source_runtime_layer_for_target(&source.layers[0], &[], target);
            let graph = finalize_source_bundle_runtime_graph_for_target(
                &"1".repeat(64),
                &"2".repeat(64),
                1,
                &source,
                &[],
                target,
            );
            assert!(
                compiled.is_err() && graph.is_err(),
                "typed={typed}, family={family}: contradictory native source accepted: compiled={compiled:?}, graph={graph:?}"
            );
        }
    }
}

#[test]
fn producer_and_consumer_share_complete_layer_semantics() {
    for (config, target) in [
        (json!({}), None),
        (json!({"runtimeFamily":"PYTHON"}), Some("python-3.14")),
        (json!({"isBinaryEntry":true}), None),
        (
            json!({"applicationRuntime":{"family":"PYTHON","args":["MODULE","demo","two words"]},
                "buildRuntimeVersion":"python-3.12"}),
            Some("python-3.12"),
        ),
        (
            json!({"applicationRuntime":{"family":"PYTHON","args":["MODULE","demo","two words"]},
                "buildRuntimeVersion":"python-3.13"}),
            Some("python-3.13"),
        ),
        (
            json!({"applicationRuntime":{"family":"PYTHON","args":["MODULE","demo","two words"]},
                "buildRuntimeVersion":"python-3.14"}),
            Some("python-3.14"),
        ),
        (
            json!({"applicationRuntime":{"family":"EXECUTABLE","args":["$(id)","two words"]},
                "buildRuntimeVersion":"native-linux-x86_64-glibc"}),
            Some("native-linux-x86_64-glibc"),
        ),
        (
            json!({"applicationRuntime":{"family":"BUN","args":["--fixture","two words"]},
            "buildRuntimeVersion":"bun-1.4.2"}),
            Some("bun-1.4.2"),
        ),
        (
            json!({"applicationRuntime":{"family":"NODE","args":["--fixture","two words"]},
            "buildRuntimeVersion":"node-22"}),
            Some("node-22"),
        ),
        (
            json!({"applicationRuntime":{"family":"NODE","args":["--fixture","two words"]},
            "buildRuntimeVersion":"node-24"}),
            Some("node-24"),
        ),
        (
            json!({"applicationRuntime":{"family":"NODE","args":["--fixture","two words"]},
            "buildRuntimeVersion":"node-26"}),
            Some("node-26"),
        ),
    ] {
        let mut config = config;
        config["memoryMb"] = json!(64);
        config["readiness"] = json!({"protocol":"HTTP","path":"/health"});
        let manifest = manifest(config);
        let graph = finalize_source_bundle_runtime_graph_for_target(
            &"1".repeat(64),
            &"2".repeat(64),
            1,
            &manifest,
            &[],
            target,
        )
        .unwrap();
        let layer = &manifest.layers[0];
        let runtime = &graph.wire().runtime_layers[0];
        assert_eq!(
            compile_source_runtime_layer_for_target(layer, &[], target).unwrap(),
            serde_json::to_value(runtime).unwrap()
        );
        verify_source_runtime_layer_for_target(layer, runtime, target).unwrap();
        for field in [
            "layerName",
            "applicationRoot",
            "entrypoint",
            "args",
            "cwd",
            "readiness",
            "memoryMb",
        ] {
            let mut changed = serde_json::to_value(runtime).unwrap();
            match field {
                "args" => changed["launch"][field] = json!(["changed"]),
                "cwd" => changed["launch"][field] = json!("other"),
                "readiness" => changed["launch"][field] = json!({"protocol":"TCP"}),
                "memoryMb" => changed["runtimeConfig"][field] = json!(128),
                _ => changed[field] = json!("other"),
            }
            let changed = serde_json::from_value(changed).unwrap();
            assert!(
                verify_source_runtime_layer_for_target(layer, &changed, target).is_err(),
                "{field}"
            );
        }
        let mut changed = layer.clone();
        changed.runtime_config.as_mut().unwrap()["memoryMb"] = Value::Null;
        assert!(verify_source_runtime_layer_for_target(&changed, runtime, target).is_err());
        changed = layer.clone();
        changed.runtime_config.as_mut().unwrap()["buildRuntimeVersion"] = json!("bun-0.0.0");
        assert!(verify_source_runtime_layer_for_target(&changed, runtime, target).is_err());
        assert!(compile_source_runtime_layer_for_target(&changed, &[], target).is_err());
    }
}

#[test]
fn source_dependency_ownership_is_checked_on_the_consumer_boundary() {
    let mut manifest = manifest(json!({}));
    let graph = finalize_source_bundle_runtime_graph_for_target(
        &"1".repeat(64),
        &"2".repeat(64),
        1,
        &manifest,
        &[],
        None,
    )
    .unwrap();
    verify_source_runtime_graph_dependencies(&manifest, &graph).unwrap();
    let mut dependency = manifest.files[0].clone();
    dependency.path = "server/node_modules/pkg/index.js".into();
    dependency.role = "dependency".into();
    manifest.files.push(dependency);
    assert!(verify_source_runtime_graph_dependencies(&manifest, &graph).is_err());
    manifest.files[1].layer_name = None;
    assert!(verify_source_runtime_graph_dependencies(&manifest, &graph).is_err());
    manifest.files[1].layer_name = Some("other".into());
    assert!(verify_source_runtime_graph_dependencies(&manifest, &graph).is_err());
}

#[test]
fn existing_python_and_bun_layers_need_independent_frozen_build_facts() {
    let mut source = manifest(json!({"applicationRuntime":{"family":"BUN","args":[]},
        "buildRuntimeVersion":"bun-1.4.2"}));
    let mut python = source.layers[0].clone();
    python.name = "python".into();
    python.root_path = Some("python".into());
    python.entrypoint = Some("python/main.py".into());
    python.runtime_config = Some(json!({"runtimeFamily":"PYTHON"}));
    let mut file = source.files[0].clone();
    file.path = "python/main.py".into();
    file.layer_name = Some("python".into());
    source.layers.push(python);
    source.files.push(file);
    assert!(
        finalize_source_bundle_runtime_graph_for_target(
            &"1".repeat(64),
            &"2".repeat(64),
            2,
            &source,
            &[],
            Some("bun-1.4.2"),
        )
        .is_err()
    );
    source.layers[1].runtime_config.as_mut().unwrap()["buildRuntimeVersion"] = json!("python-3.14");
    assert!(
        finalize_source_bundle_runtime_graph_for_target(
            &"1".repeat(64),
            &"2".repeat(64),
            2,
            &source,
            &[],
            Some("bun-1.4.2"),
        )
        .is_err()
    );
    let targets = std::collections::HashMap::from([
        ("server".into(), "bun-1.4.2".into()),
        ("python".into(), "python-3.14".into()),
    ]);
    let graph = nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_layer_targets(
        &"1".repeat(64),
        &"2".repeat(64),
        2,
        &source,
        &[],
        &targets,
    )
    .unwrap();
    for layer in &source.layers {
        let runtime = graph
            .wire()
            .runtime_layers
            .iter()
            .find(|runtime| runtime.layer_name.as_str() == layer.name)
            .unwrap();
        verify_source_runtime_layer_for_target(
            layer,
            runtime,
            targets.get(&layer.name).map(String::as_str),
        )
        .unwrap();
    }
    let mut incomplete = targets.clone();
    incomplete.remove("python");
    assert!(
        nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_layer_targets(
            &"1".repeat(64),
            &"2".repeat(64),
            2,
            &source,
            &[],
            &incomplete
        )
        .is_err()
    );
    let mut extra = targets;
    extra.insert("static-or-unknown".into(), "bun-1.4.2".into());
    assert!(
        nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_layer_targets(
            &"1".repeat(64),
            &"2".repeat(64),
            2,
            &source,
            &[],
            &extra
        )
        .is_err()
    );
}

#[test]
fn typed_mixed_layers_keep_independent_frozen_targets_and_literal_arguments() {
    use nrz_runtime_artifact::{
        RuntimeProfile, finalize_source_bundle_runtime_graph_for_layer_targets,
    };
    use std::collections::HashMap;
    let mut source = manifest(
        json!({"applicationRuntime":{"family":"BUN","args":[]}, "buildRuntimeVersion":"bun-1.4.2"}),
    );
    let mut targets = HashMap::from([("server".into(), "bun-1.4.2".into())]);
    for (name, family, target) in [
        ("python", "PYTHON", "python-3.14"),
        ("native", "EXECUTABLE", "native-linux-x86_64-glibc"),
    ] {
        let mut layer = source.layers[0].clone();
        layer.name = name.into();
        layer.root_path = Some(name.into());
        layer.entrypoint = Some(format!("{name}/entry"));
        layer.runtime_config = Some(
            json!({"applicationRuntime":{"family":family,"args":["$(id)","two words"]}, "buildRuntimeVersion":target}),
        );
        let mut file = source.files[0].clone();
        file.path = format!("{name}/entry");
        file.layer_name = Some(name.into());
        source.layers.push(layer);
        source.files.push(file);
        targets.insert(name.into(), target.into());
    }
    let graph = finalize_source_bundle_runtime_graph_for_layer_targets(
        &"1".repeat(64),
        &"2".repeat(64),
        3,
        &source,
        &[],
        &targets,
    )
    .unwrap();
    for (index, profile) in [
        RuntimeProfile::Bun,
        RuntimeProfile::Cpython314,
        RuntimeProfile::Executable,
    ]
    .into_iter()
    .enumerate()
    {
        let launch = graph.wire().runtime_layers[index].launch.as_ref().unwrap();
        assert_eq!(launch.profile, profile);
        if index > 0 {
            assert_eq!(
                launch
                    .args
                    .iter()
                    .map(|arg| arg.as_str())
                    .collect::<Vec<_>>(),
                ["$(id)", "two words"]
            );
        }
    }
    targets.insert("native".into(), "python-3.14".into());
    assert!(
        finalize_source_bundle_runtime_graph_for_layer_targets(
            &"1".repeat(64),
            &"2".repeat(64),
            3,
            &source,
            &[],
            &targets
        )
        .is_err()
    );
}
