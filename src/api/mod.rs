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
mod endpoints;
mod environments;
mod execution;
mod functions;
mod identity;
mod kv_v2;
mod preview;
mod product_copy;
mod projects;
mod publication;
mod releases;
mod runtime_logs;
mod service_config;
mod services;

#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod functions_tests;
#[cfg(test)]
mod service_config_tests;

pub use client::ApiClient;
pub use client::StructuredApiError;
pub(crate) use client::classify_api_retry;

pub use service_config::{
    ServiceConfigDeleteBody, ServiceConfigResponse, ServiceConfigSaveBody, ServiceConfigVariable,
};
