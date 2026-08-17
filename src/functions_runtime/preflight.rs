use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Serialize;

use super::process::RuntimeProcess;
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
    project_dir: &Path,
    collected: &CollectedFunctions,
) -> anyhow::Result<RuntimePreflight> {
    let runtime = RuntimeResolver::pinned()?.resolve().await?;
    for function in &collected.functions {
        RuntimeProcess::start(&runtime, project_dir, &function.entrypoint)
            .await
            .with_context(|| format!("Functions runtime failed to load '{}'", function.name))?
            .shutdown()
            .await
            .with_context(|| {
                format!("Functions runtime failed to stop after '{}'", function.name)
            })?;
    }
    Ok(RuntimePreflight {
        runtime_release_id: runtime.runtime_release_id,
        target: runtime.target,
        path: runtime.path,
        functions_loaded: collected.functions.len(),
    })
}
