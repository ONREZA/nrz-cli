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
  name: "billing-webhook",
} as const;
export default {};
"#,
    );
    let collected = collect(tmp.path()).unwrap();

    let payload = build_payload("DEPLOYMENT", &collected, None, false, Vec::new());
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
    assert!(value.get("source").is_none());
    // `[[rules]]` array-of-tables preserves order → platform assigns `position`.
    assert_eq!(value["rules"][0]["id"], "old-docs");
    assert!(value["rules"][0].get("position").is_none());
    assert!(value["rules"][0].get("enabled").is_none());
    assert_eq!(value["rules"][0]["action"]["type"], "redirect");
    assert_eq!(value["rules"][0]["action"]["target"], "/docs");
    assert!(value["rules"][0]["action"].get("ifNoFile").is_none());
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
fn load_edge_rules_parses_high_value_authoring_surface() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "segmented-cache"
condition.path = { type = "glob", value = "/reports/*" }
condition.methods = ["GET", "HEAD"]
condition.host = "app.example.test"
condition.headers = { "x-plan" = "pro" }
condition.query = { preview = "1" }
condition.cookies = { bucket = "b" }
condition.geo = ["US"]
condition.device = "mobile"
condition.sourceIpCidrs = ["203.0.113.0/24"]
action = { type = "cache", ttlSeconds = 60, swrSeconds = 30, vary = ["header", "query", "cookie", "geo", "device"] }

[[rules]]
id = "internal-rewrite"
condition.path = { type = "exact", value = "/legacy" }
action = { type = "rewrite", target = "/modern", ifNoFile = true }

[[rules]]
id = "external-rewrite"
condition.path = { type = "exact", value = "/origin" }
action = { type = "rewrite", target = "https://origin.example.test/page", external = true, ifNoFile = true }

[[rules]]
id = "headers"
action = { type = "set_headers", headers = { "x-edge" = "yes" } }

[[rules]]
id = "remove-headers"
action = { type = "remove_headers", headers = ["x-debug"] }

[[rules]]
id = "bypass"
action = { type = "bypass_cache" }

[[rules]]
id = "rate-shadow"
condition.path = { type = "prefix", value = "/api" }
action = { type = "rate_limit", limit = 10, windowSeconds = 60, key = "ip_host", mode = "shadow" }

[[rules]]
id = "api-terminal"
condition.path = { type = "exact", value = "/api" }
action = { type = "pipeline", override = true, inheritGate = true, steps = [{ use = "require-session", mode = "request", failure = "closed" }, { handle = "api" }] }
"#,
    );

    let value = load_edge_rules(tmp.path()).unwrap().unwrap();
    assert_eq!(value["rules"][0]["condition"]["headers"]["x-plan"], "pro");
    assert_eq!(value["rules"][0]["action"]["vary"][4], "device");
    assert_eq!(value["rules"][2]["action"]["external"], true);
    assert_eq!(value["rules"][6]["action"]["key"], "ip_host");
    assert_eq!(value["rules"][7]["action"]["steps"][1]["handle"], "api");

    let report = check_edge_rules(tmp.path()).unwrap().unwrap();
    assert_eq!(report.rule_count, 8);
    assert_eq!(report.rules[7].id, "api-terminal");
    assert_eq!(report.rules[7].position, 7);
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
fn load_edge_rules_reports_plain_http_external_rewrite_contract() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "plain-http-upstream"
action = { type = "rewrite", target = "http://example.test/api", external = true }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("external rewrite target must be an absolute https URL"),
        "unexpected error: {error}"
    );
}

#[test]
fn load_edge_rules_reports_primary_contract_error_not_fallback_position() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "regex-path"
condition.path = { type = "regex", value = "^/capture/(?P<slug>[a-z]+)$" }
action = { type = "set_headers", headers = { "x-edge" = "yes" } }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("unknown variant `regex`"),
        "unexpected error: {error}"
    );
    assert!(
        !message.contains("unknown field `position`"),
        "fallback position error leaked: {error}"
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
fn load_edge_rules_rejects_invalid_redirect_status() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "bad-redirect"
condition.path = { type = "exact", value = "/old" }
action = { type = "redirect", target = "/new", statusCode = 418 }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(
        error.to_string().contains("statusCode must be one of"),
        "unexpected error: {error}"
    );
}

#[test]
fn load_edge_rules_accepts_valid_redirect_status() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "ok-redirect"
condition.path = { type = "exact", value = "/old" }
action = { type = "redirect", target = "/new", statusCode = 308 }
"#,
    );

    assert!(load_edge_rules(tmp.path()).unwrap().is_some());
}

#[test]
fn load_edge_rules_accepts_path_captures_and_escapes() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "clean-urls"
condition.path = { type = "glob", value = "/blog/{slug}" }
action = { type = "rewrite", target = "/posts/{slug}.html" }

[[rules]]
id = "move-docs"
condition.path = { type = "glob", value = "/docs/{rest...}" }
action = { type = "redirect", target = "/guides/{rest}", statusCode = 308 }

[[rules]]
id = "external-api"
condition.path = { type = "glob", value = "/api/{rest...}" }
action = { type = "rewrite", target = "https://api.example.test/{rest}", external = true }

[[rules]]
id = "uri-template"
condition.path = { type = "prefix", value = "/api" }
action = { type = "set_headers", headers = { link = "</api/{{id}}>; rel=template" } }
"#,
    );

    assert!(load_edge_rules(tmp.path()).unwrap().is_some());
}

#[test]
fn load_edge_rules_rejects_undefined_capture_reference() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "bad"
condition.path = { type = "glob", value = "/blog/{slug}" }
action = { type = "redirect", target = "/posts/{missing}.html" }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("references undefined path capture '{missing}'"),
        "unexpected error: {error}"
    );
}

#[test]
fn load_edge_rules_rejects_capture_declared_in_any_branch() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "branch-capture"
condition.any = [{ path = { type = "glob", value = "/x/{a}" } }]
action = { type = "allow" }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("captures are only supported in the rule's root condition.path"),
        "unexpected error: {error}"
    );
}

#[test]
fn load_edge_rules_rejects_duplicate_and_splat_form_captures() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "dup"
condition.path = { type = "glob", value = "/{a}/{a}" }
action = { type = "allow" }
"#,
    );
    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(
        error.to_string().contains("duplicate path capture '{a}'"),
        "unexpected error: {error}"
    );

    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "splat-ref"
condition.path = { type = "glob", value = "/docs/{rest...}" }
action = { type = "redirect", target = "/guides/{rest...}" }
"#,
    );
    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(
        error.to_string().contains("by plain '{rest}'"),
        "unexpected error: {error}"
    );
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

    let payload = build_payload("DEPLOYMENT", &collected, edge_rules, false, Vec::new());
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

#[test]
fn load_edge_rules_accepts_rate_limit_rule() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "rate-api"
condition.path = { type = "prefix", value = "/api" }
action = { type = "rate_limit", limit = 100, windowSeconds = 60 }
"#,
    );

    let value = load_edge_rules(tmp.path()).unwrap().unwrap();
    assert_eq!(value["rules"][0]["action"]["type"], "rate_limit");
    let report = check_edge_rules(tmp.path()).unwrap().unwrap();
    assert_eq!(report.rules[0].action, "rate_limit");
}

#[test]
fn load_edge_rules_accepts_pipeline_rule() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "dashboard-auth"
condition.path = { type = "prefix", value = "/dashboard" }
action = { type = "pipeline", steps = [
  { use = "require-session", mode = "request" },
  { handle = "@app" },
] }
"#,
    );

    let value = load_edge_rules(tmp.path()).unwrap().unwrap();
    assert_eq!(value["rules"][0]["action"]["type"], "pipeline");
    let report = check_edge_rules(tmp.path()).unwrap().unwrap();
    assert_eq!(report.rules[0].action, "pipeline");
}

#[test]
fn load_edge_rules_rejects_pipeline_without_terminal_handle() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "broken"
action = { type = "pipeline", steps = [
  { use = "require-session", mode = "request" },
] }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(error.to_string().contains("exactly one terminal handle"));
}

#[test]
fn load_edge_rules_rejects_function_terminal_pipeline_without_override() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "implicit-terminal"
action = { type = "pipeline", steps = [
  { handle = "api" },
] }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(error.to_string().contains("override = true"));
}

#[test]
fn load_edge_rules_rejects_pipeline_security_gate_shadowing() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "dashboard-auth"
condition.path = { type = "prefix", value = "/dashboard" }
action = { type = "pipeline", steps = [
  { use = "require-session", mode = "request" },
  { handle = "@app" },
] }

[[rules]]
id = "dashboard-settings"
condition.path = { type = "exact", value = "/dashboard/settings" }
action = { type = "pipeline", steps = [
  { use = "settings-transform", mode = "request" },
  { handle = "@app" },
] }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(error.to_string().contains("shadows security gate"));
}

#[test]
fn load_edge_rules_rejects_rate_limit_window_out_of_bounds() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "onreza.rules.toml",
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "rate-bad"
action = { type = "rate_limit", limit = 100, windowSeconds = 3600 }
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("windowSeconds must be an integer between")
    );
}
