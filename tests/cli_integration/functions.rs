use super::*;

#[test]
fn functions_runtime_status_json_emits_single_object_without_downloading() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .env("XDG_CACHE_HOME", temp.path())
        .args(["functions", "runtime", "status", "--json"]);
    let output = cmd.output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "expected one JSON object, got: {stdout}");
    let value: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(
        value["runtimeReleaseId"],
        "runtime-2d20a492936c63b6dd1dd6b23f0e950af7e071e3"
    );
    assert_eq!(value["installed"], false);
    assert!(
        value["target"]
            .as_str()
            .is_some_and(|target| !target.is_empty())
    );
}

#[test]
fn functions_runtime_help_exposes_cache_management_commands() {
    let mut cmd = nrz();
    cmd.args(["functions", "runtime", "--help"]);
    let output = cmd.output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for command in ["install", "status", "path"] {
        assert!(
            stdout.contains(command),
            "missing runtime command {command}"
        );
    }
}

#[test]
fn functions_check_json_accepts_static_rules_only_project() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("onreza.rules.toml"),
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "redirect-old"
condition.path = { type = "prefix", value = "/old" }
action = { type = "redirect", target = "/new" }
"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["functions", "check", "--json"]);
    let output = cmd.output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(value["functions"].as_array().unwrap().len(), 0);
    assert_eq!(value["edgeRules"]["ruleCount"], 1);
    assert_eq!(value["edgeRules"]["rules"][0]["id"], "redirect-old");
    assert_eq!(value["edgeRules"]["rules"][0]["position"], 0);
    assert_eq!(value["edgeRules"]["rules"][0]["action"], "redirect");
}
