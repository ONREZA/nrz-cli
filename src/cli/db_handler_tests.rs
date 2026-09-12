use super::*;

fn database(id: &str, name: &str, projects: &[&str]) -> ManagedDatabase {
    ManagedDatabase {
        id: id.to_string(),
        db_name: Some(name.to_string()),
        status: None,
        cu_size: None,
        pg_version: None,
        auto_inject_db_url: None,
        env_var_name: None,
        auto_create_preview_branch: None,
        kaiki_status: None,
        project_attachments: projects
            .iter()
            .map(|id| ProjectAttachment {
                project_id: (*id).into(),
                auto_inject_db_url: None,
                env_var_name: None,
                auto_create_preview_branch: None,
            })
            .collect(),
    }
}

#[test]
fn explicit_database_id_must_be_attached_to_project() {
    let databases = vec![
        database("db-current", "current", &["project-current"]),
        database("db-other", "other", &["project-other"]),
    ];
    assert!(select_database(&databases, "project-current", Some("db-other")).is_err());
    assert_eq!(
        select_database(&databases, "project-current", Some("db-current"))
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
fn project_attachment_body_serializes_server_contract() {
    let body = nrz_api::AttachmentRequestBody {
        env_var_name: Some("DATABASE_URL".into()),
        auto_inject_db_url: Some(true),
        auto_create_preview_branch: Some(false),
    };
    assert_eq!(
        serde_json::to_value(body).unwrap(),
        serde_json::json!({"envVarName":"DATABASE_URL","autoInjectDbUrl":true,"autoCreatePreviewBranch":false})
    );
}

#[test]
fn managed_database_uses_project_attachment_settings() {
    let mut db = database("db", "primary", &["project"]);
    db.project_attachments[0].auto_inject_db_url = Some(true);
    db.project_attachments[0].env_var_name = Some("APP_DATABASE_URL".into());
    db.project_attachments[0].auto_create_preview_branch = Some(true);
    assert_eq!(db.auto_inject_db_url_for_project("project"), Some(true));
    assert_eq!(
        db.env_var_name_for_project("project"),
        Some("APP_DATABASE_URL")
    );
    assert_eq!(
        db.auto_create_preview_branch_for_project("project"),
        Some(true)
    );
}

#[test]
fn database_selection_requires_a_unique_attachment_or_exact_id() {
    let mut databases = vec![
        database("first", "same", &["project"]),
        database("second", "same", &["project"]),
    ];
    assert!(select_database(&databases, "project", None).is_err());
    assert!(select_database(&databases, "project", Some("same")).is_err());
    assert_eq!(
        select_database(&databases, "project", Some("second"))
            .unwrap()
            .id,
        "second"
    );
    databases[0].project_attachments[0].auto_inject_db_url = Some(true);
    assert_eq!(
        select_database(&databases, "project", None).unwrap().id,
        "first"
    );
    databases[1].project_attachments[0].auto_inject_db_url = Some(true);
    assert!(select_database(&databases, "project", None).is_err());
}
