//! The pull phase (M6 spec §5.3): applying a page of keys, metadata and
//! records, deferring what is ahead of its key, and moving the stored
//! cursor only past what was applied.

use std::collections::{BTreeMap, BTreeSet};

use mosaic_core::{KeyGeneration, VaultId};
use mosaic_vault::session::{ApplyOutcome, Error, RemoteRecord, UnlockedSession, VaultProtection};
use mosaic_vault::storage::VaultFlag;
use sha2::{Digest, Sha256};

use super::consts::PULL_LIMIT;
use super::keys::Resolution;
use super::served;
use super::sharing::Collected;
use super::{SessionAccess, SyncEngine, SyncError, SyncReport};
use crate::sharing::MembersView;
use crate::transport::{
    Cursor, PullPage, PulledKey, PulledMeta, PulledRecord, Transport, TransportError, TransportKind,
};

/// Counters of a run that are not part of the report.
#[derive(Debug, Default)]
pub(super) struct RunStats {
    /// Objects that did not open and that no known key conflict explains.
    pub(super) unexplained: u32,
    /// Vaults the backend accepted records for in this run: they are pulled
    /// once more before the checkpoint, so the device's view of them is
    /// clean (every accepted record is read back).
    pub(super) pushed_vaults: BTreeSet<VaultId>,
    /// The member list of each shared vault as the server gave it at the
    /// vault's last sharing step (hosted, M6b).
    pub(super) members: BTreeMap<VaultId, MembersView>,
    /// The shared vaults whose owner page said a member re-keyed and asked
    /// for a rotation (M7).
    pub(super) rotation_requested: BTreeSet<VaultId>,
}

/// How a received object was handled.
enum Applied {
    Done,
    Deferred,
}

/// The state of one vault's pull.
#[derive(Default)]
struct VaultPull {
    committed: Cursor,
    saved: Cursor,
    blocked: bool,
    deferred_records: Vec<PulledRecord>,
    deferred_metas: Vec<PulledMeta>,
}

fn note_changed(report: &mut SyncReport, vault: VaultId) {
    if !report.changed_vaults.contains(&vault) {
        report.changed_vaults.push(vault);
    }
}

fn tally(report: &mut SyncReport, vault: VaultId, outcome: ApplyOutcome) {
    let add = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    match outcome {
        ApplyOutcome::New | ApplyOutcome::FastForward => report.pulled += 1,
        ApplyOutcome::ReplacedUnreadable => {
            report.pulled += 1;
            report.replaced_unreadable += 1;
        }
        ApplyOutcome::Merged {
            restored_fields,
            stamp_collision,
        } => {
            report.pulled += 1;
            report.merged += 1;
            report.restored_fields = report.restored_fields.saturating_add(add(restored_fields));
            report.stamp_collisions += u32::from(stamp_collision);
        }
        ApplyOutcome::Split { conflict_copies } => {
            report.pulled += 1;
            report.split_copies = report.split_copies.saturating_add(add(conflict_copies));
        }
        _ => return,
    }
    note_changed(report, vault);
}

fn is_undecryptable(e: &Error) -> bool {
    matches!(
        e,
        Error::Core(mosaic_core::Error::Decryption | mosaic_core::Error::ItemCorrupted { .. })
    )
}

fn is_deferral(e: &Error) -> bool {
    matches!(
        e,
        Error::KeyGenerationNotAccepted { .. } | Error::UnknownVault | Error::VaultUnavailable
    )
}

/// Counts an object that did not open; it is expected (not warned) while a
/// concurrent rotation of its generation is being resolved (the store holds
/// a key other than the held one for that generation).
fn count_undecryptable(
    s: &UnlockedSession,
    report: &mut SyncReport,
    stats: &mut RunStats,
    vault: VaultId,
    generation: KeyGeneration,
) {
    report.undecryptable += 1;
    if !s.has_key_conflict(vault, generation).unwrap_or(false) {
        stats.unexplained += 1;
    }
}

/// After the vault was read again completely following the user's
/// acknowledgement of a finding: every local record at the current
/// generation that the server does not hold as it is (no served row, or
/// another ciphertext) and that is not queued already is queued again. The
/// edit a lying server acknowledged and dropped, or a vault it rolled back,
/// would otherwise stay lost for good. A re-sealed copy of equal content
/// costs one upload.
fn requeue_unserved(s: &mut UnlockedSession, vault: VaultId) -> Result<(), Error> {
    if !s.rebuild_vaults()?.contains(&vault) {
        return Ok(());
    }
    let Some(generation) = s
        .vaults()
        .into_iter()
        .find(|v| v.vault_id == vault)
        .map(|v| v.key_generation)
    else {
        return s.finish_rebuild(vault);
    };
    let served: BTreeMap<_, _> = s
        .served_view(vault)?
        .into_iter()
        .map(|r| (r.record_id, r.ct_hash))
        .collect();
    let records: Vec<_> = s
        .store()
        .records(&vault)?
        .into_iter()
        .filter_map(mosaic_vault::storage::RecordEntry::ok)
        .collect();
    for record in records {
        if record.outbox_seq.is_some() || record.body.key_generation() != generation {
            continue;
        }
        let hash: [u8; 32] = Sha256::digest(record.body.as_bytes()).into();
        if served.get(&record.record_id) != Some(&hash) {
            s.requeue_record(vault, record.record_id)?;
        }
    }
    s.finish_rebuild(vault)
}

fn apply_key(
    s: &mut UnlockedSession,
    vault: VaultId,
    key: &PulledKey,
    report: &mut SyncReport,
) -> Result<(), Error> {
    match s.apply_remote_vault_key(vault, key.generation, &key.wrapped) {
        Ok(()) => {
            // This install's own log repeats keys that others generated
            // (a snapshot): never attributed to this install. The store
            // keeps the lowest device seen for the bytes.
            if key.device_id != s.install_id() {
                s.note_key_observation(vault, key.generation, &key.wrapped, key.device_id)?;
            }
            note_changed(report, vault);
        }
        Err(Error::VaultKeyConflict { .. }) => {
            // Persisted: a run that fails after this, or a lock before the
            // next run, must not forget the conflict.
            s.note_key_observation(vault, key.generation, &key.wrapped, key.device_id)?;
        }
        Err(Error::KeyGenerationRollback { .. }) => {
            // This install's own older keys come back through its own log.
            if key.device_id != s.install_id() {
                report.rollback_dropped += 1;
            }
        }
        Err(e @ Error::Store(_)) => return Err(e),
        // A wrapped key that does not open under the account key, or a
        // vault this session cannot take keys for.
        Err(_) => report.undecryptable += 1,
    }
    Ok(())
}

fn apply_record(
    s: &mut UnlockedSession,
    vault: VaultId,
    record: &PulledRecord,
    report: &mut SyncReport,
    stats: &mut RunStats,
) -> Result<Applied, Error> {
    let server_seq = match record.seq {
        Some(q) => q,
        None => s.allocate_local_seqs(vault, 1)?,
    };
    let remote = RemoteRecord {
        vault_id: vault,
        record_id: record.record_id,
        body: record.body.clone(),
        server_seq,
    };
    let mut result = s.apply_remote_record(&remote);
    if matches!(result, Err(Error::Core(mosaic_core::Error::DeviceIdReused))) {
        s.rotate_device_id()?;
        result = s.apply_remote_record(&remote);
    }
    match result {
        Ok(outcome) => {
            tally(report, vault, outcome);
            Ok(Applied::Done)
        }
        Err(e) if is_deferral(&e) => Ok(Applied::Deferred),
        Err(Error::KeyGenerationRollback { .. }) => {
            report.rollback_dropped += 1;
            Ok(Applied::Done)
        }
        Err(e) if is_undecryptable(&e) => {
            count_undecryptable(s, report, stats, vault, record.body.key_generation());
            Ok(Applied::Done)
        }
        Err(e) => Err(e),
    }
}

fn apply_meta(
    s: &mut UnlockedSession,
    vault: VaultId,
    meta: &PulledMeta,
    report: &mut SyncReport,
    stats: &mut RunStats,
) -> Result<Applied, Error> {
    let mut result = s.apply_remote_vault_meta(vault, &meta.ciphertext);
    if matches!(result, Err(Error::Core(mosaic_core::Error::DeviceIdReused))) {
        s.rotate_device_id()?;
        result = s.apply_remote_vault_meta(vault, &meta.ciphertext);
    }
    match result {
        Ok(outcome) => {
            if let Some(seq) = meta.seq {
                s.note_vault_meta_seq(vault, seq)?;
            }
            if matches!(outcome, ApplyOutcome::Merged { .. } | ApplyOutcome::New) {
                note_changed(report, vault);
            }
            Ok(Applied::Done)
        }
        Err(e) if is_deferral(&e) => Ok(Applied::Deferred),
        Err(Error::KeyGenerationRollback { .. }) => {
            report.rollback_dropped += 1;
            Ok(Applied::Done)
        }
        Err(e) if is_undecryptable(&e) => {
            count_undecryptable(s, report, stats, vault, meta.ciphertext.key_generation());
            Ok(Applied::Done)
        }
        Err(e) => Err(e),
    }
}

/// Moves the committed cursor over the units of a page that were applied.
fn advance_commit(
    vp: &mut VaultPull,
    page: &PullPage,
    deferred: &[usize],
    meta_deferred: bool,
    kind: TransportKind,
) {
    if vp.blocked {
        return;
    }
    if meta_deferred {
        vp.blocked = true;
        return;
    }
    if page.units.is_empty() {
        if deferred.is_empty() {
            vp.committed = page.next.clone();
        } else {
            vp.blocked = true;
            if kind == TransportKind::Hosted {
                let lowest = deferred
                    .iter()
                    .filter_map(|i| page.records.get(*i).and_then(|r| r.seq))
                    .min();
                if let Some(seq) = lowest {
                    vp.committed = Cursor(seq.saturating_sub(1).to_be_bytes().to_vec());
                }
            }
        }
        return;
    }
    for unit in &page.units {
        if deferred
            .iter()
            .any(|i| (unit.first_record..unit.end_record).contains(i))
        {
            vp.blocked = true;
            return;
        }
        vp.committed = unit.commit.clone();
    }
}

struct PageEnv<'a> {
    report: &'a mut SyncReport,
    stats: &'a mut RunStats,
    kind: TransportKind,
}

fn apply_page(
    s: &mut UnlockedSession,
    vault: VaultId,
    page: &PullPage,
    vp: &mut VaultPull,
    env: &mut PageEnv<'_>,
) -> Result<(), Error> {
    if env.kind == TransportKind::Hosted {
        // The served view: what the server showed for every record of the
        // page, whatever becomes of it (applied, ignored as dominated,
        // deferred or unreadable). Overwritten, never a maximum.
        for record in page
            .records
            .iter()
            .filter(|r| !served::is_reserved_id(&r.record_id))
        {
            if let Some(seq) = record.seq {
                let hash: [u8; 32] = Sha256::digest(record.body.as_bytes()).into();
                s.note_served(vault, record.record_id, seq, hash)?;
            }
        }
        // The metadata and the current roster as the server showed them
        // (R70): the roster with the highest generation and sequence.
        for meta in &page.metas {
            if let Some(seq) = meta.seq {
                served::note_meta(s, vault, seq, meta.ciphertext.as_bytes())?;
            }
        }
        if let Some(r) = page
            .rosters
            .iter()
            .filter(|r| r.seq.is_some())
            .max_by_key(|r| (r.generation, r.seq))
            && let Some(seq) = r.seq
        {
            served::note_roster(s, vault, seq, r.ciphertext.as_bytes())?;
        }
    }
    let mut sorted: Vec<&PulledKey> = page.keys.iter().collect();
    sorted.sort_by(|a, b| {
        (a.generation, a.device_id, a.wrapped.as_bytes()).cmp(&(
            b.generation,
            b.device_id,
            b.wrapped.as_bytes(),
        ))
    });
    for key in sorted {
        apply_key(s, vault, key, env.report)?;
    }
    let mut meta_deferred = false;
    for meta in &page.metas {
        match apply_meta(s, vault, meta, env.report, env.stats)? {
            Applied::Done => {}
            Applied::Deferred => {
                vp.deferred_metas.push(meta.clone());
                meta_deferred = true;
            }
        }
    }
    let mut deferred = Vec::new();
    for (i, record) in page.records.iter().enumerate() {
        // A record under a reserved row ID is never applied (M3).
        if served::is_reserved_id(&record.record_id) {
            env.report.undecryptable += 1;
            continue;
        }
        match apply_record(s, vault, record, env.report, env.stats)? {
            Applied::Done => {}
            Applied::Deferred => {
                vp.deferred_records.push(record.clone());
                deferred.push(i);
            }
        }
    }
    advance_commit(vp, page, &deferred, meta_deferred, env.kind);
    if vp.committed != vp.saved {
        s.set_sync_cursor(vault, &vp.committed.0)?;
        vp.saved = vp.committed.clone();
    }
    Ok(())
}

/// Applies the deferred objects once more (their keys may have arrived on a
/// later page); returns whether none is left.
fn retry_deferred(
    s: &mut UnlockedSession,
    vault: VaultId,
    vp: &mut VaultPull,
    env: &mut PageEnv<'_>,
) -> Result<bool, Error> {
    for meta in core::mem::take(&mut vp.deferred_metas) {
        if let Applied::Deferred = apply_meta(s, vault, &meta, env.report, env.stats)? {
            vp.deferred_metas.push(meta);
        }
    }
    for record in core::mem::take(&mut vp.deferred_records) {
        if let Applied::Deferred = apply_record(s, vault, &record, env.report, env.stats)? {
            vp.deferred_records.push(record);
        }
    }
    Ok(vp.deferred_metas.is_empty() && vp.deferred_records.is_empty())
}

impl<T: Transport> SyncEngine<T> {
    /// Pulls every remote vault and every local synced vault, and resolves
    /// concurrent rotations after each.
    pub(super) fn pull_phase(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
        stats: &mut RunStats,
        now: u64,
    ) -> Result<(), SyncError> {
        let remote = self.transport.list_vaults()?;
        let (local, hardware) = access.with(|s| {
            let mut hardware: BTreeSet<VaultId> = s
                .vaults()
                .iter()
                .map(|v| v.vault_id)
                .filter(|v| {
                    matches!(
                        s.vault_protection(*v),
                        Ok(VaultProtection::HardwareKey { .. })
                    )
                })
                .collect();
            // A vault the server refuses this device (removed, or left):
            // sync stops for it and its local copy stays readable.
            hardware.extend(
                s.synced_vaults()
                    .into_iter()
                    .filter(|v| s.vault_access_lost(*v).unwrap_or(false)),
            );
            (s.synced_vaults(), hardware)
        })?;
        let mut vaults: BTreeSet<VaultId> = remote.iter().map(|v| v.vault_id).collect();
        vaults.extend(local);
        for vault in vaults.difference(&hardware).copied().collect::<Vec<_>>() {
            self.pull_vault(access, vault, report, stats)?;
            if self.resolve_key_conflict(access, vault, now)? == Resolution::Adopted {
                // The cursor was reset by the adoption: what the device
                // skipped as unreadable opens under the adopted key.
                self.pull_vault(access, vault, report, stats)?;
            }
        }
        Ok(())
    }

    /// Reads a vault again from the start (the stored cursor is reset), for
    /// a hosted device whose served view of a record no longer matches what
    /// the server accepts.
    pub(super) fn repull_vault(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        report: &mut SyncReport,
        stats: &mut RunStats,
    ) -> Result<(), SyncError> {
        access.with(|s| s.set_sync_cursor(vault, &[]))??;
        self.pull_vault(access, vault, report, stats)
    }

    /// Pulls one vault until the backend has no more.
    pub(super) fn pull_vault(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        report: &mut SyncReport,
        stats: &mut RunStats,
    ) -> Result<(), SyncError> {
        self.pull_vault_inner(access, vault, report, stats, true)
    }

    /// [`Self::pull_vault`]; `follow` lets the sharing step read the vault
    /// once more after it claimed a key (the records that waited for it).
    pub(super) fn pull_vault_inner(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        report: &mut SyncReport,
        stats: &mut RunStats,
        follow: bool,
    ) -> Result<(), SyncError> {
        let stored = access
            .with(|s| s.sync_cursor(vault))??
            .map(Cursor)
            .unwrap_or_default();
        let mut vp = VaultPull {
            committed: stored.clone(),
            saved: stored.clone(),
            ..VaultPull::default()
        };
        let mut pull_cursor = stored;
        let kind = self.config.transport_kind;
        let mut collected = Collected::default();
        loop {
            let page = match self.transport.pull(vault, &pull_cursor, PULL_LIMIT) {
                Ok(page) => page,
                Err(TransportError::HeadBelowCursor) => {
                    // The vault went back in time on the server: reported
                    // for this vault, the others carry on.
                    access.with(|s| s.set_vault_flag(vault, VaultFlag::ServerRolledBack))??;
                    return Ok(());
                }
                // A member the server removed (or that left): the vault is
                // no longer served. Sync stops for it and the local copy
                // stays readable (CRYPTO_SPEC §8.3).
                Err(TransportError::Forbidden | TransportError::NotFound)
                    if access.with(|s| s.vault_shared(vault))?? =>
                {
                    access.with(|s| s.mark_access_lost(vault))??;
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            };
            collected.absorb(&page);
            let mut env = PageEnv {
                report,
                stats,
                kind,
            };
            access.with(|s| apply_page(s, vault, &page, &mut vp, &mut env))??;
            let progressed = page.next != pull_cursor;
            pull_cursor = page.next;
            if !page.more || !progressed {
                break;
            }
        }
        if !vp.deferred_records.is_empty() || !vp.deferred_metas.is_empty() {
            let mut env = PageEnv {
                report,
                stats,
                kind,
            };
            access.with(|s| -> Result<(), Error> {
                if retry_deferred(s, vault, &mut vp, &mut env)? {
                    s.set_sync_cursor(vault, &pull_cursor.0)?;
                }
                Ok(())
            })??;
        }
        if vp.deferred_records.is_empty() && vp.deferred_metas.is_empty() {
            access.with(|s| requeue_unserved(s, vault))??;
        } else {
            // A rebuild with a record that stays deferred is bounded
            // (`check_views`, CRYPTO_SPEC §10).
            access.with(|s| s.mark_rebuild_deferred(vault))??;
        }
        self.after_pull_sharing(access, vault, &collected, report, stats, follow)
    }
}
