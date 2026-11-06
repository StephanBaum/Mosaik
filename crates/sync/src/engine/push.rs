//! The push phase (M6 spec §5.4): records, with every item of every vault
//! before any tombstone, and then the queued vault metadata.

use std::collections::BTreeSet;

use mosaic_core::{KeyGeneration, VaultId, VaultMetaCiphertext};
use mosaic_vault::session::{Error, UnlockedSession};
use mosaic_vault::storage::{OutboxSeq, RecordBody, StoredRecord};

use super::consts::{MAX_CONFLICT_ROUNDS, PUSH_FETCH, PUSH_MAX_BYTES, PUSH_MAX_RECORDS};
use super::pull::RunStats;
use super::{SessionAccess, SyncEngine, SyncError, SyncReport};
use crate::transport::{PushRecord, PushResult, Transport, TransportError};

/// The records of one vault waiting for upload, at its current generation.
struct Batch {
    vault: VaultId,
    generation: KeyGeneration,
    records: Vec<StoredRecord>,
}

fn collect_pending(
    s: &UnlockedSession,
    only: Option<VaultId>,
) -> Result<(Vec<Batch>, Vec<Batch>), Error> {
    let vaults = only.map_or_else(|| s.synced_vaults(), |v| vec![v]);
    let (mut items, mut tombs) = (Vec::new(), Vec::new());
    for vault in vaults {
        let Some(generation) = s
            .vaults()
            .into_iter()
            .find(|v| v.vault_id == vault)
            .map(|v| v.key_generation)
        else {
            continue;
        };
        let mut item_records = Vec::new();
        let mut tomb_records = Vec::new();
        // A corrupt outbox row is skipped; so is a record sealed under an
        // older generation than the vault's (it waits for its re-seal).
        for record in s
            .pending_uploads_in(vault, PUSH_FETCH)?
            .into_iter()
            .flatten()
        {
            if record.body.key_generation() != generation || record.outbox_seq.is_none() {
                continue;
            }
            match record.body {
                RecordBody::Item(_) => item_records.push(record),
                RecordBody::Tombstone(_) => tomb_records.push(record),
            }
        }
        for (list, records) in [(&mut items, item_records), (&mut tombs, tomb_records)] {
            if !records.is_empty() {
                list.push(Batch {
                    vault,
                    generation,
                    records,
                });
            }
        }
    }
    Ok((items, tombs))
}

/// Splits records into requests of at most `PUSH_MAX_RECORDS` records and
/// `PUSH_MAX_BYTES` bytes.
fn requests(records: Vec<StoredRecord>) -> Vec<Vec<StoredRecord>> {
    let mut out = Vec::new();
    let mut current: Vec<StoredRecord> = Vec::new();
    let mut bytes = 0usize;
    for record in records {
        let len = record.body.as_bytes().len();
        if !current.is_empty()
            && (current.len() >= PUSH_MAX_RECORDS || bytes.saturating_add(len) > PUSH_MAX_BYTES)
        {
            out.push(core::mem::take(&mut current));
            bytes = 0;
        }
        bytes = bytes.saturating_add(len);
        current.push(record);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

impl<T: Transport> SyncEngine<T> {
    /// Step 5. `only` limits it to one vault (after a rotation).
    pub(super) fn push_phase(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
        stats: &mut RunStats,
        only: Option<VaultId>,
    ) -> Result<(), SyncError> {
        let mut conflict_rounds = 0;
        for _ in 0..64 {
            let (items, tombs) = access.with(|s| collect_pending(s, only))??;
            if items.is_empty() && tombs.is_empty() {
                return Ok(());
            }
            let mut conflicted = BTreeSet::new();
            let mut sent = 0u32;
            for batch in items.into_iter().chain(tombs) {
                sent += self.push_batch(access, batch, &mut conflicted)?;
            }
            report.pushed += sent;
            if !conflicted.is_empty() {
                conflict_rounds += 1;
                if conflict_rounds > MAX_CONFLICT_ROUNDS {
                    return Ok(());
                }
                for vault in conflicted {
                    self.pull_vault(access, vault, report, stats)?;
                }
            } else if sent == 0 {
                return Ok(());
            }
        }
        Ok(())
    }

    /// Pushes one vault's records; returns how many the backend accepted.
    fn push_batch(
        &mut self,
        access: &mut impl SessionAccess,
        batch: Batch,
        conflicted: &mut BTreeSet<VaultId>,
    ) -> Result<u32, SyncError> {
        let Batch {
            vault,
            generation,
            records,
        } = batch;
        let mut sent = 0u32;
        for request in requests(records) {
            let push: Vec<PushRecord<'_>> = request
                .iter()
                .map(|r| PushRecord {
                    record_id: r.record_id,
                    body: &r.body,
                    base_seq: r.server_seq,
                })
                .collect();
            let results = match self.transport.push_records(vault, generation, &push) {
                Ok(results) => results,
                Err(TransportError::GenerationMismatch { .. }) => {
                    conflicted.insert(vault);
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            let mut accepted: Vec<(&StoredRecord, Option<u64>)> = Vec::new();
            for (record, result) in request.iter().zip(results) {
                match result {
                    PushResult::Accepted { seq } => accepted.push((record, seq)),
                    PushResult::Conflict { .. } => {
                        conflicted.insert(vault);
                    }
                }
            }
            sent += mark_accepted(access, vault, &accepted)?;
        }
        Ok(sent)
    }

    /// Step 6: sends the metadata that is queued.
    pub(super) fn push_metas(
        &mut self,
        access: &mut impl SessionAccess,
        stats: &mut RunStats,
        report: &mut SyncReport,
    ) -> Result<(), SyncError> {
        for _ in 0..=MAX_CONFLICT_ROUNDS {
            let uploads = access.with(UnlockedSession::pending_vault_uploads)??;
            let metas: Vec<(VaultId, VaultMetaCiphertext, Option<u64>)> = uploads
                .into_iter()
                .filter_map(|u| u.meta.map(|(m, base)| (u.vault_id, m, base)))
                .collect();
            if metas.is_empty() {
                return Ok(());
            }
            let mut conflicted = Vec::new();
            for (vault, meta, base) in metas {
                match self.transport.push_meta(vault, &meta, base)? {
                    PushResult::Accepted { seq } => accept_meta(access, vault, &meta, seq)?,
                    PushResult::Conflict { .. } => conflicted.push(vault),
                }
            }
            if conflicted.is_empty() {
                return Ok(());
            }
            for vault in conflicted {
                self.pull_vault(access, vault, report, stats)?;
            }
        }
        Ok(())
    }
}

/// Marks the metadata uploaded under the backend's sequence number or a
/// device-local one.
fn accept_meta(
    access: &mut impl SessionAccess,
    vault: VaultId,
    meta: &VaultMetaCiphertext,
    seq: Option<u64>,
) -> Result<(), SyncError> {
    access.with(|s| -> Result<(), Error> {
        let seq = match seq {
            Some(q) => q,
            None => s.allocate_local_seqs(vault, 1)?,
        };
        s.mark_vault_meta_uploaded(vault, meta, seq)?;
        Ok(())
    })??;
    Ok(())
}

/// Marks the accepted records uploaded under a server sequence number (the
/// backend's, or a device-local one from the store).
fn mark_accepted(
    access: &mut impl SessionAccess,
    vault: VaultId,
    accepted: &[(&StoredRecord, Option<u64>)],
) -> Result<u32, SyncError> {
    if accepted.is_empty() {
        return Ok(0);
    }
    let local = accepted.iter().filter(|(_, q)| q.is_none()).count();
    let local = u32::try_from(local).map_err(|_| SyncError::Locked)?;
    let marked = access.with(|s| -> Result<u32, Error> {
        let mut next = if local == 0 {
            0
        } else {
            s.allocate_local_seqs(vault, local)?
        };
        let mut n = 0u32;
        for (record, seq) in accepted {
            let Some(outbox) = record.outbox_seq else {
                continue;
            };
            let seq = if let Some(q) = seq {
                *q
            } else {
                let q = next;
                next += 1;
                q
            };
            mark(s, vault, record, outbox, seq)?;
            n += 1;
        }
        Ok(n)
    })??;
    Ok(marked)
}

fn mark(
    s: &mut UnlockedSession,
    vault: VaultId,
    record: &StoredRecord,
    outbox: OutboxSeq,
    seq: u64,
) -> Result<(), Error> {
    s.mark_uploaded(vault, record.record_id, outbox, seq)?;
    Ok(())
}
