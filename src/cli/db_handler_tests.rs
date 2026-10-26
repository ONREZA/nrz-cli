use super::*;

fn database(id: &str, name: &str, projects: &[&str]) -> ManagedDatabase {
    ManagedDatabase {
        id: id.to_string(),
        product_id: None,
        db_name: Some(name.to_string()),
        status: None,
        cu_size: None,
        pg_version: None,
        metadata: DatabaseMetadata::Legacy {
            auto_inject_db_url: None,
            env_var_name: None,
            auto_create_preview_branch: None,
            kaiki_status: None,
        },
        project_attachments: projects
            .iter()
            .map(|id| ProjectAttachment {
                project_id: id.parse().unwrap(),
                auto_inject_db_url: false,
                env_var_name: "DATABASE_URL".into(),
                auto_create_preview_branch: false,
                ..Default::default()
            })
            .collect(),
    }
}

#[test]
fn explicit_database_id_must_be_attached_to_project() {
    let databases = vec![
        database(
            "db-current",
            "current",
            &["11111111-1111-4111-8111-111111111111"],
        ),
        database(
            "db-other",
            "other",
            &["22222222-2222-4222-8222-222222222222"],
        ),
    ];
    assert!(
        select_database(
            &databases,
            Some("11111111-1111-4111-8111-111111111111"),
            Some("db-other")
        )
        .is_err()
    );
    assert_eq!(
        select_database(
            &databases,
            Some("11111111-1111-4111-8111-111111111111"),
            Some("db-current")
        )
        .unwrap()
        .id,
        "db-current"
    );
}

#[test]
fn create_body_serializes_server_contract_and_validates_size() {
    let body = wire::create_body(Some("primary".into()), Some(0.5)).unwrap();
    assert_eq!(
        serde_json::to_value(body).unwrap(),
        serde_json::json!({"dbName":"primary","cuSize":0.5})
    );
    assert!(wire::create_body(None, Some(0.75)).is_err());
    assert!(wire::create_body(None, Some(f64::NAN)).is_err());
}

#[test]
fn create_keys_match_the_public_header_contract() {
    let generated = create_idempotency_key(None).unwrap();
    assert_eq!(generated.parse::<uuid::Uuid>().unwrap().get_version_num(), 7);
    for valid in ["a".into(), "Create_1.foo:retry-2".into(), "a".repeat(128)] {
        assert_eq!(create_idempotency_key(Some(valid.clone())).unwrap(), valid);
    }
    for invalid in ["".into(), "_a".into(), "-a".into(), "a b".into(), "a\n".into(), "é".into(), "a".repeat(129)] {
        assert!(create_idempotency_key(Some(invalid)).is_err());
    }
}

#[test]
fn project_attachment_body_serializes_server_contract() {
    let body = binding_body(&explicit_binding_args()).unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(body).unwrap(),
        serde_json::json!({"serviceId":"11111111-1111-4111-8111-111111111111","productEnvironmentId":"22222222-2222-4222-8222-222222222222","ownerProductEnvironmentId":"33333333-3333-4333-8333-333333333333","providerBranchId":"provider-branch-exact","isFunctionsDefault":false,"envVarName":"DATABASE_URL","autoInjectDbUrl":true,"autoCreatePreviewBranch":false})
    );
}

#[test]
fn managed_database_uses_project_attachment_settings() {
    let mut db = database("db", "primary", &["33333333-3333-4333-8333-333333333333"]);
    db.project_attachments[0].auto_inject_db_url = true;
    db.project_attachments[0].env_var_name = "APP_DATABASE_URL".into();
    db.project_attachments[0].auto_create_preview_branch = true;
    assert_eq!(
        db.auto_inject_db_url_for_project("33333333-3333-4333-8333-333333333333"),
        Some(true)
    );
    assert_eq!(
        db.env_var_name_for_project("33333333-3333-4333-8333-333333333333"),
        Some("APP_DATABASE_URL")
    );
    assert_eq!(
        db.auto_create_preview_branch_for_project("33333333-3333-4333-8333-333333333333"),
        Some(true)
    );
}

#[test]
fn database_selection_requires_a_unique_attachment_or_exact_id() {
    let mut databases = vec![
        database("first", "same", &["33333333-3333-4333-8333-333333333333"]),
        database("second", "same", &["33333333-3333-4333-8333-333333333333"]),
    ];
    assert!(
        select_database(
            &databases,
            Some("33333333-3333-4333-8333-333333333333"),
            None
        )
        .is_err()
    );
    assert!(
        select_database(
            &databases,
            Some("33333333-3333-4333-8333-333333333333"),
            Some("same")
        )
        .is_err()
    );
    assert_eq!(
        select_database(
            &databases,
            Some("33333333-3333-4333-8333-333333333333"),
            Some("second")
        )
        .unwrap()
        .id,
        "second"
    );
    databases[0].project_attachments[0].auto_inject_db_url = true;
    assert_eq!(
        select_database(
            &databases,
            Some("33333333-3333-4333-8333-333333333333"),
            None
        )
        .unwrap()
        .id,
        "first"
    );
    databases[1].project_attachments[0].auto_inject_db_url = true;
    assert!(
        select_database(
            &databases,
            Some("33333333-3333-4333-8333-333333333333"),
            None
        )
        .is_err()
    );
}

fn explicit_binding_args() -> ConfigArgs {
    use clap::Parser;
    ConfigArgs::parse_from([
        "config",
        "--service-id",
        "11111111-1111-4111-8111-111111111111",
        "--product-environment-id",
        "22222222-2222-4222-8222-222222222222",
        "--owner-product-environment-id",
        "33333333-3333-4333-8333-333333333333",
        "--provider-branch-id",
        "provider-branch-exact",
        "--auto-inject",
        "true",
        "--env-var",
        "DATABASE_URL",
        "--preview-branches",
        "false",
        "--functions-default",
        "false",
    ])
}

#[test]
fn binding_changes_require_all_four_explicit_targets() {
    use clap::Parser;
    let args = ConfigArgs::parse_from(["config", "--auto-inject", "true"]);
    assert!(
        binding_body(&args)
            .unwrap_err()
            .downcast_ref::<output::CodedError>()
            .is_some_and(|error| error.code == "BINDING_TARGET_REQUIRED")
    );
    assert!(
        binding_body(&ConfigArgs::parse_from(["config"]))
            .unwrap()
            .is_none()
    );
    for missing in [
        "service",
        "consumer",
        "owner",
        "branch",
        "empty-branch",
        "nil-service",
    ] {
        let mut args = explicit_binding_args();
        match missing {
            "service" => args.service_id = None,
            "consumer" => args.product_environment_id = None,
            "owner" => args.owner_product_environment_id = None,
            "branch" => args.provider_branch_id = None,
            "empty-branch" => args.provider_branch_id = Some(" ".into()),
            "nil-service" => args.service_id = Some(uuid::Uuid::nil()),
            _ => unreachable!(),
        }
        assert!(
            binding_body(&args)
                .unwrap_err()
                .downcast_ref::<output::CodedError>()
                .is_some_and(|error| error.code == "BINDING_TARGET_REQUIRED"),
            "{missing}"
        );
    }
}

#[test]
fn project_summary_does_not_choose_between_service_environment_bindings() {
    let project = "33333333-3333-4333-8333-333333333333";
    let mut db = database("db", "primary", &[project, project]);
    db.project_attachments[0].auto_inject_db_url = true;
    assert!(db.is_attached_to_project(project));
    assert!(db.auto_inject_db_url_for_project(project).is_none());
    assert!(db.env_var_name_for_project(project).is_none());
}

#[test]
fn database_only_product_resolves_its_owned_database_without_attachment() {
    let project = "11111111-1111-4111-8111-111111111111";
    let mut owned = database("db-owned", "primary", &[]);
    owned.product_id = Some(project.into());
    let databases = vec![owned, database("db-legacy", "legacy", &[])];
    assert_eq!(
        select_database(&databases, Some(project), None).unwrap().id,
        "db-owned"
    );
    assert!(select_database(&databases, Some(project), Some("legacy")).is_err());
    assert!(select_database(&databases, None, None).is_err());
}
