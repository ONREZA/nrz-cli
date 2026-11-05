use nrz_api::*;

use super::ApiClient;

impl ApiClient {
    pub async fn kv2_regions(
        &self,
        store_id: Option<uuid::Uuid>,
    ) -> anyhow::Result<RegionResponse> {
        let response = self
            .platform()?
            .get_v1kv_v2regions(GetV1kvV2regionsRequest {
                query: GetV1kvV2regionsRequestQuery { store_id },
            })
            .await?;
        api_success!(
            response,
            GetV1kvV2regionsRequest,
            GetV1kvV2regionsResponse::Ok
        )
    }
    pub async fn kv2_stores(
        &self,
        after: Option<uuid::Uuid>,
        limit: u32,
    ) -> anyhow::Result<Store200Response> {
        let response = self
            .platform()?
            .get_v1kv_v2stores(GetV1kvV2storesRequest {
                query: GetV1kvV2storesRequestQuery {
                    after,
                    limit: Some(limit.into()),
                },
            })
            .await?;
        api_success!(
            response,
            GetV1kvV2storesRequest,
            GetV1kvV2storesResponse::Ok
        )
    }
    pub async fn kv2_create_store(
        &self,
        body: StoreRequestBody,
    ) -> anyhow::Result<Store200Response2> {
        let response = self
            .platform()?
            .post_v1kv_v2stores(PostV1kvV2storesRequest { body })
            .await?;
        api_success!(
            response,
            PostV1kvV2storesRequest,
            PostV1kvV2storesResponse::Ok
        )
    }
    pub async fn kv2_store(&self, store_id: uuid::Uuid) -> anyhow::Result<Store200ResponseStore> {
        let response = self
            .platform()?
            .get_v1kv_v2stores_by_store_id(GetV1kvV2storesByStoreIdRequest {
                path: GetV1kvV2storesByStoreIdRequestPath { store_id },
            })
            .await?;
        api_success!(
            response,
            GetV1kvV2storesByStoreIdRequest,
            GetV1kvV2storesByStoreIdResponse::Ok
        )
    }
    pub async fn kv2_delete_store(&self, store_id: uuid::Uuid) -> anyhow::Result<SuccessResponse> {
        let response = self
            .platform()?
            .delete_v1kv_v2stores_by_store_id(DeleteV1kvV2storesByStoreIdRequest {
                path: DeleteV1kvV2storesByStoreIdRequestPath { store_id },
            })
            .await?;
        api_success!(
            response,
            DeleteV1kvV2storesByStoreIdRequest,
            DeleteV1kvV2storesByStoreIdResponse::Ok
        )
    }
    pub async fn kv2_branches(
        &self,
        store_id: uuid::Uuid,
        after: Option<uuid::Uuid>,
        limit: u32,
    ) -> anyhow::Result<Branch200Response> {
        let response = self
            .platform()?
            .get_v1kv_v2stores_by_store_id_branches(GetV1kvV2storesByStoreIdBranchesRequest {
                path: GetV1kvV2storesByStoreIdBranchesRequestPath { store_id },
                query: GetV1kvV2storesByStoreIdBranchesRequestQuery {
                    after,
                    limit: Some(limit.into()),
                },
            })
            .await?;
        api_success!(
            response,
            GetV1kvV2storesByStoreIdBranchesRequest,
            GetV1kvV2storesByStoreIdBranchesResponse::Ok
        )
    }
    pub async fn kv2_create_branch(
        &self,
        store_id: uuid::Uuid,
        body: BranchRequestBody,
    ) -> anyhow::Result<Store200Response2Branch> {
        let response = self
            .platform()?
            .post_v1kv_v2stores_by_store_id_branches(PostV1kvV2storesByStoreIdBranchesRequest {
                path: PostV1kvV2storesByStoreIdBranchesRequestPath { store_id },
                body,
            })
            .await?;
        api_success!(
            response,
            PostV1kvV2storesByStoreIdBranchesRequest,
            PostV1kvV2storesByStoreIdBranchesResponse::Ok
        )
    }
    pub async fn kv2_branch(
        &self,
        store_id: uuid::Uuid,
        branch_id: uuid::Uuid,
    ) -> anyhow::Result<Store200Response2Branch> {
        let response = self
            .platform()?
            .get_v1kv_v2stores_by_store_id_branches_by_branch_id(
                GetV1kvV2storesByStoreIdBranchesByBranchIdRequest {
                    path: GetV1kvV2storesByStoreIdBranchesByBranchIdRequestPath {
                        store_id,
                        branch_id,
                    },
                },
            )
            .await?;
        api_success!(
            response,
            GetV1kvV2storesByStoreIdBranchesByBranchIdRequest,
            PostV1kvV2storesByStoreIdBranchesResponse::Ok
        )
    }
    pub async fn kv2_delete_branch(
        &self,
        store_id: uuid::Uuid,
        branch_id: uuid::Uuid,
    ) -> anyhow::Result<SuccessResponse> {
        let response = self
            .platform()?
            .delete_v1kv_v2stores_by_store_id_branches_by_branch_id(
                DeleteV1kvV2storesByStoreIdBranchesByBranchIdRequest {
                    path: DeleteV1kvV2storesByStoreIdBranchesByBranchIdRequestPath {
                        store_id,
                        branch_id,
                    },
                },
            )
            .await?;
        api_success!(
            response,
            DeleteV1kvV2storesByStoreIdBranchesByBranchIdRequest,
            DeleteV1kvV2storesByStoreIdResponse::Ok
        )
    }
    pub async fn kv2_batch_window(
        &self,
        store_id: uuid::Uuid,
        branch_id: uuid::Uuid,
        batch_window_ms: i64,
    ) -> anyhow::Result<Store200Response2Branch> {
        let response = self
            .platform()?
            .patch_v1kv_v2stores_by_store_id_branches_by_branch_id_batch_window(
                PatchV1kvV2storesByStoreIdBranchesByBranchIdBatchWindowRequest {
                    path: PatchV1kvV2storesByStoreIdBranchesByBranchIdBatchWindowRequestPath {
                        store_id,
                        branch_id,
                    },
                    body: BatchWindowMRequestBody { batch_window_ms },
                },
            )
            .await?;
        api_success!(
            response,
            PatchV1kvV2storesByStoreIdBranchesByBranchIdBatchWindowRequest,
            PostV1kvV2storesByStoreIdBranchesResponse::Ok
        )
    }
    pub async fn kv2_request(
        &self,
        store_id: uuid::Uuid,
        branch_id: uuid::Uuid,
        body: RequestRequestBody,
    ) -> anyhow::Result<Request200Response> {
        let response = self
            .platform()?
            .post_v1kv_v2stores_by_store_id_branches_by_branch_id_request(
                PostV1kvV2storesByStoreIdBranchesByBranchIdRequestRequest {
                    path: PostV1kvV2storesByStoreIdBranchesByBranchIdRequestRequestPath {
                        store_id,
                        branch_id,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PostV1kvV2storesByStoreIdBranchesByBranchIdRequestRequest,
            PostV1kvV2storesByStoreIdBranchesByBranchIdRequestResponse::Ok
        )
    }
    pub async fn kv2_fork(
        &self,
        store_id: uuid::Uuid,
        branch_id: uuid::Uuid,
        body: ForkRequestBody,
    ) -> anyhow::Result<Store200Response2Branch> {
        let response = self
            .platform()?
            .post_v1kv_v2stores_by_store_id_branches_by_branch_id_fork(
                PostV1kvV2storesByStoreIdBranchesByBranchIdForkRequest {
                    path: PostV1kvV2storesByStoreIdBranchesByBranchIdForkRequestPath {
                        store_id,
                        branch_id,
                    },
                    body,
                },
            )
            .await?;
        match PostV1kvV2storesByStoreIdBranchesByBranchIdForkRequest::parse_response(
            super::client::ensure_success(response).await?,
        )
        .await
        .map_err(nrz_api::response_error)?
        {
            PostV1kvV2storesByStoreIdBranchesByBranchIdForkResponse::Ok(value)
            | PostV1kvV2storesByStoreIdBranchesByBranchIdForkResponse::Accepted(value) => Ok(value),
            _ => anyhow::bail!("unexpected successful response from the platform API"),
        }
    }
}
