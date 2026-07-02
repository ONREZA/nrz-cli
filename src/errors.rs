use serde::Serialize;

use crate::output;

#[derive(Debug)]
pub(crate) struct CliError {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) phase: Option<output::Phase>,
    pub(crate) retryable: bool,
    pub(crate) details: Option<serde_json::Value>,
    pub(crate) hint: Option<String>,
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliErrorJson<'a> {
    pub(crate) error: &'a str,
    pub(crate) code: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) phase: Option<&'a str>,
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) details: Option<&'a serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) hint: Option<&'a str>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl CliError {
    pub(crate) fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        let code = code.into();
        debug_assert!(!code.is_empty(), "CliError code must be non-empty");
        Self {
            code,
            message: message.into(),
            phase: None,
            retryable: false,
            details: None,
            hint: None,
            source: None,
        }
    }

    pub(crate) fn phase(mut self, phase: output::Phase) -> Self {
        self.phase = Some(phase);
        self
    }

    pub(crate) fn details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub(crate) fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub(crate) fn into_anyhow(self) -> anyhow::Error {
        anyhow::Error::new(self)
    }

    pub(crate) fn json(&self) -> CliErrorJson<'_> {
        CliErrorJson {
            error: &self.message,
            code: &self.code,
            phase: self.phase.map(|phase| phase.as_str()),
            retryable: self.retryable,
            details: self.details.as_ref(),
            hint: self.hint.as_deref(),
        }
    }

    pub(crate) fn phase_name(&self) -> &'static str {
        self.phase.map(|phase| phase.as_str()).unwrap_or("error")
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)?;
        if let Some(hint) = &self.hint {
            write!(f, "\n\n{hint}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

pub(crate) fn find_cli_error(err: &anyhow::Error) -> Option<&CliError> {
    err.chain()
        .find_map(|cause| cause.downcast_ref::<CliError>())
}
