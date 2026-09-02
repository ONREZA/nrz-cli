// @generated vendored copy of platform crates/nrz-source-publisher/src/lib.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

//! Shared SOURCE_BUNDLE_V1 publication state machine used by local `nrz deploy`
//! and the trusted Edge Builder Agent.

mod bundle;
mod error;
mod http;
mod publisher;
mod runtime;

pub use bundle::{PreparedSourceBundle, SourceBundleInput, source_uses_multipart};
pub use error::{SourcePublicationError, StructuredControlPlaneError};
pub use http::HttpSourcePublicationTransport;
pub use publisher::{
    DeploymentPublicationStatus, NoopPublicationObserver, ObjectHeadVerification,
    ObjectUploadHeaders, ObjectUploadRequest, ObjectUploadResult, PublicationEvent,
    PublicationObserver, PublishedSourceBundle, PublishedSourceUpload, SourcePublicationRequest,
    SourcePublicationTransport, publish_source_bundle, publish_source_bundle_upload,
};
pub use runtime::{
    PublishedRuntimeArtifacts, RuntimeArtifactPublicationRequest,
    RuntimeArtifactPublicationTransport, RuntimeDependencyPublicationInput,
    RuntimeFileUploadRequest, RuntimePublicationCompleteRequest,
    RuntimePublicationCompleteResponse, RuntimePublicationPrepareRequest,
    RuntimePublicationPrepareResponse, publish_runtime_artifacts,
};
