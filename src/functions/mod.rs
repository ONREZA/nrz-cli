//! ONREZA Functions publish support: bounded source collection + local policy
//! preview. The CLI is transport — it collects self-contained function source
//! and runs the same static policy as the platform for fast feedback, but the
//! platform (`artifact-ingest`) is the authoritative trust boundary.

mod collect;
mod payload;
mod policy;
mod rules;
mod rules_authoring;

#[cfg(test)]
mod collect_tests;
#[cfg(test)]
mod publish_tests;
#[cfg(test)]
mod rules_authoring_tests;

pub use collect::{CollectedFunction, CollectedFunctions, collect};
pub use payload::{FunctionPublishPayload, GeneratedEdgeRuleSet, build_payload};
pub use policy::run_policy_preview;
pub(crate) use rules::validate_edge_rules_value;
pub use rules::{
    EdgeRulesCheckReport, check_edge_rules, edge_image_source_count, edge_rule_count,
    load_edge_rules,
};
