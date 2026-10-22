//! Public artifact schema projection from the Rust wire model.

use schemars08::{r#gen::SchemaSettings, schema::Schema};
use serde_json::json;

pub fn source_bundle_v2() -> serde_json::Value {
    artifact_schema::<crate::SourceBundleV2ManifestWire>(
        "source-bundle-v2",
        "Canonical source manifest metadata. Structural verification and trusted budgets are required in addition to this schema; file contents and publication authority are verified separately.",
    )
}

pub fn application_package_v1() -> serde_json::Value {
    artifact_schema::<crate::ApplicationPackageV1DescriptorWire>(
        "application-package-v1",
        "Application package descriptor. Exact bytes, checked geometry, allowlisted profiles and publication authority require independent domain verification.",
    )
}

pub fn runtime_artifact_graph_v3() -> serde_json::Value {
    artifact_schema::<crate::RuntimeArtifactGraphV3Wire>(
        "runtime-artifact-graph-v3",
        "Bounded immutable application and launch projection. Routing and execution authority remain release-owned; canonical byte identity and publication binding require independent verification.",
    )
}

fn artifact_schema<T: schemars08::JsonSchema>(name: &str, description: &str) -> serde_json::Value {
    let mut settings = SchemaSettings::draft2019_09();
    settings.meta_schema = Some("https://json-schema.org/draft/2020-12/schema".into());
    settings.option_add_null_type = false;
    let mut schema = serde_json::to_value(settings.into_generator().into_root_schema_for::<T>())
        .expect("wire schema is JSON");
    schema["$id"] = json!(format!("https://docs.onreza.ru/schemas/{name}.schema.json"));
    schema["description"] = json!(description);
    integer_schema_bounds(&mut schema);
    schema
}

fn legacy_graph_schema(pointer: &str) -> Schema {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schemas/runtime-artifact-graph-v2.schema.json"
    ))
    .expect("checked-in legacy runtime graph schema");
    serde_json::from_value(
        schema
            .pointer(pointer)
            .expect("legacy schema subtree")
            .clone(),
    )
    .expect("legacy artifact schema")
}

pub(crate) fn legacy_dependency(_: &mut schemars08::r#gen::SchemaGenerator) -> Schema {
    legacy_graph_schema("/properties/runtimeArtifactGraph/properties/dependencies/items")
}

pub(crate) fn dependency_mounts(_: &mut schemars08::r#gen::SchemaGenerator) -> Schema {
    legacy_graph_schema(
        "/properties/runtimeArtifactGraph/properties/runtimeLayers/items/properties/dependencyBindings/properties/mounts",
    )
}

pub(crate) fn runtime_profile(_: &mut schemars08::r#gen::SchemaGenerator) -> Schema {
    legacy_graph_schema(
        "/properties/runtimeArtifactGraph/properties/runtimeLayers/items/properties/launch/properties/profile",
    )
}

pub(crate) fn runtime_arguments(generator: &mut schemars08::r#gen::SchemaGenerator) -> Schema {
    serde_json::from_value(json!({
        "type": "array", "maxItems": 64,
        "items": generator.subschema_for::<crate::source_bundle_v2::RuntimeArgument>()
    }))
    .expect("runtime argument schema")
}

// Schemars stores numeric bounds as f64; emit exact JSON integers so max-safe
// bounds survive parsers that do not enable floating-point roundtrips.
fn integer_schema_bounds(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                if matches!(
                    key.as_str(),
                    "minimum" | "maximum" | "exclusiveMinimum" | "exclusiveMaximum"
                ) {
                    if let Some(number) = value.as_f64() {
                        assert!(
                            number.fract() == 0.0 && number.abs() <= 9_007_199_254_740_991.0,
                            "artifact schema bounds must be safe integers"
                        );
                        *value = json!(number as i64);
                    }
                } else {
                    integer_schema_bounds(value);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                integer_schema_bounds(value);
            }
        }
        _ => {}
    }
}

pub(crate) fn service_id(_: &mut schemars08::r#gen::SchemaGenerator) -> Schema {
    serde_json::from_value(json!({
        "type": "string",
        "format": "uuid",
        "pattern": "^(?!00000000-0000-0000-0000-000000000000$)"
    }))
    .expect("service ID schema")
}

pub(crate) fn headers(_: &mut schemars08::r#gen::SchemaGenerator) -> Schema {
    serde_json::from_value(json!({
        "type": "object",
        "propertyNames": {
            "type": "string", "minLength": 1, "maxLength": 256,
            "pattern": "^[!#$%&'*+\\-.^`|~0-9A-Za-z]+$"
        },
        "additionalProperties": {"type": "string", "maxLength": 8192},
        "maxProperties": 128
    }))
    .expect("route header schema")
}
