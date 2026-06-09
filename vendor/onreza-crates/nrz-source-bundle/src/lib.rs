//! Shared SOURCE_BUNDLE_V1 manifest canonicalization for nrz-cli binary builds.

pub mod manifest;

pub use manifest::{
    SOURCE_BUNDLE_V1_SCHEMA_VERSION, SourceBundleSummary, SourceLogicalManifest,
    SourceLogicalManifestEntryType, SourceLogicalManifestFile, SourceLogicalManifestLayer,
    SourceLogicalManifestRoute, canonical_source_logical_manifest_json,
    compute_logical_manifest_sha256, compute_source_artifact_id, normalize_source_path, sha256_hex,
    summarize_logical_manifest,
};
