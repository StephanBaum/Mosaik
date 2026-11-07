//! `/v1/account`: the account record, password change, TOTP and deletion.
//! Everything that changes a credential needs a fresh OPAQUE login
//! (CRYPTO_SPEC §9.4).

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use mosaic_core::{AccountId, Argon2Profile, SecretString, WrappedKey};
use mosaic_sync::wire::json::{
    AccountResponse, B64, B64Fixed, CodeRequest, DeleteAccountRequest, Empty,
    PasswordFinishRequest, PasswordFinishResponse, PasswordStartRequest, RegistrationStartResponse,
    SecretText, TotpSetupResponse, codes,
};
use mosaic_sync::wire::opaque::{
    Suite,
    ke::{RegistrationRequest, RegistrationUpload, ServerRegistration},
};

use super::{Reply, parse_body};
use crate::AppState;
use crate::auth::{Session, totp};
use crate::db::accounts::{self, Account};
use crate::db::audit;
use crate::error::ApiError;

const MAX_BODY: usize = 16 * 1024;

fn two_factor_invalid() -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, codes::TWO_FACTOR_INVALID)
}

fn load(st: &AppState, id: &[u8; 16]) -> Result<Account, ApiError> {
    st.db
        .run(|c| accounts::by_id(c, id))?
        .ok_or_else(ApiError::unauthorized)
}

/// Checks `code` against a sealed secret with the replay guard; returns the
/// matched step.
fn check_code(
    st: &AppState,
    account_id: &[u8; 16],
    sealed: Option<&[u8]>,
    code: &str,
    last_step: Option<u64>,
) -> Result<u64, ApiError> {
    let step = sealed
        .and_then(|s| totp::open(&st.keys.server_secret, account_id, s))
        .and_then(|secret| totp::verify(&secret, code, st.clock.now_ms(), last_step))
        .ok_or_else(two_factor_invalid)?;
    // Conditional store: a code is accepted once even under parallel requests.
    if st
        .db
        .run(|c| accounts::set_last_step(c, account_id, step))?
    {
        Ok(step)
    } else {
        Err(two_factor_invalid())
    }
}

pub async fn get(State(st): State<AppState>, Session(s): Session) -> Result<Reply, ApiError> {
    let a = load(&st, &s.account_id)?;
    Reply::ok(&AccountResponse {
        account_id: AccountId::from_bytes(a.account_id).map_err(|_| ApiError::internal())?,
        email: a.email,
        salt: B64Fixed(a.salt),
        profile: Argon2Profile::from_id(a.profile).map_err(|_| ApiError::internal())?,
        wrapped_ak: WrappedKey::from_bytes(&a.wrapped_ak).map_err(|_| ApiError::internal())?,
        account_version: a.account_version,
        totp_enabled: a.totp_sealed.is_some(),
    })
}

pub async fn password_start(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: PasswordStartRequest = parse_body(&body, MAX_BODY)?;
    s.require_fresh()?;
    let request = RegistrationRequest::<Suite>::deserialize(&req.registration_request.0)
        .map_err(|_| ApiError::bad_request())?;
    let started = ServerRegistration::<Suite>::start(st.setup(), request, &s.account_id)
        .map_err(|_| ApiError::bad_request())?;
    Reply::ok(&RegistrationStartResponse {
        registration_response: B64(started.message.serialize().to_vec()),
    })
}

pub async fn password_finish(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: PasswordFinishRequest = parse_body(&body, MAX_BODY)?;
    s.require_fresh()?;
    let upload = RegistrationUpload::<Suite>::deserialize(&req.registration_upload.0)
        .map_err(|_| ApiError::bad_request())?;
    let record = ServerRegistration::<Suite>::finish(upload)
        .serialize()
        .to_vec();
    let now = st.clock.now_ms();
    let version = st.db.run_tx(|tx| {
        let v = accounts::update_password(
            tx,
            &s.account_id,
            req.expected_account_version,
            &record,
            req.wrapped_ak.as_bytes(),
            &req.salt.0,
            req.profile.id(),
        )?;
        audit::record(
            tx,
            &s.account_id,
            now,
            audit::Kind::PasswordChanged,
            Some(&s.device_id),
        )?;
        Ok(v)
    })?;
    Reply::ok(&PasswordFinishResponse {
        account_version: version,
    })
}

pub async fn totp_setup(
    State(st): State<AppState>,
    Session(s): Session,
) -> Result<Reply, ApiError> {
    s.require_fresh()?;
    let a = load(&st, &s.account_id)?;
    if a.totp_sealed.is_some() {
        // Disable the active second factor first.
        return Err(ApiError::bad_request());
    }
    let secret = totp::generate().ok_or_else(ApiError::internal)?;
    let sealed = totp::seal(&st.keys.server_secret, &s.account_id, &secret)
        .ok_or_else(ApiError::internal)?;
    let (b32, uri) =
        totp::setup_values(secret.as_slice(), &a.email).ok_or_else(ApiError::internal)?;
    st.db
        .run(|c| accounts::set_totp_pending(c, &s.account_id, &sealed))?;
    Reply::ok(&TotpSetupResponse {
        secret: SecretText::new(SecretString::from(b32.expose_secret().to_owned())),
        otpauth_uri: SecretText::new(SecretString::from(uri.expose_secret().to_owned())),
    })
}

pub async fn totp_confirm(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: CodeRequest = parse_body(&body, 1024)?;
    s.require_fresh()?;
    let a = load(&st, &s.account_id)?;
    let step = check_code(
        &st,
        &s.account_id,
        a.totp_pending_sealed.as_deref(),
        req.code.expose_secret(),
        None,
    )?;
    let now = st.clock.now_ms();
    st.db.run_tx(|tx| {
        accounts::confirm_totp(tx, &s.account_id, step)?;
        audit::record(
            tx,
            &s.account_id,
            now,
            audit::Kind::TotpOn,
            Some(&s.device_id),
        )
    })?;
    Reply::ok(&Empty {})
}

pub async fn totp_disable(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: CodeRequest = parse_body(&body, 1024)?;
    s.require_fresh()?;
    let a = load(&st, &s.account_id)?;
    check_code(
        &st,
        &s.account_id,
        a.totp_sealed.as_deref(),
        req.code.expose_secret(),
        a.totp_last_step,
    )?;
    let now = st.clock.now_ms();
    st.db.run_tx(|tx| {
        accounts::clear_totp(tx, &s.account_id)?;
        audit::record(
            tx,
            &s.account_id,
            now,
            audit::Kind::TotpOff,
            Some(&s.device_id),
        )
    })?;
    Reply::ok(&Empty {})
}

pub async fn delete(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: DeleteAccountRequest = parse_body(&body, 1024)?;
    s.require_fresh()?;
    let a = load(&st, &s.account_id)?;
    if a.totp_sealed.is_some() {
        let code = req
            .code
            .as_ref()
            .ok_or_else(two_factor_invalid)?
            .expose_secret();
        check_code(
            &st,
            &s.account_id,
            a.totp_sealed.as_deref(),
            code,
            a.totp_last_step,
        )?;
    }
    let blobs = st.db.run_tx(|tx| accounts::delete(tx, &s.account_id))?;
    for id in blobs {
        st.blobs.remove(&id)?;
    }
    Reply::ok(&Empty {})
}
