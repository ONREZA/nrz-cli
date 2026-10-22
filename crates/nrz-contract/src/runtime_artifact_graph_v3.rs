//! Bounded immutable runtime projection; routing and execution authorization
//! remain release-owned, and the exact graph digest covers external wire bytes.

use crate::application_package_v1::{BlobReference, PackageDescriptor};
use crate::source_bundle_v2::{LayerName, Network, NonEmptyString, optional_non_null};
use serde::{Deserialize, Serialize};

pub use crate::RuntimeProfile;
pub use crate::runtime_artifact_graph::{
    OnrezaRuntimeArtifactGraphV2RuntimeArtifactGraphDependenciesItem as LegacyDependencyDescriptor,
    OnrezaRuntimeArtifactGraphV2RuntimeArtifactGraphRuntimeLayersItemDependencyBindingsMountsItem as DependencyMount,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub enum SchemaVersion {
    #[serde(rename = "RUNTIME_ARTIFACT_GRAPH_V3.0")]
    V3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub enum MembershipSchemaVersion {
    #[serde(rename = "APPLICATION_MEMBERSHIP_V1.0")]
    V1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(
    feature = "codegen",
    schemars(crate = "schemars08", title = "ONREZA Runtime Artifact Graph V3")
)]
pub struct RuntimeArtifactGraphV3 {
    pub schema_version: SchemaVersion,
    pub application: ApplicationMembership,
    #[cfg_attr(feature = "codegen", schemars(length(max = 16)))]
    pub dependencies: Vec<DependencyDescriptor>,
    #[cfg_attr(feature = "codegen", schemars(length(max = 10)))]
    pub layers: Vec<Layer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct ApplicationMembership {
    pub schema_version: MembershipSchemaVersion,
    pub manifest: BlobReference,
    #[cfg_attr(feature = "codegen", schemars(length(max = 2)))]
    pub components: Vec<PackageDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct DependencyDescriptor {
    #[cfg_attr(
        feature = "codegen",
        schemars(schema_with = "crate::artifact_schemas::legacy_dependency")
    )]
    pub legacy: LegacyDependencyDescriptor,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    pub physical_blob: Option<BlobReference>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "target", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
#[allow(
    clippy::large_enum_variant,
    reason = "The graph admits at most ten layers; inline wire fields avoid an allocation per Compute layer"
)]
pub enum Layer {
    #[serde(rename = "STATIC")]
    Static {
        name: LayerName,
        #[serde(
            rename = "rootPath",
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "optional_non_null"
        )]
        root_path: Option<NonEmptyString>,
    },
    #[serde(rename = "COMPUTE")]
    Compute {
        name: LayerName,
        #[serde(
            rename = "rootPath",
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "optional_non_null"
        )]
        root_path: Option<NonEmptyString>,
        entrypoint: NonEmptyString,
        launch: Launch,
        network: Network,
        limits: RuntimeLimits,
        #[cfg_attr(feature = "codegen", schemars(length(min = 1, max = 128)))]
        #[serde(rename = "buildRuntimeTarget")]
        build_runtime_target: String,
        #[serde(
            rename = "runtimePolicyLineage",
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "optional_non_null"
        )]
        runtime_policy_lineage: Option<String>,
        #[cfg_attr(
            feature = "codegen",
            schemars(schema_with = "crate::artifact_schemas::dependency_mounts")
        )]
        #[serde(rename = "dependencyMounts")]
        dependency_mounts: Vec<DependencyMount>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct Launch {
    #[cfg_attr(
        feature = "codegen",
        schemars(schema_with = "crate::artifact_schemas::runtime_profile")
    )]
    pub profile: RuntimeProfile,
    #[cfg_attr(
        feature = "codegen",
        schemars(schema_with = "crate::artifact_schemas::runtime_arguments")
    )]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct RuntimeLimits {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    #[cfg_attr(
        feature = "codegen",
        schemars(range(min = 1, max = 9007199254740991u64))
    )]
    pub timeout_ms: Option<u64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    #[cfg_attr(feature = "codegen", schemars(range(min = 32, max = 8192)))]
    pub memory_mb: Option<u64>,
}
