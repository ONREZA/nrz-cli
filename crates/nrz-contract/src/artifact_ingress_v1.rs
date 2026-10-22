//! Closed external Source V2 transfer control. Payloads use raw iroh-blobs Get.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ALPN: &[u8] = b"onreza/artifact-ingress/1";
pub const MAX_OPEN_BYTES: usize = 8 * 1024 + 1024;
pub const MAX_REPLY_BYTES: usize = 32 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Open {
    pub transfer_grant: String,
}
impl Open {
    pub fn validate(&self) -> bool {
        !self.transfer_grant.is_empty() && self.transfer_grant.len() <= 8 * 1024
    }
}
#[derive(Serialize, Deserialize)]
#[serde(
    tag = "result",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum Reply {
    Accepted {
        operation_id: Uuid,
        session_id: Uuid,
    },
    Published {
        operation_id: Uuid,
        session_id: Uuid,
        manifest_blake3: String,
    },
    Unavailable,
}
