use super::{
    BuildLogEmitter, BuildLogLevel, BuildLogPhase, BuildLogStream, BuildLogSuccess,
    ExactValueRedactor, FinishRequest, error_code, error_details, sanitize_message,
};
use crate::output;

/// Build outcome and client observation outcome have different authorities.
pub(in crate::deploy) enum BuildLogOutcome<'a> {
    Completed(BuildLogSuccess),
    ObservationStopped {
        success: BuildLogSuccess,
        error: &'a anyhow::Error,
    },
    Failed(&'a anyhow::Error),
}

impl BuildLogOutcome<'_> {
    pub(super) fn prepare(
        &self,
        phase: BuildLogPhase,
        redactor: &ExactValueRedactor,
        emitter: Option<&BuildLogEmitter>,
    ) -> FinishRequest {
        let (failure, notice) = match self {
            Self::Completed(_) => (None, None),
            Self::ObservationStopped { error, .. } => (None, Some(*error)),
            Self::Failed(error) => (Some(*error), Some(*error)),
        };
        let message = notice.map(|error| {
            let message = output::reported_terminal_diagnostic(error).map_or_else(
                || error.to_string(),
                |diagnostic| diagnostic.message.clone(),
            );
            sanitize_message(&message, redactor)
        });
        if let Some(emitter) = emitter {
            match self {
                Self::Completed(success) => {
                    emitter.info(BuildLogPhase::Complete, success.message())
                }
                Self::ObservationStopped { success, error } => {
                    emitter.emit(
                        BuildLogStream::User,
                        BuildLogLevel::Warn,
                        BuildLogPhase::Activate,
                        emitter.lifecycle_origin,
                        &error.to_string(),
                    );
                    emitter.info(BuildLogPhase::Complete, success.message());
                }
                Self::Failed(error) => emitter.error(BuildLogPhase::Error, &error.to_string()),
            }
            emitter.close();
        }
        FinishRequest {
            status: if failure.is_some() {
                nrz_api::FinishRequestBodyStatus::Failed
            } else {
                nrz_api::FinishRequestBodyStatus::Finished
            },
            message,
            error_code: failure.and_then(error_code),
            error_details: failure.and_then(error_details).map(|details| {
                match redactor.sanitize_json(&details) {
                    serde_json::Value::Object(fields) => fields.into_iter().collect(),
                    value => std::collections::HashMap::from([("value".into(), value)]),
                }
            }),
            failure_phase: failure.map(|_| phase.into()),
        }
    }
}

#[cfg(test)]
#[path = "build_log_completion_tests.rs"]
mod tests;
