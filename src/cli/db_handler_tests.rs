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
            .map(|project_id| ProjectAttachment {
                project_id: (*project_id).to_string(),
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

    assert!(find_attached_database(&databases, "project-current", "db-other").is_none());
    assert_eq!(
        find_attached_database(&databases, "project-current", "db-current")
            .map(|db| db.id.as_str()),
        Some("db-current")
    );
}
use tokio_postgres::config::{Host, SslMode};

#[test]
fn create_body_serializes_server_contract() {
    let body = CreateBody {
        db_name: Some("primary".to_string()),
        cu_size: Some(0.5),
    };

    let value = serde_json::to_value(body).expect("create body serializes");

    assert_eq!(
        value,
        serde_json::json!({
            "dbName": "primary",
            "cuSize": 0.5,
        })
    );
    assert!(value.get("name").is_none());
}

#[test]
fn db_urls_use_current_kaiki_server_routes() {
    assert_eq!(KAIKI_DATABASES_BASE, "/v1/kaiki/databases");
    assert_eq!(
        project_attachment_url(KAIKI_DATABASES_BASE, "db-1", "project-1"),
        "/v1/kaiki/databases/db-1/attachments/project-1"
    );
}

#[test]
fn project_attachment_body_serializes_server_contract() {
    let body = ProjectAttachmentBody {
        env_var_name: Some("DATABASE_URL".to_string()),
        auto_inject_db_url: Some(true),
        auto_create_preview_branch: Some(false),
    };

    let value = serde_json::to_value(body).expect("attachment body serializes");

    assert_eq!(
        value,
        serde_json::json!({
            "envVarName": "DATABASE_URL",
            "autoInjectDbUrl": true,
            "autoCreatePreviewBranch": false,
        })
    );
}

#[test]
fn managed_database_uses_project_attachment_settings() {
    let db: ManagedDatabase = serde_json::from_value(serde_json::json!({
        "id": "db-1",
        "dbName": "primary",
        "autoInjectDbUrl": false,
        "projectAttachments": [
            {
                "projectId": "project-1",
                "autoInjectDbUrl": true,
                "envVarName": "APP_DATABASE_URL",
                "autoCreatePreviewBranch": true
            }
        ]
    }))
    .expect("managed database deserializes");

    assert!(db.is_attached_to_project("project-1"));
    assert_eq!(db.auto_inject_db_url_for_project("project-1"), Some(true));
    assert_eq!(
        db.env_var_name_for_project("project-1"),
        Some("APP_DATABASE_URL")
    );
    assert_eq!(
        db.auto_create_preview_branch_for_project("project-1"),
        Some(true)
    );
}

#[test]
fn branch_list_deserializes_current_server_array() {
    let list: BranchListResponse = serde_json::from_value(serde_json::json!([
        {"id": "branch-1", "name": "main", "status": "ACTIVE"}
    ]))
    .expect("server branch array deserializes");

    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "branch-1");
    assert_eq!(list.data[0].name, "main");
}

#[test]
fn branch_list_deserializes_legacy_data_envelope() {
    let list: BranchListResponse = serde_json::from_value(serde_json::json!({
        "data": [
            {"id": "branch-2", "name": "preview", "isPreviewBranch": true}
        ]
    }))
    .expect("legacy branch envelope deserializes");

    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "branch-2");
    assert_eq!(list.data[0].is_preview_branch, Some(true));
}

#[test]
fn connection_response_deserializes_server_snake_case_uri() {
    let resp: ConnectionResponse = serde_json::from_value(serde_json::json!({
        "connection_uri": "postgres://user:pass@host/db"
    }))
    .expect("server connection response deserializes");

    assert_eq!(resp.connection_uri, "postgres://user:pass@host/db");
}

#[test]
fn postgres_config_from_uri_requires_tls_and_ignores_non_client_params() {
    let config = postgres_config_from_uri(
        "postgres://user%40mail:p%40ss@db.example.com:6543/app%2Ddb?sslmode=require&schema=public&pgbouncer=true&application_name=nrz",
    )
    .expect("connection URI parses");

    assert_eq!(config.get_user(), Some("user@mail"));
    assert_eq!(config.get_password(), Some("p@ss".as_bytes()));
    assert_eq!(config.get_dbname(), Some("app-db"));
    assert_eq!(config.get_ports(), &[6543]);
    assert_eq!(config.get_ssl_mode(), SslMode::Require);
    assert_eq!(config.get_application_name(), Some("nrz"));
    assert_eq!(config.get_connect_timeout(), Some(&QUERY_TIMEOUT));
    assert!(matches!(config.get_hosts(), [Host::Tcp(host)] if host == "db.example.com"));
}

#[test]
fn postgres_config_from_uri_allows_connect_timeout_override() {
    let config =
        postgres_config_from_uri("postgres://user:pass@db.example.com/app?connect_timeout=5")
            .expect("connection URI parses");

    assert_eq!(
        config.get_connect_timeout(),
        Some(&std::time::Duration::from_secs(5))
    );
}

#[test]
fn postgres_config_from_uri_rejects_unknown_sslmode() {
    let err = postgres_config_from_uri("postgres://user:pass@db.example.com/app?sslmode=surprise")
        .expect_err("unknown sslmode is rejected");

    assert!(format!("{err:#}").contains("unsupported PostgreSQL sslmode"));
}

#[test]
fn sql_execution_mode_uses_typed_rows_for_row_queries() {
    assert_eq!(sql_execution_mode("SELECT 1"), SqlExecutionMode::RowCapable);
    assert_eq!(
        sql_execution_mode("-- comment\n/* block */ WITH rows AS (SELECT 1) SELECT * FROM rows"),
        SqlExecutionMode::RowCapable
    );
    assert_eq!(
        sql_execution_mode("insert into events(name) values ('created') returning id"),
        SqlExecutionMode::RowCapable
    );
    assert_eq!(
        sql_execution_mode("UPDATE events SET name = 'done'"),
        SqlExecutionMode::RowCapable
    );
}

#[test]
fn sql_execution_mode_uses_simple_protocol_for_ddl_commands() {
    assert_eq!(
        sql_execution_mode("CREATE TABLE IF NOT EXISTS events(id int)"),
        SqlExecutionMode::SimpleCommand
    );
}

#[tokio::test]
#[ignore = "requires NRZ_LIVE_DB_URL"]
async fn live_db_query_uses_local_postgres_client() {
    let url = std::env::var("NRZ_LIVE_DB_URL").expect("NRZ_LIVE_DB_URL is required");

    let result = execute_sql_locally(
        &url,
        "SELECT 1::int4 AS id, true AS ok, NULL::text AS missing",
    )
    .await
    .expect("live query succeeds");

    assert_eq!(result.columns, vec!["id", "ok", "missing"]);
    assert_eq!(
        result.rows,
        vec![vec![
            serde_json::Value::Number(1.into()),
            serde_json::Value::Bool(true),
            serde_json::Value::Null,
        ]]
    );
    assert_eq!(result.row_count, 1);
}
