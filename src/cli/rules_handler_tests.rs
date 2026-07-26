use serde_json::json;

use super::rules_handler::{
    ActiveEdgeRuleSet, active_rule_set_to_authoring_value, build_edge_rules_status_request,
    edge_rule_set_authoring_to_toml, map_publish_error, replace_file_with_rollback,
    write_rules_file,
};

#[test]
fn active_rule_set_pull_conversion_drops_position_and_writes_rules_tables() {
    let active = ActiveEdgeRuleSet {
        id: "ruleset-1".to_string(),
        version: 7,
        schema_version: Some("EDGE_RULE_SET_V1".to_string()),
        source: "BUILD".to_string(),
        rules: json!([
            {
                "id": "second",
                "position": 1,
                "enabled": true,
                "condition": {},
                "action": { "type": "allow" }
            },
            {
                "id": "first",
                "position": 0,
                "enabled": true,
                "condition": { "path": { "type": "prefix", "value": "/old" } },
                "action": { "type": "redirect", "target": "/new", "statusCode": 301 }
            }
        ]),
        checksum: "abc".to_string(),
    };

    let authoring = active_rule_set_to_authoring_value(&active).unwrap();
    let toml = edge_rule_set_authoring_to_toml(&authoring).unwrap();

    assert!(!toml.contains("source"));
    assert!(toml.contains("[[rules]]"));
    assert!(!toml.contains("position"));
    assert!(
        toml.find("id = \"first\"").unwrap() < toml.find("id = \"second\"").unwrap(),
        "rules must be ordered by server position: {toml}"
    );

    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("onreza.rules.toml"), toml).unwrap();
    let parsed = crate::functions::load_edge_rules(tmp.path())
        .unwrap()
        .unwrap();
    assert_eq!(parsed["rules"][0]["id"], "first");
    assert!(parsed["rules"][0].get("position").is_none());
}

#[cfg(unix)]
#[test]
fn rules_pull_write_replaces_link_instead_of_following_it() {
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("victim.toml");
    std::fs::write(&outside_file, "outside").unwrap();
    let rules_path = project.path().join("onreza.rules.toml");
    std::os::unix::fs::symlink(&outside_file, &rules_path).unwrap();

    write_rules_file(&rules_path, b"local").unwrap();

    assert_eq!(std::fs::read_to_string(&outside_file).unwrap(), "outside");
    assert_eq!(std::fs::read_to_string(&rules_path).unwrap(), "local");
    assert!(
        !std::fs::symlink_metadata(&rules_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn rules_replacement_restores_previous_file_when_install_fails() {
    let project = tempfile::tempdir().unwrap();
    let rules_path = project.path().join("onreza.rules.toml");
    let missing_candidate = project.path().join("missing-candidate.toml");
    std::fs::write(&rules_path, "previous").unwrap();

    let error = replace_file_with_rollback(&missing_candidate, &rules_path).unwrap_err();

    assert!(
        error.to_string().contains("previous file was restored"),
        "{error:#}"
    );
    assert_eq!(std::fs::read_to_string(&rules_path).unwrap(), "previous");
    assert!(
        std::fs::read_dir(project.path())
            .unwrap()
            .flatten()
            .all(|entry| !entry.file_name().to_string_lossy().contains(".backup-"))
    );
}

#[test]
fn rules_replacement_removes_backup_after_success() {
    let project = tempfile::tempdir().unwrap();
    let rules_path = project.path().join("onreza.rules.toml");
    let candidate = project.path().join("candidate.toml");
    std::fs::write(&rules_path, "previous").unwrap();
    std::fs::write(&candidate, "replacement").unwrap();

    replace_file_with_rollback(&candidate, &rules_path).unwrap();

    assert_eq!(std::fs::read_to_string(&rules_path).unwrap(), "replacement");
    assert!(
        std::fs::read_dir(project.path())
            .unwrap()
            .flatten()
            .all(|entry| !entry.file_name().to_string_lossy().contains(".backup-"))
    );
}

#[test]
fn rules_publish_edge_divergence_mentions_publish_force() {
    let error = anyhow::Error::new(crate::api::StructuredApiError {
        status: reqwest::StatusCode::CONFLICT,
        code: "EDGE_RULES_DIVERGED".to_string(),
        message: "environment has UI-authored edge rules".to_string(),
        retry_after_seconds: None,
        details: Some(json!({
            "message": "environment has UI-authored edge rules"
        })),
    });

    let mapped = map_publish_error(error, false);
    let text = format!("{mapped:#}");

    assert!(text.contains("nrz rules pull"));
    assert!(text.contains("nrz rules publish --force-rules"));
    assert!(text.contains("failed to publish Edge Rules"));
}

#[test]
fn edge_rules_status_request_includes_local_authoring_rules() {
    let request = build_edge_rules_status_request(
        Some(json!({
            "schemaVersion": "EDGE_RULE_SET_V1",
            "source": { "origin": "build" },
            "rules": []
        })),
        false,
    )
    .unwrap();

    let value = serde_json::to_value(request).unwrap();

    assert_eq!(value["edgeRules"]["schemaVersion"], "EDGE_RULE_SET_V1");
    assert_eq!(value["edgeRules"]["rules"], json!([]));
}

#[test]
fn edge_rules_status_request_without_local_rules_is_empty() {
    let request = build_edge_rules_status_request(None, false).unwrap();

    assert_eq!(serde_json::to_value(request).unwrap(), json!({}));
}

#[test]
fn edge_rules_status_request_marks_invalid_local_rules() {
    let request = build_edge_rules_status_request(None, true).unwrap();

    assert_eq!(
        serde_json::to_value(request).unwrap(),
        json!({ "localInvalid": true })
    );
}

#[test]
fn edge_rules_status_request_rejects_local_rules_with_invalid_flag() {
    let error = build_edge_rules_status_request(
        Some(json!({
            "schemaVersion": "EDGE_RULE_SET_V1",
            "source": { "origin": "build" },
            "rules": []
        })),
        true,
    )
    .unwrap_err();

    assert!(format!("{error:#}").contains("localInvalid cannot be used with local Edge Rules"));
}
