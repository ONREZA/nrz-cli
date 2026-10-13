use super::ApiClient;
use crate::cli::endpoints::EndpointScope;
use nrz_api::*;

impl ApiClient {
    pub async fn endpoint_list(
        &self,
        scope: &EndpointScope,
    ) -> anyhow::Result<Endpoint200Response> {
        let response=self.platform()?.get_v1projects_by_id_services_by_service_id_environments_by_environment_id_endpoints(
            GetV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsRequest {path:GetV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsRequestPath {id:scope.project_id,service_id:scope.service_id,environment_id:scope.environment_id}}).await?;
        api_success!(
            response,
            GetV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsRequest,
            GetV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsResponse::Ok
        )
    }
    pub async fn endpoint_export(
        &self,
        scope: &EndpointScope,
        layer: String,
        body: ExportRequestBody,
    ) -> anyhow::Result<IdResponse> {
        let response=self.platform()?.put_v1projects_by_id_services_by_service_id_environments_by_environment_id_endpoints_by_layer_name_export(
            PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameExportRequest {path:PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameExportRequestPath {id:scope.project_id,service_id:scope.service_id,environment_id:scope.environment_id,layer_name:layer},body}).await?;
        api_success!(response,PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameExportRequest,PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameExportResponse::Ok)
    }
    pub async fn endpoint_import(
        &self,
        scope: &EndpointScope,
        layer: String,
        body: ImportRequestBody,
    ) -> anyhow::Result<IdResponse> {
        let response=self.platform()?.put_v1projects_by_id_services_by_service_id_environments_by_environment_id_endpoints_by_layer_name_import(
            PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameImportRequest {path:PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameImportRequestPath {id:scope.project_id,service_id:scope.service_id,environment_id:scope.environment_id,layer_name:layer},body}).await?;
        api_success!(response,PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameImportRequest,PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameExportResponse::Ok)
    }
    pub async fn endpoint_network(
        &self,
        scope: &EndpointScope,
        layer: String,
        body: NetworkRequestBody,
    ) -> anyhow::Result<GenerationResponse> {
        let response=self.platform()?.put_v1projects_by_id_services_by_service_id_environments_by_environment_id_endpoints_by_layer_name_network(
            PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameNetworkRequest {path:PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameNetworkRequestPath {id:scope.project_id,service_id:scope.service_id,environment_id:scope.environment_id,layer_name:layer},body}).await?;
        api_success!(response,PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameNetworkRequest,PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameNetworkResponse::Ok)
    }
    pub async fn endpoint_exposure(
        &self,
        scope: &EndpointScope,
        layer: String,
        body: ExposureRequestBody,
    ) -> anyhow::Result<GenerationResponse> {
        let response=self.platform()?.put_v1projects_by_id_services_by_service_id_environments_by_environment_id_endpoints_by_layer_name_exposure(
            PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameExposureRequest {path:PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameExposureRequestPath {id:scope.project_id,service_id:scope.service_id,environment_id:scope.environment_id,layer_name:layer},body}).await?;
        api_success!(response,PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameExposureRequest,PutV1projectsByIdServicesByServiceIdEnvironmentsByEnvironmentIdEndpointsByLayerNameNetworkResponse::Ok)
    }
}
