use anyhow::Context;
use nrz_api::*;

use super::ApiClient;

impl ApiClient {
    pub async fn project_environments(
        &self,
        project_id: &str,
    ) -> anyhow::Result<Environment200Response> {
        let response = self
            .platform()?
            .get_v1environments_by_id(GetV1environmentsByIdRequest {
                path: GetV1environmentsByIdRequestPath {
                    id: project_id.parse().context("invalid project ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            GetV1environmentsByIdRequest,
            GetV1environmentsByIdResponse::Ok
        )
    }
}
