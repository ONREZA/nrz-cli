use std::path::Path;

use anyhow::Context;
use nrz_fn_policy::{PolicyReport, PolicyStatus};
use serde::Serialize;

use crate::cli::functions::{FunctionsArgs, FunctionsCheckArgs, FunctionsCommand};
use crate::functions;
use crate::output::{self, Phase};

const POLICY_ERROR_CODE: &str = "ONREZA_FUNCTIONS_POLICY";

pub async fn run(args: FunctionsArgs, json: bool) -> anyhow::Result<()> {
    match args.command {
        FunctionsCommand::Check(args) => check(args, json),
    }
}

fn check(args: FunctionsCheckArgs, json: bool) -> anyhow::Result<()> {
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;

    let edge_rules = functions::check_edge_rules(&project_dir)?;
    let collected: functions::CollectedFunctions = functions::collect(&project_dir)?;
    if collected.is_empty() && edge_rules.is_none() {
        return Err(output::coded_error(
            "ONREZA_FUNCTIONS_NOT_FOUND",
            "no ONREZA Function entry files or onreza.rules.toml found",
        ));
    }

    let mut reports = Vec::with_capacity(collected.functions.len());
    let mut violation_count = 0usize;
    for function in &collected.functions {
        let report = functions::run_policy_preview(&function.entrypoint, &function.sources)?;
        if report.status == PolicyStatus::Failed {
            violation_count += report.violations.len();
        }
        if !json {
            report_human(function, &report);
        }
        reports.push(FunctionCheckItem {
            name: function.name.clone(),
            report,
        });
    }

    if !json {
        report_edge_rules_human(edge_rules.as_ref());
    }

    let edge_rule_count = edge_rules.as_ref().map_or(0, |report| report.rule_count);
    let policy_error = (violation_count > 0)
        .then(|| format!("function policy check failed with {violation_count} violation(s)"));

    if json {
        output::json_output(&FunctionCheckReport {
            functions: reports,
            edge_rules: edge_rules.clone(),
            error: policy_error.clone(),
            code: policy_error.as_ref().map(|_| POLICY_ERROR_CODE.to_string()),
        });
    }

    if let Some(policy_error) = policy_error {
        if json {
            return Err(output::already_reported_error());
        }
        return Err(output::coded_error(POLICY_ERROR_CODE, policy_error));
    }

    if !json {
        output::success(
            false,
            format!(
                "check passed ({} function(s), {} edge rule(s) scanned)",
                collected.functions.len(),
                edge_rule_count
            ),
            Phase::Functions,
        );
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FunctionCheckReport {
    functions: Vec<FunctionCheckItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edge_rules: Option<functions::EdgeRulesCheckReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FunctionCheckItem {
    name: String,
    report: PolicyReport,
}

fn report_human(function: &functions::CollectedFunction, report: &PolicyReport) {
    if report.violations.is_empty() {
        return;
    }
    output::status(
        false,
        "✗",
        format!(
            "{} policy violation(s) in function '{}' ({}):",
            report.violations.len(),
            function.name,
            report.entrypoint
        ),
        Phase::Functions,
    );
    for violation in &report.violations {
        let location = violation.importer.as_deref().unwrap_or(&report.entrypoint);
        eprintln!(
            "    {} {} — {}",
            console::style(location).dim(),
            console::style(&violation.capability).yellow(),
            violation.reason
        );
    }
}

fn report_edge_rules_human(report: Option<&functions::EdgeRulesCheckReport>) {
    let Some(report) = report else {
        return;
    };
    output::status(
        false,
        "✓",
        format!(
            "{} edge rule(s) validated from {}",
            report.rule_count, report.path
        ),
        Phase::Functions,
    );
}
