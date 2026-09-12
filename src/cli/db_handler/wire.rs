use super::*;

pub(super) fn create_body(
    name: Option<String>,
    cu_size: Option<f64>,
) -> anyhow::Result<nrz_api::DatabaseRequestBody> {
    Ok(nrz_api::DatabaseRequestBody {
        db_name: name,
        cu_size: cu_size
            .map(|value| {
                serde_json::from_value(serde_json::json!(value))
                    .context("unsupported compute unit size")
            })
            .transpose()?,
        ..Default::default()
    })
}

fn cu(value: nrz_api::Database200ResponseAllowedCuSize) -> f64 {
    use nrz_api::Database200ResponseAllowedCuSize::*;
    match value {
        Value0_25 => 0.25,
        Value0_5 => 0.5,
        Value1 => 1.0,
        Value2 => 2.0,
        Value3 => 3.0,
        Value4 => 4.0,
        Value5 => 5.0,
        Value6 => 6.0,
        Value7 => 7.0,
        Value8 => 8.0,
    }
}

impl From<nrz_api::Database200ResponseDatumProjectAttachment> for ProjectAttachment {
    fn from(value: nrz_api::Database200ResponseDatumProjectAttachment) -> Self {
        Self {
            project_id: value.project_id.to_string(),
            auto_inject_db_url: Some(value.auto_inject_db_url),
            env_var_name: Some(value.env_var_name),
            auto_create_preview_branch: Some(value.auto_create_preview_branch),
        }
    }
}

impl From<nrz_api::Database200ResponseDatum> for ManagedDatabase {
    fn from(value: nrz_api::Database200ResponseDatum) -> Self {
        Self {
            id: value.id.to_string(),
            db_name: Some(value.db_name),
            status: Some(value.status.to_string()),
            cu_size: Some(value.cu_size),
            pg_version: Some(value.pg_version),
            auto_inject_db_url: None,
            env_var_name: None,
            auto_create_preview_branch: None,
            kaiki_status: None,
            project_attachments: value
                .project_attachments
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<nrz_api::Database200Response> for ListResponse {
    fn from(value: nrz_api::Database200Response) -> Self {
        Self {
            data: value.data.into_iter().map(Into::into).collect(),
            allowed_cu_sizes: Some(value.allowed_cu_sizes.into_iter().map(cu).collect()),
            autoscale_max_cu: value.autoscale_max_cu.map(cu),
            plan: Some(value.plan.to_string()),
        }
    }
}

impl From<nrz_api::Database200Response3> for DbInfoResponse {
    fn from(value: nrz_api::Database200Response3) -> Self {
        Self {
            db: ManagedDatabase {
                id: value.id.to_string(),
                db_name: Some(value.db_name),
                status: Some(value.status.to_string()),
                cu_size: Some(value.cu_size),
                pg_version: Some(value.pg_version),
                auto_inject_db_url: None,
                env_var_name: None,
                auto_create_preview_branch: None,
                kaiki_status: value.kaiki_status.map(|v| v.to_string()),
                project_attachments: value
                    .project_attachments
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            },
            allowed_cu_sizes: Some(value.allowed_cu_sizes.into_iter().map(cu).collect()),
            autoscale_max_cu: value.autoscale_max_cu.map(cu),
            plan: Some(value.plan.to_string()),
        }
    }
}

impl From<nrz_api::Database200Response2> for CreateResponse {
    fn from(value: nrz_api::Database200Response2) -> Self {
        Self {
            id: value.id.to_string(),
            db_name: Some(value.db_name),
            status: Some(value.status.to_string()),
        }
    }
}

impl From<nrz_api::ManagedDatabaseBranchListItem> for Branch {
    fn from(value: nrz_api::ManagedDatabaseBranchListItem) -> Self {
        Self {
            id: value.id,
            name: value.name,
            status: Some(value.status.to_string()),
            is_preview_branch: Some(value.is_preview_branch),
        }
    }
}
impl From<nrz_api::Branch200Response2> for Branch {
    fn from(value: nrz_api::Branch200Response2) -> Self {
        Self {
            id: value.id,
            name: value.name,
            status: Some(value.status.to_string()),
            is_preview_branch: None,
        }
    }
}

pub(super) async fn database(
    client: &ApiClient,
    id: &str,
) -> anyhow::Result<nrz_api::Database200Response3> {
    let value = client.database(id).await?;
    anyhow::ensure!(
        value.id == id.parse::<uuid::Uuid>()?,
        "database response belongs to another database"
    );
    Ok(value)
}

pub(super) async fn attach(
    client: &ApiClient,
    db_id: &str,
    project_id: &str,
    body: nrz_api::AttachmentRequestBody,
) -> anyhow::Result<nrz_api::Database200ResponseDatumProjectAttachment> {
    let value = client.attach_database(db_id, project_id, body).await?;
    anyhow::ensure!(
        value.managed_database_id == db_id.parse::<uuid::Uuid>()?
            && value.project_id == project_id.parse::<uuid::Uuid>()?,
        "attachment response belongs to another database or project"
    );
    Ok(value)
}
