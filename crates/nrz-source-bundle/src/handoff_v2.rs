use serde::{Deserialize, Serialize};

use crate::source_bundle_v2::{
    SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES, SOURCE_BUNDLE_V2_MEDIA_TYPE,
    SOURCE_BUNDLE_V2_SCHEMA_VERSION, SourceBundleV2ManifestSummary,
};

pub const EDGE_BUILD_HANDOFF_V2_SCHEMA_VERSION: &str = "EDGE_BUILD_HANDOFF_V2.0";
pub const EDGE_BUILD_HANDOFF_V2_FILE: &str = "edge-build-handoff-v2.json";
pub const EDGE_BUILD_SOURCE_MANIFEST_V2_FILE: &str = "source-manifest-v2.json";
pub const EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY: &str = "source-tree-v2";

/// Untrusted producer claims. The trusted consumer must compare the complete
/// binding with its retained command and reverify canonical manifest and tree.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EdgeBuildHandoffV2 {
    pub schema_version: String,
    pub binding: EdgeBuildBindingV2,
    pub source_manifest: EdgeBuildSourceManifestV2,
    pub source_tree: EdgeBuildSourceTreeV2,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EdgeBuildBindingV2 {
    pub deployment_id: String,
    pub command_id: String,
    pub command_digest: String,
    /// Existing execution identity, never a newly minted producer identity.
    pub build_attempt_id: String,
    pub attempt: u64,
    pub source_writer_generation: u64,
    pub source_environment_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EdgeBuildSourceManifestV2 {
    pub path: String,
    pub media_type: String,
    pub schema_version: String,
    pub blake3: String,
    pub size_bytes: u64,
    pub summary: SourceBundleV2ManifestSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EdgeBuildSourceTreeV2 {
    pub path: String,
}

impl EdgeBuildBindingV2 {
    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("deploymentId", &self.deployment_id),
            ("commandId", &self.command_id),
            ("buildAttemptId", &self.build_attempt_id),
            ("sourceEnvironmentId", &self.source_environment_id),
        ] {
            if value.len() != 36
                || !value.bytes().enumerate().all(|(index, byte)| {
                    if [8, 13, 18, 23].contains(&index) {
                        byte == b'-'
                    } else {
                        byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
                    }
                })
                || !value
                    .bytes()
                    .any(|byte| (b'1'..=b'9').contains(&byte) || (b'a'..=b'f').contains(&byte))
            {
                return Err(format!(
                    "Edge build handoff {name} must be a canonical nonzero UUID"
                ));
            }
        }
        super::validate_sha256("commandDigest", &self.command_digest)?;
        if self.attempt == 0 || self.source_writer_generation == 0 {
            return Err("Edge build handoff attempt and writer generation must be positive".into());
        }
        Ok(())
    }
}

impl EdgeBuildHandoffV2 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != EDGE_BUILD_HANDOFF_V2_SCHEMA_VERSION {
            return Err("unsupported Edge build V2 handoff schemaVersion".into());
        }
        self.binding.validate()?;
        let manifest = &self.source_manifest;
        if manifest.path != EDGE_BUILD_SOURCE_MANIFEST_V2_FILE
            || self.source_tree.path != EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY
            || manifest.media_type != SOURCE_BUNDLE_V2_MEDIA_TYPE
            || manifest.schema_version != SOURCE_BUNDLE_V2_SCHEMA_VERSION
        {
            return Err("Edge build V2 handoff references are not canonical".into());
        }
        // BLAKE3 uses the same lowercase 256-bit hex encoding as SHA-256.
        if manifest.blake3.len() != 64
            || !manifest
                .blake3
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("Edge build V2 manifest blake3 must be lowercase 256-bit hex".into());
        }
        if manifest.size_bytes == 0
            || manifest.size_bytes > SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES as u64
        {
            return Err("Edge build V2 manifest sizeBytes is invalid".into());
        }
        Ok(())
    }
}
