//! Identity keys, the contact book, invites, members, rosters, shared-vault
//! rotation, claims, leaving and blocks (M6b spec §5.2). The handlers parse
//! and check the shape of what the clients upload; they never open a seal or
//! a ciphertext.

use axum::body::Bytes;
use axum::extract::{Path, State};
use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::share::ShareSeal;
use mosaic_core::{ContactsCiphertext, KeyGeneration, VaultRosterCiphertext, WrappedKey};
use mosaic_sync::wire::json::{B64, B64Fixed};

use super::wire::{
    AcceptRequest, AcceptResponse, ClaimRequest, ContactsResponse, DeclineRequest, IdentityRequest,
    IncomingInvite, InviteEntry, InviteRequest, InviteSent, InviteVault, InvitesResponse,
    MemberEntry, MembersResponse, OutgoingInvite, PutContactsRequest, PutRosterRequest,
    RoleRequest, RotateRequest, SeqResponse,
};
use super::{Reply, codes, parse_body, parse_uuid};
use crate::AppState;
use crate::auth::Session;
use crate::db::sharing::{self as dbs, Identity};
use crate::db::vaults as dbv;
use crate::error::ApiError;
use crate::ratelimit::{self, Scope};

/// Wakes every member of `vault` (their change token moves).
pub fn notify_vault(st: &AppState, vault: &[u8; 16], now: u64) {
    if let Ok(ids) = st.db.run(|c| dbv::member_ids(c, vault)) {
        notify(st, ids, now);
    }
}

/// Wakes the given accounts.
pub fn notify(st: &AppState, accounts: impl IntoIterator<Item = [u8; 16]>, now: u64) {
    for a in accounts {
        st.polls.bump(a, now);
    }
}

fn limit(st: &AppState) -> usize {
    usize::try_from(st.limits.max_request_bytes).unwrap_or(usize::MAX)
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

fn b64(bytes: &[u8]) -> String {
    Base64UrlUnpadded::encode_string(bytes)
}

// ---- identity -----------------------------------------------------------

pub async fn put_identity(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: IdentityRequest = parse_body(&body, 4096)?;
    wrapped_key(&req.wrapped_isk.0)?;
    if req.ipk.0 == [0u8; 32] {
        return Err(ApiError::bad_request());
    }
    let now = st.clock.now_ms();
    let r = st
        .db
        .run_tx(|tx| dbs::set_identity(tx, &s.account_id, &req.ipk.0, &req.wrapped_isk.0, now))?;
    match r {
        Identity::Set | Identity::Same => Ok(Reply::no_content()),
        Identity::Exists(ipk, wrapped) => Err(ApiError::conflict(codes::IDENTITY_EXISTS)
            .with_detail(&serde_json::json!({ "ipk": b64(&ipk), "wrappedIsk": b64(&wrapped) }))),
    }
}

// ---- contact book -------------------------------------------------------

pub async fn get_contacts(
    State(st): State<AppState>,
    Session(s): Session,
) -> Result<Reply, ApiError> {
    let (ciphertext, seq) = st.db.run(|c| dbs::contacts(c, &s.account_id))?;
    Reply::ok(&ContactsResponse {
        ciphertext: ciphertext.map(B64),
        seq,
    })
}

pub async fn put_contacts(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: PutContactsRequest = parse_body(&body, limit(&st))?;
    ContactsCiphertext::from_bytes(&req.ciphertext.0).map_err(|_| ApiError::bad_request())?;
    let r = st
        .db
        .run_tx(|tx| dbs::put_contacts(tx, &s.account_id, &req.ciphertext.0, req.base_seq))?;
    match r {
        Ok(seq) => {
            // The account's other devices.
            st.polls.bump(s.account_id, st.clock.now_ms());
            Reply::ok(&SeqResponse { seq })
        }
        Err(current) => Err(dbs::conflict(current)),
    }
}

// ---- invites ------------------------------------------------------------

pub async fn create_invite(
    State(st): State<AppState>,
    Session(s): Session,
    Path(vault_id): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let now = st.clock.now_ms();
    st.rate
        .check(
            now,
            Scope::InviteAccount,
            &s.account_id,
            ratelimit::INVITE_ACCOUNT,
        )
        .map_err(ApiError::rate_limited)?;
    let req: InviteRequest = parse_body(&body, 4096)?;
    seal(&req.seal.0)?;
    if KeyGeneration::new(req.generation).is_err() {
        return Err(ApiError::bad_request());
    }
    let sent = st.db.run_tx(|tx| {
        dbs::create_invite(
            tx,
            &s.account_id,
            &vault,
            &dbs::NewInvite {
                invite_id: &req.invite_id.0,
                recipient: &req.recipient.0,
                role: req.role,
                generation: req.generation,
                seal: &req.seal.0,
            },
            now,
        )
    })?;
    if sent.notify_recipient {
        st.polls.bump(req.recipient.0, now);
    }
    notify_vault(&st, &vault, now);
    Reply::created(&InviteSent {
        expires_at: sent.expires_at,
    })
}

pub async fn list_invites(
    State(st): State<AppState>,
    Session(s): Session,
) -> Result<Reply, ApiError> {
    let now = st.clock.now_ms();
    let (incoming, outgoing) = st
        .db
        .run_tx(|tx| dbs::list_invites(tx, &s.account_id, now))?;
    let incoming = incoming
        .into_iter()
        .map(|i| {
            Ok(IncomingInvite {
                invite_id: super::wire::Id(i.invite_id),
                vault_id: super::wire::Id(i.vault_id),
                sender: super::wire::Id(i.sender),
                sender_email: i.sender_email,
                role: i.role,
                generation: i.generation,
                seal: B64Fixed(
                    <[u8; ShareSeal::LEN]>::try_from(i.seal).map_err(|_| ApiError::internal())?,
                ),
                expires_at: i.expires_at,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    let outgoing = outgoing
        .into_iter()
        .map(|o| OutgoingInvite {
            invite_id: super::wire::Id(o.invite_id),
            vault_id: super::wire::Id(o.vault_id),
            recipient: super::wire::Id(o.recipient),
            role: o.role,
            state: o.state,
            delivered: o.delivered,
            expires_at: o.expires_at,
        })
        .collect();
    Reply::ok(&InvitesResponse { incoming, outgoing })
}

pub async fn invite_vault(
    State(st): State<AppState>,
    Session(s): Session,
    Path(invite_id): Path<String>,
) -> Result<Reply, ApiError> {
    let id = parse_uuid(&invite_id)?;
    let now = st.clock.now_ms();
    let (meta, roster) = st
        .db
        .run(|c| dbs::invite_vault(c, &s.account_id, &id, now))?;
    Reply::ok(&InviteVault {
        meta: B64(meta),
        roster: roster.map(B64),
    })
}

pub async fn accept_invite(
    State(st): State<AppState>,
    Session(s): Session,
    Path(invite_id): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let id = parse_uuid(&invite_id)?;
    let req: AcceptRequest = parse_body(&body, 4096)?;
    wrapped_key(&req.wrapped.0)?;
    if req.device.0 != s.device_id {
        return Err(ApiError::forbidden());
    }
    let now = st.clock.now_ms();
    let a = st.db.run_tx(|tx| {
        dbs::accept_invite(tx, &s.account_id, &id, &req.wrapped.0, &s.device_id, now)
    })?;
    notify_vault(&st, &a.vault, now);
    Reply::ok(&AcceptResponse {
        vault_id: super::wire::Id(a.vault),
        generation: a.generation,
        role: a.role,
    })
}

pub async fn decline_invite(
    State(st): State<AppState>,
    Session(s): Session,
    Path(invite_id): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let id = parse_uuid(&invite_id)?;
    let req: DeclineRequest = parse_body(&body, 1024)?;
    let sender = st
        .db
        .run_tx(|tx| dbs::decline_invite(tx, &s.account_id, &id, req.block))?;
    st.polls.bump(sender, st.clock.now_ms());
    Ok(Reply::no_content())
}

pub async fn cancel_invite(
    State(st): State<AppState>,
    Session(s): Session,
    Path(invite_id): Path<String>,
) -> Result<Reply, ApiError> {
    let id = parse_uuid(&invite_id)?;
    let recipient = st
        .db
        .run_tx(|tx| dbs::cancel_invite(tx, &s.account_id, &id))?;
    let now = st.clock.now_ms();
    notify(&st, [recipient, s.account_id], now);
    Ok(Reply::no_content())
}

// ---- members, roles, roster ---------------------------------------------

pub async fn members(
    State(st): State<AppState>,
    Session(s): Session,
    Path(vault_id): Path<String>,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let now = st.clock.now_ms();
    let (members, invites) = st.db.run(|c| dbs::members(c, &s.account_id, &vault, now))?;
    Reply::ok(&MembersResponse {
        members: members
            .into_iter()
            .map(|m| MemberEntry {
                account_id: super::wire::Id(m.account_id),
                email: m.email,
                role: m.role,
                origin: m.origin,
                added_at: m.added_at,
            })
            .collect(),
        invites: invites
            .into_iter()
            .map(|i| InviteEntry {
                invite_id: super::wire::Id(i.invite_id),
                recipient: super::wire::Id(i.recipient),
                role: i.role,
                delivered: i.delivered,
            })
            .collect(),
    })
}

pub async fn set_role(
    State(st): State<AppState>,
    Session(s): Session,
    Path((vault_id, account_id)): Path<(String, String)>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let target = parse_uuid(&account_id)?;
    let req: RoleRequest = parse_body(&body, 1024)?;
    st.db
        .run_tx(|tx| dbs::set_role(tx, &s.account_id, &vault, &target, req.role))?;
    notify_vault(&st, &vault, st.clock.now_ms());
    Ok(Reply::no_content())
}

pub async fn put_roster(
    State(st): State<AppState>,
    Session(s): Session,
    Path(vault_id): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let req: PutRosterRequest = parse_body(&body, limit(&st))?;
    let ct = VaultRosterCiphertext::from_bytes(&req.ciphertext.0)
        .map_err(|_| ApiError::bad_request())?;
    if ct.key_generation().get() != req.generation {
        return Err(ApiError::bad_request());
    }
    let seq = st.db.run_tx(|tx| {
        dbs::put_roster(
            tx,
            &s.account_id,
            &vault,
            req.generation,
            &req.ciphertext.0,
            req.base_seq,
        )
    })?;
    notify_vault(&st, &vault, st.clock.now_ms());
    Reply::ok(&SeqResponse { seq })
}

// ---- rotation, claims, leaving -------------------------------------------

/// Most entries of a rotation's lists (members and invites are capped at 50
/// each).
const MAX_LIST: usize = 128;

pub async fn rotate(
    State(st): State<AppState>,
    Session(s): Session,
    Path(vault_id): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let req: RotateRequest = parse_body(&body, limit(&st))?;
    if req.seals.len() > MAX_LIST
        || req.invite_seals.len() > MAX_LIST
        || req.revoke.len() > MAX_LIST
    {
        return Err(ApiError::too_large());
    }
    wrapped_key(&req.wrapped.0)?;
    if req.device.0 != s.device_id {
        return Err(ApiError::forbidden());
    }
    if KeyGeneration::new(req.generation).is_err() {
        return Err(ApiError::bad_request());
    }
    let roster =
        VaultRosterCiphertext::from_bytes(&req.roster.0).map_err(|_| ApiError::bad_request())?;
    if u64::from(req.generation) > st.limits.max_key_generations {
        return Err(crate::db::devices::quota("keyGenerations"));
    }
    let mut seals = Vec::with_capacity(req.seals.len());
    for r in &req.seals {
        seal(&r.seal.0)?;
        seals.push((r.recipient.0, r.seal.0.to_vec()));
    }
    let mut invite_seals = Vec::with_capacity(req.invite_seals.len());
    for r in &req.invite_seals {
        seal(&r.seal.0)?;
        invite_seals.push((r.invite_id.0, r.seal.0.to_vec()));
    }
    let revoke: Vec<[u8; 16]> = req.revoke.iter().map(|i| i.0).collect();
    let now = st.clock.now_ms();
    let rotated = st.db.run_tx(|tx| {
        dbs::rotate(
            tx,
            &s.account_id,
            &vault,
            &dbs::Rotation {
                generation: req.generation,
                wrapped: &req.wrapped.0,
                device: &s.device_id,
                expected_head: req.expected_head,
                roster: &req.roster.0,
                roster_generation: roster.key_generation().get(),
                seals: &seals,
                invite_seals: &invite_seals,
                revoke: &revoke,
            },
            now,
        )
    })?;
    notify(&st, rotated.accounts, now);
    Reply::ok(&SeqResponse { seq: rotated.seq })
}

pub async fn claim(
    State(st): State<AppState>,
    Session(s): Session,
    Path((vault_id, generation)): Path<(String, String)>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let generation: u32 = generation.parse().map_err(|_| ApiError::bad_request())?;
    if generation == 0 {
        return Err(ApiError::bad_request());
    }
    let req: ClaimRequest = parse_body(&body, 4096)?;
    wrapped_key(&req.wrapped.0)?;
    if req.device.0 != s.device_id {
        return Err(ApiError::forbidden());
    }
    let now = st.clock.now_ms();
    let seq = st.db.run_tx(|tx| {
        dbs::claim(
            tx,
            &s.account_id,
            &vault,
            generation,
            &req.wrapped.0,
            &s.device_id,
            now,
        )
    })?;
    // The account's other devices receive the wrapped key as a key row.
    st.polls.bump(s.account_id, now);
    Reply::ok(&SeqResponse { seq })
}

pub async fn leave(
    State(st): State<AppState>,
    Session(s): Session,
    Path(vault_id): Path<String>,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let now = st.clock.now_ms();
    let others = st
        .db
        .run_tx(|tx| dbs::leave(tx, &s.account_id, &vault, now))?;
    notify(&st, others, now);
    Ok(Reply::no_content())
}

// ---- blocks -------------------------------------------------------------

pub async fn block(
    State(st): State<AppState>,
    Session(s): Session,
    Path(account_id): Path<String>,
) -> Result<Reply, ApiError> {
    let other = parse_uuid(&account_id)?;
    st.db.run(|c| dbs::block(c, &s.account_id, &other))?;
    Ok(Reply::no_content())
}

pub async fn unblock(
    State(st): State<AppState>,
    Session(s): Session,
    Path(account_id): Path<String>,
) -> Result<Reply, ApiError> {
    let other = parse_uuid(&account_id)?;
    st.db.run(|c| dbs::unblock(c, &s.account_id, &other))?;
    Ok(Reply::no_content())
}
