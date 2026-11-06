//! Checkpoints, freeze and rollback detection (CRYPTO_SPEC §10, M6 spec
//! §5.7) and the end of a run.

use std::collections::{BTreeMap, BTreeSet};

use mosaic_core::{InstallId, KeyGeneration, VaultId};
use mosaic_vault::session::{Error, UnlockedSession, VaultProtection};
use mosaic_vault::storage::{PeerCheckpoint, PeerEntry, SyncHalt};

use super::pull::RunStats;
use super::{SessionAccess, SyncEngine, SyncError, SyncReport, SyncWarning};
use crate::clock::Clock;
use crate::transport::{Cursor, Transport, TransportError, TransportKind};
use crate::wire::WireError;
use crate::wire::checkpoint::{Checkpoint, CheckpointEntry, MAX_ENTRIES};
use crate::wire::cursor::FolderCursor;

/// The verified checkpoint with the highest counter per device.
fn verify_all(
    s: &UnlockedSession,
    raw: &[(InstallId, Vec<u8>)],
) -> BTreeMap<InstallId, Checkpoint> {
    let mut best: BTreeMap<InstallId, Checkpoint> = BTreeMap::new();
    for (device, bytes) in raw {
        let Ok(cp) = Checkpoint::decode(bytes, *device, s) else {
            continue;
        };
        if best.get(device).is_none_or(|b| cp.counter > b.counter) {
            best.insert(*device, cp);
        }
    }
    best
}

/// Whether device `device` has written more of the vault than this device
/// has read, by the rule of the backend kind.
fn is_lagging(kind: TransportKind, cursor: Option<&[u8]>, device: &InstallId, high: u64) -> bool {
    let read = match kind {
        TransportKind::Hosted => cursor
            .and_then(|c| <[u8; 8]>::try_from(c).ok())
            .map_or(0, u64::from_be_bytes),
        TransportKind::Folder => cursor
            .and_then(|c| FolderCursor::decode(c).ok())
            .and_then(|c| c.device(device).map(|d| d.contiguous))
            .unwrap_or(0),
    };
    high > read
}

/// What this device holds, for the lag rule.
struct Seen<'a> {
    kind: TransportKind,
    cursors: &'a BTreeMap<VaultId, Option<Vec<u8>>>,
    known: &'a BTreeSet<VaultId>,
}

/// Whether `device`'s entry for a vault shows data this device has not read.
/// A vault this device does not sync counts too: the device wrote it and it
/// never arrived (withheld), unless the entry is empty, the vault stays on
/// this device (hardware protected), or this device deleted it itself.
fn entry_lags(
    s: &UnlockedSession,
    seen: &Seen<'_>,
    device: &InstallId,
    entry: &PeerEntry,
) -> Result<bool, Error> {
    if let Some(cursor) = seen.cursors.get(&entry.vault_id) {
        return Ok(is_lagging(
            seen.kind,
            cursor.as_deref(),
            device,
            entry.high_seq,
        ));
    }
    if seen.known.contains(&entry.vault_id) || s.vault_deleted_locally(entry.vault_id)? {
        return Ok(false);
    }
    Ok(entry.high_seq > 0)
}

/// The `(device, vault)` pairs for which a device's newest stored checkpoint
/// shows data this device has not read.
fn lagging_pairs(
    s: &UnlockedSession,
    kind: TransportKind,
) -> Result<BTreeSet<(InstallId, VaultId)>, Error> {
    let mut cursors: BTreeMap<VaultId, Option<Vec<u8>>> = BTreeMap::new();
    for vault in s.synced_vaults() {
        cursors.insert(vault, s.sync_cursor(vault)?);
    }
    let known: BTreeSet<VaultId> = s.all_vault_ids()?.into_iter().collect();
    let seen = Seen {
        kind,
        cursors: &cursors,
        known: &known,
    };
    let mut lagging = BTreeSet::new();
    for cp in s.peer_checkpoints()? {
        for entry in &cp.entries {
            if entry_lags(s, &seen, &cp.device_id, entry)? {
                lagging.insert((cp.device_id, entry.vault_id));
            }
        }
    }
    Ok(lagging)
}

/// What the own checkpoint on the backend says about this install.
enum Verdict {
    Fine,
    Identity,
    Rollback,
}

impl<T: Transport> SyncEngine<T> {
    /// Step 3: reads and verifies every device's checkpoint, keeps the
    /// newest verified one of every other device (never an older one: a
    /// replayed checkpoint must not reset freeze detection) and checks this
    /// device's own against what it published. The strikes are persisted.
    pub(super) fn verify_checkpoints(
        &mut self,
        access: &mut impl SessionAccess,
        now: u64,
    ) -> Result<(), SyncError> {
        let raw = self.transport.checkpoints()?;
        let verdict = access.with(|s| -> Result<Verdict, Error> {
            let best = verify_all(s, &raw);
            let own = s.install_id();
            let local = s.checkpoint_counter()?;
            let state = s.sync_engine_state()?;
            for (device, cp) in &best {
                if *device == own {
                    continue;
                }
                s.put_peer_checkpoint(&PeerCheckpoint {
                    device_id: *device,
                    counter: cp.counter,
                    issued_at_ms: cp.issued_at_ms,
                    entries: cp
                        .entries
                        .iter()
                        .map(|e| PeerEntry {
                            vault_id: e.vault_id,
                            generation: e.generation,
                            high_seq: e.high_seq,
                        })
                        .collect(),
                })?;
            }
            let seen = best.get(&own).map_or(0, |c| c.counter);
            if seen > local {
                return Ok(Verdict::Identity);
            }
            let published = state.last_published_counter;
            if seen < published {
                // The backend lost a checkpoint the engine saw accepted. A
                // lost upload is tolerated once per published counter; the
                // second strike means the backend was rolled back.
                let mut count = state.strike_count;
                if state.strike_counter != Some(published) {
                    count = count.saturating_add(1);
                    s.set_checkpoint_strikes(
                        count,
                        state.strike_first_ms.or(Some(now)),
                        Some(published),
                    )?;
                }
                if count >= 2 {
                    return Ok(Verdict::Rollback);
                }
            } else if state.strike_count > 0 || state.strike_counter.is_some() {
                s.set_checkpoint_strikes(0, None, None)?;
            }
            Ok(Verdict::Fine)
        })??;
        match verdict {
            Verdict::Fine => Ok(()),
            Verdict::Identity => Err(Self::raise(access, SyncHalt::Identity)),
            Verdict::Rollback => Err(Self::raise(access, SyncHalt::Rollback)),
        }
    }

    /// Step 10 (lag): compares what other devices say they wrote (their
    /// newest verified checkpoints, as stored) with what this device has
    /// read, keeps the lag timers in the store, and returns the warnings
    /// past the grace and whether no device lags at all.
    pub(super) fn evaluate_lag(
        &mut self,
        access: &mut impl SessionAccess,
        now: u64,
    ) -> Result<(Vec<SyncWarning>, bool), SyncError> {
        let kind = self.config.transport_kind;
        let grace = self.config.freeze_grace_ms;
        let out = access.with(|s| -> Result<(Vec<SyncWarning>, bool), Error> {
            let lagging = lagging_pairs(s, kind)?;
            let stored: BTreeMap<(InstallId, VaultId), u64> = s
                .sync_lag()?
                .into_iter()
                .map(|(d, v, t)| ((d, v), t))
                .collect();
            let next: BTreeMap<(InstallId, VaultId), u64> = lagging
                .into_iter()
                .map(|key| (key, stored.get(&key).copied().unwrap_or(now)))
                .collect();
            if next != stored {
                let list: Vec<_> = next.iter().map(|((d, v), t)| (*d, *v, *t)).collect();
                s.set_sync_lag(&list)?;
            }
            let mut warnings: Vec<SyncWarning> = Vec::new();
            for ((device, _), since) in &next {
                if now.saturating_sub(*since) <= grace {
                    continue;
                }
                let warning = match kind {
                    TransportKind::Hosted => SyncWarning::FreezeSuspected {
                        device: *device,
                        since_ms: *since,
                    },
                    TransportKind::Folder => SyncWarning::MissingFiles {
                        device: *device,
                        since_ms: *since,
                    },
                };
                if !warnings.contains(&warning) {
                    warnings.push(warning);
                }
            }
            Ok((warnings, next.is_empty()))
        })??;
        Ok(out)
    }

    /// Step 8: publishes this device's checkpoint. The counter is persisted
    /// before the checkpoint is sent, and recorded as published once the
    /// backend accepted it.
    pub(super) fn publish_checkpoint(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
    ) -> Result<(), SyncError> {
        let (counter, vaults) = access.with(|s| -> Result<_, Error> {
            let counter = s.next_checkpoint_counter()?;
            let synced = s.synced_vaults();
            let mut vaults: Vec<(VaultId, KeyGeneration, Option<Vec<u8>>)> = Vec::new();
            for summary in s.vaults() {
                if synced.contains(&summary.vault_id) {
                    vaults.push((
                        summary.vault_id,
                        summary.key_generation,
                        s.sync_cursor(summary.vault_id)?,
                    ));
                }
            }
            Ok((counter, vaults))
        })??;
        let mut entries = Vec::new();
        for (vault_id, generation, cursor) in vaults {
            let cursor = Cursor(cursor.unwrap_or_default());
            entries.push(CheckpointEntry {
                vault_id,
                generation,
                high_seq: self.transport.checkpoint_high_seq(vault_id, &cursor)?,
            });
        }
        entries.sort_by_key(|e| e.vault_id);
        entries.truncate(MAX_ENTRIES);
        let bytes = access
            .with(|s| {
                Checkpoint {
                    device_id: s.install_id(),
                    counter,
                    issued_at_ms: clock.now_ms(),
                    entries,
                }
                .encode(s)
            })?
            .map_err(|e| match e {
                WireError::Locked => SyncError::Locked,
                _ => SyncError::Transport(TransportError::Protocol),
            })?;
        self.transport.put_checkpoint(&bytes)?;
        access.with(|s| s.set_last_published_counter(counter))??;
        Ok(())
    }

    /// The warnings and counters that close a run.
    pub(super) fn finish_report(
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
        stats: &RunStats,
        mut warnings: Vec<SyncWarning>,
    ) -> Result<(), SyncError> {
        if stats.unexplained > 0 {
            warnings.push(SyncWarning::Undecryptable {
                count: stats.unexplained,
            });
        }
        let hardware = access.with(|s| {
            s.vaults()
                .iter()
                .filter(|v| {
                    matches!(
                        s.vault_protection(v.vault_id),
                        Ok(VaultProtection::HardwareKey { .. })
                    )
                })
                .count()
        })?;
        if hardware > 0 {
            warnings.push(SyncWarning::HardwareVaultsLocal {
                count: u32::try_from(hardware).unwrap_or(u32::MAX),
            });
        }
        report.warnings = warnings;
        Ok(())
    }
}
