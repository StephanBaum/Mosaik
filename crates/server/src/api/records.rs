//! `POST /v1/vaults/{v}/records`, `PUT /v1/vaults/{v}/meta` and
//! `PUT /v1/vaults/{v}/keys/{generation}`: the write rules of CRYPTO_SPEC
//! §10.2. The server checks the shape of each ciphertext (it cannot read it)
//! and the compare-and-swap conditions.

use std::collections::HashSet;

use axum::body::Bytes;
use axum::extract::{Path, State};
use mosaic_core::{ItemCiphertext, TombstoneCiphertext, VaultMetaCiphertext};
use mosaic_sync::wire::json::{
    MAX_CIPHERTEXT_LEN, MetaResult, PushMetaRequest, PushRecordsRequest, PushRecordsResponse,
    PutKeyRequest, PutKeyResponse, RecordKind, RecordResult,
};

use super::sharing::notify_vault;
use super::{Reply, parse_body, parse_uuid};
use crate::AppState;
use crate::auth::Session;
use crate::db::records::{self, Incoming, Outcome};
use crate::db::vaults::{self as dbv, Cas};
use crate::error::ApiError;

/// Records in one upload.
const MAX_RECORDS: usize = 256;
/// Ciphertext bytes in one upload.
const MAX_BATCH_BYTES: usize = 8 * 1024 * 1024;

fn body_limit(st: &AppState) -> usize {
    usize::try_from(st.limits.max_request_bytes).unwrap_or(usize::MAX)
}

pub async fn push(
    State(st): State<AppState>,
    Session(s): Session,
    Path(vault_id): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let req: PushRecordsRequest = parse_body(&body, body_limit(&st))?;
    if req.records.len() > MAX_RECORDS {
        return Err(ApiError::too_large());
    }
    let total: usize = req.records.iter().map(|r| r.ciphertext.0.len()).sum();
    if total > MAX_BATCH_BYTES {
        return Err(ApiError::too_large());
    }
    let mut seen = HashSet::with_capacity(req.records.len());
    let mut incoming = Vec::with_capacity(req.records.len());
    for r in &req.records {
        if !seen.insert(*r.record_id.as_bytes()) || r.ciphertext.0.len() > MAX_CIPHERTEXT_LEN {
            return Err(ApiError::bad_request());
        }
        // Structure only: the header names the key generation.
        let (kind, header_generation) = match r.kind {
            RecordKind::Item => (
                1u8,
                ItemCiphertext::from_bytes(&r.ciphertext.0)
                    .map_err(|_| ApiError::bad_request())?
                    .key_generation()
                    .get(),
            ),
            RecordKind::Tombstone => (
                2u8,
                TombstoneCiphertext::from_bytes(&r.ciphertext.0)
                    .map_err(|_| ApiError::bad_request())?
                    .key_generation()
                    .get(),
            ),
        };
        incoming.push(Incoming {
            record_id: *r.record_id.as_bytes(),
            kind,
            header_generation,
            ciphertext: &r.ciphertext.0,
            base_seq: r.base_seq,
        });
    }
    let now = st.clock.now_ms();
    let outcomes = st.db.run_tx(|tx| {
        records::push(
            tx,
            &s.account_id,
            &vault,
            req.generation.get(),
            &incoming,
            st.limits.account_record_count,
            st.limits.account_record_bytes,
            now,
        )
    })?;
    if outcomes.iter().any(|o| matches!(o, Outcome::Accepted(_))) {
        notify_vault(&st, &vault, now);
    }
    let results = req
        .records
        .iter()
        .zip(outcomes)
        .map(|(r, o)| match o {
            Outcome::Accepted(server_seq) => RecordResult::Accepted {
                record_id: r.record_id,
                server_seq,
            },
            Outcome::Conflict(current_seq) => RecordResult::Conflict {
                record_id: r.record_id,
                current_seq,
            },
        })
        .collect();
    Reply::ok(&PushRecordsResponse { results })
}

pub async fn put_meta(
    State(st): State<AppState>,
    Session(s): Session,
    Path(vault_id): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let req: PushMetaRequest = parse_body(&body, body_limit(&st))?;
    let meta =
        VaultMetaCiphertext::from_bytes(&req.ciphertext.0).map_err(|_| ApiError::bad_request())?;
    let now = st.clock.now_ms();
    let cas = st.db.run_tx(|tx| {
        dbv::put_meta(
            tx,
            &s.account_id,
            &vault,
            &req.ciphertext.0,
            meta.key_generation().get(),
            req.base_seq,
        )
    })?;
    match cas {
        Cas::Accepted(server_seq) => {
            notify_vault(&st, &vault, now);
            Reply::ok(&MetaResult::Accepted { server_seq })
        }
        Cas::Conflict(current_seq) => Reply::ok(&MetaResult::Conflict { current_seq }),
    }
}

pub async fn put_key(
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
    let req: PutKeyRequest = parse_body(&body, 4096)?;
    // A key is uploaded by the device that signed in (the canonical winner of
    // two rotations is decided on this ID, CRYPTO_SPEC §10.1).
    if *req.device_id.as_bytes() != s.device_id {
        return Err(ApiError::forbidden());
    }
    if u64::from(generation) > st.limits.max_key_generations {
        return Err(crate::db::devices::quota("keyGenerations"));
    }
    let now = st.clock.now_ms();
    let server_seq = st.db.run_tx(|tx| {
        dbv::put_key(
            tx,
            &s.account_id,
            &vault,
            generation,
            req.wrapped_key.as_bytes(),
            req.device_id.as_bytes(),
            req.expected_head_seq,
            now,
        )
    })?;
    notify_vault(&st, &vault, now);
    Reply::ok(&PutKeyResponse { server_seq })
}
