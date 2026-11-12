//! `/v1/account`: the account record, password change, TOTP and deletion.
//! Everything that changes a credential needs a fresh OPAQUE login
//! (CRYPTO_SPEC §9.4).

use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::{AccountId, Argon2Profile, SecretString, WrappedKey};
use mosaic_sync::wire::json::{
    AccountResponse, AccountVersionDetail, B64, B64Fixed, CodeRequest, DeleteAccountRequest, Empty,
    OptionalCodeRequest, PasswordFinishRequest, PasswordFinishResponse, PasswordStartRequest,
    RegistrationStartResponse, SecretText, TotpSetupResponse, codes,
};
use mosaic_sync::wire::opaque::{
    Suite,
    ke::{RegistrationRequest, RegistrationUpload, ServerRegistration},
};
use serde::Deserialize;

use super::{Reply, parse_body};
use crate::AppState;
use crate::auth::{Authed, Session, totp};
use crate::db::accounts::{self, Account};
use crate::db::audit;
use crate::db::sessions;
use crate::db::sharing as dbs;
use crate::error::ApiError;

const MAX_BODY: usize = 16 * 1024;

fn two_factor_invalid() -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, codes::TWO_FACTOR_INVALID)
}

fn version_mismatch(current: u64) -> ApiError {
    ApiError::conflict(codes::ACCOUNT_VERSION_MISMATCH).with_detail(&AccountVersionDetail {
        account_version: current,
    })
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

/// The second factor for removing TOTP, a security key or a device and for
/// deleting the account (CRYPTO_SPEC §9.3 "Recent second factor"): nothing
/// when two-step verification is off; otherwise a valid TOTP `code` in the
/// request, or a `Confirm` handoff of this session within the last five
/// minutes.
pub fn require_second_factor(
    st: &AppState,
    s: &Authed,
    code: Option<&str>,
) -> Result<(), ApiError> {
    let (a, keys) = st.db.run(|c| {
        let a = accounts::by_id(c, &s.account_id)?.ok_or_else(ApiError::unauthorized)?;
        Ok((a, crate::db::webauthn::count(c, &s.account_id)?))
    })?;
    if a.totp_sealed.is_none() && keys == 0 {
        return Ok(());
    }
    if let (Some(code), Some(_)) = (code, a.totp_sealed.as_ref()) {
        check_code(
            st,
            &s.account_id,
            a.totp_sealed.as_deref(),
            code,
            a.totp_last_step,
        )?;
        return Ok(());
    }
    if s.second_factor_recent() {
        Ok(())
    } else {
        Err(two_factor_invalid())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountQuery {
    /// `1`: add `identity` (M6b). Without it the M6 body, which older
    /// clients parse strictly.
    identity: Option<u8>,
    /// `1`: add `akEpoch` (M7). A separate flag, for the same reason.
    epoch: Option<u8>,
}

pub async fn get(
    State(st): State<AppState>,
    Session(s): Session,
    query: Result<Query<AccountQuery>, QueryRejection>,
) -> Result<Reply, ApiError> {
    let Query(q) = query.map_err(|_| ApiError::bad_request())?;
    let a = load(&st, &s.account_id)?;
    let response = AccountResponse {
        account_id: AccountId::from_bytes(a.account_id).map_err(|_| ApiError::internal())?,
        email: a.email,
        salt: B64Fixed(a.salt),
        profile: Argon2Profile::from_id(a.profile).map_err(|_| ApiError::internal())?,
        wrapped_ak: WrappedKey::from_bytes(&a.wrapped_ak).map_err(|_| ApiError::internal())?,
        account_version: a.account_version,
        totp_enabled: a.totp_sealed.is_some(),
    };
    if q.identity != Some(1) && q.epoch != Some(1) {
        return Reply::ok(&response);
    }
    let mut value = serde_json::to_value(&response).map_err(|_| ApiError::internal())?;
    if q.epoch == Some(1) {
        let epoch = st.db.run(|c| crate::db::rekey::epoch(c, &s.account_id))?;
        if let Some(map) = value.as_object_mut() {
            map.insert("akEpoch".to_owned(), epoch.into());
        }
    }
    if q.identity == Some(1) {
        let identity = st.db.run(|c| dbs::identity(c, &s.account_id))?;
        if let Some(map) = value.as_object_mut() {
            map.insert(
                "identity".to_owned(),
                identity.map_or(serde_json::Value::Null, |(ipk, wrapped)| {
                    serde_json::json!({
                        "ipk": Base64UrlUnpadded::encode_string(&ipk),
                        "wrappedIsk": Base64UrlUnpadded::encode_string(&wrapped),
                    })
                }),
            );
        }
    }
    Reply::ok(&value)
}

pub async fn password_start(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: PasswordStartRequest = parse_body(&body, MAX_BODY)?;
    s.require_fresh()?;
    // The account must still be at the version this session logged in at: a
    // change made on another device since then is not overwritten.
    let a = load(&st, &s.account_id)?;
    if a.account_version != s.account_version {
        return Err(version_mismatch(a.account_version));
    }
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
            s.account_version,
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
        // Every other session of the account ends with the old password.
        sessions::after_password_change(
            tx,
            &s.account_id,
            &s.token_hash,
            v,
            sessions::Revoke::PasswordChange,
            now,
        )?;
        Ok(v)
    })?;
    // Logins and handoffs that started against the old password end with it.
    st.logins.drop_account(&s.account_id);
    st.handoffs.drop_account(&s.account_id);
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
    let req: OptionalCodeRequest = parse_body(&body, 1024)?;
    s.require_fresh()?;
    let a = load(&st, &s.account_id)?;
    if a.totp_sealed.is_none() {
        return Err(two_factor_invalid());
    }
    require_second_factor(
        &st,
        &s,
        req.code
            .as_ref()
            .map(mosaic_sync::wire::json::SecretText::expose_secret),
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
    require_second_factor(
        &st,
        &s,
        req.code
            .as_ref()
            .map(mosaic_sync::wire::json::SecretText::expose_secret),
    )?;
    let blobs = st.db.run_tx(|tx| accounts::delete(tx, &s.account_id))?;
    for id in blobs {
        st.blobs.remove(&id)?;
    }
    Reply::ok(&Empty {})
}
