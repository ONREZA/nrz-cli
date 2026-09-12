use serde::Serialize;

// --- List ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectsResponse {
    pub(super) projects: Vec<ProjectSummary>,
    pub(super) total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectSummary {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) display_name: Option<String>,
    pub(super) framework_preset: Option<String>,
    pub(super) updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateProjectResponse {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) source_type: Option<String>,
    pub(super) git_url: Option<String>,
    pub(super) branch: Option<String>,
    pub(super) created_at: Option<String>,
    pub(super) message: Option<String>,
}

// --- Info ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectDetailResponse {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) display_name: Option<String>,
    pub(super) source_type: Option<String>,
    pub(super) git_url: Option<String>,
    pub(super) branch: Option<String>,
    pub(super) framework_preset: Option<String>,
    pub(super) install_command: Option<String>,
    pub(super) build_command: Option<String>,
    pub(super) output_directory: Option<String>,
    pub(super) root_directory: Option<String>,
    pub(super) node_version: Option<String>,
    pub(super) package_manager: Option<String>,
    pub(super) auto_deploy_enabled: Option<bool>,
    pub(super) created_at: Option<String>,
    pub(super) updated_at: Option<String>,
    pub(super) deployments: Vec<nrz_api::Project200Response3Deployment>,
    #[serde(rename = "_count")]
    pub(super) count: Option<nrz_api::Project200Response3Count>,
}

// --- Update ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UpdateProjectResponse {
    pub(super) id: String,
    pub(super) message: Option<String>,
}

// --- Delete ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeleteProjectResponse {
    pub(super) id: String,
    pub(super) message: Option<String>,
}

impl From<nrz_api::Project200Response> for ProjectsResponse {
    fn from(value: nrz_api::Project200Response) -> Self {
        Self {
            projects: value
                .projects
                .into_iter()
                .map(|project| ProjectSummary {
                    id: project.id.to_string(),
                    name: project.name,
                    display_name: project.display_name,
                    framework_preset: project.framework_preset,
                    updated_at: Some(
                        project
                            .updated_at
                            .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
                    ),
                })
                .collect(),
            total: value.total,
        }
    }
}

impl From<nrz_api::Project200Response2> for CreateProjectResponse {
    fn from(value: nrz_api::Project200Response2) -> Self {
        Self {
            id: value.id.to_string(),
            name: value.name,
            source_type: Some(value.source_type.to_string()),
            git_url: value.git_url,
            branch: Some(value.branch),
            created_at: Some(
                value
                    .created_at
                    .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
            ),
            message: Some(value.message),
        }
    }
}

impl From<nrz_api::Project200Response3> for ProjectDetailResponse {
    fn from(value: nrz_api::Project200Response3) -> Self {
        Self {
            id: value.id.to_string(),
            name: value.name,
            display_name: value.display_name,
            source_type: Some(value.source_type.to_string()),
            git_url: value.git_url,
            branch: Some(value.branch),
            framework_preset: value.framework_preset,
            install_command: value.install_command,
            build_command: value.build_command,
            output_directory: value.output_directory,
            root_directory: Some(value.root_directory),
            node_version: Some(value.node_version.to_string()),
            package_manager: Some(value.package_manager.to_string()),
            auto_deploy_enabled: Some(value.auto_deploy_enabled),
            created_at: Some(
                value
                    .created_at
                    .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
            ),
            updated_at: Some(
                value
                    .updated_at
                    .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
            ),
            deployments: value.deployments,
            count: Some(value.count),
        }
    }
}
