use super::ApiClient;
use anyhow::Context;
use nrz_api::*;

impl ApiClient {
    pub async fn build_log_policy(&self) -> anyhow::Result<EnabledResponse> {
        let response = self
            .platform()?
            .get_v1build_log_sessions_workspace_policy(
                GetV1buildLogSessionsWorkspacePolicyRequest {},
            )
            .await?;
        api_success!(
            response,
            GetV1buildLogSessionsWorkspacePolicyRequest,
            GetV1buildLogSessionsWorkspacePolicyResponse::Ok
        )
    }
    pub async fn create_build_log_session(
        &self,
        body: BuildLogSessionRequestBody,
    ) -> anyhow::Result<BuildLogSession200Response> {
        let response = self
            .platform()?
            .post_v1build_log_sessions(PostV1buildLogSessionsRequest { body })
            .await?;
        api_success!(
            response,
            PostV1buildLogSessionsRequest,
            PostV1buildLogSessionsResponse::Ok
        )
    }
    pub async fn append_build_log_events(
        &self,
        id: &str,
        body: EventRequestBody,
    ) -> anyhow::Result<Event200Response> {
        let response = self
            .platform()?
            .post_v1build_log_sessions_by_id_events(PostV1buildLogSessionsByIdEventsRequest {
                path: PostV1buildLogSessionsByIdEventsRequestPath {
                    id: id.parse().context("invalid build log session ID")?,
                },
                body,
            })
            .await?;
        api_success!(
            response,
            PostV1buildLogSessionsByIdEventsRequest,
            PostV1buildLogSessionsByIdEventsResponse::Ok
        )
    }
    pub async fn finish_build_log_session(
        &self,
        id: &str,
        body: FinishRequestBody,
    ) -> anyhow::Result<Finish200Response> {
        let response = self
            .platform()?
            .post_v1build_log_sessions_by_id_finish(PostV1buildLogSessionsByIdFinishRequest {
                path: PostV1buildLogSessionsByIdFinishRequestPath {
                    id: id.parse().context("invalid build log session ID")?,
                },
                body,
            })
            .await?;
        api_success!(
            response,
            PostV1buildLogSessionsByIdFinishRequest,
            PostV1buildLogSessionsByIdFinishResponse::Ok
        )
    }
}
