//! What the sync layer needs from a session (M6, CRYPTO_SPEC §9.2, §10,
//! ADR-0045): the upload queue of vault keys and metadata, two-phase key
//! rotation, checkpoint and folder MACs, sealed device info, cursors and
//! local sequence numbers, attachment transfer, and the split halves of
//! joining an account and changing the master password.
//!
//! Keys never leave the session: `mosaic-sync` builds the byte formats and
//! asks the session for MACs and seals, one operation at a time. The
//! accessors at the end persist the engine's own state (own sequence
//! numbers, halt, peer checkpoints, lag timers, key observations,
//! delete-marker times; schema v4; the served view, confirmed keys, findings
//! against the server and rotation log; schema v5).

use mosaic_core::sync_auth::{self, FolderMacPurpose};
use mosaic_core::{
    AttachmentId, DerivedKeys, InstallId, ItemId, KeyGeneration, MasterPassword, OpaquePassword,
    PasswordParams, SecretKey, SecretVec, UnlockMethod, VaultId, VaultKey, VaultMetaCiphertext,
    WrappedKey,
};

use super::sync::key_acceptance_error;
use super::{
    Error, LockedSession, PasswordChanged, QuickUnlockEnrollment, Result, UnlockError,
    UnlockedSession, reseal_below_current,
};
use crate::storage::{
    AttachmentInfo, EngineState, Error as StorageError, KeyOrigin, MarkerStamp, OwnSeq,
    PeerCheckpoint, RecordEntry, ServedRow, SyncConfig, SyncHalt, VaultFlag, VaultKeyWrap,
};

/// Everything the sync engine has found wrong, for the user: see
/// [`UnlockedSession::sync_findings`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyncFindings {
    /// The halt (rollback or duplicated install), if sync is stopped.
    pub halt: Option<SyncHalt>,
    /// The vaults the server contradicted itself for, or rolled back.
    pub vault_flags: Vec<(VaultId, VaultFlag)>,
    /// The `(device, vault, since_ms)` pairs whose heads never met.
    pub no_common_view: Vec<(InstallId, VaultId, u64)>,
}

impl SyncFindings {
    /// Whether nothing is set.
    #[must_use]
    pub fn is_clear(&self) -> bool {
        self.halt.is_none() && self.vault_flags.is_empty() && self.no_common_view.is_empty()
    }

    /// The vaults that [`UnlockedSession::clear_vault_findings`] would
    /// rebuild: every flagged vault and every vault with no common view.
    #[must_use]
    pub fn vaults(&self) -> Vec<VaultId> {
        let mut out: Vec<VaultId> = self
            .vault_flags
            .iter()
            .map(|(v, _)| *v)
            .chain(self.no_common_view.iter().map(|(_, v, _)| *v))
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

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

    /// Whether `mac` is the folder MAC of `msg` for `purpose` under the account
    /// key that `wrapped_ak` holds when opened with the derived keys. Nothing
    /// is stored and the key is dropped at once: a joining device checks a
    /// candidate account file (CRYPTO_SPEC §10.1) before it commits to it.
    ///
    /// # Errors
    /// [`Error::UnlockFailed`] if `wrapped_ak` does not open under the keys;
    /// [`Error::Core`].
    pub fn verify_join_folder_mac(
        &self,
        wrapped_ak: &WrappedKey,
        keys: &JoinKeys,
        purpose: FolderMacPurpose,
        msg: &[u8],
        mac: &[u8; 32],
    ) -> Result<bool> {
        let ak = wrapped_ak
            .unwrap_account_key(
                &keys.keys.kek,
                &keys.params.account_id,
                &UnlockMethod::Password,
            )
            .map_err(unlock_error)?;
        let folder = ak.folder_mac_key()?;
        Ok(sync_auth::verify_folder_mac(&folder, purpose, msg, mac)?)
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
        // The same transaction notes that the backend holds the key the
        // vault was created with.
        Ok(self
            .store
            .mark_vault_created(&vault, generation, meta.as_ref())?)
    }

    /// Records that the backend accepted the wrapped key of generation `g`:
    /// the upload flag clears and the key counts as confirmed, in one
    /// transaction (the hosted key-winner rule, CRYPTO_SPEC §10.2).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_vault_key_accepted(&mut self, vault: VaultId, g: KeyGeneration) -> Result<()> {
        self.store.mark_vault_key_accepted(&vault, g)?;
        Ok(())
    }

    /// Queues a stored record for upload again (see
    /// [`crate::storage::Store::requeue_record`]).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn requeue_record(&mut self, vault: VaultId, record: ItemId) -> Result<bool> {
        Ok(self.store.requeue_record(&vault, &record)?)
    }

    /// The vaults that must be read again completely before local records
    /// the server lacks are queued ([`Self::clear_vault_findings`]).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn rebuild_vaults(&self) -> Result<Vec<VaultId>> {
        Ok(self.store.rebuild_vaults()?)
    }

    /// Ends the rebuild mark of a vault.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn finish_rebuild(&mut self, vault: VaultId) -> Result<()> {
        Ok(self.store.finish_rebuild(&vault)?)
    }

    /// Notes that a record of `vault` is still deferred after the vault was
    /// read again (a no-op unless the vault is being rebuilt).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_rebuild_deferred(&mut self, vault: VaultId) -> Result<()> {
        Ok(self.store.mark_rebuild_deferred(&vault)?)
    }

    /// Ends the rebuilds whose deferred record has outlived `grace_ms` and
    /// returns their vaults (the caller records a finding for each).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn expire_rebuild_deferrals(&mut self, now: u64, grace_ms: u64) -> Result<Vec<VaultId>> {
        Ok(self.store.expire_rebuild_deferrals(now, grace_ms)?)
    }

    /// The (peer, vault) pairs whose stored view digests are not compared
    /// (an acknowledgement of the vault, or a rebuild that was open when the
    /// peer's checkpoint arrived).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn digest_skips(&self) -> Result<Vec<(InstallId, VaultId)>> {
        Ok(self.store.digest_skips()?)
    }

    /// Where the comparisons of view digests with `device` for `vault`
    /// stand: since when they keep failing to happen (`None`: the last one
    /// succeeded) and the peer's checkpoint counter then.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn view_compare(
        &self,
        device: InstallId,
        vault: VaultId,
    ) -> Result<Option<(Option<u64>, u64)>> {
        Ok(self.store.view_compare(&device, &vault)?)
    }

    /// Stores where the comparisons stand (see [`Self::view_compare`]).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn put_view_compare(
        &mut self,
        device: InstallId,
        vault: VaultId,
        since_ms: Option<u64>,
        counter: u64,
    ) -> Result<()> {
        Ok(self
            .store
            .put_view_compare(&device, &vault, since_ms, counter)?)
    }

    /// Stores the finding that the heads of `device` and this device never
    /// met for `vault`.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn set_no_common_view(
        &mut self,
        device: InstallId,
        vault: VaultId,
        since_ms: u64,
    ) -> Result<()> {
        Ok(self.store.set_no_common_view(&device, &vault, since_ms)?)
    }

    /// What is wrong, as the user sees it: the halt and every finding against
    /// the server. The desktop asks once and acknowledges what is set with
    /// [`Self::clear_sync_halt`] and [`Self::clear_vault_findings`].
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn sync_findings(&self) -> Result<SyncFindings> {
        Ok(SyncFindings {
            halt: self.store.engine_state()?.halt,
            vault_flags: self.store.vault_flags()?,
            no_common_view: self.store.no_common_view()?,
        })
    }

    /// Acknowledges the findings against the server for `vaults`: they are
    /// cleared, the served view and cursor of those vaults are rebuilt from
    /// the server and local records the server lacks are queued again. The
    /// halt and the strikes are not touched ([`Self::clear_sync_halt`]).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn clear_vault_findings(&mut self, vaults: &[VaultId]) -> Result<()> {
        Ok(self.store.clear_vault_findings(vaults)?)
    }

    /// Records that the backend accepted the wrapped key of generation `g`
    /// (an accepted upload): the backend now holds exactly this key for the
    /// generation, which decides the key-winner rule on the hosted backend
    /// (CRYPTO_SPEC §10.2).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_vault_key_confirmed(&mut self, vault: VaultId, g: KeyGeneration) -> Result<()> {
        Ok(self.store.mark_key_confirmed(&vault, g)?)
    }

    /// Whether the backend confirmed the held vault key of generation `g`
    /// (see [`Self::mark_vault_key_confirmed`]).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn vault_key_confirmed(&self, vault: VaultId, g: KeyGeneration) -> Result<bool> {
        Ok(self.store.key_confirmed(&vault, g)?)
    }

    /// Overwrites the server's latest version of a record as this device
    /// knows it: the `seq` the server served or accepted it under and the
    /// SHA-256 of the exact ciphertext (hosted backend only; CRYPTO_SPEC
    /// §10). Never a maximum: the last answer of the server wins.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn note_served(
        &mut self,
        vault: VaultId,
        record: ItemId,
        seq: u64,
        ct_hash: [u8; 32],
    ) -> Result<()> {
        Ok(self.store.put_served(&vault, &record, seq, &ct_hash)?)
    }

    /// The `seq` under which the server last served or accepted the record
    /// (the base of the next upload of it).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn served_seq(&self, vault: VaultId, record: ItemId) -> Result<Option<u64>> {
        Ok(self.store.served(&vault, &record)?.map(|r| r.seq))
    }

    /// Every record of the vault as the server last served or accepted it,
    /// sorted by record ID (the input of the view digest).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn served_view(&self, vault: VaultId) -> Result<Vec<ServedRow>> {
        Ok(self.store.served_rows(&vault)?)
    }

    /// Stores a finding against the server for a vault; it is reported by
    /// every run until the user acknowledges it ([`Self::clear_sync_halt`]).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn set_vault_flag(&mut self, vault: VaultId, flag: VaultFlag) -> Result<()> {
        Ok(self.store.set_vault_flag(&vault, flag)?)
    }

    /// The findings against the server, by vault.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn vault_flags(&self) -> Result<Vec<(VaultId, VaultFlag)>> {
        Ok(self.store.vault_flags()?)
    }

    /// Records a conflict-driven rotation of the vault at `now_ms` (rows
    /// older than `window_ms` are dropped) and returns how many rotations
    /// the window now holds, this one included.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn log_conflict_rotation(
        &mut self,
        vault: VaultId,
        now_ms: u64,
        window_ms: u64,
    ) -> Result<u32> {
        self.store.log_rotation(&vault, now_ms, window_ms)?;
        Ok(self.store.recent_rotations(&vault, now_ms, window_ms)?)
    }

    /// How many conflict-driven rotations of the vault the window that ends
    /// at `now_ms` holds.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn recent_conflict_rotations(
        &self,
        vault: VaultId,
        now_ms: u64,
        window_ms: u64,
    ) -> Result<u32> {
        Ok(self.store.recent_rotations(&vault, now_ms, window_ms)?)
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

    /// The size of the stored ciphertext of an attachment (`None` if this
    /// store has none).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn attachment_ciphertext_len(&self, id: AttachmentId) -> Result<Option<u64>> {
        Ok(self.store.attachment_info(&id)?.map(|i| i.size))
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
    /// refer to `id`, and the bytes must authenticate under that item's file
    /// key (every chunk is opened before anything is stored).
    ///
    /// # Errors
    /// [`Error::UnknownAttachmentReference`], [`Error::UnknownVault`],
    /// [`Error::Core`] for bytes that do not authenticate,
    /// [`Error::Store`] (`AttachmentMalformed`, `AttachmentExists`,
    /// `AttachmentsUnavailable`).
    pub fn store_remote_attachment(
        &mut self,
        vault: VaultId,
        item: ItemId,
        id: AttachmentId,
        ct: &[u8],
    ) -> Result<()> {
        self.authenticate_remote_attachment(vault, item, id, ct)?;
        Ok(self.store.put_attachment(&vault, &item, &id, ct, false)?)
    }

    /// Opens every chunk of a downloaded attachment with the file key of the
    /// referencing item (CRYPTO_SPEC §7.3): the bytes are the attachment
    /// only if they authenticate.
    fn authenticate_remote_attachment(
        &self,
        vault: VaultId,
        item: ItemId,
        id: AttachmentId,
        ct: &[u8],
    ) -> Result<()> {
        let reference = self
            .vault(vault)?
            .items
            .get(&item)
            .filter(|i| !i.is_deleted())
            .and_then(|i| i.attachments().into_iter().find(|a| a.attachment_id == id))
            .ok_or(Error::UnknownAttachmentReference)?;
        mosaic_core::open_attachment(&reference.key, &vault, &item, &id, ct)?;
        Ok(())
    }

    /// Whether `ct` is the authentic ciphertext of an attachment an item
    /// refers to (it opens under the item's file key).
    ///
    /// # Errors
    /// [`Error::UnknownAttachmentReference`], [`Error::UnknownVault`].
    pub fn remote_attachment_authentic(
        &self,
        vault: VaultId,
        item: ItemId,
        id: AttachmentId,
        ct: &[u8],
    ) -> Result<bool> {
        match self.authenticate_remote_attachment(vault, item, id, ct) {
            Ok(()) => Ok(true),
            Err(Error::Core(_)) => Ok(false),
            Err(e) => Err(e),
        }
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

    // ---- Persisted engine state (schema v4) -------------------------------

    /// This install's own sequence numbers in the vault's folder log.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn own_seq(&self, vault: VaultId) -> Result<OwnSeq> {
        Ok(self.store.own_seq(&vault)?)
    }

    /// Persists that sequence numbers up to `through` may be used, before
    /// the segment is written.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn reserve_own_seq(&mut self, vault: VaultId, through: u64) -> Result<()> {
        Ok(self.store.reserve_own_seq(&vault, through)?)
    }

    /// Persists that the segment with sequence number `through` was written.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn confirm_own_seq(&mut self, vault: VaultId, through: u64) -> Result<()> {
        Ok(self.store.confirm_own_seq(&vault, through)?)
    }

    /// The engine's persisted halt, last published counter and strikes.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn sync_engine_state(&self) -> Result<EngineState> {
        Ok(self.store.engine_state()?)
    }

    /// Stops automatic sync until [`Self::clear_sync_halt`] (the first reason
    /// stays).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn set_sync_halt(&mut self, halt: SyncHalt) -> Result<()> {
        Ok(self.store.set_sync_halt(halt)?)
    }

    /// The user's acknowledgement of a halt (the desktop asks first): the
    /// halt and the strikes are cleared, and the own logs count as unchecked
    /// until written again. Reserved sequence numbers stay, so none is
    /// reused.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn clear_sync_halt(&mut self) -> Result<()> {
        Ok(self.store.clear_sync_halt()?)
    }

    /// Persists the counter of the checkpoint the backend accepted.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn set_last_published_counter(&mut self, counter: u64) -> Result<()> {
        Ok(self.store.set_last_published_counter(counter)?)
    }

    /// Persists the checkpoint strikes.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn set_checkpoint_strikes(
        &mut self,
        count: u32,
        first_ms: Option<u64>,
        at_counter: Option<u64>,
    ) -> Result<()> {
        Ok(self
            .store
            .set_checkpoint_strikes(count, first_ms, at_counter)?)
    }

    /// The newest verified checkpoint of every other device.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn peer_checkpoints(&self) -> Result<Vec<PeerCheckpoint>> {
        Ok(self.store.peer_checkpoints()?)
    }

    /// Stores a verified checkpoint of another device unless one with an
    /// equal or higher counter is stored. Returns whether it was stored.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn put_peer_checkpoint(&mut self, checkpoint: &PeerCheckpoint) -> Result<bool> {
        Ok(self.store.put_peer_checkpoint(checkpoint)?)
    }

    /// Since when each `(device, vault)` pair has lagged.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn sync_lag(&self) -> Result<Vec<(InstallId, VaultId, u64)>> {
        Ok(self.store.sync_lag()?)
    }

    /// Replaces the lag timers.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn set_sync_lag(&mut self, lag: &[(InstallId, VaultId, u64)]) -> Result<()> {
        Ok(self.store.set_sync_lag(lag)?)
    }

    /// Notes that `wrapped` was seen as the key of `generation`, written by
    /// `device` (the lowest device seen for those bytes is kept).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn note_key_observation(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        device: InstallId,
    ) -> Result<()> {
        Ok(self
            .store
            .note_key_observation(&vault, generation, wrapped, &device)?)
    }

    /// Every key seen for the generation with the lowest device that wrote
    /// exactly those bytes.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn key_observations(
        &self,
        vault: VaultId,
        generation: KeyGeneration,
    ) -> Result<Vec<(InstallId, WrappedKey)>> {
        Ok(self.store.key_observations(&vault, generation)?)
    }

    /// Whether a key other than the held one was seen for `generation` of
    /// the vault (a concurrent rotation).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn has_key_conflict(&self, vault: VaultId, generation: KeyGeneration) -> Result<bool> {
        Ok(self.store.has_key_conflict(&vault, generation)?)
    }

    /// Whether any vault has a key conflict at its current generation.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn has_any_unresolved_key_conflict(&self) -> Result<bool> {
        Ok(self.store.has_any_unresolved_key_conflict()?)
    }

    /// Forgets the keys seen for generations below the vault's current one.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn prune_key_observations(&mut self, vault: VaultId) -> Result<()> {
        Ok(self.store.prune_key_observations(&vault)?)
    }

    /// When this device first saw each delete marker of the vault, by its own
    /// clock, in the order of `markers`. Markers that are gone are forgotten.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn marker_first_seen(
        &mut self,
        vault: VaultId,
        markers: &[MarkerStamp],
        now: u64,
    ) -> Result<Vec<u64>> {
        Ok(self.store.marker_first_seen(&vault, markers, now)?)
    }

    /// Whether this device deleted the vault itself.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn vault_deleted_locally(&self, vault: VaultId) -> Result<bool> {
        Ok(self.store.vault_deleted_locally(&vault)?)
    }

    /// Every vault ID this store holds (any protection).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn all_vault_ids(&self) -> Result<Vec<VaultId>> {
        Ok(self.store.all_vault_ids()?)
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
        let removed_quick_unlock = self.store_password(&p.new_params, &p.new_wrapped_ak)?;
        Ok(PasswordChanged {
            params: p.new_params,
            opaque_password: p.new_opaque_password,
            removed_quick_unlock,
        })
    }

    /// Recovers a master password change that another run of this device
    /// applied to the backend only (CRYPTO_SPEC §9.4, "Change applied
    /// elsewhere"): the server (or the account file) holds the new password
    /// and its wrapping, this device the old ones. `keys` were derived with
    /// [`LockedSession::derive_join_keys`] from the typed new password, the
    /// Secret Key and the backend's `params`. They must open `wrapped_ak` to
    /// this session's own account key (constant-time comparison); then the
    /// parameters and the wrapping are stored exactly as
    /// [`Self::commit_master_password_change`] stores them (a quick-unlock
    /// wrapping is deleted in the same transaction). No new cryptography:
    /// the join derivation plus an equality check.
    ///
    /// # Errors
    /// [`Error::PasswordRollback`] if `params.salt` is a salt this device
    /// retired. [`Error::UnlockFailed`] for a wrong password or Secret Key, parameters
    /// that do not belong to `keys`, a wrapping that does not open, or an
    /// account key that is not this session's; nothing is stored then.
    /// [`Error::Store`] (`AccountMismatch`, `ProfileDowngrade`).
    pub fn adopt_remote_password(
        &mut self,
        keys: JoinKeys,
        wrapped_ak: &WrappedKey,
        params: PasswordParams,
    ) -> Result<PasswordChanged> {
        if keys.params != params {
            return Err(Error::UnlockFailed);
        }
        // A salt this device retired is a rollback to an old password.
        if self.store.salt_retired(&params.salt)? {
            return Err(Error::PasswordRollback);
        }
        let opened = wrapped_ak
            .unwrap_account_key(&keys.keys.kek, &params.account_id, &UnlockMethod::Password)
            .map_err(unlock_error)?;
        if opened != self.account_key {
            return Err(Error::UnlockFailed);
        }
        drop(opened);
        let removed_quick_unlock = self.store_password(&params, wrapped_ak)?;
        Ok(PasswordChanged {
            params,
            opaque_password: keys.keys.opaque_password,
            removed_quick_unlock,
        })
    }

    /// The highest folder account-file version this device wrote or adopted
    /// (0 if none): a folder offering this version or a lower one offers a
    /// rollback.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn folder_account_version(&self) -> Result<u64> {
        Ok(self.store.folder_account_version()?)
    }

    /// Whether `salt` was retired by a master password change that this
    /// device committed or adopted (a backend that offers it offers an older
    /// password).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn salt_retired(&self, salt: &mosaic_core::AccountSalt) -> Result<bool> {
        Ok(self.store.salt_retired(salt)?)
    }

    /// Records a folder account-file version this device wrote or adopted
    /// (never lowers the stored one).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn raise_folder_account_version(&mut self, version: u64) -> Result<()> {
        Ok(self.store.raise_folder_account_version(version)?)
    }

    /// Stores new password parameters and wrapping and deletes every
    /// quick-unlock wrapping in the same transaction (§6.2).
    fn store_password(
        &mut self,
        params: &PasswordParams,
        wrapped_ak: &WrappedKey,
    ) -> Result<Option<QuickUnlockEnrollment>> {
        let quick: Vec<UnlockMethod> = self
            .store
            .unlock_methods()?
            .into_iter()
            .map(|r| r.method)
            .filter(|m| matches!(m, UnlockMethod::QuickUnlock(_)))
            .collect();
        let removed = self
            .store
            .update_password_removing(params, wrapped_ak, &quick)?;
        Ok(removed
            .into_iter()
            .filter_map(QuickUnlockEnrollment::from_row)
            .max_by_key(|e| *e.enrollment_id.as_bytes()))
    }
}
