use super::ApiClient;
use nrz_api::*;
use uuid::Uuid;

impl ApiClient {
    pub async fn plan_product_fork(
        &self,
        product_id: Uuid,
        body: PlanRequestBody,
    ) -> anyhow::Result<Plan200Response> {
        let response = self
            .platform()?
            .post_v1product_forks_by_product_id_plan(PostV1productForksByProductIdPlanRequest {
                path: PostV1productForksByProductIdPlanRequestPath { product_id },
                body,
            })
            .await?;
        api_success!(
            response,
            PostV1productForksByProductIdPlanRequest,
            PostV1productForksByProductIdPlanResponse::Ok
        )
    }

    pub async fn product_fork_usage(
        &self,
        product_id: Uuid,
        fork_id: Uuid,
    ) -> anyhow::Result<Usage200Response> {
        let response = self
            .platform()?
            .get_v1product_forks_by_product_id_by_fork_id_usage(
                GetV1productForksByProductIdByForkIdUsageRequest {
                    path: GetV1productForksByProductIdByForkIdUsageRequestPath {
                        product_id,
                        fork_id,
                    },
                },
            )
            .await?;
        api_success!(
            response,
            GetV1productForksByProductIdByForkIdUsageRequest,
            GetV1productForksByProductIdByForkIdUsageResponse::Ok
        )
    }

    pub async fn product_fork_policy(
        &self,
        product_id: Uuid,
        source_id: Uuid,
    ) -> anyhow::Result<ProductForkPolicy200Response> {
        let response = self
            .platform()?
            .get_v1product_fork_policies_by_product_id_by_source_id(
                GetV1productForkPoliciesByProductIdBySourceIdRequest {
                    path: GetV1productForkPoliciesByProductIdBySourceIdRequestPath {
                        product_id,
                        source_id,
                    },
                },
            )
            .await?;
        api_success!(
            response,
            GetV1productForkPoliciesByProductIdBySourceIdRequest,
            GetV1productForkPoliciesByProductIdBySourceIdResponse::Ok
        )
    }

    pub async fn put_product_fork_policy(
        &self,
        product_id: Uuid,
        source_id: Uuid,
        body: ProductForkPolicyRequestBody,
    ) -> anyhow::Result<ProductForkPolicy200Response> {
        let response = self
            .platform()?
            .put_v1product_fork_policies_by_product_id_by_source_id(
                PutV1productForkPoliciesByProductIdBySourceIdRequest {
                    path: PutV1productForkPoliciesByProductIdBySourceIdRequestPath {
                        product_id,
                        source_id,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PutV1productForkPoliciesByProductIdBySourceIdRequest,
            GetV1productForkPoliciesByProductIdBySourceIdResponse::Ok
        )
    }

    pub async fn disable_product_fork_policy(
        &self,
        product_id: Uuid,
        source_id: Uuid,
        body: ExpectedGenerationRequestBody,
    ) -> anyhow::Result<ProductForkPolicy200Response> {
        let response = self
            .platform()?
            .delete_v1product_fork_policies_by_product_id_by_source_id(
                DeleteV1productForkPoliciesByProductIdBySourceIdRequest {
                    path: DeleteV1productForkPoliciesByProductIdBySourceIdRequestPath {
                        product_id,
                        source_id,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            DeleteV1productForkPoliciesByProductIdBySourceIdRequest,
            GetV1productForkPoliciesByProductIdBySourceIdResponse::Ok
        )
    }

    pub async fn create_product_fork(
        &self,
        product_id: Uuid,
        body: PlanRequestBody,
    ) -> anyhow::Result<ProductFork200Response> {
        let response = self
            .platform()?
            .post_v1product_forks_by_product_id(PostV1productForksByProductIdRequest {
                path: PostV1productForksByProductIdRequestPath { product_id },
                body,
            })
            .await?;
        api_success!(
            response,
            PostV1productForksByProductIdRequest,
            PostV1productForksByProductIdResponse::Ok
        )
    }
    pub async fn product_fork(
        &self,
        product_id: Uuid,
        fork_id: Uuid,
    ) -> anyhow::Result<ProductFork200Response> {
        let response = self
            .platform()?
            .get_v1product_forks_by_product_id_by_fork_id(
                GetV1productForksByProductIdByForkIdRequest {
                    path: GetV1productForksByProductIdByForkIdRequestPath {
                        product_id,
                        fork_id,
                    },
                },
            )
            .await?;
        api_success!(
            response,
            GetV1productForksByProductIdByForkIdRequest,
            PostV1productForksByProductIdResponse::Ok
        )
    }
    pub async fn retain_product_fork(
        &self,
        product_id: Uuid,
        fork_id: Uuid,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<ProductFork200Response> {
        let response = self
            .platform()?
            .patch_v1product_forks_by_product_id_by_fork_id(
                PatchV1productForksByProductIdByForkIdRequest {
                    path: PatchV1productForksByProductIdByForkIdRequestPath {
                        product_id,
                        fork_id,
                    },
                    body: ExpiresAtRequestBody {
                        expires_at,
                        additional_properties: Default::default(),
                    },
                },
            )
            .await?;
        api_success!(
            response,
            PatchV1productForksByProductIdByForkIdRequest,
            PostV1productForksByProductIdResponse::Ok
        )
    }
    pub async fn delete_product_fork(
        &self,
        product_id: Uuid,
        fork_id: Uuid,
    ) -> anyhow::Result<ProductFork200Response> {
        let response = self
            .platform()?
            .delete_v1product_forks_by_product_id_by_fork_id(
                DeleteV1productForksByProductIdByForkIdRequest {
                    path: DeleteV1productForksByProductIdByForkIdRequestPath {
                        product_id,
                        fork_id,
                    },
                },
            )
            .await?;
        api_success!(
            response,
            DeleteV1productForksByProductIdByForkIdRequest,
            PostV1productForksByProductIdResponse::Ok
        )
    }
    pub async fn update_product_fork(
        &self,
        product_id: Uuid,
        fork_id: Uuid,
        body: UpdateRequestBody,
    ) -> anyhow::Result<ProductFork200Response> {
        let response = self
            .platform()?
            .post_v1product_forks_by_product_id_by_fork_id_update(
                PostV1productForksByProductIdByForkIdUpdateRequest {
                    path: PostV1productForksByProductIdByForkIdUpdateRequestPath {
                        product_id,
                        fork_id,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PostV1productForksByProductIdByForkIdUpdateRequest,
            PostV1productForksByProductIdResponse::Ok
        )
    }
    pub async fn preview_service_publication(
        &self,
        id: Uuid,
        body: PreviewPublicationRequestBody,
    ) -> anyhow::Result<PreviewPublication200Response> {
        let response = self
            .platform()?
            .post_v1environments_by_id_actions_preview_publication(
                PostV1environmentsByIdActionsPreviewPublicationRequest {
                    path: PostV1environmentsByIdActionsPreviewPublicationRequestPath { id },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PostV1environmentsByIdActionsPreviewPublicationRequest,
            PostV1environmentsByIdActionsPreviewPublicationResponse::Ok
        )
    }
    pub async fn publish_services(
        &self,
        id: Uuid,
        key: Uuid,
        body: PublishServiceRequestBody,
    ) -> anyhow::Result<PublishService200Response> {
        let response = self
            .platform()?
            .post_v1environments_by_id_actions_publish_services(
                PostV1environmentsByIdActionsPublishServicesRequest {
                    path: PostV1environmentsByIdActionsPublishServicesRequestPath { id },
                    header: PostV1environmentsByIdActionsPublishServicesRequestHeader {
                        idempotency_key: key.to_string(),
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PostV1environmentsByIdActionsPublishServicesRequest,
            PostV1environmentsByIdActionsPublishServicesResponse::Ok
        )
    }
}
