//! The WebAuthn second factor: handoffs, the page's two routes, the key list
//! and removal (CRYPTO_SPEC §9.3, ADR-0077). No route has the handoff ID in
//! its path; it travels in request bodies only. Nothing here logs a code, a
//! challenge, a credential ID or library error text.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use mosaic_sync::wire::json::{
    AccountHandoffRequest, B64Fixed, HandoffFinishRequest, HandoffFinishResponse, HandoffKind,
    HandoffOptionsRequest, HandoffOptionsResponse, HandoffPurpose, HandoffStartResponse,
    HandoffState, HandoffStateRequest, HandoffStateResponse, LoginMethodsResponse,
    RemoveWebauthnRequest, WebauthnEntry, codes,
};
use zeroize::Zeroize;

use super::{Gated, HandoffGate, Reply, parse_body, valid_device_name};
use crate::AppState;
use crate::auth::handoff::{NewHandoff, Purpose, Refused, State as HState, Taken, Verified};
use crate::auth::webauthn::{CeremonyState, Rp, RpError, StoredCredential};
use crate::auth::{AnySession, Authed, Session};
use crate::db::accounts::SlotClaim;
use crate::db::webauthn::{self as dbw, InsertError};
use crate::db::{accounts, audit, devices, sessions};
use crate::error::ApiError;

const SMALL_BODY: usize = 2 * 1024;
/// The credential JSON is at most 16 KiB; the envelope adds a little.
pub const FINISH_BODY: usize = 16 * 1024 + 512;
/// A completed `Confirm` handoff counts for five minutes.
const CONFIRM_MS: u64 = 300_000;
/// The label of a key whose label the request left out.
const DEFAULT_LABEL: &str = "Security key";
/// Handoffs are refused for this long when a limit is hit.
const LIMIT_RETRY_S: u32 = 30;

fn two_factor_invalid() -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, codes::TWO_FACTOR_INVALID)
}

fn handoff_expired() -> ApiError {
    ApiError::new(StatusCode::GONE, codes::HANDOFF_EXPIRED)
}

fn unavailable() -> ApiError {
    ApiError::conflict(codes::WEBAUTHN_UNAVAILABLE)
}

fn rp(st: &AppState) -> Result<&Rp, ApiError> {
    st.rp.as_ref().ok_or_else(unavailable)
}

fn refused(r: Refused) -> ApiError {
    match r {
        Refused::Expired => handoff_expired(),
        Refused::Closed | Refused::WrongCode | Refused::NoState => two_factor_invalid(),
    }
}

fn stored(rows: &[dbw::Row]) -> Vec<StoredCredential> {
    rows.iter()
        .map(|r| StoredCredential {
            id: r.credential_id.clone(),
            public_key: r.public_key.clone(),
            sign_count: r.sign_count,
        })
        .collect()
}

pub async fn login_methods(
    State(st): State<AppState>,
    AnySession(s): AnySession,
) -> Result<Reply, ApiError> {
    if !s.pending {
        return Err(ApiError::bad_request());
    }
    let (totp, keys) = st.db.run(|c| {
        let a = accounts::by_id(c, &s.account_id)?.ok_or_else(ApiError::unauthorized)?;
        Ok((a.totp_sealed.is_some(), dbw::count(c, &s.account_id)?))
    })?;
    Reply::ok(&LoginMethodsResponse {
        totp,
        webauthn: keys > 0 && st.rp.is_some(),
    })
}

fn start(
    st: &AppState,
    s: &Authed,
    purpose: Purpose,
    label: Option<String>,
) -> Result<Reply, ApiError> {
    rp(st)?;
    let now = st.clock.now_ms();
    let name = st
        .db
        .run(|c| devices::list(c, &s.account_id))?
        .into_iter()
        .find(|d| d.device_id == s.device_id)
        .map_or_else(String::new, |d| d.name);
    // A login handoff ends with its pending session.
    let not_after = if purpose == Purpose::Login {
        s.expires_at
    } else {
        u64::MAX
    };
    let made = st
        .handoffs
        .create(
            now,
            NewHandoff {
                purpose,
                token_hash: s.token_hash,
                account_id: s.account_id,
                device_id: s.device_id,
                device_name: &name,
                label,
            },
            not_after,
        )
        .ok_or_else(ApiError::internal)?
        .map_err(|_| ApiError::rate_limited(LIMIT_RETRY_S))?;
    let mut response = HandoffStartResponse {
        handoff_id: B64Fixed(made.id),
        match_code: String::from_utf8_lossy(&made.match_code[..]).into_owned(),
        expires_at: made.expires,
    };
    let reply = Reply::ok(&response);
    // The wire type owns a plain `String`; wipe it once it is serialized (M9-F9).
    response.match_code.zeroize();
    reply
}

pub async fn login_handoff(
    State(st): State<AppState>,
    AnySession(s): AnySession,
) -> Result<Reply, ApiError> {
    if !s.pending {
        return Err(ApiError::bad_request());
    }
    let keys = st.db.run(|c| dbw::count(c, &s.account_id))?;
    if keys == 0 {
        return Err(ApiError::bad_request());
    }
    start(&st, &s, Purpose::Login, None)
}

pub async fn account_handoff(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: AccountHandoffRequest = parse_body(&body, SMALL_BODY)?;
    s.require_fresh()?;
    let keys = st.db.run(|c| dbw::count(c, &s.account_id))?;
    match req.purpose {
        HandoffPurpose::Register => {
            let label = match req.label {
                Some(l) if valid_device_name(&l) => l,
                Some(_) => return Err(ApiError::bad_request()),
                None => DEFAULT_LABEL.to_owned(),
            };
            if keys >= dbw::MAX_CREDENTIALS {
                return Err(devices::quota("webauthn"));
            }
            start(&st, &s, Purpose::Register, Some(label))
        }
        HandoffPurpose::Confirm => {
            if req.label.is_some() || keys == 0 {
                return Err(ApiError::bad_request());
            }
            start(&st, &s, Purpose::Confirm, None)
        }
    }
}

fn state_reply(st: &AppState, s: &Authed, body: &[u8], login: bool) -> Result<Reply, ApiError> {
    let req: HandoffStateRequest = parse_body(body, SMALL_BODY)?;
    let (state, purpose) = st
        .handoffs
        .state(&req.handoff_id.0, &s.token_hash, st.clock.now_ms())
        .ok_or_else(ApiError::not_found)?;
    if (purpose == Purpose::Login) != login {
        return Err(ApiError::not_found());
    }
    Reply::ok(&HandoffStateResponse {
        state: match state {
            HState::Waiting => HandoffState::Waiting,
            HState::Done => HandoffState::Done,
            HState::Failed => HandoffState::Failed,
            HState::Expired => HandoffState::Expired,
        },
    })
}

pub async fn login_state(
    State(st): State<AppState>,
    AnySession(s): AnySession,
    body: Bytes,
) -> Result<Reply, ApiError> {
    state_reply(&st, &s, &body, true)
}

pub async fn account_state(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    state_reply(&st, &s, &body, false)
}

pub async fn list(State(st): State<AppState>, Session(s): Session) -> Result<Reply, ApiError> {
    let rows = st.db.run(|c| dbw::list(c, &s.account_id))?;
    let out: Vec<WebauthnEntry> = rows
        .into_iter()
        .map(|r| WebauthnEntry {
            credential_ref: B64Fixed(dbw::credential_ref(&r.credential_id)),
            label: r.label,
            created_at: r.created_at,
            last_used_at: r.last_used_at,
        })
        .collect();
    Reply::ok(&out)
}

pub async fn remove(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: RemoveWebauthnRequest = parse_body(&body, SMALL_BODY)?;
    s.require_fresh()?;
    super::account::require_second_factor(
        &st,
        &s,
        req.code
            .as_ref()
            .map(mosaic_sync::wire::json::SecretText::expose_secret),
    )?;
    let now = st.clock.now_ms();
    let removed = st.db.run_tx(|tx| {
        let removed = dbw::remove(tx, &s.account_id, &req.credential_ref.0)?;
        if removed {
            audit::record(
                tx,
                &s.account_id,
                now,
                audit::Kind::WebauthnRemoved,
                Some(&s.device_id),
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

/// Claims one attempt of the per-account failure budget; `rate_limited` when
/// it is empty. A claim is refunded at most once (the refund spends it), and
/// only after the check it guards succeeded: `options` refunds its own claim
/// after the right code, `finish` hands its own claim to the transaction that
/// stores the credential.
fn claim(st: &AppState, account: &[u8; 16], now: u64) -> Result<SlotClaim, ApiError> {
    st.db
        .run(|c| accounts::claim_second_factor_slot(c, account, now))?
        .map_err(ApiError::rate_limited)
}

/// The page asks for the options with the typed code.
pub async fn options(
    State(st): State<AppState>,
    Gated(..): Gated<HandoffGate>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: HandoffOptionsRequest = parse_body(&body, SMALL_BODY)?;
    let rp = rp(&st)?;
    let now = st.clock.now_ms();
    let id = req.handoff_id.0;
    let account = st
        .handoffs
        .account_of(&id, now)
        .ok_or_else(handoff_expired)?;
    // A wrong code counts against the account before it is looked at.
    let claim = claim(&st, &account, now)?;
    let v = st
        .handoffs
        .verify_code(&id, req.code.expose_secret().as_bytes(), now)
        .map_err(refused)?;
    st.db
        .run(|c| accounts::refund_totp_slot(c, &account, claim))?;
    let (state, public_key) = build_options(&st, rp, &v)?;
    if !st.handoffs.set_state(&id, state, now) {
        return Err(handoff_expired());
    }
    Reply::ok(&HandoffOptionsResponse {
        purpose: match v.purpose {
            Purpose::Login => HandoffKind::Login,
            Purpose::Register => HandoffKind::Register,
            Purpose::Confirm => HandoffKind::Confirm,
        },
        device_name: v.device_name,
        requested_at: v.created,
        public_key,
    })
}

fn build_options(
    st: &AppState,
    rp: &Rp,
    v: &Verified,
) -> Result<(CeremonyState, serde_json::Value), ApiError> {
    let account = v.account_id;
    let invalid = |_: RpError| two_factor_invalid();
    match v.purpose {
        Purpose::Register => {
            let (handle, rows, email) = st.db.run_tx(|tx| {
                let handle = dbw::user_handle(tx, &account)?;
                let rows = dbw::list(tx, &account)?;
                let a = accounts::by_id(tx, &account)?.ok_or_else(ApiError::unauthorized)?;
                Ok((handle, rows, a.email))
            })?;
            rp.registration_options(&handle, &email, &stored(&rows))
                .map_err(invalid)
        }
        Purpose::Login | Purpose::Confirm => {
            let (handle, rows) = st.db.run(|c| {
                Ok((
                    dbw::existing_user_handle(c, &account)?,
                    dbw::list(c, &account)?,
                ))
            })?;
            let handle = handle.ok_or_else(two_factor_invalid)?;
            rp.assertion_options(&handle, &stored(&rows))
                .map_err(invalid)
        }
    }
}

/// What the finishing transaction decided.
enum Finished {
    Done,
    /// The session that started the handoff is gone: refused as expired.
    SessionGone,
    /// The counter did not rise: the audit event is written, the request is
    /// refused.
    Regressed,
}

/// The page posts the browser's credential JSON.
pub async fn finish(
    State(st): State<AppState>,
    Gated(..): Gated<HandoffGate>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: HandoffFinishRequest = parse_body(&body, FINISH_BODY)?;
    let rp = rp(&st)?;
    let now = st.clock.now_ms();
    let id = req.handoff_id.0;
    let account = st
        .handoffs
        .account_of(&id, now)
        .ok_or_else(handoff_expired)?;
    let claim = claim(&st, &account, now)?;
    // The state is taken out first: a second finish finds none.
    let taken = st.handoffs.take_for_finish(&id, now).map_err(refused)?;
    let credential = serde_json::to_vec(&req.credential).map_err(|_| ApiError::bad_request())?;
    let outcome = match taken.rp_state {
        CeremonyState::Registration(ref state) => {
            finish_register(&st, rp, &taken, state, &credential, now, claim)
        }
        CeremonyState::Assertion(ref state) => {
            finish_assert(&st, rp, &taken, state, &credential, now, claim)
        }
    };
    match outcome {
        Ok(Finished::Done) => {
            st.handoffs.complete(&id);
            Reply::ok(&HandoffFinishResponse { ok: true })
        }
        Ok(Finished::SessionGone) => Err(handoff_expired()),
        Ok(Finished::Regressed) | Err(_) => Err(two_factor_invalid()),
    }
}

fn finish_register(
    st: &AppState,
    rp: &Rp,
    taken: &Taken,
    state: &passkey_auth::RegistrationState,
    credential: &[u8],
    now: u64,
    claim: SlotClaim,
) -> Result<Finished, ApiError> {
    let new = rp
        .finish_registration(state, credential)
        .map_err(|_| two_factor_invalid())?;
    let label = taken
        .label
        .clone()
        .unwrap_or_else(|| DEFAULT_LABEL.to_owned());
    st.db.run_tx(|tx| {
        // The session that asked must still exist (a removed device, a reset,
        // a password change ends it, and with it the handoff).
        if !sessions::has_session(tx, &taken.token_hash, &taken.account_id, false)? {
            return Ok(Finished::SessionGone);
        }
        match dbw::insert(
            tx,
            &taken.account_id,
            &new.id,
            &new.public_key,
            new.sign_count,
            &label,
            now,
        )? {
            Ok(()) => {}
            Err(InsertError::Limit | InsertError::Duplicate) => return Err(two_factor_invalid()),
        }
        accounts::refund_totp_slot(tx, &taken.account_id, claim)?;
        audit::record(
            tx,
            &taken.account_id,
            now,
            audit::Kind::WebauthnAdded,
            Some(&taken.device_id),
        )?;
        Ok(Finished::Done)
    })
}

fn finish_assert(
    st: &AppState,
    rp: &Rp,
    taken: &Taken,
    state: &passkey_auth::AuthenticationState,
    credential: &[u8],
    now: u64,
    claim: SlotClaim,
) -> Result<Finished, ApiError> {
    let rows = st.db.run(|c| dbw::list(c, &taken.account_id))?;
    let outcome = match rp.finish_assertion(state, credential, &stored(&rows)) {
        Ok(o) => o,
        Err(RpError::CounterReplay) => {
            record_regression(st, taken, now)?;
            return Ok(Finished::Regressed);
        }
        Err(_) => return Err(two_factor_invalid()),
    };
    let idle = st.limits.session_idle_days.saturating_mul(86_400_000);
    st.db.run_tx(|tx| {
        // The session that asked must still exist; nothing is stored if not.
        let pending = taken.purpose == Purpose::Login;
        if !sessions::has_session(tx, &taken.token_hash, &taken.account_id, pending)? {
            return Ok(Finished::SessionGone);
        }
        // Compare-and-set: a concurrent finish that stored an equal or higher
        // count first makes this one a replay.
        if !dbw::store_count(
            tx,
            &taken.account_id,
            &outcome.credential_id,
            outcome.new_count,
            now,
        )? {
            audit::record(
                tx,
                &taken.account_id,
                now,
                audit::Kind::WebauthnCounterRegressed,
                Some(&taken.device_id),
            )?;
            return Ok(Finished::Regressed);
        }
        match taken.purpose {
            Purpose::Login => {
                // As `/login/totp`: the session stops being pending.
                if !sessions::complete_2fa(tx, &taken.token_hash, now.saturating_add(idle))? {
                    return Ok(Finished::SessionGone);
                }
                audit::record(
                    tx,
                    &taken.account_id,
                    now,
                    audit::Kind::Login,
                    Some(&taken.device_id),
                )?;
            }
            Purpose::Confirm => {
                if !sessions::set_second_factor_until(
                    tx,
                    &taken.token_hash,
                    &taken.account_id,
                    now.saturating_add(CONFIRM_MS),
                )? {
                    return Ok(Finished::SessionGone);
                }
            }
            Purpose::Register => return Err(two_factor_invalid()),
        }
        accounts::refund_totp_slot(tx, &taken.account_id, claim)?;
        Ok(Finished::Done)
    })
}

fn record_regression(st: &AppState, taken: &Taken, now: u64) -> Result<(), ApiError> {
    st.db.run_tx(|tx| {
        audit::record(
            tx,
            &taken.account_id,
            now,
            audit::Kind::WebauthnCounterRegressed,
            Some(&taken.device_id),
        )
    })
}
