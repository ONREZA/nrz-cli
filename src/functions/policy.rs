use anyhow::Context;
use nrz_fn_policy::{PolicyConfig, PolicyReport, SourceSet, run_function_policy_check};

/// Build the policy config from the generated contract embedded in `nrz-contract`.
/// This is the single source of truth shared with the platform publish gate, so
/// the CLI preview and the authoritative `artifact-ingest` check stay in lockstep.
pub fn policy_config() -> anyhow::Result<PolicyConfig> {
    serde_json::from_str(nrz_contract::ONREZA_FUNCTIONS_POLICY_CONFIG_JSON)
        .context("failed to parse embedded ONREZA Functions policy config")
}

/// Run the publish-time function policy scan over the bounded source set.
/// The CLI runs this for a local preview; the platform always re-validates.
pub fn run_policy_preview(entrypoint: &str, sources: &SourceSet) -> anyhow::Result<PolicyReport> {
    Ok(run_function_policy_check(
        &policy_config()?,
        entrypoint,
        sources,
    ))
}
