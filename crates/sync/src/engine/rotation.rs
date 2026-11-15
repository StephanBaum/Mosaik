//! Vault key rotation (M6 spec §5.5, ADR-0013 item 2, M2-R12): the key is
//! offered to the backend first and the rotation is committed locally only
//! when the backend accepted it.

use mosaic_core::{KeyGeneration, VaultId};
use mosaic_vault::session::{
    Error, PreparedRotation, RotationJob, RotationKind, RotationPhase, UnlockedSession,
};

use super::pull::RunStats;
use super::rekey::{RekeyError, RekeyProgress};
use super::sharing::RotateOutcome;
use super::{RotationOutcome, SessionAccess, SyncEngine, SyncError, SyncReport};
use crate::clock::Clock;
use crate::transport::{Cursor, KeyPushResult, Transport};

/// Attempts at one rotation (a head that moves, a session that locked).
const ROTATION_ATTEMPTS: usize = 3;

/// What [`SyncEngine::rotate_vault_key`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultKeyRotation {
    /// How the key rotation ended.
    pub outcome: RotationOutcome,
    /// The attachment re-seal, if it was asked for and this device's key won.
    pub progress: Option<RekeyProgress>,
}

impl<T: Transport> SyncEngine<T> {
    /// Rotates a vault's key: pulls and pushes the vault so the new key is
    /// based on its latest state, offers the key to the backend and, once it
    /// is accepted, re-seals and sends the records under it.
    ///
    /// # Errors
    /// [`SyncError`].
    pub fn rotate_vault(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
        vault: VaultId,
    ) -> Result<RotationOutcome, SyncError> {
        self.rotate_vault_with(access, clock, vault, false)
    }

    /// [`Self::rotate_vault`]; with `reseal`, the vault-only job is opened
    /// (and its queue refreshed) before every offer of the key (M9-F1).
    fn rotate_vault_with(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
        vault: VaultId,
        reseal: bool,
    ) -> Result<RotationOutcome, SyncError> {
        // A halt stops a rotation like any other run, and nothing is written
        // before the checks that can raise one.
        Self::ensure_not_halted(access)?;
        self.check_own_logs(access)?;
        self.verify_checkpoints(access, clock.now_ms())?;
        let mut report = SyncReport::default();
        let mut stats = RunStats::default();
        self.pull_vault(access, vault, &mut report, &mut stats)?;
        self.push_phase(access, &mut report, &mut stats, Some(vault))?;
        if access.with(|s| s.vault_shared(vault))?? {
            // A shared vault rotates through the owner's atomic request
            // (CRYPTO_SPEC §8.3): it is the owner's device only, and every
            // member gets a share.
            if reseal {
                let next = Self::current_generation(access, vault)?
                    .next()
                    .map_err(|e| SyncError::Session(Error::Core(e)))?;
                access.with(|s| s.open_vault_offer_job(vault, next))??;
            }
            return match self.rotate_shared(
                access,
                vault,
                &std::collections::BTreeSet::new(),
                false,
                &mut report,
                &mut stats,
            )? {
                RotateOutcome::Rotated(generation) => Ok(RotationOutcome::Rotated(generation)),
                RotateOutcome::Retry | RotateOutcome::Blocked => {
                    Err(SyncError::Session(Error::NotOwner))
                }
            };
        }
        self.rotate_inner(access, vault, reseal)
    }

    /// Rotates a vault's key and, with `reseal_attachments`, re-seals the
    /// vault's attachments under fresh attachment IDs and file keys (the
    /// optional step of CRYPTO_SPEC §7.1, M7-R2): a vault-only rotation on
    /// the rotation journal ([`RotationKind::Vault`], phases 2 and 5 to 7). The
    /// re-seal runs only when this device's key won
    /// ([`RotationOutcome::Rotated`]). The job is opened before the key is
    /// offered (M9-F1), so it survives a crash or a lock at any point after
    /// the offer, and [`SyncEngine::vault_rotate_continue`] resumes it; it is
    /// dropped when another device's key won. A failed call leaves it open.
    ///
    /// A retry settles such a phase-2 job of this vault (one with an empty
    /// queue counts as this vault's: it re-seals nothing) as
    /// [`SyncEngine::vault_rotate_continue`] does, after a pull of the vault:
    /// if the vault holds the offered generation, the key of the earlier call
    /// landed and its re-seal is this call's result (no second rotation);
    /// otherwise the job is dropped and the rotation runs with this call's
    /// `reseal_attachments`.
    ///
    /// # Errors
    /// [`RekeyError::Session`] with `RotationInProgress` if
    /// `reseal_attachments` is set and any other rotation job is open
    /// (checked before anything changes); the errors of
    /// [`SyncEngine::rotate_vault`] as [`RekeyError::Sync`]; transport
    /// failures during the re-seal.
    pub fn rotate_vault_key(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
        vault: VaultId,
        reseal_attachments: bool,
    ) -> Result<VaultKeyRotation, RekeyError> {
        if let Some(job) = access.with(|s| s.rotation_job())?? {
            if Self::is_offer_job_of(access, &job, vault)? {
                Self::ensure_not_halted(access)?;
                if self.offer_landed(access, &job)? {
                    let progress = self.reseal_vault_job(access, clock)?;
                    let generation = Self::current_generation(access, vault)?;
                    return Ok(VaultKeyRotation {
                        outcome: RotationOutcome::Rotated(generation),
                        progress: Some(progress),
                    });
                }
                Self::drop_offer_job(access)?;
            } else if reseal_attachments {
                return Err(RekeyError::Session(Error::RotationInProgress));
            }
        }
        let outcome = self.rotate_vault_with(access, clock, vault, reseal_attachments)?;
        let mut progress = None;
        if reseal_attachments {
            match outcome {
                RotationOutcome::Rotated(_) => {
                    progress = Some(self.reseal_vault_job(access, clock)?);
                }
                RotationOutcome::RotatedByAnother(_) => Self::drop_offer_job(access)?,
            }
        }
        Ok(VaultKeyRotation { outcome, progress })
    }

    /// Resumes an open vault-only rotation job (an interrupted attachment
    /// re-seal, M7-R2): re-seals what is left, uploads and closes the job.
    /// Call it again while [`RekeyProgress::more_attachments`] is set.
    ///
    /// A job still at phase 2 (the key was offered, then a crash or a lock,
    /// M9-F1) is settled first: the vault is pulled, and if it holds the
    /// offered generation (or a later one) the re-seal runs; otherwise the
    /// backend never took the key, the job is dropped and the progress says
    /// `finished` with nothing done.
    ///
    /// # Errors
    /// [`RekeyError::NoJob`] without a vault job; [`RekeyError`].
    pub fn vault_rotate_continue(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
    ) -> Result<RekeyProgress, RekeyError> {
        let job = access
            .with(|s| s.rotation_job())??
            .filter(|j| j.kind == RotationKind::Vault)
            .ok_or(RekeyError::NoJob)?;
        if job.phase == RotationPhase::BackendStarted && !self.offer_landed(access, &job)? {
            Self::drop_offer_job(access)?;
            return Ok(RekeyProgress {
                finished: true,
                ..RekeyProgress::default()
            });
        }
        self.reseal_vault_job(access, clock)
    }

    /// Whether the vault of a phase-2 job holds the offered generation after
    /// a pull. A job with an empty queue has nothing to re-seal: `false`.
    fn offer_landed(
        &mut self,
        access: &mut impl SessionAccess,
        job: &RotationJob,
    ) -> Result<bool, SyncError> {
        let Some(vault) = access.with(|s| s.reseal_queue_vault())?? else {
            return Ok(false);
        };
        let mut report = SyncReport::default();
        let mut stats = RunStats::default();
        self.pull_vault(access, vault, &mut report, &mut stats)?;
        let held = u64::from(Self::current_generation(access, vault)?.get());
        Ok(job.target_version.is_some_and(|offered| held >= offered))
    }

    /// Whether `job` is a phase-2 vault job (the key offered, the outcome not
    /// known) of `vault`, or one with an empty queue, which re-seals nothing.
    fn is_offer_job_of(
        access: &mut impl SessionAccess,
        job: &RotationJob,
        vault: VaultId,
    ) -> Result<bool, SyncError> {
        if job.kind != RotationKind::Vault || job.phase != RotationPhase::BackendStarted {
            return Ok(false);
        }
        Ok(access
            .with(|s| s.reseal_queue_vault())??
            .is_none_or(|queued| queued == vault))
    }

    /// Drops the phase-2 vault job, if one is open.
    fn drop_offer_job(access: &mut impl SessionAccess) -> Result<(), SyncError> {
        let open = access.with(|s| s.rotation_job())??.is_some_and(|j| {
            j.kind == RotationKind::Vault && j.phase == RotationPhase::BackendStarted
        });
        if open {
            access.with(UnlockedSession::drop_vault_offer_job)??;
        }
        Ok(())
    }

    fn reseal_vault_job(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
    ) -> Result<RekeyProgress, RekeyError> {
        access.with(|s| s.set_rotation_phase(RotationPhase::Attachments))??;
        let mut progress = RekeyProgress::default();
        if self.reseal_loop(access, &mut progress)? {
            return Ok(progress);
        }
        self.finish_after_reseal(access, clock, &mut progress)?;
        Ok(progress)
    }

    /// The rotation itself, without the pull before it.
    pub(super) fn rotate_inner(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        reseal: bool,
    ) -> Result<RotationOutcome, SyncError> {
        let install = access.with(|s| s.install_id())?;
        let mut report = SyncReport::default();
        let mut stats = RunStats::default();
        let start = Self::current_generation(access, vault)?;
        let mut prepared: Option<PreparedRotation> = None;
        let mut last_error = None;
        for _ in 0..ROTATION_ATTEMPTS {
            let p = match prepared.take() {
                Some(p) => p,
                None => match access.with(|s| s.prepare_vault_rotation(vault)) {
                    Ok(Ok(p)) => p,
                    // The session locked, or relocked, between two steps:
                    // prepare again.
                    Ok(Err(Error::PreparedStale)) => continue,
                    Err(SyncError::Locked) => {
                        last_error = Some(SyncError::Locked);
                        continue;
                    }
                    Ok(Err(e)) => return Err(e.into()),
                    Err(e) => return Err(e),
                },
            };
            let generation = p.generation();
            if reseal {
                // Before every offer, so the queue also holds what a pull
                // after a moved head brought (M9-F1).
                access.with(|s| s.open_vault_offer_job(vault, generation))??;
            }
            let wrapped = p.wrapped().clone();
            let head = access
                .with(|s| s.sync_cursor(vault))??
                .map(Cursor)
                .unwrap_or_default();
            match self.transport.push_vault_key(
                vault,
                generation,
                &wrapped,
                install,
                Some(&head),
            )? {
                KeyPushResult::Accepted { .. } => {
                    // `None`: the prepared key went stale (the session relocked
                    // after the backend took it). The key is in this
                    // install's own log; pulling it took it, and if it did
                    // not arrive yet the next attempt prepares anew.
                    if let Some(outcome) =
                        self.commit_rotation(access, vault, p, &mut report, &mut stats)?
                    {
                        return Ok(outcome);
                    }
                    if Self::current_generation(access, vault)? > start {
                        return Self::current_outcome(access, vault);
                    }
                }
                KeyPushResult::GenerationTaken { .. } => {
                    drop(p);
                    self.pull_vault(access, vault, &mut report, &mut stats)?;
                    return Self::current_outcome(access, vault);
                }
                KeyPushResult::HeadMoved { .. } => {
                    self.pull_vault(access, vault, &mut report, &mut stats)?;
                    self.push_phase(access, &mut report, &mut stats, Some(vault))?;
                    // Same key again, against the new head.
                    prepared = Some(p);
                }
            }
        }
        Err(last_error.unwrap_or(SyncError::Transport(
            crate::transport::TransportError::Protocol,
        )))
    }

    pub(super) fn current_generation(
        access: &mut impl SessionAccess,
        vault: VaultId,
    ) -> Result<KeyGeneration, SyncError> {
        access
            .with(|s| {
                s.vaults()
                    .into_iter()
                    .find(|v| v.vault_id == vault)
                    .map(|v| v.key_generation)
            })?
            .ok_or(SyncError::Session(Error::UnknownVault))
    }

    /// Commits the prepared rotation. `Ok(None)` if it went stale (the
    /// session relocked since it was prepared); the pull that follows takes
    /// the key the backend already holds.
    fn commit_rotation(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        p: PreparedRotation,
        report: &mut SyncReport,
        stats: &mut RunStats,
    ) -> Result<Option<RotationOutcome>, SyncError> {
        match access.with(|s| s.commit_vault_rotation(p))? {
            Ok(generation) => {
                self.push_phase(access, report, stats, Some(vault))?;
                Ok(Some(RotationOutcome::Rotated(generation)))
            }
            Err(Error::KeyGenerationRollback { .. }) => {
                // The vault accepted that generation or a later one while
                // the key was on its way: take what is there.
                self.pull_vault(access, vault, report, stats)?;
                Self::current_outcome(access, vault).map(Some)
            }
            Err(Error::PreparedStale) => {
                self.pull_vault(access, vault, report, stats)?;
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn current_outcome(
        access: &mut impl SessionAccess,
        vault: VaultId,
    ) -> Result<RotationOutcome, SyncError> {
        let generation = access
            .with(|s| {
                s.vaults()
                    .into_iter()
                    .find(|v| v.vault_id == vault)
                    .map(|v| v.key_generation)
            })?
            .ok_or(SyncError::Session(Error::UnknownVault))?;
        Ok(RotationOutcome::RotatedByAnother(generation))
    }
}
