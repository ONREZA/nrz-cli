use super::*;

pub(super) fn binding_body(
    args: &ConfigArgs,
) -> anyhow::Result<Option<nrz_api::AttachmentRequestBody>> {
    let has_updates = args.auto_inject.is_some()
        || args.env_var.is_some()
        || args.preview_branches.is_some()
        || args.functions_default.is_some()
        || args.service_id.is_some()
        || args.product_environment_id.is_some()
        || args.owner_product_environment_id.is_some()
        || args.provider_branch_id.is_some();
    if !has_updates {
        return Ok(None);
    }
    let required = |value: Option<uuid::Uuid>, flag: &str| {
        value.filter(|id| !id.is_nil()).ok_or_else(|| {
            output::coded_error(
                "BINDING_TARGET_REQUIRED",
                format!("--{flag} must identify the exact binding target"),
            )
        })
    };
    let service_id = required(args.service_id, "service-id")?;
    let product_environment_id = required(args.product_environment_id, "product-environment-id")?;
    let owner_product_environment_id = required(
        args.owner_product_environment_id,
        "owner-product-environment-id",
    )?;
    let provider_branch_id = args
        .provider_branch_id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| {
            output::coded_error(
                "BINDING_TARGET_REQUIRED",
                "--provider-branch-id must identify the exact provider branch",
            )
        })?;
    Ok(Some(nrz_api::AttachmentRequestBody {
        service_id,
        product_environment_id,
        owner_product_environment_id,
        provider_branch_id: provider_branch_id.to_owned(),
        env_var_name: args.env_var.clone(),
        auto_inject_db_url: args.auto_inject,
        auto_create_preview_branch: args.preview_branches,
        is_functions_default: args.functions_default,
        additional_properties: Default::default(),
    }))
}

pub(super) async fn cmd_config(
    client: &ApiClient,
    project_id: &str,
    db_id: &str,
    json: bool,
    args: ConfigArgs,
) -> anyhow::Result<()> {
    if let Some(body) = binding_body(&args)? {
        let resp = wire::attach(client, db_id, project_id, body)
            .await
            .context("failed to update Service database binding")?;
        if json {
            output::json_output(&resp);
        } else {
            output::success(false, "Service database binding updated", output::Phase::Db);
        }
    } else {
        let info = wire::database(client, db_id)
            .await
            .context("failed to get database info")?;
        let bindings: Vec<_> = info
            .db
            .project_attachments
            .iter()
            .filter(|binding| binding.project_id.to_string() == project_id)
            .collect();
        if json {
            output::json_output(&serde_json::json!({"bindings": bindings}));
        } else if bindings.is_empty() {
            eprintln!(
                "  No Service bindings in this Product. Set all four binding target flags to create one."
            );
        } else {
            for binding in bindings {
                eprintln!(
                    "  Service: {}  ProductEnvironment: {}",
                    binding.service_id, binding.product_environment_id
                );
                eprintln!(
                    "    Owner ProductEnvironment: {}  Provider branch: {}",
                    binding.owner_product_environment_id,
                    output::terminal_line(&binding.provider_branch_id)
                );
                eprintln!(
                    "    {}: {}  Preview branches: {}  Functions default: {}",
                    output::terminal_line(&binding.env_var_name),
                    binding.auto_inject_db_url,
                    binding.auto_create_preview_branch,
                    binding.is_functions_default
                );
            }
        }
    }
    Ok(())
}
