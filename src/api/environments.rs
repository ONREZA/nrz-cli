use anyhow::Context;
use nrz_api::*;

use super::ApiClient;

impl ApiClient {
    pub async fn environment_variables(&self, project_id: &str) -> anyhow::Result<Env200Response> {
        let response = self
            .platform()?
            .get_v1projects_by_id_env(GetV1projectsByIdEnvRequest {
                path: GetV1projectsByIdEnvRequestPath {
                    id: project_id.parse().context("invalid project ID")?,
                },
                query: GetV1projectsByIdEnvRequestQuery::default(),
            })
            .await?;
        api_success!(
            response,
            GetV1projectsByIdEnvRequest,
            GetV1projectsByIdEnvResponse::Ok
        )
    }

    pub async fn set_environment_variable(
        &self,
        project_id: &str,
        body: EnvRequestBody,
    ) -> anyhow::Result<Env200Response2> {
        let response = self
            .platform()?
            .post_v1projects_by_id_env(PostV1projectsByIdEnvRequest {
                path: PostV1projectsByIdEnvRequestPath {
                    id: project_id.parse().context("invalid project ID")?,
                },
                body,
            })
            .await?;
        api_success!(
            response,
            PostV1projectsByIdEnvRequest,
            PostV1projectsByIdEnvResponse::Ok
        )
    }

    pub async fn delete_environment_variable(
        &self,
        project_id: &str,
        key: &str,
        confirmed: bool,
    ) -> anyhow::Result<Env200Response3> {
        let response = self
            .platform()?
            .delete_v1projects_by_id_env_by_key(DeleteV1projectsByIdEnvByKeyRequest {
                path: DeleteV1projectsByIdEnvByKeyRequestPath {
                    id: project_id.parse().context("invalid project ID")?,
                    key: key.to_owned(),
                },
                query: DeleteV1projectsByIdEnvByKeyRequestQuery {
                    confirmed: Some(confirmed.to_string()),
                },
            })
            .await?;
        api_success!(
            response,
            DeleteV1projectsByIdEnvByKeyRequest,
            DeleteV1projectsByIdEnvByKeyResponse::Ok
        )
    }
}
