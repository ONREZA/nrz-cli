//! Shared SOURCE_BUNDLE_V1 manifest and archive verification logic.

pub mod application_runtime;
pub mod dependency;
pub mod handoff;
pub mod manifest;
mod path_graph;
mod pax;
pub mod python;
pub use python::{
    PythonMinor, PythonToolchainVersions, python_toolchain_versions,
    validate_runtime_version_declaration,
};
pub mod source_build_context;
pub mod source_bundle_v2;
pub mod verifier;

pub use path_graph::{PathWorkBudget, PathWorkError, SourceArchivePathIndex, SourcePathGraphError};
pub use pax::encode_pax_record;

pub use source_build_context::{
    BuildToolchainDeclaration, BuildToolchainFamily, SOURCE_BUILD_CONTEXT_SCHEMA_VERSION,
    SourceBuildContext,
};

pub use application_runtime::{
    APPLICATION_RUNTIME_CONFIG_KEY, ApplicationRuntimeDeclaration, ApplicationRuntimeFamily,
    ApplicationRuntimeIntent, layer_application_runtime, source_application_runtime,
    validate_build_runtime_version,
};

pub use dependency::{
    DependencySourceTree, DependencySourceTreeError, DependencySourceTreeSpec,
    PYTHON_314_SITE_PACKAGES_ROOT, dependency_source_tree_specs, extract_dependency_source_trees,
};
pub use handoff::{
    EDGE_BUILD_HANDOFF_V1_FILE, EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION, EDGE_BUILD_HANDOFF_V2_FILE,
    EDGE_BUILD_HANDOFF_V2_SCHEMA_VERSION, EDGE_BUILD_SOURCE_BUNDLE_V1_FILE,
    EDGE_BUILD_SOURCE_MANIFEST_V2_FILE, EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY, EdgeBuildBindingV2,
    EdgeBuildHandoffV1, EdgeBuildHandoffV2, EdgeBuildSourceBundleV1, EdgeBuildSourceManifestV2,
    EdgeBuildSourceTreeV2, SOURCE_BUNDLE_V1_MEDIA_TYPE,
};
pub use manifest::{
    RUNTIME_READINESS_CONFIG_KEY, RouteFallthroughCondition, SOURCE_BUNDLE_V1_SCHEMA_VERSION,
    SourceBundleSummary, SourceLogicalManifest, SourceLogicalManifestEntryType,
    SourceLogicalManifestFile, SourceLogicalManifestLayer, SourceLogicalManifestRoute,
    SourceRuntimeReadiness, canonical_source_logical_manifest_json,
    compute_logical_manifest_sha256, compute_source_artifact_id, normalize_source_path, sha256_hex,
    source_runtime_readiness, summarize_logical_manifest, validate_source_route_headers,
};
pub use verifier::{
    SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH, SOURCE_BUNDLE_VERIFIER_FIXED_MEMORY_BYTES,
    SourceBundleVerificationBudget, SourceBundleVerificationFailure, SourceBundleVerificationInput,
    SourceBundleVerificationMemory, SourceBundleVerificationResult,
    SourceBundleVerificationSummary, source_bundle_verification_manifest_memory_bytes,
    source_bundle_verifier_max_decoder_memory_bytes, verify_source_bundle_bytes,
    verify_source_bundle_stream, verify_source_bundle_stream_with_admission,
};

mod network;
pub use network::{
    COMPUTE_NETWORK_SCHEMA_VERSION, ComputeCaller, ComputeEndpointReadiness, ComputeNetwork,
    ComputePort, layer_compute_network, validate_source_network,
};
