use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Map, Value};

use super::rules_authoring;

const RULES_FILENAME: &str = "onreza.rules.toml";
const RATE_LIMIT_MAX_REQUESTS: u64 = 100_000;
const RATE_LIMIT_MIN_WINDOW_SECONDS: u64 = 10;
const RATE_LIMIT_MAX_WINDOW_SECONDS: u64 = 600;
const REDIRECT_STATUS_CODES: [u64; 4] = [301, 302, 307, 308];
const REDIRECT_TARGET_MESSAGE: &str =
    "redirect target must be a relative path or an absolute http(s) URL";
const INTERNAL_REWRITE_TARGET_MESSAGE: &str = "internal rewrite target must be a relative path";
const EXTERNAL_REWRITE_TARGET_MESSAGE: &str =
    "external rewrite target must be an absolute https URL";

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

    let raw: Value = toml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("failed to parse {} as TOML: {error}", path.display()))?;
    let value = rules_authoring::normalize_authoring_value(&raw)
        .map_err(|error| anyhow::anyhow!("{}: {error}", path.display()))?;
    validate_edge_rules_authoring_value(&path, &value)?;
    Ok(Some((path, value)))
}

fn validate_edge_rules_authoring_value(path: &Path, value: &Value) -> anyhow::Result<()> {
    reject_authored_positions(path, value)?;

    let authoring_error =
        match serde_json::from_value::<nrz_contract::EdgeRuleSetAuthoring>(value.clone()) {
            Ok(_) => {
                validate_edge_rules_refinements(path, value)?;
                return Ok(());
            }
            Err(error) => error,
        };

    let candidate = edge_rules_contract_validation_value(value);
    serde_json::from_value::<nrz_contract::EdgeRuleSetAuthoring>(candidate).map_err(
        |fallback_error| {
            let error = if is_fallback_position_error(&fallback_error) {
                authoring_error
            } else {
                fallback_error
            };
            anyhow::anyhow!(
                "{} does not match the EdgeRuleSetAuthoring contract: {error}",
                path.display()
            )
        },
    )?;
    validate_edge_rules_refinements(path, value)?;
    Ok(())
}

pub(crate) fn validate_edge_rules_value(label: &str, value: &Value) -> anyhow::Result<()> {
    validate_edge_rules_authoring_value(Path::new(label), value)
}

fn is_fallback_position_error(error: &serde_json::Error) -> bool {
    let message = error.to_string();
    message.contains("unknown field `position`") || message.contains("unknown field \"position\"")
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
        let action_type = action.get("type").and_then(Value::as_str);
        if action_type == Some("rate_limit") {
            validate_rate_limit_bounds(path, rule_id, action)?;
        }
        if action_type == Some("redirect") {
            validate_redirect_status(path, rule_id, action)?;
        }
        validate_action_target(path, rule_id, action)?;
        if action_type == Some("pipeline") {
            validate_pipeline_action(path, rule_id, action)?;
        }
        validate_path_captures(path, rule_id, rule)?;
        if action_type != Some("cache") {
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
    validate_pipeline_security_gate_shadowing(path, rules)?;

    Ok(())
}

/// The generated contract types rate_limit `limit`/`windowSeconds` as plain
/// integers (typify emits no range validation), so bounds must be checked here to
/// match the platform's authoring schema and fail fast in `nrz functions check`.
fn validate_rate_limit_bounds(
    path: &Path,
    rule_id: &str,
    action: &Map<String, Value>,
) -> anyhow::Result<()> {
    let limit = action.get("limit").and_then(Value::as_u64);
    if !limit.is_some_and(|limit| (1..=RATE_LIMIT_MAX_REQUESTS).contains(&limit)) {
        return Err(anyhow::anyhow!(
            "{} rate_limit rule '{rule_id}' limit must be an integer between 1 and {RATE_LIMIT_MAX_REQUESTS}",
            path.display()
        ));
    }
    let window = action.get("windowSeconds").and_then(Value::as_u64);
    if !window.is_some_and(|window| {
        (RATE_LIMIT_MIN_WINDOW_SECONDS..=RATE_LIMIT_MAX_WINDOW_SECONDS).contains(&window)
    }) {
        return Err(anyhow::anyhow!(
            "{} rate_limit rule '{rule_id}' windowSeconds must be an integer between {RATE_LIMIT_MIN_WINDOW_SECONDS} and {RATE_LIMIT_MAX_WINDOW_SECONDS}",
            path.display()
        ));
    }
    Ok(())
}

/// The generated contract types redirect `statusCode` as a plain integer (typify
/// emits no value constraint for it), so the allowed set must be checked here to
/// match the platform's authoring schema and fail fast in `nrz functions check`.
fn validate_redirect_status(
    path: &Path,
    rule_id: &str,
    action: &Map<String, Value>,
) -> anyhow::Result<()> {
    let Some(status) = action.get("statusCode") else {
        return Ok(());
    };
    if !status
        .as_u64()
        .is_some_and(|code| REDIRECT_STATUS_CODES.contains(&code))
    {
        return Err(anyhow::anyhow!(
            "{} redirect rule '{rule_id}' statusCode must be one of 301, 302, 307, 308",
            path.display()
        ));
    }
    Ok(())
}

fn validate_action_target(
    path: &Path,
    rule_id: &str,
    action: &Map<String, Value>,
) -> anyhow::Result<()> {
    let Some(action_type) = action.get("type").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(target) = action.get("target").and_then(Value::as_str) else {
        return Ok(());
    };

    let error = match action_type {
        "redirect" if !valid_redirect_target(target) => Some(REDIRECT_TARGET_MESSAGE),
        "rewrite"
            if action.get("external").and_then(Value::as_bool) == Some(true)
                && !valid_absolute_url_target(target, &["https"]) =>
        {
            Some(EXTERNAL_REWRITE_TARGET_MESSAGE)
        }
        "rewrite"
            if action.get("external").and_then(Value::as_bool) != Some(true)
                && !valid_internal_rewrite_target(target) =>
        {
            Some(INTERNAL_REWRITE_TARGET_MESSAGE)
        }
        _ => None,
    };
    if let Some(error) = error {
        return Err(anyhow::anyhow!(
            "{} rule '{rule_id}' {error}",
            path.display()
        ));
    }
    Ok(())
}

fn valid_redirect_target(target: &str) -> bool {
    let target = target.trim();
    if target.is_empty() || target.chars().any(char::is_whitespace) {
        return false;
    }
    valid_absolute_url_target(target, &["http", "https"])
        || (!has_unsafe_relative_target_prefix(target))
}

fn valid_internal_rewrite_target(target: &str) -> bool {
    let target = target.trim();
    !target.is_empty()
        && !target.chars().any(char::is_whitespace)
        && !has_unsafe_relative_target_prefix(target)
        && !target.contains('#')
}

fn has_unsafe_relative_target_prefix(target: &str) -> bool {
    target.starts_with("//") || target.contains("://") || has_url_scheme_like_prefix(target)
}

fn has_url_scheme_like_prefix(target: &str) -> bool {
    let Some((scheme, _)) = target.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}

fn valid_absolute_url_target(target: &str, protocols: &[&str]) -> bool {
    let target = target.trim();
    if target.is_empty() || target.chars().any(char::is_whitespace) {
        return false;
    }
    let lower = target.to_ascii_lowercase();
    if !protocols
        .iter()
        .any(|protocol| lower.starts_with(&format!("{protocol}://")))
    {
        return false;
    }
    let Ok(url) = url::Url::parse(target) else {
        return false;
    };
    protocols.contains(&url.scheme())
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some_and(|host| !host.is_empty())
}

fn validate_pipeline_action(
    path: &Path,
    rule_id: &str,
    action: &Map<String, Value>,
) -> anyhow::Result<()> {
    let Some(steps) = action.get("steps").and_then(Value::as_array) else {
        return Err(anyhow::anyhow!(
            "{} pipeline rule '{rule_id}' must declare steps",
            path.display()
        ));
    };

    let terminal_indexes = steps
        .iter()
        .enumerate()
        .filter_map(|(index, step)| {
            step.as_object()
                .and_then(|step| step.contains_key("handle").then_some(index))
        })
        .collect::<Vec<_>>();
    if terminal_indexes.len() != 1 {
        return Err(anyhow::anyhow!(
            "{} pipeline rule '{rule_id}' must declare exactly one terminal handle step",
            path.display()
        ));
    }

    let terminal_index = terminal_indexes[0];
    let terminal_handle = steps[terminal_index]
        .as_object()
        .and_then(|step| step.get("handle"))
        .and_then(Value::as_str);
    if terminal_handle.is_some_and(|handle| handle != "@app")
        && action.get("override").and_then(Value::as_bool) != Some(true)
    {
        return Err(anyhow::anyhow!(
            "{} pipeline rule '{rule_id}' function terminal must set override = true",
            path.display()
        ));
    }
    for (index, step) in steps.iter().enumerate() {
        let Some(step) = step.as_object() else {
            continue;
        };
        let Some(mode) = step.get("mode").and_then(Value::as_str) else {
            continue;
        };
        if mode == "request" && index > terminal_index {
            return Err(anyhow::anyhow!(
                "{} pipeline rule '{rule_id}' request step at steps[{index}] must appear before the terminal handle",
                path.display()
            ));
        }
        if matches!(mode, "response" | "observe") && index < terminal_index {
            return Err(anyhow::anyhow!(
                "{} pipeline rule '{rule_id}' {mode} step at steps[{index}] must appear after the terminal handle",
                path.display()
            ));
        }
    }

    Ok(())
}

/// Path captures are a refinement on top of the generated contract (plain
/// strings to typify), so they must be checked here to match the platform's
/// authoring schema and fail fast in `nrz functions check`: `{name}`/`{name...}`
/// captures live only in the root glob `condition.path`; redirect/rewrite
/// `target` and `set_headers` values reference them as `{name}` (splats too —
/// without `...`); `{{name}}` escapes interpolation and emits the literal token.
fn validate_path_captures(
    path: &Path,
    rule_id: &str,
    rule: &Map<String, Value>,
) -> anyhow::Result<()> {
    let condition = rule.get("condition").and_then(Value::as_object);
    let defined = validate_glob_capture_syntax(
        path,
        rule_id,
        condition.and_then(|condition| condition.get("path")),
        true,
    )?;
    if let Some(branches) = condition
        .and_then(|condition| condition.get("any"))
        .and_then(Value::as_array)
    {
        for branch in branches {
            let branch_path = branch.as_object().and_then(|leaf| leaf.get("path"));
            validate_glob_capture_syntax(path, rule_id, branch_path, false)?;
        }
    }
    let not_path = condition
        .and_then(|condition| condition.get("not"))
        .and_then(Value::as_object)
        .and_then(|leaf| leaf.get("path"));
    validate_glob_capture_syntax(path, rule_id, not_path, false)?;

    let Some(action) = rule.get("action").and_then(Value::as_object) else {
        return Ok(());
    };
    let mut references = Vec::new();
    match action.get("type").and_then(Value::as_str) {
        Some("redirect" | "rewrite") => {
            if let Some(target) = action.get("target").and_then(Value::as_str) {
                collect_capture_references(target, &mut references);
            }
        }
        Some("set_headers") => {
            if let Some(headers) = action.get("headers").and_then(Value::as_object) {
                for value in headers.values().filter_map(Value::as_str) {
                    collect_capture_references(value, &mut references);
                }
            }
        }
        _ => {}
    }
    for (name, splat) in references {
        if splat {
            return Err(anyhow::anyhow!(
                "{} rule '{rule_id}' must reference splat capture '{{{name}...}}' by plain '{{{name}}}'",
                path.display()
            ));
        }
        if !defined.contains(&name) {
            return Err(anyhow::anyhow!(
                "{} rule '{rule_id}' references undefined path capture '{{{name}}}'; declare it in a glob path matcher",
                path.display()
            ));
        }
    }
    Ok(())
}

fn validate_glob_capture_syntax(
    path: &Path,
    rule_id: &str,
    path_condition: Option<&Value>,
    allow_captures: bool,
) -> anyhow::Result<HashSet<String>> {
    let mut names = HashSet::new();
    let Some(path_condition) = path_condition.and_then(Value::as_object) else {
        return Ok(names);
    };
    if path_condition.get("type").and_then(Value::as_str) != Some("glob") {
        return Ok(names);
    }
    let Some(value) = path_condition.get("value").and_then(Value::as_str) else {
        return Ok(names);
    };

    let mut splats = 0;
    let mut rest = value;
    while let Some(offset) = rest.find(['{', '}']) {
        let tail = &rest[offset..];
        let Some((name, splat, consumed)) = parse_capture_token(tail) else {
            return Err(anyhow::anyhow!(
                "{} rule '{rule_id}' glob path has a malformed capture token; use '{{name}}' for a segment or '{{name...}}' for the remainder",
                path.display()
            ));
        };
        if !allow_captures {
            return Err(anyhow::anyhow!(
                "{} rule '{rule_id}' declares path capture '{{{name}}}' in an any/not branch; captures are only supported in the rule's root condition.path",
                path.display()
            ));
        }
        if !names.insert(name.to_string()) {
            return Err(anyhow::anyhow!(
                "{} rule '{rule_id}' declares duplicate path capture '{{{name}}}'",
                path.display()
            ));
        }
        if splat {
            splats += 1;
            if splats > 1 {
                return Err(anyhow::anyhow!(
                    "{} rule '{rule_id}' path may declare at most one splat capture '{{name...}}'",
                    path.display()
                ));
            }
        }
        rest = &tail[consumed..];
    }
    Ok(names)
}

// Parses `{name}` / `{name...}` at the start of `tail`. Returns
// `(name, is_splat, bytes_consumed)` or `None` when `tail` does not begin with a
// well-formed capture token.
fn parse_capture_token(tail: &str) -> Option<(&str, bool, usize)> {
    let body = tail.strip_prefix('{')?;
    let bytes = body.as_bytes();
    let mut name_len = 0;
    while name_len < bytes.len()
        && (bytes[name_len].is_ascii_alphanumeric() || bytes[name_len] == b'_')
    {
        name_len += 1;
    }
    if name_len == 0 || !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    let name = &body[..name_len];
    let after_name = &body[name_len..];
    if let Some(after_splat) = after_name.strip_prefix("...") {
        after_splat.strip_prefix('}')?;
        Some((name, true, 1 + name_len + 4))
    } else {
        after_name.strip_prefix('}')?;
        Some((name, false, 1 + name_len + 1))
    }
}

fn collect_capture_references(value: &str, references: &mut Vec<(String, bool)>) {
    let mut rest = value;
    while let Some(offset) = rest.find('{') {
        let tail = &rest[offset..];
        if let Some(remainder) = strip_escaped_capture_token(tail) {
            rest = remainder;
            continue;
        }
        if let Some((name, splat, consumed)) = parse_capture_token(tail) {
            references.push((name.to_string(), splat));
            rest = &tail[consumed..];
        } else {
            rest = &tail[1..];
        }
    }
}

// `{{name}}` / `{{name...}}` escapes interpolation; returns the text after the
// escape or `None` when `tail` is not an escaped token.
fn strip_escaped_capture_token(tail: &str) -> Option<&str> {
    let body = tail.strip_prefix('{')?;
    let (_, _, consumed) = parse_capture_token(body)?;
    body[consumed..].strip_prefix('}')
}

fn validate_pipeline_security_gate_shadowing(path: &Path, rules: &[Value]) -> anyhow::Result<()> {
    for (broader_index, broader) in rules.iter().enumerate() {
        let Some(broader) = broader.as_object() else {
            continue;
        };
        let broader_id = rule_id(broader);
        let Some(broader_action) = broader.get("action").and_then(Value::as_object) else {
            continue;
        };
        if broader_action.get("type").and_then(Value::as_str) != Some("pipeline") {
            continue;
        }
        let broader_gates = pipeline_security_gate_uses(broader_action);
        if broader_gates.is_empty() {
            continue;
        }

        for (narrower_index, narrower) in rules.iter().enumerate() {
            if narrower_index == broader_index {
                continue;
            }
            let Some(narrower) = narrower.as_object() else {
                continue;
            };
            let narrower_id = rule_id(narrower);
            let Some(narrower_action) = narrower.get("action").and_then(Value::as_object) else {
                continue;
            };
            if narrower_action.get("type").and_then(Value::as_str) != Some("pipeline") {
                continue;
            }
            if narrower_action.get("inheritGate").and_then(Value::as_bool) == Some(false) {
                continue;
            }
            if !is_more_specific_path_within(narrower.get("condition"), broader.get("condition")) {
                continue;
            }

            let narrower_gates = pipeline_security_gate_uses(narrower_action);
            for gate in &broader_gates {
                if narrower_gates.contains(gate) {
                    continue;
                }
                return Err(anyhow::anyhow!(
                    "{} pipeline rule '{narrower_id}' shadows security gate '{gate}' from broader rule '{broader_id}'; re-declare the gate or set inherit_gate = false",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

fn rule_id(rule: &Map<String, Value>) -> &str {
    rule.get("id")
        .and_then(Value::as_str)
        .unwrap_or("<missing>")
}

fn pipeline_security_gate_uses(action: &Map<String, Value>) -> HashSet<String> {
    action
        .get("steps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .filter_map(|step| {
            let function_name = step.get("use").and_then(Value::as_str)?;
            let mode = step.get("mode").and_then(Value::as_str)?;
            if mode != "request" {
                return None;
            }
            let failure = step
                .get("failure")
                .and_then(Value::as_str)
                .unwrap_or("closed");
            let cache_position = step.get("cachePosition").and_then(Value::as_str);
            if failure == "closed" && cache_position.unwrap_or("before") == "before" {
                Some(function_name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn is_more_specific_path_within(
    maybe_narrower_condition: Option<&Value>,
    maybe_broader_condition: Option<&Value>,
) -> bool {
    let narrower_path = maybe_narrower_condition
        .and_then(Value::as_object)
        .and_then(|condition| condition.get("path"));
    let broader_path = maybe_broader_condition
        .and_then(Value::as_object)
        .and_then(|condition| condition.get("path"));
    match (narrower_path, broader_path) {
        (Some(_), None) => true,
        (Some(narrower), Some(broader)) => {
            let Some(narrower_prefix) = static_path_prefix(narrower) else {
                return false;
            };
            let Some(broader_prefix) = static_path_prefix(broader) else {
                return false;
            };
            if narrower_prefix == broader_prefix {
                return path_specificity(narrower) > path_specificity(broader);
            }
            narrower_prefix.starts_with(&broader_prefix)
        }
        _ => false,
    }
}

fn path_specificity(path: &Value) -> usize {
    let prefix = static_path_prefix(path).unwrap_or_default();
    let kind_weight = match path.get("type").and_then(Value::as_str).unwrap_or_default() {
        "exact" => 3,
        "prefix" => 2,
        "glob" => 1,
        _ => 0,
    };
    prefix.len() * 10 + kind_weight
}

fn static_path_prefix(path: &Value) -> Option<String> {
    let path = path.as_object()?;
    let kind = path.get("type").and_then(Value::as_str)?;
    let value = path.get("value").and_then(Value::as_str)?;
    match kind {
        "exact" | "prefix" => Some(value.to_string()),
        "glob" => {
            let wildcard_index = value.find(['*', '?', '[', '{']).unwrap_or(value.len());
            Some(value[..wildcard_index].to_string())
        }
        _ => None,
    }
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
                .entry("ifNoFile".to_string())
                .or_insert(Value::Bool(true));
        }
        Some("rewrite") => {
            action
                .entry("external".to_string())
                .or_insert(Value::Bool(false));
            action
                .entry("ifNoFile".to_string())
                .or_insert(Value::Bool(true));
        }
        Some("cache") => {
            action
                .entry("vary".to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
        }
        _ => {}
    }
}

/// Request-dependent dimensions a cache rule must vary by, collected across the
/// root condition plus every `any` branch and the `not` branch. Mirrors the Zod
/// `requestDependentVaryDimensions`; an axis missed here would let one client's
/// cached response leak to another (cache poisoning).
fn request_dependent_vary_dimensions(condition: Option<&Value>) -> Vec<&'static str> {
    let Some(condition) = condition.and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut dimensions = Vec::new();
    collect_leaf_vary_dimensions(condition, &mut dimensions);
    if let Some(branches) = condition.get("any").and_then(Value::as_array) {
        for branch in branches {
            if let Some(branch) = branch.as_object() {
                collect_leaf_vary_dimensions(branch, &mut dimensions);
            }
        }
    }
    if let Some(not) = condition.get("not").and_then(Value::as_object) {
        collect_leaf_vary_dimensions(not, &mut dimensions);
    }
    dimensions
}

fn collect_leaf_vary_dimensions(leaf: &Map<String, Value>, dimensions: &mut Vec<&'static str>) {
    let mut push = |dimension: &'static str| {
        if !dimensions.contains(&dimension) {
            dimensions.push(dimension);
        }
    };
    for (field, dimension) in [("geo", "geo"), ("asn", "asn")] {
        if leaf
            .get(field)
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty())
        {
            push(dimension);
        }
    }
    if leaf.get("device").is_some() {
        push("device");
    }
    for (field, dimension) in [
        ("headers", "header"),
        ("cookies", "cookie"),
        ("query", "query"),
    ] {
        if leaf
            .get(field)
            .and_then(Value::as_object)
            .is_some_and(|map| !map.is_empty())
        {
            push(dimension);
        }
    }
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
