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
) -> anyhow::Result<()> {
    let branch: Branch = client
        .create_database_branch(
            db_id,
            nrz_api::BranchRequestBody {
                name: name.to_string(),
                ..Default::default()
            },
        )
        .await
        .context("failed to create branch")?
        .into();

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
    anyhow::ensure!(
        response.status == "deleted",
        "branch deletion was not accepted"
    );

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
