//! Shared SOURCE_BUNDLE_V1 manifest canonicalization for nrz-cli binary builds.

pub mod handoff;
pub mod manifest;

pub use handoff::{
    EDGE_BUILD_HANDOFF_V1_FILE, EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION,
    EDGE_BUILD_SOURCE_BUNDLE_V1_FILE, EdgeBuildHandoffV1, EdgeBuildSourceBundleV1,
    SOURCE_BUNDLE_V1_MEDIA_TYPE,
};
pub use manifest::{
    RouteFallthroughCondition, SOURCE_BUNDLE_V1_SCHEMA_VERSION, SourceBundleSummary,
    SourceLogicalManifest, SourceLogicalManifestEntryType, SourceLogicalManifestFile,
    SourceLogicalManifestLayer, SourceLogicalManifestRoute, canonical_source_logical_manifest_json,
    compute_logical_manifest_sha256, compute_source_artifact_id, normalize_source_path, sha256_hex,
    summarize_logical_manifest,
};
