use super::*;

// ── Error-code contract (user-fault failures carry CodedError) ───────

#[test]
fn create_deployment_error_maps_edge_rules_divergence() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: StatusCode::BAD_REQUEST,
        code: "EDGE_RULES_DIVERGED".to_string(),
        message:
            "environment has UI-authored edge rules; run `nrz rules pull` to import them, or redeploy with --force-rules"
                .to_string(),
        retry_after_seconds: None,
        details: None,
    }
    .into();

    let mapped = map_create_deployment_error(error, false);
    let rendered = format!("{mapped:#}");
    let coded = mapped
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
        .expect("mapped error must keep EDGE_RULES_DIVERGED code");

    assert_eq!(coded.code, "EDGE_RULES_DIVERGED");
    assert!(rendered.contains("nrz rules pull"));
    assert!(rendered.contains("--force-rules"));
}

#[test]
fn create_deployment_validation_error_in_json_mode_preserves_details() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: StatusCode::BAD_REQUEST,
        code: "VALIDATION_ERROR".to_string(),
        message: "Ошибка валидации данных".to_string(),
        retry_after_seconds: None,
        details: Some(serde_json::json!({
            "fields": [{ "field": "manifest.meta", "message": "Некорректное значение" }]
        })),
    }
    .into();

    let mapped = map_create_deployment_error(error, true);

    assert!(
        mapped
            .downcast_ref::<crate::output::AlreadyReportedError>()
            .is_some(),
        "validation error in JSON mode must be fully reported with details, got: {mapped:#}"
    );
}

#[test]
fn source_registration_validation_error_preserves_actionable_diagnostic() {
    let details = serde_json::json!({
        "fields": [{
            "field": "functions.edgeRules.rules.0.action.target",
            "message": "external rewrite target must be an absolute https URL"
        }]
    });
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: StatusCode::BAD_REQUEST,
        code: "VALIDATION_ERROR".to_string(),
        message: "Validation failed".to_string(),
        retry_after_seconds: None,
        details: Some(details.clone()),
    }
    .into();

    let mapped =
        map_source_registration_error(error, true, "failed to register admitted deployment source");
    let diagnostic = pre_source_failure_diagnostic(&mapped, None).expect("pre-source diagnostic");

    assert_eq!(diagnostic.code, "VALIDATION_ERROR");
    assert!(
        diagnostic
            .message
            .contains("functions.edgeRules.rules.0.action.target")
    );
    assert!(
        diagnostic
            .message
            .contains("external rewrite target must be an absolute https URL")
    );
    assert_eq!(diagnostic.details.as_ref(), Some(&details));

    let body =
        pre_source_failure_body(0, PreSourceFailureCode::BuildFailed, Some(diagnostic)).unwrap();
    let value = serde_json::to_value(body).unwrap();
    assert_eq!(value["errorCode"], "BUILD_FAILED");
    assert_eq!(value["diagnostic"]["code"], "VALIDATION_ERROR");
    assert_eq!(value["diagnostic"]["details"], details);
}

#[test]
fn source_registration_edge_rules_divergence_is_actionable() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: StatusCode::CONFLICT,
        code: "FUNCTION_PUBLISH_FAILED".to_string(),
        message: "Ошибка валидации данных".to_string(),
        retry_after_seconds: None,
        details: Some(serde_json::json!({
            "field": "edgeRules",
            "errorCode": "EDGE_RULES_DIVERGED",
            "message": "Edge Rules changed remotely. Run `nrz rules pull` or deploy with `--force-rules`."
        })),
    }
    .into();

    let mapped = map_source_registration_error(
        error,
        false,
        "failed to register admitted deployment source",
    );
    let diagnostic = pre_source_failure_diagnostic(&mapped, None).expect("pre-source diagnostic");

    assert_eq!(diagnostic.code, "EDGE_RULES_DIVERGED");
    assert!(diagnostic.message.contains("nrz rules pull"));
    assert!(diagnostic.message.contains("--force-rules"));
}

#[test]
fn human_source_registration_error_preserves_structured_details() {
    let details = serde_json::json!({
        "fields": [{"field": "manifest.layers.0", "message": "invalid layer"}]
    });
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: StatusCode::BAD_REQUEST,
        code: "VALIDATION_ERROR".to_string(),
        message: "Validation failed".to_string(),
        retry_after_seconds: None,
        details: Some(details.clone()),
    }
    .into();

    let mapped = map_source_registration_error(
        error,
        false,
        "failed to register admitted deployment source",
    );
    let diagnostic = pre_source_failure_diagnostic(&mapped, None).expect("pre-source diagnostic");

    assert_eq!(diagnostic.code, "VALIDATION_ERROR");
    assert_eq!(diagnostic.details.as_ref(), Some(&details));
}

#[test]
fn pre_source_failure_uses_the_build_log_secret_redactor() {
    let redactor =
        ExactValueRedactor::from_values(["materialized-secret-value".to_string()]).unwrap();
    let error: anyhow::Error = output::CodedError::new(
        "BUILD_EXIT_CODE",
        "build failed with materialized-secret-value",
    )
    .into();

    let diagnostic = pre_source_failure_diagnostic(&error, Some(&redactor))
        .expect("coded build failure must have a diagnostic");

    assert_eq!(diagnostic.code, "BUILD_EXIT_CODE");
    assert_eq!(diagnostic.message, "build failed with [REDACTED]");
}

#[test]
fn pre_source_failure_redacts_structured_details() {
    let redactor =
        ExactValueRedactor::from_values(["materialized-secret-value".to_string()]).unwrap();
    let error = crate::errors::CliError::new("VALIDATION_ERROR", "validation failed")
        .details(serde_json::json!({
            "fields": [{
                "field": "manifest.layers.0",
                "message": "materialized-secret-value is invalid"
            }]
        }))
        .into_anyhow();

    let diagnostic = pre_source_failure_diagnostic(&error, Some(&redactor))
        .expect("typed error must have a diagnostic");

    assert_eq!(
        diagnostic.details,
        Some(serde_json::json!({
            "fields": [{
                "field": "manifest.layers.0",
                "message": "[REDACTED] is invalid"
            }]
        }))
    );
}

#[test]
fn untyped_pre_source_failure_keeps_actionable_internal_diagnostic() {
    let error = anyhow::anyhow!("source bundle invariant failed");

    let diagnostic = pre_source_failure_diagnostic(&error, None)
        .expect("untyped failures must still be persisted");

    assert_eq!(diagnostic.code, "INTERNAL_ERROR");
    assert_eq!(diagnostic.message, "source bundle invariant failed");
    assert!(diagnostic.details.is_none());
}

#[test]
fn boundary_wrap_nuxt_missing_server_is_missing_process_entry() {
    // validate_process_output is an internal helper; the boundary wrap that
    // tags its failures with MISSING_PROCESS_ENTRY lives at the call site in
    // deploy::run (`with_default_code(..., "MISSING_PROCESS_ENTRY")`). We
    // simulate that wrap here so the full user-visible classification path is
    // exercised end-to-end.
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();

    let detection = make_detection("nuxt", None);
    let raw = validate_process_output(&output_dir, dir.path(), &detection)
        .expect_err("nuxt without server/index.mjs must fail");
    let wrapped = crate::output::with_default_code(raw, "MISSING_PROCESS_ENTRY");
    expect_code(&wrapped, "MISSING_PROCESS_ENTRY");
}

#[test]
fn boundary_wrap_preserves_more_specific_framework_unsupported() {
    // CF Workers detection is tagged FRAMEWORK_UNSUPPORTED deeper in the stack;
    // the outer boundary wrap (MISSING_PROCESS_ENTRY) must NOT clobber it.
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"@cloudflare/vite-plugin":"^1.0.0"}}"#,
    )
    .unwrap();

    let detection = make_detection("tanstack-start", None);
    let raw = validate_process_output(&output_dir, dir.path(), &detection)
        .expect_err("CF workers detection must fail");
    let wrapped = crate::output::with_default_code(raw, "MISSING_PROCESS_ENTRY");
    expect_code(&wrapped, "FRAMEWORK_UNSUPPORTED");
}

#[test]
fn ensure_process_entry_missing_user_entry_is_invalid_deploy_entry() {
    // User set [deploy] entry in onreza.toml but the file isn't in the build
    // output — the point-coded INVALID_DEPLOY_ENTRY must win over the outer
    // MISSING_PROCESS_ENTRY wrap, so users see the specific diagnosis.
    let dir = tempdir().unwrap();
    let detection = make_detection("nuxt", None);
    let err = ensure_process_entry(dir.path(), dir.path(), Some("server.mjs"), &detection, true)
        .expect_err("user entry missing on disk must fail");
    let wrapped = crate::output::with_default_code(err, "MISSING_PROCESS_ENTRY");
    expect_code(&wrapped, "INVALID_DEPLOY_ENTRY");
}

#[test]
fn ensure_process_entry_missing_user_entry_suggests_nested_output_dir() {
    // Regression from production: selected output root was /workspace, but the
    // build emitted onreza-output/server.cjs. The user needs an outputDirectory
    // fix, not a generic "server.cjs missing" message.
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("onreza-output")).unwrap();
    fs::write(
        dir.path().join("onreza-output/server.cjs"),
        "console.log('ok')",
    )
    .unwrap();

    let detection = make_detection("express", None);
    let err = ensure_process_entry(dir.path(), dir.path(), Some("server.cjs"), &detection, true)
        .expect_err("entry is outside selected output root");
    let msg = err.to_string();

    assert!(
        msg.contains("onreza-output/server.cjs"),
        "should mention discovered nested entry: {msg}"
    );
    assert!(
        msg.contains("[build] output_directory = \"onreza-output\""),
        "should suggest the outputDirectory fix: {msg}"
    );
    expect_code(&err, "INVALID_DEPLOY_ENTRY");
}

#[test]
fn format_deployment_failure_includes_runtime_startup_details() {
    let status = DeploymentStatusResponse {
        id: "dep-1".to_string(),
        status: "failed".to_string(),
        url: None,
        production: None,
        error: Some("Pre-warm failed".to_string()),
        error_code: Some("DEPLOY_PREWARM_PORT_MISMATCH".to_string()),
        error_details: Some(DeploymentErrorDetails {
            runtime_startup_failure: Some(RuntimeStartupFailureDetails {
                code: Some("port_mismatch".to_string()),
                message: Some("Your app is listening on port 3000.".to_string()),
                check_type: Some("tcp".to_string()),
                health_path: None,
                expected_port: Some(30123),
                detected_ports: vec![3000],
                timeout_seconds: Some(30),
                attempts: Some(2700),
                last_error: Some("Connection refused".to_string()),
                process_entry: Some("server.js".to_string()),
                log_tail: Some("server started on 3000".to_string()),
                retry_after_seconds: None,
            }),
        }),
        created_at: None,
        ready_at: None,
    };

    let msg = format_deployment_failure("Pre-warm failed", &status);

    assert!(msg.contains("Your app is listening on port 3000."));
    assert!(msg.contains("expected port: 30123"));
    assert!(msg.contains("detected ports: 3000"));
    assert!(msg.contains("Recent runtime output"));
}

#[test]
fn parse_compute_type_rejects_unknown_value_with_code() {
    let err = parse_compute_type("lambda").expect_err("unknown compute must fail");
    expect_code(&err, "INVALID_COMPUTE_TYPE");
}

#[test]
fn validate_health_path_rejects_query_string_with_code() {
    let err =
        validate_health_path("/health?x=1", "--health-check-path").expect_err("query must fail");
    expect_code(&err, "INVALID_ARGUMENT");
}

#[test]
fn platform_fault_errors_do_not_carry_coded_error() {
    // Negative coverage: uncoded `anyhow!` / `?` on I/O errors must leave the
    // chain free of CodedError, so the builder routes them to Sentry. If this
    // test ever goes green with a CodedError present, the contract "empty code
    // = platform-fault" has been silently eroded.
    let err = anyhow::anyhow!("simulated platform-fault");
    assert!(
        err.chain()
            .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
            .is_none(),
        "plain anyhow errors must not carry CodedError — got: {err:#}"
    );
}

#[test]
fn with_default_code_attaches_code_and_preserves_source_chain() {
    // A semantic (non-I/O) error walked through .context(..) and then
    // with_default_code must gain a CodedError AND keep the earlier context
    // reachable through the chain for downstream tooling.
    let err = anyhow::anyhow!("field \"entry\" missing").context("validating manifest");
    let wrapped = crate::output::with_default_code(err, "INVALID_MANIFEST");
    expect_code(&wrapped, "INVALID_MANIFEST");
    let rendered = format!("{wrapped:#}");
    assert!(
        rendered.contains("validating manifest") && rendered.contains("field \"entry\" missing"),
        "source chain must survive through CodedError wrapping: {rendered}"
    );
}

#[test]
fn with_default_code_skips_io_errors_so_platform_faults_reach_sentry() {
    // Guard against accidentally classifying a platform-fault I/O failure
    // (permission denied, TOCTOU, EIO) as user-fault just because the outer
    // boundary wrap fires on every error. io::Error anywhere in the chain must
    // keep the error uncoded so the builder routes it to Sentry.
    use std::io;
    let io_err: anyhow::Error =
        anyhow::Error::new(io::Error::other("perm")).context("canonicalizing entry");
    let result = crate::output::with_default_code(io_err, "MISSING_PROCESS_ENTRY");
    assert!(
        result
            .chain()
            .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
            .is_none(),
        "io::Error paths must stay uncoded: {result:#}"
    );
}

#[test]
fn wire_manifest_contract_rejects_unknown_layer_fields() {
    let conformant = serde_json::json!({
        "version": 1,
        "layers": [{ "name": "static", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*", "layer": "static" }],
    });
    let wire = conform_manifest_to_wire_contract(conformant)
        .expect("a conformant manifest must pass the wire contract");
    assert!(wire["version"].is_number());
    assert_eq!(wire["layers"][0]["target"], "STATIC");

    // A field the platform ManifestSchema does not define (e.g. the legacy `export`)
    // must be rejected at the wire instead of forwarded for the server to reject.
    let with_unknown = serde_json::json!({
        "version": 1,
        "layers": [{ "name": "static", "target": "STATIC", "directory": ".", "export": "esm" }],
        "routes": [{ "pattern": "^/.*", "layer": "static" }],
    });
    assert!(
        conform_manifest_to_wire_contract(with_unknown).is_err(),
        "unknown manifest fields must not reach the server"
    );
}

#[test]
fn wire_manifest_contract_enforces_runtime_memory_bounds() {
    for memory_mb in [-1, 31, 8193] {
        let manifest = serde_json::json!({
            "version": 1,
            "layers": [{
                "name": "app",
                "target": "COMPUTE",
                "directory": ".",
                "entry": "server.js",
                "runtime": { "memoryMb": memory_mb },
            }],
            "routes": [{ "pattern": "^/.*", "layer": "app" }],
        });
        assert!(
            conform_manifest_to_wire_contract(manifest).is_err(),
            "runtime.memoryMb={memory_mb} must not reach the server"
        );
    }

    for memory_mb in [32, 8192] {
        let manifest = serde_json::json!({
            "version": 1,
            "layers": [{
                "name": "app",
                "target": "COMPUTE",
                "directory": ".",
                "entry": "server.js",
                "runtime": { "memoryMb": memory_mb },
            }],
            "routes": [{ "pattern": "^/.*", "layer": "app" }],
        });
        conform_manifest_to_wire_contract(manifest)
            .unwrap_or_else(|error| panic!("runtime.memoryMb={memory_mb} must be valid: {error}"));
    }
}

#[test]
fn wire_manifest_contract_preserves_route_fallthrough_when() {
    let fallthrough_when = serde_json::json!([
        { "type": "header", "name": "rsc", "value": "1" },
        { "type": "query", "name": "_rsc" },
    ]);
    let manifest = serde_json::json!({
        "version": 1,
        "layers": [{ "name": "static", "target": "STATIC", "directory": "." }],
        "routes": [{
            "pattern": "^/.*",
            "layer": "static",
            "fallthrough": true,
            "fallthroughWhen": fallthrough_when.clone(),
        }],
    });

    let wire = conform_manifest_to_wire_contract(manifest)
        .expect("fallthroughWhen must be part of the manifest wire contract");
    assert_eq!(wire["routes"][0]["fallthroughWhen"], fallthrough_when);
}

#[test]
fn wire_functions_contract_validates_origin_against_server_enum() {
    let ok = crate::functions::FunctionPublishPayload {
        origin: "DEPLOYMENT",
        functions: vec![],
        edge_rules: None,
        edge_rules_force: false,
        generated_edge_rule_sets: Vec::new(),
    };
    assert!(
        conform_functions_to_wire_contract(Some(ok))
            .unwrap()
            .is_some()
    );
    assert!(conform_functions_to_wire_contract(None).unwrap().is_none());

    // An origin value the server contract does not define must be rejected at the wire.
    let bad = crate::functions::FunctionPublishPayload {
        origin: "BOGUS",
        functions: vec![],
        edge_rules: None,
        edge_rules_force: false,
        generated_edge_rule_sets: Vec::new(),
    };
    assert!(
        conform_functions_to_wire_contract(Some(bad)).is_err(),
        "an unknown functions origin must not reach the server"
    );
}

#[test]
fn wire_functions_contract_accepts_generated_edge_rule_contributions() {
    let payload = crate::functions::FunctionPublishPayload {
        origin: "DEPLOYMENT",
        functions: vec![],
        edge_rules: None,
        edge_rules_force: false,
        generated_edge_rule_sets: vec![crate::functions::GeneratedEdgeRuleSet {
            producer: "nextjs-adapter".to_string(),
            version: Some("16.2.9".to_string()),
            edge_rules: serde_json::json!({
                "schemaVersion": "EDGE_RULE_SET_V1",
                "rules": [],
            }),
        }],
    };

    let value = conform_functions_to_wire_contract(Some(payload))
        .unwrap()
        .expect("functions payload");

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

#[test]
fn pre_source_mutations_use_the_stable_execution_contract() {
    assert_eq!(
        crate::execution_context::RUNNER_CONTEXT_PROTOCOL,
        "runner-context-v4"
    );
    let body =
        pre_source_failure_body(2, PreSourceFailureCode::MaterializationFailed, None).unwrap();

    let value = serde_json::to_value(body).unwrap();
    assert_eq!(value["protocolVersion"], "execution-context-v2");
    assert_eq!(value["attempt"], 2);
    assert_eq!(value["errorCode"], "MATERIALIZATION_FAILED");

    let source = source_request_body(2,
        "019b8952-ca22-76f0-b134-88becec7c629".parse().unwrap(),
        serde_json::json!({ "version": 1, "layers":[{"name":"static","target":"STATIC","directory":"."}], "routes":[{"pattern":"^/.*","layer":"static"}] }), None).unwrap();
    let source_value = serde_json::to_value(source).unwrap();
    assert_eq!(source_value["protocolVersion"], "execution-context-v2");
    assert_eq!(source_value["attempt"], 2);
    assert_eq!(
        source_value["operationId"],
        "019b8952-ca22-76f0-b134-88becec7c629"
    );
}

#[test]
fn runner_protocol_mismatch_requires_a_cli_update() {
    let error = require_runner_context_protocol("runner-context-v2").unwrap_err();
    let coded = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
        .unwrap();

    assert_eq!(coded.code, "CLI_UPDATE_REQUIRED");
}
