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

fn database_summary(
    id: uuid::Uuid,
    product_id: Option<uuid::Uuid>,
    db_name: String,
    status: nrz_api::ManagedDatabaseStatus,
    cu_size: f64,
    pg_version: i64,
    project_attachments: Vec<ProjectAttachment>,
) -> ManagedDatabase {
    ManagedDatabase {
        id: id.to_string(),
        product_id: product_id.map(|id| id.to_string()),
        db_name: Some(db_name),
        status: Some(status.to_string()),
        cu_size: Some(cu_size),
        pg_version: Some(pg_version),
        metadata: DatabaseMetadata::Legacy {
            auto_inject_db_url: None,
            env_var_name: None,
            auto_create_preview_branch: None,
            kaiki_status: None,
        },
        project_attachments,
    }
}

impl From<nrz_api::ManagedDatabaseLegacyListItem> for ManagedDatabase {
    fn from(value: nrz_api::ManagedDatabaseLegacyListItem) -> Self {
        database_summary(
            value.id,
            value.product_id,
            value.db_name,
            value.status,
            value.cu_size,
            value.pg_version,
            value.project_attachments,
        )
    }
}

impl From<nrz_api::ManagedDatabaseNativeMetadata> for ManagedDatabase {
    fn from(value: nrz_api::ManagedDatabaseNativeMetadata) -> Self {
        let mut db = database_summary(
            value.id,
            value.product_id,
            value.db_name,
            value.status,
            value.cu_size,
            value.pg_version,
            value.project_attachments,
        );
        db.metadata = DatabaseMetadata::Native {
            workspace_id: value.workspace_id,
            role_name: value.role_name,
            created_at: value.created_at,
            updated_at: value.updated_at,
        };
        db
    }
}

impl From<nrz_api::ManagedDatabaseListItem> for ManagedDatabase {
    fn from(value: nrz_api::ManagedDatabaseListItem) -> Self {
        match value {
            nrz_api::ManagedDatabaseListItem::ManagedDatabaseNativeMetadata(value) => value.into(),
            nrz_api::ManagedDatabaseListItem::ManagedDatabaseLegacyListItem(value) => value.into(),
        }
    }
}

impl From<nrz_api::ManagedDatabaseList> for ListResponse {
    fn from(value: nrz_api::ManagedDatabaseList) -> Self {
        Self {
            data: value.data.into_iter().map(Into::into).collect(),
            creation: value.creation,
            plan: Some(value.plan.to_string()),
        }
    }
}

impl From<nrz_api::ManagedDatabaseLegacyDetail> for DbInfoResponse {
    fn from(value: nrz_api::ManagedDatabaseLegacyDetail) -> Self {
        let mut db = database_summary(
            value.id,
            value.product_id,
            value.db_name,
            value.status,
            value.cu_size,
            value.pg_version,
            value.project_attachments,
        );
        db.metadata = DatabaseMetadata::Legacy {
            auto_inject_db_url: None,
            env_var_name: None,
            auto_create_preview_branch: None,
            kaiki_status: value.kaiki_status.map(|status| status.to_string()),
        };
        Self {
            db,
            details: DatabaseInfoDetails::Legacy {
                allowed_cu_sizes: Some(value.allowed_cu_sizes.into_iter().map(cu).collect()),
                autoscale_max_cu: value.autoscale_max_cu.map(cu),
                plan: Some(value.plan.to_string()),
            },
        }
    }
}

impl From<nrz_api::ManagedDatabaseNativeDetail> for DbInfoResponse {
    fn from(value: nrz_api::ManagedDatabaseNativeDetail) -> Self {
        let mut db = database_summary(
            value.id,
            value.product_id,
            value.db_name,
            value.status,
            value.cu_size,
            value.pg_version,
            value.project_attachments,
        );
        db.metadata = DatabaseMetadata::Native {
            workspace_id: value.workspace_id,
            role_name: value.role_name,
            created_at: value.created_at,
            updated_at: value.updated_at,
        };
        Self {
            db,
            details: DatabaseInfoDetails::Native {
                plan: value.plan.to_string(),
                permissions: value.permissions.into_iter().map(|permission| permission.to_string()).collect(),
            },
        }
    }
}

impl From<nrz_api::ManagedDatabaseDetail> for DbInfoResponse {
    fn from(value: nrz_api::ManagedDatabaseDetail) -> Self {
        match value {
            nrz_api::ManagedDatabaseDetail::ManagedDatabaseNativeDetail(value) => value.into(),
            nrz_api::ManagedDatabaseDetail::ManagedDatabaseLegacyDetail(value) => value.into(),
        }
    }
}

pub(super) fn created(
    value: nrz_api::ManagedDatabaseCreateAccepted,
    idempotency_key: String,
) -> CreateResponse {
    CreateResponse {
        id: value.id.to_string(),
        workspace_id: value.workspace_id.to_string(),
        operation_id: value.operation_id.to_string(),
        status: value.status.to_string(),
        idempotency_key,
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
) -> anyhow::Result<DbInfoResponse> {
    let value: DbInfoResponse = client.database(id).await?.into();
    anyhow::ensure!(
        value.db.id == id.parse::<uuid::Uuid>()?.to_string(),
        "database response belongs to another database"
    );
    Ok(value)
}

pub(super) async fn attach(
    client: &ApiClient,
    db_id: &str,
    project_id: &str,
    body: nrz_api::AttachmentRequestBody,
) -> anyhow::Result<ProjectAttachment> {
    let selected = (
        body.service_id,
        body.product_environment_id,
        body.owner_product_environment_id,
        body.provider_branch_id.clone(),
    );
    let value = client.attach_database(db_id, project_id, body).await?;
    anyhow::ensure!(
        value.managed_database_id == db_id.parse::<uuid::Uuid>()?
            && value.project_id == project_id.parse::<uuid::Uuid>()?,
        "attachment response belongs to another database or project"
    );
    anyhow::ensure!(
        (
            value.service_id,
            value.product_environment_id,
            value.owner_product_environment_id,
            value.provider_branch_id.as_str()
        ) == (selected.0, selected.1, selected.2, selected.3.as_str()),
        "attachment response belongs to another Service, ProductEnvironment or provider branch"
    );
    Ok(value)
}
