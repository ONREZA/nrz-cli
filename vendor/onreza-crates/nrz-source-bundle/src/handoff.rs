// @generated vendored copy of platform crates/nrz-source-bundle/src/handoff.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

#![allow(dead_code, unused, clippy::all)]
use serde::{Deserialize, Serialize};

pub const EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION: &str = "EDGE_BUILD_HANDOFF_V1.0";
pub const EDGE_BUILD_HANDOFF_V1_FILE: &str = "edge-build-handoff-v1.json";
pub const EDGE_BUILD_SOURCE_BUNDLE_V1_FILE: &str = "source-bundle-v1.tar.zst";
pub const SOURCE_BUNDLE_V1_MEDIA_TYPE: &str = "application/vnd.onreza.source-bundle.tar+zstd.v1";

const SHA256_HEX_LENGTH: usize = 64;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Untrusted sandbox output consumed by the Builder Agent.
///
/// The fixed relative file name deliberately prevents the producer from
/// selecting an arbitrary host path. Consumers must still open the file below
/// their trusted workspace root without following symbolic links and verify
/// every declared digest before publishing local artifact state.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct EdgeBuildHandoffV1 {
    pub schema_version: String,
    pub source_bundle: EdgeBuildSourceBundleV1,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct EdgeBuildSourceBundleV1 {
    pub path: String,
    pub media_type: String,
    pub schema_version: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub logical_manifest_sha256: String,
}

impl EdgeBuildHandoffV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION {
            return Err(format!(
                "unsupported Edge build handoff schemaVersion: {}",
                self.schema_version
            ));
        }
        self.source_bundle.validate()
    }
}

impl EdgeBuildSourceBundleV1 {
    fn validate(&self) -> Result<(), String> {
        if self.path != EDGE_BUILD_SOURCE_BUNDLE_V1_FILE {
            return Err("Edge build handoff source bundle path is not canonical".to_string());
        }
        if self.media_type != SOURCE_BUNDLE_V1_MEDIA_TYPE {
            return Err("Edge build handoff source bundle mediaType is unsupported".to_string());
        }
        if self.schema_version != crate::manifest::SOURCE_BUNDLE_V1_SCHEMA_VERSION {
            return Err(format!(
                "unsupported SOURCE_BUNDLE schemaVersion: {}",
                self.schema_version
            ));
        }
        validate_sha256("source bundle sha256", &self.sha256)?;
        validate_sha256(
            "source bundle logicalManifestSha256",
            &self.logical_manifest_sha256,
        )?;
        if self.size_bytes == 0 || self.size_bytes > MAX_SAFE_INTEGER {
            return Err("Edge build handoff source bundle sizeBytes is invalid".to_string());
        }
        Ok(())
    }
}

fn validate_sha256(label: &str, value: &str) -> Result<(), String> {
    if value.len() != SHA256_HEX_LENGTH
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "Edge build handoff {label} must be a lowercase SHA-256 digest"
        ));
    }
    Ok(())
}
