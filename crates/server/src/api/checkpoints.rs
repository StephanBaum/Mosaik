//! `PUT /v1/checkpoints/{deviceId}` and `GET /v1/checkpoints` (CRYPTO_SPEC
//! §10). The body is the checkpoint in its wire format; the server cannot
//! verify the MAC (it holds no key) and checks length, shape, that the body
//! names the path's device and that the counter only grows. The clients
//! verify the MAC.

use axum::body::Bytes;
use axum::extract::{Path, State};
use mosaic_core::InstallId;
use mosaic_sync::wire::checkpoint::Checkpoint;
use mosaic_sync::wire::json::{
    B64, CheckpointEntry, CheckpointsResponse, MAX_CHECKPOINT_UPLOAD_LEN, codes,
};

use super::{Reply, parse_uuid};
use crate::AppState;
use crate::auth::Session;
use crate::db::checkpoints;
use crate::error::ApiError;

pub async fn put(
    State(st): State<AppState>,
    Session(s): Session,
    Path(device_id): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let device = parse_uuid(&device_id)?;
    if body.len() > MAX_CHECKPOINT_UPLOAD_LEN {
        return Err(ApiError::too_large());
    }
    let install = InstallId::from_bytes(device).map_err(|_| ApiError::bad_request())?;
    let checkpoint =
        Checkpoint::decode_unverified(&body, install).map_err(|_| ApiError::bad_request())?;
    // A device uploads only its own checkpoint.
    if device != s.device_id {
        return Err(ApiError::forbidden());
    }
    let now = st.clock.now_ms();
    let stored = st
        .db
        .run(|c| checkpoints::put(c, &s.account_id, &device, checkpoint.counter, &body, now))?;
    if stored {
        Reply::empty()
    } else {
        Err(ApiError::conflict(codes::CHECKPOINT_STALE))
    }
}

pub async fn list(State(st): State<AppState>, Session(s): Session) -> Result<Reply, ApiError> {
    let rows = st.db.run(|c| checkpoints::list(c, &s.account_id))?;
    let checkpoints = rows
        .into_iter()
        .map(|(device, body)| {
            Ok(CheckpointEntry {
                device_id: InstallId::from_bytes(device).map_err(|_| ApiError::internal())?,
                body: B64(body),
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Reply::ok(&CheckpointsResponse { checkpoints })
}
