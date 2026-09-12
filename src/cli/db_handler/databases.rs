use super::*;

pub(super) async fn cmd_list(
    client: &ApiClient,
    project_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let mut list: ListResponse = client
        .databases()
        .await
        .context("failed to list databases")?
        .into();
    list.data.retain(|db| db.is_attached_to_project(project_id));

    if json {
        output::json_output(&list);
        return Ok(());
    }

    if list.data.is_empty() {
        eprintln!("  No databases found. Create one with: nrz db create");
        return Ok(());
    }

    if let Some(plan) = &list.plan {
        eprintln!("  Plan: {}", output::terminal_line(plan));
    }
    if let Some(sizes) = &list.allowed_cu_sizes {
        let s: Vec<String> = sizes.iter().map(|v| format!("{v}")).collect();
        eprintln!("  Allowed CU sizes: {}", s.join(", "));
    }
    eprintln!();

    for db in &list.data {
        let id = output::terminal_line(&db.id);
        let name = output::terminal_line(db.db_name.as_deref().unwrap_or("(unnamed)"));
        let status = output::terminal_line(db.status.as_deref().unwrap_or("unknown"));
        let cu = db.cu_size.map(|v| format!("{v}")).unwrap_or_default();
        let inject = if db.auto_inject_db_url_for_project(project_id) == Some(true) {
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
    project_id: &str,
    json: bool,
    name: Option<String>,
    cu_size: Option<f64>,
    wait: bool,
) -> anyhow::Result<()> {
    output::status(json, "~", "Creating database...", output::Phase::Db);

    let body = wire::create_body(name, cu_size)?;
    let created: CreateResponse = client
        .create_database(body)
        .await
        .context("failed to create database")?
        .into();
    wire::attach(
        client,
        &created.id,
        project_id,
        nrz_api::AttachmentRequestBody::default(),
    )
    .await
    .with_context(|| {
        format!(
            "database {} was created, but failed to attach it to project {}",
            created.id, project_id
        )
    })?;

    if wait {
        output::status(
            json,
            "~",
            "Waiting for database to become active...",
            output::Phase::Db,
        );
        for _ in 0..120 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let info = wire::database(client, &created.id)
                .await
                .context("failed to check database status")?;
            match info.status {
                nrz_api::Database200ResponseDatumStatus::Active => {
                    output::success(json, "Database is active", output::Phase::Db);
                    if json {
                        output::json_output(&DbInfoResponse::from(info));
                    }
                    return Ok(());
                }
                nrz_api::Database200ResponseDatumStatus::Error => {
                    bail!("database creation failed; check `nrz db info` for details")
                }
                _ => continue,
            }
        }
        bail!(
            "timed out waiting for database to become active (~4 minutes). \
             Check status with: nrz db info"
        );
    }

    if json {
        output::json_output(&created);
    } else {
        let name = created.db_name.as_deref().unwrap_or("kaikidb");
        output::success(
            json,
            format!("Database created: {} ({})", name, created.id),
            output::Phase::Db,
        );
        if created.status.as_deref() == Some("CREATING") {
            eprintln!("    Database is being provisioned. Use `nrz db info` to check status.");
        }
    }
    Ok(())
}

pub(super) async fn cmd_info(
    client: &ApiClient,
    project_id: &str,
    db_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let info: DbInfoResponse = wire::database(client, db_id)
        .await
        .context("failed to get database info")?
        .into();

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
    if let Some(true) = db.auto_inject_db_url_for_project(project_id) {
        let var = db
            .env_var_name_for_project(project_id)
            .unwrap_or("DATABASE_URL");
        eprintln!("  Auto-inject: {}", output::terminal_line(var));
        if db.auto_create_preview_branch_for_project(project_id) == Some(true) {
            eprintln!("  Preview branches: enabled");
        }
    }
    if let Some(plan) = &info.plan {
        eprintln!("  Plan:       {}", output::terminal_line(plan));
    }
    if let Some(sizes) = &info.allowed_cu_sizes {
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
