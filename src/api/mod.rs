pub mod client;

#[cfg(test)]
mod client_tests;

pub use client::ApiClient;
pub use client::StructuredApiError;
