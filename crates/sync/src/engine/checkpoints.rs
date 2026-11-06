//! Checkpoints, freeze and rollback detection (CRYPTO_SPEC §10, M6 spec
//! §5.7) and the end of a run.

use std::collections::{BTreeMap, BTreeSet};

use mosaic_core::{InstallId, KeyGeneration, VaultId};
use mosaic_vault::session::{Error, UnlockedSession, VaultProtection};

use super::pull::RunStats;
use super::{SessionAccess, SyncEngine, SyncError, SyncReport, SyncWarning};
use crate::clock::Clock;
use crate::transport::{Cursor, Transport, TransportKind};
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

impl<T: Transport> SyncEngine<T> {
    /// Step 3: reads and verifies every device's checkpoint and checks this
    /// device's own against its counter.
    pub(super) fn verify_checkpoints(
        &mut self,
        access: &mut impl SessionAccess,
    ) -> Result<(), SyncError> {
        let raw = self.transport.checkpoints()?;
        let (best, own, local) = access.with(|s| -> Result<_, Error> {
            Ok((verify_all(s, &raw), s.install_id(), s.checkpoint_counter()?))
        })??;
        let seen = best.get(&own).map_or(0, |c| c.counter);
        if seen > local {
            return Err(SyncError::IdentityDuplicated);
        }
        if seen < local {
            // A lost upload is tolerated once per counter value.
            if self.strike_at != Some(local) {
                self.strikes += 1;
                self.strike_at = Some(local);
            }
            if self.strikes >= 2 {
                return Err(SyncError::RollbackDetected);
            }
        } else {
            self.strikes = 0;
            self.strike_at = None;
        }
        self.others = best.into_iter().filter(|(d, _)| *d != own).collect();
        Ok(())
    }

    /// Step 10 (lag): compares what other devices say they wrote with what
    /// this device has read, and returns the warnings past the grace.
    pub(super) fn evaluate_lag(
        &mut self,
        access: &mut impl SessionAccess,
        now: u64,
    ) -> Result<Vec<SyncWarning>, SyncError> {
        let cursors: BTreeMap<VaultId, Option<Vec<u8>>> =
            access.with(|s| -> Result<_, Error> {
                let mut out = BTreeMap::new();
                for vault in s.synced_vaults() {
                    out.insert(vault, s.sync_cursor(vault)?);
                }
                Ok(out)
            })??;
        let kind = self.config.transport_kind;
        let mut lagging: BTreeSet<(InstallId, VaultId)> = BTreeSet::new();
        for (device, cp) in &self.others {
            for entry in &cp.entries {
                let Some(cursor) = cursors.get(&entry.vault_id) else {
                    continue;
                };
                if is_lagging(kind, cursor.as_deref(), device, entry.high_seq) {
                    lagging.insert((*device, entry.vault_id));
                }
            }
        }
        self.lag.retain(|k, _| lagging.contains(k));
        for key in lagging {
            self.lag.entry(key).or_insert(now);
        }
        let mut warnings: Vec<SyncWarning> = Vec::new();
        for ((device, _), since) in &self.lag {
            if now.saturating_sub(*since) <= self.config.freeze_grace_ms {
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
        Ok(warnings)
    }

    /// Step 8: publishes this device's checkpoint. The counter is persisted
    /// before the checkpoint is sent.
    pub(super) fn publish_checkpoint(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
    ) -> Result<(), SyncError> {
        let (counter, vaults) = access.with(|s| -> Result<_, Error> {
            let counter = s.next_checkpoint_counter()?;
            let mut vaults: Vec<(VaultId, KeyGeneration, Option<Vec<u8>>)> = Vec::new();
            for summary in s.vaults() {
                if s.synced_vaults().contains(&summary.vault_id) {
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
            .map_err(|_| SyncError::Session(Error::UnlockFailed))?;
        self.transport.put_checkpoint(&bytes)?;
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
