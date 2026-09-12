use super::*;

// ── framework preset source handling ─────────────────────────

#[test]
fn server_framework_other_does_not_mask_local_vite_detection() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"vite":"^5.0.0"},"scripts":{"start":"vite --host","build":"vite build"}}"#,
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(
        dir.path(),
        authoritative_server_framework_preset(Some("other")),
    );

    assert_eq!(detection.framework, "vite");
    assert_eq!(
        detection.suggested_compute,
        crate::detect::types::ComputeType::Static
    );
}

#[test]
fn server_framework_specific_preset_is_usable_when_local_framework_absent() {
    assert_eq!(
        authoritative_server_framework_preset(Some("vite")),
        Some("vite")
    );
    assert_eq!(authoritative_server_framework_preset(Some("other")), None);
}

// ── ProjectInfo deserialization ──────────────────────────────

#[test]
fn project_info_deserializes_camel_case() {
    let json = r#"{
        "id": "proj_123",
        "frameworkPreset": "vite",
        "rootDirectory": ".",
        "packageManager": "NPM",
        "installCommand": "npm ci",
        "installCommandSource": "DETECTED",
        "buildCommand": "npm run build",
        "buildCommandSource": "USER",
        "outputDirectory": "dist",
        "outputDirectorySource": "USER"
    }"#;
    let info: ProjectInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.framework_preset.unwrap(), "vite");
    assert_eq!(info.root_directory, ".");
    assert_eq!(info.package_manager, "NPM");
    assert_eq!(info.install_command.unwrap(), "npm ci");
    assert_eq!(
        info.install_command_source.unwrap(),
        crate::build::BuildSettingSource::Detected
    );
    assert_eq!(info.build_command.unwrap(), "npm run build");
    assert_eq!(
        info.build_command_source.unwrap(),
        crate::build::BuildSettingSource::User
    );
    assert_eq!(info.output_directory.unwrap(), "dist");
    assert_eq!(
        info.output_directory_source.unwrap(),
        crate::build::BuildSettingSource::User
    );
}

#[test]
fn project_info_optional_fields_default_to_none_when_snapshot_identity_is_present() {
    let json = r#"{"id":"proj_123","rootDirectory":".","packageManager":"NPM"}"#;
    let info: ProjectInfo = serde_json::from_str(json).unwrap();
    assert!(info.install_command.is_none());
    assert!(info.install_command_source.is_none());
    assert!(info.build_command.is_none());
    assert!(info.build_command_source.is_none());
    assert!(info.output_directory.is_none());
    assert!(info.output_directory_source.is_none());
}

// (Content-Type guessing tests removed: blob/bundle PUTs go through `put_blob`,
// which omits Content-Type so the SigV4 signature stays valid — the helper
// `guess_content_type` is no longer in the codebase.)

// ── framework_static_hint tests ──────────────────────────────

#[test]
fn static_hint_known_frameworks_non_empty() {
    assert!(!framework_static_hint("nextjs").is_empty());
    assert!(!framework_static_hint("nuxt").is_empty());
    assert!(!framework_static_hint("sveltekit").is_empty());
    assert!(!framework_static_hint("astro").is_empty());
    assert!(!framework_static_hint("react-router").is_empty());
    assert!(!framework_static_hint("remix").is_empty());
    assert!(!framework_static_hint("solidstart").is_empty());
    assert!(!framework_static_hint("qwik").is_empty());
    assert!(!framework_static_hint("analog").is_empty());
    assert!(framework_static_hint("nextjs").contains("export"));
    assert!(framework_static_hint("react-router").contains("ssr: false"));
    assert!(framework_static_hint("remix").contains("ssr: false"));
    assert!(framework_static_hint("solidstart").contains("ssr: false"));
    assert!(framework_static_hint("analog").contains("ssr: false"));
}

#[test]
fn static_hint_unknown_returns_empty() {
    assert!(framework_static_hint("vite").is_empty());
    assert!(framework_static_hint("unknown").is_empty());
}

// ── compute/manifest contract tests ─────────────────────────

#[test]
fn process_with_manifest_is_ok() {
    // Manifest can declare COMPUTE layers — PROCESS + manifest is valid.
    assert!(validate_compute_manifest_contract(ComputeType::Process, true).is_ok());
}

#[test]
fn static_without_manifest_is_ok() {
    assert!(validate_compute_manifest_contract(ComputeType::Static, false).is_ok());
}

#[test]
fn static_with_manifest_is_ok() {
    // Manifest can declare only STATIC layers — STATIC + manifest is valid.
    assert!(validate_compute_manifest_contract(ComputeType::Static, true).is_ok());
}

#[test]
fn process_without_manifest_is_error() {
    // Safety net: PROCESS auto-generation should always produce a manifest before
    // validate_compute_manifest_contract is called, so reaching here with has_manifest=false
    // is an unexpected state.
    let err = validate_compute_manifest_contract(ComputeType::Process, false)
        .expect_err("PROCESS without manifest should fail");
    assert!(
        err.to_string().contains("Internal error"),
        "unexpected error: {err}"
    );
}

#[test]
fn functions_payload_serializes_edge_rules_force() {
    let value =
        conform_functions_to_wire_contract(Some(crate::functions::FunctionPublishPayload {
            origin: "DEPLOYMENT",
            functions: vec![],
            edge_rules: Some(serde_json::json!({
                "schemaVersion": "EDGE_RULE_SET_V1",
                "source": { "origin": "build" },
                "rules": [
                    {
                        "id": "allow-all",
                        "action": { "type": "allow" }
                    }
                ]
            })),
            edge_rules_force: true,
            generated_edge_rule_sets: Vec::new(),
        }))
        .unwrap()
        .unwrap();

    assert_eq!(value["edgeRulesForce"], true);
}

#[tokio::test]
async fn build_functions_payload_generates_nextjs_edge_rules_when_local_rules_absent() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".onreza")).unwrap();
    fs::write(
        tmp.path().join(".onreza/next-adapter-output.json"),
        r#"
{
  "version": 1,
  "adapter": { "name": "@onreza/nrz-next-adapter", "version": "0.34.1" },
  "config": {
    "images": {
      "loader": "custom",
      "loaderFile": "./.onreza/cache/next-adapter/onreza-image-loader.mjs"
    }
  },
  "deploymentHints": {
    "imageOptimizer": {
      "status": "onreza_optimizer",
      "remoteImageSources": [
        {
          "id": "next.images.remote-pattern.0",
          "protocol": "https",
          "hostname": "cdn.example.com",
          "pathname": "/tenant/**",
          "search": ""
        }
      ]
    }
  },
  "routing": {
    "beforeMiddleware": [
      {
        "source": "/old",
        "headers": { "Location": "/new" },
        "status": 308
      }
    ]
  },
  "outputs": {}
}
"#,
    )
    .unwrap();

    let payload = build_functions_payload(
        &nrz::config::ProjectConfig::default(),
        tmp.path(),
        true,
        false,
    )
    .await
    .unwrap()
    .expect("generated Next.js Edge Rules should create a functions payload");
    let value = serde_json::to_value(&payload).unwrap();

    assert_eq!(value["origin"], "DEPLOYMENT");
    assert_eq!(value["functions"].as_array().unwrap().len(), 0);
    assert!(value.get("edgeRules").is_none());
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["producer"],
        "nextjs-adapter"
    );
    assert!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]
            .get("source")
            .is_none()
    );
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]["rules"][0]["action"]["type"],
        "redirect"
    );
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]["rules"][0]["action"]["target"],
        "/new"
    );
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]["imageSources"][0]["hostname"],
        "cdn.example.com"
    );
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]["imageSources"][0]["search"],
        ""
    );
}

#[tokio::test]
async fn build_functions_payload_sends_empty_deployment_snapshot_without_adapter() {
    let tmp = tempdir().unwrap();
    let payload = build_functions_payload(
        &nrz::config::ProjectConfig::default(),
        tmp.path(),
        true,
        false,
    )
    .await
    .unwrap()
    .expect("deployment snapshot must clear stale generated adapter config");
    let value = serde_json::to_value(&payload).unwrap();

    assert_eq!(value["origin"], "DEPLOYMENT");
    assert_eq!(value["functions"], serde_json::json!([]));
    assert!(value.get("edgeRules").is_none());
    assert!(value.get("generatedEdgeRuleSets").is_none());
}

#[tokio::test]
async fn build_functions_payload_sends_empty_nextjs_generated_contribution_for_clearing() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".onreza")).unwrap();
    fs::write(
        tmp.path().join(".onreza/next-adapter-output.json"),
        r#"
{
  "version": 1,
  "adapter": { "name": "@onreza/nrz-next-adapter", "version": "0.34.1" },
  "routing": {},
  "outputs": {}
}
"#,
    )
    .unwrap();

    let payload = build_functions_payload(
        &nrz::config::ProjectConfig::default(),
        tmp.path(),
        true,
        false,
    )
    .await
    .unwrap()
    .expect("empty Next.js generated contribution should clear stale adapter rules");
    let value = serde_json::to_value(&payload).unwrap();

    assert_eq!(value["origin"], "DEPLOYMENT");
    assert_eq!(value["functions"].as_array().unwrap().len(), 0);
    assert!(value.get("edgeRules").is_none());
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["producer"],
        "nextjs-adapter"
    );
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]["rules"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
async fn build_functions_payload_sends_user_and_nextjs_generated_rules_separately() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".onreza")).unwrap();
    fs::write(
        tmp.path().join(".onreza/next-adapter-output.json"),
        r#"
{
  "version": 1,
  "adapter": { "name": "@onreza/nrz-next-adapter", "version": "0.34.1" },
  "routing": {
    "beforeMiddleware": [
      {
        "source": "/old",
        "headers": { "Location": "/new" },
        "status": 308
      }
    ]
  },
  "outputs": {}
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("onreza.rules.toml"),
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "user-owned"
action = { type = "allow" }
"#,
    )
    .unwrap();

    let payload = build_functions_payload(
        &nrz::config::ProjectConfig::default(),
        tmp.path(),
        true,
        false,
    )
    .await
    .unwrap()
    .expect("user Edge Rules should create a functions payload");
    let value = serde_json::to_value(&payload).unwrap();

    assert_eq!(value["edgeRules"]["rules"][0]["id"], "user-owned");
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["producer"],
        "nextjs-adapter"
    );
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]["rules"][0]["action"]["target"],
        "/new"
    );
}

#[test]
fn publication_limit_error_in_json_mode_preserves_runtime_breakdown() {
    let mapped = map_publication_error(limit_exceeded_publication_error(), true, &file_breakdown());
    assert!(
        mapped
            .downcast_ref::<crate::output::AlreadyReportedError>()
            .is_some(),
        "limit error in JSON mode must be fully reported, got: {mapped:#}"
    );
    let diagnostic = output::reported_terminal_diagnostic(&mapped).unwrap();
    assert_eq!(
        diagnostic.details.as_ref().unwrap()["runtimeArtifactFiles"]["total"],
        20_679
    );
    assert_eq!(
        diagnostic.details.as_ref().unwrap()["runtimeArtifactFiles"]["nodeModules"],
        19_620
    );
}

#[test]
fn publication_limit_error_in_human_mode_is_actionable() {
    let mapped =
        map_publication_error(limit_exceeded_publication_error(), false, &file_breakdown());
    assert!(
        mapped
            .downcast_ref::<crate::output::AlreadyReportedError>()
            .is_none()
    );
    assert!(
        mapped
            .to_string()
            .contains("Runtime artifact file breakdown")
    );
    assert!(mapped.to_string().contains("node_modules 19620"));
    assert!(
        mapped
            .to_string()
            .contains("static adapter only when server-side execution is not required")
    );
}

#[test]
fn publication_platform_error_in_json_mode_is_not_swallowed() {
    let error = nrz_source_publisher::StructuredControlPlaneError {
        status: reqwest::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
        code: "INTERNAL".into(),
        message: "boom".into(),
        retry_after: None,
        details: None,
    }
    .into();
    let mapped = map_publication_error(error, true, &file_breakdown());
    assert!(
        mapped
            .downcast_ref::<crate::output::AlreadyReportedError>()
            .is_none()
    );
    assert!(
        mapped
            .to_string()
            .contains("failed to publish verified source bundle")
    );
}

#[test]
fn file_entry_serializes_with_camel_case_content_hash() {
    let entry = FileEntry {
        path: "a.js".into(),
        size: 42,
        content_hash: "abc123".into(),
        kind: crate::artifact::ArtifactFileKind::File,
        symlink_resolved_path: None,
    };
    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["contentHash"], "abc123");
    assert_eq!(json["path"], "a.js");
    assert_eq!(json["size"], 42);
    // Server schema (FileEntrySchema) is `.strict()`, so any stray key would
    // make the deployment-create POST fail validation.
    let obj = json.as_object().unwrap();
    assert_eq!(
        obj.len(),
        3,
        "expected exactly path/size/contentHash, got {obj:?}"
    );
}
