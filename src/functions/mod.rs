//! ONREZA Functions publish support: bounded source collection and Edge Rules.
//! The CLI validates executable compatibility with its pinned native runtime;
//! the native runtime remains the executable compatibility authority.

mod collect;
mod payload;
mod rules;
mod rules_authoring;

#[cfg(test)]
mod collect_tests;
#[cfg(test)]
mod publish_tests;
#[cfg(test)]
mod rules_authoring_tests;

pub use collect::{CollectedFunctions, collect};
pub use payload::{FunctionPublishPayload, GeneratedEdgeRuleSet, build_payload};
pub(crate) use rules::validate_edge_rules_value;
pub use rules::{
    EdgeRulesCheckReport, check_edge_rules, edge_image_source_count, edge_rule_count,
    load_edge_rules,
};
