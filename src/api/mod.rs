macro_rules! api_success {
    ($response:expr, $request:ty, $variant:path) => {
        match <$request>::parse_response(crate::api::client::ensure_success($response).await?)
            .await
            .map_err(nrz_api::response_error)?
        {
            $variant(value) => Ok(value),
            _ => anyhow::bail!("unexpected successful response from the platform API"),
        }
    };
}

mod build_logs;
pub mod client;
mod databases;
mod deployments;
mod domains;
mod environments;
mod execution;
mod functions;
mod identity;
mod preview;
mod projects;
mod publication;
mod runtime_logs;

#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod environment_tests;
#[cfg(test)]
mod functions_tests;

pub use client::ApiClient;
pub use client::StructuredApiError;
pub(crate) use client::classify_api_retry;
