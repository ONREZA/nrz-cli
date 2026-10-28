//! Shared artifact codecs and schema-generated artifact and Edge Rules models.
//! HTTP requests and responses belong to the OpenAPI-generated nrz-api crate.

pub mod artifact_ingress_v1;

#[cfg(test)]
mod contract_tests;
mod generated;

pub mod application_package_v1;
pub mod artifact_json_v1;
pub mod runtime_artifact_graph_v3;
mod runtime_profile;
pub mod source_bundle_v2;
pub use runtime_profile::RuntimeProfile;

#[cfg(test)]
mod source_bundle_v2_tests;

#[cfg(test)]
mod runtime_artifact_graph_v3_tests;

#[cfg(feature = "codegen")]
pub mod artifact_schemas;

#[cfg(test)]
mod artifact_json_v1_tests;

#[cfg(feature = "codegen")]
pub mod codegen;

#[cfg(all(test, feature = "codegen"))]
mod codegen_tests;

pub use generated::*;

pub use application_package_v1::PackageDescriptor as ApplicationPackageV1DescriptorWire;
pub use edge_rules::OnrezaEdgeRuleSetV1 as EdgeRuleSetAuthoring;
pub use runtime_artifact_graph::{
    OnrezaRuntimeArtifactGraphV2DependencyMaterializationManifest as DependencyMaterializationManifestV1Wire,
    OnrezaRuntimeArtifactGraphV2RuntimeArtifactGraph as RuntimeArtifactGraphV2Wire,
    OnrezaRuntimeArtifactGraphV2RuntimeArtifactGraphRuntimeLayersItem as RuntimeLayerWire,
    OnrezaRuntimeArtifactGraphV2RuntimeArtifactGraphRuntimeLayersItemLaunch as RuntimeLaunchWire,
    OnrezaRuntimeArtifactGraphV2RuntimeArtifactGraphRuntimeLayersItemLaunchReadinessProtocol as RuntimeReadinessProtocol,
};
pub use runtime_artifact_graph_v3::RuntimeArtifactGraphV3 as RuntimeArtifactGraphV3Wire;
pub use source_bundle_v2::OnrezaSourceBundleV2 as SourceBundleV2ManifestWire;
