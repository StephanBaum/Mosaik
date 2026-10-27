//! Applying records, vault keys and vault metadata received from the
//! server (CRYPTO_SPEC §10), and the outbox. Local only: the network
//! protocol is M6.

use core::cmp::Ordering;

use mosaic_core::model::{Causality, VaultMeta};
use mosaic_core::{ItemId, KeyGeneration, VaultId, VaultKey, VaultMetaCiphertext, WrappedKey};

use super::{
    Error, OpenVault, Result, UnlockedSession, load_vault, open_record, reseal_below_current,
};
use crate::storage::{
    Error as StorageError, Origin, OutboxSeq, RecordBody, RecordEntry, VaultKeyWrap,
};

/// A record as received from the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRecord {
    /// The vault the server filed it under (part of the AAD).
    pub vault_id: VaultId,
    /// The record ID, which is the item ID (part of the AAD).
    pub record_id: ItemId,
    /// The ciphertext.
    pub body: RecordBody,
    /// The server's sequence number of this version.
    pub server_seq: u64,
}

/// What [`UnlockedSession::apply_remote_record`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApplyOutcome {
    /// A record this device did not have; stored as received.
    New,
    /// This device held the record, but its row could not be read (damaged,
    /// or sealed under a key this device no longer has), so there was no
    /// rollback anchor to compare with (§10): the received version replaced
    /// it without a version check. A malicious server could have served an
    /// older version of the item here; show it to the user as "restored
    /// from the server" (`THREAT_MODEL` R17).
    ReplacedUnreadable,
    /// The received version contains everything the local one had; stored
    /// as received.
    FastForward,
    /// Local and received versions were concurrent (or a repair was
    /// needed); the merge was stored and queued for upload.
    Merged {
        /// Fields written back after a delete raced an edit
        /// ([`mosaic_core::MergeOutcome::restored_fields`]).
        restored_fields: usize,
        /// Two writers share a device ID somewhere
        /// ([`mosaic_core::MergeOutcome::stamp_collision`]); warn the user.
        stamp_collision: bool,
    },
    /// The local version already contains the received one (a duplicate,
    /// a replay or a rollback, §10); nothing changed.
    Ignored,
    /// The merge did not fit the item size limit (ADR-0016): the record
    /// now holds the reduced merge every device computes
    /// ([`mosaic_core::Item::split_oversized`]: each field keeps the value
    /// it shows), queued for upload, and every value that moved out is a
    /// conflict copy in the same vault, under an ID every device derives
    /// the same way and marked with [`mosaic_core::Item::conflict_copy_of`]
    /// (find them by that mark). Show them to the user.
    Split {
        /// How many conflict copies the split has (some may have been
        /// stored earlier, by this device or as received from another).
        conflict_copies: usize,
    },
}

/// The §7.1 rule for a received object: exactly the highest accepted
/// generation.
pub(super) fn check_generation(accepted: KeyGeneration, offered: KeyGeneration) -> Result<()> {
    match offered.cmp(&accepted) {
        Ordering::Equal => Ok(()),
        Ordering::Less => Err(Error::KeyGenerationRollback {
            highest_accepted: accepted.get(),
            offered: offered.get(),
        }),
        Ordering::Greater => Err(Error::KeyGenerationNotAccepted {
            highest_accepted: accepted.get(),
            offered: offered.get(),
        }),
    }
}

/// The session's errors for the store's key-acceptance refusals, so a
/// caller sees one variant per rule whichever layer caught it
/// (`docs/CORE_API.md`: a rollback is dropped and never retried).
pub(super) fn key_acceptance_error(e: StorageError) -> Error {
    match e {
        StorageError::VaultKeyConflict { generation } => Error::VaultKeyConflict { generation },
        StorageError::KeyGenerationRollback {
            highest_accepted,
            offered,
        } => Error::KeyGenerationRollback {
            highest_accepted,
            offered,
        },
        other => Error::Store(other),
    }
}

impl UnlockedSession {
    /// Applies a record received from the server.
    ///
    /// The record must be sealed under the vault's highest accepted key
    /// generation (§7.1). It is decrypted with the IDs it was filed under,
    /// so a record moved to another ID or vault fails to authenticate. Then
    /// its authenticated version vector is compared with the local state
    /// (§10):
    ///
    /// - no local state: stored as received ([`ApplyOutcome::New`], or
    ///   [`ApplyOutcome::ReplacedUnreadable`] if the local row exists but
    ///   cannot be read and so gives no rollback anchor);
    /// - the local state dominates or equals it: ignored (rollback, replay
    ///   or duplicate);
    /// - newer or concurrent: merged with
    ///   [`mosaic_core::Item::merge_and_repair`]. If the result is exactly
    ///   the received version it is stored as received; otherwise the merge
    ///   is sealed under the current key and queued for upload, so the
    ///   server learns what this device added. A merge too large for the
    ///   item limit is split instead of failing ([`ApplyOutcome::Split`],
    ///   ADR-0016).
    ///
    /// Attachment files the result no longer references are removed.
    ///
    /// # Errors
    /// [`Error::KeyGenerationRollback`] / [`Error::KeyGenerationNotAccepted`]
    /// for a record of another generation; [`Error::Core`] with
    /// `ItemCorrupted` if it does not authenticate (nothing is stored), or
    /// with `DeviceIdReused` if, for a record this device already holds,
    /// it carries edits under this device's ID that this device never
    /// made (a higher counter, or one of this device's stamps with other
    /// contents): call [`UnlockedSession::rotate_device_id`] and apply it
    /// again; the colliding values of both writers are then kept, as
    /// conflict copies where they differ (ADR-0015). Equal version vectors
    /// with different contents are such a collision too: they are merged
    /// without losing a value and reported as `stamp_collision`.
    /// (A record this device never held is not checked: a moved item's
    /// copy legitimately keeps the source's version vector.) [`Error::UnknownVault`],
    /// [`Error::VaultUnavailable`], [`Error::Store`].
    pub fn apply_remote_record(&mut self, record: &RemoteRecord) -> Result<ApplyOutcome> {
        let (vault_id, item_id) = (record.vault_id, record.record_id);
        let device = self.device;
        let ov = self.vault(vault_id)?;
        ov.current_key()?;
        check_generation(ov.generation, record.body.key_generation())?;
        let received = open_record(&record.body, &ov.keys, &vault_id, &item_id)?;
        let (stored, damaged_row) = match self.store.record(&vault_id, &item_id) {
            Ok(stored) => (stored, false),
            // A row that fails the store's own checks: it has no readable
            // state and may be in the outbox; the received version replaces
            // it (and stays queued, so no local change is dropped).
            Err(StorageError::Corrupt) => (None, true),
            Err(e) => return Err(e.into()),
        };
        let pending = damaged_row || stored.as_ref().is_some_and(|s| s.outbox_seq.is_some());
        // A version vector only grows, but the server's order of versions
        // need not match it; never move the stored sequence number back.
        let server_seq = stored
            .as_ref()
            .and_then(|s| s.server_seq)
            .map_or(record.server_seq, |s| s.max(record.server_seq));

        let Some(local) = ov.items.get(&item_id) else {
            // No reuse check here: a record this device never stored can
            // legitimately carry its counter, because a moved item's copy
            // keeps the source's version vector (ADR-0012).
            let origin = if pending {
                // Only a corrupt local row can be pending without a
                // readable state; the received version replaces it.
                Origin::Merged { server_seq }
            } else {
                Origin::Remote { server_seq }
            };
            let unreadable = damaged_row || ov.corrupt.contains(&item_id);
            self.store_received(vault_id, item_id, record, received, origin)?;
            return Ok(if unreadable {
                ApplyOutcome::ReplacedUnreadable
            } else {
                ApplyOutcome::New
            });
        };

        match local.compare(&received) {
            Causality::Dominates => return Ok(ApplyOutcome::Ignored),
            Causality::Equal if *local == received => {
                // The same version re-encrypted under a newer generation
                // (a rotation) replaces the older ciphertext.
                if let Some(s) = &stored
                    && !pending
                    && s.body.key_generation() < record.body.key_generation()
                {
                    self.store.upsert_record(
                        &vault_id,
                        &item_id,
                        &record.body,
                        Origin::Remote { server_seq },
                    )?;
                }
                return Ok(ApplyOutcome::Ignored);
            }
            Causality::Equal | Causality::DominatedBy | Causality::Concurrent => {}
        }

        let outcome = local.merge_and_repair_after_reuse(&received, device, &self.retired)?;
        let restored_fields = outcome.restored_fields;
        let stamp_collision = outcome.stamp_collision;
        if outcome.item == received && !pending {
            self.store_received(
                vault_id,
                item_id,
                record,
                outcome.item,
                Origin::Remote { server_seq },
            )?;
            // A collision with equal values (only the write time differs)
            // has nothing to write back, so the result can equal the
            // received record and still be reported as a collision.
            if restored_fields == 0 && !stamp_collision {
                return Ok(ApplyOutcome::FastForward);
            }
        } else {
            match self.put_item(
                vault_id,
                item_id,
                outcome.item,
                Origin::Merged { server_seq },
            ) {
                Ok(()) => {}
                // The merge is valid but too large to seal; nothing was
                // written yet.
                Err(Error::Core(mosaic_core::Error::SizeLimit)) => {
                    return self.apply_oversized(vault_id, item_id, &received, server_seq);
                }
                Err(e) => return Err(e),
            }
        }
        self.prune_attachments(vault_id, item_id)?;
        Ok(ApplyOutcome::Merged {
            restored_fields,
            stamp_collision,
        })
    }

    /// [`Self::apply_remote_record`] for a merge that does not fit the item
    /// size limit: the deterministic split of ADR-0016. The values that move
    /// out of the record are stored as conflict copies first, under their
    /// derived IDs, so they are safe before the record changes; then the
    /// reduced record replaces it and is queued, so every device converges
    /// on it. A copy this device already holds under its ID (made by
    /// another device or by an earlier split here, which built the same
    /// copy, and maybe edited or deleted since) is left as it is.
    fn apply_oversized(
        &mut self,
        vault_id: VaultId,
        item_id: ItemId,
        received: &mosaic_core::Item,
        server_seq: u64,
    ) -> Result<ApplyOutcome> {
        let device = self.device;
        let ov = self.vault(vault_id)?;
        let local = ov.items.get(&item_id).ok_or(Error::UnknownItem)?;
        let split =
            local.split_oversized(received, &item_id, ov.current_key()?, device, &self.retired)?;
        let conflict_copies = split.conflict_copies.len();
        for (copy_id, copy) in split.conflict_copies {
            if !self.vault(vault_id)?.items.contains_key(&copy_id) {
                self.put_item(vault_id, copy_id, copy, Origin::Local)?;
            }
        }
        self.put_item(vault_id, item_id, split.item, Origin::Merged { server_seq })?;
        self.prune_attachments(vault_id, item_id)?;
        Ok(ApplyOutcome::Split { conflict_copies })
    }

    /// Stores a received ciphertext unchanged and caches its decryption.
    fn store_received(
        &mut self,
        vault_id: VaultId,
        item_id: ItemId,
        record: &RemoteRecord,
        item: mosaic_core::Item,
        origin: Origin,
    ) -> Result<()> {
        self.store
            .upsert_record(&vault_id, &item_id, &record.body, origin)?;
        self.index.update(vault_id, item_id, &item);
        let ov = self.vaults.get_mut(&vault_id).ok_or(Error::UnknownVault)?;
        ov.corrupt.remove(&item_id);
        ov.items.insert(item_id, item);
        self.prune_attachments(vault_id, item_id)
    }

    /// Applies a wrapped vault key received from the server: a vault shared
    /// with or created on another device of this account, or a rotation
    /// (§8.3). The wrapping is opened with the account key first, which
    /// authenticates the vault ID and the generation.
    ///
    /// A new generation raises the vault's anti-rollback mark, and every
    /// readable record (and the metadata) still sealed under an older
    /// generation is re-sealed under the new key and queued, so peers that
    /// already reject the old generation (§7.1) still get every version
    /// this device holds (ADR-0012 item 7). A generation below the mark is
    /// rejected. The key of a generation this device already holds is
    /// compared in constant time: the same key again is a no-op, a
    /// different one is a concurrent rotation and is refused
    /// ([`Error::VaultKeyConflict`], ADR-0013), so nothing sealed under
    /// the held key becomes unreadable.
    ///
    /// A vault that is unavailable because its current wrapped key is
    /// damaged or does not open under the account key (ADR-0010 item 9) is
    /// repaired by a wrapping that opens, of the current generation or a
    /// newer one: the unreadable row is replaced (it holds nothing this
    /// session could ever read, and the new wrapping authenticates the
    /// vault and the generation), and the vault's metadata and records are
    /// loaded (ADR-0012 item 11).
    ///
    /// # Errors
    /// [`Error::Core`] with `Decryption` if the wrapping does not open;
    /// [`Error::KeyGenerationRollback`] for a generation below the
    /// accepted one; [`Error::VaultKeyConflict`]; [`Error::VaultUnavailable`]
    /// for a hardware-protected vault.
    pub fn apply_remote_vault_key(
        &mut self,
        vault_id: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
    ) -> Result<()> {
        let vk =
            wrapped.unwrap_vault_key(&self.account_key, &self.account_id, &vault_id, generation)?;
        let Some(ov) = self.vaults.get_mut(&vault_id) else {
            // A vault deleted locally keeps its generation floor (ADR-0010):
            // a re-share below it is the session's rollback error too.
            self.store
                .create_vault(&vault_id, generation, wrapped)
                .map_err(key_acceptance_error)?;
            let mut ov = OpenVault::new(generation);
            ov.keys.insert(generation, vk);
            self.vaults.insert(vault_id, ov);
            return Ok(());
        };
        if !ov.available() {
            return self.repair_vault_key(vault_id, generation, wrapped, &vk);
        }
        if generation == ov.generation
            && let Some(held) = ov.keys.get(&generation)
        {
            // `VaultKey`'s equality is constant-time (`subtle`).
            return if *held == vk {
                Ok(())
            } else {
                Err(Error::VaultKeyConflict {
                    generation: generation.get(),
                })
            };
        }
        let state = self
            .store
            .accept_vault_key(&vault_id, generation, wrapped)
            .map_err(key_acceptance_error)?;
        ov.keys.insert(generation, vk);
        if state.highest_accepted_generation > ov.generation {
            ov.generation = state.highest_accepted_generation;
            reseal_below_current(&mut self.store, vault_id, ov)?;
        }
        Ok(())
    }

    /// [`Self::apply_remote_vault_key`] for a vault without its current
    /// key; `vk` is `wrapped` opened under the account key.
    fn repair_vault_key(
        &mut self,
        vault_id: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        vk: &VaultKey,
    ) -> Result<()> {
        let state = self.store.vault(&vault_id)?.ok_or(Error::UnknownVault)?;
        if state.key_wrap != VaultKeyWrap::AccountKey {
            // Hardware-protected (§7.4): its keys live under a TK, which
            // this session does not hold; an AK wrapping never replaces one.
            return Err(Error::VaultUnavailable);
        }
        let current = state.highest_accepted_generation;
        match generation.cmp(&current) {
            Ordering::Less => {
                return Err(Error::KeyGenerationRollback {
                    highest_accepted: current.get(),
                    offered: generation.get(),
                });
            }
            Ordering::Equal => {
                let held = match self.store.vault_key(&vault_id, generation) {
                    Ok(held) => held,
                    Err(StorageError::Corrupt) => None,
                    Err(e) => return Err(e.into()),
                };
                let readable = held.and_then(|w| {
                    w.unwrap_vault_key(&self.account_key, &self.account_id, &vault_id, generation)
                        .ok()
                });
                match readable {
                    // Not reached while the current key opens (the vault is
                    // then available), but a readable key is never replaced.
                    Some(held) if held == *vk => {}
                    Some(_) => {
                        return Err(Error::VaultKeyConflict {
                            generation: generation.get(),
                        });
                    }
                    None => match self.store.rewrap_vault_key(&vault_id, generation, wrapped) {
                        Ok(()) => {}
                        // No row at all: add it at the accepted generation.
                        Err(StorageError::UnknownVaultKey) => {
                            self.store
                                .accept_vault_key(&vault_id, generation, wrapped)
                                .map_err(key_acceptance_error)?;
                        }
                        Err(e) => return Err(e.into()),
                    },
                }
            }
            Ordering::Greater => {
                self.store
                    .accept_vault_key(&vault_id, generation, wrapped)
                    .map_err(key_acceptance_error)?;
            }
        }
        let state = self.store.vault(&vault_id)?.ok_or(Error::UnknownVault)?;
        let ov = load_vault(&self.store, &self.account_key, &self.account_id, &state)?;
        if ov.available() {
            reseal_below_current(&mut self.store, vault_id, &ov)?;
        }
        for (item_id, item) in &ov.items {
            self.index.update(vault_id, *item_id, item);
        }
        self.vaults.insert(vault_id, ov);
        Ok(())
    }

    /// Applies vault metadata received from the server: merged with the
    /// local metadata like an item, with
    /// [`mosaic_core::model::VaultMeta::merge_and_repair`] (a rename on two
    /// devices keeps both names as a conflict, also when two writers share
    /// a device ID and so wrote the same stamp). Returns what was done:
    /// [`ApplyOutcome::New`] without local metadata, and
    /// [`ApplyOutcome::ReplacedUnreadable`] if the local metadata row
    /// exists but could not be read (damaged, or sealed under a generation
    /// whose key is not held), so the received version replaced it without
    /// a rollback check (`THREAT_MODEL` R17).
    ///
    /// # Errors
    /// [`Error::KeyGenerationRollback`] / [`Error::KeyGenerationNotAccepted`]
    /// for another generation; [`Error::Core`] if it does not
    /// authenticate, or with `DeviceIdReused` if it carries edits under
    /// this device's ID that this device never made (call
    /// [`UnlockedSession::rotate_device_id`] and apply it again);
    /// [`Error::UnknownVault`], [`Error::VaultUnavailable`],
    /// [`Error::Store`].
    pub fn apply_remote_vault_meta(
        &mut self,
        vault_id: VaultId,
        meta: &VaultMetaCiphertext,
    ) -> Result<ApplyOutcome> {
        let device = self.device;
        let ov = self.vaults.get_mut(&vault_id).ok_or(Error::UnknownVault)?;
        let vk = ov.current_key()?;
        check_generation(ov.generation, meta.key_generation())?;
        let received = VaultMeta::open(meta, vk, &vault_id)?;
        let (merged, outcome) = match &ov.meta {
            // No readable local metadata: stored as received. A row that
            // exists but cannot be read gave no rollback anchor (R17).
            None if ov.meta_unreadable => (None, ApplyOutcome::ReplacedUnreadable),
            None => (None, ApplyOutcome::New),
            Some(local) => {
                match local.compare(&received) {
                    Causality::Dominates => return Ok(ApplyOutcome::Ignored),
                    Causality::Equal if *local == received => return Ok(ApplyOutcome::Ignored),
                    Causality::Equal | Causality::DominatedBy | Causality::Concurrent => {}
                }
                let (m, stamp_collision) =
                    local.merge_and_repair_after_reuse(&received, device, &self.retired)?;
                let outcome = if m == received && !stamp_collision {
                    ApplyOutcome::FastForward
                } else {
                    ApplyOutcome::Merged {
                        restored_fields: 0,
                        stamp_collision,
                    }
                };
                (Some(m), outcome)
            }
        };
        let merged = match merged {
            Some(m) if m != received => {
                self.store
                    .set_vault_meta(&vault_id, &m.seal(vk, &vault_id)?)?;
                m
            }
            _ => {
                self.store.set_vault_meta(&vault_id, meta)?;
                received
            }
        };
        ov.meta = Some(merged);
        ov.meta_unreadable = false;
        Ok(outcome)
    }

    /// Up to `limit` local changes waiting for upload, oldest first (a
    /// corrupt row is reported in place, see
    /// [`crate::storage::Store::outbox`]).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn pending_uploads(&self, limit: usize) -> Result<Vec<RecordEntry>> {
        Ok(self.store.outbox(limit)?)
    }

    /// Records that the server stored the change at `outbox_seq` as
    /// `server_seq` (see [`crate::storage::Store::mark_uploaded`]).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_uploaded(
        &mut self,
        vault_id: VaultId,
        record_id: ItemId,
        outbox_seq: OutboxSeq,
        server_seq: u64,
    ) -> Result<bool> {
        Ok(self
            .store
            .mark_uploaded(&vault_id, &record_id, outbox_seq, server_seq)?)
    }
}
