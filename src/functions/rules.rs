use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Map, Value};

const RULES_FILENAME: &str = "onreza.rules.toml";

/// Load `onreza.rules.toml` (sibling of `onreza.toml`) as a JSON value.
///
/// The CLI is pure transport: it parses the declarative TOML into JSON and ships
/// it as `edgeRules`. The platform assigns rule positions by `[[rules]]` order and
/// validates against the `EdgeRuleSetAuthoring` contract (the authoritative trust
/// boundary), so the CLI does not apply authoring defaults itself.
pub fn load_edge_rules(project_dir: &Path) -> anyhow::Result<Option<Value>> {
    Ok(read_edge_rules_value(project_dir)?.map(|(_, value)| value))
}

pub fn check_edge_rules(project_dir: &Path) -> anyhow::Result<Option<EdgeRulesCheckReport>> {
    let Some((path, value)) = read_edge_rules_value(project_dir)? else {
        return Ok(None);
    };
    Ok(Some(build_edge_rules_check_report(&path, &value)))
}

pub fn edge_rule_count(value: &Value) -> usize {
    value
        .get("rules")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeRulesCheckReport {
    pub path: String,
    pub rule_count: usize,
    pub rules: Vec<EdgeRuleCheckItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeRuleCheckItem {
    pub id: String,
    pub position: usize,
    pub action: String,
    pub enabled: bool,
}

fn read_edge_rules_value(project_dir: &Path) -> anyhow::Result<Option<(PathBuf, Value)>> {
    let path = project_dir.join(RULES_FILENAME);
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(anyhow::anyhow!(
                "failed to read {}: {error}",
                path.display()
            ));
        }
    };

    let value: Value = toml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("failed to parse {} as TOML: {error}", path.display()))?;
    validate_edge_rules_authoring_value(&path, &value)?;
    Ok(Some((path, value)))
}

fn validate_edge_rules_authoring_value(path: &Path, value: &Value) -> anyhow::Result<()> {
    reject_authored_positions(path, value)?;

    if serde_json::from_value::<nrz_contract::EdgeRuleSetAuthoring>(value.clone()).is_ok() {
        validate_edge_rules_refinements(path, value)?;
        return Ok(());
    }

    let candidate = edge_rules_contract_validation_value(value);
    serde_json::from_value::<nrz_contract::EdgeRuleSetAuthoring>(candidate).map_err(|error| {
        anyhow::anyhow!(
            "{} does not match the EdgeRuleSetAuthoring contract: {error}",
            path.display()
        )
    })?;
    validate_edge_rules_refinements(path, value)?;
    Ok(())
}

fn reject_authored_positions(path: &Path, value: &Value) -> anyhow::Result<()> {
    let Some(rules) = value.get("rules").and_then(Value::as_array) else {
        return Ok(());
    };

    for (index, rule) in rules.iter().enumerate() {
        let Some(rule) = rule.as_object() else {
            continue;
        };
        if rule.contains_key("position") {
            return Err(anyhow::anyhow!(
                "{} uses rules[{index}].position, but position is derived from [[rules]] order; remove the field",
                path.display()
            ));
        }
    }

    Ok(())
}

fn validate_edge_rules_refinements(path: &Path, value: &Value) -> anyhow::Result<()> {
    let normalized = edge_rules_contract_validation_value(value);
    let Some(rules) = normalized.get("rules").and_then(Value::as_array) else {
        return Ok(());
    };

    let mut seen_ids = HashSet::new();
    for (index, rule) in rules.iter().enumerate() {
        let Some(rule) = rule.as_object() else {
            continue;
        };
        let rule_id = rule
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>");
        if !seen_ids.insert(rule_id.to_string()) {
            return Err(anyhow::anyhow!(
                "{} has duplicate edge rule id at rules[{index}].id: {rule_id}",
                path.display()
            ));
        }

        let Some(action) = rule.get("action").and_then(Value::as_object) else {
            continue;
        };
        if action.get("type").and_then(Value::as_str) != Some("cache") {
            continue;
        }

        let vary = action
            .get("vary")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();
        for dimension in request_dependent_vary_dimensions(rule.get("condition")) {
            if !vary.contains(dimension) {
                return Err(anyhow::anyhow!(
                    "{} cache rule '{rule_id}' must vary by {dimension}",
                    path.display()
                ));
            }
        }
    }

    Ok(())
}

fn edge_rules_contract_validation_value(value: &Value) -> Value {
    let mut candidate = value.clone();
    let Some(rules) = candidate.get_mut("rules").and_then(Value::as_array_mut) else {
        return candidate;
    };

    for (index, rule) in rules.iter_mut().enumerate() {
        let Some(rule) = rule.as_object_mut() else {
            continue;
        };
        rule.insert("position".to_string(), Value::from(index as u64));
        rule.entry("enabled").or_insert(Value::Bool(true));
        rule.entry("condition")
            .or_insert_with(|| Value::Object(Map::new()));
        normalize_edge_rule_action_authoring_value(rule);
    }

    candidate
}

fn normalize_edge_rule_action_authoring_value(rule: &mut Map<String, Value>) {
    let Some(action) = rule.get_mut("action").and_then(Value::as_object_mut) else {
        return;
    };

    match action.get("type").and_then(Value::as_str) {
        Some("deny") => {
            action
                .entry("statusCode".to_string())
                .or_insert(Value::from(403_u16));
            action
                .entry("mode".to_string())
                .or_insert(Value::String("enforce".to_string()));
        }
        Some("redirect") => {
            action
                .entry("statusCode".to_string())
                .or_insert(Value::from(301_u16));
            action
                .entry("force".to_string())
                .or_insert(Value::Bool(false));
        }
        Some("rewrite") => {
            action
                .entry("external".to_string())
                .or_insert(Value::Bool(false));
            action
                .entry("force".to_string())
                .or_insert(Value::Bool(false));
        }
        Some("cache") => {
            action
                .entry("vary".to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
        }
        _ => {}
    }
}

fn request_dependent_vary_dimensions(condition: Option<&Value>) -> Vec<&'static str> {
    let Some(condition) = condition.and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut dimensions = Vec::new();
    if condition
        .get("geo")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
    {
        dimensions.push("geo");
    }
    if condition.get("device").is_some() {
        dimensions.push("device");
    }
    if condition
        .get("headers")
        .and_then(Value::as_object)
        .is_some_and(|headers| !headers.is_empty())
    {
        dimensions.push("header");
    }
    if condition
        .get("cookies")
        .and_then(Value::as_object)
        .is_some_and(|cookies| !cookies.is_empty())
    {
        dimensions.push("cookie");
    }
    if condition
        .get("query")
        .and_then(Value::as_object)
        .is_some_and(|query| !query.is_empty())
    {
        dimensions.push("query");
    }
    dimensions
}

fn build_edge_rules_check_report(path: &Path, value: &Value) -> EdgeRulesCheckReport {
    let normalized = edge_rules_contract_validation_value(value);
    let rules = normalized
        .get("rules")
        .and_then(Value::as_array)
        .map(|rules| {
            rules
                .iter()
                .enumerate()
                .filter_map(|(fallback_position, rule)| {
                    let rule = rule.as_object()?;
                    let action = rule.get("action")?.as_object()?;
                    Some(EdgeRuleCheckItem {
                        id: rule
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or("<missing>")
                            .to_string(),
                        position: rule
                            .get("position")
                            .and_then(Value::as_u64)
                            .map_or(fallback_position, |position| position as usize),
                        action: action
                            .get("type")
                            .and_then(Value::as_str)
                            .unwrap_or("<missing>")
                            .to_string(),
                        enabled: rule.get("enabled").and_then(Value::as_bool).unwrap_or(true),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    EdgeRulesCheckReport {
        path: path.display().to_string(),
        rule_count: edge_rule_count(value),
        rules,
    }
}
