//! Unlocking with the master password and with the quick-unlock key
//! (CRYPTO_SPEC §6.1, §6.2, §6.9.2 to §6.9.4), and the helpers that take the
//! locked store out of the core for one long call and put it back.

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
        self.put_back_with(taken, false);
    }

    /// As [`Core::put_back`], but a `lock()` that came meanwhile stays
    /// requested, for a call that goes on to unlock (the join, M10-G2 round 2,
    /// N3): `finish_unlocked` then ends locked.
    pub(crate) fn put_back_keeping_lock(&self, taken: Taken) {
        self.put_back_with(taken, true);
    }

    fn put_back_with(&self, taken: Taken, keep_lock: bool) {
        // The flag is cleared under the mutex, with the phase (G1-1): a `lock()`
        // that came meanwhile has nothing to lock, and one that comes after
        // this sees the locked phase.
        if let Ok(mut guard) = self.guard() {
            let strict = if keep_lock {
                false
            } else {
                guard.pending.lock = false;
                std::mem::take(&mut guard.pending.strict)
            };
            guard.phase = match taken {
                Taken::NoAccount(ls) => Phase::NoAccount(ls),
                Taken::Locked(mut ls) => {
                    // A strict lock that came while the store was out has
                    // nothing left to lock, but still sets the limit (F3, N4).
                    if strict {
                        ls.require_password();
                    }
                    // A boot marker the failed-attempt limit could not delete
                    // is tried again (G1-13).
                    let _ = ls.enforce_boot_marker_limit();
                    Phase::Locked(ls)
                }
            };
            // A `close_store()` that came meanwhile closes the store now
            // (G3i-1). A put-back that goes on to unlock leaves it to the end
            // of that unlock.
            if !keep_lock {
                guard.close_if_requested();
            }
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
        let mut info = UnlockedInfo {
            account: session.account_id(),
            email: session.sync_config().ok().flatten().and_then(|c| c.email),
            quick_unlock_enrolled: session.quick_unlock_enrollment().ok().flatten().is_some(),
            unlock,
        };
        let mut guard = self.guard()?;
        if let Some(strict) = guard.pending.take_lock() {
            self.generation_bump();
            guard.phase = Phase::Locked(if strict {
                session.lock_requiring_password()
            } else {
                session.lock()
            });
            guard.close_if_requested();
            drop(guard);
            return Ok(self.state());
        }
        guard.settings.unlocked(&mut session);
        let distrusted = guard.settings.take_distrusted();
        // The age of a quick-unlock enrolment was decided before the settings
        // were checked (the unverified file may only shorten it, and the age
        // byte is not authenticated). It is decided again now that the settings
        // MAC verified: the shorter of the enrolment's age and the verified
        // value applies (N1). An enrolment does not outlive settings that could
        // not be verified (F1, F2).
        let checked =
            self.quick_expired_after_check(&mut session, &guard.settings, distrusted, unlock);
        if let QuickCheck::Expired(alias) = checked {
            if unlock == UnlockKind::Quick {
                self.generation_bump();
                guard.settings.locked();
                guard.expired_alias = alias;
                guard.phase = Phase::Locked(session.lock());
                guard.close_if_requested();
                drop(guard);
                return Err(MobileError::QuickUnlockExpired);
            }
            // A password unlock goes on without the enrolment; the platform
            // learns the key to delete from the expired alias.
            info.quick_unlock_enrolled = false;
            guard.expired_alias = alias;
        } else {
            guard.expired_alias = None;
        }
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
        distrusted: bool,
        unlock: UnlockKind,
    ) -> QuickCheck {
        let enrollment = match session.quick_unlock_enrollment() {
            Ok(Some(enrollment)) => enrollment,
            Ok(None) => return QuickCheck::Stands,
            // A failed read counts as expired, like a decode error (F4).
            Err(_) => {
                let _ = session.remove_quick_unlock();
                return QuickCheck::Expired(None);
            }
        };
        let decoded = Params::decode(&enrollment.platform_params, self.config.platform);
        let now = i64::try_from(self.now_ms()).unwrap_or(i64::MAX);
        // Settings that were not verified in this run (the key or the nonce
        // could not be read, F1) or whose file was missing or failed its MAC
        // (the strict stand-in may exceed a lowered verified value, F2) leave
        // no verified age: the enrolment expires.
        let trusted = settings.confirmed() && !distrusted;
        let stands = trusted
            && decoded.as_ref().is_ok_and(|p| {
                unlock != UnlockKind::Quick
                    || !enrollment
                        .is_expired(&settings.quick_unlock_policy_for(p.max_age_days), now)
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

#[cfg(all(test, feature = "test-hooks"))]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use mosaic_vault::session::QuickMethod;
    use zeroize::Zeroizing;

    use super::*;
    use crate::config::{CoreConfig, Platform};
    use crate::storage::{SecureStorage, StorageError};

    #[derive(Default)]
    struct Mem(Mutex<HashMap<String, Vec<u8>>>);

    impl SecureStorage for Mem {
        fn get(
            &self,
            name: &str,
        ) -> core::result::Result<Option<Zeroizing<Vec<u8>>>, StorageError> {
            Ok(self
                .0
                .lock()
                .map_err(|_| StorageError)?
                .get(name)
                .map(|v| Zeroizing::new(v.clone())))
        }
        fn put(&self, name: &str, value: &[u8]) -> core::result::Result<(), StorageError> {
            self.0
                .lock()
                .map_err(|_| StorageError)?
                .insert(name.to_owned(), value.to_vec());
            Ok(())
        }
        fn delete(&self, name: &str) -> core::result::Result<(), StorageError> {
            self.0.lock().map_err(|_| StorageError)?.remove(name);
            Ok(())
        }
    }

    /// F3: a plain lock won, an unlock took the locked store, a strict lock
    /// came while it was out, and the unlock failed. The store goes back with
    /// the password required (the flags are not simply dropped).
    #[test]
    fn put_back_keeps_a_strict_lock_that_found_the_store_out() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::open(
            CoreConfig {
                store_dir: dir.path().join("mosaic"),
                platform: Platform::Android,
                context: CoreContext::App,
                boot_id: vec![0, 0, 0, 7],
                trust_anchors_der: Vec::new(),
            },
            Arc::new(Mem::default()),
        )
        .unwrap();
        core.test_hooks()
            .create_local_account("user@example.com", "correct horse battery staple")
            .unwrap();
        core.lock();
        let taken = core.take_locked().unwrap();
        core.lock_requiring_password();
        core.put_back(taken);
        let guard = core.guard().unwrap();
        let Phase::Locked(locked) = &guard.phase else {
            panic!("not locked");
        };
        assert!(locked.password_required(QuickMethod::Biometric));
        assert!(!guard.pending.strict);
        assert!(!guard.pending.lock);
    }

    const RT_PASSWORD: &str = "correct horse battery staple";

    /// An opened core with a local account, unlocked; the Secret Key's
    /// display form comes with it.
    #[expect(clippy::unwrap_used, reason = "a test helper")]
    fn core_with_account(dir: &std::path::Path) -> (Core, Zeroizing<String>) {
        let core = Core::open(
            CoreConfig {
                store_dir: dir.join("mosaic"),
                platform: Platform::Ios,
                context: CoreContext::App,
                boot_id: vec![0, 0, 0, 7],
                trust_anchors_der: Vec::new(),
            },
            Arc::new(Mem::default()),
        )
        .unwrap();
        let (_, key) = core
            .test_hooks()
            .create_local_account("user@example.com", RT_PASSWORD)
            .unwrap();
        (core, key)
    }

    /// RT-verify-ios-1 (red team verify 2026-11-14, G3i-1 follow-up; fixed: the
    /// requests live in `Inner::pending`, and `close_store()` decides lock and
    /// close in one critical section). The window, as found:
    /// `close_store()` decides in two critical sections: `lock_with` takes
    /// the mutex, locks an `Unlocked` phase (no lock is requested, the phase
    /// was there to lock), releases the mutex for `finish_lock`, and only
    /// then takes the mutex again to close. Between the two an unlock can
    /// take the locked store (`take_locked`, phase `Unlocking`): the close
    /// section then sets only `pending.close`. The unlock's success path
    /// (`finish_unlocked`) honours `pending.lock` alone, so it ends
    /// `Unlocked` with the store open and its file lock held, after a
    /// `close_store()` that returned. The flag stays set and closes the
    /// store at the next settled lock instead (a surprise `StoreClosed`).
    /// Defense wanted: every path that settles a phase treats
    /// `pending.close` like `pending.lock` (lock, then close), and
    /// `close_store()` sets the flag in the same critical section that
    /// decides the lock. This test puts the core into the state the window
    /// leaves (`Unlocking`, no lock requested, a close requested) and lets
    /// the unlock finish.
    #[test]
    fn rt_verify_ios_1_close_requested_without_a_lock_requested_does_not_end_unlocked() {
        let dir = tempfile::tempdir().unwrap();
        let (core, key) = core_with_account(dir.path());
        core.lock();
        // The unlock took the store out.
        let taken = core.take_locked().unwrap();
        // The app goes to the background: `close_store()` finds `Unlocking`.
        core.close_store();
        assert!(core.guard().unwrap().pending.close);
        // The window of `close_store()`: its lock section ran before this
        // unlock took the store, so no lock is requested.
        core.guard().unwrap().pending.lock = false;
        let Taken::Locked(locked) = taken else {
            panic!("no account");
        };
        let secret_key = SecretKey::parse(&key).unwrap();
        let Ok(mut session) =
            locked.unlock(&MasterPassword::new(RT_PASSWORD.to_owned()), &secret_key)
        else {
            panic!("the password unlock failed");
        };
        core.record_boot(&mut session);
        let state = core.finish_unlocked(session, UnlockKind::Password).unwrap();
        assert_eq!(
            state,
            SessionState::StoreClosed,
            "a close during the unlock must end closed, not {state:?}"
        );
        assert!(!core.guard().unwrap().pending.close);
    }

    /// RT-verify-ios-2: the same window with a long call. `restore_unlocked`
    /// honours `pending.lock` alone; with only `pending.close` set it puts
    /// the unlocked phase back and the flag stays armed.
    #[test]
    fn rt_verify_ios_2_restore_unlocked_honours_a_close_requested_alone() {
        let dir = tempfile::tempdir().unwrap();
        let (core, _) = core_with_account(dir.path());
        let unlocked = core.take_unlocked().unwrap();
        core.close_store();
        assert!(core.guard().unwrap().pending.close);
        core.guard().unwrap().pending.lock = false;
        let restored = core.restore_unlocked(unlocked);
        assert_eq!(restored, Err(MobileError::Locked), "the close must win");
        assert_eq!(core.state(), SessionState::StoreClosed);
        assert!(!core.guard().unwrap().pending.close);
    }

    /// A reopen that is already running does the opening: a second
    /// `reopen_store()` only reports the state and neither opens the store nor
    /// clears the running reopen's flag.
    #[test]
    fn a_second_reopen_while_one_runs_does_not_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let (core, _) = core_with_account(dir.path());
        core.close_store();
        assert_eq!(core.state(), SessionState::StoreClosed);
        // The state a running reopen leaves: still closed, flag set.
        core.guard().unwrap().pending.reopening = true;
        assert_eq!(core.reopen_store(), Ok(SessionState::StoreClosed));
        let guard = core.guard().unwrap();
        assert!(matches!(guard.phase, Phase::Closed));
        assert!(guard.pending.reopening, "the running reopen keeps its flag");
    }

    /// A lock that arrives while a lock is settling (the session is out in
    /// `finish_lock`) is met by that lock: no request survives to lock the
    /// next unlock, and a strict one still requires the password.
    #[test]
    fn a_lock_during_a_settling_lock_does_not_survive() {
        let dir = tempfile::tempdir().unwrap();
        let (core, _) = core_with_account(dir.path());
        let unlocked = core.take_unlocked().unwrap();
        {
            let mut guard = core.guard().unwrap();
            guard.pending.lock = true;
            guard.pending.strict = true;
        }
        core.finish_lock(*unlocked, false);
        let guard = core.guard().unwrap();
        assert!(!guard.pending.lock && !guard.pending.strict && !guard.pending.close);
        let Phase::Locked(locked) = &guard.phase else {
            panic!("not locked");
        };
        assert!(locked.password_required(QuickMethod::Biometric));
    }

    /// A fresh core without an account, as a phone before a join.
    #[expect(clippy::unwrap_used, reason = "a test helper")]
    fn core_without_account(dir: &std::path::Path) -> Core {
        Core::open(
            CoreConfig {
                store_dir: dir.join("mosaic"),
                platform: Platform::Ios,
                context: CoreContext::App,
                boot_id: vec![0, 0, 0, 7],
                trust_anchors_der: Vec::new(),
            },
            Arc::new(Mem::default()),
        )
        .unwrap()
    }

    /// RT-verify2-ios-1 (red team verify 2 of 2026-11-14). `put_back_keeping_lock`
    /// settles a phase and keeps `pending.lock` and `pending.strict` armed
    /// for the `finish_join` that is meant to follow (N3). When no
    /// `finish_join` follows (the user cancels the join, or it expires), the
    /// requests outlive the phase they were made against: the `NoAccount`
    /// phase is settled, `join_cancel` does not clear them, and neither does a
    /// later `lock()` on the settled phase (`lock_with`'s `other` arm). The
    /// next phase settled through `finish_unlocked`, here the account the user
    /// creates instead (a second join behaves the same), is locked at once,
    /// with the password required because the stale lock was strict.
    /// Fail-safe, but stale state applied to a later request. Defense wanted:
    /// `join_cancel` and the join's expiry settle the requests they kept, or
    /// `lock_with` on a settled phase clears `pending.lock`/`strict` (a lock
    /// on a locked phase is met).
    #[test]
    fn rt_verify2_ios_1_a_lock_kept_for_an_abandoned_join_outlives_it() {
        let dir = tempfile::tempdir().unwrap();
        let core = core_without_account(dir.path());
        // `join_start`: the store is out during the network call...
        let taken = core.take_locked().unwrap();
        // ...and the re-authentication limit (or the app's background hook) locks meanwhile.
        core.lock_requiring_password();
        // The login succeeded: the lock is kept for `finish_join` (N3).
        core.put_back_keeping_lock(taken);
        // The user never finishes the join.
        core.join_cancel();
        core.lock();
        assert_eq!(core.state(), SessionState::NoAccount);
        let (state, _) = core
            .test_hooks()
            .create_local_account("user@example.com", RT_PASSWORD)
            .unwrap();
        assert!(
            matches!(state, SessionState::Unlocked { .. }),
            "the account created after an abandoned join must end unlocked, not {state:?}"
        );
    }

    /// RT-verify2-ios-2: the same window with `close_store()`. The rewrite
    /// says whoever settles a held-out phase locks and then closes it, but
    /// `put_back_keeping_lock` is a settling path that leaves `pending.close`
    /// to a `finish_join` that may never come. After `close_store()` returned
    /// (iOS: the app went to the background during the join's network call),
    /// the store is open again with its file lock held, and stays so through
    /// `join_cancel` until the next phase that is settled. Defense wanted: as
    /// RT-verify2-ios-1, or `put_back_keeping_lock` honours `pending.close`
    /// (a close ends the join: it needs the store).
    #[test]
    fn rt_verify2_ios_2_a_close_kept_for_an_abandoned_join_leaves_the_store_open() {
        let dir = tempfile::tempdir().unwrap();
        let core = core_without_account(dir.path());
        let taken = core.take_locked().unwrap();
        core.close_store();
        core.put_back_keeping_lock(taken);
        core.join_cancel();
        assert_eq!(
            core.state(),
            SessionState::StoreClosed,
            "a close_store() that returned must leave the store closed"
        );
    }
    /// A join in flight (its fetch runs): a strict lock is armed with its
    /// strictness, a close is armed, and `join_cancel` and the expiry leave
    /// them. The settled phase stays `NoAccount` until the join ends.
    #[test]
    fn a_lock_and_a_close_during_a_join_in_flight_are_armed_and_kept() {
        let dir = tempfile::tempdir().unwrap();
        let core = core_without_account(dir.path());
        core.guard().unwrap().pending.join_in_flight = true;
        core.lock_requiring_password();
        {
            let guard = core.guard().unwrap();
            assert!(guard.pending.lock && guard.pending.strict && !guard.pending.close);
        }
        core.close_store();
        core.join_cancel();
        core.guard().unwrap().drop_expired_join(u64::MAX);
        {
            let guard = core.guard().unwrap();
            assert!(guard.pending.lock && guard.pending.strict && guard.pending.close);
            assert!(matches!(guard.phase, Phase::NoAccount(_)));
        }
        // The join ends (its guard clears the mark): the same calls settle.
        core.guard().unwrap().pending.join_in_flight = false;
        core.join_cancel();
        assert_eq!(core.state(), SessionState::StoreClosed);
    }
}
