//! Emergency access routes (M6b spec §4, §5.2). The server keeps the clock:
//! the grantee's `release` works only after the waiting period or the
//! grantor's approval, and a refusal starts a 24 h cooldown.

use axum::body::Bytes;
use axum::extract::{Path, State};
use mosaic_core::share::ShareSeal;
use mosaic_sync::wire::json::B64Fixed;

use super::sharing::{notify, notify_vault};
use super::wire::{
    EmergencyResponse, GrantRequest, GrantedEntry, Id, ReceivedEntry, ReleaseAt, ReleaseResponse,
    SealsRequest, VaultGeneration, VaultSeal, VaultSealOut,
};
use super::{Reply, parse_body, parse_uuid};
use crate::AppState;
use crate::auth::{Authed, Session};
use crate::db::emergency::{self as dbe, NewGrant, Seal};
use crate::error::ApiError;
use crate::ratelimit::{self, Scope};

fn limit(st: &AppState) -> usize {
    usize::try_from(st.limits.max_request_bytes).unwrap_or(usize::MAX)
}

fn seals(list: &[VaultSeal]) -> Result<Vec<Seal>, ApiError> {
    list.iter()
        .map(|s| {
            ShareSeal::from_bytes(&s.seal.0).map_err(|_| ApiError::bad_request())?;
            if s.generation == 0 {
                return Err(ApiError::bad_request());
            }
            Ok(Seal {
                vault_id: s.vault_id.0,
                generation: s.generation,
                blob: s.seal.0.to_vec(),
            })
        })
        .collect()
}

pub async fn overview(State(st): State<AppState>, Session(s): Session) -> Result<Reply, ApiError> {
    let (given, received) = st.db.run(|c| dbe::overview(c, &s.account_id))?;
    Reply::ok(&EmergencyResponse {
        granted: given
            .into_iter()
            .map(|g| GrantedEntry {
                grantee: Id(g.grantee),
                email: g.email,
                wait_days: g.wait_days,
                scope: g.scope,
                vaults: g
                    .vaults
                    .into_iter()
                    .map(|(v, generation)| VaultGeneration {
                        vault_id: Id(v),
                        generation,
                    })
                    .collect(),
                state: g.state,
                requested_at: g.requested_at,
                release_at: g.release_at,
            })
            .collect(),
        received: received
            .into_iter()
            .map(|g| ReceivedEntry {
                grantor: Id(g.grantor),
                email: g.email,
                wait_days: g.wait_days,
                state: g.state,
                requested_at: g.requested_at,
                release_at: g.release_at,
                cooldown_until: g.cooldown_until,
            })
            .collect(),
    })
}

pub async fn put_grant(
    State(st): State<AppState>,
    Session(s): Session,
    Path(grantee): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let grantee = parse_uuid(&grantee)?;
    let req: GrantRequest = parse_body(&body, limit(&st))?;
    s.require_fresh()?;
    if !matches!(req.scope, 1 | 2) {
        return Err(ApiError::bad_request());
    }
    let list = seals(&req.seals)?;
    let now = st.clock.now_ms();
    st.db.run_tx(|tx| {
        dbe::put_grant(
            tx,
            &s.account_id,
            &grantee,
            &NewGrant {
                wait_days: req.wait_days,
                scope: req.scope,
                seals: &list,
            },
            now,
        )
    })?;
    notify(&st, [s.account_id, grantee], now);
    Ok(Reply::no_content())
}

pub async fn put_seals(
    State(st): State<AppState>,
    Session(s): Session,
    Path(grantee): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let grantee = parse_uuid(&grantee)?;
    let req: SealsRequest = parse_body(&body, limit(&st))?;
    let list = seals(&req.seals)?;
    st.db
        .run_tx(|tx| dbe::put_seals(tx, &s.account_id, &grantee, &list))?;
    Ok(Reply::no_content())
}

pub async fn delete_grant(
    State(st): State<AppState>,
    Session(s): Session,
    Path(grantee): Path<String>,
) -> Result<Reply, ApiError> {
    let grantee = parse_uuid(&grantee)?;
    st.db
        .run_tx(|tx| dbe::delete_grant(tx, &s.account_id, &grantee))?;
    notify(&st, [s.account_id, grantee], st.clock.now_ms());
    Ok(Reply::no_content())
}

pub async fn request(
    State(st): State<AppState>,
    Session(s): Session,
    Path(grantor): Path<String>,
) -> Result<Reply, ApiError> {
    let grantor = parse_uuid(&grantor)?;
    let now = st.clock.now_ms();
    st.rate
        .check(
            now,
            Scope::EmergencyRequest,
            &s.account_id,
            ratelimit::EMERGENCY_REQUEST,
        )
        .map_err(ApiError::rate_limited)?;
    let release_at = st
        .db
        .run_tx(|tx| dbe::request(tx, &s.account_id, &grantor, now))?;
    notify(&st, [s.account_id, grantor], now);
    Reply::ok(&ReleaseAt { release_at })
}

fn decide(st: &AppState, s: &Authed, grantee: &str, approve: bool) -> Result<Reply, ApiError> {
    let grantee = parse_uuid(grantee)?;
    if approve {
        s.require_fresh()?;
    }
    let now = st.clock.now_ms();
    st.db
        .run_tx(|tx| dbe::decide(tx, &s.account_id, &grantee, approve, now))?;
    notify(st, [s.account_id, grantee], now);
    Ok(Reply::no_content())
}

pub async fn decline(
    State(st): State<AppState>,
    Session(s): Session,
    Path(grantee): Path<String>,
) -> Result<Reply, ApiError> {
    decide(&st, &s, &grantee, false)
}

pub async fn approve(
    State(st): State<AppState>,
    Session(s): Session,
    Path(grantee): Path<String>,
) -> Result<Reply, ApiError> {
    decide(&st, &s, &grantee, true)
}

pub async fn release(
    State(st): State<AppState>,
    Session(s): Session,
    Path(grantor): Path<String>,
) -> Result<Reply, ApiError> {
    let grantor = parse_uuid(&grantor)?;
    let now = st.clock.now_ms();
    let released = st
        .db
        .run_tx(|tx| dbe::release(tx, &s.account_id, &grantor, now))?;
    notify(&st, [s.account_id, released.grantor], now);
    for seal in &released.seals {
        notify_vault(&st, &seal.vault_id, now);
    }
    let seals = released
        .seals
        .into_iter()
        .map(|x| {
            Ok(VaultSealOut {
                vault_id: Id(x.vault_id),
                generation: x.generation,
                seal: B64Fixed(
                    <[u8; ShareSeal::LEN]>::try_from(x.blob).map_err(|_| ApiError::internal())?,
                ),
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Reply::ok(&ReleaseResponse { seals })
}
