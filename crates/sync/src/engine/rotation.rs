//! Vault key rotation (M6 spec §5.5, ADR-0013 item 2, M2-R12): the key is
//! offered to the backend first and the rotation is committed locally only
//! when the backend accepted it.

use mosaic_core::VaultId;
use mosaic_vault::session::{Error, PreparedRotation};

use super::pull::RunStats;
use super::{RotationOutcome, SessionAccess, SyncEngine, SyncError, SyncReport};
use crate::clock::Clock;
use crate::transport::{Cursor, KeyPushResult, Transport};

/// Attempts at one rotation (a head that moves, a session that locked).
const ROTATION_ATTEMPTS: usize = 3;

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
        let _ = clock;
        let mut report = SyncReport::default();
        let mut stats = RunStats::default();
        self.pull_vault(access, vault, &mut report, &mut stats)?;
        self.push_phase(access, &mut report, &mut stats, Some(vault))?;
        self.rotate_inner(access, vault)
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
        let mut prepared: Option<PreparedRotation> = None;
        for _ in 0..ROTATION_ATTEMPTS {
            let p = match prepared.take() {
                Some(p) => p,
                None => access.with(|s| s.prepare_vault_rotation(vault))??,
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
                    return self.commit_rotation(access, vault, p, &mut report, &mut stats);
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
        Err(SyncError::Transport(
            crate::transport::TransportError::Protocol,
        ))
    }

    fn commit_rotation(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        p: PreparedRotation,
        report: &mut SyncReport,
        stats: &mut RunStats,
    ) -> Result<RotationOutcome, SyncError> {
        match access.with(|s| s.commit_vault_rotation(p))? {
            Ok(generation) => {
                self.push_phase(access, report, stats, Some(vault))?;
                Ok(RotationOutcome::Rotated(generation))
            }
            Err(Error::KeyGenerationRollback { .. }) => {
                // The vault accepted that generation or a later one while
                // the key was on its way: take what is there.
                self.pull_vault(access, vault, report, stats)?;
                Self::current_outcome(access, vault)
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
