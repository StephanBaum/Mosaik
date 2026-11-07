//! The push phase (M6 spec §5.4): records, with every item of every vault
//! before any tombstone, and then the queued vault metadata.
//!
//! On the hosted backend a push is based on the served view (CRYPTO_SPEC
//! §10): the `server_seq` under which the server last served or accepted the
//! record. A conflict that the following pull does not explain (the record
//! it brings is not newer than what the device already holds) makes the
//! device read the vault again from the start, once per run; a record that
//! is still refused after that is the server contradicting itself, and the
//! device reports it and keeps the change queued.

use std::collections::BTreeSet;

use mosaic_core::{ItemId, KeyGeneration, VaultId, VaultMetaCiphertext};
use mosaic_vault::session::{Error, UnlockedSession};
use mosaic_vault::storage::{OutboxSeq, RecordBody, StoredRecord, VaultFlag};
use sha2::{Digest, Sha256};

use super::consts::{MAX_CONFLICT_ROUNDS, PUSH_FETCH, PUSH_MAX_BYTES, PUSH_MAX_RECORDS};
use super::pull::RunStats;
use super::{SessionAccess, SyncEngine, SyncError, SyncReport};
use crate::transport::{PushRecord, PushResult, Transport, TransportError, TransportKind};

/// A record waiting for upload and the version it is based on.
struct Pending {
    record: StoredRecord,
    base: Option<u64>,
}

/// The records of one vault waiting for upload, at its current generation.
struct Batch {
    vault: VaultId,
    generation: KeyGeneration,
    records: Vec<Pending>,
}

/// What a round of pushes was refused for.
#[derive(Default)]
struct Conflicts {
    vaults: BTreeSet<VaultId>,
    /// Record, and the base it was pushed with.
    records: Vec<(VaultId, ItemId, Option<u64>)>,
}

fn collect_pending(
    s: &UnlockedSession,
    only: Option<VaultId>,
    hosted: bool,
    skip: &BTreeSet<VaultId>,
) -> Result<(Vec<Batch>, Vec<Batch>), Error> {
    let vaults = only.map_or_else(|| s.synced_vaults(), |v| vec![v]);
    let (mut items, mut tombs) = (Vec::new(), Vec::new());
    for vault in vaults {
        if skip.contains(&vault) {
            continue;
        }
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
            let base = if hosted {
                s.served_seq(vault, record.record_id)?
            } else {
                record.server_seq
            };
            match record.body {
                RecordBody::Item(_) => item_records.push(Pending { record, base }),
                RecordBody::Tombstone(_) => tomb_records.push(Pending { record, base }),
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
fn requests(records: Vec<Pending>) -> Vec<Vec<Pending>> {
    let mut out = Vec::new();
    let mut current: Vec<Pending> = Vec::new();
    let mut bytes = 0usize;
    for record in records {
        let len = record.record.body.as_bytes().len();
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
        let hosted = self.config.transport_kind == TransportKind::Hosted;
        let mut conflict_rounds = 0;
        // Vaults read again from the start in this phase, and vaults whose
        // records the server keeps refusing (left queued for the next run).
        let mut rebuilt = BTreeSet::new();
        let mut stuck = BTreeSet::new();
        for _ in 0..64 {
            let (items, tombs) = access.with(|s| collect_pending(s, only, hosted, &stuck))??;
            if items.is_empty() && tombs.is_empty() {
                return Ok(());
            }
            let mut conflicts = Conflicts::default();
            let mut sent = 0u32;
            for batch in items.into_iter().chain(tombs) {
                let vault = batch.vault;
                let n = self.push_batch(access, batch, &mut conflicts, hosted)?;
                if n > 0 && hosted {
                    stats.pushed_vaults.insert(vault);
                }
                sent += n;
            }
            report.pushed += sent;
            if !conflicts.vaults.is_empty() {
                conflict_rounds += 1;
                if conflict_rounds > MAX_CONFLICT_ROUNDS {
                    return Ok(());
                }
                for vault in &conflicts.vaults {
                    self.pull_vault(access, *vault, report, stats)?;
                }
                if hosted {
                    self.recheck_conflicts(
                        access,
                        report,
                        stats,
                        &conflicts,
                        &mut rebuilt,
                        &mut stuck,
                    )?;
                }
            } else if sent == 0 {
                return Ok(());
            }
        }
        Ok(())
    }

    /// After the pull that follows a refused push: a record whose served
    /// version the pull did not change was refused on a base the server
    /// itself last gave this device. The vault is read again from the start
    /// (once per phase); if the record is still unchanged after that, the
    /// server contradicts itself: the vault is flagged and its changes stay
    /// queued.
    fn recheck_conflicts(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
        stats: &mut RunStats,
        conflicts: &Conflicts,
        rebuilt: &mut BTreeSet<VaultId>,
        stuck: &mut BTreeSet<VaultId>,
    ) -> Result<(), SyncError> {
        for vault in unexplained(access, conflicts)? {
            if rebuilt.insert(vault) {
                self.repull_vault(access, vault, report, stats)?;
            }
        }
        for vault in unexplained(access, conflicts)? {
            if rebuilt.contains(&vault) {
                access.with(|s| s.set_vault_flag(vault, VaultFlag::ServerInconsistent))??;
                stuck.insert(vault);
            }
        }
        Ok(())
    }

    /// Pushes one vault's records; returns how many the backend accepted.
    fn push_batch(
        &mut self,
        access: &mut impl SessionAccess,
        batch: Batch,
        conflicts: &mut Conflicts,
        hosted: bool,
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
                .map(|p| PushRecord {
                    record_id: p.record.record_id,
                    body: &p.record.body,
                    base_seq: p.base,
                })
                .collect();
            let results = match self.transport.push_records(vault, generation, &push) {
                Ok(results) => results,
                Err(TransportError::GenerationMismatch { .. }) => {
                    conflicts.vaults.insert(vault);
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            let mut accepted: Vec<(&StoredRecord, Option<u64>)> = Vec::new();
            for (pending, result) in request.iter().zip(results) {
                match result {
                    PushResult::Accepted { seq } => accepted.push((&pending.record, seq)),
                    PushResult::Conflict { .. } => {
                        conflicts.vaults.insert(vault);
                        conflicts
                            .records
                            .push((vault, pending.record.record_id, pending.base));
                    }
                }
            }
            sent += mark_accepted(access, vault, &accepted, hosted)?;
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

/// The vaults of the refused records whose served version the pull after
/// the refusal did not change.
fn unexplained(
    access: &mut impl SessionAccess,
    conflicts: &Conflicts,
) -> Result<BTreeSet<VaultId>, SyncError> {
    Ok(access.with(|s| -> Result<BTreeSet<VaultId>, Error> {
        let mut out = BTreeSet::new();
        for (vault, record, base) in &conflicts.records {
            if s.served_seq(*vault, *record)? == *base {
                out.insert(*vault);
            }
        }
        Ok(out)
    })??)
}

/// Marks the accepted records uploaded under a server sequence number (the
/// backend's, or a device-local one from the store). On the hosted backend
/// the served view takes the accepted version, and an acknowledgement under
/// a sequence number the device had already read is refused: an honest
/// server numbers a new record above its head, which is above every cursor.
fn mark_accepted(
    access: &mut impl SessionAccess,
    vault: VaultId,
    accepted: &[(&StoredRecord, Option<u64>)],
    hosted: bool,
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
        let cursor = s
            .sync_cursor(vault)?
            .and_then(|c| <[u8; 8]>::try_from(c.as_slice()).ok())
            .map_or(0, u64::from_be_bytes);
        let mut n = 0u32;
        for (record, seq) in accepted {
            let Some(outbox) = record.outbox_seq else {
                continue;
            };
            let seq = if let Some(q) = seq {
                if hosted && *q <= cursor {
                    // Impossible from an honest server: not acknowledged
                    // here (the change stays queued), reported at once.
                    s.set_vault_flag(vault, VaultFlag::ServerInconsistent)?;
                    continue;
                }
                if hosted {
                    let hash: [u8; 32] = Sha256::digest(record.body.as_bytes()).into();
                    s.note_served(vault, record.record_id, *q, hash)?;
                }
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
