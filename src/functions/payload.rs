use serde::Serialize;
use serde_json::Value;

use super::collect::CollectedFunctions;

/// Function publish/source snapshot payload. Mirrors the shared
/// `FunctionPublishPayloadSchema`; the platform re-validates it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionPublishPayload {
    pub origin: &'static str,
    pub functions: Vec<FunctionPublishSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_rules: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionPublishSpec {
    pub source: FunctionSourceFile,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionSourceFile {
    pub path: String,
    pub content_text: String,
}

/// Assemble the publish payload from the discovered functions and optional edge
/// rules. The platform re-extracts triggers from each entry source.
pub fn build_payload(
    origin: &'static str,
    collected: &CollectedFunctions,
    edge_rules: Option<Value>,
) -> FunctionPublishPayload {
    let functions = collected
        .functions
        .iter()
        .map(|function| {
            let content_text = function
                .sources
                .get(&function.entrypoint)
                .expect("collected function entrypoint must exist in source set")
                .clone();
            FunctionPublishSpec {
                source: FunctionSourceFile {
                    path: function.entrypoint.clone(),
                    content_text,
                },
            }
        })
        .collect();

    FunctionPublishPayload {
        origin,
        functions,
        edge_rules,
    }
}
