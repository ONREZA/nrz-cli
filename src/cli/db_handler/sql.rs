use super::QueryResult;
use anyhow::{Context, bail};
use futures::StreamExt;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tokio_postgres::{
    Client, Row, SimpleQueryMessage,
    config::SslMode,
    types::{ToSql, Type},
};
use tokio_postgres_rustls::MakeRustlsConnect;
use url::Url;
const QUERY_TIMEOUT: Duration = Duration::from_secs(30);
const QUERY_MAX_ROWS: usize = 1000;
pub(super) async fn execute_sql_locally(
    connection_uri: &str,
    sql: &str,
) -> anyhow::Result<QueryResult> {
    let config = postgres_config_from_uri(connection_uri)?;
    let tls = make_tls_connector()?;
    let start = Instant::now();
    let (client, connection_task) = connect_postgres(config, tls).await?;
    let result = match sql_execution_mode(sql) {
        SqlExecutionMode::RowCapable => execute_typed_query(&client, sql, start).await,
        SqlExecutionMode::SimpleCommand => execute_simple_query(&client, sql, start).await,
    };
    drop(client);
    connection_task.abort();
    let _ = connection_task.await;

    result
}

async fn connect_postgres(
    config: tokio_postgres::Config,
    tls: MakeRustlsConnect,
) -> anyhow::Result<(Client, JoinHandle<()>)> {
    let (client, connection) = tokio::time::timeout(QUERY_TIMEOUT, config.connect(tls))
        .await
        .map_err(|_| connection_timeout_error())?
        .context("failed to connect to database")?;
    let connection_task = tokio::spawn(async move {
        if let Err(error) = connection.await {
            tracing::warn!("database connection error: {error}");
        }
    });

    Ok((client, connection_task))
}

fn make_tls_connector() -> anyhow::Result<MakeRustlsConnect> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let (connector, cert_errors) = MakeRustlsConnect::with_native_certs()
        .map_err(|errors| anyhow::anyhow!("failed to load native TLS roots: {errors:?}"))?;
    if !cert_errors.is_empty() {
        tracing::warn!("some native TLS roots failed to load: {cert_errors:?}");
    }
    Ok(connector)
}

fn postgres_config_from_uri(uri: &str) -> anyhow::Result<tokio_postgres::Config> {
    let url = Url::parse(uri).context("invalid PostgreSQL connection URI")?;
    match url.scheme() {
        "postgres" | "postgresql" => {}
        scheme => bail!("unsupported PostgreSQL connection URI scheme: {scheme}"),
    }

    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("PostgreSQL connection URI is missing host"))?;
    let user = decode_uri_component(url.username(), "username")?;
    if user.is_empty() {
        bail!("PostgreSQL connection URI is missing username");
    }
    let database = decode_uri_component(url.path().trim_start_matches('/'), "database")?;
    if database.is_empty() {
        bail!("PostgreSQL connection URI is missing database name");
    }

    let mut config = tokio_postgres::Config::new();
    config
        .host(host)
        .port(url.port().unwrap_or(5432))
        .user(user)
        .dbname(database)
        .ssl_mode(SslMode::Require)
        .connect_timeout(QUERY_TIMEOUT);
    if let Some(password) = url.password() {
        config.password(decode_uri_component(password, "password")?);
    }

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "application_name" => {
                config.application_name(value.into_owned());
            }
            "connect_timeout" => {
                let seconds = value
                    .parse::<u64>()
                    .with_context(|| format!("invalid PostgreSQL connect_timeout: {value}"))?;
                config.connect_timeout(Duration::from_secs(seconds));
            }
            "options" => {
                config.options(value.into_owned());
            }
            "sslmode" => validate_sslmode(&value)?,
            _ => {}
        }
    }

    Ok(config)
}

fn validate_sslmode(value: &str) -> anyhow::Result<()> {
    match value {
        "disable" | "allow" | "prefer" | "require" | "verify-ca" | "verify-full" => Ok(()),
        other => bail!("unsupported PostgreSQL sslmode: {other}"),
    }
}

fn decode_uri_component(value: &str, label: &str) -> anyhow::Result<String> {
    percent_encoding::percent_decode_str(value)
        .decode_utf8()
        .map(std::borrow::Cow::into_owned)
        .with_context(|| format!("invalid percent-encoding in PostgreSQL URI {label}"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SqlExecutionMode {
    RowCapable,
    SimpleCommand,
}

fn sql_execution_mode(sql: &str) -> SqlExecutionMode {
    match first_sql_keyword(sql).as_deref() {
        Some(
            "delete" | "explain" | "insert" | "merge" | "select" | "show" | "table" | "update"
            | "values" | "with",
        ) => SqlExecutionMode::RowCapable,
        _ => SqlExecutionMode::SimpleCommand,
    }
}

fn first_sql_keyword(sql: &str) -> Option<String> {
    let mut rest = sql.trim_start();
    loop {
        if let Some(after_comment) = rest.strip_prefix("--") {
            rest = after_comment
                .split_once('\n')
                .map(|(_, tail)| tail.trim_start())
                .unwrap_or("");
            continue;
        }
        if let Some(after_comment) = rest.strip_prefix("/*") {
            rest = after_comment
                .split_once("*/")
                .map(|(_, tail)| tail.trim_start())
                .unwrap_or("");
            continue;
        }
        break;
    }

    let keyword: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_lowercase())
        .collect();
    if keyword.is_empty() {
        None
    } else {
        Some(keyword)
    }
}

async fn execute_typed_query(
    client: &Client,
    sql: &str,
    start: Instant,
) -> anyhow::Result<QueryResult> {
    let statement = match tokio::time::timeout(QUERY_TIMEOUT, client.prepare(sql)).await {
        Ok(Ok(statement)) => statement,
        Ok(Err(_)) => return execute_simple_query(client, sql, start).await,
        Err(_) => return Err(query_timeout_error()),
    };

    if statement.columns().is_empty() {
        return execute_simple_query(client, sql, start).await;
    }

    if !statement
        .columns()
        .iter()
        .all(|column| supports_typed_json(column.type_()))
    {
        return execute_simple_query(client, sql, start).await;
    }

    let params = std::iter::empty::<&(dyn ToSql + Sync)>();
    let stream = tokio::time::timeout(QUERY_TIMEOUT, client.query_raw(&statement, params))
        .await
        .map_err(|_| query_timeout_error())?
        .context("failed to execute SQL")?;
    match tokio::time::timeout(
        QUERY_TIMEOUT,
        typed_query_result_from_stream(stream, start, statement.columns()),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(query_timeout_error()),
    }
}

async fn execute_simple_query(
    client: &Client,
    sql: &str,
    start: Instant,
) -> anyhow::Result<QueryResult> {
    let stream = tokio::time::timeout(QUERY_TIMEOUT, client.simple_query_raw(sql))
        .await
        .map_err(|_| query_timeout_error())?
        .context("failed to execute SQL")?;
    match tokio::time::timeout(
        QUERY_TIMEOUT,
        simple_query_result_from_stream(stream, start),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(query_timeout_error()),
    }
}

async fn typed_query_result_from_stream(
    stream: tokio_postgres::RowStream,
    start: Instant,
    columns: &[tokio_postgres::Column],
) -> anyhow::Result<QueryResult> {
    let columns = columns.iter().map(|col| col.name().to_string()).collect();
    let mut rows = Vec::new();
    let mut selected_row_count = 0_i64;

    futures::pin_mut!(stream);
    while let Some(row) = stream.next().await {
        let row = row.context("failed to execute SQL")?;
        selected_row_count = selected_row_count.saturating_add(1);
        if rows.len() < QUERY_MAX_ROWS {
            rows.push(row_to_json_values(&row)?);
        }
    }

    Ok(QueryResult {
        columns,
        rows,
        row_count: selected_row_count,
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

fn supports_typed_json(ty: &Type) -> bool {
    ty == &Type::BOOL
        || ty == &Type::INT2
        || ty == &Type::INT4
        || ty == &Type::INT8
        || ty == &Type::FLOAT4
        || ty == &Type::FLOAT8
        || ty == &Type::TEXT
        || ty == &Type::VARCHAR
        || ty == &Type::BPCHAR
        || ty == &Type::NAME
        || ty == &Type::UNKNOWN
        || ty == &Type::JSON
        || ty == &Type::JSONB
        || ty == &Type::UUID
        || ty == &Type::OID
}

fn row_to_json_values(row: &Row) -> anyhow::Result<Vec<serde_json::Value>> {
    (0..row.len())
        .map(|idx| cell_to_json_value(row, idx))
        .collect()
}

fn cell_to_json_value(row: &Row, idx: usize) -> anyhow::Result<serde_json::Value> {
    let ty = row.columns()[idx].type_();
    if ty == &Type::BOOL {
        let value = row.try_get::<usize, Option<bool>>(idx)?;
        return Ok(optional_json(value, serde_json::Value::Bool));
    }
    if ty == &Type::INT2 {
        let value = row.try_get::<usize, Option<i16>>(idx)?;
        return Ok(optional_json(value, |value| {
            serde_json::Value::Number(i64::from(value).into())
        }));
    }
    if ty == &Type::INT4 {
        let value = row.try_get::<usize, Option<i32>>(idx)?;
        return Ok(optional_json(value, |value| {
            serde_json::Value::Number(i64::from(value).into())
        }));
    }
    if ty == &Type::INT8 {
        let value = row.try_get::<usize, Option<i64>>(idx)?;
        return Ok(optional_json(value, |value| {
            serde_json::Value::Number(value.into())
        }));
    }
    if ty == &Type::OID {
        let value = row.try_get::<usize, Option<u32>>(idx)?;
        return Ok(optional_json(value, |value| {
            serde_json::Value::Number(u64::from(value).into())
        }));
    }
    if ty == &Type::FLOAT4 {
        let value = row.try_get::<usize, Option<f32>>(idx)?;
        return Ok(optional_json(value, |value| {
            json_number_from_f64(value.into())
        }));
    }
    if ty == &Type::FLOAT8 {
        let value = row.try_get::<usize, Option<f64>>(idx)?;
        return Ok(optional_json(value, json_number_from_f64));
    }
    if ty == &Type::JSON || ty == &Type::JSONB {
        let value = row.try_get::<usize, Option<serde_json::Value>>(idx)?;
        return Ok(value.unwrap_or(serde_json::Value::Null));
    }
    if ty == &Type::UUID {
        let value = row.try_get::<usize, Option<uuid::Uuid>>(idx)?;
        return Ok(optional_json(value, |value| {
            serde_json::Value::String(value.to_string())
        }));
    }
    if is_text_type(ty) {
        let value = row.try_get::<usize, Option<String>>(idx)?;
        return Ok(optional_json(value, serde_json::Value::String));
    }

    bail!("unsupported PostgreSQL result type: {}", ty.name());
}

fn optional_json<T>(
    value: Option<T>,
    convert: impl FnOnce(T) -> serde_json::Value,
) -> serde_json::Value {
    value.map(convert).unwrap_or(serde_json::Value::Null)
}

fn json_number_from_f64(value: f64) -> serde_json::Value {
    serde_json::Number::from_f64(value)
        .map(serde_json::Value::Number)
        .unwrap_or(serde_json::Value::Null)
}

fn is_text_type(ty: &Type) -> bool {
    ty == &Type::TEXT
        || ty == &Type::VARCHAR
        || ty == &Type::BPCHAR
        || ty == &Type::NAME
        || ty == &Type::UNKNOWN
}

async fn simple_query_result_from_stream(
    stream: tokio_postgres::SimpleQueryStream,
    start: Instant,
) -> anyhow::Result<QueryResult> {
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut command_row_count = 0_i64;
    let mut selected_row_count = 0_i64;

    futures::pin_mut!(stream);
    while let Some(message) = stream.next().await {
        match message {
            Err(error) => return Err(error).context("failed to execute SQL"),
            Ok(message) => apply_query_message(
                message,
                &mut columns,
                &mut rows,
                &mut command_row_count,
                &mut selected_row_count,
            ),
        }
    }

    let row_count = if selected_row_count > 0 {
        selected_row_count
    } else {
        command_row_count
    };

    Ok(QueryResult {
        columns,
        rows,
        row_count,
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

fn apply_query_message(
    message: SimpleQueryMessage,
    columns: &mut Vec<String>,
    rows: &mut Vec<Vec<serde_json::Value>>,
    command_row_count: &mut i64,
    selected_row_count: &mut i64,
) {
    match message {
        SimpleQueryMessage::RowDescription(desc) if rows.is_empty() => {
            *columns = desc.iter().map(|col| col.name().to_string()).collect();
        }
        SimpleQueryMessage::Row(row) => {
            *selected_row_count = selected_row_count.saturating_add(1);
            if rows.len() < QUERY_MAX_ROWS {
                if columns.is_empty() {
                    *columns = row
                        .columns()
                        .iter()
                        .map(|col| col.name().to_string())
                        .collect();
                }
                let values = (0..row.len())
                    .map(|idx| match row.get(idx) {
                        Some(value) => serde_json::Value::String(value.to_string()),
                        None => serde_json::Value::Null,
                    })
                    .collect();
                rows.push(values);
            }
        }
        SimpleQueryMessage::CommandComplete(count) => {
            *command_row_count = i64::try_from(count).unwrap_or(i64::MAX);
        }
        SimpleQueryMessage::RowDescription(_) => {}
        _ => {}
    }
}

fn query_timeout_error() -> anyhow::Error {
    anyhow::anyhow!("query timed out after {}s", QUERY_TIMEOUT.as_secs())
}

fn connection_timeout_error() -> anyhow::Error {
    anyhow::anyhow!(
        "database connection timed out after {}s",
        QUERY_TIMEOUT.as_secs()
    )
}

#[cfg(test)]
#[path = "sql_tests.rs"]
mod tests;
