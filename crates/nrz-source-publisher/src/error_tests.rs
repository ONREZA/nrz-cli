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
        let error = SourcePublicationError::from(StructuredControlPlaneError {
            status,
            code: code.into(),
            message: "provider detail".into(),
            retry_after: None,
            details: None,
        });
        assert_eq!(error.terminal_error_code(), expected);
    }
    assert_eq!(
        SourcePublicationError::AmbiguousTransport("connection lost".into()).terminal_error_code(),
        None
    );
}
