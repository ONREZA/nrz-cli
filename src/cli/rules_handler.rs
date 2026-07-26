use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::api::ApiClient;
use crate::auth;
use crate::cli::rules::{
    RulesArgs, RulesCheckArgs, RulesCommand, RulesPublishArgs, RulesPullArgs, RulesStatusArgs,
};
use crate::execution_context;
use crate::functions;
use crate::output;
use nrz::config;
use nrz::config::ProjectConfig;

const RULES_FILENAME: &str = "onreza.rules.toml";
const EDGE_RULE_SET_SCHEMA_VERSION: &str = "EDGE_RULE_SET_V1";

pub async fn run(
    args: RulesArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    match args.command {
        RulesCommand::Check(args) => check(args, json),
        RulesCommand::Pull(args) => pull(args, json, token, workspace, config).await,
        RulesCommand::Publish(args) => publish(args, json, token, workspace, config).await,
        RulesCommand::Status(args) => status(args, json, token, workspace, config).await,
    }
}

fn check(args: RulesCheckArgs, json: bool) -> anyhow::Result<()> {
    let project_dir = canonical_project_dir(&args.dir)?;
    let Some(report) = functions::check_edge_rules(&project_dir)? else {
        return Err(output::coded_error(
            "ONREZA_RULES_NOT_FOUND",
            format!("{} not found", project_dir.join(RULES_FILENAME).display()),
        ));
    };

    if json {
        output::json_output(&report);
    } else {
        report_check_human(&report);
        output::success(
            false,
            format!("rules check passed ({} edge rule(s))", report.rule_count),
            output::Phase::Rules,
        );
    }
    Ok(())
}

async fn pull(
    args: RulesPullArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let project_dir = canonical_project_dir(&args.dir)?;
    let ctx = remote_context(
        token,
        workspace,
        config,
        &project_dir,
        args.project_id.as_deref(),
        args.environment.as_deref(),
    )
    .await?;
    let active = get_active_rule_set(&ctx.client, &ctx.project_id, &ctx.environment_id).await?;
    let authoring = active_rule_set_to_authoring_value(&active)?;
    let content = edge_rule_set_authoring_to_toml(&authoring)?;
    let path = project_dir.join(RULES_FILENAME);

    confirm_overwrite(&path, args.force, json)?;
    write_rules_file(&path, content.as_bytes())?;

    let rule_count = authoring
        .get("rules")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let output = RulesPullOutput {
        path: path.display().to_string(),
        environment_id: ctx.environment_id,
        rule_count,
        version: active.version,
        source: active.source,
        checksum: active.checksum,
    };

    if json {
        output::json_output(&output);
    } else {
        output::success(
            false,
            format!(
                "pulled {} edge rule(s) into {}",
                output.rule_count, output.path
            ),
            output::Phase::Rules,
        );
    }
    Ok(())
}

async fn publish(
    args: RulesPublishArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let project_dir = canonical_project_dir(&args.dir)?;
    let edge_rules = functions::load_edge_rules(&project_dir)?.ok_or_else(|| {
        output::coded_error(
            "ONREZA_RULES_NOT_FOUND",
            format!("{} not found", project_dir.join(RULES_FILENAME).display()),
        )
    })?;
    let rule_count = functions::edge_rule_count(&edge_rules);
    let ctx = remote_context(
        token,
        workspace,
        config,
        &project_dir,
        args.project_id.as_deref(),
        args.environment.as_deref(),
    )
    .await?;
    let body = functions::FunctionPublishPayload {
        origin: "CLI",
        functions: Vec::new(),
        edge_rules: Some(edge_rules),
        edge_rules_force: args.force_rules,
        generated_edge_rule_sets: Vec::new(),
    };
    let response: Value = match ctx
        .client
        .post(
            &format!(
                "/v1/projects/{}/function-activations/environments/{}/functions/publish",
                ctx.project_id, ctx.environment_id
            ),
            &body,
        )
        .await
    {
        Ok(response) => response,
        Err(error) => return Err(map_publish_error(error, json)),
    };

    if json {
        output::json_output(&RulesPublishOutput {
            environment_id: ctx.environment_id,
            rule_count,
            result: response,
        });
    } else {
        output::success(
            false,
            format!("published {rule_count} edge rule(s)"),
            output::Phase::Rules,
        );
    }
    Ok(())
}

async fn status(
    args: RulesStatusArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let project_dir = canonical_project_dir(&args.dir)?;
    let ctx = remote_context(
        token,
        workspace,
        config,
        &project_dir,
        args.project_id.as_deref(),
        args.environment.as_deref(),
    )
    .await?;
    let local = load_local_rules_for_status(&project_dir);
    let request = build_edge_rules_status_request(local.edge_rules, local.local_invalid)?;
    let response =
        get_edge_rules_status(&ctx.client, &ctx.project_id, &ctx.environment_id, &request).await?;
    let output = RulesStatusOutput::from_contract(response, local.file)?;

    if json {
        output::json_output(&output);
    } else {
        report_status_human(&output);
    }
    Ok(())
}

struct RemoteContext {
    client: ApiClient,
    project_id: String,
    environment_id: String,
}

async fn remote_context(
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
    project_dir: &Path,
    project_id: Option<&str>,
    environment: Option<&str>,
) -> anyhow::Result<RemoteContext> {
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;
    let project_id = config::resolve_project_id(project_id, config)?;
    let environment_id = execution_context::resolve_for_mutation(
        &client,
        &project_id,
        project_dir,
        environment,
        None,
    )
    .await?
    .environment_id;
    Ok(RemoteContext {
        client,
        project_id,
        environment_id,
    })
}

pub(crate) fn map_publish_error(error: anyhow::Error, json: bool) -> anyhow::Error {
    let Some(api_error) = error.downcast_ref::<crate::api::StructuredApiError>() else {
        return error.context("failed to publish Edge Rules");
    };
    if api_error.code != "EDGE_RULES_DIVERGED" {
        return error.context("failed to publish Edge Rules");
    }

    let message = format_edge_rules_diverged_failure(api_error);
    if json {
        return output::report_terminal_error(
            "rules",
            &message,
            "EDGE_RULES_DIVERGED",
            api_error.details.as_ref(),
        );
    }
    output::coded_error("EDGE_RULES_DIVERGED", message).context("failed to publish Edge Rules")
}

fn format_edge_rules_diverged_failure(error: &crate::api::StructuredApiError) -> String {
    let message = error
        .details
        .as_ref()
        .and_then(|details| details.get("message"))
        .and_then(Value::as_str)
        .unwrap_or(error.message.as_str());
    format!(
        "Edge Rules diverged: {message}. Run `nrz rules pull` to import dashboard-authored rules, or rerun `nrz rules publish --force-rules` to replace them."
    )
}

async fn get_active_rule_set(
    client: &ApiClient,
    project_id: &str,
    environment_id: &str,
) -> anyhow::Result<ActiveEdgeRuleSet> {
    let response: ActiveEdgeRuleSetResponse = client
        .get(&format!(
            "/v1/projects/{project_id}/function-activations/environments/{environment_id}/edge-rules"
        ))
        .await
        .context("failed to fetch active Edge Rules")?;
    response.rule_set.ok_or_else(|| {
        output::coded_error(
            "ONREZA_RULES_NOT_FOUND",
            "no user-authored Edge Rules found for this environment",
        )
    })
}

async fn get_edge_rules_status(
    client: &ApiClient,
    project_id: &str,
    environment_id: &str,
    request: &nrz_contract::CliEdgeRulesStatusRequest,
) -> anyhow::Result<nrz_contract::CliEdgeRulesStatusResponse> {
    client
        .post(
            &format!(
                "/v1/projects/{project_id}/function-activations/environments/{environment_id}/edge-rules/status"
            ),
            request,
        )
        .await
        .context("failed to fetch Edge Rules status")
}

fn canonical_project_dir(dir: &str) -> anyhow::Result<PathBuf> {
    Path::new(dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {dir}"))
}

fn confirm_overwrite(path: &Path, force: bool, json: bool) -> anyhow::Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error).with_context(|| format!("failed to inspect {}", path.display()));
        }
    };
    if metadata
        .as_ref()
        .is_some_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(output::coded_error(
            "UNSAFE_FILE_TARGET",
            format!("refusing to overwrite symbolic link {}", path.display()),
        ));
    }
    if metadata.is_none() || force {
        return Ok(());
    }
    if json || !std::io::stdin().is_terminal() {
        return Err(output::coded_error(
            "FILE_EXISTS",
            format!(
                "{} already exists; pass --force to overwrite it",
                path.display()
            ),
        ));
    }

    eprint!("  Overwrite {}? [y/N]: ", path.display());
    std::io::stderr().flush()?;
    let mut line = String::new();
    let bytes = std::io::stdin().lock().read_line(&mut line)?;
    if bytes == 0 || !matches!(line.trim(), "y" | "Y" | "yes" | "YES") {
        return Err(output::coded_error(
            "CANCELLED",
            format!("left {} unchanged", path.display()),
        ));
    }
    Ok(())
}

pub(crate) fn write_rules_file(path: &Path, content: &[u8]) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("rules path has no parent: {}", path.display()))?;
    let temp_path = parent.join(format!(
        ".{RULES_FILENAME}.tmp-{}",
        uuid::Uuid::now_v7().simple()
    ));
    let write_result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .with_context(|| format!("failed to create {}", temp_path.display()))?;
        file.write_all(content)
            .with_context(|| format!("failed to write {}", temp_path.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to sync {}", temp_path.display()))?;
        drop(file);

        #[cfg(windows)]
        if std::fs::symlink_metadata(path).is_ok() {
            std::fs::remove_file(path)
                .with_context(|| format!("failed to replace {}", path.display()))?;
        }

        std::fs::rename(&temp_path, path)
            .with_context(|| format!("failed to replace {}", path.display()))
    })();
    if write_result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    write_result
}

fn report_check_human(report: &functions::EdgeRulesCheckReport) {
    output::status(
        false,
        "✓",
        format!("{} edge rule(s) in {}", report.rule_count, report.path),
        output::Phase::Rules,
    );
    for rule in &report.rules {
        eprintln!(
            "    {:<24} {:<14} {}",
            rule.id,
            rule.action,
            if rule.enabled { "enabled" } else { "disabled" }
        );
    }
}

pub(crate) fn active_rule_set_to_authoring_value(
    rule_set: &ActiveEdgeRuleSet,
) -> anyhow::Result<Value> {
    let rules = rule_set
        .rules
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("active Edge Rules response has non-array rules"))?;
    let mut indexed_rules: Vec<(usize, &Value)> = rules.iter().enumerate().collect();
    indexed_rules.sort_by_key(|(index, rule)| {
        (
            rule.get("position")
                .and_then(Value::as_i64)
                .unwrap_or(*index as i64),
            *index,
        )
    });

    let rules = indexed_rules
        .into_iter()
        .map(|(_, rule)| {
            let mut rule = rule.clone();
            if let Some(object) = rule.as_object_mut() {
                object.remove("position");
            }
            rule
        })
        .collect::<Vec<_>>();

    let value = json!({
        "schemaVersion": rule_set
            .schema_version
            .as_deref()
            .unwrap_or(EDGE_RULE_SET_SCHEMA_VERSION),
        "rules": rules,
    });
    serde_json::from_value::<nrz_contract::EdgeRuleSetAuthoring>(value.clone())
        .context("active Edge Rules cannot be converted to onreza.rules.toml authoring shape")?;
    Ok(value)
}

pub(crate) fn edge_rule_set_authoring_to_toml(value: &Value) -> anyhow::Result<String> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Edge Rules authoring value must be an object"))?;
    let rules = object
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("Edge Rules authoring value must contain rules[]"))?;

    let mut out = String::new();
    if let Some(schema_version) = object.get("schemaVersion") {
        write_assignment(&mut out, "schemaVersion", schema_version)?;
    }
    if let Some(source) = object.get("source") {
        write_assignment(&mut out, "source", source)?;
    }

    for rule in rules {
        let rule = rule
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Edge Rules rule must be an object"))?;
        out.push('\n');
        out.push_str("[[rules]]\n");
        for key in ordered_rule_keys(rule) {
            if key == "position" {
                continue;
            }
            let value = rule.get(key).expect("ordered key must exist");
            write_assignment(&mut out, key, value)?;
        }
    }
    Ok(out)
}

fn ordered_rule_keys(rule: &Map<String, Value>) -> Vec<&str> {
    let preferred = ["id", "name", "enabled", "condition", "action"];
    let mut keys = Vec::with_capacity(rule.len());
    for key in preferred {
        if rule.contains_key(key) {
            keys.push(key);
        }
    }
    for key in rule.keys() {
        let key = key.as_str();
        if key != "position" && !preferred.contains(&key) {
            keys.push(key);
        }
    }
    keys
}

fn write_assignment(out: &mut String, key: &str, value: &Value) -> anyhow::Result<()> {
    out.push_str(&format!(
        "{} = {}\n",
        toml_key(key),
        toml_inline_value(&json_to_toml_value(value)?)?
    ));
    Ok(())
}

fn json_to_toml_value(value: &Value) -> anyhow::Result<toml::Value> {
    match value {
        Value::Null => Err(anyhow::anyhow!("null cannot be represented in TOML")),
        Value::Bool(value) => Ok(toml::Value::Boolean(*value)),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(toml::Value::Integer(value))
            } else if let Some(value) = value.as_u64() {
                let value = i64::try_from(value)
                    .context("TOML integer value exceeds signed 64-bit range")?;
                Ok(toml::Value::Integer(value))
            } else if let Some(value) = value.as_f64() {
                Ok(toml::Value::Float(value))
            } else {
                Err(anyhow::anyhow!("unsupported JSON number"))
            }
        }
        Value::String(value) => Ok(toml::Value::String(value.clone())),
        Value::Array(values) => values
            .iter()
            .map(json_to_toml_value)
            .collect::<anyhow::Result<Vec<_>>>()
            .map(toml::Value::Array),
        Value::Object(values) => {
            let mut table = toml::map::Map::new();
            for (key, value) in values {
                table.insert(key.clone(), json_to_toml_value(value)?);
            }
            Ok(toml::Value::Table(table))
        }
    }
}

fn toml_inline_value(value: &toml::Value) -> anyhow::Result<String> {
    match value {
        toml::Value::Array(values) => {
            let values = values
                .iter()
                .map(toml_inline_value)
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok(format!("[{}]", values.join(", ")))
        }
        toml::Value::Table(table) => {
            let entries = table
                .iter()
                .map(|(key, value)| {
                    Ok(format!("{} = {}", toml_key(key), toml_inline_value(value)?))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok(format!("{{ {} }}", entries.join(", ")))
        }
        toml::Value::String(_)
        | toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_) => Ok(value.to_string()),
    }
}

fn toml_key(key: &str) -> String {
    if key
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        key.to_string()
    } else {
        toml::Value::String(key.to_string()).to_string()
    }
}

pub(crate) fn build_edge_rules_status_request(
    edge_rules: Option<Value>,
    local_invalid: bool,
) -> anyhow::Result<nrz_contract::CliEdgeRulesStatusRequest> {
    let body = match (edge_rules, local_invalid) {
        (Some(_), true) => {
            return Err(anyhow::anyhow!(
                "localInvalid cannot be used with local Edge Rules"
            ));
        }
        (Some(edge_rules), false) => json!({ "edgeRules": edge_rules }),
        (None, true) => json!({ "localInvalid": true }),
        (None, false) => json!({}),
    };
    serde_json::from_value(body)
        .context("ONREZA Edge Rules status request does not match CLI contract")
}

fn load_local_rules_for_status(project_dir: &Path) -> LocalRulesForStatus {
    let path = project_dir.join(RULES_FILENAME).display().to_string();
    match functions::load_edge_rules(project_dir) {
        Ok(Some(edge_rules)) => {
            let rule_count = functions::edge_rule_count(&edge_rules);
            LocalRulesForStatus {
                edge_rules: Some(edge_rules),
                local_invalid: false,
                file: LocalRulesFileStatus {
                    path: Some(path),
                    rule_count: Some(rule_count),
                    valid: true,
                    error: None,
                },
            }
        }
        Ok(None) => LocalRulesForStatus {
            edge_rules: None,
            local_invalid: false,
            file: LocalRulesFileStatus {
                path: Some(path),
                rule_count: None,
                valid: false,
                error: Some("not found".to_string()),
            },
        },
        Err(error) => LocalRulesForStatus {
            edge_rules: None,
            local_invalid: true,
            file: LocalRulesFileStatus {
                path: Some(path),
                rule_count: None,
                valid: false,
                error: Some(format!("{error:#}")),
            },
        },
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveEdgeRuleSetResponse {
    rule_set: Option<ActiveEdgeRuleSet>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActiveEdgeRuleSet {
    #[allow(dead_code)]
    pub(crate) id: String,
    pub(crate) version: i64,
    pub(crate) schema_version: Option<String>,
    pub(crate) source: String,
    pub(crate) rules: Value,
    pub(crate) checksum: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RulesPullOutput {
    path: String,
    environment_id: String,
    rule_count: usize,
    version: i64,
    source: String,
    checksum: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RulesPublishOutput {
    environment_id: String,
    rule_count: usize,
    result: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RulesStatusOutput {
    environment_id: String,
    status: String,
    active: Value,
    local: Value,
    local_file: LocalRulesFileStatus,
}

impl RulesStatusOutput {
    fn from_contract(
        response: nrz_contract::CliEdgeRulesStatusResponse,
        local_file: LocalRulesFileStatus,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            environment_id: response.environment_id.to_string(),
            status: response.status.to_string(),
            active: serde_json::to_value(response.active)
                .context("failed to serialize Edge Rules active status")?,
            local: serde_json::to_value(response.local)
                .context("failed to serialize Edge Rules local status")?,
            local_file,
        })
    }
}

#[derive(Debug)]
struct LocalRulesForStatus {
    edge_rules: Option<Value>,
    local_invalid: bool,
    file: LocalRulesFileStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalRulesFileStatus {
    path: Option<String>,
    rule_count: Option<usize>,
    valid: bool,
    error: Option<String>,
}

impl LocalRulesFileStatus {
    fn summary(&self) -> String {
        if self.valid {
            match (self.path.as_deref(), self.rule_count) {
                (Some(path), Some(count)) => format!("{path} ({count} rule(s), valid)"),
                (Some(path), None) => format!("{path} (valid)"),
                _ => "valid".to_string(),
            }
        } else {
            self.error.as_deref().unwrap_or("invalid").to_string()
        }
    }
}

fn report_status_human(output: &RulesStatusOutput) {
    eprintln!("  Environment: {}", output.environment_id);
    eprintln!("  Status: {}", output.status);
    eprintln!("  Active: {}", format_status_side(&output.active, true));
    eprintln!("  Local: {}", format_status_side(&output.local, false));
    eprintln!("  Local file: {}", output.local_file.summary());
}

fn format_status_side(value: &Value, active: bool) -> String {
    if value
        .get("invalid")
        .and_then(Value::as_bool)
        .is_some_and(|invalid| invalid)
    {
        return "invalid".to_string();
    }

    if value
        .get("present")
        .and_then(Value::as_bool)
        .is_some_and(|present| !present)
    {
        return "absent".to_string();
    }

    let mut parts = Vec::new();
    if let Some(rule_count) = value.get("ruleCount").and_then(Value::as_i64) {
        parts.push(format!("{rule_count} rule(s)"));
    }
    if active {
        if let Some(version) = value.get("version").and_then(Value::as_i64) {
            parts.push(format!("v{version}"));
        }
        if let Some(source) = value.get("source").and_then(Value::as_str) {
            parts.push(source.to_string());
        }
        if let Some(published_at) = value.get("publishedAt").and_then(Value::as_str) {
            parts.push(format!("published {published_at}"));
        }
    }
    if let Some(checksum) = value.get("checksum").and_then(Value::as_str) {
        parts.push(format!("sha256 {checksum}"));
    }
    if parts.is_empty() {
        "present".to_string()
    } else {
        parts.join(", ")
    }
}
