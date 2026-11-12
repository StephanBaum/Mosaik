//! Unlocking with the master password and with the quick-unlock key
//! (CRYPTO_SPEC §6.1, §6.2, §6.9.2 to §6.9.4), and the helpers that take the
//! locked store out of the core for one long call and put it back.

use std::sync::atomic::Ordering;

use mosaic_core::{AccountId, Kek, MasterPassword, SecretBytes, SecretKey};
use mosaic_vault::session::{
    BootId, Error as SessionError, LockedSession, Reauthenticated, UnlockedSession,
};
use zeroize::Zeroizing;

use crate::Core;
use crate::config::CoreContext;
use crate::error::{MobileError, Result};
use crate::quick_unlock::params::Params;
use crate::secret::text_of;
use crate::state::{Phase, SessionState, UnlockKind, UnlockedInfo, UnlockedPhase};
use crate::storage::{read, secret_key_name};

/// The 64 MiB margin of the memory preflight (CRYPTO_SPEC §6.9.4).
const MEMORY_MARGIN_BYTES: u64 = 64 * 1024 * 1024;

/// Wrong master passwords in a row at a re-authentication before the session
/// ends and the password is required (ADR-0028 item 8; the desktop's limit).
pub(crate) const MAX_FAILED_REAUTH: u8 = 5;

/// The age of a quick-unlock enrolment decided against the verified settings.
enum QuickCheck {
    Stands,
    /// Expired and removed; the alias of the platform key to delete.
    Expired(Option<String>),
}

/// The locked store, taken out of the core for one long call.
pub(crate) enum Taken {
    /// A store without an account.
    NoAccount(LockedSession),
    /// A store with an account.
    Locked(LockedSession),
}

impl Core {
    /// Takes the locked store out; the core reports `Busy` until
    /// [`Core::put_back`] or [`Core::finish_unlocked`].
    pub(crate) fn take_locked(&self) -> Result<Taken> {
        let mut guard = self.guard()?;
        match std::mem::replace(&mut guard.phase, Phase::Unlocking) {
            Phase::NoAccount(ls) => Ok(Taken::NoAccount(ls)),
            Phase::Locked(ls) => Ok(Taken::Locked(ls)),
            other => {
                let error = match &other {
                    Phase::Closed if guard.store_busy => MobileError::StoreInUse,
                    Phase::Closed => MobileError::StoreUnavailable,
                    Phase::Unlocked(_) => MobileError::InvalidInput,
                    _ => MobileError::Busy,
                };
                guard.phase = other;
                Err(error)
            }
        }
    }

    /// Puts a taken store back. A `lock()` that came meanwhile has nothing
    /// left to lock.
    pub(crate) fn put_back(&self, taken: Taken) {
        // The flag is cleared under the mutex, with the phase (G1-1): a `lock()`
        // that came meanwhile has nothing to lock, and one that comes after
        // this sees the locked phase.
        if let Ok(mut guard) = self.guard() {
            self.lock_requested.store(false, Ordering::SeqCst);
            self.lock_strict.store(false, Ordering::SeqCst);
            guard.phase = match taken {
                Taken::NoAccount(ls) => Phase::NoAccount(ls),
                Taken::Locked(mut ls) => {
                    // A boot marker the failed-attempt limit could not delete
                    // is tried again (G1-13).
                    let _ = ls.enforce_boot_marker_limit();
                    Phase::Locked(ls)
                }
            };
        }
    }

    /// Stores a freshly unlocked session: settings check, cached facts, and
    /// the lock a `lock()` during the unlock asked for.
    pub(crate) fn finish_unlocked(
        &self,
        mut session: UnlockedSession,
        unlock: UnlockKind,
    ) -> Result<SessionState> {
        if unlock == UnlockKind::Password {
            // A right password ends the count of wrong re-authentications,
            // which is stored with the vault (N2). A failed write only keeps
            // the count, the stricter outcome.
            let _ = session.reset_failed_reauth();
        }
        let info = UnlockedInfo {
            account: session.account_id(),
            email: session.sync_config().ok().flatten().and_then(|c| c.email),
            quick_unlock_enrolled: session.quick_unlock_enrollment().ok().flatten().is_some(),
            unlock,
        };
        let mut guard = self.guard()?;
        if self.lock_requested.swap(false, Ordering::SeqCst) {
            let strict = self.lock_strict.swap(false, Ordering::SeqCst);
            self.generation_bump();
            guard.phase = Phase::Locked(if strict {
                session.lock_requiring_password()
            } else {
                session.lock()
            });
            drop(guard);
            return Ok(self.state());
        }
        guard.settings.unlocked(&mut session);
        // The age of a quick-unlock enrolment was decided before the settings
        // were checked (the unverified file may only shorten it, and the age
        // byte is not authenticated). It is decided again now that the settings
        // MAC verified: the shorter of the enrolment's age and the verified
        // value applies (N1).
        if unlock == UnlockKind::Quick
            && let QuickCheck::Expired(alias) =
                self.quick_expired_after_check(&mut session, &guard.settings)
        {
            self.generation_bump();
            guard.settings.locked();
            guard.expired_alias = alias;
            guard.phase = Phase::Locked(session.lock());
            drop(guard);
            return Err(MobileError::QuickUnlockExpired);
        }
        guard.expired_alias = None;
        guard.phase = Phase::Unlocked(Box::new(UnlockedPhase {
            owned: mosaic_sync::engine::OwnedSession::new(session),
            info,
        }));
        drop(guard);
        Ok(self.state())
    }

    /// After a quick unlock and the settings check: whether the enrolment is
    /// expired under the verified settings. If so it is removed from the store
    /// and the platform key's alias (when known) is returned.
    fn quick_expired_after_check(
        &self,
        session: &mut UnlockedSession,
        settings: &crate::settings::SettingsGuard,
    ) -> QuickCheck {
        let Some(enrollment) = session.quick_unlock_enrollment().ok().flatten() else {
            return QuickCheck::Stands;
        };
        let decoded = Params::decode(&enrollment.platform_params, self.config.platform);
        let now = i64::try_from(self.now_ms()).unwrap_or(i64::MAX);
        let stands = decoded.as_ref().is_ok_and(|p| {
            !enrollment.is_expired(&settings.quick_unlock_policy_for(p.max_age_days), now)
        });
        if stands {
            return QuickCheck::Stands;
        }
        let _ = session.remove_quick_unlock();
        QuickCheck::Expired(decoded.ok().map(|p| p.key_alias()))
    }

    /// The Secret Key of an account from secure storage.
    pub(crate) fn stored_secret_key(&self, account: &AccountId) -> Result<SecretKey> {
        let raw = read(self.storage.as_ref(), &secret_key_name(account))?
            .ok_or(MobileError::SecretKeyRequired)?;
        let mut array: [u8; 16] = raw
            .as_slice()
            .try_into()
            .map_err(|_| MobileError::StorageUnavailable)?;
        let key = SecretKey::from_secret(SecretBytes::from_array(&mut array));
        zeroize::Zeroize::zeroize(&mut array);
        Ok(key)
    }

    /// The quick-unlock policy for the enrolment of `locked`: its own verified
    /// maximum age, shortened by verified settings, never above 30 days
    /// (G1-10).
    pub(crate) fn quick_policy(
        &self,
        locked: &LockedSession,
    ) -> mosaic_vault::session::QuickUnlockPolicy {
        let enrollment = locked.quick_unlock_enrollment().ok().flatten();
        let guard = self.guard();
        match (enrollment, guard) {
            (Some(e), Ok(g)) => {
                crate::quick_unlock::policy_of(&e, self.config.platform, &g.settings)
            }
            _ => mosaic_vault::session::QuickUnlockPolicy::default(),
        }
    }

    /// Records that the master password unlocked in this device boot
    /// (CRYPTO_SPEC §6.9.2). An empty boot ID records nothing.
    pub(crate) fn record_boot(&self, session: &mut UnlockedSession) {
        if let Ok(boot) = BootId::new(&self.config.boot_id) {
            let now = i64::try_from(self.now_ms()).unwrap_or(i64::MAX);
            let _ = session.record_boot_marker(&boot, now);
        }
    }

    /// Checks the master password against the stored Secret Key and the open
    /// session (a second Argon2id). The proof enrols quick unlock.
    ///
    /// # Errors
    /// `locked`, `busy`, `emptyPassword`, `wrongCredentials`,
    /// `secretKeyRequired`, `storageUnavailable`.
    pub(crate) fn verify_password(&self, password: &str) -> Result<Reauthenticated> {
        let account = self.account_id()?;
        let secret_key = self.stored_secret_key(&account)?;
        let password = MasterPassword::new(password.to_owned());
        let unlocked = self.take_unlocked()?;
        // The count of wrong passwords is stored with the vault and changed in
        // the same step as the check, before the answer: a restart of the app
        // does not start it again, and the guess is counted before it is
        // refused (N2).
        let checked = {
            use mosaic_sync::wire::SessionHandle as _;
            unlocked.owned.with_session_mut(|s| {
                let result = s.verify_master_password(&password, &secret_key);
                let failures = match &result {
                    Ok(_) => {
                        let _ = s.reset_failed_reauth();
                        None
                    }
                    // A count that cannot be stored counts as the limit.
                    Err(SessionError::UnlockFailed) => {
                        Some(s.note_failed_reauth().unwrap_or(MAX_FAILED_REAUTH))
                    }
                    Err(_) => None,
                };
                (result, failures)
            })
        };
        self.work_pause();
        let restored = self.restore_unlocked(unlocked);
        let (result, failures) = checked.ok_or(MobileError::Locked)?;
        if failures.is_some_and(|n| n >= MAX_FAILED_REAUTH) {
            // A guesser at an unlocked phone cannot go on: the session ends
            // and only the master password unlocks again. A `lock()` that came
            // meanwhile does not spare the limit: the strict lock also finds a
            // locked session (N4).
            self.lock_requiring_password();
            return Err(MobileError::Locked);
        }
        restored?;
        result.map_err(MobileError::from)
    }

    /// Unlocks with the master password and the Secret Key (stored, or typed
    /// when the phone does not hold it).
    ///
    /// # Errors
    /// `passwordInApp` in the extension context, `busy`, `noAccount`,
    /// `emptyPassword`, `secretKeyRequired`, `secretKeyMalformed`,
    /// `secretKeyMistyped`, `wrongCredentials`, `storageUnavailable`.
    pub fn unlock_password(
        &self,
        password: Vec<u8>,
        secret_key: Option<Vec<u8>>,
    ) -> Result<SessionState> {
        if self.config.context == CoreContext::IosAutofillExtension {
            return Err(MobileError::PasswordInApp);
        }
        let mut password = text_of(password)?;
        let typed = secret_key.map(text_of).transpose()?;
        if password.is_empty() {
            return Err(MobileError::EmptyPassword);
        }
        if matches!(self.state(), SessionState::Unlocked { .. }) {
            return Ok(self.state());
        }
        let account = self.account_id()?;
        // The storage callback runs before the core's mutex is taken.
        let secret_key = match typed {
            Some(text) => SecretKey::parse(&text)?,
            None => self.stored_secret_key(&account)?,
        };
        let locked = match self.take_locked()? {
            Taken::Locked(ls) => ls,
            other @ Taken::NoAccount(_) => {
                self.put_back(other);
                return Err(MobileError::NoAccount);
            }
        };
        let master = MasterPassword::new(std::mem::take(&mut *password));
        let result = locked.unlock(&master, &secret_key);
        self.work_pause();
        match result {
            Ok(mut session) => {
                self.record_boot(&mut session);
                self.finish_unlocked(session, UnlockKind::Password)
            }
            Err(failed) => {
                self.put_back(Taken::Locked(failed.session));
                Err(failed.error.into())
            }
        }
    }

    /// Unlocks with `W`, the 32 bytes the platform decrypted with its
    /// biometric key (CRYPTO_SPEC §6.2, §6.9.2).
    ///
    /// # Errors
    /// `busy`, `noAccount`, `invalidInput` (`W` is not 32 bytes),
    /// `passwordRequired` (no boot marker for this boot, five failures, or no
    /// enrolment), `quickUnlockExpired`, `quickUnlockInvalidated`.
    pub fn unlock_quick(&self, w: Vec<u8>) -> Result<SessionState> {
        let w = Zeroizing::new(w);
        let mut array: [u8; 32] = w
            .as_slice()
            .try_into()
            .map_err(|_| MobileError::InvalidInput)?;
        let kek = Kek::from_secret(SecretBytes::from_array(&mut array));
        zeroize::Zeroize::zeroize(&mut array);
        if matches!(self.state(), SessionState::Unlocked { .. }) {
            return Ok(self.state());
        }
        let now = i64::try_from(self.now_ms()).unwrap_or(i64::MAX);
        let locked = match self.take_locked()? {
            Taken::Locked(ls) => ls,
            other @ Taken::NoAccount(_) => {
                self.put_back(other);
                return Err(MobileError::NoAccount);
            }
        };
        let policy = self.quick_policy(&locked);
        // The key to delete if the enrolment ends here.
        let alias = locked
            .quick_unlock_enrollment()
            .ok()
            .flatten()
            .and_then(|e| Params::decode(&e.platform_params, self.config.platform).ok())
            .map(|p| p.key_alias());
        let result = locked.unlock_quick(&kek, &policy, now);
        self.work_pause();
        match result {
            Ok(session) => self.finish_unlocked(session, UnlockKind::Quick),
            Err(failed) => {
                if matches!(
                    failed.error,
                    SessionError::QuickUnlockExpired | SessionError::QuickUnlockInvalidated
                ) && let Ok(mut guard) = self.guard()
                {
                    guard.expired_alias = alias;
                }
                self.put_back(Taken::Locked(failed.session));
                Err(failed.error.into())
            }
        }
    }

    /// The Argon2 memory of the stored profile plus 64 MiB, which the
    /// platform compares with the memory available to the process before an
    /// unlock (CRYPTO_SPEC §6.9.4). `None` without an account.
    #[must_use]
    pub fn required_unlock_memory_bytes(&self) -> Option<u64> {
        use mosaic_sync::wire::SessionHandle as _;
        let owned = {
            let guard = self.guard().ok()?;
            match &guard.phase {
                Phase::Locked(ls) => {
                    let params = ls.store().account().ok().flatten()?;
                    return Some(memory_of(&params));
                }
                Phase::Unlocked(u) => u.owned.clone(),
                _ => return None,
            }
        };
        owned
            .with_session(|s| s.store().account().ok().flatten())
            .flatten()
            .map(|p| memory_of(&p))
    }
}

fn memory_of(params: &mosaic_core::PasswordParams) -> u64 {
    u64::from(params.profile.m_cost_kib()) * 1024 + MEMORY_MARGIN_BYTES
}
