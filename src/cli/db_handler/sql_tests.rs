use super::*;
use tokio_postgres::config::{Host, SslMode};

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
