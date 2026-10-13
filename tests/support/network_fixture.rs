//! Explicit network substitution for fresh writer manifests; missing declarations stay missing.

pub fn network() -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": "COMPUTE_NETWORK_V1",
        "exposure": "PUBLIC",
        "ports": [{"name": "http", "protocol": "http", "port": 3000, "binding": "primary"}],
        "readiness": {"protocol": "tcp", "port": "http"},
        "allowFrom": []
    })
}

pub fn manifest(source: &str) -> String {
    source.replace("\"$PUBLIC_NETWORK\"", &network().to_string())
}
