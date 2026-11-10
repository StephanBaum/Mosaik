//! CLI sessions (CRYPTO_SPEC §6.8, ADR-0072): a short-lived cache of `AK`
//! that lets the `mosaic` CLI open the store without the master password.
//!
//! - **Creation** needs a session that was unlocked with the master
//!   password in this same process, right now:
//!   [`UnlockedSession::create_cli_session`] refuses with
//!   [`Error::ReauthenticationRequired`] for a session opened from a CLI
//!   session, by quick unlock or by a security key. A session therefore
//!   never renews itself (review decision L3); only typing the master
//!   password again does. There is no other way to seal one: the account
//!   key is private to the session.
//! - **Opening** ([`LockedSession::unlock_cli_session`]) checks the rules of
//!   §6.8 and yields an [`UnlockedSession`] that does not count as a
//!   password unlock.
//!
//! The vault layer holds neither part: the CLI keeps the key part in the OS
//! keystore and the file in the runtime directory (`mosaic-platform`), and
//! deletes **both** after any [`Error::CliSessionRejected`], at expiry, at
//! `lock` and after every credential change (a password change, Secret Key
//! rotation, profile upgrade or re-key also ends every session by itself,
//! because the file is bound to the store's salt and epoch).

use core::time::Duration;

use mosaic_core::{
    AccountId, AccountKey, CliSessionBinding, CliSessionFile, CliSessionKey, open_cli_session,
    seal_cli_session,
};

use super::unlock_methods::QuickState;
use super::{Error, LockedSession, Result, UnlockError, UnlockedSession};
use crate::storage::{Store, VaultKeyWrap};

/// What a session file must be bound to: this store's account, install,
/// epoch and salt.
fn binding_of(store: &Store) -> Result<CliSessionBinding> {
    let params = store.account()?.ok_or(Error::NoAccount)?;
    Ok(CliSessionBinding {
        account_id: params.account_id,
        install_id: store.install_id()?,
        ak_epoch: store.ak_epoch()?,
        salt: params.salt,
    })
}

/// Rule 6 of §6.8: a vault key this store holds under `AK` opens with the
/// recovered key. A store with no such key (no vault yet) has nothing to
/// check and passes: the AEAD and the binding already vouch for the file.
fn ak_fits_store(store: &Store, ak: &AccountKey, account_id: &AccountId) -> Result<bool> {
    let mut held = false;
    for state in store.vaults()? {
        if state.key_wrap != VaultKeyWrap::AccountKey {
            continue;
        }
        for (generation, wrapped) in store.readable_vault_keys(&state.vault_id)? {
            held = true;
            if wrapped
                .unwrap_vault_key(ak, account_id, &state.vault_id, generation)
                .is_ok()
            {
                return Ok(true);
            }
        }
    }
    Ok(!held)
}

impl LockedSession {
    /// The account ID the store holds, so the caller can name the keystore
    /// entry and the session file before it unlocks.
    ///
    /// # Errors
    /// [`Error::NoAccount`], [`Error::Store`].
    pub fn account_id(&self) -> Result<AccountId> {
        Ok(self.store.account()?.ok_or(Error::NoAccount)?.account_id)
    }

    /// Opens a store from a CLI session (§6.8): the key part `key` (from
    /// the OS keystore), the file `file` and the clock `now_ms` (Unix ms).
    ///
    /// Every refusal of §6.8 is [`Error::CliSessionRejected`] (expired, for
    /// another account, install, epoch or salt, damaged, over-long, no held
    /// vault key opens under the recovered key); the caller then deletes
    /// both parts. The session does not count as a master-password unlock:
    /// [`UnlockedSession::create_cli_session`] refuses on it, and so does
    /// every quick-unlock rule that asks for the password.
    ///
    /// # Errors
    /// [`Error::NoAccount`], [`Error::Store`] (the store is unchanged; keep
    /// the session parts), [`Error::CliSessionRejected`].
    pub fn unlock_cli_session(
        self,
        key: &CliSessionKey,
        file: &CliSessionFile,
        now_ms: u64,
    ) -> core::result::Result<UnlockedSession, UnlockError> {
        match self.recover_cli_account_key(key, file, now_ms) {
            Ok((account_id, ak)) => {
                let quick = QuickState {
                    password_unlocked: false,
                    failed_quick: self.quick.failed_quick,
                    fresh_password: false,
                };
                UnlockedSession::load(self.store, account_id, ak, quick)
            }
            Err(error) => Err(UnlockError {
                session: self,
                error,
            }),
        }
    }

    fn recover_cli_account_key(
        &self,
        key: &CliSessionKey,
        file: &CliSessionFile,
        now_ms: u64,
    ) -> Result<(AccountId, AccountKey)> {
        let binding = binding_of(&self.store)?;
        let ak =
            open_cli_session(key, file, &binding, now_ms).map_err(|_| Error::CliSessionRejected)?;
        if !ak_fits_store(&self.store, &ak, &binding.account_id)? {
            return Err(Error::CliSessionRejected);
        }
        Ok((binding.account_id, ak))
    }
}

impl UnlockedSession {
    /// Seals a CLI session for this store (§6.8): the key part for the OS
    /// keystore and the file for the runtime directory. `now_ms` is the
    /// Unix time in milliseconds; `ttl` is 1 to 30 minutes.
    ///
    /// Only a session unlocked **with the master password in this process**
    /// may do it (new account, password unlock or join): a session opened
    /// from a CLI session, by quick unlock or with a security key gets
    /// [`Error::ReauthenticationRequired`]. The CLI must call this right
    /// after prompting for the password and never from a session it opened
    /// from a file.
    ///
    /// # Errors
    /// [`Error::ReauthenticationRequired`]; [`Error::Core`] with
    /// `InvalidTtl` for a `ttl` outside 1 to 30 minutes; [`Error::Store`].
    pub fn create_cli_session(
        &self,
        ttl: Duration,
        now_ms: u64,
    ) -> Result<(CliSessionKey, CliSessionFile)> {
        if !self.password_fresh {
            return Err(Error::ReauthenticationRequired);
        }
        let ttl_ms = u64::try_from(ttl.as_millis()).unwrap_or(u64::MAX);
        let binding = binding_of(&self.store)?;
        Ok(seal_cli_session(
            &self.account_key,
            &binding,
            now_ms,
            ttl_ms,
        )?)
    }
}
