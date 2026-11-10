//! `GET /v1/info`.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use mosaic_sync::wire::json::{
    Features, InfoFeaturesResponse, InfoResponse, Limits, PROTOCOL_VERSION,
};
use serde::Deserialize;

use super::Reply;
use crate::AppState;
use crate::error::ApiError;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InfoQuery {
    /// `1`: add `features` (M9). Without it the M6 body, which older clients
    /// parse strictly.
    features: Option<u8>,
}

/// The protocol version, the server identity (the OPAQUE `server_identity`),
/// who may register and the limits.
pub async fn get(
    State(st): State<AppState>,
    query: Result<Query<InfoQuery>, QueryRejection>,
) -> Result<Reply, ApiError> {
    let Query(q) = query.map_err(|_| ApiError::bad_request())?;
    let limits = Limits {
        max_request_bytes: st.limits.max_request_bytes,
        max_attachment_bytes: u64::try_from(st.max_attachment).unwrap_or(u64::MAX),
        account_attachment_bytes: st.limits.account_attachment_bytes,
        account_record_bytes: st.limits.account_record_bytes,
    };
    if q.features == Some(1) {
        return Reply::ok(&InfoFeaturesResponse {
            protocol: PROTOCOL_VERSION,
            server_identity: st.origin.clone(),
            registration: st.registration,
            limits,
            features: Features {
                webauthn: st.rp.is_some(),
            },
        });
    }
    Reply::ok(&InfoResponse {
        protocol: PROTOCOL_VERSION,
        server_identity: st.origin.clone(),
        registration: st.registration,
        limits,
    })
}
