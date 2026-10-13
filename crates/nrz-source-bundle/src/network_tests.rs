use super::*;
use serde_json::json;
fn private_network() -> serde_json::Value {
    json!({"schemaVersion":"COMPUTE_NETWORK_V1","exposure":"PRIVATE","ports":[{"name":"redis","protocol":"tcp","port":6379,"binding":"fixed"}],"readiness":{"protocol":"tcp","port":"redis"},"allowFrom":[]})
}
// Legacy aggregate readiness rejected mixed endpoints and never checked private route exposure.
#[test]
fn private_only_and_mixed_readiness_preserve_per_layer_policy() {
    let mut public = private_network();
    public["exposure"] = json!("PUBLIC");
    public["ports"][0] = json!({"name":"http","protocol":"http","port":3000,"binding":"primary"});
    public["readiness"] = json!({"protocol":"http","port":"http","path":"/health"});
    let manifest: SourceLogicalManifest = serde_json::from_value(json!({
            "schemaVersion":"SOURCE_BUNDLE_V1.0","capabilities":[COMPUTE_NETWORK_SCHEMA_VERSION],"files":[],"entrypoints":[],
            "layers":[{"name":"web","target":"COMPUTE","runtimeConfig":{"network":public}},{"name":"cache","target":"COMPUTE","runtimeConfig":{"network":private_network()}}],
            "routes":[{"pattern":"^/.*$","layerName":"web"}]
        })).unwrap();
    assert!(validate_source_network(&manifest).unwrap());
    assert!(
        crate::source_runtime_readiness(&manifest)
            .unwrap()
            .is_none()
    );
    let mut private_only = manifest.clone();
    private_only.layers.remove(0);
    private_only.routes.clear();
    assert!(validate_source_network(&private_only).unwrap());
    private_only.routes.push(crate::SourceLogicalManifestRoute {
        pattern: "^/.*$".into(),
        layer_name: "cache".into(),
        priority: None,
        methods: None,
        fallthrough_when: None,
        headers: None,
    });
    assert!(validate_source_network(&private_only).is_err());
}
#[test]
fn invalid_ports_unknown_fields_and_incomplete_capability_fail_closed() {
    let mut network = private_network();
    network["readiness"]["port"] = json!("missing");
    assert!(layer_compute_network(Some(&json!({"network":network}))).is_err());
    let mut network = private_network();
    network["allowUnknown"] = json!(true);
    assert!(layer_compute_network(Some(&json!({"network":network}))).is_err());
    let manifest: SourceLogicalManifest = serde_json::from_value(json!({"schemaVersion":"SOURCE_BUNDLE_V1.0","capabilities":[COMPUTE_NETWORK_SCHEMA_VERSION],"files":[],"entrypoints":[],"layers":[{"name":"server","target":"COMPUTE"}],"routes":[]})).unwrap();
    assert!(validate_source_network(&manifest).is_err());
}
#[test]
fn caller_identity_rejects_nil_controls_and_uuid_case_duplicates() {
    let id = "019b8952-ca22-76f0-b134-88becec7c630";
    for callers in [
        json!([{"serviceId":"00000000-0000-0000-0000-000000000000","layerName":"api"}]),
        json!([{"serviceId":id,"layerName":"api\n"}]),
        json!([{"serviceId":id,"layerName":"api"},{"serviceId":id.to_uppercase(),"layerName":"api"}]),
    ] {
        let mut network = private_network();
        network["allowFrom"] = callers;
        assert!(layer_compute_network(Some(&json!({"network":network}))).is_err());
    }
    let mut network = private_network();
    network["allowFrom"] = json!([{"serviceId":id,"layerName":"слой"}]);
    assert!(layer_compute_network(Some(&json!({"network":network}))).is_ok());
}

#[test]
fn modern_layer_names_preserve_legacy_readers_and_reject_control_identity() {
    let mut manifest: SourceLogicalManifest = serde_json::from_value(json!({
        "schemaVersion":"SOURCE_BUNDLE_V1.0","capabilities":[COMPUTE_NETWORK_SCHEMA_VERSION],"files":[],"entrypoints":[],
        "layers":[{"name":"😀".repeat(40),"target":"COMPUTE","runtimeConfig":{"network":private_network()}}],"routes":[]
    })).unwrap();
    assert!(validate_source_network(&manifest).unwrap());
    manifest.layers[0].name = "api\n".into();
    assert!(validate_source_network(&manifest).is_err());
    manifest.capabilities.clear();
    manifest.layers[0].runtime_config = None;
    assert!(!validate_source_network(&manifest).unwrap());
}
