use super::ApiClient;
use anyhow::Context;
use nrz_api::*;

impl ApiClient {
    pub async fn admit_deployment(
        &self,
        id: &str,
        body: AdmitRequestBody,
    ) -> anyhow::Result<Admit200Response> {
        let response = self
            .platform()?
            .post_v1projects_by_id_deployments_admit(PostV1projectsByIdDeploymentsAdmitRequest {
                path: PostV1projectsByIdDeploymentsAdmitRequestPath {
                    id: id.parse().context("invalid resource ID")?,
                },
                body,
            })
            .await?;
        api_success!(
            response,
            PostV1projectsByIdDeploymentsAdmitRequest,
            PostV1projectsByIdDeploymentsAdmitResponse::Ok
        )
    }
    pub async fn runner_context(&self, id: &str) -> anyhow::Result<RunnerContext200Response> {
        let response = self
            .platform()?
            .get_v1deployments_by_id_runner_context(GetV1deploymentsByIdRunnerContextRequest {
                path: GetV1deploymentsByIdRunnerContextRequestPath {
                    id: id.parse().context("invalid resource ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            GetV1deploymentsByIdRunnerContextRequest,
            GetV1deploymentsByIdRunnerContextResponse::Ok
        )
    }
    pub async fn fail_before_source(
        &self,
        id: &str,
        body: FailBeforeSourceRequestBody,
    ) -> anyhow::Result<FailBeforeSource200Response> {
        let response = self
            .platform()?
            .post_v1deployments_by_id_execution_context_fail_before_source(
                PostV1deploymentsByIdExecutionContextFailBeforeSourceRequest {
                    path: PostV1deploymentsByIdExecutionContextFailBeforeSourceRequestPath {
                        id: id.parse().context("invalid resource ID")?,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PostV1deploymentsByIdExecutionContextFailBeforeSourceRequest,
            PostV1deploymentsByIdExecutionContextFailBeforeSourceResponse::Ok
        )
    }
    pub async fn skip_before_source(
        &self,
        id: &str,
        body: SkipBeforeSourceRequestBody,
    ) -> anyhow::Result<FailBeforeSource200Response> {
        let response = self
            .platform()?
            .post_v1deployments_by_id_execution_context_skip_before_source(
                PostV1deploymentsByIdExecutionContextSkipBeforeSourceRequest {
                    path: PostV1deploymentsByIdExecutionContextSkipBeforeSourceRequestPath {
                        id: id.parse().context("invalid resource ID")?,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PostV1deploymentsByIdExecutionContextSkipBeforeSourceRequest,
            PostV1deploymentsByIdExecutionContextFailBeforeSourceResponse::Ok
        )
    }
    pub async fn register_source(
        &self,
        id: &str,
        body: SourceRequestBody,
    ) -> anyhow::Result<Source200Response> {
        let response = self
            .platform()?
            .post_v1deployments_by_id_source(PostV1deploymentsByIdSourceRequest {
                path: PostV1deploymentsByIdSourceRequestPath {
                    id: id.parse().context("invalid resource ID")?,
                },
                body,
            })
            .await?;
        api_success!(
            response,
            PostV1deploymentsByIdSourceRequest,
            PostV1deploymentsByIdSourceResponse::Ok
        )
    }
    pub async fn update_compute_config(
        &self,
        id: &str,
        body: ComputeConfigRequestBody,
    ) -> anyhow::Result<ComputeConfig200Response> {
        let response = self
            .platform()?
            .put_v1compute_config_by_project_id(PutV1computeConfigByProjectIdRequest {
                path: PutV1computeConfigByProjectIdRequestPath {
                    project_id: id.parse().context("invalid resource ID")?,
                },
                body,
            })
            .await?;
        api_success!(
            response,
            PutV1computeConfigByProjectIdRequest,
            PutV1computeConfigByProjectIdResponse::Ok
        )
    }
}
