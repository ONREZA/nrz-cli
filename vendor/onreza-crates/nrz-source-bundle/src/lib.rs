// @generated vendored copy of platform crates/nrz-source-bundle/src/lib.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

//! Shared SOURCE_BUNDLE_V1 manifest and archive verification logic.

pub mod dependency;
pub mod handoff;
pub mod manifest;
pub mod verifier;

pub use dependency::{
    DependencySourceTree, DependencySourceTreeError, DependencySourceTreeSpec,
    PYTHON_314_SITE_PACKAGES_ROOT, dependency_source_tree_specs, extract_dependency_source_trees,
};
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
pub use verifier::{
    SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH, SourceBundleVerificationFailure,
    SourceBundleVerificationInput, SourceBundleVerificationResult, SourceBundleVerificationSummary,
    verify_source_bundle_bytes, verify_source_bundle_stream,
};
