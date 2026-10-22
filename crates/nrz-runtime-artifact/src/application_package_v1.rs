//! Package metadata validation only: no content, publication or mount authority proof.

use nrz_contract::application_package_v1::{BlobReference, PackageDescriptor};

use crate::MAX_SAFE_INTEGER;

pub const APPLICATION_PACKAGE_MEDIA_TYPE: &str =
    "application/vnd.onreza.application.erofs-verity.v1";
pub const APPLICATION_VERITY_BLOCK_SIZE: u64 = 4096;
pub const APPLICATION_VERITY_HASHES_PER_BLOCK: u64 = APPLICATION_VERITY_BLOCK_SIZE / 32;

/// Canonical V1 geometry; no byte integrity or custody claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationPackageLayout {
    pub data_block_count: u64,
    pub tree_offset: u64,
    pub tree_length: u64,
    pub package_size: u64,
}

pub fn application_package_layout(
    erofs_size: u64,
) -> Result<ApplicationPackageLayout, PackageError> {
    require(
        erofs_size > 0 && erofs_size <= MAX_SAFE_INTEGER,
        "invalid EROFS size",
    )?;
    let data_block_count = erofs_size.div_ceil(APPLICATION_VERITY_BLOCK_SIZE);
    let tree_offset = data_block_count
        .checked_mul(APPLICATION_VERITY_BLOCK_SIZE)
        .ok_or_else(|| PackageError("data length overflow".into()))?;
    let mut blocks = data_block_count;
    let mut tree_blocks = 0_u64;
    while blocks > 1 {
        blocks = blocks.div_ceil(APPLICATION_VERITY_HASHES_PER_BLOCK);
        tree_blocks = tree_blocks
            .checked_add(blocks)
            .ok_or_else(|| PackageError("verity tree block count overflow".into()))?;
    }
    let tree_length = tree_blocks
        .checked_mul(APPLICATION_VERITY_BLOCK_SIZE)
        .ok_or_else(|| PackageError("verity tree length overflow".into()))?;
    let package_size = tree_offset
        .checked_add(tree_length)
        .ok_or_else(|| PackageError("package size overflow".into()))?;
    require(
        package_size <= MAX_SAFE_INTEGER,
        "package size exceeds safe integer range",
    )?;
    Ok(ApplicationPackageLayout {
        data_block_count,
        tree_offset,
        tree_length,
        package_size,
    })
}

/// Exact caller-owned allowlist entry; this library has no production profile defaults.
#[derive(Debug)]
pub struct ApplicationPackagePolicy {
    pub canonicalization_profile: String,
    pub generator_identity: String,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid application package descriptor: {0}")]
pub struct PackageError(String);

/// Immutable metadata proof, without a claim about package bytes or authorization.
/// ```compile_fail,E0616
/// use nrz_runtime_artifact::application_package_v1::VerifiedApplicationPackageDescriptor;
/// fn mutate(verified: &mut VerifiedApplicationPackageDescriptor) {
///     verified.wire.blob.size = 0;
/// }
/// ```
/// ```compile_fail,E0594
/// use nrz_runtime_artifact::application_package_v1::VerifiedApplicationPackageDescriptor;
/// fn mutate(verified: &mut VerifiedApplicationPackageDescriptor) {
///     verified.wire().blob.size = 0;
/// }
/// ```
#[derive(Debug)]
pub struct VerifiedApplicationPackageDescriptor {
    wire: PackageDescriptor,
}

impl VerifiedApplicationPackageDescriptor {
    #[must_use]
    pub fn wire(&self) -> &PackageDescriptor {
        &self.wire
    }
}

pub fn verify_application_package_descriptor(
    wire: PackageDescriptor,
    policy: &ApplicationPackagePolicy,
) -> Result<VerifiedApplicationPackageDescriptor, PackageError> {
    validate_application_package_descriptor(&wire, policy)?;
    Ok(VerifiedApplicationPackageDescriptor { wire })
}

pub fn validate_application_package_descriptor(
    wire: &PackageDescriptor,
    policy: &ApplicationPackagePolicy,
) -> Result<(), PackageError> {
    validate_blob_reference(&wire.blob)?;
    require(
        wire.blob.media_type == APPLICATION_PACKAGE_MEDIA_TYPE,
        "unsupported package media type",
    )?;
    for (actual, expected) in [
        (
            &wire.canonicalization_profile,
            &policy.canonicalization_profile,
        ),
        (&wire.generator_identity, &policy.generator_identity),
    ] {
        require(
            !actual.is_empty() && actual.chars().count() <= 256 && !actual.contains('\0'),
            "profile and generator identity must contain 1–256 characters without NUL",
        )?;
        require(
            actual == expected,
            "unsupported profile or generator identity",
        )?;
    }

    let verity = &wire.verity;
    require(
        verity.version == 1
            && u64::from(verity.data_block_size) == APPLICATION_VERITY_BLOCK_SIZE
            && u64::from(verity.hash_block_size) == APPLICATION_VERITY_BLOCK_SIZE,
        "V1 requires verity version 1 and 4096-byte data/hash blocks",
    )?;
    require(
        lower_hex_32(&verity.root_hash) && lower_hex_32(&verity.salt),
        "verity root and salt must be 32-byte lowercase hexadecimal values",
    )?;
    for size in [
        wire.erofs_size,
        verity.data_block_count,
        verity.data_offset,
        verity.tree_offset,
        verity.tree_length,
    ] {
        require(
            size <= MAX_SAFE_INTEGER,
            "geometry exceeds safe integer range",
        )?;
    }
    let layout = application_package_layout(wire.erofs_size)?;
    require(verity.data_offset == 0, "V1 data offset must be zero")?;
    require(
        verity.data_block_count == layout.data_block_count,
        "EROFS size does not match canonical data padding",
    )?;
    require(
        verity.tree_offset == layout.tree_offset,
        "tree offset must equal the exact padded data length",
    )?;
    require(
        verity.tree_length == layout.tree_length,
        "verity tree length does not match its geometry",
    )?;
    require(
        wire.blob.size == layout.package_size,
        "package size must equal data plus tree with no trailing bytes",
    )
}

pub(crate) fn validate_blob_reference(blob: &BlobReference) -> Result<(), PackageError> {
    require(
        blob.size <= MAX_SAFE_INTEGER,
        "blob size exceeds safe integer range",
    )?;
    require(
        lower_hex_32(&blob.digest.value),
        "blob digest must be 32-byte lowercase hexadecimal",
    )
}

fn lower_hex_32(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require(condition: bool, message: &str) -> Result<(), PackageError> {
    if condition {
        Ok(())
    } else {
        Err(PackageError(message.into()))
    }
}

#[cfg(test)]
#[path = "application_package_v1_tests.rs"]
mod tests;
