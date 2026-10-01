use super::ApiClient;
use anyhow::Context;
use nrz_api::*;

impl ApiClient {
    pub async fn deployment_status(&self, id: &str) -> anyhow::Result<Status200Response> {
        let response = self
            .platform()?
            .get_v1deployments_by_id_status(GetV1deploymentsByIdStatusRequest {
                path: GetV1deploymentsByIdStatusRequestPath {
                    id: id.parse().context("invalid deployment ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            GetV1deploymentsByIdStatusRequest,
            GetV1deploymentsByIdStatusResponse::Ok
        )
    }
    pub async fn deployment(&self, id: &str) -> anyhow::Result<Deployment200Response> {
        let response = self
            .platform()?
            .get_v1deployments_by_id(GetV1deploymentsByIdRequest {
                path: GetV1deploymentsByIdRequestPath {
                    id: id.parse().context("invalid deployment ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            GetV1deploymentsByIdRequest,
            GetV1deploymentsByIdResponse::Ok
        )
    }
    pub async fn project_deployments(
        &self,
        project_id: &str,
        limit: u32,
        offset: u32,
    ) -> anyhow::Result<Project200Response6> {
        let response = self
            .platform()?
            .get_v1deployments_project_by_project_id(GetV1deploymentsProjectByProjectIdRequest {
                path: GetV1deploymentsProjectByProjectIdRequestPath {
                    project_id: project_id.parse().context("invalid project ID")?,
                },
                query: GetV1deploymentsProjectByProjectIdRequestQuery {
                    limit: Some(serde_json::json!(limit)),
                    offset: Some(serde_json::json!(offset)),
                    is_preview: None,
                },
            })
            .await?;
        api_success!(
            response,
            GetV1deploymentsProjectByProjectIdRequest,
            GetV1deploymentsProjectByProjectIdResponse::Ok
        )
    }
}
