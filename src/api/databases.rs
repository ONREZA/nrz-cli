use super::ApiClient;
use anyhow::Context;
use nrz_api::*;

impl ApiClient {
    pub async fn databases(&self) -> anyhow::Result<Database200Response> {
        let response = self
            .platform()?
            .get_v1kaiki_databases(GetV1kaikiDatabasesRequest {})
            .await?;
        api_success!(
            response,
            GetV1kaikiDatabasesRequest,
            GetV1kaikiDatabasesResponse::Ok
        )
    }
    pub async fn create_database(
        &self,
        body: DatabaseRequestBody,
    ) -> anyhow::Result<Database200Response2> {
        let response = self
            .platform()?
            .post_v1kaiki_databases(PostV1kaikiDatabasesRequest { body })
            .await?;
        api_success!(
            response,
            PostV1kaikiDatabasesRequest,
            PostV1kaikiDatabasesResponse::Ok
        )
    }
    pub async fn database(&self, id: &str) -> anyhow::Result<Database200Response3> {
        let response = self
            .platform()?
            .get_v1kaiki_databases_by_id(GetV1kaikiDatabasesByIdRequest {
                path: GetV1kaikiDatabasesByIdRequestPath {
                    id: id.parse().context("invalid resource ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            GetV1kaikiDatabasesByIdRequest,
            GetV1kaikiDatabasesByIdResponse::Ok
        )
    }
    pub async fn delete_database(&self, id: &str) -> anyhow::Result<StatusResponse> {
        let response = self
            .platform()?
            .delete_v1kaiki_databases_by_id(DeleteV1kaikiDatabasesByIdRequest {
                path: DeleteV1kaikiDatabasesByIdRequestPath {
                    id: id.parse().context("invalid resource ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            DeleteV1kaikiDatabasesByIdRequest,
            DeleteV1kaikiDatabasesByIdResponse::Ok
        )
    }
    pub async fn start_database(&self, id: &str) -> anyhow::Result<Start200Response> {
        let response = self
            .platform()?
            .post_v1kaiki_databases_by_id_start(PostV1kaikiDatabasesByIdStartRequest {
                path: PostV1kaikiDatabasesByIdStartRequestPath {
                    id: id.parse().context("invalid resource ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            PostV1kaikiDatabasesByIdStartRequest,
            PostV1kaikiDatabasesByIdStopResponse::Ok
        )
    }
    pub async fn stop_database(&self, id: &str) -> anyhow::Result<Start200Response> {
        let response = self
            .platform()?
            .post_v1kaiki_databases_by_id_stop(PostV1kaikiDatabasesByIdStopRequest {
                path: PostV1kaikiDatabasesByIdStopRequestPath {
                    id: id.parse().context("invalid resource ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            PostV1kaikiDatabasesByIdStopRequest,
            PostV1kaikiDatabasesByIdStopResponse::Ok
        )
    }
    pub async fn database_connection(&self, id: &str) -> anyhow::Result<Connection200Response> {
        let response = self
            .platform()?
            .get_v1kaiki_databases_by_id_connection(GetV1kaikiDatabasesByIdConnectionRequest {
                path: GetV1kaikiDatabasesByIdConnectionRequestPath {
                    id: id.parse().context("invalid resource ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            GetV1kaikiDatabasesByIdConnectionRequest,
            GetV1kaikiDatabasesByIdConnectionResponse::Ok
        )
    }
    pub async fn database_branches(&self, id: &str) -> anyhow::Result<Branch200Response> {
        let response = self
            .platform()?
            .get_v1kaiki_databases_by_id_branches(GetV1kaikiDatabasesByIdBranchesRequest {
                path: GetV1kaikiDatabasesByIdBranchesRequestPath {
                    id: id.parse().context("invalid resource ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            GetV1kaikiDatabasesByIdBranchesRequest,
            GetV1kaikiDatabasesByIdBranchesResponse::Ok
        )
    }
    pub async fn create_database_branch(
        &self,
        id: &str,
        body: BranchRequestBody,
    ) -> anyhow::Result<Branch200Response2> {
        let response = self
            .platform()?
            .post_v1kaiki_databases_by_id_branches(PostV1kaikiDatabasesByIdBranchesRequest {
                path: PostV1kaikiDatabasesByIdBranchesRequestPath {
                    id: id.parse().context("invalid resource ID")?,
                },
                body,
            })
            .await?;
        api_success!(
            response,
            PostV1kaikiDatabasesByIdBranchesRequest,
            PostV1kaikiDatabasesByIdBranchesResponse::Ok
        )
    }
    pub async fn delete_database_branch(
        &self,
        id: &str,
        branch_id: &str,
    ) -> anyhow::Result<StatusResponse> {
        let response = self
            .platform()?
            .delete_v1kaiki_databases_by_id_branches_by_branch_id(
                DeleteV1kaikiDatabasesByIdBranchesByBranchIdRequest {
                    path: DeleteV1kaikiDatabasesByIdBranchesByBranchIdRequestPath {
                        id: id.parse().context("invalid resource ID")?,
                        branch_id: branch_id.to_string(),
                    },
                },
            )
            .await?;
        api_success!(
            response,
            DeleteV1kaikiDatabasesByIdBranchesByBranchIdRequest,
            DeleteV1kaikiDatabasesByIdResponse::Ok
        )
    }
    pub async fn database_branch_connection(
        &self,
        id: &str,
        branch_id: &str,
    ) -> anyhow::Result<Connection200Response> {
        let response = self
            .platform()?
            .get_v1kaiki_databases_by_id_branches_by_branch_id_connection(
                GetV1kaikiDatabasesByIdBranchesByBranchIdConnectionRequest {
                    path: GetV1kaikiDatabasesByIdBranchesByBranchIdConnectionRequestPath {
                        id: id.parse().context("invalid resource ID")?,
                        branch_id: branch_id.to_string(),
                    },
                },
            )
            .await?;
        api_success!(
            response,
            GetV1kaikiDatabasesByIdBranchesByBranchIdConnectionRequest,
            GetV1kaikiDatabasesByIdConnectionResponse::Ok
        )
    }
    pub async fn attach_database(
        &self,
        id: &str,
        project_id: &str,
        body: AttachmentRequestBody,
    ) -> anyhow::Result<Database200ResponseDatumProjectAttachment> {
        let response = self
            .platform()?
            .patch_v1kaiki_databases_by_id_attachments_by_project_id(
                PatchV1kaikiDatabasesByIdAttachmentsByProjectIdRequest {
                    path: PatchV1kaikiDatabasesByIdAttachmentsByProjectIdRequestPath {
                        id: id.parse().context("invalid resource ID")?,
                        project_id: project_id.parse().context("invalid resource ID")?,
                    },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PatchV1kaikiDatabasesByIdAttachmentsByProjectIdRequest,
            PatchV1kaikiDatabasesByIdAttachmentsByProjectIdResponse::Ok
        )
    }
}
