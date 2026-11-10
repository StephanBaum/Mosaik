//! Key rotation in the session (M7, CRYPTO_SPEC §6.4 to §6.7): credential
//! changes (password, Secret Key, KDF profile) through one prepare path, the
//! full re-key in the two-phase style of the M6 rotation, the adoption of a
//! re-key another device made, the attachment re-seal and the rotation
//! journal.
//!
//! Like every rotation in this crate, `prepare_*` changes nothing and returns
//! a value that holds its keys in zeroizing types, is not `Clone` and belongs
//! to the unlocked session that made it ([`Error::PreparedStale`] across a
//! lock). The sync layer offers it to the backend and calls `commit_*` only
//! when the backend accepted. The network protocol is the sync layer's.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use mosaic_core::model::{AttachmentRef, GrantCover, Role, Text};
use mosaic_core::sync_auth::{self, MAX_EPOCH_JUMP};
use mosaic_core::{
    AccountId, AccountKey, Argon2Profile, AttachmentId, AttachmentKey, ContactsCiphertext,
    CredentialChange, Fingerprint, IdentityPublicKey, IdentitySecret, ItemId, KeyGeneration,
    MasterPassword, OpaquePassword, PasswordParams, SecretKey, ShareSeal, UnlockMethod, VaultId,
    VaultKey, VaultRosterCiphertext, WrappedKey, open_attachment, seal_attachment,
    seal_emergency_vault_key, seal_vault_key,
};

use super::sharing::{InviteId, Ownership, RotationPlan, active};
use super::sync_support::{JoinKeys, PreparedPasswordChange};
use super::unlock_methods::{HardwareKeyEnrollment, QuickUnlockEnrollment};
use super::{Error, OpenVault, Result, UnlockedSession, dup_item, now, seal_record};
use crate::storage::{
    AttachmentSwap, EpochSwitch, FOLDER_MARKER_LEN, FolderRekeyFloor, IdentityWrite,
    NewRotationJob, RecordBody, RekeyWrite, ResealRow, ResealState, RotatedVaultWrite, RotationJob,
    RotationKind, RotationPhase, SyncKind, VaultKeyWrap,
};

/// What the folder backend needs of a re-key: the account-file version the
/// re-keying device is going to write, and the marker's time.
#[derive(Clone, Copy, Debug)]
pub struct FolderRekeyPlan {
    /// The `account_version` of the new `.macct`: the highest verified one
    /// plus one (at least 1).
    pub to_account_version: u64,
    /// The marker's `created_at_ms`.
    pub created_at_ms: u64,
}

/// What the owner's device decided for a re-key (the backend's view of the
/// account, checked against the roster and the contact pins as for a
/// rotation).
#[derive(Debug, Default)]
pub struct RekeyPlan {
    /// For every owned shared vault: who stays (with the served key), the
    /// pending invitees and the emergency grantees. `revoked` must be empty:
    /// a re-key locks nobody out.
    pub shared: Vec<(VaultId, RotationPlan)>,
    /// The backend's head of each owned vault (`expectedHead`); 0 if absent.
    pub heads: Vec<(VaultId, u64)>,
    /// A folder re-key: what the marker and the account file need.
    pub folder: Option<FolderRekeyPlan>,
    /// The highest account-key epoch this device saw in its backend (a
    /// verified marker, or a plausible unverifiable file); 0 if none. The
    /// re-key targets `max(own epoch, seen_epoch) + 1`.
    pub seen_epoch: u32,
}

/// The inputs of [`UnlockedSession::prepare_rekey`].
#[derive(Debug)]
pub struct RekeyRequest<'a> {
    /// The current master password.
    pub current_password: &'a MasterPassword,
    /// The current Secret Key.
    pub current_secret_key: &'a SecretKey,
    /// The new master password: it must differ from the current one.
    pub new_password: &'a MasterPassword,
    /// The new Secret Key (already saved and verified in the new kit): it
    /// must differ from the current one and from every retired one.
    pub new_secret_key: &'a SecretKey,
    /// The new profile, not weaker than the current one.
    pub profile: Argon2Profile,
    /// What the backend knows.
    pub plan: RekeyPlan,
}

/// An owned shared vault in [`RekeyReview`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedOwnedReview {
    /// The vault.
    pub vault_id: VaultId,
    /// Its active members besides the owner.
    pub members: Vec<AccountId>,
}

/// A vault someone else owns, as [`RekeyReview`] lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemberVaultReview {
    /// The vault.
    pub vault_id: VaultId,
    /// Its owner.
    pub owner: AccountId,
}

/// What a re-key will touch, for the review screen
/// ([`UnlockedSession::rekey_review`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RekeyReview {
    /// Vaults the account owns that are not shared: they rotate to `g+1`.
    pub owned_vaults: Vec<VaultId>,
    /// Vaults the account owns and shares: they rotate with fresh seals.
    pub shared_owned: Vec<SharedOwnedReview>,
    /// Vaults others own: re-wrapped here, readable to the holder of the old
    /// AK until their owner rotates (THREAT_MODEL R86).
    pub member_vaults: Vec<MemberVaultReview>,
    /// Hardware-protected vaults: not affected (their key is under TK).
    pub tk_vaults: Vec<VaultId>,
    /// The accounts that hold an emergency grant from this account.
    pub grants: Vec<AccountId>,
    /// Registered security keys: their unlock wrappings are deleted.
    pub hardware_keys: u32,
    /// Quick unlock is enrolled: its wrapping is deleted.
    pub quick_unlock: bool,
    /// Attachments of owned vaults this store does not hold: they have to be
    /// fetched to be re-sealed, else they keep their old key.
    pub unreachable_attachments: Vec<(VaultId, ItemId, AttachmentId)>,
}

/// What a shared vault's rotation inside a re-key sends to the backend.
#[derive(Debug)]
pub struct SharedRekeyOut {
    vault_id: VaultId,
    roster: VaultRosterCiphertext,
    seals: Vec<(AccountId, ShareSeal)>,
    invite_seals: Vec<(InviteId, ShareSeal)>,
}

impl SharedRekeyOut {
    /// The vault.
    #[must_use]
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// The roster at the new generation (the owner's entry carries the new
    /// identity's fingerprint).
    #[must_use]
    pub const fn roster(&self) -> &VaultRosterCiphertext {
        &self.roster
    }

    /// One seal per remaining member, from the **new** identity key.
    #[must_use]
    pub fn seals(&self) -> &[(AccountId, ShareSeal)] {
        &self.seals
    }

    /// One refreshed seal per pending invite.
    #[must_use]
    pub fn invite_seals(&self) -> &[(InviteId, ShareSeal)] {
        &self.invite_seals
    }
}

/// A full re-key prepared but not applied
/// ([`UnlockedSession::prepare_rekey`]). Holds the new AK, every new vault
/// key and the new identity key (all zeroizing, not `Clone`), and exposes only
/// what the backend needs.
#[derive(Debug)]
pub struct PreparedRekey {
    session_tag: [u8; 16],
    from_epoch: u32,
    to_epoch: u32,
    old_opaque_password: OpaquePassword,
    new_opaque_password: OpaquePassword,
    new_account_key: AccountKey,
    new_params: PasswordParams,
    new_wrapped_ak: WrappedKey,
    new_secret_key: SecretKey,
    retire_secret_key: [u8; 32],
    identity: Option<NewIdentity>,
    owned_keys: Vec<VaultKey>,
    vault_rows: Vec<(VaultId, KeyGeneration, WrappedKey, u64)>,
    shared: Vec<SharedRekeyOut>,
    rewrapped: Vec<(VaultId, KeyGeneration, WrappedKey)>,
    member_rewraps: Vec<(VaultId, KeyGeneration, WrappedKey)>,
    emergency_seals: Vec<(AccountId, VaultId, KeyGeneration, ShareSeal)>,
    folder_marker: Option<[u8; FOLDER_MARKER_LEN]>,
    target_version: Option<u64>,
}

struct NewIdentity {
    isk: IdentitySecret,
    ipk: IdentityPublicKey,
    wrapped: WrappedKey,
}

impl fmt::Debug for NewIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NewIdentity").finish_non_exhaustive()
    }
}

impl PreparedRekey {
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

    /// The new public parameters (new salt, the new profile).
    #[must_use]
    pub const fn new_params(&self) -> &PasswordParams {
        &self.new_params
    }

    /// The new AK wrapped under `KEK'`.
    #[must_use]
    pub const fn new_wrapped_ak(&self) -> &WrappedKey {
        &self.new_wrapped_ak
    }

    /// The epoch the re-key moves to.
    #[must_use]
    pub const fn to_epoch(&self) -> u32 {
        self.to_epoch
    }

    /// The new identity key: `ipk` and `isk` wrapped under the new AK. `None`
    /// if the account has no identity key.
    #[must_use]
    pub fn identity(&self) -> Option<(IdentityPublicKey, WrappedKey)> {
        self.identity.as_ref().map(|i| (i.ipk, i.wrapped.clone()))
    }

    /// The new key of every owned vault: vault, generation, the key wrapped
    /// under the new AK and the backend's expected head.
    #[must_use]
    pub fn vault_keys(&self) -> &[(VaultId, KeyGeneration, WrappedKey, u64)] {
        &self.vault_rows
    }

    /// What each owned shared vault sends: the roster at the new generation
    /// and the seals from the new identity key.
    #[must_use]
    pub fn shared_rotations(&self) -> &[SharedRekeyOut] {
        &self.shared
    }

    /// The current key of every vault someone else owns, wrapped under the
    /// new AK.
    #[must_use]
    pub fn member_rewraps(&self) -> &[(VaultId, KeyGeneration, WrappedKey)] {
        &self.member_rewraps
    }

    /// Fresh emergency seals (grantee, vault, generation, seal) from the new
    /// identity key for every grant that covers a rotated vault.
    #[must_use]
    pub fn emergency_seals(&self) -> &[(AccountId, VaultId, KeyGeneration, ShareSeal)] {
        &self.emergency_seals
    }

    /// A folder re-key: the marker, MAC'd under the **current** `K_fold`
    /// (built now, because the commit drops the current AK).
    #[must_use]
    pub const fn folder_marker(&self) -> Option<&[u8; FOLDER_MARKER_LEN]> {
        self.folder_marker.as_ref()
    }
}

/// What [`UnlockedSession::commit_rekey`] did.
#[derive(Debug)]
pub struct RekeyCommitted {
    /// The new Secret Key: the app moves it from the pending keystore entry
    /// to the normal one.
    pub new_secret_key: SecretKey,
    /// The quick-unlock enrollment the commit deleted (the app deletes the
    /// platform key).
    pub removed_quick_unlock: Option<QuickUnlockEnrollment>,
    /// The security-key unlock wrappings the commit deleted (the app offers
    /// to register them again).
    pub removed_hardware_keys: Vec<HardwareKeyEnrollment>,
    /// Attachments still to re-seal ([`UnlockedSession::reseal_next_attachment`]).
    pub attachments_to_reseal: u32,
}

/// The inputs of [`UnlockedSession::adopt_remote_rekey`].
pub struct RekeyAdoption<'a> {
    /// The keys derived from the typed new password and the typed Secret Key
    /// under the backend's `params`.
    pub keys: JoinKeys,
    /// The typed (new) Secret Key.
    pub typed_secret_key: &'a SecretKey,
    /// This device's current Secret Key (the one the re-key replaced).
    pub current_secret_key: &'a SecretKey,
    /// The backend's wrapped AK.
    pub wrapped_ak: &'a WrappedKey,
    /// The backend's parameters.
    pub params: PasswordParams,
    /// The backend's account-key epoch. Hosted: exactly this device's plus
    /// one. Folder (`any_higher_epoch`): any epoch above this device's.
    pub epoch: u32,
    /// A folder adoption may jump over epochs (CRYPTO_SPEC 6.7, review H1): a
    /// re-key targets `max(own, highest seen) + 1`, and the typed Secret Key
    /// that opens a verified account file is the authenticator. Hosted
    /// adoption stays at `current + 1`.
    pub any_higher_epoch: bool,
    /// The backend's identity key (`wrapped isk`, `ipk`).
    pub identity: Option<(&'a WrappedKey, &'a IdentityPublicKey)>,
    /// The backend's own wrapping of the current key of each vault someone
    /// else owns. Each is stored as is (after a check that it opens to the
    /// held key), so the view digest the other devices compute stays the
    /// same (review M3).
    pub member_wraps: &'a [(VaultId, KeyGeneration, WrappedKey)],
}

impl fmt::Debug for RekeyAdoption<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RekeyAdoption")
            .field("epoch", &self.epoch)
            .finish_non_exhaustive()
    }
}

/// What [`UnlockedSession::adopt_remote_rekey`] did.
#[derive(Debug)]
pub struct RekeyAdopted {
    /// The new public parameters.
    pub params: PasswordParams,
    /// `OPW'` of the typed password.
    pub opaque_password: OpaquePassword,
    /// The new epoch.
    pub epoch: u32,
    /// The quick-unlock enrollment that was deleted.
    pub removed_quick_unlock: Option<QuickUnlockEnrollment>,
    /// The security-key unlock wrappings that were deleted.
    pub removed_hardware_keys: Vec<HardwareKeyEnrollment>,
}

/// The result of one [`UnlockedSession::reseal_next_attachment`] call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResealStep {
    /// Nothing is left to re-seal.
    Done,
    /// The attachment was re-sealed under a fresh ID and file key.
    Resealed {
        /// The attachment that was replaced.
        old: AttachmentId,
        /// Its replacement.
        new: AttachmentId,
    },
    /// The ciphertext does not open under the file key: the attachment was
    /// marked unreachable and keeps its old key (THREAT_MODEL R85).
    Unreadable {
        /// The vault.
        vault_id: VaultId,
        /// The attachment.
        attachment_id: AttachmentId,
    },
    /// No item refers to the attachment any more: nothing to do.
    Skipped {
        /// The attachment.
        old: AttachmentId,
    },
    /// The ciphertext is not held here: fetch it and call again with it, or
    /// call [`UnlockedSession::mark_attachment_unreachable`].
    NeedsFetch {
        /// The vault.
        vault_id: VaultId,
        /// The item.
        item_id: ItemId,
        /// The attachment to fetch.
        attachment_id: AttachmentId,
    },
}

/// An owned vault's records and metadata re-sealed under its new key.
type ResealedOwned = (
    Vec<(ItemId, RecordBody)>,
    Option<mosaic_core::VaultMetaCiphertext>,
);

/// What `build_vault_rotations` returns.
struct VaultRotations {
    keys: Vec<VaultKey>,
    rows: Vec<(VaultId, KeyGeneration, WrappedKey, u64)>,
    shared: Vec<SharedRekeyOut>,
    emergency: Vec<(AccountId, VaultId, KeyGeneration, ShareSeal)>,
}

/// The vaults of the session by what a re-key does with them.
struct Classified {
    owned: Vec<VaultId>,
    shared: Vec<VaultId>,
    member: Vec<(VaultId, AccountId)>,
    tk: Vec<VaultId>,
}

impl UnlockedSession {
    /// The account-key epoch (CRYPTO_SPEC §6.7), 1 until the first re-key.
    #[must_use]
    pub const fn ak_epoch(&self) -> u32 {
        self.ak_epoch
    }

    /// Checks the epoch a backend serves against this device's: lower is a
    /// rollback of a re-key (the caller records the `AccountRollback`
    /// finding). Equal and higher are for the caller's next step (an equal
    /// epoch must open to this AK, a higher one is adopted).
    ///
    /// # Errors
    /// [`Error::EpochRollback`].
    pub const fn check_backend_epoch(&self, offered: u32) -> Result<()> {
        if offered < self.ak_epoch {
            return Err(Error::EpochRollback {
                stored: self.ak_epoch,
                offered,
            });
        }
        Ok(())
    }

    /// The highest account version (hosted or folder) this device has seen
    /// from its backend (0 if none).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn highest_account_version(&self) -> Result<u64> {
        Ok(self.store.folder_account_version()?)
    }

    /// Refuses an account version below the highest this device has seen
    /// (CRYPTO_SPEC §9.4).
    ///
    /// # Errors
    /// [`Error::AccountVersionRollback`].
    pub fn check_account_version(&self, offered: u64) -> Result<()> {
        if offered < self.store.folder_account_version()? {
            return Err(Error::AccountVersionRollback);
        }
        Ok(())
    }

    /// Records an account version seen from the backend (never lowers it).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn note_account_version(&mut self, version: u64) -> Result<()> {
        Ok(self.store.raise_folder_account_version(version)?)
    }

    /// Every Secret Key this device replaced, as retired commitments.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn retired_secret_keys(&self) -> Result<Vec<[u8; 32]>> {
        Ok(self.store.retired_secret_keys()?)
    }

    pub(super) fn account_profile(&self) -> Result<Argon2Profile> {
        Ok(self.store.account()?.ok_or(Error::NoAccount)?.profile)
    }

    // ---- credential changes (§6.4, §6.5, §5.3) ---------------------------------

    /// The one prepare path of a master password change, a Secret Key
    /// rotation and a KDF profile upgrade, or any mix
    /// ([`CredentialChange`]): checks the current password and Secret Key,
    /// then derives the new wrapping under a new salt and both OPAQUE
    /// passwords. Nothing is stored; the commit is
    /// [`Self::commit_master_password_change`], which deletes the quick-unlock
    /// wrapping, keeps the hardware-key ones (AK is unchanged) and, for a new
    /// Secret Key, keeps the retired commitment of the old one.
    ///
    /// # Errors
    /// [`Error::UnlockFailed`] for a wrong current password or Secret Key;
    /// [`Error::SecretKeyUnchanged`] if a new Secret Key is the current or a
    /// retired one; [`Error::Core`] (`ProfileDowngrade` for a weaker
    /// profile, checked before any Argon2id work; an empty new password).
    pub fn prepare_credential_change(
        &self,
        current_password: &MasterPassword,
        current_secret_key: &SecretKey,
        change: &CredentialChange<'_>,
    ) -> Result<PreparedPasswordChange> {
        if change.profile < self.account_profile()? {
            return Err(Error::Core(mosaic_core::Error::ProfileDowngrade));
        }
        if let Some(new_sk) = change.new_secret_key {
            let retired = self.store.retired_secret_keys()?;
            if new_sk == current_secret_key || new_sk.is_retired(&self.account_id, &retired) {
                return Err(Error::SecretKeyUnchanged);
            }
        }
        let (params, old_keys) = self.verified_keys(current_password, current_secret_key)?;
        let wrapped = self
            .store
            .unlock_method(&UnlockMethod::Password)?
            .ok_or(Error::NoAccount)?
            .wrapped_account_key;
        let change_out = mosaic_core::change_credentials(
            &self.account_key,
            current_password,
            current_secret_key,
            &params,
            &wrapped,
            change,
        )
        .map_err(|e| match e {
            mosaic_core::Error::Decryption => Error::UnlockFailed,
            other => Error::Core(other),
        })?;
        Ok(PreparedPasswordChange {
            session_tag: self.session_tag,
            old_opaque_password: old_keys.opaque_password,
            new_opaque_password: change_out.opaque_password,
            new_params: change_out.params,
            new_wrapped_ak: change_out.wrapped_account_key,
            retire_secret_key: change
                .new_secret_key
                .map(|_| current_secret_key.retired_commitment(&self.account_id)),
        })
    }

    // ---- the review -------------------------------------------------------------

    /// Sorts the vaults by what a re-key does with them.
    fn classify_vaults(&self) -> Result<Classified> {
        let mut out = Classified {
            owned: Vec::new(),
            shared: Vec::new(),
            member: Vec::new(),
            tk: Vec::new(),
        };
        for (id, ov) in &self.vaults {
            if ov.protection != VaultKeyWrap::AccountKey {
                out.tk.push(*id);
                continue;
            }
            let ownership = self.ownership(*id)?;
            if !ov.available() {
                // A vault shared to this account whose current key is not
                // held (an unclaimed share, a generation not pulled yet) is
                // skipped: no re-wrap is sent for it, and the server expects
                // none (review M4). A vault that might be owned must open.
                if matches!(ownership, Ownership::Of(_)) {
                    continue;
                }
                return Err(Error::VaultUnavailable);
            }
            let owned = match ownership {
                Ownership::This => true,
                Ownership::Of(owner) => {
                    out.member.push((*id, owner));
                    false
                }
                // A vault created here before the contact book recorded
                // ownership: the same rule as `ensure_vault_owned`. On a
                // folder backend nothing can be shared, so a vault of unknown
                // ownership (a device that joined the folder) is owned; on
                // hosted it stays unknown (fail closed, CRYPTO_SPEC §6.6).
                Ownership::Unknown => {
                    if self.store.vault_key_origin(id, KeyGeneration::FIRST)?
                        == Some(crate::storage::KeyOrigin::Local)
                        || self.unknown_ownership_is_owned()?
                    {
                        true
                    } else {
                        return Err(Error::OwnerUnknown);
                    }
                }
            };
            if owned {
                if self.vault_shared(*id)? {
                    out.shared.push(*id);
                } else {
                    out.owned.push(*id);
                }
            }
        }
        Ok(out)
    }

    /// A re-key needs a clean sync (CRYPTO_SPEC §6.6 step 1): no halt other
    /// than a re-key marker, no finding, no pending revocation, and on a
    /// hosted backend every owned vault exists there.
    fn check_rekey_clean(&self, owned: &[VaultId]) -> Result<()> {
        let state = self.store.engine_state()?;
        // The re-key-pending flag is not a halt: a device that learned of a
        // re-key elsewhere may re-key again (to the highest epoch seen plus
        // one).
        if state.halt.is_some() {
            return Err(Error::RekeyNotClean);
        }
        if !self.store.vault_flags()?.is_empty()
            || !self.store.no_common_view()?.is_empty()
            || !self.store.sharing_findings()?.is_empty()
            || !self.store.pending_revocations()?.is_empty()
        {
            return Err(Error::RekeyNotClean);
        }
        if self.hosted()? {
            for v in owned {
                if !self.store.vault_sync_state(v)?.remote_created {
                    return Err(Error::RekeyNotClean);
                }
            }
        }
        Ok(())
    }

    /// The backend's rule for a vault whose owner nothing records: only a
    /// folder backend, where sharing is impossible, counts it as owned.
    fn unknown_ownership_is_owned(&self) -> Result<bool> {
        Ok(matches!(self.store.sync_config()?, Some(c) if c.kind == SyncKind::Folder))
    }

    fn hosted(&self) -> Result<bool> {
        Ok(matches!(self.store.sync_config()?, Some(c) if c.kind == SyncKind::Hosted))
    }

    /// The accounts that hold an emergency grant covering `vault`.
    fn grantees_of(&self, vault: VaultId) -> Result<BTreeSet<AccountId>> {
        let book = self.contact_book()?;
        Ok(book
            .contacts()
            .into_iter()
            .filter(|c| match book.grant_cover(&c.account_id) {
                Some(GrantCover::All) => true,
                Some(GrantCover::Vaults(v)) => v.contains(&vault),
                None => false,
            })
            .map(|c| c.account_id)
            .collect())
    }

    /// What a re-key touches (the review screen of the wizard). Refuses like
    /// [`Self::prepare_rekey`] when sync is not clean.
    ///
    /// # Errors
    /// [`Error::RekeyNotClean`], [`Error::VaultUnavailable`],
    /// [`Error::OwnerUnknown`], [`Error::Core`], [`Error::Store`].
    pub fn rekey_review(&self) -> Result<RekeyReview> {
        let class = self.classify_vaults()?;
        let mut all_owned = class.owned.clone();
        all_owned.extend(&class.shared);
        self.check_rekey_clean(&all_owned)?;
        let mut review = RekeyReview {
            owned_vaults: class.owned,
            tk_vaults: class.tk,
            ..RekeyReview::default()
        };
        for vault in class.shared {
            let members = self
                .load_roster(vault)?
                .map(|r| {
                    active(r.members())
                        .into_iter()
                        .map(|m| m.account_id)
                        .filter(|a| *a != self.account_id)
                        .collect()
                })
                .unwrap_or_default();
            review.shared_owned.push(SharedOwnedReview {
                vault_id: vault,
                members,
            });
        }
        review.member_vaults = class
            .member
            .into_iter()
            .map(|(vault_id, owner)| MemberVaultReview { vault_id, owner })
            .collect();
        let book = self.contact_book()?;
        review.grants = book
            .contacts()
            .into_iter()
            .filter(|c| book.grant_cover(&c.account_id).is_some())
            .map(|c| c.account_id)
            .collect();
        for row in self.store.unlock_methods()? {
            match row.method {
                UnlockMethod::QuickUnlock(_) => review.quick_unlock = true,
                UnlockMethod::HardwareKey(_) => {
                    review.hardware_keys = review.hardware_keys.saturating_add(1);
                }
                UnlockMethod::Password => {}
            }
        }
        let mut rotated: Vec<VaultId> = review.owned_vaults.clone();
        rotated.extend(review.shared_owned.iter().map(|v| v.vault_id));
        review.unreachable_attachments = self.missing_attachments(&rotated)?;
        Ok(review)
    }

    /// The attachments of `vaults` whose ciphertext this store does not hold.
    fn missing_attachments(
        &self,
        vaults: &[VaultId],
    ) -> Result<Vec<(VaultId, ItemId, AttachmentId)>> {
        let mut out = Vec::new();
        for (a, vault, item) in self.attachment_rows(vaults)? {
            if self.store.attachment_info(&a)?.is_none() {
                out.push((vault, item, a));
            }
        }
        Ok(out)
    }

    // ---- prepare ---------------------------------------------------------------

    /// The marker (CRYPTO_SPEC §10.1) MAC'd under the current `K_fold`.
    fn build_folder_marker(
        &self,
        from_epoch: u32,
        to_epoch: u32,
        plan: &FolderRekeyPlan,
    ) -> Result<[u8; FOLDER_MARKER_LEN]> {
        if plan.to_account_version == 0 {
            return Err(Error::Core(mosaic_core::Error::Malformed));
        }
        let mut msg = Vec::with_capacity(FOLDER_MARKER_LEN);
        msg.extend_from_slice(b"MREK");
        msg.push(1);
        msg.extend_from_slice(self.account_id.as_bytes());
        msg.extend_from_slice(&from_epoch.to_be_bytes());
        msg.extend_from_slice(&to_epoch.to_be_bytes());
        msg.extend_from_slice(&plan.to_account_version.to_be_bytes());
        msg.extend_from_slice(self.install.as_bytes());
        msg.extend_from_slice(&plan.created_at_ms.to_be_bytes());
        let mac = sync_auth::folder_rekey_mac(&self.folder_key()?, &msg)?;
        msg.extend_from_slice(&mac);
        msg.try_into()
            .map_err(|_| Error::Core(mosaic_core::Error::Malformed))
    }

    /// The new identity key pair, wrapped under the new AK, if the account
    /// has an identity key.
    fn new_identity(&self, new_ak: &AccountKey) -> Result<Option<NewIdentity>> {
        if self.store.identity_key()?.is_none() {
            return Ok(None);
        }
        let (isk, ipk) = IdentitySecret::generate()?;
        let wrapped = isk.wrap(new_ak, &self.account_id)?;
        Ok(Some(NewIdentity { isk, ipk, wrapped }))
    }

    /// One owned shared vault's part of the re-key: the roster at `g+1` with
    /// the owner's new fingerprint, and the seals from the new identity key.
    fn build_shared_rekey(
        &self,
        vault: VaultId,
        plan: &RotationPlan,
        key: &VaultKey,
        identity: &NewIdentity,
        emergency: &mut Vec<(AccountId, VaultId, KeyGeneration, ShareSeal)>,
    ) -> Result<SharedRekeyOut> {
        if !plan.revoked.is_empty() {
            return Err(Error::PlanMismatch);
        }
        let roster = self.load_roster(vault)?.ok_or(Error::NoRoster)?;
        self.check_rotation_plan(&roster, plan)?;
        let grantees: BTreeSet<AccountId> = plan.grants.iter().map(|(a, _)| *a).collect();
        if grantees.len() != plan.grants.len() || grantees != self.grantees_of(vault)? {
            return Err(Error::PlanMismatch);
        }
        let generation = key.generation();
        let mut new_roster = roster.advance(generation)?;
        let own_fpr = Fingerprint::of(&self.account_id, &identity.ipk);
        new_roster.edit(self.device, now()).set_member(
            &self.account_id,
            Role::Owner,
            generation,
            &own_fpr,
            &self.own_email()?,
        )?;
        let seal_to = |a: &AccountId, ipk: &IdentityPublicKey| {
            seal_vault_key(key, &vault, &self.account_id, &identity.isk, a, ipk)
        };
        let seals = plan
            .remaining
            .iter()
            .map(|(a, ipk)| Ok((*a, seal_to(a, ipk)?)))
            .collect::<Result<Vec<_>>>()?;
        let invite_seals = plan
            .invites
            .iter()
            .map(|(i, a, ipk)| Ok((*i, seal_to(a, ipk)?)))
            .collect::<Result<Vec<_>>>()?;
        for (a, ipk) in &plan.grants {
            emergency.push((
                *a,
                vault,
                generation,
                seal_emergency_vault_key(key, &vault, &self.account_id, &identity.isk, a, ipk)?,
            ));
        }
        Ok(SharedRekeyOut {
            vault_id: vault,
            roster: new_roster.seal(key, &vault)?,
            seals,
            invite_seals,
        })
    }

    /// Every held vault key (every generation) wrapped under `new_ak`.
    fn rewrap_held(
        &self,
        new_ak: &AccountKey,
    ) -> Result<Vec<(VaultId, KeyGeneration, WrappedKey)>> {
        let mut out = Vec::new();
        for (id, ov) in &self.vaults {
            if ov.protection != VaultKeyWrap::AccountKey {
                continue;
            }
            for (g, vk) in &ov.keys {
                out.push((*id, *g, vk.wrap(new_ak, &self.account_id, id)?));
            }
        }
        Ok(out)
    }

    /// The new key, wrapped key, roster and seals of every owned vault.
    fn build_vault_rotations(
        &self,
        owned_all: &[VaultId],
        plan: &RekeyPlan,
        new_ak: &AccountKey,
        identity: Option<&NewIdentity>,
    ) -> Result<VaultRotations> {
        let heads: BTreeMap<VaultId, u64> = plan.heads.iter().copied().collect();
        let mut out = VaultRotations {
            keys: Vec::new(),
            rows: Vec::new(),
            shared: Vec::new(),
            emergency: Vec::new(),
        };
        for vault in owned_all {
            let next = self.vault(*vault)?.generation.next()?;
            let key = VaultKey::generate(next)?;
            let wrapped = key.wrap(new_ak, &self.account_id, vault)?;
            if let Some((_, rplan)) = plan.shared.iter().find(|(v, _)| v == vault) {
                let id = identity.ok_or(Error::NoIdentity)?;
                let shared =
                    self.build_shared_rekey(*vault, rplan, &key, id, &mut out.emergency)?;
                out.shared.push(shared);
            }
            out.rows.push((
                *vault,
                next,
                wrapped,
                heads.get(vault).copied().unwrap_or(0),
            ));
            out.keys.push(key);
        }
        Ok(out)
    }

    /// Prepares a full re-key (CRYPTO_SPEC §6.6 steps 1 and 2): a new AK, a
    /// new identity key and a new vault key one generation up for every owned
    /// vault, the roster and seals of every owned shared vault from the
    /// **new** identity key, the re-wrapped keys of every vault someone else
    /// owns, fresh emergency seals, and (folder) the marker under the current
    /// `K_fold`. Nothing is stored; offer the result to the backend, then call
    /// [`Self::commit_rekey`].
    ///
    /// # Errors
    /// [`Error::UnlockFailed`] for a wrong current password or Secret Key;
    /// [`Error::PasswordUnchanged`], [`Error::SecretKeyUnchanged`] (the new
    /// one is the current or a retired one); [`Error::RekeyNotClean`];
    /// [`Error::PlanMismatch`], [`Error::MemberKeyMismatch`],
    /// [`Error::ContactUnverified`] if the plan does not cover the owned
    /// shared vaults as the roster and the pins say;
    /// [`Error::EpochExhausted`] (the target `max(own epoch,
    /// plan.seen_epoch) + 1` overflows), [`Error::EpochJumpTooLarge`] (the
    /// target is more than `MAX_EPOCH_JUMP` above the own epoch, ADR-0064;
    /// the sync engine never plans one); [`Error::VaultUnavailable`],
    /// [`Error::OwnerUnknown`] (hosted only), [`Error::NoIdentity`];
    /// [`Error::Core`] (`ProfileDowngrade`, an empty password);
    /// [`Error::Store`].
    pub fn prepare_rekey(&self, req: RekeyRequest<'_>) -> Result<PreparedRekey> {
        let RekeyRequest {
            current_password,
            current_secret_key,
            new_password,
            new_secret_key,
            profile,
            plan,
        } = req;
        let to_epoch = self
            .ak_epoch
            .max(plan.seen_epoch)
            .checked_add(1)
            .ok_or(Error::EpochExhausted)?;
        if to_epoch - self.ak_epoch > MAX_EPOCH_JUMP {
            return Err(Error::EpochJumpTooLarge {
                stored: self.ak_epoch,
                offered: to_epoch,
            });
        }
        if profile < self.account_profile()? {
            return Err(Error::Core(mosaic_core::Error::ProfileDowngrade));
        }
        let retired = self.store.retired_secret_keys()?;
        if new_secret_key == current_secret_key
            || new_secret_key.is_retired(&self.account_id, &retired)
        {
            return Err(Error::SecretKeyUnchanged);
        }
        let class = self.classify_vaults()?;
        let mut owned_all = class.owned.clone();
        owned_all.extend(&class.shared);
        self.check_rekey_clean(&owned_all)?;
        let (params, old_keys) = self.verified_keys(current_password, current_secret_key)?;
        // The new password must not open the current wrapping (with the
        // current Secret Key): one more derivation under the current
        // parameters.
        match self.verified_keys(new_password, current_secret_key) {
            Ok(_) => return Err(Error::PasswordUnchanged),
            Err(Error::UnlockFailed) => {}
            Err(e) => return Err(e),
        }
        let rekeyed = mosaic_core::rekey_account(&params, new_password, new_secret_key, profile)?;
        let new_ak = &rekeyed.account_key;
        let identity = self.new_identity(new_ak)?;
        let plan_vaults: BTreeSet<VaultId> = plan.shared.iter().map(|(v, _)| *v).collect();
        if plan_vaults.len() != plan.shared.len()
            || plan_vaults != class.shared.iter().copied().collect::<BTreeSet<_>>()
        {
            return Err(Error::PlanMismatch);
        }
        let rot = self.build_vault_rotations(&owned_all, &plan, new_ak, identity.as_ref())?;
        let rewrapped = self.rewrap_held(new_ak)?;
        let member_ids: BTreeSet<VaultId> = class.member.iter().map(|(v, _)| *v).collect();
        let member_rewraps = rewrapped
            .iter()
            .filter(|(v, g, _)| {
                member_ids.contains(v) && self.vaults.get(v).is_some_and(|ov| ov.generation == *g)
            })
            .cloned()
            .collect();
        let folder_marker = plan
            .folder
            .as_ref()
            .map(|f| self.build_folder_marker(self.ak_epoch, to_epoch, f))
            .transpose()?;
        // An owned copy of the new Secret Key for the commit to hand back.
        let new_secret_key_owned = SecretKey::parse(new_secret_key.to_display()?.expose_secret())?;
        Ok(PreparedRekey {
            session_tag: self.session_tag,
            from_epoch: self.ak_epoch,
            to_epoch,
            old_opaque_password: old_keys.opaque_password,
            new_opaque_password: rekeyed.opaque_password,
            new_wrapped_ak: rekeyed.wrapped_account_key,
            new_params: rekeyed.params,
            new_account_key: rekeyed.account_key,
            new_secret_key: new_secret_key_owned,
            retire_secret_key: current_secret_key.retired_commitment(&self.account_id),
            identity,
            owned_keys: rot.keys,
            vault_rows: rot.rows,
            shared: rot.shared,
            rewrapped,
            member_rewraps,
            emergency_seals: rot.emergency,
            folder_marker,
            target_version: plan.folder.map(|f| f.to_account_version),
        })
    }

    // ---- commit ----------------------------------------------------------------

    /// The contact book re-sealed under the new account key's `K_contacts`.
    fn reseal_contacts(&self, new_ak: &AccountKey) -> Result<Option<ContactsCiphertext>> {
        if self.store.contacts_doc()?.is_none() {
            return Ok(None);
        }
        let book = self.contact_book()?;
        Ok(Some(book.seal(&new_ak.contacts_key()?, &self.account_id)?))
    }

    /// The attachments of the vaults in `ids`, live or in the trash.
    fn attachment_rows(&self, ids: &[VaultId]) -> Result<Vec<(AttachmentId, VaultId, ItemId)>> {
        let mut rows = Vec::new();
        for vault in ids {
            for (item_id, item) in self
                .vault(*vault)?
                .items
                .iter()
                .filter(|(_, i)| !i.is_deleted())
            {
                rows.extend(
                    item.attachments()
                        .into_iter()
                        .map(|a| (a.attachment_id, *vault, *item_id)),
                );
            }
        }
        Ok(rows)
    }

    /// Re-seals every readable record and the metadata of an owned vault under
    /// its new key.
    fn reseal_owned(&self, vault: VaultId, key: &VaultKey) -> Result<ResealedOwned> {
        let ov = self.vault(vault)?;
        let records = ov
            .items
            .iter()
            .map(|(id, item)| Ok((*id, seal_record(item, key, &vault, id)?)))
            .collect::<Result<Vec<_>>>()?;
        let meta = ov.meta.as_ref().map(|m| m.seal(key, &vault)).transpose()?;
        Ok((records, meta))
    }

    /// Applies a prepared re-key after the backend accepted it (hosted) or
    /// right before the folder objects are written (CRYPTO_SPEC §6.6 step 3),
    /// in ONE store transaction: the new parameters, password wrapping and
    /// epoch, the old salt retired, the Secret Key replaced retired, every
    /// held vault key of every generation re-wrapped, the new key and the
    /// re-sealed records, metadata and roster of every owned vault, the new
    /// identity key, the contact book re-sealed, the quick-unlock and every
    /// hardware-key wrapping deleted (both returned), the peer state cleared,
    /// one re-seal job per attachment of an owned vault, and the journal at
    /// phase 5. TK vaults are not touched. The session then holds the new AK.
    ///
    /// # Errors
    /// [`Error::PreparedStale`] if it was prepared in an earlier unlocked
    /// session or the epoch moved since; [`Error::Store`]
    /// (`RekeyIncomplete` if a vault key appeared since the prepare,
    /// `EpochMismatch`, and the errors of the writes), [`Error::Core`]. On an
    /// error nothing changed.
    pub fn commit_rekey(&mut self, p: PreparedRekey) -> Result<RekeyCommitted> {
        if p.session_tag != self.session_tag || p.from_epoch != self.ak_epoch {
            return Err(Error::PreparedStale);
        }
        let hosted = self.hosted()?;
        let mut owned_ids: Vec<VaultId> = p.vault_rows.iter().map(|(v, ..)| *v).collect();
        owned_ids.sort();
        let mut sealed = Vec::new();
        for ((vault, ..), key) in p.vault_rows.iter().zip(&p.owned_keys) {
            sealed.push(self.reseal_owned(*vault, key)?);
        }
        let attachments = self.attachment_rows(&owned_ids)?;
        let contacts = self.reseal_contacts(&p.new_account_key)?;
        let identity_write = p.identity.as_ref().map(|i| IdentityWrite {
            wrapped_isk: &i.wrapped,
            ipk: &i.ipk,
            published: hosted,
        });
        let vaults: Vec<RotatedVaultWrite<'_>> = p
            .vault_rows
            .iter()
            .zip(&sealed)
            .map(
                |((vault, generation, wrapped, _), (records, meta))| RotatedVaultWrite {
                    vault_id: *vault,
                    generation: *generation,
                    wrapped,
                    records,
                    meta: meta.as_ref(),
                    roster: p
                        .shared
                        .iter()
                        .find(|s| s.vault_id == *vault)
                        .map(|s| &s.roster),
                },
            )
            .collect();
        let removed = self.store.commit_rekey(&RekeyWrite {
            switch: EpochSwitch {
                params: &p.new_params,
                password_wrapping: &p.new_wrapped_ak,
                to_epoch: p.to_epoch,
                rewrapped_keys: &p.rewrapped,
                identity: identity_write,
                contacts: contacts.as_ref(),
                retire_secret_key: &p.retire_secret_key,
                hold: &[],
            },
            vaults: &vaults,
            accepted_by_backend: hosted,
            attachments: &attachments,
            folder_marker: p.folder_marker.as_ref(),
            target_version: p.target_version,
        })?;
        drop(vaults);
        let PreparedRekey {
            to_epoch,
            new_account_key,
            new_secret_key,
            identity,
            owned_keys,
            vault_rows,
            ..
        } = p;
        // The store holds the new epoch: the session follows.
        for ((vault, generation, ..), key) in vault_rows.into_iter().zip(owned_keys) {
            if let Some(ov) = self.vaults.get_mut(&vault) {
                ov.keys.insert(generation, key);
                ov.generation = generation;
            }
        }
        self.account_key = new_account_key;
        if let Some(i) = identity {
            self.identity_key = Some(super::sharing::Identity {
                isk: i.isk,
                ipk: i.ipk,
            });
        }
        self.ak_epoch = to_epoch;
        self.created_handed.clear();
        self.contacts_handed = None;
        let (quick, hardware) = split_removed(removed);
        Ok(RekeyCommitted {
            new_secret_key,
            removed_quick_unlock: quick,
            removed_hardware_keys: hardware,
            attachments_to_reseal: u32::try_from(attachments.len()).unwrap_or(u32::MAX),
        })
    }

    // ---- adoption --------------------------------------------------------------

    /// The backend's wrapping for each member vault's current generation,
    /// checked to open to the held key; every other held key is re-wrapped
    /// here.
    fn adoption_wraps(
        &self,
        new_ak: &AccountKey,
        member_wraps: &[(VaultId, KeyGeneration, WrappedKey)],
    ) -> Result<Vec<(VaultId, KeyGeneration, WrappedKey)>> {
        let mut used = BTreeSet::new();
        let mut out = Vec::new();
        for (id, ov) in &self.vaults {
            if ov.protection != VaultKeyWrap::AccountKey {
                continue;
            }
            let member = matches!(self.ownership(*id)?, Ownership::Of(_));
            for (g, vk) in &ov.keys {
                let served = member_wraps
                    .iter()
                    .find(|(v, wg, _)| v == id && wg == g && *g == ov.generation && member);
                let wrapped = if let Some((_, _, w)) = served {
                    self.check_served_wrap(new_ak, w, *id, *g, vk)?;
                    used.insert((*id, *g));
                    w.clone()
                } else {
                    vk.wrap(new_ak, &self.account_id, id)?
                };
                out.push((*id, *g, wrapped));
            }
        }
        // A served wrapping for a key that is not a held member current key.
        if member_wraps
            .iter()
            .any(|(v, g, _)| !used.contains(&(*v, *g)))
        {
            return Err(Error::PlanMismatch);
        }
        Ok(out)
    }

    /// The backend's wrapping must open (under the new AK) to the held key.
    fn check_served_wrap(
        &self,
        new_ak: &AccountKey,
        wrapped: &WrappedKey,
        vault: VaultId,
        generation: KeyGeneration,
        held: &VaultKey,
    ) -> Result<()> {
        let opened = wrapped
            .unwrap_vault_key(new_ak, &self.account_id, &vault, generation)
            .map_err(|_| Error::UnlockFailed)?;
        // `VaultKey` equality is constant time.
        if opened == *held {
            Ok(())
        } else {
            Err(Error::UnlockFailed)
        }
    }

    /// The checks of an adoption that need no key material of the backend.
    fn check_adoption(
        &self,
        epoch: u32,
        any_higher: bool,
        typed: &SecretKey,
        current: &SecretKey,
        salt: &mosaic_core::AccountSalt,
    ) -> Result<()> {
        if epoch < self.ak_epoch {
            return Err(Error::EpochRollback {
                stored: self.ak_epoch,
                offered: epoch,
            });
        }
        if epoch == self.ak_epoch {
            return Err(Error::UnlockFailed);
        }
        if !any_higher && self.ak_epoch.checked_add(1) != Some(epoch) {
            return Err(Error::EpochNotNext {
                stored: self.ak_epoch,
                offered: epoch,
            });
        }
        // Above the own epoch here, so the subtraction cannot underflow.
        if epoch - self.ak_epoch > MAX_EPOCH_JUMP {
            return Err(Error::EpochJumpTooLarge {
                stored: self.ak_epoch,
                offered: epoch,
            });
        }
        if self.store.salt_retired(salt)? {
            return Err(Error::PasswordRollback);
        }
        let retired = self.store.retired_secret_keys()?;
        if typed == current || typed.is_retired(&self.account_id, &retired) {
            return Err(Error::SecretKeyUnchanged);
        }
        Ok(())
    }

    /// Adopts a re-key another device made (CRYPTO_SPEC §6.7). Requires
    /// exactly this device's epoch plus one (lower is a rollback, equal is
    /// refused, anything else is not adopted; with `any_higher_epoch` any
    /// epoch up to `MAX_EPOCH_JUMP` above), a salt this device did not
    /// retire, a wrapping that opens under the typed keys, and a typed Secret
    /// Key that is neither the current one nor a retired one (constant time).
    /// There is no adoption at an equal epoch. One store transaction like the
    /// local part of [`Self::commit_rekey`], without new vault keys: the
    /// identity is replaced from the backend's wrapped `isk` after the
    /// derived `ipk` equals the served one, the contact book is re-sealed,
    /// quick and hardware unlock are deleted, peer state is cleared, history
    /// generations are re-wrapped here and the backend's own wrapping of each
    /// member vault's current key is stored as served.
    ///
    /// # Errors
    /// [`Error::EpochRollback`], [`Error::UnlockFailed`] (equal epoch, wrong
    /// keys, parameters that do not belong to the keys, a served wrapping that
    /// does not open to the held key), [`Error::EpochNotNext`],
    /// [`Error::EpochJumpTooLarge`],
    /// [`Error::PasswordRollback`], [`Error::SecretKeyUnchanged`],
    /// [`Error::IdentityMismatch`], [`Error::PlanMismatch`], [`Error::Store`]
    /// (`AccountMismatch`, `ProfileDowngrade`, `RekeyIncomplete`).
    pub fn adopt_remote_rekey(&mut self, a: RekeyAdoption<'_>) -> Result<RekeyAdopted> {
        let RekeyAdoption {
            keys,
            typed_secret_key,
            current_secret_key,
            wrapped_ak,
            params,
            epoch,
            any_higher_epoch,
            identity,
            member_wraps,
        } = a;
        if keys.params != params {
            return Err(Error::UnlockFailed);
        }
        self.check_adoption(
            epoch,
            any_higher_epoch,
            typed_secret_key,
            current_secret_key,
            &params.salt,
        )?;
        let new_ak = wrapped_ak
            .unwrap_account_key(&keys.keys.kek, &params.account_id, &UnlockMethod::Password)
            .map_err(|e| match e {
                mosaic_core::Error::Decryption => Error::UnlockFailed,
                other => Error::Core(other),
            })?;
        let fresh;
        let identity_write = match identity {
            Some((wrapped, ipk)) => {
                let isk = wrapped
                    .unwrap_identity(&new_ak, &self.account_id)
                    .map_err(|_| Error::IdentityMismatch)?;
                if isk.public_key()? != *ipk {
                    return Err(Error::IdentityMismatch);
                }
                Some(IdentityWrite {
                    wrapped_isk: wrapped,
                    ipk,
                    published: true,
                })
            }
            // The backend has no identity key (a folder, or one never
            // published): a key this device had under the old AK is replaced
            // by a fresh unpublished one.
            None if self.store.identity_key()?.is_some() => {
                let (isk, ipk) = IdentitySecret::generate()?;
                let wrapped = isk.wrap(&new_ak, &self.account_id)?;
                fresh = (isk, ipk, wrapped);
                Some(IdentityWrite {
                    wrapped_isk: &fresh.2,
                    ipk: &fresh.1,
                    published: false,
                })
            }
            None => None,
        };
        let rewrapped = self.adoption_wraps(&new_ak, member_wraps)?;
        let contacts = self.reseal_contacts(&new_ak)?;
        let retire = current_secret_key.retired_commitment(&self.account_id);
        // Every owned vault a backend has seen is held at the generation this
        // device has until the re-key's next one arrives (review M5). A vault
        // no backend ever saw is not held: none of its ciphertext or
        // wrappings left the device, its key is re-wrapped under the new AK
        // above and it syncs normally (ADR-0064).
        let mut hold = Vec::new();
        for (id, ov) in &self.vaults {
            if ov.protection == VaultKeyWrap::AccountKey
                && !matches!(self.ownership(*id)?, Ownership::Of(_))
                && self.store.vault_backend_known(id)?
            {
                hold.push((*id, ov.generation));
            }
        }
        let removed = self.store.adopt_rekey(&EpochSwitch {
            params: &params,
            password_wrapping: wrapped_ak,
            to_epoch: epoch,
            rewrapped_keys: &rewrapped,
            identity: identity_write,
            contacts: contacts.as_ref(),
            retire_secret_key: &retire,
            hold: &hold,
        })?;
        self.account_key = new_ak;
        self.ak_epoch = epoch;
        // The identity the store now holds, opened under the new AK.
        self.identity_key =
            super::sharing::load_identity(&self.store, &self.account_key, &self.account_id);
        self.created_handed.clear();
        self.contacts_handed = None;
        let (quick, hardware) = split_removed(removed);
        Ok(RekeyAdopted {
            params,
            opaque_password: keys.keys.opaque_password,
            epoch,
            removed_quick_unlock: quick,
            removed_hardware_keys: hardware,
        })
    }

    // ---- attachments -----------------------------------------------------------

    /// Re-seals the next attachment of the queue under a fresh attachment ID
    /// and file key in the same vault (CRYPTO_SPEC §7.3, ADR-0017), updating
    /// the item reference, queueing the new blob and the item for upload and
    /// recording the old ID for deletion once the referencing record is
    /// accepted: ONE store transaction, so a crash loses at most this step.
    /// A ciphertext this store does not hold has to be fetched: the answer is
    /// [`ResealStep::NeedsFetch`], and the next call carries the bytes
    /// (`fetched`, authenticated under the old file key before anything is
    /// stored). A blob that cannot be fetched is marked with
    /// [`Self::mark_attachment_unreachable`].
    ///
    /// # Errors
    /// [`Error::Store`] (`AttachmentsUnavailable` for an in-memory store).
    pub fn reseal_next_attachment(&mut self, fetched: Option<&[u8]>) -> Result<ResealStep> {
        let Some(row) = self.store.next_reseal_todo()? else {
            return Ok(ResealStep::Done);
        };
        let ResealRow {
            old_id,
            vault_id,
            item_id,
            ..
        } = row;
        let reference = self
            .vaults
            .get(&vault_id)
            .and_then(|ov| ov.items.get(&item_id))
            .filter(|i| !i.is_deleted())
            .and_then(|i| {
                i.attachments()
                    .into_iter()
                    .find(|a| a.attachment_id == old_id)
            });
        let Some(old_ref) = reference else {
            self.store.mark_reseal_obsolete(&old_id)?;
            return Ok(ResealStep::Skipped { old: old_id });
        };
        let ct = if self.store.attachment_info(&old_id)?.is_some() {
            self.store.attachment(&old_id)?
        } else if let Some(bytes) = fetched {
            bytes.to_vec()
        } else {
            return Ok(ResealStep::NeedsFetch {
                vault_id,
                item_id,
                attachment_id: old_id,
            });
        };
        // A blob that does not open under its file key cannot be re-sealed:
        // it is recorded as unreachable (listed, keeps its old key) and
        // skipped, so it never blocks the job (review L4).
        let Ok(plain) = open_attachment(&old_ref.key, &vault_id, &item_id, &old_id, &ct) else {
            self.store.mark_reseal_unreachable(&old_id)?;
            return Ok(ResealStep::Unreadable {
                vault_id,
                attachment_id: old_id,
            });
        };
        let new_ref = AttachmentRef::new(
            AttachmentId::new_v7()?,
            Text::new(old_ref.file_name.as_str()),
            Text::new(old_ref.mime.as_str()),
            old_ref.size,
            AttachmentKey::generate()?,
        );
        let new_ct = seal_attachment(
            &new_ref.key,
            &vault_id,
            &item_id,
            &new_ref.attachment_id,
            plain.expose_secret(),
        )?;
        drop(plain);
        let (copy, body) = self.reseal_item_copy(vault_id, item_id, &old_id, &new_ref)?;
        self.store
            .stage_attachment_file(&new_ref.attachment_id, &new_ct)?;
        let swapped = self.store.swap_attachment(&AttachmentSwap {
            vault_id,
            item_id,
            record: &body,
            old_id,
            new_id: new_ref.attachment_id,
            new_len: u64::try_from(new_ct.len()).unwrap_or(u64::MAX),
        });
        if let Err(e) = swapped {
            self.store
                .discard_staged_attachment_files(&[new_ref.attachment_id])?;
            return Err(e.into());
        }
        self.index.update(vault_id, item_id, &copy);
        if let Some(ov) = self.vaults.get_mut(&vault_id) {
            ov.items.insert(item_id, copy);
        }
        Ok(ResealStep::Resealed {
            old: old_id,
            new: new_ref.attachment_id,
        })
    }

    /// The item with `old` replaced by `new_ref`, and it sealed under the
    /// vault's current key.
    fn reseal_item_copy(
        &self,
        vault_id: VaultId,
        item_id: ItemId,
        old: &AttachmentId,
        new_ref: &AttachmentRef,
    ) -> Result<(mosaic_core::Item, RecordBody)> {
        let ov: &OpenVault = self.vault(vault_id)?;
        let item = ov.items.get(&item_id).ok_or(Error::UnknownItem)?;
        let mut copy = dup_item(item)?;
        {
            let mut edit = copy.edit(self.device, now());
            edit.remove_attachment(old)?;
            edit.set_attachment(new_ref)?;
        }
        let body = seal_record(&copy, ov.current_key()?, &vault_id, &item_id)?;
        Ok((copy, body))
    }

    /// Records that an attachment is neither held nor fetchable: it keeps its
    /// old file key and is listed by [`Self::unreachable_attachments`]
    /// (THREAT_MODEL R85).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_attachment_unreachable(&mut self, id: AttachmentId) -> Result<bool> {
        Ok(self.store.mark_reseal_unreachable(&id)?)
    }

    /// The attachments of the last re-seal that keep their old file key.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn unreachable_attachments(&self) -> Result<Vec<ResealRow>> {
        Ok(self
            .store
            .reseal_rows()?
            .into_iter()
            .filter(|r| r.state == ResealState::Unreachable)
            .collect())
    }

    /// Queues the attachment re-seal for one vault's key rotation (the
    /// optional step of CRYPTO_SPEC §7.1) and opens a vault job at phase 6.
    /// Returns how many attachments are queued.
    ///
    /// # Errors
    /// [`Error::RotationInProgress`], [`Error::UnknownVault`],
    /// [`Error::WrongVaultProtection`], [`Error::Store`].
    pub fn queue_vault_attachment_reseal(&mut self, vault: VaultId) -> Result<u32> {
        let ov = self.vault(vault)?;
        if ov.protection != VaultKeyWrap::AccountKey {
            return Err(Error::WrongVaultProtection);
        }
        let generation = ov.generation;
        let rows = self.attachment_rows(&[vault])?;
        self.store.queue_attachment_reseal(generation, &rows)?;
        Ok(u32::try_from(rows.len()).unwrap_or(u32::MAX))
    }

    /// The vaults shared to this account, with the generation held, that
    /// this device can re-wrap on adoption. Needs no clean sync; a vault of
    /// unknown ownership or without its current key is left out (review M1).
    #[must_use]
    pub fn member_vault_generations(&self) -> Vec<(VaultId, KeyGeneration)> {
        self.vaults
            .iter()
            .filter(|(id, ov)| {
                ov.protection == VaultKeyWrap::AccountKey
                    && ov.available()
                    && matches!(self.ownership(**id), Ok(Ownership::Of(_)))
            })
            .map(|(id, ov)| (*id, ov.generation))
            .collect()
    }

    /// Whether pushes of `vault` wait for the next key generation after an
    /// adopted re-key (review M5). Every genuine re-key rotates every owned
    /// vault, so the hold ends when the key of the next generation arrives.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn rekey_held(&self, vault: VaultId) -> Result<bool> {
        Ok(self.store.rekey_held(&vault)?)
    }

    /// Raises the account-key epoch without a key change. Never lowers it.
    /// Test support only (feature `test-util`, ADR-0064): a join stores the
    /// backend's epoch through `join_account_with_keys_at`, and nothing else
    /// may move the epoch without the matching key.
    ///
    /// # Errors
    /// [`Error::Store`].
    #[cfg(any(test, feature = "test-util"))]
    pub fn raise_epoch_to(&mut self, epoch: u32) -> Result<()> {
        self.store.raise_ak_epoch(epoch)?;
        self.ak_epoch = self.ak_epoch.max(epoch);
        Ok(())
    }

    /// Queues the attachments of every owned vault for the re-key job of a
    /// device that adopted its own re-key after a crash (the prepared queue
    /// died, review M2) and moves the job to phase 6. Returns how many are
    /// queued.
    ///
    /// # Errors
    /// [`Error::Store`] (`NoRotationJob`).
    pub fn queue_adoption_reseal(&mut self) -> Result<u32> {
        let mut ids = Vec::new();
        for (id, ov) in &self.vaults {
            if ov.protection == VaultKeyWrap::AccountKey
                && !matches!(self.ownership(*id)?, Ownership::Of(_))
            {
                ids.push(*id);
            }
        }
        let rows = self.attachment_rows(&ids)?;
        self.store.queue_rekey_job_reseal(&rows)?;
        Ok(u32::try_from(rows.len()).unwrap_or(u32::MAX))
    }

    /// Drops a rotation job whose local part is committed (phase 5 or later,
    /// and for a re-key the store at the target epoch), after the user
    /// confirmed (review L4, ADR-0064): attachments still to re-seal keep
    /// their old key and are listed, and the job stops blocking a change of
    /// backend. Phases 1 to 4 are resume-only.
    ///
    /// # Errors
    /// [`Error::Store`] (`NoRotationJob`, `RotationTooFar` before the commit).
    pub fn abandon_committed_rotation_job(&mut self) -> Result<()> {
        Ok(self.store.abandon_committed_rotation_job()?)
    }

    // ---- the journal -----------------------------------------------------------

    /// The rotation journal row, if a job is open.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn rotation_job(&self) -> Result<Option<RotationJob>> {
        Ok(self.store.rotation_job()?)
    }

    /// Opens a job at phase 1 (kinds Secret Key, re-key, profile). The epochs
    /// are filled in for a re-key.
    ///
    /// # Errors
    /// [`Error::RotationInProgress`], [`Error::EpochExhausted`],
    /// [`Error::Store`] (`InvalidRotationPhase` for a vault job: it is opened
    /// by [`Self::queue_vault_attachment_reseal`]).
    pub fn begin_rotation_job(
        &mut self,
        kind: RotationKind,
        target_version: Option<u64>,
    ) -> Result<()> {
        let rekey = kind == RotationKind::Rekey;
        let to_epoch = if rekey {
            Some(self.ak_epoch.checked_add(1).ok_or(Error::EpochExhausted)?)
        } else {
            None
        };
        Ok(self.store.begin_rotation_job(&NewRotationJob {
            kind,
            phase: RotationPhase::Prepared,
            from_epoch: rekey.then_some(self.ak_epoch),
            to_epoch,
            target_version,
        })?)
    }

    /// Moves the job to `phase` (the order is not enforced: a resume may
    /// return to phase 1, and a folder re-key records phase 4 after the
    /// local commit).
    ///
    /// # Errors
    /// [`Error::Store`] (`NoRotationJob`, `InvalidRotationPhase`).
    pub fn set_rotation_phase(&mut self, phase: RotationPhase) -> Result<()> {
        Ok(self.store.set_rotation_phase(phase)?)
    }

    /// Closes the job once nothing is left: no attachment to re-seal and the
    /// local commit done. The sync layer calls it when the outbox of every
    /// rotated vault is empty.
    ///
    /// # Errors
    /// [`Error::Store`] (`NoRotationJob`, `RotationNotFinished`).
    pub fn finish_rotation_job(&mut self) -> Result<()> {
        Ok(self.store.finish_rotation_job()?)
    }

    /// Drops a job the backend has not seen (phase 1). The app deletes the
    /// pending Secret Key.
    ///
    /// # Errors
    /// [`Error::Store`] (`NoRotationJob`, `RotationTooFar`).
    pub fn abandon_rotation_job(&mut self) -> Result<()> {
        Ok(self.store.abandon_rotation_job()?)
    }

    /// Whether a verified re-key marker named this device's epoch.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn rekey_pending(&self) -> Result<bool> {
        Ok(self.store.rekey_pending()?)
    }

    /// Sets or clears the re-key-pending flag (adopting a re-key clears it).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn set_rekey_pending(&mut self, pending: bool) -> Result<()> {
        Ok(self.store.set_rekey_pending(pending)?)
    }

    // ---- folder floors ---------------------------------------------------------

    /// The pre-re-key own-segment floors of the folder backend.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn folder_rekey_floors(&self) -> Result<Vec<FolderRekeyFloor>> {
        Ok(self.store.folder_rekey_floors()?)
    }

    /// Notes that own segments of `vault` up to `floor` were sealed under the
    /// previous epoch (a device that adopted a re-key).
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::Store`].
    pub fn note_folder_rekey_floor(&mut self, vault: VaultId, floor: u64, now: u64) -> Result<()> {
        self.vault(vault)?;
        Ok(self.store.note_folder_rekey_floor(&vault, floor, now)?)
    }

    /// Notes that a complete new-epoch snapshot set of `vault` exists in the
    /// own log.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_folder_rekey_snapshot(&mut self, vault: VaultId) -> Result<()> {
        self.store.mark_folder_rekey_snapshot(&vault)?;
        Ok(())
    }

    /// Dismisses an unverifiable higher-epoch account file (by path and
    /// SHA-256): it never raises a finding again.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn dismiss_unverifiable_file(&mut self, path: &str, sha256: &[u8; 32]) -> Result<()> {
        Ok(self.store.dismiss_unverifiable_file(path, sha256)?)
    }

    /// Undoes the dismissal of a file (a re-key marker the user restores,
    /// ADR-0064). Returns whether a dismissal was removed. The sync engine's
    /// `undismiss_rekey_marker` calls it for a marker the folder shows.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn undismiss_file(&mut self, path: &str, sha256: &[u8; 32]) -> Result<bool> {
        Ok(self.store.undismiss_file(path, sha256)?)
    }

    /// Whether the file was dismissed.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn file_dismissed(&self, path: &str, sha256: &[u8; 32]) -> Result<bool> {
        Ok(self.store.file_dismissed(path, sha256)?)
    }
}

/// Splits the deleted unlock rows into the quick-unlock enrollment and the
/// hardware keys.
fn split_removed(
    rows: Vec<crate::storage::StoredUnlockMethod>,
) -> (Option<QuickUnlockEnrollment>, Vec<HardwareKeyEnrollment>) {
    let mut quick: Option<QuickUnlockEnrollment> = None;
    let mut hardware = Vec::new();
    for row in rows {
        match row.method {
            UnlockMethod::HardwareKey(id) => {
                if let Ok(h) =
                    HardwareKeyEnrollment::from_params(id, &row.params, row.updated_at_ms)
                {
                    hardware.push(h);
                }
            }
            UnlockMethod::QuickUnlock(_) => {
                let e = QuickUnlockEnrollment::from_row(row);
                quick = match (quick, e) {
                    (Some(a), Some(b)) => Some(
                        if a.enrollment_id.as_bytes() >= b.enrollment_id.as_bytes() {
                            a
                        } else {
                            b
                        },
                    ),
                    (a, b) => a.or(b),
                };
            }
            UnlockMethod::Password => {}
        }
    }
    (quick, hardware)
}
