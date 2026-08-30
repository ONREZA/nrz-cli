//! Generated platform contract types for nrz-cli binary builds.

mod generated;

pub use generated::*;

pub use cli_api::{
    OnrezaCliApiV1EdgeRulesStatusRequest as CliEdgeRulesStatusRequest,
    OnrezaCliApiV1EdgeRulesStatusResponse as CliEdgeRulesStatusResponse,
    OnrezaCliApiV1FunctionTestInvokeRequest as CliFunctionTestInvokeRequest,
    OnrezaCliApiV1FunctionTestInvokeResponse as CliFunctionTestInvokeResponse,
    OnrezaCliApiV1MultipartCompleteRequest as CliMultipartCompleteRequest,
    OnrezaCliApiV1MultipartCompleteResponse as CliMultipartCompleteResponse,
    OnrezaCliApiV1PrepareUploadRequest as CliPrepareUploadRequest,
    OnrezaCliApiV1PrepareUploadResponse as CliPrepareUploadResponse,
    OnrezaCliApiV1PrepareUploadResponseMultipartChunksItem as CliPrepareUploadResponseMultipartChunk,
    OnrezaCliApiV1PrepareUploadResponsePresignedPutHeaders as CliPrepareUploadResponsePresignedPutHeaders,
    OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHead as CliPrepareUploadResponsePresignedPutVerifyHead,
    OnrezaCliApiV1PrepareUploadResponseRequiredComplete as CliPrepareUploadRequiredComplete,
    OnrezaCliApiV1UploadCompleteRequest as CliUploadCompleteRequest,
    OnrezaCliApiV1UploadCompleteResponse as CliUploadCompleteResponse,
    OnrezaCliApiV1UploadFailedRequest as CliUploadFailedRequest,
    OnrezaCliApiV1UploadFailedResponse as CliUploadFailedResponse,
};
pub use edge_rules::OnrezaEdgeRuleSetV1 as EdgeRuleSetAuthoring;
pub use runtime_artifact_graph::{
    OnrezaRuntimeArtifactGraphV2DependencyMaterializationManifest as DependencyMaterializationManifestV1Wire,
    OnrezaRuntimeArtifactGraphV2RuntimeArtifactGraph as RuntimeArtifactGraphV2Wire,
};
