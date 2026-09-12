use super::*;

pub(super) async fn cmd_connection(
    client: &ApiClient,
    db_id: &str,
    json: bool,
    branch: Option<&str>,
) -> anyhow::Result<()> {
    let uri = fetch_connection_uri(client, db_id, branch).await?;

    if json {
        output::json_output(&serde_json::json!({"connectionUri": uri}));
    } else {
        println!("{uri}");
    }
    Ok(())
}

pub(super) async fn cmd_query(
    client: &ApiClient,
    db_id: &str,
    json: bool,
    sql: &str,
    branch: Option<&str>,
) -> anyhow::Result<()> {
    let connection_uri = fetch_connection_uri(client, db_id, branch).await?;
    let result = execute_sql_locally(&connection_uri, sql)
        .await
        .context("query failed")?;

    if json {
        output::json_output(&result);
        return Ok(());
    }

    // Human-readable table output
    if !result.columns.is_empty() {
        eprintln!("  {}", output::terminal_line(&result.columns.join(" | ")));
        eprintln!("  {}", "-".repeat(result.columns.len() * 12));
    }
    for row in &result.rows {
        let vals: Vec<String> = row.iter().map(format_cell).collect();
        eprintln!("  {}", output::terminal_line(&vals.join(" | ")));
    }
    eprintln!();
    eprintln!("  ({} row(s))", result.row_count);
    eprintln!("  Time: {:.1}ms", result.duration_ms);

    Ok(())
}

pub(super) async fn cmd_schema(
    client: &ApiClient,
    db_id: &str,
    json: bool,
    branch: Option<&str>,
) -> anyhow::Result<()> {
    let connection_uri = fetch_connection_uri(client, db_id, branch).await?;

    // Query table list
    let tables_sql = r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = 'public'
        ORDER BY table_name
    "#;
    let tables_result = execute_sql_locally(&connection_uri, tables_sql)
        .await
        .context("failed to query schema")?;

    let table_names: Vec<String> = tables_result
        .rows
        .iter()
        .filter_map(|r| value_as_str(r, 0).map(String::from))
        .collect();

    // Query columns for all tables
    let columns_sql = r#"
        SELECT table_name, column_name, data_type, is_nullable, column_default
        FROM information_schema.columns
        WHERE table_schema = 'public'
        ORDER BY table_name, ordinal_position
    "#;
    let cols_result = execute_sql_locally(&connection_uri, columns_sql)
        .await
        .context("failed to query columns")?;

    let mut schema = SchemaOutput { tables: Vec::new() };

    for name in &table_names {
        let columns: Vec<SchemaColumn> = cols_result
            .rows
            .iter()
            .filter(|r| value_as_str(r, 0) == Some(name.as_str()))
            .map(|r| SchemaColumn {
                name: value_as_str(r, 1).unwrap_or("").to_string(),
                col_type: value_as_str(r, 2).unwrap_or("").to_string(),
                nullable: value_as_str(r, 3) == Some("YES"),
                default: value_as_str(r, 4).map(String::from),
            })
            .collect();

        schema.tables.push(SchemaTable {
            name: name.clone(),
            columns,
        });
    }

    if json {
        output::json_output(&schema);
    } else {
        if schema.tables.is_empty() {
            eprintln!("  No tables found in public schema.");
            return Ok(());
        }
        for table in &schema.tables {
            let table_name = output::terminal_line(&table.name);
            eprintln!(
                "  {} {}",
                console::style("TABLE").dim(),
                console::style(table_name).bold()
            );
            for col in &table.columns {
                let nullable = if col.nullable { "NULL" } else { "NOT NULL" };
                let default = col
                    .default
                    .as_ref()
                    .map(|d| format!(" DEFAULT {d}"))
                    .unwrap_or_default();
                let name = output::terminal_line(&col.name);
                let column_type = output::terminal_line(&col.col_type);
                let default = output::terminal_line(&default);
                eprintln!(
                    "    {} {} {}{default}",
                    console::style(name).cyan(),
                    column_type,
                    nullable,
                );
            }
            eprintln!();
        }
    }
    Ok(())
}
