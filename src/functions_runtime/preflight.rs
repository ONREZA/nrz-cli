use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Context, bail};
use serde::Serialize;

use super::inspection::inspect;
use super::release::RuntimeResolver;
use crate::functions::CollectedFunctions;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimePreflight {
    pub(crate) runtime_release_id: String,
    pub(crate) target: String,
    pub(crate) path: PathBuf,
    pub(crate) functions_loaded: usize,
}

pub(crate) async fn preflight(
    collected: &mut CollectedFunctions,
) -> anyhow::Result<RuntimePreflight> {
    let runtime = RuntimeResolver::configured()?.resolve().await?;
    preflight_with_runtime(collected, &runtime).await
}

pub(super) async fn preflight_with_runtime(
    collected: &mut CollectedFunctions,
    runtime: &super::CachedRuntime,
) -> anyhow::Result<RuntimePreflight> {
    for function in &mut collected.functions {
        function.inspected = None;
    }
    let mut inspected = Vec::with_capacity(collected.functions.len());
    let mut names = HashSet::new();
    for function in &collected.functions {
        if function.sources.len() != 1 {
            bail!(
                "function '{}' must contain exactly one entry source",
                function.name
            );
        }
        let content = function
            .sources
            .get(&function.entrypoint)
            .context("captured function source has no entrypoint")?;
        let mut result = inspect(runtime, &function.entrypoint, content)
            .await
            .with_context(|| format!("Functions runtime failed to inspect '{}'", function.name))?;
        let config = result
            .config
            .as_object_mut()
            .context("function config is not an object")?;
        config
            .entry("name")
            .or_insert_with(|| serde_json::json!(function.name));
        config
            .entry("triggers")
            .or_insert_with(|| serde_json::json!([]));
        let declaration: nrz_api::FunctionDeclaration = serde_json::from_value(result.config)
            .map_err(|_| {
                crate::output::coded_error(
                    "INVALID_CONFIG",
                    "function declaration does not match the OpenAPI contract",
                )
            })?;
        let handlers = serde_json::from_value(serde_json::json!(result.handlers))?;
        let mut spec = nrz_api::FunctionPublishSpec {
            source: nrz_api::FunctionPublishSpecSource {
                path: function.entrypoint.clone(),
                content_text: content.clone(),
            },
            declaration,
            handlers,
        };
        for trigger in &mut spec.declaration.triggers {
            trigger.name = trigger.name.trim().to_string();
        }
        nrz_api::functions::validate_publication(&spec)
            .map_err(|error| crate::output::with_default_code(error, "INVALID_CONFIG"))?;
        if !names.insert(spec.declaration.name.clone()) {
            return Err(crate::output::coded_error(
                "INVALID_CONFIG",
                format!("duplicate ONREZA Function name '{}'", spec.declaration.name),
            ));
        }
        inspected.push(spec);
    }
    for (function, spec) in collected.functions.iter_mut().zip(inspected) {
        function.name = spec.declaration.name.clone();
        function.inspected = Some(spec);
    }
    Ok(RuntimePreflight {
        runtime_release_id: runtime.runtime_release_id.clone(),
        target: runtime.target.clone(),
        path: runtime.path.clone(),
        functions_loaded: collected.functions.len(),
    })
}
