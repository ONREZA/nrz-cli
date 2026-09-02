// @generated vendored copy of platform crates/nrz-source-publisher/src/error.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Error)]
#[error("control-plane request failed ({status} {code}): {message}")]
pub struct StructuredControlPlaneError {
    pub status: u16,
    pub code: String,
    pub message: String,
    pub retry_after: Option<Duration>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Error)]
pub enum SourcePublicationError {
    #[error(transparent)]
    ControlPlane(#[from] StructuredControlPlaneError),
    #[error("control-plane transport failed before a response was received: {0}")]
    AmbiguousTransport(String),
    #[error("control-plane response is invalid: {0}")]
    InvalidResponse(String),
    #[error("source bundle is invalid: {0}")]
    InvalidSourceBundle(String),
    #[error("source bundle I/O failed while attempting to {operation}: {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("conditional source upload conflicts with an existing object: {0}")]
    ConditionalUploadConflict(String),
    #[error("source object upload failed: {0}")]
    ObjectUpload(String),
    #[error("source publication did not converge before its deadline: {0}")]
    Deadline(String),
}

impl SourcePublicationError {
    #[must_use]
    pub fn structured_control_plane(&self) -> Option<&StructuredControlPlaneError> {
        match self {
            Self::ControlPlane(error) => Some(error),
            _ => None,
        }
    }
}
