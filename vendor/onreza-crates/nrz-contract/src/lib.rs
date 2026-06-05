//! Generated platform contract types for nrz-cli binary builds.

mod generated;
mod policy_config;

pub use generated::*;
/// Frozen ONREZA Functions static-policy config. Deserialize into the consumer's
/// policy config type so denied/allowed sets stay drift-free.
pub use policy_config::ONREZA_FUNCTIONS_POLICY_CONFIG_JSON;

pub use cli_api::{
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
pub use onreza_functions_policy::OnrezaFunctionsPolicyResultV1 as OnrezaFunctionsPolicyResult;
pub use onreza_functions_runtime_policy::OnrezaFunctionsRuntimePolicyV1 as OnrezaFunctionsRuntimePolicy;
