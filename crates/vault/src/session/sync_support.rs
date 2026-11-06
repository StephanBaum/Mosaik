//! What the sync layer needs from a session (M6, CRYPTO_SPEC §9.2, §10,
//! ADR-0045): the upload queue of vault keys and metadata, two-phase key
//! rotation, checkpoint and folder MACs, sealed device info, cursors and
//! local sequence numbers, attachment transfer, and the split halves of
//! joining an account and changing the master password.
//!
//! Keys never leave the session: `mosaic-sync` builds the byte formats and
//! asks the session for MACs and seals.

use mosaic_core::sync_auth::{self, FolderMacPurpose};
use mosaic_core::{
    AccountId, AttachmentId, DerivedKeys, InstallId, ItemId, KeyGeneration, MasterPassword,
    OpaquePassword, PasswordParams, SecretKey, SecretVec, UnlockMethod, VaultId, VaultKey,
    VaultMetaCiphertext, WrappedKey,
};

use super::sync::key_acceptance_error;
use super::{
    Error, LockedSession, PasswordChanged, QuickUnlockEnrollment, Result, UnlockError,
    UnlockedSession, reseal_below_current,
};
use crate::storage::{
    AttachmentInfo, Error as StorageError, KeyOrigin, RecordEntry, SyncConfig, VaultKeyWrap,
};

/// What a vault still has to send to the backend.
#[derive(Debug)]
pub struct PendingVaultUpload {
    /// The vault.
    pub vault_id: VaultId,
    /// The vault does not exist on the backend yet: its current generation,
    /// the wrapped key of that generation and its metadata create it.
    pub create: Option<(KeyGeneration, WrappedKey, VaultMetaCiphertext)>,
    /// Wrapped keys queued for upload (an offline rotation), oldest
    /// generation first. Empty while `create` is set.
    pub keys: Vec<(KeyGeneration, WrappedKey)>,
    /// Metadata queued for upload with the backend's sequence number of the
    /// metadata last seen (the base of the compare-and-swap). `None` while
    /// `create` is set, which carries the metadata.
    pub meta: Option<(VaultMetaCiphertext, Option<u64>)>,
}

/// A vault key rotation prepared but not applied: nothing in the store or
/// the session has changed ([`UnlockedSession::prepare_vault_rotation`]). The
/// sync engine offers the wrapped key to the backend first and commits only
/// if the backend accepts it ([`UnlockedSession::commit_vault_rotation`]).
/// The key is zeroized on drop; it cannot be cloned. It belongs to the
/// unlocked session that prepared it: after a lock, a commit is refused
/// ([`Error::PreparedStale`]).
#[derive(Debug)]
pub struct PreparedRotation {
    /// The tag of the session that prepared it.
    session_tag: [u8; 16],
    vault_id: VaultId,
    generation: KeyGeneration,
    key: VaultKey,
    wrapped: WrappedKey,
}

impl PreparedRotation {
    /// The vault.
    #[must_use]
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// The generation of the new key.
    #[must_use]
    pub const fn generation(&self) -> KeyGeneration {
        self.generation
    }

    /// The new key wrapped under the account key, as it is uploaded.
    #[must_use]
    pub const fn wrapped(&self) -> &WrappedKey {
        &self.wrapped
    }
}

/// An attachment an item refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachmentRef {
    /// The vault of the item.
    pub vault_id: VaultId,
    /// The item.
    pub item_id: ItemId,
    /// The attachment.
    pub attachment_id: AttachmentId,
    /// `false` if the item is in the trash.
    pub live: bool,
}

/// Whether this store holds an attachment's ciphertext.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentFileState {
    /// No row: the ciphertext has to be fetched.
    Missing,
    /// The ciphertext is stored.
    Present {
        /// It still has to be uploaded.
        upload_pending: bool,
    },
}

/// The public account values another device needs to join (CRYPTO_SPEC §9.2,
/// §10.1).
#[derive(Debug)]
pub struct AccountBootstrap {
    /// Account ID, Argon2id salt and profile.
    pub params: PasswordParams,
    /// `AK` wrapped under the password `KEK`.
    pub wrapped_ak: WrappedKey,
}

/// A master password change that is derived but not stored: both OPAQUE
/// passwords and the new parameters and wrapping
/// ([`UnlockedSession::prepare_master_password_change`]). The backend learns
/// the new registration first; the change is committed locally afterwards
/// ([`UnlockedSession::commit_master_password_change`]). Zeroizing, no
/// `Clone`. It belongs to the unlocked session that prepared it: after a
/// lock, a commit is refused ([`Error::PreparedStale`]).
#[derive(Debug)]
pub struct PreparedPasswordChange {
    /// The tag of the session that prepared it.
    session_tag: [u8; 16],
    old_opaque_password: OpaquePassword,
    new_opaque_password: OpaquePassword,
    new_params: PasswordParams,
    new_wrapped_ak: WrappedKey,
}

impl PreparedPasswordChange {
    /// `OPW` of the current password, for the fresh login of CRYPTO_SPEC §9.4.
    #[must_use]
    pub const fn old_opaque_password(&self) -> &OpaquePassword {
        &self.old_opaque_password
    }

    /// `OPW'` of the new password, for OPAQUE re-registration.
    #[must_use]
    pub const fn new_opaque_password(&self) -> &OpaquePassword {
        &self.new_opaque_password
    }

    /// The new public parameters (a new salt).
    #[must_use]
    pub const fn new_params(&self) -> &PasswordParams {
        &self.new_params
    }

    /// `AK` wrapped under the new `KEK'`.
    #[must_use]
    pub const fn new_wrapped_ak(&self) -> &WrappedKey {
        &self.new_wrapped_ak
    }
}

/// The keys of a join, derived from the master password and the Secret Key
/// before the join completes ([`LockedSession::derive_join_keys`]): the
/// device logs in with `OPW`, fetches the wrapped `AK`, then finishes with
/// [`LockedSession::join_account_with_keys`]. It carries the parameters it
/// was derived under, and the join stores exactly those, so the stored
/// parameters are the ones the keys belong to. Zeroizing, no `Clone`. There
/// is no session yet at this point, so it has no session tag.
#[derive(Debug)]
pub struct JoinKeys {
    keys: DerivedKeys,
    params: PasswordParams,
}

impl JoinKeys {
    /// `OPW`, the OPAQUE password input.
    #[must_use]
    pub const fn opaque_password(&self) -> &OpaquePassword {
        &self.keys.opaque_password
    }

    /// The public parameters the keys were derived under.
    #[must_use]
    pub const fn params(&self) -> &PasswordParams {
        &self.params
    }
}

/// The keys a transport needs to MAC and verify its files without a session
/// ([`UnlockedSession::sync_keys`]): the three keys derived from the account
/// key (CRYPTO_SPEC §10, §10.1) plus the IDs they are bound to. The folder
/// transport holds them while it is open and the owner drops the transport
/// when the session locks. Zeroizing, no `Clone`, redacted `Debug`.
#[derive(Debug)]
pub struct SyncKeys {
    /// The account.
    pub account_id: AccountId,
    /// This install.
    pub install_id: InstallId,
    /// `K_cp`, the checkpoint MAC key.
    pub checkpoint: mosaic_core::CheckpointKey,
    /// `K_fold`, the folder MAC key.
    pub folder: mosaic_core::FolderMacKey,
    /// `K_dev`, the device-info key.
    pub device_info: mosaic_core::DeviceInfoKey,
}

/// A damaged row reads as absent (the caller repairs it by syncing).
fn tolerate_corrupt<T>(row: core::result::Result<Option<T>, StorageError>) -> Result<Option<T>> {
    match row {
        Err(StorageError::Corrupt) => Ok(None),
        other => Ok(other?),
    }
}

fn unlock_error(e: mosaic_core::Error) -> Error {
    match e {
        mosaic_core::Error::Decryption => Error::UnlockFailed,
        other => Error::Core(other),
    }
}

impl LockedSession {
    /// The first half of [`LockedSession::join_account`]: derives `KEK` and
    /// `OPW` from the master password and the Secret Key under the account's
    /// public parameters (Argon2id, §5.3, §5.4). No store is touched.
    ///
    /// # Errors
    /// [`Error::Core`] for an empty password or a failed allocation.
    pub fn derive_join_keys(
        params: &PasswordParams,
        password: &MasterPassword,
        secret_key: &SecretKey,
    ) -> Result<JoinKeys> {
        Ok(JoinKeys {
            keys: mosaic_core::derive_password_keys(password, secret_key, params)?,
            params: *params,
        })
    }

    /// The second half of [`LockedSession::join_account`]: opens the wrapped
    /// `AK` with the derived keys, stores the account (with the parameters
    /// the keys carry) and unlocks.
    ///
    /// # Errors
    /// [`Error::UnlockFailed`] (nothing is stored then), which includes
    /// parameters that do not match the wrapped key;
    /// [`Error::Store`] with `AccountExists`.
    pub fn join_account_with_keys(
        mut self,
        wrapped_ak: &WrappedKey,
        keys: JoinKeys,
    ) -> core::result::Result<UnlockedSession, UnlockError> {
        let params = keys.params;
        let opened = wrapped_ak
            .unwrap_account_key(&keys.keys.kek, &params.account_id, &UnlockMethod::Password)
            .map_err(unlock_error);
        // The derived keys are wiped here, not at the end of the unlock.
        drop(keys);
        let joined = opened.and_then(|ak| {
            self.store.init_account(&params, wrapped_ak)?;
            Ok(ak)
        });
        match joined {
            Ok(ak) => UnlockedSession::load(
                self.store,
                params.account_id,
                ak,
                super::QuickState::PASSWORD,
            ),
            Err(error) => Err(UnlockError {
                session: self,
                error,
            }),
        }
    }

    /// The backend choice stored with this account, if sync is set up.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn sync_config(&self) -> Result<Option<SyncConfig>> {
        Ok(self.store.sync_config()?)
    }
}

impl UnlockedSession {
    /// This install's stable ID (CRYPTO_SPEC §10), read when the session was
    /// unlocked. Unlike [`UnlockedSession::device_id`] it does not change
    /// between sessions.
    #[must_use]
    pub const fn install_id(&self) -> InstallId {
        self.install
    }

    /// The keys a transport needs to MAC and verify files without a session
    /// ([`SyncKeys`]). Derived anew on every call.
    ///
    /// # Errors
    /// [`Error::Core`].
    pub fn sync_keys(&self) -> Result<SyncKeys> {
        Ok(SyncKeys {
            account_id: self.account_id,
            install_id: self.install,
            checkpoint: self.account_key.checkpoint_key()?,
            folder: self.account_key.folder_mac_key()?,
            device_info: self.account_key.device_info_key()?,
        })
    }

    /// The backend choice, if sync is set up.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn sync_config(&self) -> Result<Option<SyncConfig>> {
        Ok(self.store.sync_config()?)
    }

    /// Sets or clears (`None`) the backend choice.
    ///
    /// # Errors
    /// [`Error::Store`] with `SyncDataInvalid` for an out-of-range value.
    pub fn set_sync_config(&mut self, config: Option<&SyncConfig>) -> Result<()> {
        Ok(self.store.set_sync_config(config)?)
    }

    /// Prepares the store for a (new) backend in one transaction: every
    /// record of a synced vault is queued with its `server_seq` cleared, the
    /// current vault keys, the metadata and every attachment are queued, the
    /// vaults count as not created on the backend, and cursors, local
    /// sequence blocks and pending attachment deletions are cleared.
    /// Hardware-protected vaults are left alone.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn reset_sync_state(&mut self) -> Result<()> {
        self.store.reset_sync_state()?;
        self.created_handed.clear();
        Ok(())
    }

    /// The vaults that sync: wrapped under the account key and available in
    /// this session (hardware-protected vaults stay on their device).
    #[must_use]
    pub fn synced_vaults(&self) -> Vec<VaultId> {
        self.vaults
            .iter()
            .filter(|(_, ov)| ov.protection == VaultKeyWrap::AccountKey && ov.available())
            .map(|(id, _)| *id)
            .collect()
    }

    /// Up to `limit` local changes of one vault waiting for upload, oldest
    /// first.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn pending_uploads_in(&self, vault: VaultId, limit: usize) -> Result<Vec<RecordEntry>> {
        Ok(self.store.outbox_in(&vault, limit)?)
    }

    /// What every synced vault still has to send besides its records: the
    /// vaults to create on the backend, queued wrapped keys and queued
    /// metadata. The session remembers the metadata it hands out for a
    /// creation, so [`Self::mark_vault_created_remotely`] clears the queued
    /// flag only if that very metadata is still the stored one.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn pending_vault_uploads(&mut self) -> Result<Vec<PendingVaultUpload>> {
        let mut out = Vec::new();
        for vault_id in self.synced_vaults() {
            let state = self.store.vault_sync_state(&vault_id)?;
            let generation = self.vault(vault_id)?.generation;
            let upload = if state.remote_created {
                let meta = if state.meta_upload_pending {
                    tolerate_corrupt(self.store.vault_meta(&vault_id))?
                        .map(|m| (m, state.meta_server_seq))
                } else {
                    None
                };
                PendingVaultUpload {
                    vault_id,
                    create: None,
                    keys: self.store.pending_vault_keys(&vault_id)?,
                    meta,
                }
            } else {
                let key = tolerate_corrupt(self.store.vault_key(&vault_id, generation))?;
                let meta = tolerate_corrupt(self.store.vault_meta(&vault_id))?;
                let create = key.zip(meta).map(|(k, m)| (generation, k, m));
                if let Some((g, _, m)) = &create {
                    self.created_handed.insert(vault_id, (*g, m.clone()));
                }
                PendingVaultUpload {
                    vault_id,
                    create,
                    keys: Vec::new(),
                    meta: None,
                }
            };
            if upload.create.is_some() || !upload.keys.is_empty() || upload.meta.is_some() {
                out.push(upload);
            }
        }
        Ok(out)
    }

    /// Records that the backend created the vault with the key and metadata
    /// from [`Self::pending_vault_uploads`] (or already had it): the queued
    /// keys up to that generation and that metadata are done.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::Store`].
    pub fn mark_vault_created_remotely(&mut self, vault: VaultId) -> Result<()> {
        let (generation, meta) = match self.created_handed.remove(&vault) {
            Some((g, m)) => (g, Some(m)),
            None => (self.vault(vault)?.generation, None),
        };
        Ok(self
            .store
            .mark_vault_created(&vault, generation, meta.as_ref())?)
    }

    /// Records that the backend stored the wrapped key of generation `g`.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_vault_key_uploaded(&mut self, vault: VaultId, g: KeyGeneration) -> Result<()> {
        self.store.mark_vault_key_uploaded(&vault, g)?;
        Ok(())
    }

    /// Records that the backend stored `uploaded` as the vault's metadata at
    /// `seq`. The queued flag is cleared only if the stored metadata is
    /// still exactly `uploaded`; the sequence number is kept if it is not
    /// below the stored one. Returns whether the flag was cleared.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::Store`].
    pub fn mark_vault_meta_uploaded(
        &mut self,
        vault: VaultId,
        uploaded: &VaultMetaCiphertext,
        seq: u64,
    ) -> Result<bool> {
        Ok(self.store.mark_vault_meta_uploaded(&vault, uploaded, seq)?)
    }

    /// Notes the backend's sequence number of the vault's metadata (after
    /// applying a pulled one).
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::Store`].
    pub fn note_vault_meta_seq(&mut self, vault: VaultId, seq: u64) -> Result<()> {
        Ok(self.store.note_vault_meta_seq(&vault, seq)?)
    }

    /// Whether this install generated the vault key of generation `g`
    /// ([`KeyOrigin::Local`]) or received it ([`KeyOrigin::Remote`]); `None`
    /// if no such key is held. It decides the device in the canonical-winner
    /// rule (CRYPTO_SPEC §10.1).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn vault_key_origin(&self, vault: VaultId, g: KeyGeneration) -> Result<Option<KeyOrigin>> {
        Ok(self.store.vault_key_origin(&vault, g)?)
    }

    /// Generates the next vault key and wraps it under the account key
    /// without changing anything: not the store, not the session. Offer
    /// [`PreparedRotation::wrapped`] to the backend; if it accepts, call
    /// [`Self::commit_vault_rotation`].
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::VaultUnavailable`],
    /// [`Error::WrongVaultProtection`] for a hardware-protected vault,
    /// [`Error::Core`] (`InvalidKeyGeneration` at the last generation).
    pub fn prepare_vault_rotation(&self, vault: VaultId) -> Result<PreparedRotation> {
        let ov = self.vault(vault)?;
        ov.current_key()?;
        if ov.protection != VaultKeyWrap::AccountKey {
            return Err(Error::WrongVaultProtection);
        }
        let generation = ov.generation.next()?;
        let key = VaultKey::generate(generation)?;
        let wrapped = key.wrap(&self.account_key, &self.account_id, &vault)?;
        Ok(PreparedRotation {
            session_tag: self.session_tag,
            vault_id: vault,
            generation,
            key,
            wrapped,
        })
    }

    /// Applies a prepared rotation after the backend accepted its key: the
    /// key is accepted (the anti-rollback mark rises, §7.1) as generated here
    /// and already uploaded, then every readable record and the metadata are
    /// re-sealed under it and queued. Like [`Self::rotate_vault_key`] after
    /// the key is generated.
    ///
    /// # Errors
    /// [`Error::PreparedStale`] if it was prepared in an earlier unlocked
    /// session; [`Error::KeyGenerationRollback`] if the vault has accepted
    /// that generation or a later one meanwhile; [`Error::UnknownVault`],
    /// [`Error::VaultUnavailable`], [`Error::Store`].
    pub fn commit_vault_rotation(&mut self, p: PreparedRotation) -> Result<KeyGeneration> {
        let PreparedRotation {
            session_tag,
            vault_id,
            generation,
            key,
            wrapped,
        } = p;
        if session_tag != self.session_tag {
            return Err(Error::PreparedStale);
        }
        let ov = self.vaults.get_mut(&vault_id).ok_or(Error::UnknownVault)?;
        ov.current_key()?;
        if ov.generation >= generation {
            return Err(Error::KeyGenerationRollback {
                highest_accepted: ov.generation.get(),
                offered: generation.get(),
            });
        }
        self.store
            .accept_uploaded_vault_key(&vault_id, generation, &wrapped)
            .map_err(key_acceptance_error)?;
        ov.keys.insert(generation, key);
        ov.generation = generation;
        reseal_below_current(&mut self.store, vault_id, ov)?;
        Ok(generation)
    }

    fn checkpoint_key(&self) -> Result<mosaic_core::CheckpointKey> {
        Ok(self.account_key.checkpoint_key()?)
    }

    fn folder_key(&self) -> Result<mosaic_core::FolderMacKey> {
        Ok(self.account_key.folder_mac_key()?)
    }

    /// The MAC of this install's checkpoint (CRYPTO_SPEC §10): `device` is
    /// [`Self::install_id`].
    ///
    /// # Errors
    /// [`Error::Core`].
    pub fn checkpoint_mac(&self, counter: u64, seen_digest: &[u8; 32]) -> Result<[u8; 32]> {
        Ok(sync_auth::checkpoint_mac(
            &self.checkpoint_key()?,
            &self.install,
            counter,
            seen_digest,
        )?)
    }

    /// Whether `mac` is the checkpoint MAC of `device` (constant time).
    ///
    /// # Errors
    /// [`Error::Core`].
    pub fn verify_checkpoint_mac(
        &self,
        device: InstallId,
        counter: u64,
        seen_digest: &[u8; 32],
        mac: &[u8; 32],
    ) -> Result<bool> {
        Ok(sync_auth::verify_checkpoint_mac(
            &self.checkpoint_key()?,
            &device,
            counter,
            seen_digest,
            mac,
        )?)
    }

    /// The folder MAC of `msg` for `purpose` (CRYPTO_SPEC §10.1).
    ///
    /// # Errors
    /// [`Error::Core`].
    pub fn folder_mac(&self, purpose: FolderMacPurpose, msg: &[u8]) -> Result<[u8; 32]> {
        Ok(sync_auth::folder_mac(&self.folder_key()?, purpose, msg)?)
    }

    /// Whether `mac` is the folder MAC of `msg` for `purpose` (constant
    /// time).
    ///
    /// # Errors
    /// [`Error::Core`].
    pub fn verify_folder_mac(
        &self,
        purpose: FolderMacPurpose,
        msg: &[u8],
        mac: &[u8; 32],
    ) -> Result<bool> {
        Ok(sync_auth::verify_folder_mac(
            &self.folder_key()?,
            purpose,
            msg,
            mac,
        )?)
    }

    /// Seals this install's device info (`.mdev`, CRYPTO_SPEC §10.1).
    ///
    /// # Errors
    /// [`Error::Core`] (`SizeLimit` above 512 bytes, `Malformed` below: the plaintext is exactly 512 bytes).
    pub fn seal_device_info(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        Ok(sync_auth::seal_device_info(
            &self.account_key.device_info_key()?,
            &self.account_id,
            &self.install,
            plaintext,
        )?)
    }

    /// Opens the device info that install `device` sealed.
    ///
    /// # Errors
    /// [`Error::Core`] (`Decryption` for a forged file, `Malformed`).
    pub fn open_device_info(&self, device: InstallId, sealed: &[u8]) -> Result<SecretVec> {
        Ok(sync_auth::open_device_info(
            &self.account_key.device_info_key()?,
            &self.account_id,
            &device,
            sealed,
        )?)
    }

    /// The next checkpoint counter, persisted before it is returned.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn next_checkpoint_counter(&mut self) -> Result<u64> {
        Ok(self.store.next_checkpoint_counter()?)
    }

    /// The highest checkpoint counter used so far.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn checkpoint_counter(&self) -> Result<u64> {
        Ok(self.store.checkpoint_counter()?)
    }

    /// The opaque pull cursor of a vault, if one is stored.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn sync_cursor(&self, vault: VaultId) -> Result<Option<Vec<u8>>> {
        Ok(self.store.sync_cursor_blob(&vault)?)
    }

    /// Stores the opaque pull cursor of a vault (at most 64 KiB).
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::Store`].
    pub fn set_sync_cursor(&mut self, vault: VaultId, cursor: &[u8]) -> Result<()> {
        self.store
            .set_sync_cursor_blob(&vault, cursor)
            .map_err(|e| match e {
                StorageError::UnknownVault => Error::UnknownVault,
                other => Error::Store(other),
            })
    }

    /// Reserves `n` consecutive device-local sequence numbers for a vault
    /// and returns the first (see `Store::allocate_local_seqs`).
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::Store`].
    pub fn allocate_local_seqs(&mut self, vault: VaultId, n: u32) -> Result<u64> {
        self.store
            .allocate_local_seqs(&vault, n)
            .map_err(|e| match e {
                StorageError::UnknownVault => Error::UnknownVault,
                other => Error::Store(other),
            })
    }

    /// Every attachment that an item of a synced vault refers to, live or in
    /// the trash.
    #[must_use]
    pub fn attachment_references(&self) -> Vec<AttachmentRef> {
        let mut out = Vec::new();
        for (vault_id, ov) in &self.vaults {
            if ov.protection != VaultKeyWrap::AccountKey {
                continue;
            }
            for (item_id, item) in ov.items.iter().filter(|(_, i)| !i.is_deleted()) {
                let live = item.trashed_at().is_none();
                out.extend(item.attachments().into_iter().map(|a| AttachmentRef {
                    vault_id: *vault_id,
                    item_id: *item_id,
                    attachment_id: a.attachment_id,
                    live,
                }));
            }
        }
        out
    }

    /// Whether the ciphertext of an attachment is stored here.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn attachment_file_state(&self, id: AttachmentId) -> Result<AttachmentFileState> {
        Ok(match self.store.attachment_info(&id)? {
            None => AttachmentFileState::Missing,
            Some(info) => AttachmentFileState::Present {
                upload_pending: info.upload_pending,
            },
        })
    }

    /// The stored ciphertext of an attachment, for upload.
    ///
    /// # Errors
    /// [`Error::Store`] (`UnknownAttachment`, `Corrupt` for a missing file,
    /// `AttachmentsUnavailable` for an in-memory store).
    pub fn attachment_ciphertext(&self, id: AttachmentId) -> Result<Vec<u8>> {
        Ok(self.store.attachment(&id)?)
    }

    /// Stores an attachment ciphertext downloaded from the backend, not
    /// queued for upload. An item of `vault` (live or in the trash) must
    /// refer to `id`.
    ///
    /// # Errors
    /// [`Error::UnknownAttachmentReference`], [`Error::UnknownVault`],
    /// [`Error::Store`] (`AttachmentMalformed`, `AttachmentExists`,
    /// `AttachmentsUnavailable`).
    pub fn store_remote_attachment(
        &mut self,
        vault: VaultId,
        item: ItemId,
        id: AttachmentId,
        ct: &[u8],
    ) -> Result<()> {
        let referenced = self.vault(vault)?.items.get(&item).is_some_and(|i| {
            !i.is_deleted() && i.attachments().iter().any(|a| a.attachment_id == id)
        });
        if !referenced {
            return Err(Error::UnknownAttachmentReference);
        }
        Ok(self.store.put_attachment(&vault, &item, &id, ct, false)?)
    }

    /// Attachments waiting for upload, oldest first.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn pending_attachment_uploads(&self) -> Result<Vec<AttachmentInfo>> {
        Ok(self.store.pending_attachment_uploads()?)
    }

    /// Records that the backend stored the attachment. Returns whether it
    /// was pending.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_attachment_uploaded(&mut self, id: AttachmentId) -> Result<bool> {
        Ok(self.store.mark_attachment_uploaded(&id)?)
    }

    /// Attachments deleted here after they had been uploaded, whose deletion
    /// the backend does not know yet.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn pending_attachment_deletions(&self) -> Result<Vec<(VaultId, AttachmentId)>> {
        Ok(self.store.pending_attachment_deletions()?)
    }

    /// Records that the backend knows the attachment is deleted.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_attachment_deletion_sent(&mut self, id: AttachmentId) -> Result<()> {
        Ok(self.store.mark_attachment_deletion_sent(&id)?)
    }

    /// The public values a joining device needs: the account parameters and
    /// the password wrapping of `AK`.
    ///
    /// # Errors
    /// [`Error::NoAccount`], [`Error::Store`].
    pub fn account_bootstrap(&self) -> Result<AccountBootstrap> {
        let params = self.store.account()?.ok_or(Error::NoAccount)?;
        let wrapped = self
            .store
            .unlock_method(&UnlockMethod::Password)?
            .ok_or(Error::NoAccount)?;
        Ok(AccountBootstrap {
            params,
            wrapped_ak: wrapped.wrapped_account_key,
        })
    }

    /// Derives the keys of `password` and `secret_key` under the stored
    /// parameters and checks that they open the stored password wrapping to
    /// this session's account key.
    fn verified_keys(
        &self,
        password: &MasterPassword,
        secret_key: &SecretKey,
    ) -> Result<(PasswordParams, DerivedKeys)> {
        let params = self.store.account()?.ok_or(Error::NoAccount)?;
        let wrapped = self
            .store
            .unlock_method(&UnlockMethod::Password)?
            .ok_or(Error::NoAccount)?;
        let keys = mosaic_core::derive_password_keys(password, secret_key, &params)?;
        let ak = wrapped
            .wrapped_account_key
            .unwrap_account_key(&keys.kek, &params.account_id, &UnlockMethod::Password)
            .map_err(unlock_error)?;
        if ak == self.account_key {
            Ok((params, keys))
        } else {
            Err(Error::UnlockFailed)
        }
    }

    /// `OPW` for a fresh OPAQUE login (CRYPTO_SPEC §9.4): derived with the
    /// stored parameters.
    ///
    /// # Errors
    /// [`Error::UnlockFailed`] unless the derived `KEK` opens the stored
    /// password wrapping; [`Error::NoAccount`], [`Error::Core`],
    /// [`Error::Store`].
    pub fn opaque_password(
        &self,
        password: &MasterPassword,
        secret_key: &SecretKey,
    ) -> Result<OpaquePassword> {
        Ok(self.verified_keys(password, secret_key)?.1.opaque_password)
    }

    /// The first half of [`Self::change_master_password`]: checks `current`
    /// and the Secret Key, derives the new wrapping under a new salt and
    /// both OPAQUE passwords. Nothing is stored; the backend can learn the
    /// new registration before [`Self::commit_master_password_change`].
    ///
    /// # Errors
    /// [`Error::UnlockFailed`] for a wrong current password or Secret Key;
    /// [`Error::Core`] for an empty new password.
    pub fn prepare_master_password_change(
        &self,
        current: &MasterPassword,
        new: &MasterPassword,
        secret_key: &SecretKey,
    ) -> Result<PreparedPasswordChange> {
        let (params, old_keys) = self.verified_keys(current, secret_key)?;
        let change = mosaic_core::change_master_password(
            &self.account_key,
            new,
            secret_key,
            &params,
            params.profile,
        )?;
        Ok(PreparedPasswordChange {
            session_tag: self.session_tag,
            old_opaque_password: old_keys.opaque_password,
            new_opaque_password: change.opaque_password,
            new_params: change.params,
            new_wrapped_ak: change.wrapped_account_key,
        })
    }

    /// The second half of [`Self::change_master_password`]: stores the new
    /// parameters and wrapping. A quick-unlock wrapping does not survive
    /// the change (§6.2); it is deleted in the same transaction and returned
    /// so the app deletes its platform key.
    ///
    /// # Errors
    /// [`Error::PreparedStale`] if it was prepared in an earlier unlocked
    /// session; [`Error::Store`] (`AccountMismatch`, `ProfileDowngrade`).
    pub fn commit_master_password_change(
        &mut self,
        p: PreparedPasswordChange,
    ) -> Result<PasswordChanged> {
        if p.session_tag != self.session_tag {
            return Err(Error::PreparedStale);
        }
        let quick: Vec<UnlockMethod> = self
            .store
            .unlock_methods()?
            .into_iter()
            .map(|r| r.method)
            .filter(|m| matches!(m, UnlockMethod::QuickUnlock(_)))
            .collect();
        let removed =
            self.store
                .update_password_removing(&p.new_params, &p.new_wrapped_ak, &quick)?;
        let removed_quick_unlock = removed
            .into_iter()
            .filter_map(QuickUnlockEnrollment::from_row)
            .max_by_key(|e| *e.enrollment_id.as_bytes());
        Ok(PasswordChanged {
            params: p.new_params,
            opaque_password: p.new_opaque_password,
            removed_quick_unlock,
        })
    }
}
