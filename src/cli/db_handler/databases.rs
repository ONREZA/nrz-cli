use super::*;

pub(super) async fn cmd_list(
    client: &ApiClient,
    project_id: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let mut list: ListResponse = client
        .databases()
        .await
        .context("failed to list databases")?
        .into();
    if let Some(project_id) = project_id {
        list.data.retain(|db| db.belongs_to_project(project_id));
    }

    if json {
        output::json_output(&list);
        return Ok(());
    }

    if let Some(plan) = &list.plan {
        eprintln!("  Plan: {}", output::terminal_line(plan));
    }
    match &list.creation {
        nrz_api::ManagedDatabaseCreationChoices::ManagedDatabaseCreationChoicesAvailable(choices) => {
            let profiles: Vec<_> = choices.profiles.iter()
                .map(|profile| format!("PG {} / {} CU", profile.pg_version, profile.cu_size))
                .collect();
            eprintln!("  Create profiles: {}", profiles.join(", "));
        }
        nrz_api::ManagedDatabaseCreationChoices::ManagedDatabaseCreationChoicesUnavailable(_) => {
            eprintln!("  New database creation is unavailable");
        }
    }
    eprintln!();

    if list.data.is_empty() {
        eprintln!("  No databases found.");
        if matches!(
            &list.creation,
            nrz_api::ManagedDatabaseCreationChoices::ManagedDatabaseCreationChoicesAvailable(_)
        ) {
            eprintln!("  Create one with: nrz db create");
        }
        return Ok(());
    }

    for db in &list.data {
        let id = output::terminal_line(&db.id);
        let name = output::terminal_line(db.db_name.as_deref().unwrap_or("(unnamed)"));
        let status = output::terminal_line(db.status.as_deref().unwrap_or("unknown"));
        let cu = db.cu_size.map(|v| format!("{v}")).unwrap_or_default();
        let inject =
            if project_id.and_then(|id| db.auto_inject_db_url_for_project(id)) == Some(true) {
                " [auto-inject]"
            } else {
                ""
            };
        eprintln!(
            "  {} {} ({}CU, {}){inject}",
            console::style(id).dim(),
            console::style(name).bold(),
            cu,
            format_status(&status),
        );
    }
    Ok(())
}

pub(super) async fn cmd_create(
    client: &ApiClient,
    json: bool,
    name: Option<String>,
    cu_size: Option<f64>,
    idempotency_key: Option<String>,
    wait: bool,
) -> anyhow::Result<()> {
    let key = create_idempotency_key(idempotency_key)?;
    let body = wire::create_body(name, cu_size)?;
    output::status(
        json,
        "~",
        format!("Creating database (retry key {key})..."),
        output::Phase::Db,
    );
    let mut created = client
        .create_database(body, &key)
        .await
        .map(|response| wire::created(response, key.clone()))
        .map_err(|error| create_error(error, &key, None))?;

    if wait {
        return wait_for_create(client, json, &mut created)
            .await
            .map_err(|error| create_error(error, &key, Some(&created)));
    }

    if json {
        output::json_output(&created);
    } else {
        output::success(
            false,
            format!(
                "Database creation accepted: {} ({})",
                created.id,
                created.status,
            ),
            output::Phase::Db,
        );
        eprintln!("    Operation: {}", created.operation_id);
        eprintln!("    Check status with: nrz db info {}", created.id);
    }
    Ok(())
}

pub(super) fn create_idempotency_key(key: Option<String>) -> anyhow::Result<String> {
    let key = key.unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
    anyhow::ensure!(
        (1..=128).contains(&key.len())
            && key.as_bytes()[0].is_ascii_alphanumeric()
            && key.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(&byte)),
        "--idempotency-key requires 1..128 ASCII characters: start with a letter or digit; use letters, digits, _, ., :, or -"
    );
    Ok(key)
}

async fn wait_for_create(
    client: &ApiClient,
    json: bool,
    created: &mut CreateResponse,
) -> anyhow::Result<()> {
    output::status(
        json,
        "~",
        format!(
            "Waiting for database {} (operation {}) to become active...",
            created.id, created.operation_id
        ),
        output::Phase::Db,
    );
    for _ in 0..120 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let info = wire::database(client, &created.id)
            .await
            .context("failed to check database status")?;
        created.status = info.db.status.clone().unwrap_or_else(|| "unknown".into());
        match created.status.as_str() {
            "ACTIVE" => {
                output::success(json, format!("Database {} is active", created.id), output::Phase::Db);
                if json {
                    let mut result = serde_json::to_value(info)?;
                    result["idempotencyKey"] = serde_json::json!(created.idempotency_key);
                    result["operationId"] = serde_json::json!(created.operation_id);
                    result["workspaceId"] = serde_json::json!(created.workspace_id);
                    output::json_output(&result);
                }
                return Ok(());
            }
            "ERROR" => {
                bail!("database creation failed; check `nrz db info` for details")
            }
            _ => continue,
        }
    }
    bail!("timed out waiting for database to become active (~4 minutes)");
}

fn create_error(
    error: anyhow::Error,
    key: &str,
    created: Option<&CreateResponse>,
) -> anyhow::Error {
    let api_error = error.downcast_ref::<crate::api::StructuredApiError>();
    let code = api_error
        .map(|error| error.code.as_str())
        .unwrap_or("DB_CREATE_FAILED");
    let mut details = api_error
        .and_then(|error| error.details.clone())
        .unwrap_or_else(|| serde_json::json!({}));
    if !details.is_object() {
        details = serde_json::json!({"originalDetails": details});
    }
    details["idempotencyKey"] = serde_json::json!(key);
    let hint = if let Some(created) = created {
        details["databaseId"] = serde_json::json!(created.id);
        details["operationId"] = serde_json::json!(created.operation_id);
        details["workspaceId"] = serde_json::json!(created.workspace_id);
        details["status"] = serde_json::json!(created.status);
        format!(
            "Database creation was accepted (operation {}). Check status with: nrz db info {}. Any create retry must reuse --idempotency-key {key} and the same arguments",
            created.operation_id, created.id
        )
    } else {
        format!("The create result may be unknown. Retry the same arguments with --idempotency-key {key}")
    };
    crate::errors::CliError::new(code, format!("{error:#}"))
        .phase(output::Phase::Db)
        .details(details)
        .hint(hint)
        .with_source(error)
        .into_anyhow()
}

pub(super) async fn cmd_info(
    client: &ApiClient,
    project_id: Option<&str>,
    db_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let info = wire::database(client, db_id)
        .await
        .context("failed to get database info")?;

    if json {
        output::json_output(&info);
        return Ok(());
    }

    let db = &info.db;
    let name = output::terminal_line(db.db_name.as_deref().unwrap_or("(unnamed)"));
    let status = output::terminal_line(db.status.as_deref().unwrap_or("unknown"));
    eprintln!(
        "  {} {}",
        console::style(name).bold(),
        format_status(&status),
    );
    eprintln!("  ID:         {}", output::terminal_line(&db.id));
    if let Some(cu) = db.cu_size {
        eprintln!("  CU size:    {cu}");
    }
    if let Some(pg) = db.pg_version {
        eprintln!("  PostgreSQL: {pg}");
    }
    if let Some(true) = project_id.and_then(|id| db.auto_inject_db_url_for_project(id)) {
        let var = project_id
            .and_then(|id| db.env_var_name_for_project(id))
            .unwrap_or("DATABASE_URL");
        eprintln!("  Auto-inject: {}", output::terminal_line(var));
        if project_id.and_then(|id| db.auto_create_preview_branch_for_project(id)) == Some(true) {
            eprintln!("  Preview branches: enabled");
        }
    }
    if let Some(plan) = info.plan() {
        eprintln!("  Plan:       {}", output::terminal_line(plan));
    }
    if let Some(sizes) = info.allowed_cu_sizes() {
        let s: Vec<String> = sizes.iter().map(|v| format!("{v}")).collect();
        eprintln!("  Allowed CU: {}", s.join(", "));
    }

    Ok(())
}

pub(super) async fn cmd_delete(
    client: &ApiClient,
    db_id: &str,
    json: bool,
    force: bool,
) -> anyhow::Result<()> {
    if !force {
        if json || !std::io::stdin().is_terminal() {
            bail!("--force is required to delete database in non-interactive mode");
        }
        eprint!(
            "  {} Delete database {}? [y/N] ",
            console::style("?").yellow().bold(),
            output::terminal_line(db_id)
        );
        std::io::stderr().flush()?;
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            eprintln!("  Cancelled.");
            return Ok(());
        }
    }

    let response = client
        .delete_database(db_id)
        .await
        .context("failed to delete database")?;
    anyhow::ensure!(
        response.status == "deleted",
        "database deletion was not accepted"
    );

    if json {
        output::json_output(&serde_json::json!({"deleted": db_id}));
    } else {
        output::success(
            false,
            format!("Database {db_id} deleted"),
            output::Phase::Db,
        );
    }
    Ok(())
}

pub(super) async fn cmd_start_stop(
    client: &ApiClient,
    db_id: &str,
    json: bool,
    action: &str,
) -> anyhow::Result<()> {
    let resp = match action {
        "start" => client.start_database(db_id).await,
        "stop" => client.stop_database(db_id).await,
        _ => bail!("unsupported database action"),
    }
    .with_context(|| format!("failed to {action} database"))?;

    if json {
        output::json_output(&resp);
    } else {
        let status = resp.status;
        output::success(
            false,
            format!("Database {db_id}: {status}"),
            output::Phase::Db,
        );
    }
    Ok(())
}
