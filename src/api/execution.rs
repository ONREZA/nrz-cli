use anyhow::Context;
use nrz_api::*;

use super::ApiClient;

impl ApiClient {
    pub async fn resolve_execution_context(
        &self,
        id: &str,
        body: ResolveRequestBody,
    ) -> anyhow::Result<Resolve200Response> {
        let response = self
            .platform()?
            .post_v1projects_by_id_execution_context_resolve(
                PostV1projectsByIdExecutionContextResolveRequest {
                    path: PostV1projectsByIdExecutionContextResolveRequestPath {
                        id: id
                            .parse()
                            .context("invalid execution context resource ID")?,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PostV1projectsByIdExecutionContextResolveRequest,
            PostV1projectsByIdExecutionContextResolveResponse::Ok
        )
    }
    pub async fn materialize_execution_context(
        &self,
        id: &str,
        body: MaterializeRequestBody,
    ) -> anyhow::Result<Materialize200Response> {
        let response = self
            .platform()?
            .post_v1projects_by_id_execution_context_materialize(
                PostV1projectsByIdExecutionContextMaterializeRequest {
                    path: PostV1projectsByIdExecutionContextMaterializeRequestPath {
                        id: id
                            .parse()
                            .context("invalid execution context resource ID")?,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PostV1projectsByIdExecutionContextMaterializeRequest,
            PostV1projectsByIdExecutionContextMaterializeResponse::Ok
        )
    }
    pub async fn materialize_deployment_context(
        &self,
        id: &str,
        body: PurposeRequestBody,
    ) -> anyhow::Result<Materialize200Response> {
        let response = self
            .platform()?
            .post_v1deployments_by_id_execution_context_materialize(
                PostV1deploymentsByIdExecutionContextMaterializeRequest {
                    path: PostV1deploymentsByIdExecutionContextMaterializeRequestPath {
                        id: id
                            .parse()
                            .context("invalid execution context resource ID")?,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PostV1deploymentsByIdExecutionContextMaterializeRequest,
            PostV1projectsByIdExecutionContextMaterializeResponse::Ok
        )
    }
}
