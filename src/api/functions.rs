use super::ApiClient;
use anyhow::Context;
use nrz_api::*;

impl ApiClient {
    pub async fn functions(
        &self,
        project_id: &str,
        environment_id: &str,
    ) -> anyhow::Result<FunctionListResponse> {
        let response = self.platform()?.get_v1projects_by_id_function_activations_environments_by_environment_id_functions(GetV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdFunctionsRequest { path: GetV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdFunctionsRequestPath { id: project_id.parse().context("invalid project ID")?, environment_id: environment_id.parse().context("invalid environment ID")? },}).await?;
        api_success!(
            response,
            GetV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdFunctionsRequest,
            GetV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdFunctionsResponse::Ok
        )
    }
    pub async fn test_invoke_function(
        &self,
        project_id: &str,
        environment_id: &str,
        function_id: &str,
        revision_id: &str,
        body: TestInvokeRequestBody,
    ) -> anyhow::Result<FunctionTestInvokeResponse> {
        let response = self.platform()?.post_v1projects_by_id_function_activations_environments_by_environment_id_functions_by_function_id_revisions_by_revision_id_test_invoke(PostV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdFunctionsByFunctionIdRevisionsByRevisionIdTestInvokeRequest { path: PostV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdFunctionsByFunctionIdRevisionsByRevisionIdTestInvokeRequestPath { id: project_id.parse().context("invalid project ID")?, environment_id: environment_id.parse().context("invalid environment ID")?, function_id: function_id.parse().context("invalid function id")?, revision_id: revision_id.parse().context("invalid revision id")? }, body,}).await?;
        api_success!(response, PostV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdFunctionsByFunctionIdRevisionsByRevisionIdTestInvokeRequest, PostV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdFunctionsByFunctionIdRevisionsByRevisionIdTestInvokeResponse::Ok)
    }
    pub async fn publish_edge_rules(
        &self,
        project_id: &str,
        environment_id: &str,
        body: PublishRequestBody,
    ) -> anyhow::Result<FunctionsPublishResponse> {
        let response = self.platform()?.post_v1projects_by_id_function_activations_environments_by_environment_id_functions_publish(PostV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdFunctionsPublishRequest { path: PostV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdFunctionsPublishRequestPath { id: project_id.parse().context("invalid project ID")?, environment_id: environment_id.parse().context("invalid environment ID")? }, body,}).await?;
        api_success!(response, PostV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdFunctionsPublishRequest, PostV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdFunctionsPublishResponse::Ok)
    }
    pub async fn active_edge_rules(
        &self,
        project_id: &str,
        environment_id: &str,
    ) -> anyhow::Result<ActiveEdgeRulesResponse> {
        let response = self.platform()?.get_v1projects_by_id_function_activations_environments_by_environment_id_edge_rules(GetV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdEdgeRulesRequest { path: GetV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdEdgeRulesRequestPath { id: project_id.parse().context("invalid project ID")?, environment_id: environment_id.parse().context("invalid environment ID")? },}).await?;
        api_success!(
            response,
            GetV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdEdgeRulesRequest,
            GetV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdEdgeRulesResponse::Ok
        )
    }
    pub async fn edge_rules_status(
        &self,
        project_id: &str,
        environment_id: &str,
        body: StatusRequestBody,
    ) -> anyhow::Result<EdgeRulesStatusResponse> {
        let response = self.platform()?.post_v1projects_by_id_function_activations_environments_by_environment_id_edge_rules_status(PostV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdEdgeRulesStatusRequest { path: PostV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdEdgeRulesStatusRequestPath { id: project_id.parse().context("invalid project ID")?, environment_id: environment_id.parse().context("invalid environment ID")? }, body,}).await?;
        api_success!(response, PostV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdEdgeRulesStatusRequest, PostV1projectsByIdFunctionActivationsEnvironmentsByEnvironmentIdEdgeRulesStatusResponse::Ok)
    }
}
