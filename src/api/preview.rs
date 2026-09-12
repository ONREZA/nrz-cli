use super::ApiClient;
use anyhow::Context;
use nrz_api::*;

impl ApiClient {
    pub async fn create_preview_access(
        &self,
        project_id: &str,
        body: PreviewAccessRequestBody,
    ) -> anyhow::Result<AccessResponse> {
        let response = self
            .platform()?
            .post_v1preview_access_by_id(PostV1previewAccessByIdRequest {
                path: PostV1previewAccessByIdRequestPath {
                    id: project_id.parse().context("invalid project ID")?,
                },
                body,
            })
            .await?;
        api_success!(
            response,
            PostV1previewAccessByIdRequest,
            PostV1previewAccessByIdResponse::Ok
        )
    }
    pub async fn revoke_preview_access(
        &self,
        project_id: &str,
        secret_id: &str,
    ) -> anyhow::Result<SuccessResponse> {
        let response = self
            .platform()?
            .delete_v1preview_access_by_id_by_secret_id(
                DeleteV1previewAccessByIdBySecretIdRequest {
                    path: DeleteV1previewAccessByIdBySecretIdRequestPath {
                        id: project_id.parse().context("invalid project ID")?,
                        secret_id: secret_id.parse().context("invalid preview secret ID")?,
                    },
                },
            )
            .await?;
        api_success!(
            response,
            DeleteV1previewAccessByIdBySecretIdRequest,
            DeleteV1previewAccessByIdBySecretIdResponse::Ok
        )
    }
}
