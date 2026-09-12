//! Cross-field publication invariants that JSON Schema cannot express.

use std::collections::HashSet;

use anyhow::{bail, ensure};
use validator::Validate as _;

use crate::{FunctionDeclaredTriggerType, FunctionHandler, FunctionPublishSpec};

pub const MAX_FUNCTIONS_PER_PUBLISH: usize = 1_000;
pub const MAX_FUNCTION_SOURCE_FILE_BYTES: u64 = 128 * 1024;
pub const MAX_FUNCTION_SOURCE_FILES_PER_FUNCTION: usize = 1;

/// Data validation only. Runtime loading belongs to the isolated Functions engine.
pub fn validate_publication(function: &FunctionPublishSpec) -> anyhow::Result<()> {
    function
        .validate()
        .map_err(|_| anyhow::anyhow!("function does not match the OpenAPI contract"))?;
    ensure!(
        is_supported_source_path(&function.source.path),
        "function source path must be relative and canonical"
    );
    ensure!(
        function.source.content_text.len() as u64 <= MAX_FUNCTION_SOURCE_FILE_BYTES,
        "function source exceeds the UTF-8 byte limit"
    );
    let handlers = function.handlers.iter().collect::<HashSet<_>>();
    ensure!(
        handlers.len() == function.handlers.len(),
        "function handlers must be unique"
    );
    let mut names = HashSet::new();
    for trigger in &function.declaration.triggers {
        ensure!(
            !trigger.name.trim().is_empty() && trigger.name == trigger.name.trim(),
            "function trigger name must be trimmed and nonempty"
        );
        ensure!(
            names.insert(&trigger.name),
            "duplicate function trigger name"
        );
        let handler = match trigger.r#type {
            FunctionDeclaredTriggerType::Manual => FunctionHandler::Manual,
            FunctionDeclaredTriggerType::Queue => FunctionHandler::Queue,
            FunctionDeclaredTriggerType::Scheduled => FunctionHandler::Scheduled,
        };
        if !handlers.contains(&handler) {
            bail!("function trigger requires its matching handler");
        }
    }
    Ok(())
}

/// Shared source-storage path policy, including historical non-branded snapshots.
pub fn is_supported_source_path(path: &str) -> bool {
    !path.contains(['\\', '\0', ':'])
        && path
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | ".." | "node_modules"))
        && [".ts", ".tsx", ".js", ".jsx", ".mjs"]
            .iter()
            .any(|suffix| path.ends_with(suffix))
}
