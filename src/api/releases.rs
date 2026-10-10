use super::ApiClient;
use anyhow::Context;
use nrz_api::*;

impl ApiClient {
    pub async fn environment_serving(
        &self,
        environment_id: &str,
    ) -> anyhow::Result<Serving200Response> {
        let response = self
            .platform()?
            .get_v1environments_by_id_serving(GetV1environmentsByIdServingRequest {
                path: GetV1environmentsByIdServingRequestPath {
                    id: environment_id.parse().context("invalid environment ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            GetV1environmentsByIdServingRequest,
            GetV1environmentsByIdServingResponse::Ok
        )
    }

    pub async fn environment_releases(
        &self,
        environment_id: &str,
    ) -> anyhow::Result<Release200Response> {
        let response = self
            .platform()?
            .get_v1environments_by_id_releases(GetV1environmentsByIdReleasesRequest {
                path: GetV1environmentsByIdReleasesRequestPath {
                    id: environment_id.parse().context("invalid environment ID")?,
                },
                query: GetV1environmentsByIdReleasesRequestQuery {
                    before_created_at: None,
                    before_release_id: None,
                    limit: Some("100".to_owned()),
                },
            })
            .await?;
        api_success!(
            response,
            GetV1environmentsByIdReleasesRequest,
            GetV1environmentsByIdReleasesResponse::Ok
        )
    }

    pub async fn activate_environment_release(
        &self,
        environment_id: &str,
        release_id: &str,
        expected_generation: String,
        idempotency_key: String,
        finish_observation_early: bool,
    ) -> anyhow::Result<ActivateRelease202Response> {
        let response = self
            .platform()?
            .post_v1environments_by_id_actions_activate_release(
                PostV1environmentsByIdActionsActivateReleaseRequest {
                    path: PostV1environmentsByIdActionsActivateReleaseRequestPath {
                        id: environment_id.parse().context("invalid environment ID")?,
                    },
                    header: PostV1environmentsByIdActionsActivateReleaseRequestHeader {
                        idempotency_key,
                    },
                    body: ActivateReleaseRequestBody {
                        release_id: release_id.parse().context("invalid release ID")?,
                        expected_generation,
                        finish_observation_early: Some(finish_observation_early),
                        additional_properties: Default::default(),
                    },
                },
            )
            .await?;
        api_success!(
            response,
            PostV1environmentsByIdActionsActivateReleaseRequest,
            PostV1environmentsByIdActionsActivateReleaseResponse::Accepted
        )
    }
}
