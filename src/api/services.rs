use anyhow::Context;
use nrz_api::*;

use super::ApiClient;

impl ApiClient {
    pub async fn list_services(
        &self,
        product_id: uuid::Uuid,
    ) -> anyhow::Result<Service200Response> {
        let response = self
            .platform()?
            .get_v1projects_by_id_services(GetV1projectsByIdServicesRequest {
                path: GetV1projectsByIdServicesRequestPath { id: product_id },
            })
            .await?;
        api_success!(
            response,
            GetV1projectsByIdServicesRequest,
            GetV1projectsByIdServicesResponse::Ok
        )
    }
    pub async fn create_cli_source(
        &self,
        product_id: uuid::Uuid,
    ) -> anyhow::Result<Service200ResponseSource> {
        let response = self
            .platform()?
            .post_v1projects_by_id_sources(PostV1projectsByIdSourcesRequest {
                path: PostV1projectsByIdSourcesRequestPath { id: product_id },
                body: SourceTypeRequestBody {
                    source_type: "CLI".into(),
                },
            })
            .await?;
        api_success!(
            response,
            PostV1projectsByIdSourcesRequest,
            PostV1projectsByIdSourcesResponse::Ok
        )
    }
    pub async fn configure_service(
        &self,
        product_id: uuid::Uuid,
        service_id: Option<uuid::Uuid>,
        body: ServiceRequestBody,
    ) -> anyhow::Result<Service200Response2> {
        if let Some(service_id) = service_id {
            let response = self
                .platform()?
                .patch_v1projects_by_id_services_by_service_id(
                    PatchV1projectsByIdServicesByServiceIdRequest {
                        path: PatchV1projectsByIdServicesByServiceIdRequestPath {
                            id: product_id,
                            service_id,
                        },
                        body,
                    },
                )
                .await?;
            api_success!(
                response,
                PatchV1projectsByIdServicesByServiceIdRequest,
                PostV1projectsByIdServicesResponse::Ok
            )
        } else {
            let response = self
                .platform()?
                .post_v1projects_by_id_services(PostV1projectsByIdServicesRequest {
                    path: PostV1projectsByIdServicesRequestPath { id: product_id },
                    body,
                })
                .await?;
            api_success!(
                response,
                PostV1projectsByIdServicesRequest,
                PostV1projectsByIdServicesResponse::Ok
            )
        }
    }

    pub async fn remove_service(
        &self,
        product_id: &str,
        service_id: &str,
    ) -> anyhow::Result<Service200Response3> {
        let response = self
            .platform()?
            .delete_v1projects_by_id_services_by_service_id(
                DeleteV1projectsByIdServicesByServiceIdRequest {
                    path: DeleteV1projectsByIdServicesByServiceIdRequestPath {
                        id: product_id.parse().context("invalid project ID")?,
                        service_id: service_id.parse().context("invalid service ID")?,
                    },
                },
            )
            .await?;
        api_success!(
            response,
            DeleteV1projectsByIdServicesByServiceIdRequest,
            DeleteV1projectsByIdServicesByServiceIdResponse::Ok
        )
    }
}
