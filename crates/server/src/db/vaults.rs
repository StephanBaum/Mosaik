//! Vaults, their keys and metadata, and the change feed (M6 spec §6.2).

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::{accounts, devices, id16, int, u64_col};
use crate::config::MAX_VAULTS_PER_ACCOUNT;
use crate::error::ApiError;
use mosaic_sync::wire::json::{GenerationDetail, HeadMovedDetail, codes};

/// A vault the caller may use.
pub struct VaultRow {
    pub current_generation: u32,
    pub head_seq: u64,
}

fn generation_col(r: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<u32> {
    let v: i64 = r.get(idx)?;
    u32::try_from(v).map_err(|_| rusqlite::Error::InvalidQuery)
}

/// The vault, if `account` is a member: `not_found` when there is no such
/// vault, `forbidden` when the account is not a member.
pub fn access(
    conn: &Connection,
    account: &[u8; 16],
    vault: &[u8; 16],
) -> Result<VaultRow, ApiError> {
    let row = conn
        .query_row(
            "SELECT v.current_generation, v.head_seq, \
             EXISTS (SELECT 1 FROM memberships m WHERE m.vault_id = v.vault_id AND m.account_id = ?2) \
             FROM vaults v WHERE v.vault_id = ?1",
            params![vault.as_slice(), account.as_slice()],
            |r| {
                Ok((
                    VaultRow {
                        current_generation: generation_col(r, 0)?,
                        head_seq: u64_col(r, 1)?,
                    },
                    r.get::<_, i64>(2)? != 0,
                ))
            },
        )
        .optional()?;
    match row {
        None => Err(ApiError::not_found()),
        Some((_, false)) => Err(ApiError::forbidden()),
        Some((v, true)) => Ok(v),
    }
}

/// The vaults the account is a member of: `(vault, generation, head_seq)`.
pub fn list(conn: &Connection, account: &[u8; 16]) -> Result<Vec<([u8; 16], u32, u64)>, ApiError> {
    let mut stmt = conn.prepare(
        "SELECT v.vault_id, v.current_generation, v.head_seq FROM vaults v \
         JOIN memberships m ON m.vault_id = v.vault_id WHERE m.account_id = ?1 ORDER BY v.vault_id",
    )?;
    let rows = stmt.query_map([account.as_slice()], |r| {
        Ok((id16(r, 0)?, generation_col(r, 1)?, u64_col(r, 2)?))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// What a vault creation carries.
pub struct NewVault<'a> {
    pub vault: &'a [u8; 16],
    pub generation: u32,
    pub wrapped: &'a [u8; 74],
    pub device: &'a [u8; 16],
    pub meta: &'a [u8],
}

/// Whether `create` made a vault.
pub enum Created {
    /// A new vault; its head sequence.
    New(u64),
    /// The caller already had it; its head sequence.
    Existing(u64),
}

/// Creates a vault for `account` (idempotent for a member). `vault_exists`
/// if another account owns the ID.
pub fn create(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    v: &NewVault<'_>,
    now: u64,
) -> Result<Created, ApiError> {
    let existing: Option<(i64, bool)> = tx
        .query_row(
            "SELECT v.head_seq, EXISTS (SELECT 1 FROM memberships m WHERE m.vault_id = v.vault_id \
             AND m.account_id = ?2) FROM vaults v WHERE v.vault_id = ?1",
            params![v.vault.as_slice(), account.as_slice()],
            |r| Ok((r.get(0)?, r.get::<_, i64>(1)? != 0)),
        )
        .optional()?;
    match existing {
        Some((head, true)) => return Ok(Created::Existing(u64::try_from(head).unwrap_or(0))),
        Some((_, false)) => return Err(ApiError::conflict(codes::VAULT_EXISTS)),
        None => {}
    }
    let usage = accounts::usage(tx, account)?;
    if usage.vaults >= MAX_VAULTS_PER_ACCOUNT {
        return Err(devices::quota("vaults"));
    }
    // Key at sequence 1, metadata at sequence 2.
    tx.execute(
        "INSERT INTO vaults (vault_id, owner_account_id, current_generation, head_seq, created_at) \
         VALUES (?1, ?2, ?3, 2, ?4)",
        params![
            v.vault.as_slice(),
            account.as_slice(),
            v.generation,
            int(now)
        ],
    )?;
    tx.execute(
        "INSERT INTO memberships (account_id, vault_id, role) VALUES (?1, ?2, 1)",
        params![account.as_slice(), v.vault.as_slice()],
    )?;
    tx.execute(
        "INSERT INTO vault_keys (vault_id, account_id, generation, wrapped, device_id, server_seq, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
        params![
            v.vault.as_slice(),
            account.as_slice(),
            v.generation,
            v.wrapped.as_slice(),
            v.device.as_slice(),
            int(now)
        ],
    )?;
    tx.execute(
        "INSERT INTO vault_meta (vault_id, key_generation, ciphertext, server_seq) VALUES (?1, ?2, ?3, 2)",
        params![v.vault.as_slice(), v.generation, v.meta],
    )?;
    Ok(Created::New(2))
}

/// A page of the change feed.
pub struct Changes {
    pub head_seq: u64,
    pub generation: u32,
    pub keys: Vec<ChangeKey>,
    pub meta: Option<(Vec<u8>, u64)>,
    pub records: Vec<ChangeRecord>,
    pub more: bool,
}

pub struct ChangeKey {
    pub generation: u32,
    pub wrapped: Vec<u8>,
    pub device_id: [u8; 16],
    pub server_seq: u64,
}

pub struct ChangeRecord {
    pub record_id: [u8; 16],
    pub kind: u8,
    pub ciphertext: Vec<u8>,
    pub server_seq: u64,
}

/// The most ciphertext bytes in one page (so a page stays far below the
/// client's 16 MiB response cap); a page always holds at least one record.
const PAGE_BYTES: usize = 8 * 1024 * 1024;

/// Reads everything after `since`: all keys and the metadata if newer, and up
/// to `limit` records in sequence order.
pub fn changes(
    conn: &Connection,
    account: &[u8; 16],
    vault: &[u8; 16],
    since: u64,
    limit: usize,
) -> Result<Changes, ApiError> {
    let v = access(conn, account, vault)?;
    let mut stmt = conn.prepare(
        "SELECT generation, wrapped, device_id, server_seq FROM vault_keys \
         WHERE vault_id = ?1 AND account_id = ?2 AND server_seq > ?3 ORDER BY generation",
    )?;
    let keys = stmt
        .query_map(
            params![vault.as_slice(), account.as_slice(), int(since)],
            |r| {
                Ok(ChangeKey {
                    generation: generation_col(r, 0)?,
                    wrapped: r.get(1)?,
                    device_id: id16(r, 2)?,
                    server_seq: u64_col(r, 3)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    let meta = conn
        .query_row(
            "SELECT ciphertext, server_seq FROM vault_meta WHERE vault_id = ?1 AND server_seq > ?2",
            params![vault.as_slice(), int(since)],
            |r| Ok((r.get::<_, Vec<u8>>(0)?, u64_col(r, 1)?)),
        )
        .optional()?;
    let mut stmt = conn.prepare(
        "SELECT record_id, kind, ciphertext, server_seq FROM records \
         WHERE vault_id = ?1 AND server_seq > ?2 ORDER BY server_seq LIMIT ?3",
    )?;
    let fetched = stmt
        .query_map(
            params![
                vault.as_slice(),
                int(since),
                i64::try_from(limit.saturating_add(1)).unwrap_or(i64::MAX)
            ],
            |r| {
                Ok(ChangeRecord {
                    record_id: id16(r, 0)?,
                    kind: r.get(1)?,
                    ciphertext: r.get(2)?,
                    server_seq: u64_col(r, 3)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let mut more = fetched.len() > limit;
    let mut records = Vec::with_capacity(fetched.len().min(limit));
    let mut bytes = 0usize;
    for rec in fetched.into_iter().take(limit) {
        bytes = bytes.saturating_add(rec.ciphertext.len());
        if !records.is_empty() && bytes > PAGE_BYTES {
            more = true;
            break;
        }
        records.push(rec);
    }
    Ok(Changes {
        head_seq: v.head_seq,
        generation: v.current_generation,
        keys,
        meta,
        records,
        more,
    })
}

/// Outcome of a metadata or key upload.
pub enum Cas {
    /// Accepted at this sequence.
    Accepted(u64),
    /// The stored sequence differs.
    Conflict(u64),
}

/// Replaces the vault metadata if the stored sequence equals `base_seq`.
/// `generation_mismatch` if the metadata is not sealed under the vault's
/// current generation.
pub fn put_meta(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    vault: &[u8; 16],
    ciphertext: &[u8],
    meta_generation: u32,
    base_seq: Option<u64>,
) -> Result<Cas, ApiError> {
    let v = access(tx, account, vault)?;
    if meta_generation != v.current_generation {
        return Err(generation_mismatch(v.current_generation));
    }
    let stored: Option<i64> = tx
        .query_row(
            "SELECT server_seq FROM vault_meta WHERE vault_id = ?1",
            [vault.as_slice()],
            |r| r.get(0),
        )
        .optional()?;
    let stored = stored.and_then(|s| u64::try_from(s).ok());
    let ok = match (base_seq, stored) {
        (None, None) => true,
        (Some(b), Some(s)) => b == s,
        _ => false,
    };
    if !ok {
        return Ok(Cas::Conflict(stored.unwrap_or(0)));
    }
    let seq = v.head_seq.saturating_add(1);
    tx.execute(
        "INSERT INTO vault_meta (vault_id, key_generation, ciphertext, server_seq) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT (vault_id) DO UPDATE SET key_generation = excluded.key_generation, \
         ciphertext = excluded.ciphertext, server_seq = excluded.server_seq",
        params![vault.as_slice(), meta_generation, ciphertext, int(seq)],
    )?;
    tx.execute(
        "UPDATE vaults SET head_seq = ?1 WHERE vault_id = ?2",
        params![int(seq), vault.as_slice()],
    )?;
    Ok(Cas::Accepted(seq))
}

/// `generation_mismatch` with the current generation.
pub fn generation_mismatch(current: u32) -> ApiError {
    ApiError::conflict(codes::GENERATION_MISMATCH).with_detail(&GenerationDetail {
        current_generation: current,
    })
}

/// Stores the key of the next generation (ADR-0013, CRYPTO_SPEC §10.2): the
/// generation must be `current + 1` and, if `expected_head` is given, the
/// vault head must equal it.
#[expect(
    clippy::too_many_arguments,
    reason = "one transaction-scoped helper; the arguments are the request fields"
)]
pub fn put_key(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    vault: &[u8; 16],
    generation: u32,
    wrapped: &[u8; 74],
    device: &[u8; 16],
    expected_head: Option<u64>,
    now: u64,
) -> Result<u64, ApiError> {
    let v = access(tx, account, vault)?;
    let exists: i64 = tx.query_row(
        "SELECT COUNT(*) FROM vault_keys WHERE vault_id = ?1 AND account_id = ?2 AND generation = ?3",
        params![vault.as_slice(), account.as_slice(), generation],
        |r| r.get(0),
    )?;
    if exists > 0 {
        return Err(
            ApiError::conflict(codes::GENERATION_TAKEN).with_detail(&GenerationDetail {
                current_generation: v.current_generation,
            }),
        );
    }
    if generation != v.current_generation.saturating_add(1) {
        return Err(generation_mismatch(v.current_generation));
    }
    if expected_head.is_some_and(|h| h != v.head_seq) {
        return Err(
            ApiError::conflict(codes::HEAD_MOVED).with_detail(&HeadMovedDetail {
                head_seq: v.head_seq,
            }),
        );
    }
    let seq = v.head_seq.saturating_add(1);
    tx.execute(
        "INSERT INTO vault_keys (vault_id, account_id, generation, wrapped, device_id, server_seq, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            vault.as_slice(),
            account.as_slice(),
            generation,
            wrapped.as_slice(),
            device.as_slice(),
            int(seq),
            int(now)
        ],
    )?;
    tx.execute(
        "UPDATE vaults SET current_generation = ?1, head_seq = ?2 WHERE vault_id = ?3",
        params![generation, int(seq), vault.as_slice()],
    )?;
    Ok(seq)
}
