use anyhow::Context as _;
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
    #[serde(default, skip_serializing_if = "is_false")]
    pub edge_rules_force: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generated_edge_rule_sets: Vec<GeneratedEdgeRuleSet>,
}

pub use nrz_api::FunctionPublishSpec;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedEdgeRuleSet {
    pub producer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub edge_rules: Value,
}

/// Assemble the publish payload from the discovered functions and optional edge
/// rules. Only inspected source and evaluated metadata may cross this boundary.
pub fn build_payload(
    origin: &'static str,
    collected: &CollectedFunctions,
    edge_rules: Option<Value>,
    edge_rules_force: bool,
    generated_edge_rule_sets: Vec<GeneratedEdgeRuleSet>,
) -> anyhow::Result<FunctionPublishPayload> {
    let functions = collected
        .functions
        .iter()
        .map(|function| {
            function.inspected.clone().with_context(|| {
                format!(
                    "function '{}' has not passed runtime inspection",
                    function.entrypoint
                )
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(FunctionPublishPayload {
        origin,
        functions,
        edge_rules,
        edge_rules_force,
        generated_edge_rule_sets,
    })
}

fn is_false(value: &bool) -> bool {
    !*value
}
