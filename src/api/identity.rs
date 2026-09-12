use anyhow::bail;
use nrz_api::{GetV1userRequest, GetV1userResponse, User200Response};

use super::{ApiClient, client::ensure_success};

impl ApiClient {
    pub fn platform(&self) -> anyhow::Result<nrz_api::PlatformClientRaw> {
        Ok(
            nrz_api::PlatformClient::with_client(self.base_url(), self.http_client().clone())?
                .raw(),
        )
    }

    pub async fn user(&self) -> anyhow::Result<User200Response> {
        let response = self.platform()?.get_v1user(GetV1userRequest {}).await?;
        match GetV1userRequest::parse_response(ensure_success(response).await?)
            .await
            .map_err(nrz_api::response_error)?
        {
            GetV1userResponse::Ok(user) => Ok(user),
            _ => bail!("unexpected successful response from the user API"),
        }
    }
}
