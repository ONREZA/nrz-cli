use serde::Serialize;

// ── CLI output types ──────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ManagedDatabase {
    pub(super) id: String,
    pub(super) db_name: Option<String>,
    pub(super) status: Option<String>,
    pub(super) cu_size: Option<f64>,
    pub(super) pg_version: Option<i64>,
    pub(super) auto_inject_db_url: Option<bool>,
    pub(super) env_var_name: Option<String>,
    pub(super) auto_create_preview_branch: Option<bool>,
    pub(super) kaiki_status: Option<String>,
    pub(super) project_attachments: Vec<ProjectAttachment>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectAttachment {
    pub(super) project_id: String,
    pub(super) auto_inject_db_url: Option<bool>,
    pub(super) env_var_name: Option<String>,
    pub(super) auto_create_preview_branch: Option<bool>,
}

impl ManagedDatabase {
    pub(super) fn attachment_for_project(&self, project_id: &str) -> Option<&ProjectAttachment> {
        self.project_attachments
            .iter()
            .find(|attachment| attachment.project_id == project_id)
    }

    pub(super) fn is_attached_to_project(&self, project_id: &str) -> bool {
        self.attachment_for_project(project_id).is_some()
    }

    pub(super) fn auto_inject_db_url_for_project(&self, project_id: &str) -> Option<bool> {
        self.attachment_for_project(project_id)
            .and_then(|attachment| attachment.auto_inject_db_url)
            .or(self.auto_inject_db_url)
    }

    pub(super) fn env_var_name_for_project(&self, project_id: &str) -> Option<&str> {
        self.attachment_for_project(project_id)
            .and_then(|attachment| attachment.env_var_name.as_deref())
            .or(self.env_var_name.as_deref())
    }

    pub(super) fn auto_create_preview_branch_for_project(&self, project_id: &str) -> Option<bool> {
        self.attachment_for_project(project_id)
            .and_then(|attachment| attachment.auto_create_preview_branch)
            .or(self.auto_create_preview_branch)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListResponse {
    pub(super) data: Vec<ManagedDatabase>,
    pub(super) allowed_cu_sizes: Option<Vec<f64>>,
    pub(super) autoscale_max_cu: Option<f64>,
    pub(super) plan: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DbInfoResponse {
    #[serde(flatten)]
    pub(super) db: ManagedDatabase,
    pub(super) allowed_cu_sizes: Option<Vec<f64>>,
    pub(super) autoscale_max_cu: Option<f64>,
    pub(super) plan: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateResponse {
    pub(super) id: String,
    pub(super) db_name: Option<String>,
    pub(super) status: Option<String>,
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
