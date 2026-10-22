use super::*;

pub(super) fn map_publication_error(
    error: anyhow::Error,
    json: bool,
    runtime_artifact_files: &crate::artifact::RuntimeArtifactFileBreakdown,
) -> anyhow::Error {
    if let Some(api_error) = error.downcast_ref::<crate::api::StructuredApiError>() {
        let is_file_limit = api_error.code == "LIMIT_EXCEEDED"
            && api_error
                .details
                .as_ref()
                .and_then(|details| details.get("limitType"))
                .and_then(serde_json::Value::as_str)
                == Some("maxDeploymentFiles");
        let details = is_file_limit.then(|| {
            let mut details = api_error
                .details
                .clone()
                .unwrap_or_else(|| serde_json::json!({}));
            if let Some(object) = details.as_object_mut() {
                object.insert(
                    "runtimeArtifactFiles".to_string(),
                    serde_json::json!(runtime_artifact_files),
                );
            }
            details
        });
        let message = if is_file_limit {
            runtime_artifact_file_limit_message(&api_error.message, runtime_artifact_files)
        } else {
            api_error.message.clone()
        };

        if json
            && matches!(
                api_error.code.as_str(),
                "LIMIT_EXCEEDED" | "SUBSCRIPTION_REQUIRED"
            )
        {
            return output::report_terminal_error(
                "deploy",
                &message,
                &api_error.code,
                details.as_ref().or(api_error.details.as_ref()),
            );
        }
        if matches!(
            api_error.code.as_str(),
            "LIMIT_EXCEEDED" | "SUBSCRIPTION_REQUIRED"
        ) {
            let mut mapped =
                crate::errors::CliError::new(&api_error.code, message).phase(output::Phase::Deploy);
            if let Some(details) = details.or_else(|| api_error.details.clone()) {
                mapped = mapped.details(details);
            }
            return mapped.into_anyhow();
        }
    }
    error.context("failed to publish verified source bundle")
}

fn runtime_artifact_file_limit_message(
    message: &str,
    breakdown: &crate::artifact::RuntimeArtifactFileBreakdown,
) -> String {
    let categories = breakdown
        .categories()
        .map(|(label, count)| format!("{label} {count}"))
        .collect::<Vec<_>>();
    let mut guidance = format!(
        "{message}\n\nRuntime artifact file breakdown: {}; total {}.",
        categories.join(", "),
        breakdown.total
    );
    if breakdown.includes_installed_dependencies() {
        guidance.push_str(
            "\nFor PROCESS/SSR deployments, installed runtime dependencies are part of the runtime artifact even when Output Directory selects a build directory.",
        );
        guidance.push_str(
            "\nReduce the runtime closure by removing unused dependencies after the build or by producing a self-contained application artifact. Use a static adapter only when server-side execution is not required.",
        );
    } else {
        guidance.push_str(
            "\nReduce generated deployment files or produce a more compact, self-contained artifact before retrying.",
        );
    }
    guidance
}
