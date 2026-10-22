use crate::SourceBundleV2ManifestWire;
use serde_json::{Value, json};

fn manifest() -> Value {
    json!({
        "schemaVersion": "SOURCE_BUNDLE_V2.0", "capabilities": [],
        "files": [], "layers": [], "routes": [], "entrypoints": []
    })
}

#[test]
fn wire_retains_present_empty_collections_and_uuid_spelling() {
    let mut value = manifest();
    value["routes"] = json!([{
        "pattern": "/", "layerName": "public", "headers": {}, "methods": []
    }]);
    value["layers"] = json!([{
        "name": "compute", "target": "COMPUTE", "runtimeConfig": {
            "applicationRuntime": {"family": "BUN", "args": []},
            "network": {
                "schemaVersion": "COMPUTE_NETWORK_V1", "exposure": "PUBLIC",
                "ports": [{"name": "http", "port": 8080, "protocol": "http", "binding": "primary"}],
                "readiness": {"protocol": "tcp", "port": "http"},
                "allowFrom": [{"serviceId": "ABAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA", "layerName": "other"}]
            }
        }
    }]);
    let wire: SourceBundleV2ManifestWire = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(wire).unwrap(), value);

    value["routes"][0]
        .as_object_mut()
        .unwrap()
        .remove("methods");
    value["routes"][0]
        .as_object_mut()
        .unwrap()
        .remove("headers");
    let wire: SourceBundleV2ManifestWire = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(wire).unwrap(), value);
}

#[test]
fn wire_rejects_removed_legacy_runtime_fields() {
    for (field, legacy) in [
        ("runtimeFamily", json!("JAVASCRIPT")),
        ("isBinaryEntry", json!(false)),
        ("readiness", json!({"protocol": "TCP"})),
        ("maxConcurrency", json!(1)),
    ] {
        let mut value = manifest();
        value["layers"] = json!([{
            "name": "api", "target": "COMPUTE", "runtimeConfig": {
                "applicationRuntime": {"family": "BUN", "args": []},
                "network": {
                    "schemaVersion": "COMPUTE_NETWORK_V1", "exposure": "PUBLIC",
                    "ports": [{"name": "http", "port": 3000, "protocol": "http", "binding": "primary"}],
                    "readiness": {"protocol": "tcp", "port": "http"}, "allowFrom": []
                }
            }
        }]);
        value["layers"][0]["runtimeConfig"][field] = legacy;
        assert!(
            serde_json::from_value::<SourceBundleV2ManifestWire>(value).is_err(),
            "{field}"
        );
    }
}

#[test]
fn wire_requires_fields_and_rejects_unknowns_and_null_options() {
    for field in [
        "schemaVersion",
        "capabilities",
        "files",
        "layers",
        "routes",
        "entrypoints",
    ] {
        let mut value = manifest();
        value.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<SourceBundleV2ManifestWire>(value).is_err(),
            "{field}"
        );
    }
    let mut value = manifest();
    value["unknown"] = json!(true);
    assert!(serde_json::from_value::<SourceBundleV2ManifestWire>(value).is_err());
    let mut value = manifest();
    value["routes"] = json!([{"pattern": "/", "layerName": "public", "headers": null}]);
    assert!(serde_json::from_value::<SourceBundleV2ManifestWire>(value).is_err());
}

#[cfg(feature = "codegen")]
#[test]
fn committed_source_schema_matches_rust_projection() {
    let committed: Value =
        serde_json::from_str(include_str!("../schemas/source-bundle-v2.schema.json")).unwrap();
    assert_eq!(crate::artifact_schemas::source_bundle_v2(), committed);
}
