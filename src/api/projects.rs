use anyhow::Context;
use nrz_api::*;

use super::ApiClient;

impl ApiClient {
    pub async fn projects(&self, limit: u32, offset: u32) -> anyhow::Result<Project200Response> {
        let response = self
            .platform()?
            .get_v1projects(GetV1projectsRequest {
                query: GetV1projectsRequestQuery {
                    limit: Some(f64::from(limit)),
                    offset: Some(f64::from(offset)),
                    name: None,
                },
            })
            .await?;
        api_success!(response, GetV1projectsRequest, GetV1projectsResponse::Ok)
    }

    pub async fn project(&self, id: &str) -> anyhow::Result<Project200Response3> {
        let response = self
            .platform()?
            .get_v1projects_by_id(GetV1projectsByIdRequest {
                path: GetV1projectsByIdRequestPath {
                    id: id.parse().context("invalid project ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            GetV1projectsByIdRequest,
            GetV1projectsByIdResponse::Ok
        )
    }

    pub async fn create_project(
        &self,
        body: ProjectRequestBody,
    ) -> anyhow::Result<Project200Response2> {
        let response = self
            .platform()?
            .post_v1projects(PostV1projectsRequest { body })
            .await?;
        api_success!(response, PostV1projectsRequest, PostV1projectsResponse::Ok)
    }

    pub async fn update_project(
        &self,
        id: &str,
        body: ProjectRequestBody2,
    ) -> anyhow::Result<Project200Response5> {
        let response = self
            .platform()?
            .patch_v1projects_by_id(PatchV1projectsByIdRequest {
                path: PatchV1projectsByIdRequestPath {
                    id: id.parse().context("invalid project ID")?,
                },
                body,
            })
            .await?;
        api_success!(
            response,
            PatchV1projectsByIdRequest,
            PatchV1projectsByIdResponse::Ok
        )
    }

    pub async fn delete_project(&self, id: &str) -> anyhow::Result<Project200Response4> {
        let response = self
            .platform()?
            .delete_v1projects_by_id(DeleteV1projectsByIdRequest {
                path: DeleteV1projectsByIdRequestPath {
                    id: id.parse().context("invalid project ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            DeleteV1projectsByIdRequest,
            DeleteV1projectsByIdResponse::Ok
        )
    }
    pub async fn sync_detection(
        &self,
        id: &str,
        body: DetectionRequestBody,
    ) -> anyhow::Result<Project200Response4> {
        let response = self
            .platform()?
            .post_v1projects_by_id_detection(PostV1projectsByIdDetectionRequest {
                path: PostV1projectsByIdDetectionRequestPath {
                    id: id.parse().context("invalid project ID")?,
                },
                body,
            })
            .await?;
        api_success!(
            response,
            PostV1projectsByIdDetectionRequest,
            DeleteV1projectsByIdResponse::Ok
        )
    }
}
