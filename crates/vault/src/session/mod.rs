//! An unlocked vault session: `mosaic-core` key flows and item model on
//! top of the local [`Store`].
//!
//! A [`LockedSession`] owns a store and holds no key. Creating an account,
//! joining one on a new device, or unlocking with the master password and
//! the Secret Key turns it into an [`UnlockedSession`], which holds the
//! account key, every vault key it could unwrap, the decrypted items and an
//! in-memory search index. [`UnlockedSession::lock`] drops all of them
//! (every one zeroizes on drop) and gives the locked session back
//! (CRYPTO_SPEC §11).
//!
//! What the session guarantees:
//!
//! - **Only ciphertext reaches the disk.** Every write goes through the
//!   store, which accepts only `mosaic-core` ciphertext types. Item
//!   plaintext, titles, URLs and the search index live only in memory
//!   (§11).
//! - **Every local edit is one sync event** by this session's writer ID
//!   (a fresh device ID for every unlock, ADR-0015), sealed under the
//!   vault's current key and queued in the outbox.
//! - **Received records are merged, never overwritten**
//!   ([`UnlockedSession::apply_remote_record`]): a record whose
//!   authenticated version vector the local state dominates is ignored
//!   (rollback or replay, §10); a newer or concurrent one is combined with
//!   [`mosaic_core::Item::merge_and_repair`] and the result is persisted.
//! - **Key generations only go up** (§7.1): objects below a vault's
//!   highest accepted generation are rejected, and the store's triggers
//!   repeat that rule.
//!
//! The network sync itself is M6; this layer exposes what it needs
//! ([`UnlockedSession::pending_uploads`],
//! [`UnlockedSession::mark_uploaded`] and the `apply_remote_*` methods).
//!
//! ```no_run
//! use mosaic_core::{Argon2Profile, MasterPassword};
//! use mosaic_core::model::{ItemType, SecretField};
//! use mosaic_vault::session::LockedSession;
//! use mosaic_vault::storage::Store;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let locked = LockedSession::open(Store::open("vault.db")?);
//! let password = MasterPassword::new("correct horse battery staple".to_owned());
//! let (mut session, account) = locked
//!     .create_account(&password, Argon2Profile::DEFAULT)
//!     .map_err(|e| e.error)?;
//! // Show account.emergency_kit to the user once, then keep
//! // account.secret_key in the OS keystore.
//! let vault = session.create_vault("Personal")?;
//! let item = session.add_item(vault, &ItemType::Login, |e| {
//!     e.set_title("Example")?;
//!     e.set_secret(SecretField::Password, "hunter2")
//! })?;
//! assert_eq!(session.search("exam").len(), 1);
//! let locked = session.lock();
//! let session = locked
//!     .unlock(&password, &account.secret_key)
//!     .map_err(|e| e.error)?;
//! assert!(session.item(vault, item).is_ok());
//! # Ok(())
//! # }
//! ```

mod attachments;
mod error;
mod items;
mod search;
mod sync;
mod vaults;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use mosaic_core::model::{Item, Timestamp, Tombstone, VaultMeta};
use mosaic_core::{
    AccountId, AccountKey, Argon2Profile, DeviceId, ItemId, KeyGeneration, MasterPassword,
    OpaquePassword, PasswordParams, SecretKey, SecretString, UnlockMethod, VaultId, VaultKey,
    WrappedKey,
};

use crate::storage::{Origin, RecordBody, Store, VaultKeyWrap, VaultState};

pub use attachments::MAX_ACCOUNT_ATTACHMENT_BYTES;
pub use error::{Error, Result};
pub use items::{ItemFilter, ItemRef, OtpCode, TrashFilter};
pub use sync::{ApplyOutcome, RemoteRecord};
pub use vaults::VaultSummary;

/// Session construction for the fuzz targets only. Compiled only under
/// `cfg(fuzzing)` (set by cargo-fuzz); normal builds do not contain it.
#[cfg(fuzzing)]
#[doc(hidden)]
pub mod fuzzing {
    use mosaic_core::{AccountId, AccountKey};

    use super::{UnlockError, UnlockedSession};
    use crate::storage::Store;

    /// An unlocked session over `store` with an account key the caller
    /// already holds, skipping Argon2id, so a fuzz target can build fresh
    /// sessions for every input.
    ///
    /// # Errors
    /// As for an unlock after the key is derived.
    pub fn unlocked(
        store: Store,
        account_id: AccountId,
        account_key: AccountKey,
    ) -> core::result::Result<UnlockedSession, UnlockError> {
        UnlockedSession::load(Box::new(store), account_id, account_key)
    }
}

/// A session without keys: the store, and nothing that needs unlocking.
pub struct LockedSession {
    store: Box<Store>,
}

impl fmt::Debug for LockedSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LockedSession").finish_non_exhaustive()
    }
}

/// A failed unlock (or account creation): the error, and the locked
/// session back so the caller can try again.
pub struct UnlockError {
    /// The locked session, unchanged by a failed unlock.
    pub session: LockedSession,
    /// What went wrong.
    pub error: Error,
}

impl fmt::Debug for UnlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnlockError")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for UnlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}

impl std::error::Error for UnlockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// What the Emergency Kit shows (CRYPTO_SPEC §5.2): the account ID and the
/// Secret Key in its display format. The app adds the account email, the
/// server URL and a blank field for the master password.
#[derive(Debug)]
pub struct EmergencyKit {
    /// The account.
    pub account_id: AccountId,
    /// The Secret Key, `M1-XXXX-...` (shown once).
    pub secret_key: SecretString,
}

/// The result of [`LockedSession::create_account`].
#[derive(Debug)]
pub struct CreatedAccount {
    /// Show once, then never again.
    pub emergency_kit: EmergencyKit,
    /// The Secret Key. The app keeps it in the OS keystore (§5.2); the
    /// store never holds it.
    pub secret_key: SecretKey,
    /// The public parameters (pre-login, §9.2).
    pub params: PasswordParams,
    /// The OPAQUE registration input for the sync account (M6).
    pub opaque_password: OpaquePassword,
}

/// The result of [`UnlockedSession::change_master_password`].
#[derive(Debug)]
pub struct PasswordChanged {
    /// The new public parameters (new salt).
    pub params: PasswordParams,
    /// `OPW'` for OPAQUE re-registration (M6, §9.4).
    pub opaque_password: OpaquePassword,
}

/// A vault as the session holds it.
struct OpenVault {
    /// The highest accepted key generation; new objects are sealed under it.
    generation: KeyGeneration,
    /// Every vault key this session could unwrap, by generation.
    keys: BTreeMap<KeyGeneration, VaultKey>,
    /// The decrypted metadata, if stored and readable.
    meta: Option<VaultMeta>,
    /// A metadata row exists but could not be read (damaged, or sealed
    /// under a generation whose key is not held), so it gives no rollback
    /// anchor (`THREAT_MODEL` R17).
    meta_unreadable: bool,
    /// Every readable record, live items and tombstones.
    items: BTreeMap<ItemId, Item>,
    /// Records that failed to decrypt or decode.
    corrupt: BTreeSet<ItemId>,
}

impl OpenVault {
    fn new(generation: KeyGeneration) -> Self {
        Self {
            generation,
            keys: BTreeMap::new(),
            meta: None,
            meta_unreadable: false,
            items: BTreeMap::new(),
            corrupt: BTreeSet::new(),
        }
    }

    /// Whether the key of the current generation is held.
    fn available(&self) -> bool {
        self.keys.contains_key(&self.generation)
    }

    /// The key new objects are sealed under.
    fn current_key(&self) -> Result<&VaultKey> {
        self.keys
            .get(&self.generation)
            .ok_or(Error::VaultUnavailable)
    }

    fn live_item(&self, item_id: ItemId) -> Result<&Item> {
        let item = self.items.get(&item_id).ok_or(Error::UnknownItem)?;
        if item.is_deleted() {
            return Err(Error::ItemDeleted);
        }
        Ok(item)
    }
}

/// An unlocked session. See the [module documentation](self).
pub struct UnlockedSession {
    store: Box<Store>,
    account_id: AccountId,
    device: DeviceId,
    /// Writer IDs this session rotated away from ([`Self::rotate_device_id`]
    /// after `DeviceIdReused`): another writer shares them, so a received
    /// record that collides under one of them is merged without losing
    /// either writer's edits (ADR-0015).
    retired: Vec<DeviceId>,
    account_key: AccountKey,
    vaults: BTreeMap<VaultId, OpenVault>,
    index: search::SearchIndex,
}

impl fmt::Debug for UnlockedSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnlockedSession")
            .field("account_id", &self.account_id)
            .field("device", &self.device)
            .field("vaults", &self.vaults.len())
            .finish_non_exhaustive()
    }
}

impl LockedSession {
    /// Wraps an open store.
    #[must_use]
    pub fn open(store: Store) -> Self {
        Self {
            store: Box::new(store),
        }
    }

    /// Read access to the store (ciphertext only).
    #[must_use]
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Gives the store back.
    #[must_use]
    pub fn into_store(self) -> Store {
        *self.store
    }

    /// Whether the store holds an account (otherwise create or join one).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn has_account(&self) -> Result<bool> {
        Ok(self.store.account()?.is_some())
    }

    /// Creates a new account in this (empty) store and unlocks it: account
    /// ID, salt, Secret Key, account key and its password wrapping
    /// (CRYPTO_SPEC §5, §6), plus a fresh device ID for this install.
    /// `profile` normally comes from [`mosaic_core::kdf::calibrate`].
    ///
    /// # Errors
    /// [`Error::Store`] with `AccountExists` for a store that already has
    /// an account; [`Error::Core`] for an empty password or a failed
    /// allocation or CSPRNG.
    pub fn create_account(
        mut self,
        password: &MasterPassword,
        profile: Argon2Profile,
    ) -> core::result::Result<(UnlockedSession, CreatedAccount), UnlockError> {
        match create_account_in(&mut self.store, password, profile) {
            Ok((account_key, created)) => {
                UnlockedSession::load(self.store, created.params.account_id, account_key)
                    .map(|session| (session, created))
            }
            Err(error) => Err(UnlockError {
                session: self,
                error,
            }),
        }
    }

    /// Adds this device to an existing account: checks the master password
    /// and Secret Key against the account's password wrapping (as fetched
    /// through pre-login, §9.2), stores the account and unlocks. Vaults
    /// arrive afterwards through [`UnlockedSession::apply_remote_vault_key`].
    ///
    /// # Errors
    /// [`Error::UnlockFailed`] for a wrong password or Secret Key (nothing
    /// is stored then); [`Error::Store`] with `AccountExists` if the store
    /// already has an account.
    pub fn join_account(
        mut self,
        params: &PasswordParams,
        wrapped_account_key: &WrappedKey,
        password: &MasterPassword,
        secret_key: &SecretKey,
    ) -> core::result::Result<UnlockedSession, UnlockError> {
        let joined = derive_account_key(password, secret_key, params, wrapped_account_key)
            .and_then(|ak| {
                self.store.init_account(params, wrapped_account_key)?;
                Ok(ak)
            });
        match joined {
            Ok(ak) => UnlockedSession::load(self.store, params.account_id, ak),
            Err(error) => Err(UnlockError {
                session: self,
                error,
            }),
        }
    }

    /// Unlocks with the master password and the Secret Key (§6.1), then
    /// unwraps every vault key and decrypts every record into memory.
    /// Records that fail to decrypt are listed by
    /// [`UnlockedSession::corrupt_records`] instead of failing the unlock.
    ///
    /// # Errors
    /// [`Error::NoAccount`], [`Error::UnlockFailed`], [`Error::Store`].
    pub fn unlock(
        self,
        password: &MasterPassword,
        secret_key: &SecretKey,
    ) -> core::result::Result<UnlockedSession, UnlockError> {
        match unlock_account_key(&self.store, password, secret_key) {
            Ok((account_id, ak)) => UnlockedSession::load(self.store, account_id, ak),
            Err(error) => Err(UnlockError {
                session: self,
                error,
            }),
        }
    }
}

fn create_account_in(
    store: &mut Store,
    password: &MasterPassword,
    profile: Argon2Profile,
) -> Result<(AccountKey, CreatedAccount)> {
    if store.account()?.is_some() {
        return Err(crate::storage::Error::AccountExists.into());
    }
    let new = mosaic_core::create_account(password, profile)?;
    store.init_account(&new.params, &new.wrapped_account_key)?;
    let emergency_kit = EmergencyKit {
        account_id: new.params.account_id,
        secret_key: new.secret_key.to_display()?,
    };
    Ok((
        new.account_key,
        CreatedAccount {
            emergency_kit,
            secret_key: new.secret_key,
            params: new.params,
            opaque_password: new.opaque_password,
        },
    ))
}

fn derive_account_key(
    password: &MasterPassword,
    secret_key: &SecretKey,
    params: &PasswordParams,
    wrapped: &WrappedKey,
) -> Result<AccountKey> {
    mosaic_core::unlock_with_password(password, secret_key, params, wrapped).map_err(|e| match e {
        mosaic_core::Error::Decryption => Error::UnlockFailed,
        other => Error::Core(other),
    })
}

fn unlock_account_key(
    store: &Store,
    password: &MasterPassword,
    secret_key: &SecretKey,
) -> Result<(AccountId, AccountKey)> {
    let params = store.account()?.ok_or(Error::NoAccount)?;
    let wrapped = store
        .unlock_method(&UnlockMethod::Password)?
        .ok_or(Error::NoAccount)?;
    let ak = derive_account_key(password, secret_key, &params, &wrapped.wrapped_account_key)?;
    Ok((params.account_id, ak))
}

/// Wall-clock time for edit stamps.
fn now() -> Timestamp {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    Timestamp::from_unix_millis(ms)
}

/// An independent copy of an item (`Item` is deliberately not `Clone`).
fn dup_item(item: &Item) -> Result<Item> {
    if let Some(t) = item.to_tombstone() {
        return Ok(Item::from(t));
    }
    Ok(Item::from_json(item.to_json()?.expose_secret())?)
}

/// Seals an item, or its tombstone if it is deleted, as record `item_id`.
fn seal_record(
    item: &Item,
    vk: &VaultKey,
    vault_id: &VaultId,
    item_id: &ItemId,
) -> Result<RecordBody> {
    Ok(match item.to_tombstone() {
        Some(t) => RecordBody::Tombstone(t.seal(vk, vault_id, item_id)?),
        None => RecordBody::Item(item.seal(vk, vault_id, item_id)?),
    })
}

/// Decrypts a stored or received record with the vault's key of its
/// generation.
fn open_record(
    body: &RecordBody,
    keys: &BTreeMap<KeyGeneration, VaultKey>,
    vault_id: &VaultId,
    item_id: &ItemId,
) -> Result<Item> {
    let vk = keys
        .get(&body.key_generation())
        .ok_or(Error::VaultUnavailable)?;
    Ok(match body {
        RecordBody::Item(ct) => Item::open(ct, vk, vault_id, item_id)?,
        RecordBody::Tombstone(ct) => Item::from(Tombstone::open(ct, vk, vault_id, item_id)?),
    })
}

/// Loads one vault: unwraps its keys, decrypts its metadata and records.
fn load_vault(
    store: &Store,
    ak: &AccountKey,
    account_id: &AccountId,
    state: &VaultState,
) -> Result<OpenVault> {
    let vault_id = state.vault_id;
    let mut ov = OpenVault::new(state.highest_accepted_generation);
    if state.key_wrap != VaultKeyWrap::AccountKey {
        // A hardware-protected TOTP vault (§7.4) needs its TK, not AK.
        return Ok(ov);
    }
    // A damaged key row is left out, and only that generation is lost:
    // without the current one the vault is unavailable and the unlock goes
    // on (ADR-0010 item 9) until a wrapping that opens arrives
    // (`apply_remote_vault_key`); records under the other generations stay
    // readable.
    for (generation, wrapped) in store.readable_vault_keys(&vault_id)? {
        // A wrapping that does not open is skipped; without the current
        // generation the vault is unavailable rather than the unlock failing.
        if let Ok(vk) = wrapped.unwrap_vault_key(ak, account_id, &vault_id, generation) {
            ov.keys.insert(generation, vk);
        }
    }
    if !ov.available() {
        return Ok(ov);
    }
    let meta = match store.vault_meta(&vault_id) {
        Ok(meta) => meta,
        // Damaged metadata reads as absent, like metadata that does not
        // decrypt, but is remembered as unreadable.
        Err(crate::storage::Error::Corrupt) => {
            ov.meta_unreadable = true;
            None
        }
        Err(e) => return Err(e.into()),
    };
    if let Some(ct) = meta {
        ov.meta = ov
            .keys
            .get(&ct.key_generation())
            .and_then(|vk| VaultMeta::open(&ct, vk, &vault_id).ok());
        ov.meta_unreadable = ov.meta.is_none();
    }
    for entry in store.records(&vault_id)? {
        match entry {
            Ok(rec) => match open_record(&rec.body, &ov.keys, &vault_id, &rec.record_id) {
                Ok(item) => {
                    ov.items.insert(rec.record_id, item);
                }
                Err(_) => {
                    ov.corrupt.insert(rec.record_id);
                }
            },
            Err(corrupt) => {
                if let Some(id) = corrupt.record_id {
                    ov.corrupt.insert(id);
                }
            }
        }
    }
    Ok(ov)
}

/// A fresh writer ID for this session (recorded in the store as the last
/// one used) and every vault.
///
/// The writer ID that stamps edits is new for every unlock, never read
/// back from the store (ADR-0015, CRYPTO_SPEC §10): a store restored from
/// a backup or copied to another profile or machine therefore can never
/// write under an ID that another copy also uses, and with the store lock
/// (ADR-0010 item 10) two writers never share one.
fn load_state(
    store: &mut Store,
    ak: &AccountKey,
    account_id: &AccountId,
) -> Result<(DeviceId, BTreeMap<VaultId, OpenVault>)> {
    let device = DeviceId::new_v7()?;
    store.set_device_id(&device)?;
    let mut vaults = BTreeMap::new();
    for state in store.vaults()? {
        let ov = load_vault(store, ak, account_id, &state)?;
        if ov.available() {
            // Finishes a rotation or a key acceptance that was interrupted
            // before every record was re-sealed (ADR-0012 item 7).
            reseal_below_current(store, state.vault_id, &ov)?;
        }
        vaults.insert(state.vault_id, ov);
    }
    Ok((device, vaults))
}

/// Re-seals, under the vault's current key, every readable record stored
/// below the current generation, and queues each for upload; the metadata
/// too if it is older (CRYPTO_SPEC §7.1). Peers that accepted the current
/// generation reject anything older as a rollback, so without this a
/// version only this device holds (an edit uploaded under the old key that
/// the rotating device never saw, or a rotation interrupted half-way)
/// would never reach them. Rows that do not decrypt stay as they are.
fn reseal_below_current(store: &mut Store, vault_id: VaultId, ov: &OpenVault) -> Result<()> {
    let vk = ov.current_key()?;
    let below: Vec<ItemId> = store
        .records_below_generation(&vault_id, ov.generation)?
        .into_iter()
        .filter_map(crate::storage::RecordEntry::ok)
        .map(|r| r.record_id)
        .collect();
    for item_id in below {
        if let Some(item) = ov.items.get(&item_id) {
            let body = seal_record(item, vk, &vault_id, &item_id)?;
            store.upsert_record(&vault_id, &item_id, &body, Origin::Local)?;
        }
    }
    if let Some(meta) = &ov.meta {
        let current = matches!(
            store.vault_meta(&vault_id),
            Ok(Some(ct)) if ct.key_generation() == ov.generation
        );
        if !current {
            store.set_vault_meta(&vault_id, &meta.seal(vk, &vault_id)?)?;
        }
    }
    Ok(())
}

impl UnlockedSession {
    /// Builds the session from an unlocked account key. On failure the
    /// store goes back into a [`LockedSession`].
    fn load(
        mut store: Box<Store>,
        account_id: AccountId,
        account_key: AccountKey,
    ) -> core::result::Result<Self, UnlockError> {
        match load_state(&mut store, &account_key, &account_id) {
            Ok((device, vaults)) => {
                let mut index = search::SearchIndex::default();
                for (vault_id, ov) in &vaults {
                    for (item_id, item) in &ov.items {
                        index.update(*vault_id, *item_id, item);
                    }
                }
                Ok(Self {
                    store,
                    account_id,
                    device,
                    retired: Vec::new(),
                    account_key,
                    vaults,
                    index,
                })
            }
            Err(error) => Err(UnlockError {
                session: LockedSession { store },
                error,
            }),
        }
    }

    /// Locks: drops the account key, every vault key, every decrypted item
    /// and the search index (all of them zeroize on drop, CRYPTO_SPEC §11)
    /// and returns the locked session.
    #[must_use]
    pub fn lock(self) -> LockedSession {
        let Self { store, .. } = self;
        LockedSession { store }
    }

    /// The account.
    #[must_use]
    pub const fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// This session's writer ID, which stamps every local edit. It is new
    /// for every unlock (ADR-0015).
    #[must_use]
    pub const fn device_id(&self) -> DeviceId {
        self.device
    }

    /// Read access to the store (ciphertext only), for sync and
    /// diagnostics.
    #[must_use]
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Records that are stored but could not be decrypted or decoded, by
    /// vault. They are kept untouched; a newer version received from the
    /// server replaces them.
    #[must_use]
    pub fn corrupt_records(&self) -> Vec<(VaultId, ItemId)> {
        self.vaults
            .iter()
            .flat_map(|(v, ov)| ov.corrupt.iter().map(|i| (*v, *i)))
            .collect()
    }

    /// Changes the master password (CRYPTO_SPEC §6.4): checks `current`
    /// with `secret_key`, derives the new wrapping under the account's
    /// profile with a new salt, and replaces the stored one. Vault keys and
    /// items are not touched. The result carries what the server needs
    /// (M6: OPAQUE re-registration under §9.4).
    ///
    /// # Errors
    /// [`Error::UnlockFailed`] if `current` or `secret_key` is wrong (then
    /// nothing changes); [`Error::Core`] for an empty new password.
    pub fn change_master_password(
        &mut self,
        current: &MasterPassword,
        new_password: &MasterPassword,
        secret_key: &SecretKey,
    ) -> Result<PasswordChanged> {
        let (_, check) = unlock_account_key(&self.store, current, secret_key)?;
        if check != self.account_key {
            return Err(Error::UnlockFailed);
        }
        drop(check);
        let params = self.store.account()?.ok_or(Error::NoAccount)?;
        let change = mosaic_core::change_master_password(
            &self.account_key,
            new_password,
            secret_key,
            &params,
            params.profile,
        )?;
        self.store
            .update_password(&change.params, &change.wrapped_account_key)?;
        Ok(PasswordChanged {
            params: change.params,
            opaque_password: change.opaque_password,
        })
    }

    /// Replaces this session's writer ID after a merge reported
    /// [`mosaic_core::Error::DeviceIdReused`] (CRYPTO_SPEC §10): another
    /// writer shares the old ID. Retry the failed merge afterwards. The
    /// old ID stays suspect for the rest of the session: a received record
    /// or metadata that collides under it is merged with
    /// [`mosaic_core::Item::merge_and_repair_after_reuse`], so neither
    /// writer's edits are lost (ADR-0015).
    ///
    /// # Errors
    /// [`Error::Store`], [`Error::Core`] if the CSPRNG fails.
    pub fn rotate_device_id(&mut self) -> Result<DeviceId> {
        let d = DeviceId::new_v7()?;
        self.store.set_device_id(&d)?;
        self.retired.push(self.device);
        self.device = d;
        Ok(d)
    }

    fn vault(&self, vault_id: VaultId) -> Result<&OpenVault> {
        self.vaults.get(&vault_id).ok_or(Error::UnknownVault)
    }

    /// Seals `item` under the vault's current key, stores it with
    /// `origin`, and updates the cache and the search index.
    fn put_item(
        &mut self,
        vault_id: VaultId,
        item_id: ItemId,
        item: Item,
        origin: Origin,
    ) -> Result<()> {
        let ov = self.vaults.get_mut(&vault_id).ok_or(Error::UnknownVault)?;
        let body = seal_record(&item, ov.current_key()?, &vault_id, &item_id)?;
        self.store
            .upsert_record(&vault_id, &item_id, &body, origin)?;
        self.index.update(vault_id, item_id, &item);
        ov.corrupt.remove(&item_id);
        ov.items.insert(item_id, item);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
