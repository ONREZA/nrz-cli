use super::*;

pub(super) async fn cmd_branches_list(
    client: &ApiClient,
    db_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let list = BranchListResponse {
        data: client
            .database_branches(db_id)
            .await
            .context("failed to list branches")?
            .into_iter()
            .map(Into::into)
            .collect(),
    };

    if json {
        output::json_output(&list);
        return Ok(());
    }

    if list.data.is_empty() {
        eprintln!("  No branches found.");
        return Ok(());
    }

    for b in &list.data {
        let id = output::terminal_line(&b.id);
        let name = output::terminal_line(&b.name);
        let status = output::terminal_line(b.status.as_deref().unwrap_or("unknown"));
        let preview = if b.is_preview_branch == Some(true) {
            " (preview)"
        } else {
            ""
        };
        eprintln!(
            "  {} {}{preview} ({})",
            console::style(id).dim(),
            console::style(name).bold(),
            format_status(&status),
        );
    }
    Ok(())
}

pub(super) async fn cmd_branches_create(
    client: &ApiClient,
    db_id: &str,
    json: bool,
    name: &str,
    idempotency_key: Option<uuid::Uuid>,
) -> anyhow::Result<()> {
    let key = idempotency_key.unwrap_or_else(uuid::Uuid::now_v7);
    output::status(
        json,
        "~",
        format!("Branch attempt: --idempotency-key {key}"),
        output::Phase::Db,
    );
    let response = client
        .create_database_branch(
            db_id,
            nrz_api::BranchRequestBody2 {
                name: name.to_string(),
                ..Default::default()
            },
            key,
        )
        .await
        .context("failed to create branch")?;
    let branch: Branch = match response {
        nrz_api::Branch200Response3::Object(value) => value.into(),
        nrz_api::Branch200Response3::Object2(value) => {
            let operation = DatabaseOperation::from(value).with_key(key);
            operation.verify_database(db_id)?;
            operation.verify_type(OperationType::BranchCreate)?;
            operation.check_outcome()?;
            print_operation(&operation, json);
            return Ok(());
        }
    };

    if json {
        output::json_output(&branch);
    } else {
        output::success(
            false,
            format!("Branch created: {} ({})", branch.name, branch.id),
            output::Phase::Db,
        );
    }
    Ok(())
}

pub(super) async fn cmd_branches_delete(
    client: &ApiClient,
    db_id: &str,
    json: bool,
    branch: &str,
) -> anyhow::Result<()> {
    let branch_id = resolve_branch(client, db_id, branch).await?;
    let response = client
        .delete_database_branch(db_id, &branch_id)
        .await
        .context("failed to delete branch")?;
    match response {
        nrz_api::Database200Response4::Object(value) => {
            let operation = DatabaseOperation::from(value);
            operation.verify_database(db_id)?;
            operation.verify_type(OperationType::BranchDelete)?;
            operation.verify_branch(&branch_id)?;
            operation.check_outcome()?;
            print_operation(&operation, json);
            return Ok(());
        }
        nrz_api::Database200Response4::Status(value) => anyhow::ensure!(
            value.status == "deleted",
            "branch deletion was not accepted"
        ),
    }

    if json {
        output::json_output(&serde_json::json!({"deleted": branch_id}));
    } else {
        output::success(false, format!("Branch {branch} deleted"), output::Phase::Db);
    }
    Ok(())
}

pub(super) async fn cmd_branch_connection(
    client: &ApiClient,
    db_id: &str,
    json: bool,
    branch: &str,
) -> anyhow::Result<()> {
    let uri = fetch_connection_uri(client, db_id, Some(branch)).await?;

    if json {
        output::json_output(&serde_json::json!({"connectionUri": uri}));
    } else {
        println!("{}", uri);
    }
    Ok(())
}
