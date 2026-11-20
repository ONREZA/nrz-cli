use nrz_api::*;

use super::ApiClient;

pub use nrz_api::{
    Config200Response as ServiceConfigResponse,
    Config200Response2 as ServiceConfigMutationResponse,
    ConfigRequestBody as ServiceConfigSaveBody, ConfigRequestBody2 as ServiceConfigDeleteBody,
    ConfigRequestBodyVariable as ServiceConfigVariable,
};

impl ApiClient {
    pub async fn service_config(
        &self,
        product_id: uuid::Uuid,
        service_id: uuid::Uuid,
        environment_id: Option<uuid::Uuid>,
        branch: Option<&str>,
    ) -> anyhow::Result<ServiceConfigResponse> {
        let response = self
            .platform()?
            .get_v1projects_by_id_services_by_service_id_config(
                GetV1projectsByIdServicesByServiceIdConfigRequest {
                    path: GetV1projectsByIdServicesByServiceIdConfigRequestPath {
                        id: product_id,
                        service_id,
                    },
                    query: GetV1projectsByIdServicesByServiceIdConfigRequestQuery {
                        environment_id,
                        branch: branch.map(str::to_owned),
                    },
                },
            )
            .await?;
        api_success!(
            response,
            GetV1projectsByIdServicesByServiceIdConfigRequest,
            GetV1projectsByIdServicesByServiceIdConfigResponse::Ok
        )
    }

    pub async fn save_service_config(
        &self,
        product_id: uuid::Uuid,
        service_id: uuid::Uuid,
        body: ServiceConfigSaveBody,
    ) -> anyhow::Result<ServiceConfigMutationResponse> {
        let response = self
            .platform()?
            .put_v1projects_by_id_services_by_service_id_config(
                PutV1projectsByIdServicesByServiceIdConfigRequest {
                    path: PutV1projectsByIdServicesByServiceIdConfigRequestPath {
                        id: product_id,
                        service_id,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PutV1projectsByIdServicesByServiceIdConfigRequest,
            PutV1projectsByIdServicesByServiceIdConfigResponse::Ok
        )
    }

    pub async fn delete_service_config(
        &self,
        product_id: uuid::Uuid,
        service_id: uuid::Uuid,
        key: &str,
        body: ServiceConfigDeleteBody,
    ) -> anyhow::Result<ServiceConfigMutationResponse> {
        let response = self
            .platform()?
            .delete_v1projects_by_id_services_by_service_id_config_by_key(
                DeleteV1projectsByIdServicesByServiceIdConfigByKeyRequest {
                    path: DeleteV1projectsByIdServicesByServiceIdConfigByKeyRequestPath {
                        id: product_id,
                        service_id,
                        key: key.to_owned(),
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            DeleteV1projectsByIdServicesByServiceIdConfigByKeyRequest,
            PutV1projectsByIdServicesByServiceIdConfigResponse::Ok
        )
    }
}
