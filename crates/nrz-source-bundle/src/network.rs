use crate::SourceLogicalManifest;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const COMPUTE_NETWORK_SCHEMA_VERSION: &str = "COMPUTE_NETWORK_V1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComputeNetwork {
    pub schema_version: String,
    pub exposure: String,
    pub ports: Vec<ComputePort>,
    pub readiness: ComputeEndpointReadiness,
    pub allow_from: Vec<ComputeCaller>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputePort {
    pub name: String,
    pub protocol: String,
    pub port: u16,
    pub binding: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputeEndpointReadiness {
    pub protocol: String,
    pub port: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComputeCaller {
    pub service_id: String,
    pub layer_name: String,
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.as_bytes()[0].is_ascii_alphanumeric()
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
fn valid_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}
impl ComputeNetwork {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != COMPUTE_NETWORK_SCHEMA_VERSION
            || !["PUBLIC", "PRIVATE"].contains(&self.exposure.as_str())
        {
            return Err("unsupported endpoint network declaration".into());
        }
        if self.ports.is_empty() || self.ports.len() > 16 || self.allow_from.len() > 128 {
            return Err("endpoint ports/callers exceed bounds".into());
        }
        let mut names = HashSet::new();
        let mut numbers = HashSet::new();
        let mut primary = 0;
        for port in &self.ports {
            if !valid_name(&port.name)
                || port.port == 0
                || !["http", "grpc", "websocket", "tcp"].contains(&port.protocol.as_str())
                || !["primary", "fixed"].contains(&port.binding.as_str())
                || !names.insert(port.name.as_str())
                || !numbers.insert(port.port)
            {
                return Err("invalid or duplicate endpoint port".into());
            }
            primary += usize::from(port.binding == "primary");
        }
        if primary > 1 || !names.contains(self.readiness.port.as_str()) {
            return Err("multiple primary ports or undeclared readiness port".into());
        }
        match self.readiness.protocol.as_str() {
            "tcp" if self.readiness.path.is_none() => {}
            "http"
                if self.readiness.path.as_ref().is_some_and(|path| {
                    path.starts_with('/')
                        && path.len() <= 2048
                        && !path
                            .bytes()
                            .any(|c| c <= 32 || c == 127 || c == b'?' || c == b'#')
                }) => {}
            _ => return Err("invalid endpoint readiness".into()),
        }
        let mut callers = HashSet::new();
        for caller in &self.allow_from {
            if !valid_uuid(&caller.service_id)
                || caller.layer_name.is_empty()
                || caller.layer_name.chars().count() > 64
                || !callers.insert((&caller.service_id, &caller.layer_name))
            {
                return Err("invalid or duplicate endpoint caller".into());
            }
        }
        Ok(())
    }
    pub fn public_default() -> Self {
        Self {
            schema_version: COMPUTE_NETWORK_SCHEMA_VERSION.into(),
            exposure: "PUBLIC".into(),
            ports: vec![ComputePort {
                name: "http".into(),
                protocol: "http".into(),
                port: 3000,
                binding: "primary".into(),
            }],
            readiness: ComputeEndpointReadiness {
                protocol: "tcp".into(),
                port: "http".into(),
                path: None,
            },
            allow_from: vec![],
        }
    }
}

pub fn layer_compute_network(
    config: Option<&serde_json::Value>,
) -> Result<Option<ComputeNetwork>, String> {
    config
        .and_then(|config| config.get("network"))
        .map(|value| {
            let network: ComputeNetwork = serde_json::from_value(value.clone())
                .map_err(|e| format!("invalid endpoint network: {e}"))?;
            network.validate()?;
            Ok(network)
        })
        .transpose()
}

/// Retained V1 has no network capability. New writers require a complete declaration.
pub fn validate_source_network(manifest: &SourceLogicalManifest) -> Result<bool, String> {
    let declared = manifest
        .capabilities
        .iter()
        .any(|c| c == COMPUTE_NETWORK_SCHEMA_VERSION);
    let mut names = HashSet::new();
    for layer in &manifest.layers {
        if !names.insert(layer.name.as_str()) {
            return Err("duplicate source layer".into());
        }
        let network = layer_compute_network(layer.runtime_config.as_ref())?;
        if network.is_some() && (!declared || layer.target != "COMPUTE") {
            return Err("network declaration requires COMPUTE network capability".into());
        }
        if declared && layer.target == "COMPUTE" && network.is_none() {
            return Err("network declarations must cover every COMPUTE layer".into());
        }
        if let Some(network) = network {
            if network.exposure == "PUBLIC"
                && !manifest
                    .routes
                    .iter()
                    .any(|route| route.layer_name == layer.name)
            {
                return Err("PUBLIC endpoint requires an explicit public route".into());
            }
            if manifest
                .routes
                .iter()
                .any(|route| route.layer_name == layer.name)
                && (network.exposure == "PRIVATE"
                    || !network.ports.iter().any(|port| {
                        port.binding == "primary"
                            && ["http", "grpc", "websocket"].contains(&port.protocol.as_str())
                    }))
            {
                return Err("public route targets PRIVATE or non-HTTP endpoint".into());
            }
        }
    }
    if declared
        && manifest.routes.is_empty()
        && manifest.layers.iter().any(|layer| {
            layer.target != "COMPUTE"
                || !layer_compute_network(layer.runtime_config.as_ref())
                    .ok()
                    .flatten()
                    .is_some_and(|network| network.exposure == "PRIVATE")
        })
    {
        return Err("empty route set requires private-only COMPUTE".into());
    }
    Ok(declared)
}

#[cfg(test)]
mod tests {
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
        public["ports"][0] =
            json!({"name":"http","protocol":"http","port":3000,"binding":"primary"});
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
}
