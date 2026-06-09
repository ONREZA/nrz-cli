use std::fs;
use std::path::Path;

use super::load_edge_rules;

fn write_rules(dir: &Path, contents: &str) {
    fs::write(dir.join("onreza.rules.toml"), contents).unwrap();
}

#[test]
fn sugar_normalizes_rule_when_and_action_kind() {
    let tmp = tempfile::tempdir().unwrap();
    write_rules(
        tmp.path(),
        r#"
schema = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rule]]
id = "redirect-old"
when.path = { prefix = "/old" }
action.redirect = { target = "/new", status_code = 301 }
"#,
    );

    let value = load_edge_rules(tmp.path()).unwrap().unwrap();
    assert_eq!(value["schemaVersion"], "EDGE_RULE_SET_V1");
    let rule = &value["rules"][0];
    assert_eq!(rule["id"], "redirect-old");
    assert_eq!(rule["condition"]["path"]["type"], "prefix");
    assert_eq!(rule["condition"]["path"]["value"], "/old");
    assert_eq!(rule["action"]["type"], "redirect");
    assert_eq!(rule["action"]["target"], "/new");
    assert_eq!(rule["action"]["statusCode"], 301);
    // No TOML-only aliases leak into the canonical shape.
    assert!(rule.get("when").is_none());
    assert!(rule["action"].get("redirect").is_none());
}

#[test]
fn sugar_camelizes_snake_case_fields() {
    let tmp = tempfile::tempdir().unwrap();
    write_rules(
        tmp.path(),
        r#"
schema = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rule]]
id = "throttle"
when.path = { prefix = "/api" }
action.rate_limit = { limit = 100, window_seconds = 60, key = "ip" }

[[rule]]
id = "dashboard"
when.path = { prefix = "/dashboard" }
action.pipeline = { inherit_gate = true, steps = [
  { use = "require-session", mode = "request", failure = "closed", cache_position = "before" },
  { handle = "@app" },
] }
"#,
    );

    let value = load_edge_rules(tmp.path()).unwrap().unwrap();
    assert_eq!(value["rules"][0]["action"]["windowSeconds"], 60);
    let pipeline = &value["rules"][1]["action"];
    assert_eq!(pipeline["type"], "pipeline");
    assert_eq!(pipeline["inheritGate"], true);
    assert_eq!(pipeline["steps"][0]["cachePosition"], "before");
    assert!(pipeline.get("inherit_gate").is_none());
}

#[test]
fn sugar_normalizes_path_inside_any_and_not() {
    let tmp = tempfile::tempdir().unwrap();
    write_rules(
        tmp.path(),
        r#"
schema = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rule]]
id = "guard"
when.any = [
  { path = { prefix = "/admin" } },
  { geo = ["KP"] },
]
when.not = { path = { glob = "/public/*" } }
action.deny = {}
"#,
    );

    let value = load_edge_rules(tmp.path()).unwrap().unwrap();
    let condition = &value["rules"][0]["condition"];
    assert_eq!(condition["any"][0]["path"]["type"], "prefix");
    assert_eq!(condition["any"][0]["path"]["value"], "/admin");
    assert_eq!(condition["any"][1]["geo"][0], "KP");
    assert_eq!(condition["not"]["path"]["type"], "glob");
    assert_eq!(condition["not"]["path"]["value"], "/public/*");
}

#[test]
fn sugar_preserves_user_map_keys_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    write_rules(
        tmp.path(),
        r#"
schema = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rule]]
id = "headered"
when.headers = { "x-plan" = "pro", "x_legacy_flag" = "1" }
action.set_headers = { headers = { "cache-control" = "no-store" } }
"#,
    );

    let value = load_edge_rules(tmp.path()).unwrap().unwrap();
    let condition = &value["rules"][0]["condition"];
    // Header names are data: snake→camel must not touch them.
    assert_eq!(condition["headers"]["x-plan"], "pro");
    assert_eq!(condition["headers"]["x_legacy_flag"], "1");
    assert_eq!(
        value["rules"][0]["action"]["headers"]["cache-control"],
        "no-store"
    );
}

#[test]
fn sugar_expands_named_action_reference() {
    let tmp = tempfile::tempdir().unwrap();
    write_rules(
        tmp.path(),
        r#"
schema = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[actions.security_headers.set_headers]
headers = { "x-frame-options" = "DENY" }

[[rule]]
id = "apply-security-headers"
action.use = "security_headers"
"#,
    );

    let value = load_edge_rules(tmp.path()).unwrap().unwrap();
    // The `actions` table is authoring-only and must not survive normalization.
    assert!(value.get("actions").is_none());
    let action = &value["rules"][0]["action"];
    assert_eq!(action["type"], "set_headers");
    assert_eq!(action["headers"]["x-frame-options"], "DENY");
}

#[test]
fn sugar_rejects_multiple_action_kinds() {
    let tmp = tempfile::tempdir().unwrap();
    write_rules(
        tmp.path(),
        r#"
schema = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rule]]
id = "ambiguous"
action.redirect = { target = "/new" }
action.deny = {}
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(
        error.to_string().contains("exactly one action.<kind>"),
        "unexpected error: {error}"
    );
}

#[test]
fn sugar_rejects_unknown_named_action() {
    let tmp = tempfile::tempdir().unwrap();
    write_rules(
        tmp.path(),
        r#"
schema = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rule]]
id = "dangling"
action.use = "missing"
"#,
    );

    let error = load_edge_rules(tmp.path()).unwrap_err();
    assert!(
        error.to_string().contains("unknown named action 'missing'"),
        "unexpected error: {error}"
    );
}
