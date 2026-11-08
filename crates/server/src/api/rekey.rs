//! `/v1/account/rekey/{start,finish}`: the atomic full re-key (M7 spec §4.2,
//! CRYPTO_SPEC §6.6). Both steps need a fresh login. The handlers parse and
//! check the shape of what the client uploads; they never open a wrapping or
//! a seal.

use axum::body::Bytes;
use axum::extract::State;
use mosaic_core::share::ShareSeal;
use mosaic_core::{Argon2Profile, KeyGeneration, VaultRosterCiphertext, WrappedKey};
use mosaic_sync::wire::json::{B64, RegistrationStartResponse};
use mosaic_sync::wire::opaque::{
    Suite,
    ke::{RegistrationRequest, RegistrationUpload, ServerRegistration},
};

use super::sharing::notify;
use super::wire::{RekeyFinishRequest, RekeyFinishResponse, RekeyStartRequest};
use super::{Reply, parse_body};
use crate::AppState;
use crate::auth::Session;
use crate::db::rekey as dbr;
use crate::error::ApiError;

/// Most bytes of a `finish` body.
const MAX_FINISH_BODY: usize = 4 * 1024 * 1024;
/// Most vaults of one account (`MAX_VAULTS_PER_ACCOUNT`).
const MAX_VAULTS: usize = 1000;
/// Most entries of a seal list of one vault (members and invites are capped
/// at 50 each).
const MAX_LIST: usize = 128;

/// The checked lists of one owned vault.
struct Parsed {
    roster: Option<(Vec<u8>, u32)>,
    seals: Vec<([u8; 16], Vec<u8>)>,
    invite_seals: Vec<([u8; 16], Vec<u8>)>,
}

fn wrapped_key(bytes: &[u8; 74]) -> Result<(), ApiError> {
    WrappedKey::from_bytes(bytes)
        .map(|_| ())
        .map_err(|_| ApiError::bad_request())
}

fn seal(bytes: &[u8]) -> Result<(), ApiError> {
    ShareSeal::from_bytes(bytes)
        .map(|_| ())
        .map_err(|_| ApiError::bad_request())
}

pub async fn start(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: RekeyStartRequest = parse_body(&body, 16 * 1024)?;
    s.require_fresh()?;
    st.db
        .run(|c| dbr::check_start(c, &s.account_id, s.account_version, req.ak_epoch))?;
    let request = RegistrationRequest::<Suite>::deserialize(&req.registration_request.0)
        .map_err(|_| ApiError::bad_request())?;
    let started = ServerRegistration::<Suite>::start(st.setup(), request, &s.account_id)
        .map_err(|_| ApiError::bad_request())?;
    Reply::ok(&RegistrationStartResponse {
        registration_response: B64(started.message.serialize().to_vec()),
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "parsing the request lists, then one call"
)]
pub async fn finish(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let max =
        MAX_FINISH_BODY.min(usize::try_from(st.limits.max_request_bytes).unwrap_or(usize::MAX));
    let req: RekeyFinishRequest = parse_body(&body, max)?;
    s.require_fresh()?;
    if req.vaults.len() > MAX_VAULTS
        || req.member_rewraps.len() > MAX_VAULTS
        || req.emergency_seals.len() > MAX_VAULTS * 20
    {
        return Err(ApiError::too_large());
    }
    if req.device.0 != s.device_id {
        return Err(ApiError::forbidden());
    }
    let profile = Argon2Profile::from_id(req.profile).map_err(|_| ApiError::bad_request())?;
    wrapped_key(&req.wrapped_ak.0)?;
    if let Some(id) = &req.identity {
        wrapped_key(&id.wrapped_isk.0)?;
    }
    let upload = RegistrationUpload::<Suite>::deserialize(&req.registration_upload.0)
        .map_err(|_| ApiError::bad_request())?;
    let record = ServerRegistration::<Suite>::finish(upload)
        .serialize()
        .to_vec();

    // Shape checks per entry; the lists are owned here so the database layer
    // borrows plain slices.
    let mut parsed = Vec::with_capacity(req.vaults.len());
    for v in &req.vaults {
        if v.seals.len() > MAX_LIST || v.invite_seals.len() > MAX_LIST {
            return Err(ApiError::too_large());
        }
        wrapped_key(&v.wrapped.0)?;
        if KeyGeneration::new(v.generation).is_err() {
            return Err(ApiError::bad_request());
        }
        if u64::from(v.generation) > st.limits.max_key_generations {
            return Err(crate::db::devices::quota("keyGenerations"));
        }
        let roster = match &v.roster {
            Some(r) => {
                let ct =
                    VaultRosterCiphertext::from_bytes(&r.0).map_err(|_| ApiError::bad_request())?;
                Some((r.0.clone(), ct.key_generation().get()))
            }
            None => None,
        };
        let mut seals = Vec::with_capacity(v.seals.len());
        for r in &v.seals {
            seal(&r.seal.0)?;
            seals.push((r.recipient.0, r.seal.0.to_vec()));
        }
        let mut invite_seals = Vec::with_capacity(v.invite_seals.len());
        for r in &v.invite_seals {
            seal(&r.seal.0)?;
            invite_seals.push((r.invite_id.0, r.seal.0.to_vec()));
        }
        parsed.push(Parsed {
            roster,
            seals,
            invite_seals,
        });
    }
    let vaults: Vec<dbr::OwnedVault<'_>> = req
        .vaults
        .iter()
        .zip(&parsed)
        .map(|(v, p)| dbr::OwnedVault {
            vault: v.vault_id.0,
            generation: v.generation,
            wrapped: &v.wrapped.0,
            expected_head: v.expected_head,
            roster: p.roster.as_ref().map(|(b, g)| (b.as_slice(), *g)),
            seals: &p.seals,
            invite_seals: &p.invite_seals,
        })
        .collect();
    let mut rewraps = Vec::with_capacity(req.member_rewraps.len());
    for m in &req.member_rewraps {
        wrapped_key(&m.wrapped.0)?;
        rewraps.push(dbr::MemberRewrap {
            vault: m.vault_id.0,
            generation: m.generation,
            wrapped: &m.wrapped.0,
        });
    }
    let mut emergency = Vec::with_capacity(req.emergency_seals.len());
    for e in &req.emergency_seals {
        seal(&e.seal.0)?;
        emergency.push(dbr::EmergencySeal {
            grantee: e.grantee.0,
            vault: e.vault_id.0,
            generation: e.generation,
            seal: e.seal.0.to_vec(),
        });
    }

    let now = st.clock.now_ms();
    let done = st.db.run_tx(|tx| {
        dbr::apply(
            tx,
            &s.account_id,
            &dbr::Rekey {
                record: &record,
                wrapped_ak: &req.wrapped_ak.0,
                salt: &req.salt.0,
                profile: profile.id(),
                epoch: req.ak_epoch,
                identity: req.identity.as_ref().map(|i| dbr::Identity {
                    ipk: &i.ipk.0,
                    wrapped_isk: &i.wrapped_isk.0,
                }),
                vaults: &vaults,
                member_rewraps: &rewraps,
                emergency_seals: &emergency,
                device: &s.device_id,
                session_version: s.account_version,
                keep_session: &s.token_hash,
            },
            now,
        )
    })?;
    // Logins that started against the old password end with it.
    st.logins.drop_account(&s.account_id);
    notify(&st, done.notify, now);
    Reply::ok(&RekeyFinishResponse {
        account_version: done.account_version,
        ak_epoch: done.epoch,
    })
}
