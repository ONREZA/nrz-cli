use anyhow::Context;
use nrz_api::*;

use super::ApiClient;

impl ApiClient {
    pub async fn remove_service(
        &self,
        product_id: &str,
        service_id: &str,
    ) -> anyhow::Result<Service200Response> {
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
