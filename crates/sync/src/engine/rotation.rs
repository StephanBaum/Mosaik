//! Vault key rotation (M6 spec §5.5, ADR-0013 item 2, M2-R12): the key is
//! offered to the backend first and the rotation is committed locally only
//! when the backend accepted it.

use mosaic_core::{KeyGeneration, VaultId};
use mosaic_vault::session::{Error, PreparedRotation, RotationKind, RotationPhase};

use super::pull::RunStats;
use super::rekey::{RekeyError, RekeyProgress};
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
            return match self.rotate_shared(
                access,
                vault,
                &std::collections::BTreeSet::new(),
                false,
                &mut report,
                &mut stats,
            )? {
                Some(generation) => Ok(RotationOutcome::Rotated(generation)),
                None => Err(SyncError::Session(Error::NotOwner)),
            };
        }
        self.rotate_inner(access, vault)
    }

    /// Rotates a vault's key and, with `reseal_attachments`, re-seals the
    /// vault's attachments under fresh attachment IDs and file keys (the
    /// optional step of CRYPTO_SPEC §7.1, M7-R2): a vault-only rotation on
    /// the rotation journal ([`RotationKind::Vault`], phases 5 to 7). The
    /// re-seal runs only when this device's key won
    /// ([`RotationOutcome::Rotated`]); the job it opens survives a crash or
    /// a lock, and [`SyncEngine::vault_rotate_continue`] resumes it.
    ///
    /// # Errors
    /// [`RekeyError::Session`] with `RotationInProgress` if a rotation job is
    /// open (checked before anything changes); the errors of
    /// [`SyncEngine::rotate_vault`] as [`RekeyError::Sync`]; transport
    /// failures during the re-seal.
    pub fn rotate_vault_key(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
        vault: VaultId,
        reseal_attachments: bool,
    ) -> Result<VaultKeyRotation, RekeyError> {
        if reseal_attachments && access.with(|s| s.rotation_job())??.is_some() {
            return Err(RekeyError::Session(Error::RotationInProgress));
        }
        let outcome = self.rotate_vault(access, clock, vault)?;
        let mut progress = None;
        if reseal_attachments && matches!(outcome, RotationOutcome::Rotated(_)) {
            access.with(|s| s.queue_vault_attachment_reseal(vault))??;
            progress = Some(self.reseal_vault_job(access, clock)?);
        }
        Ok(VaultKeyRotation { outcome, progress })
    }

    /// Resumes an open vault-only rotation job (an interrupted attachment
    /// re-seal, M7-R2): re-seals what is left, uploads and closes the job.
    /// Call it again while [`RekeyProgress::more_attachments`] is set.
    ///
    /// # Errors
    /// [`RekeyError::NoJob`] without a vault job; [`RekeyError`].
    pub fn vault_rotate_continue(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
    ) -> Result<RekeyProgress, RekeyError> {
        access
            .with(|s| s.rotation_job())??
            .filter(|j| j.kind == RotationKind::Vault)
            .ok_or(RekeyError::NoJob)?;
        self.reseal_vault_job(access, clock)
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
