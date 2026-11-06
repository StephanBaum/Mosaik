//! Vault creation on the backend and the canonical-winner rule for
//! concurrent key rotations (M6 spec §5.5, CRYPTO_SPEC §10.1).

use mosaic_core::{InstallId, KeyGeneration, VaultId, WrappedKey};
use mosaic_vault::session::{Error, KeyOrigin, UnlockedSession};

use super::{SessionAccess, SyncEngine, SyncError, SyncReport};
use crate::transport::{KeyPushResult, Transport};

impl<T: Transport> SyncEngine<T> {
    /// Step 2: creates the vaults the backend does not have and sends the
    /// wrapped keys an offline rotation queued.
    pub(super) fn upload_vaults(
        &mut self,
        access: &mut impl SessionAccess,
        _report: &mut SyncReport,
    ) -> Result<(), SyncError> {
        let install = access.with(|s| s.install_id())?;
        let uploads = access.with(UnlockedSession::pending_vault_uploads)??;
        for up in uploads {
            let vault = up.vault_id;
            if let Some((generation, wrapped, meta)) = &up.create {
                // `Created` and `Exists` both count as created; the pull
                // that follows brings in whatever an existing vault holds
                // before anything is pushed.
                self.transport
                    .create_vault(vault, *generation, wrapped, meta, install)?;
                access.with(|s| s.mark_vault_created_remotely(vault))??;
            }
            for (generation, wrapped) in &up.keys {
                match self
                    .transport
                    .push_vault_key(vault, *generation, wrapped, install, None)?
                {
                    // A taken generation is resolved by the canonical-winner
                    // rule after the next pull, not by retrying.
                    KeyPushResult::Accepted { .. } | KeyPushResult::GenerationTaken { .. } => {
                        access.with(|s| s.mark_vault_key_uploaded(vault, *generation))??;
                    }
                    KeyPushResult::HeadMoved { .. } => {}
                }
            }
        }
        Ok(())
    }

    /// After a pull: if another key of the current generation conflicted
    /// with the held one and the held one is not the canonical winner (the
    /// lowest `(device, wrapped bytes)`), rotates to the next generation.
    ///
    /// The conflicts are read from the store, where the pull noted every key
    /// it saw for a generation with the lowest device that wrote exactly
    /// those bytes: a conflict learned in a run that failed afterwards, or
    /// before a lock, is resolved by the next run. Every key is ranked by the
    /// lowest device seen for its bytes, the held one too (this install for
    /// a key it generated, or a lower device that carried the same bytes),
    /// so two devices with the same view agree on the winner.
    pub(super) fn resolve_key_conflict(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
    ) -> Result<(), SyncError> {
        let loses = access.with(|s| -> Result<bool, Error> {
            let Some(generation) = s
                .vaults()
                .into_iter()
                .find(|v| v.vault_id == vault)
                .map(|v| v.key_generation)
            else {
                return Ok(false);
            };
            s.prune_key_observations(vault)?;
            let seen = s.key_observations(vault, generation)?;
            let Some((origin, held, install)) = held_key(s, vault, generation)? else {
                return Ok(false);
            };
            let observed = seen
                .iter()
                .find(|(_, w)| w.as_bytes() == held.as_bytes())
                .map(|(d, _)| *d);
            let device = match (origin, observed) {
                (KeyOrigin::Local, Some(d)) => d.min(install),
                (KeyOrigin::Local, None) => install,
                (KeyOrigin::Remote, Some(d)) => d,
                // Unknown writer: assume the worst, so the device rotates
                // (safe: a rotation loses nothing).
                (KeyOrigin::Remote, None) => InstallId::from_bytes([0xff; 16]).unwrap_or(install),
            };
            let held_rank = (*device.as_bytes(), held.as_bytes().to_vec());
            Ok(seen.iter().any(|(d, w)| {
                w.as_bytes() != held.as_bytes()
                    && (*d.as_bytes(), w.as_bytes().to_vec()) < held_rank
            }))
        })??;
        if loses {
            self.rotate_inner(access, vault)?;
        }
        Ok(())
    }
}

/// The origin and wrapping of the key held for `generation`, and this
/// install's ID.
fn held_key(
    s: &UnlockedSession,
    vault: VaultId,
    generation: KeyGeneration,
) -> Result<Option<(KeyOrigin, WrappedKey, InstallId)>, Error> {
    let origin = s.vault_key_origin(vault, generation)?;
    let wrapped = s
        .store()
        .vault_keys(&vault)?
        .into_iter()
        .find(|(g, _)| *g == generation)
        .map(|(_, w)| w);
    Ok(origin.zip(wrapped).map(|(o, w)| (o, w, s.install_id())))
}
