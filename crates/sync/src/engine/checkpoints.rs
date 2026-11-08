//! Checkpoints, freeze and rollback detection (CRYPTO_SPEC §10, M6 spec
//! §5.7) and the end of a run.

use std::collections::{BTreeMap, BTreeSet};

use mosaic_core::{InstallId, KeyGeneration, VaultId};
use mosaic_vault::session::{Error, KeyOrigin, UnlockedSession, VaultProtection};
use mosaic_vault::storage::{PeerCheckpoint, PeerEntry, SyncHalt, VaultFlag};

use super::consts::NO_COMMON_VIEW_GRACE_MS;
use super::pull::RunStats;
use super::served::{record_rows, served_objects};
use super::{SessionAccess, SyncEngine, SyncError, SyncReport, SyncWarning};
use crate::clock::Clock;
use crate::transport::{Cursor, Rejection, Transport, TransportError, TransportKind};
use crate::wire::WireError;
use crate::wire::checkpoint::{
    Checkpoint, CheckpointEntry, MAX_ENTRIES, NO_VIEW, view_digest, view_key_commit,
    view_meta_commit,
};
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

/// The digest of this device's view of a hosted vault at `high_seq`, if the
/// view is clean: no served record lies beyond `high_seq` (the cursor), so
/// the view is exactly what the server showed up to that head. The digest
/// commits to the vault key this device holds (the wrapping as the server
/// serves it), to the vault metadata and to the roster as served (R70); a
/// key this device generated and the server never accepted is not committed
/// to at all (`None`: it is about to be replaced). An open finding does not
/// stop the digest: the server should not learn that it was caught.
fn own_view(s: &UnlockedSession, vault: VaultId, high_seq: u64) -> Result<Option<[u8; 32]>, Error> {
    let Some(generation) = s
        .vaults()
        .into_iter()
        .find(|v| v.vault_id == vault)
        .map(|v| v.key_generation)
    else {
        return Ok(None);
    };
    let Some(origin) = s.vault_key_origin(vault, generation)? else {
        return Ok(None);
    };
    if origin == KeyOrigin::Local && !s.vault_key_confirmed(vault, generation)? {
        return Ok(None);
    }
    let Some(wrapped) = s
        .store()
        .vault_keys(&vault)?
        .into_iter()
        .find(|(g, _)| *g == generation)
        .map(|(_, w)| w)
    else {
        return Ok(None);
    };
    let all = s.served_view(vault)?;
    if all.iter().any(|r| r.seq > high_seq) {
        return Ok(None);
    }
    let Some((meta, roster)) = served_objects(s, vault, &all)? else {
        return Ok(None);
    };
    let rows = record_rows(&all);
    let commit = view_key_commit(&vault, generation, wrapped.as_bytes().as_slice());
    let meta_commit = view_meta_commit(&vault, &meta, &roster);
    Ok(view_digest(&vault, high_seq, &commit, &meta_commit, &rows).ok())
}

/// The hosted cursor of a vault as a sequence number.
fn cursor_seq(cursor: Option<&[u8]>) -> u64 {
    cursor
        .and_then(|c| <[u8; 8]>::try_from(c).ok())
        .map_or(0, u64::from_be_bytes)
}

/// Compares one peer entry (same generation, synced vault) with this
/// device's view at the same head; a difference is a finding. Returns
/// whether the digests could be compared (the peer published one, the heads
/// are equal and this device's view is clean).
fn compare_view(s: &mut UnlockedSession, entry: &PeerEntry) -> Result<bool, Error> {
    if entry.view_digest == NO_VIEW {
        return Ok(false);
    }
    let cursor = s.sync_cursor(entry.vault_id)?;
    if cursor_seq(cursor.as_deref()) != entry.high_seq {
        return Ok(false);
    }
    let Some(mine) = own_view(s, entry.vault_id, entry.high_seq)? else {
        return Ok(false);
    };
    if mine != entry.view_digest {
        s.set_vault_flag(entry.vault_id, VaultFlag::ServerInconsistent)?;
    }
    Ok(true)
}

/// The bookkeeping of one peer entry: a comparison ends the count; the
/// first run that cannot compare starts it (with the peer counter then); a
/// peer whose counter has risen since, with failures spanning the grace
/// *across runs*, is reported (`NoCommonView`, persisted). A single run after
/// a long pause (or a clock jump) that finds no peer at its head does not
/// raise it: the runs that follow normally meet.
fn track_comparison(
    s: &mut UnlockedSession,
    cp: &PeerCheckpoint,
    entry: &PeerEntry,
    compared: bool,
    now: u64,
) -> Result<(), Error> {
    let (device, vault) = (cp.device_id, entry.vault_id);
    if compared {
        return s.put_view_compare(device, vault, None, cp.counter);
    }
    match s.view_compare(device, vault)? {
        Some((Some(since), counter)) if since <= now => {
            if cp.counter > counter && now - since > NO_COMMON_VIEW_GRACE_MS {
                s.set_no_common_view(device, vault, since)?;
            }
            Ok(())
        }
        // First failure, or a count started in the future (the clock was set
        // back): it starts now.
        _ => s.put_view_compare(device, vault, Some(now), cp.counter),
    }
}

/// Compares one peer's entries with this device's views. Per (peer, vault):
/// the digest of a vault that was acknowledged or is being rebuilt is not
/// compared; the others are.
fn compare_peer(
    s: &mut UnlockedSession,
    cp: &PeerCheckpoint,
    synced: &[VaultId],
    generations: &BTreeMap<VaultId, KeyGeneration>,
    skips: &[(InstallId, VaultId)],
    now: u64,
) -> Result<(), Error> {
    for entry in cp.entries.iter().filter(|e| synced.contains(&e.vault_id)) {
        if skips.contains(&(cp.device_id, entry.vault_id)) {
            continue;
        }
        let same = generations.get(&entry.vault_id) == Some(&entry.generation);
        let compared = same && compare_view(s, entry)?;
        track_comparison(s, cp, entry, compared, now)?;
    }
    Ok(())
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
                            view_digest: e.view_digest,
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
            // A stored start in the future belongs to a clock that has since
            // been set back: it counts from now (otherwise a freeze would
            // stay unreported until the old clock time came round again).
            let next: BTreeMap<(InstallId, VaultId), u64> = lagging
                .into_iter()
                .map(|key| {
                    let since = stored.get(&key).copied().filter(|t| *t <= now);
                    (key, since.unwrap_or(now))
                })
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

    /// Compares this device's served view of every hosted vault with the
    /// view digests other devices published for the same head (CRYPTO_SPEC
    /// §10): the device's cursor equals the entry's `high_seq`, the
    /// generations agree, and this device's view is clean. A different
    /// digest means the server showed two devices different contents for the
    /// same head; the vault is flagged (persisted).
    pub(super) fn check_views(
        &mut self,
        access: &mut impl SessionAccess,
        now: u64,
    ) -> Result<(), SyncError> {
        if self.config.transport_kind != TransportKind::Hosted {
            return Ok(());
        }
        access.with(|s| -> Result<(), Error> {
            let generations: BTreeMap<VaultId, KeyGeneration> = s
                .vaults()
                .into_iter()
                .map(|v| (v.vault_id, v.key_generation))
                .collect();
            let synced = s.synced_vaults();
            let peers = s.peer_checkpoints()?;
            let skips = s.digest_skips()?;
            // A rebuild that keeps a deferred record ends here, with a
            // finding for its vault (CRYPTO_SPEC §10): the skip of stale
            // digests must not outlive the rebuild for good.
            for vault in s.expire_rebuild_deferrals(now, NO_COMMON_VIEW_GRACE_MS)? {
                s.set_vault_flag(vault, VaultFlag::ServerInconsistent)?;
            }
            for cp in &peers {
                compare_peer(s, cp, &synced, &generations, &skips, now)?;
            }
            Ok(())
        })??;
        Ok(())
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
        let hosted = self.config.transport_kind == TransportKind::Hosted;
        let mut entries = Vec::new();
        for (vault_id, generation, cursor) in vaults {
            let cursor = Cursor(cursor.unwrap_or_default());
            let high_seq = self.transport.checkpoint_high_seq(vault_id, &cursor)?;
            let view = if hosted {
                access
                    .with(|s| own_view(s, vault_id, high_seq))??
                    .unwrap_or(NO_VIEW)
            } else {
                NO_VIEW
            };
            entries.push(CheckpointEntry {
                vault_id,
                generation,
                high_seq,
                view_digest: view,
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
        match self.transport.put_checkpoint(&bytes) {
            Ok(()) => access.with(|s| s.set_last_published_counter(counter))??,
            // The server holds this counter or a higher one: no honest
            // device causes that (it sends a higher counter every run), so
            // it is a repeat or a forged upload from a stolen token (R62).
            // The run goes on, but the published counter stays where it
            // was: counting it would turn the forged checkpoint's absence
            // from the verified ones into strikes and a halt that comes
            // back after every acknowledgement.
            Err(TransportError::Rejected(Rejection::CheckpointStale)) => {}
            Err(e) => return Err(e.into()),
        }
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
        for (device, vault, since_ms) in access.with(|s| s.sync_findings())??.no_common_view {
            warnings.push(SyncWarning::NoCommonView {
                device,
                vault,
                since_ms,
            });
        }
        for (vault, flag) in access.with(|s| s.vault_flags())?? {
            warnings.push(match flag {
                VaultFlag::ServerInconsistent => SyncWarning::ServerInconsistent { vault },
                VaultFlag::ServerRolledBack => SyncWarning::ServerRollback { vault },
            });
        }
        if hardware > 0 {
            warnings.push(SyncWarning::HardwareVaultsLocal {
                count: u32::try_from(hardware).unwrap_or(u32::MAX),
            });
        }
        // The warnings the steps of the run raised (a rotation that waits)
        // come first.
        let mut all = core::mem::take(&mut report.warnings);
        all.extend(warnings);
        report.warnings = all;
        Ok(())
    }
}
