//! `/v1/devices`: the device list, rename and removal.

use axum::body::Bytes;
use axum::extract::{Path, State};
use mosaic_core::InstallId;
use mosaic_sync::wire::json::{
    DeviceEntry, DevicesResponse, OptionalCodeRequest, RenameDeviceRequest, SecretText,
};

use super::{Reply, parse_body, parse_uuid, valid_device_name};
use crate::AppState;
use crate::auth::Session;
use crate::db::{audit, devices};
use crate::error::ApiError;
use mosaic_sync::wire::json::codes;

pub async fn list(State(st): State<AppState>, Session(s): Session) -> Result<Reply, ApiError> {
    let rows = st.db.run(|c| devices::list(c, &s.account_id))?;
    let devices = rows
        .into_iter()
        .map(|d| {
            Ok(DeviceEntry {
                device_id: InstallId::from_bytes(d.device_id).map_err(|_| ApiError::internal())?,
                name: d.name,
                created_at: d.created_at,
                last_seen_at: d.last_seen_at,
                current: d.device_id == s.device_id,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Reply::ok(&DevicesResponse { devices })
}

pub async fn rename(
    State(st): State<AppState>,
    Session(s): Session,
    Path(device_id): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let device = parse_uuid(&device_id)?;
    let req: RenameDeviceRequest = parse_body(&body, 1024)?;
    if !valid_device_name(&req.name) {
        return Err(ApiError::bad_request());
    }
    if st
        .db
        .run(|c| devices::rename(c, &s.account_id, &device, &req.name))?
    {
        Reply::empty()
    } else {
        Err(ApiError::not_found())
    }
}

pub async fn remove(
    State(st): State<AppState>,
    Session(s): Session,
    Path(device_id): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let device = parse_uuid(&device_id)?;
    // The body is optional: `{"code": ...}` when two-step verification is on
    // and the session has no recent `Confirm` handoff.
    let req: OptionalCodeRequest = if body.is_empty() {
        OptionalCodeRequest::default()
    } else {
        parse_body(&body, 1024)?
    };
    s.require_fresh()?;
    // An account with a security key must show a second factor (CRYPTO_SPEC
    // §9.3). A TOTP-only account is not asked yet: the M6 client sends no
    // code with this request, and T3 adds it to the client first.
    if st
        .db
        .run(|c| crate::db::webauthn::count(c, &s.account_id))?
        > 0
    {
        super::account::require_second_factor(
            &st,
            &s,
            req.code.as_ref().map(SecretText::expose_secret),
        )?;
    }
    if device == s.device_id {
        return Err(ApiError::conflict(codes::DEVICE_IS_CURRENT));
    }
    let now = st.clock.now_ms();
    let removed = st.db.run_tx(|tx| {
        let removed = devices::revoke(tx, &s.account_id, &device, now)?;
        if removed {
            audit::record(
                tx,
                &s.account_id,
                now,
                audit::Kind::DeviceRemoved,
                Some(&device),
            )?;
        }
        Ok(removed)
    })?;
    if removed {
        Reply::empty()
    } else {
        Err(ApiError::not_found())
    }
}
