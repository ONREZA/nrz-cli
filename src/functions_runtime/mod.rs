mod preflight;
mod process;
mod release;

#[cfg(test)]
mod tests;

pub(super) const RUNTIME_PROTOCOL_VERSION: &str = "onreza-functions-poc/v2";

pub(crate) use preflight::{RuntimePreflight, preflight};
pub(crate) use release::{CachedRuntime, RuntimeResolver};
