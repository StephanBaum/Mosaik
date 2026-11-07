//! Vault creation on the backend and the key rules for concurrent key
//! rotations (M6 spec §5.5, CRYPTO_SPEC §10.1 and §10.2).
//!
//! Folder: the canonical winner is the lowest `(device, wrapped bytes)`; the
//! loser rotates again. Hosted: the server holds exactly one key per
//! generation, and that key is canonical. A device whose key for the
//! generation the server never accepted (an offline rotation that got
//! `generation_taken`) adopts the server's key and re-seals what it holds
//! under it; no further generation is created. A second valid key for a
//! generation the server confirmed (or delivered) is the server contradicting
//! itself: it is reported, the rank rule decides, and the rotations that
//! result are capped.

use mosaic_core::{InstallId, KeyGeneration, VaultId, WrappedKey};
use mosaic_vault::session::{AdoptOutcome, Error, KeyOrigin, UnlockedSession};
use mosaic_vault::storage::VaultFlag;

use super::consts::{CONFLICT_ROTATION_WINDOW_MS, MAX_CONFLICT_ROTATIONS};
use super::{SessionAccess, SyncEngine, SyncError, SyncReport};
use crate::transport::{KeyPushResult, Transport, TransportKind};

/// What the key rules did for a vault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Resolution {
    /// Nothing changed that needs another pull.
    Settled,
    /// The server's key replaced the device's own and the vault cursor was
    /// reset: the vault must be pulled again from the start.
    Adopted,
}

/// What the stored observations say to do.
enum Decision {
    Nothing,
    Rotate,
    Adopt {
        generation: KeyGeneration,
        wrapped: WrappedKey,
    },
}

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
                    KeyPushResult::Accepted { .. } => {
                        access.with(|s| s.mark_vault_key_accepted(vault, *generation))??;
                    }
                    // A taken generation is resolved by the key rules after
                    // the next pull, not by retrying. The key stays
                    // unconfirmed: on the hosted backend the server's key
                    // for the generation replaces it (adoption).
                    KeyPushResult::GenerationTaken { .. } => {
                        access.with(|s| s.mark_vault_key_uploaded(vault, *generation))??;
                    }
                    KeyPushResult::HeadMoved { .. } => {}
                }
            }
        }
        Ok(())
    }

    /// After a pull: applies the key rules to a vault whose generation has
    /// more than one key in the store.
    ///
    /// The conflicts are read from the store, where the pull noted every key
    /// it saw for a generation with the lowest device that wrote exactly
    /// those bytes: a conflict learned in a run that failed afterwards, or
    /// before a lock, is resolved by the next run.
    ///
    /// Folder: every key is ranked by the lowest device seen for its bytes,
    /// the held one too (this install for a key it generated, or a lower
    /// device that carried the same bytes), so two devices with the same view
    /// agree on the winner; the loser rotates to the next generation.
    ///
    /// Hosted: see the module documentation. The rank rule of a
    /// contradicting server rotates at most [`MAX_CONFLICT_ROTATIONS`] times
    /// per vault per day (persisted); after that the warning stays and the
    /// device stops rotating.
    pub(super) fn resolve_key_conflict(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        now: u64,
    ) -> Result<Resolution, SyncError> {
        let hosted = self.config.transport_kind == TransportKind::Hosted;
        let decision = access.with(|s| decide(s, vault, hosted, now))??;
        match decision {
            Decision::Nothing => Ok(Resolution::Settled),
            Decision::Rotate => {
                self.rotate_inner(access, vault)?;
                Ok(Resolution::Settled)
            }
            Decision::Adopt {
                generation,
                wrapped,
            } => {
                let adopted = access.with(|s| -> Result<bool, Error> {
                    match s.adopt_remote_vault_key(vault, generation, &wrapped) {
                        Ok(AdoptOutcome::Adopted { .. }) => {
                            // Whatever was skipped as unreadable while the
                            // keys differed opens now: read the vault again
                            // from the start.
                            s.set_sync_cursor(vault, &[])?;
                            Ok(true)
                        }
                        Ok(_) => Ok(false),
                        // The held key was confirmed after all: the server
                        // contradicts itself.
                        Err(Error::VaultKeyConflict { .. }) => {
                            s.set_vault_flag(vault, VaultFlag::ServerInconsistent)?;
                            Ok(false)
                        }
                        Err(e) => Err(e),
                    }
                })??;
                Ok(if adopted {
                    Resolution::Adopted
                } else {
                    Resolution::Settled
                })
            }
        }
    }
}

fn decide(
    s: &mut UnlockedSession,
    vault: VaultId,
    hosted: bool,
    now: u64,
) -> Result<Decision, Error> {
    let Some(generation) = s
        .vaults()
        .into_iter()
        .find(|v| v.vault_id == vault)
        .map(|v| v.key_generation)
    else {
        return Ok(Decision::Nothing);
    };
    s.prune_key_observations(vault)?;
    let seen = s.key_observations(vault, generation)?;
    let Some((origin, held, install)) = held_key(s, vault, generation)? else {
        return Ok(Decision::Nothing);
    };
    let differing: Vec<&(InstallId, WrappedKey)> = seen
        .iter()
        .filter(|(_, w)| w.as_bytes() != held.as_bytes())
        .collect();
    if differing.is_empty() {
        return Ok(Decision::Nothing);
    }
    if hosted && origin == KeyOrigin::Local && !s.vault_key_confirmed(vault, generation)? {
        // One server key for the generation: adopt it. More than one
        // distinct verified candidate is the server contradicting itself:
        // reported, nothing adopted.
        if let [(_, wrapped)] = differing.as_slice() {
            return Ok(Decision::Adopt {
                generation,
                wrapped: wrapped.clone(),
            });
        }
        s.set_vault_flag(vault, VaultFlag::ServerInconsistent)?;
        return Ok(Decision::Nothing);
    }
    if hosted {
        // The server delivered a second valid key for a generation it holds
        // one for.
        s.set_vault_flag(vault, VaultFlag::ServerInconsistent)?;
    }
    let observed = seen
        .iter()
        .find(|(_, w)| w.as_bytes() == held.as_bytes())
        .map(|(d, _)| *d);
    let device = match (origin, observed) {
        (KeyOrigin::Local, Some(d)) => d.min(install),
        (KeyOrigin::Local, None) => install,
        (KeyOrigin::Remote, Some(d)) => d,
        // Unknown writer: assume the worst, so the device rotates (safe: a
        // rotation loses nothing).
        (KeyOrigin::Remote, None) => InstallId::from_bytes([0xff; 16]).unwrap_or(install),
    };
    let held_rank = (*device.as_bytes(), held.as_bytes().to_vec());
    let loses = differing
        .iter()
        .any(|(d, w)| (*d.as_bytes(), w.as_bytes().to_vec()) < held_rank);
    if !loses {
        return Ok(Decision::Nothing);
    }
    if hosted {
        if s.recent_conflict_rotations(vault, now, CONFLICT_ROTATION_WINDOW_MS)?
            >= MAX_CONFLICT_ROTATIONS
        {
            // The cap: the warning stays, the rotating stops.
            return Ok(Decision::Nothing);
        }
        s.log_conflict_rotation(vault, now, CONFLICT_ROTATION_WINDOW_MS)?;
    }
    Ok(Decision::Rotate)
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
