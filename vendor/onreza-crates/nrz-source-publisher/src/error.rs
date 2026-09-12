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
    /// A definitive rejection of this artifact, after publication's bounded retries.
    /// Transport ambiguity, throttling, credentials and server failures retain recovery.
    #[must_use]
    pub fn terminal_error_code(&self) -> Option<&'static str> {
        let Self::ControlPlane(error) = self else {
            return None;
        };
        if !(400..500).contains(&error.status) || error.status == 429 {
            return None;
        }
        match error.code.as_str() {
            "LIMIT_EXCEEDED" => Some("LIMIT_EXCEEDED"),
            "VALIDATION_ERROR" => Some("INVALID_ARTIFACT_HANDOFF"),
            "EDGE_RULES_DIVERGED" => Some("EDGE_RULES_DIVERGED"),
            _ => None,
        }
    }

    #[must_use]
    pub fn structured_control_plane(&self) -> Option<&StructuredControlPlaneError> {
        match self {
            Self::ControlPlane(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_definitive_artifact_rejection_from_recovery() {
        for (status, code, expected) in [
            (403, "LIMIT_EXCEEDED", Some("LIMIT_EXCEEDED")),
            (400, "VALIDATION_ERROR", Some("INVALID_ARTIFACT_HANDOFF")),
            (503, "LIMIT_EXCEEDED", None),
            (429, "LIMIT_EXCEEDED", None),
            (409, "OPERATION_IN_PROGRESS", None),
            (401, "UNAUTHORIZED", None),
            (403, "UNKNOWN", None),
        ] {
            let error = SourcePublicationError::ControlPlane(StructuredControlPlaneError {
                status,
                code: code.into(),
                message: "provider detail".into(),
                retry_after: None,
                details: None,
            });
            assert_eq!(error.terminal_error_code(), expected);
        }
        assert_eq!(
            SourcePublicationError::AmbiguousTransport("connection lost".into())
                .terminal_error_code(),
            None
        );
    }
}
