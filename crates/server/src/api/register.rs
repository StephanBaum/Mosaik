//! `POST /v1/register/start` and `/register/finish`: OPAQUE registration
//! (CRYPTO_SPEC §9.2). The server never sees the OPAQUE password.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use mosaic_sync::wire::json::{
    B64, Empty, RegisterFinishRequest, RegisterStartRequest, RegistrationMode,
    RegistrationStartResponse, codes,
};
use mosaic_sync::wire::opaque::{
    Suite,
    ke::{RegistrationRequest, RegistrationUpload, ServerRegistration},
};

use super::prelogin::normalized_email;
use super::{Gated, RegisterFinishGate, RegisterStartGate, Reply, parse_body, valid_device_name};
use crate::AppState;
use crate::db::{accounts, audit, devices, invites};
use crate::error::ApiError;

const MAX_BODY: usize = 16 * 1024;

fn check_open(st: &AppState) -> Result<(), ApiError> {
    if st.registration == RegistrationMode::Closed {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            codes::REGISTRATION_CLOSED,
        ));
    }
    Ok(())
}

pub async fn start(
    State(st): State<AppState>,
    _gate: Gated<RegisterStartGate>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: RegisterStartRequest = parse_body(&body, MAX_BODY)?;
    check_open(&st)?;
    normalized_email(&req.email)?;
    let request = RegistrationRequest::<Suite>::deserialize(&req.registration_request.0)
        .map_err(|_| ApiError::bad_request())?;
    // Stateless and the same for a registered and an unregistered email: a
    // taken email shows only at `finish` (THREAT_MODEL R58).
    let started =
        ServerRegistration::<Suite>::start(st.setup(), request, req.account_id.as_bytes())
            .map_err(|_| ApiError::bad_request())?;
    Reply::ok(&RegistrationStartResponse {
        registration_response: B64(started.message.serialize().to_vec()),
    })
}

pub async fn finish(
    State(st): State<AppState>,
    _gate: Gated<RegisterFinishGate>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: RegisterFinishRequest = parse_body(&body, MAX_BODY)?;
    let now = st.clock.now_ms();
    check_open(&st)?;
    let email = normalized_email(&req.email)?;
    if !valid_device_name(&req.device_name) {
        return Err(ApiError::bad_request());
    }
    let upload = RegistrationUpload::<Suite>::deserialize(&req.registration_upload.0)
        .map_err(|_| ApiError::bad_request())?;
    let record = ServerRegistration::<Suite>::finish(upload)
        .serialize()
        .to_vec();
    let invite = if st.registration == RegistrationMode::Invite {
        let code = req
            .invite_code
            .as_deref()
            .and_then(invites::normalize)
            .ok_or_else(|| ApiError::new(StatusCode::FORBIDDEN, codes::INVALID_INVITE))?;
        Some(invites::hash(&code))
    } else {
        None
    };
    let account = *req.account_id.as_bytes();
    let device = *req.device_id.as_bytes();
    st.db.run_tx(|tx| {
        if let Some(hash) = &invite
            && !invites::redeem(tx, hash, now)?
        {
            return Err(ApiError::new(StatusCode::FORBIDDEN, codes::INVALID_INVITE));
        }
        accounts::insert(
            tx,
            &accounts::NewAccount {
                account_id: account,
                email: &email,
                salt: req.salt.0,
                profile: req.profile.id(),
                opaque_record: &record,
                wrapped_ak: req.wrapped_ak.as_bytes(),
            },
            now,
        )?;
        devices::upsert_on_login(tx, &account, &device, &req.device_name, now)?;
        audit::record(tx, &account, now, audit::Kind::Created, Some(&device))
    })?;
    Reply::created(&Empty {})
}
