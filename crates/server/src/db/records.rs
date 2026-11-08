//! Record upload with the per-record compare-and-swap (CRYPTO_SPEC §10.2).

use rusqlite::{OptionalExtension, Transaction, params};

use super::{accounts, devices, int, vaults};
use crate::error::ApiError;

/// A record in an upload.
pub struct Incoming<'a> {
    pub record_id: [u8; 16],
    /// 1 item, 2 tombstone.
    pub kind: u8,
    /// The key generation in the ciphertext header.
    pub header_generation: u32,
    pub ciphertext: &'a [u8],
    pub base_seq: Option<u64>,
}

/// The result of one record.
pub enum Outcome {
    Accepted(u64),
    Conflict(u64),
}

/// Applies a batch in the caller's transaction. Every record must be sealed
/// under the vault's current generation (checked by the caller against the
/// ciphertext headers; `generation` is what the request declared):
/// otherwise the whole batch fails with `generation_mismatch`.
#[expect(
    clippy::too_many_arguments,
    reason = "one transaction-scoped helper with the quota limits as plain numbers"
)]
pub fn push(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    vault: &[u8; 16],
    generation: u32,
    incoming: &[Incoming<'_>],
    max_records: u64,
    max_bytes: u64,
    now: u64,
) -> Result<Vec<Outcome>, ApiError> {
    let v = vaults::access_role(tx, account, vault, vaults::role::WRITE)?;
    if generation != v.current_generation {
        return Err(vaults::generation_mismatch(v.current_generation));
    }
    if incoming
        .iter()
        .any(|r| r.header_generation != v.current_generation)
    {
        return Err(vaults::generation_mismatch(v.current_generation));
    }
    // Storage is charged to the vault's owner.
    let usage = accounts::usage(tx, &v.owner)?;
    let mut count = usage.records;
    let mut bytes = usage.record_bytes;
    let mut head = v.head_seq;
    let mut out = Vec::with_capacity(incoming.len());
    for rec in incoming {
        let stored: Option<(i64, i64)> = tx
            .query_row(
                "SELECT server_seq, length(ciphertext) FROM records WHERE vault_id = ?1 AND record_id = ?2",
                params![vault.as_slice(), rec.record_id.as_slice()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let stored =
            stored.map(|(s, l)| (u64::try_from(s).unwrap_or(0), u64::try_from(l).unwrap_or(0)));
        let accepted = match (rec.base_seq, stored) {
            (None, None) => true,
            (Some(b), Some((s, _))) => b == s,
            _ => false,
        };
        if !accepted {
            out.push(Outcome::Conflict(stored.map_or(0, |(s, _)| s)));
            continue;
        }
        let new_len = u64::try_from(rec.ciphertext.len()).unwrap_or(u64::MAX);
        match stored {
            None => count = count.saturating_add(1),
            Some((_, old_len)) => bytes = bytes.saturating_sub(old_len),
        }
        bytes = bytes.saturating_add(new_len);
        if count > max_records {
            return Err(devices::quota("recordCount"));
        }
        if bytes > max_bytes {
            return Err(devices::quota("recordBytes"));
        }
        head = head.saturating_add(1);
        tx.execute(
            "INSERT INTO records (vault_id, record_id, kind, key_generation, ciphertext, server_seq, uploaded_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT (vault_id, record_id) DO UPDATE SET kind = excluded.kind, \
             key_generation = excluded.key_generation, ciphertext = excluded.ciphertext, \
             server_seq = excluded.server_seq, uploaded_at = excluded.uploaded_at",
            params![
                vault.as_slice(),
                rec.record_id.as_slice(),
                rec.kind,
                generation,
                rec.ciphertext,
                int(head),
                int(now)
            ],
        )?;
        out.push(Outcome::Accepted(head));
    }
    if head != v.head_seq {
        tx.execute(
            "UPDATE vaults SET head_seq = ?1 WHERE vault_id = ?2",
            params![int(head), vault.as_slice()],
        )?;
    }
    Ok(out)
}
