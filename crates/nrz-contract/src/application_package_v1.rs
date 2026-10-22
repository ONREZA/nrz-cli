//! Application package metadata; V1 fixes appended, superblock-free verity layout.

use serde::{Deserialize, Serialize};

use crate::source_bundle_v2::BlobDigest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub enum SchemaVersion {
    #[serde(rename = "APPLICATION_PACKAGE_V1.0")]
    ApplicationPackageV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub enum Role {
    Static,
    Compute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub enum VerityHashAlgorithm {
    #[serde(rename = "SHA256")]
    Sha256,
}

/// New physical objects use the existing BLAKE3-only source digest domain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct BlobReference {
    pub media_type: String,
    pub digest: BlobDigest,
    #[cfg_attr(
        feature = "codegen",
        schemars(range(min = 0, max = 9007199254740991u64))
    )]
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(
    feature = "codegen",
    schemars(crate = "schemars08", title = "ONREZA Application Package V1")
)]
pub struct PackageDescriptor {
    pub schema_version: SchemaVersion,
    pub role: Role,
    pub blob: BlobReference,
    #[cfg_attr(
        feature = "codegen",
        schemars(length(min = 1, max = 256), regex(pattern = r"^[^\u0000]*$"))
    )]
    pub canonicalization_profile: String,
    #[cfg_attr(
        feature = "codegen",
        schemars(length(min = 1, max = 256), regex(pattern = r"^[^\u0000]*$"))
    )]
    pub generator_identity: String,
    /// Exact EROFS length before canonical alignment padding.
    #[cfg_attr(
        feature = "codegen",
        schemars(range(min = 1, max = 9007199254740991u64))
    )]
    pub erofs_size: u64,
    pub verity: VerityGeometry,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct VerityGeometry {
    #[cfg_attr(feature = "codegen", schemars(range(min = 1, max = 1)))]
    pub version: u32,
    pub hash_algorithm: VerityHashAlgorithm,
    /// A verity root authenticates this geometry, not the whole package digest.
    #[cfg_attr(feature = "codegen", schemars(regex(pattern = "^[0-9a-f]{64}$")))]
    pub root_hash: String,
    /// V1 fixes a 32-byte salt; recipe derivation belongs to trusted materialization.
    #[cfg_attr(feature = "codegen", schemars(regex(pattern = "^[0-9a-f]{64}$")))]
    pub salt: String,
    #[cfg_attr(feature = "codegen", schemars(range(min = 4096, max = 4096)))]
    pub data_block_size: u32,
    #[cfg_attr(feature = "codegen", schemars(range(min = 4096, max = 4096)))]
    pub hash_block_size: u32,
    #[cfg_attr(
        feature = "codegen",
        schemars(range(min = 1, max = 9007199254740991u64))
    )]
    pub data_block_count: u64,
    #[cfg_attr(feature = "codegen", schemars(range(min = 0, max = 0)))]
    pub data_offset: u64,
    #[cfg_attr(
        feature = "codegen",
        schemars(range(min = 1, max = 9007199254740991u64))
    )]
    pub tree_offset: u64,
    #[cfg_attr(
        feature = "codegen",
        schemars(range(min = 0, max = 9007199254740991u64))
    )]
    pub tree_length: u64,
}
