//! `GET /v1/vaults`, `PUT /v1/vaults/{id}` and the change feed.

use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::{InstallId, ItemId, KeyGeneration, VaultId, VaultMetaCiphertext, WrappedKey};
use mosaic_sync::wire::json::{
    B64, ChangeKey, ChangeMeta, ChangeRecord, ChangesResponse, CreateVaultRequest, HeadResponse,
    RecordKind, VaultEntry, VaultsResponse,
};
use serde::Deserialize;

use super::{Reply, parse_body, parse_uuid};
use crate::AppState;
use crate::auth::Session;
use crate::db::sharing as dbs;
use crate::db::vaults::{self as dbv, Created, NewVault};
use crate::error::ApiError;

/// Most records in one page of changes.
const MAX_PAGE: usize = 500;

fn generation(g: u32) -> Result<KeyGeneration, ApiError> {
    KeyGeneration::new(g).map_err(|_| ApiError::internal())
}

pub async fn list(State(st): State<AppState>, Session(s): Session) -> Result<Reply, ApiError> {
    let rows = st.db.run(|c| dbv::list(c, &s.account_id))?;
    let vaults = rows
        .into_iter()
        .map(|(id, g, head)| {
            Ok(VaultEntry {
                vault_id: VaultId::from_bytes(id).map_err(|_| ApiError::internal())?,
                generation: generation(g)?,
                head_seq: head,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Reply::ok(&VaultsResponse { vaults })
}

pub async fn create(
    State(st): State<AppState>,
    Session(s): Session,
    Path(vault_id): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let req: CreateVaultRequest = parse_body(
        &body,
        usize::try_from(st.limits.max_request_bytes).unwrap_or(usize::MAX),
    )?;
    let meta = VaultMetaCiphertext::from_bytes(&req.meta.0).map_err(|_| ApiError::bad_request())?;
    if meta.key_generation() != req.generation {
        return Err(ApiError::bad_request());
    }
    // A vault is created by the device that signed in.
    if *req.device_id.as_bytes() != s.device_id {
        return Err(ApiError::forbidden());
    }
    if u64::from(req.generation.get()) > st.limits.max_key_generations {
        return Err(crate::db::devices::quota("keyGenerations"));
    }
    let now = st.clock.now_ms();
    let created = st.db.run_tx(|tx| {
        dbv::create(
            tx,
            &s.account_id,
            &NewVault {
                vault: &vault,
                generation: req.generation.get(),
                wrapped: req.wrapped_key.as_bytes(),
                device: req.device_id.as_bytes(),
                meta: &req.meta.0,
            },
            now,
        )
    })?;
    match created {
        Created::New(head_seq) => {
            st.polls.bump(s.account_id, now);
            Reply::created(&HeadResponse { head_seq })
        }
        Created::Existing(head_seq) => Reply::ok(&HeadResponse { head_seq }),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangesQuery {
    since: Option<u64>,
    limit: Option<usize>,
    /// `1`: the sharing-aware page (M6b) adds `role`, `shares` and `rosters`.
    /// Without it the page is the M6 page, which older clients parse strictly.
    sharing: Option<u8>,
    /// `1` with `sharing=1`: the owner's page adds `rotationRequested` (M7).
    /// A flag of its own, for the same reason.
    rotation: Option<u8>,
}

pub async fn changes(
    State(st): State<AppState>,
    Session(s): Session,
    Path(vault_id): Path<String>,
    query: Result<Query<ChangesQuery>, QueryRejection>,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let Query(q) = query.map_err(|_| ApiError::bad_request())?;
    let limit = q.limit.unwrap_or(MAX_PAGE);
    if limit == 0 || limit > MAX_PAGE {
        return Err(ApiError::bad_request());
    }
    let since = q.since.unwrap_or(0);
    let (page, extras) = st.db.run(|c| {
        let page = dbv::changes(c, &s.account_id, &vault, since, limit)?;
        let extras = if q.sharing == Some(1) {
            Some(dbs::changes_extras(c, &s.account_id, &vault, since)?)
        } else {
            None
        };
        Ok((page, extras))
    })?;
    let next_since = if page.more {
        page.records.last().map_or(page.head_seq, |r| r.server_seq)
    } else {
        page.head_seq
    };
    let keys = page
        .keys
        .into_iter()
        .map(|k| {
            Ok(ChangeKey {
                generation: generation(k.generation)?,
                wrapped_key: WrappedKey::from_bytes(&k.wrapped)
                    .map_err(|_| ApiError::internal())?,
                device_id: InstallId::from_bytes(k.device_id).map_err(|_| ApiError::internal())?,
                server_seq: k.server_seq,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    let records = page
        .records
        .into_iter()
        .map(|r| {
            Ok(ChangeRecord {
                record_id: ItemId::from_bytes(r.record_id).map_err(|_| ApiError::internal())?,
                kind: if r.kind == 1 {
                    RecordKind::Item
                } else {
                    RecordKind::Tombstone
                },
                ciphertext: B64(r.ciphertext),
                server_seq: r.server_seq,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    let response = ChangesResponse {
        head_seq: page.head_seq,
        generation: generation(page.generation)?,
        keys,
        meta: page.meta.map(|(ciphertext, server_seq)| ChangeMeta {
            ciphertext: B64(ciphertext),
            server_seq,
        }),
        records,
        more: page.more,
        next_since,
    };
    let Some(extras) = extras else {
        return Reply::json(StatusCode::OK, &response);
    };
    let rotation_requested = if q.rotation == Some(1) && extras.role == dbv::role::OWNER {
        Some(
            st.db
                .run(|c| crate::db::rekey::rotation_requested(c, &vault))?,
        )
    } else {
        None
    };
    let value = with_sharing(&response, &extras, rotation_requested)?;
    Reply::json(StatusCode::OK, &value)
}

/// The M6 page plus the sharing part: the caller's role, the unclaimed shares
/// addressed to them, the rosters newer than `since` and, for the owner,
/// whether a member asked for a rotation.
fn with_sharing(
    response: &ChangesResponse,
    extras: &dbs::Extras,
    rotation_requested: Option<bool>,
) -> Result<serde_json::Value, ApiError> {
    let mut value = serde_json::to_value(response).map_err(|_| ApiError::internal())?;
    let shares: Vec<serde_json::Value> = extras
        .shares
        .iter()
        .map(|sh| {
            serde_json::json!({
                "generation": sh.generation,
                "sender": crate::db::uuid_text(&sh.sender),
                "seal": Base64UrlUnpadded::encode_string(&sh.seal),
                "seq": sh.server_seq,
            })
        })
        .collect();
    let rosters: Vec<serde_json::Value> = extras
        .rosters
        .iter()
        .map(|r| {
            serde_json::json!({
                "generation": r.generation,
                "ciphertext": Base64UrlUnpadded::encode_string(&r.ciphertext),
                "seq": r.server_seq,
            })
        })
        .collect();
    if let Some(map) = value.as_object_mut() {
        map.insert("role".to_owned(), extras.role.into());
        map.insert("shares".to_owned(), shares.into());
        map.insert("rosters".to_owned(), rosters.into());
        if let Some(asked) = rotation_requested {
            // A member re-keyed: the owner rotates (M7 spec §3.7).
            map.insert("rotationRequested".to_owned(), asked.into());
        }
    }
    Ok(value)
}
