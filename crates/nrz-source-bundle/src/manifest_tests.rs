use super::*;
use serde_json::json;

#[test]
fn normalizes_safe_relative_paths_only() {
    assert_eq!(
        normalize_source_path("dist/index.html").unwrap(),
        "dist/index.html"
    );
    assert!(normalize_source_path("").is_err());
    assert!(normalize_source_path("/dist/index.html").is_err());
    assert!(normalize_source_path("dist/../secret").is_err());
    assert!(normalize_source_path("dist//index.html").is_err());
    assert!(normalize_source_path("dist\\index.html").is_err());
    for path in ["dist\0index.html", ".", "..", "dist/./index.html", "dist/"] {
        assert!(
            normalize_source_path(path).is_err(),
            "unsafe path admitted: {path:?}"
        );
    }
    for path in ["dist/я.html", "file..name", "file name"] {
        assert_eq!(normalize_source_path(path).unwrap(), path);
    }
}

#[test]
fn serialized_file_defaults_and_explicit_flags_preserve_the_wire_contract() {
    for (entry_type, executable, link_target) in [
        (SourceLogicalManifestEntryType::File, false, None),
        (SourceLogicalManifestEntryType::File, true, None),
        (
            SourceLogicalManifestEntryType::Symlink,
            false,
            Some("target"),
        ),
    ] {
        let file = SourceLogicalManifestFile {
            path: "source".into(),
            sha256: "a".repeat(64),
            size: 0,
            entry_type,
            executable,
            link_target: link_target.map(str::to_string),
            content_type: None,
            role: "compute".into(),
            layer_name: None,
        };
        let mut expected = json!({
            "path":"source", "sha256":"a".repeat(64), "size":0,
            "contentType":null, "role":"compute", "layerName":null
        });
        if executable {
            expected["executable"] = json!(true);
        }
        if let Some(target) = link_target {
            expected["entryType"] = json!("symlink");
            expected["linkTarget"] = json!(target);
        }
        let encoded = serde_json::to_value(&file).unwrap();
        assert_eq!(encoded, expected);
        let decoded: SourceLogicalManifestFile = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.entry_type, entry_type);
        assert_eq!(decoded.executable, executable);
        assert_eq!(decoded.link_target.as_deref(), link_target);
    }
}

#[test]
fn route_header_limits_admit_the_inclusive_boundaries() {
    for headers in [
        (0..128)
            .map(|i| (format!("x-{i}"), "value".to_string()))
            .collect(),
        std::collections::BTreeMap::from([("x".repeat(256), "value".into())]),
        std::collections::BTreeMap::from([("x-value".into(), "x".repeat(8192))]),
    ] {
        let manifest = serde_json::from_value(json!({
            "schemaVersion":SOURCE_BUNDLE_V1_SCHEMA_VERSION,
            "routes":[{"pattern":"/", "layerName":"app", "headers":headers}]
        }))
        .unwrap();
        validate_source_route_headers(&manifest).unwrap();
    }
}

#[test]
fn readiness_paths_and_protocol_diagnostics_preserve_the_contract() {
    let manifest = |readiness: Value| {
        serde_json::from_value(json!({
            "schemaVersion":SOURCE_BUNDLE_V1_SCHEMA_VERSION,
            "layers":[{"name":"app", "target":"COMPUTE", "runtimeConfig":{
                "readiness":readiness
            }}]
        }))
        .unwrap()
    };
    let tcp = source_runtime_readiness(&manifest(json!({"protocol":"TCP"})))
        .unwrap()
        .unwrap();
    assert_eq!(tcp, SourceRuntimeReadiness::Tcp);
    assert_eq!(tcp.health_check_path(), None);
    for path in ["/health".to_string(), format!("/{}", "я".repeat(254))] {
        let http = source_runtime_readiness(&manifest(json!({"protocol":"HTTP", "path":path})))
            .unwrap()
            .unwrap();
        assert_eq!(http.health_check_path(), Some(path.as_str()));
    }
    for readiness in [
        json!({"protocol":"HTTP", "path":format!("/{}", "я".repeat(255))}),
        json!({"protocol":"HTTP", "path":"/health?ready=1"}),
        json!({"protocol":"HTTP", "path":"/health#ready"}),
        json!({"protocol":"HTTP", "path":"/../health"}),
        json!({"protocol":"HTTP", "path":""}),
        json!({"protocol":"HTTP"}),
        json!({"protocol":"UDP"}),
        json!({"protocol":"TCP", "extra":true}),
        json!(false),
    ] {
        assert!(source_runtime_readiness(&manifest(readiness)).is_err());
    }
    for path in [json!("/health"), Value::Null] {
        assert_eq!(
            source_runtime_readiness(&manifest(json!({"protocol":"TCP", "path":path})))
                .unwrap_err(),
            "TCP runtimeConfig.readiness must not declare path"
        );
    }
}

#[test]
fn resolves_one_immutable_readiness_contract_for_compute_layers() {
    let manifest = SourceLogicalManifest {
        schema_version: SOURCE_BUNDLE_V1_SCHEMA_VERSION.to_string(),
        capabilities: Vec::new(),
        files: Vec::new(),
        layers: vec![SourceLogicalManifestLayer {
            name: "server".to_string(),
            target: "COMPUTE".to_string(),
            root_path: None,
            entrypoint: Some("main.py".to_string()),
            runtime_config: Some(serde_json::json!({
                "readiness": { "protocol": "HTTP", "path": "/healthz" },
                "runtimeFamily": "PYTHON"
            })),
        }],
        routes: Vec::new(),
        entrypoints: Vec::new(),
    };

    assert_eq!(
        source_runtime_readiness(&manifest).unwrap(),
        Some(SourceRuntimeReadiness::Http {
            path: "/healthz".to_string()
        })
    );
}

#[test]
fn rejects_ambiguous_or_invalid_runtime_readiness() {
    let layer = |name: &str, readiness: Option<Value>| SourceLogicalManifestLayer {
        name: name.to_string(),
        target: "COMPUTE".to_string(),
        root_path: None,
        entrypoint: Some("main.py".to_string()),
        runtime_config: readiness.map(|readiness| serde_json::json!({ "readiness": readiness })),
    };
    let manifest = |layers| SourceLogicalManifest {
        schema_version: SOURCE_BUNDLE_V1_SCHEMA_VERSION.to_string(),
        capabilities: Vec::new(),
        files: Vec::new(),
        layers,
        routes: Vec::new(),
        entrypoints: Vec::new(),
    };

    assert!(
        source_runtime_readiness(&manifest(vec![
            layer("one", Some(serde_json::json!({ "protocol": "TCP" }))),
            layer("two", None),
        ]))
        .is_err()
    );
    assert!(
        source_runtime_readiness(&manifest(vec![layer(
            "server",
            Some(serde_json::json!({ "protocol": "HTTP", "path": "health" })),
        )]))
        .is_err()
    );
}
