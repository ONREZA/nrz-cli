//! Canonical SOURCE_BUNDLE_V2 manifest metadata; no archive or file-content proof.

use nrz_contract::{SourceBundleV2ManifestWire, artifact_json_v1};
use serde_json::Value;

#[path = "source_bundle_v2_admission.rs"]
pub mod admission;
#[path = "source_bundle_v2_memory.rs"]
mod memory;
#[path = "source_bundle_v2_preflight.rs"]
mod preflight;
#[path = "source_bundle_v2_static.rs"]
pub mod static_projection;
#[path = "source_bundle_v2_validation.rs"]
mod validation;
pub use validation::valid_runtime_policy_lineage;

pub const SOURCE_BUNDLE_V2_SCHEMA_VERSION: &str = "SOURCE_BUNDLE_V2.0";
pub const SOURCE_BUNDLE_V2_MEDIA_TYPE: &str =
    "application/vnd.onreza.source-bundle.manifest.v2+json";
pub const SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES: usize = 512 * 1024 * 1024;
pub(crate) const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Caller-owned limits; manifest claims never establish admission authority.
#[derive(Debug, Clone, Copy)]
pub struct SourceBundleV2ManifestBudget {
    pub max_file_count: u64,
    pub max_logical_bytes: u64,
    pub max_static_file_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceBundleV2ManifestSummary {
    pub file_count: u64,
    pub logical_static_bytes: u64,
    pub artifact_size_bytes: u64,
    pub max_static_file_size_bytes: u64,
}

/// Immutable metadata proof produced only by the manifest verifiers.
///
/// Callers cannot construct a proof from unchecked claims:
/// ```compile_fail,E0451
/// use nrz_contract::SourceBundleV2ManifestWire;
/// use nrz_source_bundle::source_bundle_v2::{
///     SourceBundleV2ManifestSummary, VerifiedSourceBundleV2Manifest,
/// };
/// fn forge(wire: SourceBundleV2ManifestWire, summary: SourceBundleV2ManifestSummary)
///     -> VerifiedSourceBundleV2Manifest
/// {
///     VerifiedSourceBundleV2Manifest {
///         wire, summary, manifest_blake3: String::new(), manifest_size_bytes: 0,
///     }
/// }
/// ```
/// Its claims cannot be replaced independently:
/// ```compile_fail,E0616
/// use nrz_source_bundle::source_bundle_v2::VerifiedSourceBundleV2Manifest;
/// fn mutate(verified: &mut VerifiedSourceBundleV2Manifest) {
///     verified.wire.files.clear();
/// }
/// ```
/// ```compile_fail,E0616
/// use nrz_source_bundle::source_bundle_v2::VerifiedSourceBundleV2Manifest;
/// fn mutate(verified: &mut VerifiedSourceBundleV2Manifest) {
///     verified.summary.file_count = 0;
/// }
/// ```
/// ```compile_fail,E0616
/// use nrz_source_bundle::source_bundle_v2::VerifiedSourceBundleV2Manifest;
/// fn mutate(verified: &mut VerifiedSourceBundleV2Manifest) {
///     verified.manifest_blake3.clear();
/// }
/// ```
#[derive(Debug)]
pub struct VerifiedSourceBundleV2Manifest {
    wire: SourceBundleV2ManifestWire,
    summary: SourceBundleV2ManifestSummary,
    manifest_blake3: String,
    manifest_size_bytes: u64,
}

impl VerifiedSourceBundleV2Manifest {
    /// Borrow the verified wire for this proof's lifetime. An explicit clone is
    /// unchecked data and must be verified again after editing.
    /// ```compile_fail,E0596
    /// use nrz_source_bundle::source_bundle_v2::VerifiedSourceBundleV2Manifest;
    /// fn mutate(verified: &mut VerifiedSourceBundleV2Manifest) {
    ///     verified.wire().files.clear();
    /// }
    /// ```
    #[must_use]
    pub fn wire(&self) -> &SourceBundleV2ManifestWire {
        &self.wire
    }

    /// Return a detached summary snapshot; editing it cannot alter this proof.
    #[must_use]
    pub fn summary(&self) -> SourceBundleV2ManifestSummary {
        self.summary
    }

    /// Lowercase BLAKE3 of the exact canonical manifest bytes, never file bytes.
    #[must_use]
    pub fn manifest_blake3(&self) -> &str {
        &self.manifest_blake3
    }

    #[must_use]
    pub fn manifest_size_bytes(&self) -> u64 {
        self.manifest_size_bytes
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourceBundleV2ManifestError {
    #[error(transparent)]
    Canonical(#[from] artifact_json_v1::Error),
    #[error("invalid SOURCE_BUNDLE_V2 manifest shape: {0}")]
    Shape(#[from] serde_json::Error),
    #[error("invalid SOURCE_BUNDLE_V2 manifest: {0}")]
    Invalid(String),
}

pub fn verify_source_bundle_v2_manifest(
    value: Value,
    budget: SourceBundleV2ManifestBudget,
) -> Result<VerifiedSourceBundleV2Manifest, SourceBundleV2ManifestError> {
    encode_verified_source_bundle_v2_manifest(value, budget).map(|(_, verified)| verified)
}

pub fn encode_source_bundle_v2_manifest(
    value: Value,
    budget: SourceBundleV2ManifestBudget,
) -> Result<Vec<u8>, SourceBundleV2ManifestError> {
    let (bytes, _, _) = encode_validated_value(value, budget)?;
    Ok(bytes)
}

/// Encode canonical producer bytes and retain their immutable metadata proof.
pub fn encode_verified_source_bundle_v2_manifest(
    value: Value,
    budget: SourceBundleV2ManifestBudget,
) -> Result<(Vec<u8>, VerifiedSourceBundleV2Manifest), SourceBundleV2ManifestError> {
    let (bytes, wire, summary) = encode_validated_value(value, budget)?;
    let verified = VerifiedSourceBundleV2Manifest {
        wire,
        summary,
        manifest_blake3: blake3::hash(&bytes).to_hex().to_string(),
        manifest_size_bytes: bytes.len() as u64,
    };
    Ok((bytes, verified))
}

pub fn decode_source_bundle_v2_manifest(
    bytes: &[u8],
    budget: SourceBundleV2ManifestBudget,
) -> Result<VerifiedSourceBundleV2Manifest, SourceBundleV2ManifestError> {
    let value = artifact_json_v1::decode(bytes, limits())?;
    verify_value(value, bytes, budget)
}

fn decode_with_work_budget(
    bytes: &[u8],
    budget: SourceBundleV2ManifestBudget,
    work: &mut crate::path_graph::PathWorkBudget,
) -> Result<VerifiedSourceBundleV2Manifest, validation::ValidationError> {
    let value =
        artifact_json_v1::decode(bytes, limits()).map_err(SourceBundleV2ManifestError::from)?;
    let wire = serde_json::from_value(value)?;
    let summary = validation::validate_with_work(&wire, budget, work)?;
    Ok(VerifiedSourceBundleV2Manifest {
        wire,
        summary,
        manifest_blake3: blake3::hash(bytes).to_hex().to_string(),
        manifest_size_bytes: bytes.len() as u64,
    })
}

fn verify_value(
    value: Value,
    canonical_bytes: &[u8],
    budget: SourceBundleV2ManifestBudget,
) -> Result<VerifiedSourceBundleV2Manifest, SourceBundleV2ManifestError> {
    let (wire, summary) = validate_value(value, budget)?;
    Ok(VerifiedSourceBundleV2Manifest {
        wire,
        summary,
        manifest_blake3: blake3::hash(canonical_bytes).to_hex().to_string(),
        manifest_size_bytes: canonical_bytes.len() as u64,
    })
}

fn validate_value(
    value: Value,
    budget: SourceBundleV2ManifestBudget,
) -> Result<(SourceBundleV2ManifestWire, SourceBundleV2ManifestSummary), SourceBundleV2ManifestError>
{
    let wire = serde_json::from_value(value)?;
    let summary = validation::validate(&wire, budget)?;
    Ok((wire, summary))
}

fn encode_validated_value(
    value: Value,
    budget: SourceBundleV2ManifestBudget,
) -> Result<
    (
        Vec<u8>,
        SourceBundleV2ManifestWire,
        SourceBundleV2ManifestSummary,
    ),
    SourceBundleV2ManifestError,
> {
    let bytes = artifact_json_v1::encode(&value, limits())?;
    let value = normalize_producer_numbers(value, &bytes)?;
    let (wire, summary) = validate_value(value, budget)?;
    Ok((bytes, wire, summary))
}

fn normalize_producer_numbers(
    mut value: Value,
    canonical_bytes: &[u8],
) -> Result<Value, SourceBundleV2ManifestError> {
    if has_float(&value) {
        // The byte profile normalizes safe integral floats before schema decoding.
        drop(value);
        value = artifact_json_v1::decode(canonical_bytes, limits())?;
    }
    Ok(value)
}

fn has_float(value: &Value) -> bool {
    match value {
        Value::Number(number) => number.is_f64(),
        Value::Array(values) => values.iter().any(has_float),
        Value::Object(values) => values.values().any(has_float),
        _ => false,
    }
}

fn limits() -> artifact_json_v1::Limits {
    artifact_json_v1::Limits {
        max_bytes: SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES,
        max_depth: artifact_json_v1::MAX_SUPPORTED_DEPTH,
    }
}

#[cfg(test)]
#[path = "source_bundle_v2_tests.rs"]
mod tests;

/// Validate bounded source runtime metadata without rebuilding file inventory.
/// This does not prove file ownership or source contents; the full manifest
/// verifier and the exact authenticated manifest identity remain necessary.
pub fn validate_source_bundle_v2_runtime_metadata(
    layers: &[nrz_contract::source_bundle_v2::Layer],
    routes: &[nrz_contract::source_bundle_v2::Route],
    capabilities: &[nrz_contract::source_bundle_v2::Capability],
    entrypoints: &[nrz_contract::source_bundle_v2::NonEmptyString],
) -> Result<(), SourceBundleV2ManifestError> {
    validation::validate_metadata(layers, routes, capabilities, entrypoints)
}
