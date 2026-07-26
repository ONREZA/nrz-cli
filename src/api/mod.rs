pub mod client;

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

#[cfg(test)]
mod client_tests;

pub use client::ApiClient;
pub(crate) use client::ConditionalUploadConflict;
pub(crate) use client::PresignedHeadVerify;
pub(crate) use client::PresignedPutHeaders;
pub use client::StructuredApiError;
pub(crate) use client::classify_api_retry;

const API_COMPONENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'!')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b'.')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}')
    .add(b'~');

pub(crate) fn path_segment(value: &str) -> String {
    utf8_percent_encode(value, API_COMPONENT).to_string()
}

pub(crate) fn query_value(value: &str) -> String {
    utf8_percent_encode(value, API_COMPONENT).to_string()
}
