use super::ApiClient;
use anyhow::Context;
use nrz_api::*;

impl ApiClient {
    pub async fn admit_deployment(
        &self,
        id: &str,
        body: AdmitRequestBody2,
    ) -> anyhow::Result<Admit200Response2> {
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
    pub async fn create_deployment_set(
        &self,
        body: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let response = self
            .platform()?
            .post_v1deployments_set(PostV1deploymentsSetRequest {
                body: serde_json::from_value(body)?,
            })
            .await?;
        let value: anyhow::Result<_> = api_success!(
            response,
            PostV1deploymentsSetRequest,
            PostV1deploymentsSetResponse::Ok
        );
        Ok(serde_json::to_value(value?)?)
    }
    pub async fn admit_deployment_set(
        &self,
        project_id: uuid::Uuid,
        body: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let response = self
            .platform()?
            .post_v1projects_by_id_deployments_admit_set(
                PostV1projectsByIdDeploymentsAdmitSetRequest {
                    path: PostV1projectsByIdDeploymentsAdmitSetRequestPath { id: project_id },
                    body: serde_json::from_value(body)?,
                },
            )
            .await?;
        let value: anyhow::Result<_> = api_success!(
            response,
            PostV1projectsByIdDeploymentsAdmitSetRequest,
            PostV1deploymentsSetResponse::Ok
        );
        Ok(serde_json::to_value(value?)?)
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
        body: SourceRequestBody2,
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
    pub async fn register_source_v2_metadata(
        &self,
        id: &str,
        body: SourceRequestBody2,
    ) -> anyhow::Result<Metadatum200Response> {
        // Both routes share the server schema; generator names are operation-local.
        let body: MetadatumRequestBody = serde_json::from_value(serde_json::to_value(body)?)?;
        let response = self
            .platform()?
            .post_v1deployments_by_id_source_v2metadata(
                PostV1deploymentsByIdSourceV2metadataRequest {
                    path: PostV1deploymentsByIdSourceV2metadataRequestPath {
                        id: id.parse().context("invalid resource ID")?,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PostV1deploymentsByIdSourceV2metadataRequest,
            PostV1deploymentsByIdSourceV2metadataResponse::Ok
        )
    }
    pub async fn admit_static_source_v2(
        &self,
        id: &str,
        body: AdmitRequestBody,
    ) -> anyhow::Result<Admit200Response> {
        let response = self
            .platform()?
            .post_v1deployments_by_id_static_source_v2admit(
                PostV1deploymentsByIdStaticSourceV2admitRequest {
                    path: PostV1deploymentsByIdStaticSourceV2admitRequestPath {
                        id: id.parse().context("invalid resource ID")?,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PostV1deploymentsByIdStaticSourceV2admitRequest,
            PostV1deploymentsByIdStaticSourceV2admitResponse::Ok
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
