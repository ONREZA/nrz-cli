use super::*;
use crate::{
    RuntimeArtifactPublicationTransport, SourcePublicationTransport, StructuredControlPlaneError,
};
use nrz_api::*;
use serde::Deserialize;
use validator::Validate as _;

macro_rules! sdk_success {
    ($response:expr, $request:ty, $variant:path) => {
        match <$request>::parse_response(ensure_success($response).await?)
            .await
            .map_err(sdk_error)?
        {
            $variant(value) => Ok(value),
            _ => Err(SourcePublicationError::InvalidResponse(
                "unexpected successful HTTP response".into(),
            )),
        }
    };
}

impl HttpSourcePublicationTransport {
    fn platform(&self) -> Result<PlatformClientRaw, SourcePublicationError> {
        PlatformClient::with_client(&self.base_url, self.api_client.clone())
            .map(|client| client.raw())
            .map_err(sdk_error)
    }
}

impl SourcePublicationTransport for HttpSourcePublicationTransport {
    async fn prepare_upload(
        &self,
        id: uuid::Uuid,
        body: &PrepareUploadRequestBody,
    ) -> Result<PrepareUpload200Response, SourcePublicationError> {
        let response = self
            .platform()?
            .post_v1deployments_by_id_prepare_upload(PostV1deploymentsByIdPrepareUploadRequest {
                path: PostV1deploymentsByIdPrepareUploadRequestPath { id },
                body: body.clone(),
            })
            .await
            .map_err(sdk_error)?;
        let value = sdk_success!(
            response,
            PostV1deploymentsByIdPrepareUploadRequest,
            PostV1deploymentsByIdPrepareUploadResponse::Ok
        )?;
        value.validate().map_err(|_| invalid_response())?;
        Ok(value)
    }
    async fn complete_multipart(
        &self,
        id: uuid::Uuid,
        body: &MultipartCompleteRequestBody,
    ) -> Result<MultipartComplete200Response, SourcePublicationError> {
        let response = self
            .platform()?
            .post_v1deployments_by_id_multipart_complete(
                PostV1deploymentsByIdMultipartCompleteRequest {
                    path: PostV1deploymentsByIdMultipartCompleteRequestPath { id },
                    body: body.clone(),
                },
            )
            .await
            .map_err(sdk_error)?;
        let value = sdk_success!(
            response,
            PostV1deploymentsByIdMultipartCompleteRequest,
            PostV1deploymentsByIdMultipartCompleteResponse::Ok
        )?;
        let matches = match &value {
            MultipartComplete200Response::Object(value) => {
                value.deployment_id == id && value.upload_session_id == body.upload_session_id
            }
            MultipartComplete200Response::Object2(value) => value.deployment_id == id,
        };
        ensure_binding(matches)?;
        Ok(value)
    }
    async fn complete_upload(
        &self,
        id: uuid::Uuid,
        body: &UploadCompleteRequestBody,
    ) -> Result<UploadComplete200Response, SourcePublicationError> {
        let response = self
            .platform()?
            .post_v1deployments_by_id_upload_complete(PostV1deploymentsByIdUploadCompleteRequest {
                path: PostV1deploymentsByIdUploadCompleteRequestPath { id },
                body: body.clone(),
            })
            .await
            .map_err(sdk_error)?;
        let value = sdk_success!(
            response,
            PostV1deploymentsByIdUploadCompleteRequest,
            PostV1deploymentsByIdUploadCompleteResponse::Ok
        )?;
        let matches = match &value {
            UploadComplete200Response::Object(value) => {
                value.deployment_id == id && value.upload_session_id == body.upload_session_id
            }
            UploadComplete200Response::Object2(value) => {
                value.deployment_id == id && value.upload_session_id == body.upload_session_id
            }
            UploadComplete200Response::Object3(value) => {
                value.deployment_id == id && value.upload_session_id == body.upload_session_id
            }
            UploadComplete200Response::Object4(value) => value.deployment_id == id,
            UploadComplete200Response::Object5(_) => true,
            UploadComplete200Response::Object6(value) => value.deployment_id == id,
        };
        ensure_binding(matches)?;
        Ok(value)
    }
    async fn report_upload_failed(
        &self,
        id: uuid::Uuid,
        body: &UploadFailedRequestBody,
    ) -> Result<UploadFailed200Response, SourcePublicationError> {
        let response = self
            .platform()?
            .post_v1deployments_by_id_upload_failed(PostV1deploymentsByIdUploadFailedRequest {
                path: PostV1deploymentsByIdUploadFailedRequestPath { id },
                body: body.clone(),
            })
            .await
            .map_err(sdk_error)?;
        let value = sdk_success!(
            response,
            PostV1deploymentsByIdUploadFailedRequest,
            PostV1deploymentsByIdUploadFailedResponse::Ok
        )?;
        let matches = match &value {
            UploadFailed200Response::Object(value) => {
                value.deployment_id == id && value.upload_session_id == body.upload_session_id
            }
            UploadFailed200Response::Object2(value) => {
                value.deployment_id == id && value.upload_session_id == body.upload_session_id
            }
        };
        ensure_binding(matches)?;
        Ok(value)
    }
    async fn deployment_status(
        &self,
        id: uuid::Uuid,
    ) -> Result<DeploymentPublicationStatus, SourcePublicationError> {
        let response = self
            .platform()?
            .get_v1deployments_by_id_status(GetV1deploymentsByIdStatusRequest {
                path: GetV1deploymentsByIdStatusRequestPath { id },
            })
            .await
            .map_err(sdk_error)?;
        let value = sdk_success!(
            response,
            GetV1deploymentsByIdStatusRequest,
            GetV1deploymentsByIdStatusResponse::Ok
        )?;
        ensure_binding(value.id == id)?;
        Ok(DeploymentPublicationStatus {
            status: value.status.to_string(),
            runtime_artifact_graph_digest: value.runtime_artifact_graph_digest,
            runtime_artifact_graph: value
                .runtime_artifact_graph
                .map(|graph| {
                    serde_json::from_value(
                        serde_json::to_value(graph).map_err(|_| invalid_response())?,
                    )
                    .map_err(|_| invalid_response())
                })
                .transpose()?,
            error: value.error,
            error_code: value.error_code,
        })
    }
    async fn put_object(
        &self,
        request: ObjectUploadRequest,
    ) -> Result<ObjectUploadResult, SourcePublicationError> {
        self.put_object_inner(request).await
    }
}

impl RuntimeArtifactPublicationTransport for HttpSourcePublicationTransport {
    async fn prepare_runtime_artifacts(
        &self,
        id: uuid::Uuid,
        body: &PrepareRequestBody,
    ) -> Result<Prepare200Response, SourcePublicationError> {
        let response = self
            .platform()?
            .post_v1deployments_by_id_runtime_artifacts_prepare(
                PostV1deploymentsByIdRuntimeArtifactsPrepareRequest {
                    path: PostV1deploymentsByIdRuntimeArtifactsPrepareRequestPath { id },
                    body: body.clone(),
                },
            )
            .await
            .map_err(sdk_error)?;
        let value = sdk_success!(
            response,
            PostV1deploymentsByIdRuntimeArtifactsPrepareRequest,
            PostV1deploymentsByIdRuntimeArtifactsPrepareResponse::Ok
        )?;
        value.validate().map_err(|_| invalid_response())?;
        Ok(value)
    }
    async fn complete_runtime_artifacts(
        &self,
        id: uuid::Uuid,
        body: &PrepareRequestBody,
    ) -> Result<Complete200Response, SourcePublicationError> {
        let response = self
            .platform()?
            .post_v1deployments_by_id_runtime_artifacts_complete(
                PostV1deploymentsByIdRuntimeArtifactsCompleteRequest {
                    path: PostV1deploymentsByIdRuntimeArtifactsCompleteRequestPath { id },
                    body: body.clone(),
                },
            )
            .await
            .map_err(sdk_error)?;
        sdk_success!(
            response,
            PostV1deploymentsByIdRuntimeArtifactsCompleteRequest,
            PostV1deploymentsByIdRuntimeArtifactsCompleteResponse::Ok
        )
    }
    async fn put_runtime_file(
        &self,
        request: RuntimeFileUploadRequest,
    ) -> Result<ObjectUploadResult, SourcePublicationError> {
        self.put_file_inner(request).await
    }
}

fn ensure_binding(matches: bool) -> Result<(), SourcePublicationError> {
    if matches {
        Ok(())
    } else {
        Err(SourcePublicationError::InvalidResponse(
            "response belongs to another deployment or upload session".into(),
        ))
    }
}
fn invalid_response() -> SourcePublicationError {
    SourcePublicationError::InvalidResponse(
        "response does not match the platform OpenAPI contract".into(),
    )
}
fn sdk_error(error: anyhow::Error) -> SourcePublicationError {
    if error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<reqwest::Error>())
        .any(is_ambiguous_transport)
    {
        SourcePublicationError::AmbiguousTransport(
            "request or response transport interrupted".into(),
        )
    } else {
        invalid_response()
    }
}

#[derive(Deserialize, Default)]
struct ApiErrorBody {
    error: Option<String>,
    message: Option<String>,
    code: Option<serde_json::Value>,
    #[serde(alias = "retryAfterSeconds")]
    retry_after_seconds: Option<u64>,
    details: Option<serde_json::Value>,
}
async fn ensure_success(
    response: reqwest::Response,
) -> Result<reqwest::Response, SourcePublicationError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let retry_after = parse_retry_after(response.headers());
    let body = response.text().await.map_err(transport_error)?;
    let error = serde_json::from_str::<ApiErrorBody>(&body).unwrap_or_default();
    Err(StructuredControlPlaneError {
        status: status.as_u16(),
        code: error
            .code
            .map(|value| match value {
                serde_json::Value::String(v) => v,
                v => v.to_string(),
            })
            .unwrap_or_else(|| format!("HTTP_{}", status.as_u16())),
        message: error
            .message
            .or(error.error)
            .unwrap_or_else(|| status.to_string()),
        retry_after: error
            .retry_after_seconds
            .map(Duration::from_secs)
            .or(retry_after),
        details: error.details,
    }
    .into())
}
