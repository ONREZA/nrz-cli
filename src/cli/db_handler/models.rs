use serde::Serialize;

// ── CLI output types ──────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ManagedDatabase {
    pub(super) id: String,
    pub(super) product_id: Option<String>,
    pub(super) db_name: Option<String>,
    pub(super) status: Option<String>,
    pub(super) cu_size: Option<f64>,
    pub(super) pg_version: Option<i64>,
    #[serde(flatten)]
    pub(super) metadata: DatabaseMetadata,
    pub(super) project_attachments: Vec<ProjectAttachment>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "providerContour", rename_all_fields = "camelCase")]
pub(super) enum DatabaseMetadata {
    #[serde(rename = "PLATFORM_NATIVE")]
    Native {
        workspace_id: uuid::Uuid,
        role_name: String,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
    },
    #[serde(rename = "LEGACY")]
    Legacy {
        auto_inject_db_url: Option<bool>,
        env_var_name: Option<String>,
        auto_create_preview_branch: Option<bool>,
        kaiki_status: Option<String>,
    },
}

pub(super) type ProjectAttachment = nrz_api::ManagedDatabaseProjectAttachment;

impl ManagedDatabase {
    pub(super) fn belongs_to_project(&self, project_id: &str) -> bool {
        self.product_id.as_deref() == Some(project_id) || self.is_attached_to_project(project_id)
    }

    pub(super) fn attachment_for_project(&self, project_id: &str) -> Option<&ProjectAttachment> {
        let mut matching = self
            .project_attachments
            .iter()
            .filter(|attachment| attachment.project_id.to_string() == project_id);
        let first = matching.next()?;
        matching.next().is_none().then_some(first)
    }

    pub(super) fn is_attached_to_project(&self, project_id: &str) -> bool {
        self.project_attachments
            .iter()
            .any(|binding| binding.project_id.to_string() == project_id)
    }

    pub(super) fn auto_inject_db_url_for_project(&self, project_id: &str) -> Option<bool> {
        self.attachment_for_project(project_id)
            .map(|attachment| attachment.auto_inject_db_url)
            .or(match &self.metadata {
                DatabaseMetadata::Legacy { auto_inject_db_url, .. } => *auto_inject_db_url,
                DatabaseMetadata::Native { .. } => None,
            })
    }

    pub(super) fn env_var_name_for_project(&self, project_id: &str) -> Option<&str> {
        self.attachment_for_project(project_id)
            .map(|attachment| attachment.env_var_name.as_str())
            .or(match &self.metadata {
                DatabaseMetadata::Legacy { env_var_name, .. } => env_var_name.as_deref(),
                DatabaseMetadata::Native { .. } => None,
            })
    }

    pub(super) fn auto_create_preview_branch_for_project(&self, project_id: &str) -> Option<bool> {
        self.attachment_for_project(project_id)
            .map(|attachment| attachment.auto_create_preview_branch)
            .or(match &self.metadata {
                DatabaseMetadata::Legacy { auto_create_preview_branch, .. } => *auto_create_preview_branch,
                DatabaseMetadata::Native { .. } => None,
            })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListResponse {
    pub(super) data: Vec<ManagedDatabase>,
    pub(super) creation: nrz_api::ManagedDatabaseCreationChoices,
    pub(super) plan: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DbInfoResponse {
    #[serde(flatten)]
    pub(super) db: ManagedDatabase,
    #[serde(flatten)]
    pub(super) details: DatabaseInfoDetails,
}

#[derive(Debug, Serialize)]
#[serde(untagged, rename_all_fields = "camelCase")]
pub(super) enum DatabaseInfoDetails {
    Native {
        plan: String,
        permissions: Vec<String>,
    },
    Legacy {
        allowed_cu_sizes: Option<Vec<f64>>,
        autoscale_max_cu: Option<f64>,
        plan: Option<String>,
    },
}

impl DbInfoResponse {
    pub(super) fn plan(&self) -> Option<&str> {
        match &self.details {
            DatabaseInfoDetails::Native { plan, .. } => Some(plan),
            DatabaseInfoDetails::Legacy { plan, .. } => plan.as_deref(),
        }
    }

    pub(super) fn allowed_cu_sizes(&self) -> Option<&[f64]> {
        match &self.details {
            DatabaseInfoDetails::Native { .. } => None,
            DatabaseInfoDetails::Legacy { allowed_cu_sizes, .. } => allowed_cu_sizes.as_deref(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateResponse {
    pub(super) id: String,
    pub(super) workspace_id: String,
    pub(super) status: String,
    pub(super) operation_id: String,
    pub(super) idempotency_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Branch {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) status: Option<String>,
    pub(super) is_preview_branch: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(super) struct BranchListResponse {
    pub(super) data: Vec<Branch>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct QueryResult {
    pub(super) columns: Vec<String>,
    pub(super) rows: Vec<Vec<serde_json::Value>>,
    pub(super) row_count: i64,
    pub(super) duration_ms: f64,
}

// ── Schema introspection types ──────────────────────────────

#[derive(Debug, Serialize)]
pub(super) struct SchemaOutput {
    pub(super) tables: Vec<SchemaTable>,
}

#[derive(Debug, Serialize)]
pub(super) struct SchemaTable {
    pub(super) name: String,
    pub(super) columns: Vec<SchemaColumn>,
}

#[derive(Debug, Serialize)]
pub(super) struct SchemaColumn {
    pub(super) name: String,
    #[serde(rename = "type")]
    pub(super) col_type: String,
    pub(super) nullable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) default: Option<String>,
}
