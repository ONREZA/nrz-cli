//! Shared artifact and Edge Rules models generated from the committed schemas.
//! HTTP requests and responses belong to the OpenAPI-generated nrz-api crate.

mod generated;

pub use generated::*;

pub use edge_rules::OnrezaEdgeRuleSetV1 as EdgeRuleSetAuthoring;
pub use runtime_artifact_graph::{
    OnrezaRuntimeArtifactGraphV2DependencyMaterializationManifest as DependencyMaterializationManifestV1Wire,
    OnrezaRuntimeArtifactGraphV2RuntimeArtifactGraph as RuntimeArtifactGraphV2Wire,
    OnrezaRuntimeArtifactGraphV2RuntimeArtifactGraphRuntimeLayersItem as RuntimeLayerWire,
    OnrezaRuntimeArtifactGraphV2RuntimeArtifactGraphRuntimeLayersItemLaunch as RuntimeLaunchWire,
    OnrezaRuntimeArtifactGraphV2RuntimeArtifactGraphRuntimeLayersItemLaunchProfile as RuntimeProfile,
    OnrezaRuntimeArtifactGraphV2RuntimeArtifactGraphRuntimeLayersItemLaunchReadinessProtocol as RuntimeReadinessProtocol,
};
