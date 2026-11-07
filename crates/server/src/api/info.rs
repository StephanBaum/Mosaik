//! `GET /v1/info`.

use axum::extract::State;
use mosaic_sync::wire::json::{InfoResponse, Limits, PROTOCOL_VERSION};

use super::Reply;
use crate::AppState;
use crate::error::ApiError;

/// The protocol version, the server identity (the OPAQUE `server_identity`),
/// who may register and the limits.
pub async fn get(State(st): State<AppState>) -> Result<Reply, ApiError> {
    Reply::ok(&InfoResponse {
        protocol: PROTOCOL_VERSION,
        server_identity: st.origin.clone(),
        registration: st.registration,
        limits: Limits {
            max_request_bytes: st.limits.max_request_bytes,
            max_attachment_bytes: u64::try_from(st.max_attachment).unwrap_or(u64::MAX),
            account_attachment_bytes: st.limits.account_attachment_bytes,
            account_record_bytes: st.limits.account_record_bytes,
        },
    })
}
