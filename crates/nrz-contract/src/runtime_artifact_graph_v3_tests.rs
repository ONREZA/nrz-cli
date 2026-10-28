use crate::RuntimeArtifactGraphV3Wire;
use serde_json::{Value, json};

fn graph() -> Value {
    json!({
        "schemaVersion": "RUNTIME_ARTIFACT_GRAPH_V3.0",
        "application": {
            "schemaVersion": "APPLICATION_MEMBERSHIP_V1.0",
            "manifest": {"mediaType": "application/vnd.onreza.source-bundle.manifest.v2+json",
                "digest": {"algorithm": "BLAKE3", "value": "a".repeat(64)}, "size": 1024},
            "components": []
        },
        "dependencies": [],
        "layers": [
            {"target": "STATIC", "name": "public"},
            {"target": "COMPUTE", "name": "api", "rootPath": "server",
                "entrypoint": "server/main.js",
                "launch": {"profile": "NODE_24", "args": ["", "--flag"]},
                "network": {"schemaVersion": "COMPUTE_NETWORK_V1", "exposure": "PUBLIC",
                    "ports": [{"name": "http", "protocol": "http", "port": 3000, "binding": "primary"}],
                    "readiness": {"protocol": "tcp", "port": "http"}, "allowFrom": []},
                "limits": {"timeoutMs": 9007199254740991_u64, "memoryMb": 256},
                "buildRuntimeTarget": "node-24", "dependencyMounts": []}
        ]
    })
}

#[test]
fn graph_wire_preserves_targets_launch_and_safe_integer_limits() {
    let value = graph();
    let wire: RuntimeArtifactGraphV3Wire = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(wire).unwrap(), value);
}

#[test]
fn graph_wire_rejects_unknowns_removed_launch_fields_and_null_options() {
    for (pointer, field, supplied) in [
        ("", "graphDigest", json!("a".repeat(64))),
        ("", "routes", json!([])),
        ("/application", "artifactId", json!("b".repeat(64))),
        ("/layers/0", "rootPath", Value::Null),
        ("/layers/1", "rootPath", Value::Null),
        ("/layers/1", "runtimePolicyLineage", Value::Null),
        ("/layers/1/limits", "timeoutMs", Value::Null),
        ("/layers/1/limits", "memoryMb", Value::Null),
        ("/layers/1/launch", "cwd", json!(".")),
        ("/layers/1/launch", "readiness", json!({"protocol": "TCP"})),
    ] {
        let mut value = graph();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(field.into(), supplied);
        assert!(
            serde_json::from_value::<RuntimeArtifactGraphV3Wire>(value).is_err(),
            "{pointer}/{field}"
        );
    }
}

#[test]
fn graph_wire_requires_closed_discriminators_and_compute_launch() {
    for (pointer, supplied) in [
        ("/schemaVersion", json!("RUNTIME_ARTIFACT_GRAPH_V2.2")),
        (
            "/application/schemaVersion",
            json!("APPLICATION_MEMBERSHIP_V2.0"),
        ),
        ("/layers/0/target", json!("MIDDLEWARE")),
        ("/layers/1/launch/profile", json!("NODE_028")),
    ] {
        let mut value = graph();
        *value.pointer_mut(pointer).unwrap() = supplied;
        assert!(
            serde_json::from_value::<RuntimeArtifactGraphV3Wire>(value).is_err(),
            "{pointer}"
        );
    }
    let mut value = graph();
    value["layers"][1].as_object_mut().unwrap().remove("launch");
    assert!(serde_json::from_value::<RuntimeArtifactGraphV3Wire>(value).is_err());
}

#[test]
fn dependency_physical_reference_is_optional_but_cannot_be_null_or_sha256() {
    let mut value = graph();
    value["dependencies"] = json!([{"legacy": {
        "materializationId": "b".repeat(64), "kind": "JAVASCRIPT_NODE_MODULES",
        "mountPoint": "/output/server/node_modules", "manifestDigest": "c".repeat(64),
        "blobDescriptor": {"mediaType": "application/vnd.onreza.dependency.erofs.v1",
            "digest": format!("sha256:{}", "d".repeat(64)), "size": 4096},
        "compatibility": {"runtimeFamily": "javascript", "runtimeVersion": "node-24",
            "os": "linux", "architecture": "x86_64", "libc": "glibc", "abi": "node-127",
            "packageManager": "npm", "packageManagerVersion": "10",
            "runnerRootfsDigest": format!("sha256:{}", "e".repeat(64)), "buildPolicyGeneration": 1}
    }}]);
    let wire: RuntimeArtifactGraphV3Wire = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(wire).unwrap(), value);
    value["dependencies"][0]["physicalBlob"] = Value::Null;
    assert!(serde_json::from_value::<RuntimeArtifactGraphV3Wire>(value.clone()).is_err());
    value["dependencies"][0]["physicalBlob"] = json!({
        "mediaType": "application/vnd.onreza.dependency.erofs.v1",
        "digest": {"algorithm": "BLAKE3", "value": "f".repeat(64)}, "size": 4096
    });
    let wire: RuntimeArtifactGraphV3Wire = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(wire).unwrap(), value);
    value["dependencies"][0]["physicalBlob"]["digest"]["algorithm"] = json!("SHA256");
    assert!(serde_json::from_value::<RuntimeArtifactGraphV3Wire>(value).is_err());
}

#[cfg(feature = "codegen")]
#[test]
fn runtime_graph_schema_projects_legacy_types_without_mirroring() {
    let mut legacy: Value = serde_json::from_str(include_str!(
        "../schemas/runtime-artifact-graph-v2.schema.json"
    ))
    .unwrap();
    sort_required(&mut legacy);
    let graph = crate::artifact_schemas::runtime_artifact_graph_v3();
    assert_eq!(
        graph["definitions"]["DependencyDescriptor"]["properties"]["legacy"],
        legacy
            .pointer("/properties/runtimeArtifactGraph/properties/dependencies/items")
            .unwrap()
            .clone()
    );
    assert_eq!(graph["definitions"]["Launch"]["properties"]["profile"],
        legacy.pointer("/properties/runtimeArtifactGraph/properties/runtimeLayers/items/properties/launch/properties/profile").unwrap().clone());
    let serialized = serde_json::to_string(&graph).unwrap();
    assert!(serialized.contains("9007199254740991"));
    assert!(!serialized.contains("9007199254740991.0"));
}

#[cfg(feature = "codegen")]
#[test]
fn committed_package_and_graph_schemas_match_rust_projection() {
    for (committed, actual) in [
        (
            include_str!("../schemas/application-package-v1.schema.json"),
            crate::artifact_schemas::application_package_v1(),
        ),
        (
            include_str!("../schemas/runtime-artifact-graph-v3.schema.json"),
            crate::artifact_schemas::runtime_artifact_graph_v3(),
        ),
    ] {
        assert_eq!(serde_json::from_str::<Value>(committed).unwrap(), actual);
    }
}

#[cfg(feature = "codegen")]
#[test]
fn layer_schema_field_names_match_serialized_wire_variants() {
    let mut value = graph();
    value["layers"][0]["rootPath"] = json!("public");
    value["layers"][1]["runtimePolicyLineage"] = json!("NODE:24");
    let wire: RuntimeArtifactGraphV3Wire = serde_json::from_value(value).unwrap();
    let value = serde_json::to_value(wire).unwrap();
    let schema = crate::artifact_schemas::runtime_artifact_graph_v3();
    let variants = schema["definitions"]["Layer"]["oneOf"].as_array().unwrap();
    for layer in value["layers"].as_array().unwrap() {
        let variant = variants
            .iter()
            .find(|variant| variant["properties"]["target"]["enum"][0] == layer["target"])
            .unwrap();
        let wire_fields: std::collections::BTreeSet<_> =
            layer.as_object().unwrap().keys().collect();
        let schema_fields: std::collections::BTreeSet<_> =
            variant["properties"].as_object().unwrap().keys().collect();
        assert_eq!(wire_fields, schema_fields);
        for required in variant["required"].as_array().unwrap() {
            assert!(layer.get(required.as_str().unwrap()).is_some());
        }
    }
}

#[cfg(feature = "codegen")]
fn sort_required(schema: &mut Value) {
    match schema {
        Value::Object(fields) => {
            for (name, value) in fields {
                if name == "required" {
                    value
                        .as_array_mut()
                        .unwrap()
                        .sort_by(|left, right| left.as_str().unwrap().cmp(right.as_str().unwrap()));
                } else {
                    sort_required(value);
                }
            }
        }
        Value::Array(values) => values.iter_mut().for_each(sort_required),
        _ => {}
    }
}
