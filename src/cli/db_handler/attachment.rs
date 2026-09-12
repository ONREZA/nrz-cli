use super::*;

pub(super) async fn cmd_config(
    client: &ApiClient,
    project_id: &str,
    db_id: &str,
    json: bool,
    args: ConfigArgs,
) -> anyhow::Result<()> {
    let has_updates =
        args.auto_inject.is_some() || args.env_var.is_some() || args.preview_branches.is_some();

    if has_updates {
        let body = nrz_api::AttachmentRequestBody {
            env_var_name: args.env_var,
            auto_inject_db_url: args.auto_inject,
            auto_create_preview_branch: args.preview_branches,
        };
        let resp = wire::attach(client, db_id, project_id, body)
            .await
            .context("failed to update project database attachment")?;

        if json {
            output::json_output(&resp);
        } else {
            output::success(
                false,
                "Database project settings updated",
                output::Phase::Db,
            );
        }
    } else {
        // Show current settings
        let info: DbInfoResponse = wire::database(client, db_id)
            .await
            .context("failed to get database info")?
            .into();

        if json {
            output::json_output(&serde_json::json!({
                "autoInject": info.db.auto_inject_db_url_for_project(project_id),
                "envVar": info.db.env_var_name_for_project(project_id),
                "previewBranches": info.db.auto_create_preview_branch_for_project(project_id),
            }));
        } else {
            let enabled = info.db.auto_inject_db_url_for_project(project_id) == Some(true);
            let var = info
                .db
                .env_var_name_for_project(project_id)
                .unwrap_or("DATABASE_URL");
            let preview = info.db.auto_create_preview_branch_for_project(project_id) == Some(true);
            eprintln!("  Auto-inject:      {}", if enabled { "on" } else { "off" });
            eprintln!("  Env variable:     {}", output::terminal_line(var));
            eprintln!("  Preview branches: {}", if preview { "on" } else { "off" });
        }
    }
    Ok(())
}
