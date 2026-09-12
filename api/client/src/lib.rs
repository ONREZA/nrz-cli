//! Generated ONREZA platform API and shared HTTP policy.

pub mod functions;

#[allow(
    clippy::needless_question_mark,
    clippy::needless_return,
    clippy::needless_update,
    reason = "generated SDK templates; behavioral checks and Rust compilation remain enabled"
)]
pub mod generated;
pub use generated::*;

/// Keep incompatible response values out of terminal errors, including token/env responses.
/// Transport errors retain their original type so consumers can classify retries.
pub fn response_error(error: anyhow::Error) -> anyhow::Error {
    if error.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<oas3_gen_support::DiagnosticsError>(),
            Some(oas3_gen_support::DiagnosticsError::DeserializationError { .. })
        )
    }) {
        anyhow::anyhow!(
            "server response does not match the OpenAPI contract; update the CLI or check server compatibility"
        )
    } else {
        error
    }
}
