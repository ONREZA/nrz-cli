use serde_json::{Map, Value};

/// Action kinds expressible as `action.<kind>` sugar. The canonical `type` tag
/// value is the kind name verbatim (snake_case), so this list doubles as the
/// allowed discriminant set.
const ACTION_KINDS: &[&str] = &[
    "allow",
    "log",
    "deny",
    "redirect",
    "rewrite",
    "set_headers",
    "remove_headers",
    "cache",
    "bypass_cache",
    "pipeline",
    "rate_limit",
];

/// Tagged path-matcher variants in authoring profile (`when.path = { prefix = … }`).
const PATH_VARIANTS: &[&str] = &["exact", "prefix", "glob", "regex"];

/// Condition fields whose values are user-keyed string maps (HTTP header/query/
/// cookie names). Their inner keys are data, never schema, so the snake→camel
/// pass must leave them byte-for-byte intact.
const OPAQUE_MAP_KEYS: &[&str] = &["headers", "query", "cookies"];

/// Normalize the authoring profile of `onreza.rules.toml` into the canonical
/// `EdgeRuleSetAuthoring` shape: structural sugar (`schema`/`[[rule]]`/`when`/
/// `action.<kind>`/tagged path/named actions) is expanded, then every schema key
/// is mechanically rewritten snake_case→camelCase. The transform is idempotent on
/// already-canonical input, so hand-written canonical files keep round-tripping.
pub fn normalize_authoring_value(raw: &Value) -> anyhow::Result<Value> {
    let Some(root) = raw.as_object() else {
        return Ok(raw.clone());
    };
    let mut out = root.clone();

    if let Some(schema) = out.remove("schema") {
        out.entry("schemaVersion".to_string()).or_insert(schema);
    }

    let named_actions = extract_named_actions(&mut out)?;

    if !out.contains_key("rules")
        && let Some(rule) = out.remove("rule")
    {
        out.insert("rules".to_string(), rule);
    }

    if let Some(rules) = out.get_mut("rules").and_then(Value::as_array_mut) {
        for rule in rules.iter_mut() {
            if let Some(rule) = rule.as_object_mut() {
                normalize_rule(rule, &named_actions)?;
            }
        }
    }

    Ok(snake_to_camel_value(Value::Object(out)))
}

fn extract_named_actions(root: &mut Map<String, Value>) -> anyhow::Result<Map<String, Value>> {
    let Some(actions) = root.remove("actions") else {
        return Ok(Map::new());
    };
    let actions = actions
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("[actions] must be a table of named actions"))?;
    let mut resolved = Map::new();
    for (name, action) in actions {
        let action = action
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("named action '{name}' must be a table"))?;
        // Named actions are reference leaves: passing an empty map rejects any
        // nested `use`, enforcing the one-level bound from the RFC.
        let canonical = canonicalize_action(action, &Map::new(), name)?;
        resolved.insert(name.clone(), canonical);
    }
    Ok(resolved)
}

fn normalize_rule(rule: &mut Map<String, Value>, named: &Map<String, Value>) -> anyhow::Result<()> {
    let rule_id = rule
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("<missing>")
        .to_string();

    if !rule.contains_key("condition")
        && let Some(when) = rule.remove("when")
    {
        rule.insert("condition".to_string(), when);
    }
    if let Some(condition) = rule.get_mut("condition").and_then(Value::as_object_mut) {
        normalize_condition(condition);
    }

    if let Some(action) = rule.get("action").and_then(Value::as_object) {
        let canonical = canonicalize_action(action, named, &rule_id)?;
        rule.insert("action".to_string(), canonical);
    }

    Ok(())
}

fn normalize_condition(condition: &mut Map<String, Value>) {
    normalize_path_in_leaf(condition);
    if let Some(branches) = condition.get_mut("any").and_then(Value::as_array_mut) {
        for branch in branches.iter_mut() {
            if let Some(branch) = branch.as_object_mut() {
                normalize_path_in_leaf(branch);
            }
        }
    }
    if let Some(not) = condition.get_mut("not").and_then(Value::as_object_mut) {
        normalize_path_in_leaf(not);
    }
}

fn normalize_path_in_leaf(leaf: &mut Map<String, Value>) {
    let canonical = {
        let Some(path) = leaf.get("path").and_then(Value::as_object) else {
            return;
        };
        if path.contains_key("type") {
            return;
        }
        let mut found = None;
        for variant in PATH_VARIANTS {
            if let Some(value) = path.get(*variant) {
                found = Some(serde_json::json!({ "type": variant, "value": value.clone() }));
                break;
            }
        }
        match found {
            Some(canonical) => canonical,
            None => return,
        }
    };
    leaf.insert("path".to_string(), canonical);
}

fn canonicalize_action(
    action: &Map<String, Value>,
    named: &Map<String, Value>,
    rule_id: &str,
) -> anyhow::Result<Value> {
    if action.contains_key("type") {
        return Ok(Value::Object(action.clone()));
    }
    if action.len() != 1 {
        return Err(anyhow::anyhow!(
            "edge rule '{rule_id}' action must declare exactly one action.<kind>"
        ));
    }
    let (key, value) = action.iter().next().expect("action has exactly one entry");
    if key == "use" {
        let name = value.as_str().ok_or_else(|| {
            anyhow::anyhow!("edge rule '{rule_id}' action.use must name a reusable action")
        })?;
        return named.get(name).cloned().ok_or_else(|| {
            anyhow::anyhow!("edge rule '{rule_id}' references unknown named action '{name}'")
        });
    }
    if !ACTION_KINDS.contains(&key.as_str()) {
        return Err(anyhow::anyhow!(
            "edge rule '{rule_id}' has unknown action kind '{key}'"
        ));
    }
    let mut canonical = value.as_object().cloned().unwrap_or_default();
    canonical.insert("type".to_string(), Value::String(key.clone()));
    Ok(Value::Object(canonical))
}

fn snake_to_camel_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, child) in map {
                let opaque = OPAQUE_MAP_KEYS.contains(&key.as_str());
                let child = if opaque {
                    child
                } else {
                    snake_to_camel_value(child)
                };
                out.insert(snake_to_camel_key(&key), child);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(snake_to_camel_value).collect()),
        other => other,
    }
}

fn snake_to_camel_key(key: &str) -> String {
    if !key.contains('_') {
        return key.to_string();
    }
    let mut out = String::with_capacity(key.len());
    let mut upgrade_next = false;
    for ch in key.chars() {
        if ch == '_' {
            upgrade_next = true;
        } else if upgrade_next {
            out.extend(ch.to_uppercase());
            upgrade_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}
