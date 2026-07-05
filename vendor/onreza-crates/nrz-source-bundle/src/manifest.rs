// @generated vendored copy of platform crates/nrz-source-bundle/src/manifest.rs.
// Do not edit; regenerate via 'bun run sync:nrz-cli-crates <nrz-cli-path>'.

#![allow(dead_code, unused, clippy::all)]
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub const SOURCE_BUNDLE_V1_SCHEMA_VERSION: &str = "SOURCE_BUNDLE_V1.0";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct SourceLogicalManifest {
    pub schema_version: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub files: Vec<SourceLogicalManifestFile>,
    #[serde(default)]
    pub layers: Vec<SourceLogicalManifestLayer>,
    #[serde(default)]
    pub routes: Vec<SourceLogicalManifestRoute>,
    #[serde(default)]
    pub entrypoints: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct SourceLogicalManifestFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "is_source_file_entry_type")]
    pub entry_type: SourceLogicalManifestEntryType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_target: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    pub role: String,
    #[serde(default)]
    pub layer_name: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourceLogicalManifestEntryType {
    #[default]
    File,
    Symlink,
}

fn is_source_file_entry_type(entry_type: &SourceLogicalManifestEntryType) -> bool {
    *entry_type == SourceLogicalManifestEntryType::File
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct SourceLogicalManifestLayer {
    pub name: String,
    pub target: String,
    #[serde(default)]
    pub root_path: Option<String>,
    #[serde(default)]
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub runtime_config: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct SourceLogicalManifestRoute {
    pub pattern: String,
    pub layer_name: String,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub methods: Option<Vec<String>>,
    #[serde(default)]
    pub fallthrough_when: Option<Vec<RouteFallthroughCondition>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub enum RouteFallthroughCondition {
    Header {
        name: String,
        #[serde(default)]
        value: Option<String>,
    },
    Query {
        name: String,
        #[serde(default)]
        value: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceBundleSummary {
    pub file_count: i32,
    pub logical_static_bytes: i64,
    pub artifact_size_bytes: i64,
}

pub fn canonical_source_logical_manifest_json(manifest: &Value) -> String {
    stable_stringify_value(manifest)
}

pub fn compute_logical_manifest_sha256(manifest: &Value) -> String {
    sha256_hex(canonical_source_logical_manifest_json(manifest).as_bytes())
}

pub fn compute_source_artifact_id(
    owner_workspace_id: &str,
    logical_manifest_sha256: &str,
    source_sha256: &str,
    schema_version: Option<&str>,
) -> String {
    let schema_version = schema_version.unwrap_or(SOURCE_BUNDLE_V1_SCHEMA_VERSION);
    sha256_hex(
        format!(
            "{owner_workspace_id}\0{logical_manifest_sha256}\0{source_sha256}\0{schema_version}"
        )
        .as_bytes(),
    )
}

pub fn summarize_logical_manifest(manifest: &SourceLogicalManifest) -> SourceBundleSummary {
    let mut logical_static_bytes = 0_i64;
    let mut artifact_size_bytes = 0_i64;

    for file in &manifest.files {
        let size = i64::try_from(file.size).unwrap_or(i64::MAX);
        if file.role == "static" {
            logical_static_bytes = logical_static_bytes.saturating_add(size);
        } else {
            artifact_size_bytes = artifact_size_bytes.saturating_add(size);
        }
    }

    SourceBundleSummary {
        file_count: i32::try_from(manifest.files.len()).unwrap_or(i32::MAX),
        logical_static_bytes,
        artifact_size_bytes,
    }
}

pub fn normalize_source_path(path: &str) -> Result<String, String> {
    if path.is_empty() || path.starts_with('/') || path.contains('\0') || path.contains('\\') {
        return Err(format!("Invalid archive path: {path}"));
    }
    if path
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!("Unsafe archive path: {path}"));
    }
    Ok(path.to_string())
}

pub fn sha256_hex(input: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_ref());
    hex::encode(hasher.finalize())
}

fn stable_stringify_value(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => value.to_string(),
        Value::Array(items) => {
            let values = items
                .iter()
                .map(stable_stringify_value)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{values}]")
        }
        Value::Object(entries) => stable_stringify_object(entries),
    }
}

fn stable_stringify_object(entries: &Map<String, Value>) -> String {
    let mut keys = entries.keys().collect::<Vec<_>>();
    keys.sort();
    let values = keys
        .into_iter()
        .map(|key| {
            let value = entries
                .get(key)
                .expect("stable stringify key came from object map");
            format!(
                "{}:{}",
                serde_json::to_string(key).unwrap(),
                stable_stringify_value(value)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{values}}}")
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
