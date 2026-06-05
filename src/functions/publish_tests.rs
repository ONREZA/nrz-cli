use std::fs;
use std::path::Path;

use super::{build_payload, check_edge_rules, collect, load_edge_rules};

fn write(dir: &Path, rel: &str, contents: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn build_payload_maps_config_triggers_to_wire() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "functions/billing-webhook.nrz-fn.ts",
        r#"
export const config = {
  triggers: [{
    name: "api",
    type: "http",
    matchers: ["^/api/.*$"],
    methods: ["get"],
    on_failure: "fail_closed",
    priority: 5,
    config: { override: true },
  }],
} as const;
export default {};
"#,
    );
    let collected = collect(tmp.path()).unwrap();

    let payload = build_payload("DEPLOYMENT", &collected, None);
    let value = serde_json::to_value(&payload).unwrap();

    assert_eq!(value["origin"], "DEPLOYMENT");
    let spec = &value["functions"][0];
    assert!(spec.get("name").is_none());
    assert!(spec.get("entrypoint").is_none());
    assert!(spec.get("sources").is_none());
    assert_eq!(
        spec["source"]["path"],
        "functions/billing-webhook.nrz-fn.ts"
    );

    assert!(spec.get("triggers").is_none());
}

#[test]
fn load_edge_rules_absent_is_none() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(load_edge_rules(tmp.path()).unwrap().is_none());
}

#[test]
fn load_edge_rules_parses_toml_array_of_tables() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "old-docs"
condition.path = { type = "prefix", value = "/old-docs" }
action = { type = "redirect", target = "/docs" }

[[rules]]
id = "api-cache"
condition.path = { type = "prefix", value = "/api" }
action = { type = "cache", ttlSeconds = 60 }
"#,
    );

    let value = load_edge_rules(tmp.path()).unwrap().unwrap();
    assert_eq!(value["schemaVersion"], "EDGE_RULE_SET_V1");
    assert_eq!(value["source"]["origin"], "build");
    // `[[rules]]` array-of-tables preserves order → platform assigns `position`.
    assert_eq!(value["rules"][0]["id"], "old-docs");
    assert!(value["rules"][0].get("position").is_none());
    assert!(value["rules"][0].get("enabled").is_none());
    assert_eq!(value["rules"][0]["action"]["type"], "redirect");
    assert_eq!(value["rules"][0]["action"]["target"], "/docs");
    assert!(value["rules"][0]["action"].get("force").is_none());
    assert_eq!(value["rules"][1]["action"]["type"], "cache");
    assert_eq!(value["rules"][1]["action"]["ttlSeconds"], 60);
    assert!(value["rules"][1]["action"].get("vary").is_none());

    let report = check_edge_rules(tmp.path()).unwrap().unwrap();
    assert_eq!(report.rule_count, 2);
    assert_eq!(report.rules[0].id, "old-docs");
    assert_eq!(report.rules[0].position, 0);
    assert_eq!(report.rules[0].action, "redirect");
}

#[test]
fn load_edge_rules_rejects_non_contract_toml() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "broken"
action = { type = "unknown" }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("does not match the EdgeRuleSetAuthoring contract")
    );
}

#[test]
fn load_edge_rules_rejects_authored_position() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "old-docs"
position = 10
action = { type = "redirect", target = "/docs" }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(error.to_string().contains("position is derived"));
}

#[test]
fn load_edge_rules_rejects_duplicate_rule_ids() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "duplicate"
action = { type = "redirect", target = "/docs" }

[[rules]]
id = "duplicate"
action = { type = "allow" }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(error.to_string().contains("duplicate edge rule id"));
}

#[test]
fn load_edge_rules_rejects_cache_rule_missing_request_vary() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "query-cache"
condition.query = { preview = "1" }
action = { type = "cache", ttlSeconds = 60 }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(error.to_string().contains("must vary by query"));
}

#[test]
fn build_payload_can_publish_edge_rules_without_functions() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "old-docs"
condition.path = { type = "prefix", value = "/old-docs" }
action = { type = "redirect", target = "/docs" }
"#,
    );
    let collected = collect(tmp.path()).unwrap();
    let edge_rules = load_edge_rules(tmp.path()).unwrap();

    let payload = build_payload("DEPLOYMENT", &collected, edge_rules);
    let value = serde_json::to_value(&payload).unwrap();

    assert!(value["functions"].as_array().unwrap().is_empty());
    assert_eq!(value["edgeRules"]["rules"][0]["id"], "old-docs");
    assert!(value["edgeRules"]["rules"][0].get("position").is_none());
}

#[test]
fn check_edge_rules_reports_rules_without_functions() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "old-docs"
action = { type = "redirect", target = "/docs" }
"#,
    );

    let report = check_edge_rules(tmp.path()).unwrap().unwrap();
    assert_eq!(report.rule_count, 1);
    assert_eq!(report.rules[0].id, "old-docs");
    assert_eq!(report.rules[0].position, 0);
    assert_eq!(report.rules[0].action, "redirect");
    assert!(report.rules[0].enabled);
}
