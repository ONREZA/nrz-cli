use super::ApiClient;
use anyhow::Context;
use nrz_api::*;

impl ApiClient {
    pub async fn runtime_logs(
        &self,
        project_id: &str,
        query: GetV1projectsByIdRuntimeLogsRequestQuery,
    ) -> anyhow::Result<RuntimeLog200Response> {
        let response = self
            .platform()?
            .get_v1projects_by_id_runtime_logs(GetV1projectsByIdRuntimeLogsRequest {
                path: GetV1projectsByIdRuntimeLogsRequestPath {
                    id: project_id.parse().context("invalid project ID")?,
                },
                query,
            })
            .await?;
        api_success!(
            response,
            GetV1projectsByIdRuntimeLogsRequest,
            GetV1projectsByIdRuntimeLogsResponse::Ok
        )
    }
}
