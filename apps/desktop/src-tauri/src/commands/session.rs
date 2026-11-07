//! Session commands: state, account creation, Emergency Kit check, unlock,
//! lock, and the event subscription. Also the Emergency Kit PDF and
//! re-authentication, which the other commands use. Quick unlock is in
//! `commands/quick_unlock.rs`.

use mosaic_core::{AccountId, MasterPassword, SecretKey};
use mosaic_vault::session::{Error as SessionError, LockedSession, UnlockError};
use tauri::State;
use tauri::ipc::Channel;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::{AppEvent, LockReason};
use crate::contract::secret::{SecretIn, SecretOut};
use crate::contract::session::{
    AppStatus, CreateAccountRequest, CreatedAccount, EmergencyKit, KitSaved, KitVerification,
    ReauthenticateRequest, SaveKitRequest, SessionState, ShortcutState, UnlockRequest,
    VerifyKitRequest,
};
use crate::emergency_kit::{self, KitValues};
use crate::persist::{AccountInfo, valid_email};
use crate::quick_unlock::MAX_FAILED_REAUTH;
use crate::shortcut::ShortcutStatus;
use crate::state::{AppState, Backend, Slot, guard};

/// Longest accepted vault name, in bytes.
pub(crate) const MAX_VAULT_NAME_LEN: usize = 256;
/// Wrong Emergency Kit answers that lock the vault.
pub(crate) const MAX_WRONG_KIT_ANSWERS: u8 = 5;

/// `session_state`.
#[tauri::command]
pub async fn session_state(state: State<'_, AppState>) -> IpcResult<SessionState> {
    state.run(Backend::session_state).await
}

/// `create_account`.
#[tauri::command]
pub async fn create_account(
    state: State<'_, AppState>,
    request: CreateAccountRequest,
) -> IpcResult<CreatedAccount> {
    state.run(move |b| b.create_account(request)).await
}

/// `save_emergency_kit`.
#[tauri::command]
pub async fn save_emergency_kit(
    state: State<'_, AppState>,
    request: SaveKitRequest,
) -> IpcResult<KitSaved> {
    state.run(move |b| b.save_emergency_kit(request)).await
}

/// `app_status`.
#[tauri::command]
pub async fn app_status(state: State<'_, AppState>) -> IpcResult<AppStatus> {
    Ok(state.backend().app_status())
}

/// `verify_emergency_kit`.
#[tauri::command]
pub async fn verify_emergency_kit(
    state: State<'_, AppState>,
    request: VerifyKitRequest,
) -> IpcResult<KitVerification> {
    state.run(move |b| b.verify_emergency_kit(&request)).await
}

/// `unlock`.
#[tauri::command]
pub async fn unlock(state: State<'_, AppState>, request: UnlockRequest) -> IpcResult<SessionState> {
    state.run(move |b| b.unlock(request)).await
}

/// `lock`.
#[tauri::command]
pub async fn lock(state: State<'_, AppState>) -> IpcResult<SessionState> {
    state.run(|b| b.lock(LockReason::Manual)).await
}

/// `reauthenticate`.
#[tauri::command]
pub async fn reauthenticate(
    state: State<'_, AppState>,
    request: ReauthenticateRequest,
) -> IpcResult<()> {
    state.run(move |b| b.reauthenticate(request.password)).await
}

/// `subscribe_events`: the webview hands over a channel for [`AppEvent`]s.
#[tauri::command]
pub async fn subscribe_events(
    state: State<'_, AppState>,
    channel: Channel<AppEvent>,
) -> IpcResult<()> {
    state.backend().subscribe(channel)
}

pub(crate) fn master_password(secret: SecretIn) -> MasterPassword {
    // The copy moves into the zeroizing `MasterPassword`; the `SecretIn`
    // buffer zeroizes when it drops at the end of this function.
    let password = MasterPassword::new(secret.expose_secret().to_owned());
    drop(secret);
    password
}

pub(crate) fn validate_text(value: &str, max_len: usize) -> IpcResult<&str> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > max_len
        || value
            .chars()
            .any(|c| c.is_control() || super::is_spoofing_char(c))
    {
        return Err(IpcError::new(ErrorCode::InvalidInput));
    }
    Ok(value)
}

impl Backend {
    /// The current state.
    ///
    /// # Errors
    /// The store's code when it is unavailable; `storeUnavailable` if the
    /// store cannot be read.
    pub fn session_state(&self) -> IpcResult<SessionState> {
        let slot = self.slot()?;
        self.state_of(&slot)
    }

    pub(crate) fn secret_key_of(&self, account: AccountId) -> Option<SecretKey> {
        self.secret_keys.load(account).ok().flatten()
    }

    pub(crate) fn state_of(&self, slot: &Slot) -> IpcResult<SessionState> {
        let account = self.account();
        let email = account.as_ref().and_then(|a| a.email.clone());
        match slot {
            Slot::Unavailable(code) => Err(IpcError::new(*code)),
            Slot::Taken => Err(IpcError::new(ErrorCode::Busy)),
            Slot::Locked(locked) => {
                if locked.has_account()? {
                    Ok(SessionState::Locked {
                        email,
                        secret_key_stored: account
                            .is_some_and(|a| self.secret_keys.is_stored(a.account_id.get())),
                        quick_unlock: self
                            .quick_unlock
                            .offered(locked, &self.quick_unlock_policy()),
                    })
                } else {
                    Ok(SessionState::NoAccount)
                }
            }
            Slot::Unlocked(session) => Ok(SessionState::Unlocked {
                account_id: session.account_id().into(),
                email,
                kit_verified: account.is_some_and(|a| a.kit_verified),
            }),
        }
    }

    /// What follows every unlock: the SSH agent starts if the user turned
    /// it on (a failure shows up as `error` in `ssh_agent_status`), and the
    /// unlock hooks run (the browser endpoint tells its sessions).
    pub(crate) fn after_unlock(&self, slot: &Slot) {
        if self.stored().ssh_agent_enabled
            && let Ok(session) = slot.unlocked()
        {
            let _ = self.adapters.ssh.start(session, self.event_sink());
        }
        // Unlocking (password, biometrics or a security key) verifies the user.
        self.note_user_verification();
        self.run_unlock_hooks();
        // Sync starts when it is set up (a thread; nothing here waits for it).
        self.sync_after_unlock(slot);
    }

    /// What this system can and cannot do.
    #[must_use]
    pub fn app_status(&self) -> AppStatus {
        let shortcut = match self.shortcut_status.lock().ok().and_then(|s| *s) {
            Some(ShortcutStatus::Registered) => ShortcutState::Registered,
            Some(ShortcutStatus::Conflict) => ShortcutState::Conflict,
            Some(ShortcutStatus::Unavailable) | None => ShortcutState::Unavailable,
        };
        AppStatus {
            shortcut,
            screen_lock_detection: self.screen_lock_detection(),
            otp_screen_scan: self.screen.supported(),
        }
    }

    /// Whether a locked screen can be detected on this system right now.
    #[must_use]
    pub fn screen_lock_detection(&self) -> bool {
        cfg!(target_os = "linux")
            && self
                .os_sources
                .lock()
                .is_ok_and(|sources| sources.iter().any(|s| s.contains("Session.Lock")))
    }

    /// Records the unlocked account in the state file (best effort: a file
    /// that cannot be written keeps the state in memory).
    pub(crate) fn remember_account(
        &self,
        slot: &mut Slot,
        account_id: AccountId,
        email: Option<String>,
    ) {
        let _ = self.update_stored_in(slot, |stored| {
            let keep = stored
                .account
                .as_ref()
                .filter(|a| a.account_id.get() == account_id)
                .and_then(|a| a.email.clone());
            let kit_verified = stored
                .account
                .as_ref()
                .is_some_and(|a| a.account_id.get() == account_id && a.kit_verified);
            stored.account = Some(AccountInfo {
                account_id: account_id.into(),
                email: email.or(keep),
                kit_verified,
            });
        });
    }

    /// Creates the account and its first vault, stores the Secret Key and
    /// returns the Emergency Kit.
    ///
    /// # Errors
    /// `accountExists`, `alreadyUnlocked`, `invalidInput` (email or vault
    /// name), `emptyPassword`, or a store error.
    pub fn create_account(&self, request: CreateAccountRequest) -> IpcResult<CreatedAccount> {
        let email = request.email.trim().to_owned();
        if !valid_email(&email) {
            return Err(IpcError::new(ErrorCode::InvalidInput));
        }
        let vault_name = validate_text(&request.default_vault_name, MAX_VAULT_NAME_LEN)?.to_owned();
        let password = master_password(request.password);
        if password.expose_secret().is_empty() {
            return Err(IpcError::new(ErrorCode::EmptyPassword));
        }

        let mut slot = self.slot()?;
        let locked = take_locked(&mut slot)?;
        if let Err(error) = locked
            .has_account()
            .map_err(IpcError::from)
            .and_then(|has| {
                if has {
                    Err(IpcError::new(ErrorCode::AccountExists))
                } else {
                    Ok(())
                }
            })
        {
            *slot = Slot::Locked(locked);
            return Err(error);
        }
        let profile = match (self.choose_profile)() {
            Ok(profile) => profile,
            Err(error) => {
                *slot = Slot::Locked(locked);
                return Err(error.into());
            }
        };
        let (mut session, created) = match locked.create_account(&password, profile) {
            Ok(done) => done,
            Err(UnlockError { session, error }) => {
                *slot = Slot::Locked(session);
                return Err(error.into());
            }
        };
        // The account exists from here on; failures below leave it unlocked
        // so the user can retry the vault from the app.
        let vault = session.create_vault(&vault_name);
        let account_id = session.account_id();
        *slot = Slot::Unlocked(Box::new(session));
        self.secret_keys.save(account_id, &created.secret_key)?;
        // The settings check first (it can clear the account block of a
        // file that does not verify), then the account is recorded.
        self.settings_unlocked(&mut slot);
        self.remember_account(&mut slot, account_id, Some(email.clone()));
        self.after_unlock(&slot);
        // The kit is unconfirmed until `verify_emergency_kit` matches.
        *guard(&self.kit_pending)? = Some(account_id);
        self.touch();
        vault?;
        let session = self.state_of(&slot)?;
        Ok(CreatedAccount {
            emergency_kit: EmergencyKit {
                account_id: account_id.into(),
                email,
                secret_key: SecretOut::new(created.emergency_kit.secret_key),
            },
            session,
        })
    }

    /// Whether `request.last_group` matches the stored Secret Key. Needs an
    /// unlocked vault (otherwise it would be an unauthenticated oracle on
    /// the key's last group). A match ends the onboarding window in which
    /// the kit can be saved without the password.
    ///
    /// # Errors
    /// `locked`; `secretKeyRequired` if this device holds no Secret Key;
    /// `tooManyAttempts` on the fifth wrong answer since the last unlock (the
    /// vault locks and the next unlock needs the master password).
    pub fn verify_emergency_kit(&self, request: &VerifyKitRequest) -> IpcResult<KitVerification> {
        // The slot is held from the unlocked check to the write, and the
        // wrong-answer counter from the cap check to the increment: parallel
        // guesses cannot pass the cap together, and the flag is only
        // written to the account that is still unlocked.
        let mut slot = self.slot()?;
        let account = slot.unlocked()?.account_id();
        let mut wrong = guard(&self.kit_wrong)?;
        if *wrong >= MAX_WRONG_KIT_ANSWERS {
            return Err(IpcError::new(ErrorCode::TooManyAttempts));
        }
        let key = self
            .secret_key_of(account)
            .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
        let matches = key.matches_last_group(request.last_group.expose_secret());
        if !matches {
            // The answer is only four characters: after five wrong ones the
            // vault locks and the next unlock needs the master password.
            *wrong = wrong.saturating_add(1);
            let reached = *wrong >= MAX_WRONG_KIT_ANSWERS;
            drop(wrong);
            drop(slot);
            if reached {
                let _ = self.lock_with(LockReason::Manual, true);
                return Err(IpcError::new(ErrorCode::TooManyAttempts));
            }
            return Ok(KitVerification { matches });
        }
        *wrong = 0;
        drop(wrong);
        *guard(&self.kit_pending)? = None;
        self.update_stored_in(&mut slot, |s| {
            if let Some(account) = s.account.as_mut() {
                account.kit_verified = true;
            }
        })?;
        Ok(KitVerification { matches })
    }

    /// Unlocks with the master password and the stored or typed Secret Key.
    /// A typed Secret Key that unlocks is stored for next time.
    ///
    /// # Errors
    /// `wrongCredentials`, `unlockDelayed` (after the third wrong password
    /// in a row, see [`crate::unlock_throttle`]; checked before anything
    /// else), `secretKeyRequired`, `secretKeyMalformed`, `secretKeyMistyped`,
    /// `alreadyUnlocked`, `noAccount`, or a store error.
    pub fn unlock(&self, request: UnlockRequest) -> IpcResult<SessionState> {
        // The slot first: concurrent attempts queue here, and each reads the
        // clock and the backoff only once the one before it has finished and
        // recorded its failure. Checked before anything else runs (no key
        // parsing, no derivation).
        let mut slot = self.slot()?;
        let now = (self.unlock_clock)();
        guard(&self.unlock_throttle)?
            .check(now)
            .map_err(|_| IpcError::new(ErrorCode::UnlockDelayed))?;
        let typed = request.secret_key.is_some();
        let secret_key = match request.secret_key {
            Some(typed) => SecretKey::parse(typed.expose_secret())?,
            None => self
                .account()
                .and_then(|a| self.secret_key_of(a.account_id.get()))
                .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?,
        };
        let password = master_password(request.password);

        let locked = take_locked(&mut slot)?;
        match locked.unlock(&password, &secret_key) {
            Ok(session) => {
                let account_id = session.account_id();
                *slot = Slot::Unlocked(Box::new(session));
                if typed {
                    self.secret_keys.save(account_id, &secret_key)?;
                }
                guard(&self.unlock_throttle)?.succeeded();
                self.settings_unlocked(&mut slot);
                self.remember_account(&mut slot, account_id, None);
                self.after_unlock(&slot);
                self.touch();
                self.reconcile_slot(&mut slot);
                self.state_of(&slot)
            }
            Err(UnlockError { session, error }) => {
                *slot = Slot::Locked(session);
                let error = IpcError::from(error);
                if error.code == ErrorCode::WrongCredentials {
                    guard(&self.unlock_throttle)?.failed((self.unlock_clock)());
                }
                Err(error)
            }
        }
    }

    /// Locks: drops every key and decrypted item (they zeroize), then
    /// [`Backend::after_lock`]. Locking a locked vault is a no-op.
    ///
    /// # Errors
    /// The store's code when it is unavailable.
    pub fn lock(&self, reason: LockReason) -> IpcResult<SessionState> {
        self.lock_with(reason, false)
    }

    /// [`Backend::lock`] that also refuses every quick-unlock method until
    /// the next password unlock (the reauth-limit lock, ADR-0028 item 8):
    /// a plain lock resets the failed-attempt count, so locking after five
    /// wrong passwords would otherwise let the next Hello prompt reopen the
    /// guessing.
    fn lock_with(&self, reason: LockReason, require_password: bool) -> IpcResult<SessionState> {
        // A master password change in flight finishes first (at most three
        // minutes): locking between the server's answer and the local commit
        // would leave the device with the old password.
        // Sleep locks at once: a change cut off by it is recovered with the
        // new password.
        if reason != LockReason::Sleep {
            self.sync.wait_for_change();
        }
        // Pending logins and joins (with their keys and Secret Key) end with
        // every lock, also in the onboarding where no vault is unlocked.
        self.sync.drop_pending();
        let mut slot = self.slot()?;
        let mut locked_now = false;
        if matches!(*slot, Slot::Unlocked(_))
            && let Slot::Unlocked(session) = std::mem::replace(&mut *slot, Slot::Taken)
        {
            *slot = Slot::Locked(if require_password {
                session.lock_requiring_password()
            } else {
                session.lock()
            });
            locked_now = true;
        }
        let state = self.state_of(&slot);
        drop(slot);
        if locked_now {
            self.after_lock(reason);
        }
        state
    }

    /// What follows every lock: the last generated password, import files
    /// and adapter state are dropped, the quick-unlock grant is forgotten and
    /// a running security-key operation is cancelled, the clipboard is
    /// cleared if it still holds what Mosaic copied, the lock hooks run (the
    /// onboarding kit window closes) and the webview is told.
    pub fn after_lock(&self, reason: LockReason) {
        self.settings_locked();
        if let Ok(mut wrong) = self.kit_wrong.lock() {
            *wrong = 0;
        }
        self.quick_unlock.on_lock();
        self.clear_user_verification();
        if let Ok(mut pending) = self.kit_pending.lock() {
            *pending = None;
        }
        if let Ok(mut generated) = self.last_generated.lock() {
            *generated = None;
        }
        if let Ok(mut pending) = self.otp_pending.lock() {
            pending.clear();
        }
        if let Ok(mut files) = self.import_files.lock() {
            files.clear();
        }
        self.adapters.import.clear();
        self.adapters.ssh.stop();
        // The sync thread ends; the transport fails closed from here on.
        self.sync_after_lock();
        let _ = self.clipboard.clear_if_ours();
        self.run_lock_hooks(reason);
        self.emit(&AppEvent::Locked { reason });
    }

    /// Checks the master password while unlocked (UX_GUIDELINES §8: turning
    /// a protection off, or enrolling a quick method, needs it again) with
    /// `UnlockedSession::verify_master_password`. Nothing locks or unlocks:
    /// the vault stays open on a wrong password. A right password keeps the
    /// vault's `Reauthenticated` proof for two minutes (the enroll commands
    /// take `password: null` within that time); the fifth wrong one in a row
    /// locks the vault **and** requires the password for the next unlock
    /// (no quick method is offered until then). Any lock resets the count
    /// and drops the proof.
    ///
    /// # Errors
    /// `locked`, `secretKeyRequired`, `wrongCredentials`, `emptyPassword`.
    pub fn reauthenticate(&self, password: SecretIn) -> IpcResult<()> {
        self.reauthenticate_master(&master_password(password))
    }

    /// [`Backend::reauthenticate`] for a password already in a zeroizing
    /// type. A right password also marks the user as verified for passkeys
    /// ([`Backend::note_user_verification`]).
    ///
    /// # Errors
    /// As [`Backend::reauthenticate`].
    pub(crate) fn reauthenticate_master(&self, password: &MasterPassword) -> IpcResult<()> {
        self.check_master(password, true)
    }

    /// The passkey dialog's check: like [`Backend::reauthenticate_master`]
    /// (wrong passwords count towards the five-failure lock, a right one
    /// marks the user as verified for passkeys) but it earns **no**
    /// two-minute re-authentication grant, so a passkey confirmation cannot
    /// be spent on a protection change or a move out of a hardware vault.
    ///
    /// # Errors
    /// As [`Backend::reauthenticate`].
    pub(crate) fn verify_master_only(&self, password: &MasterPassword) -> IpcResult<()> {
        self.check_master(password, false)
    }

    fn check_master(&self, password: &MasterPassword, grant: bool) -> IpcResult<()> {
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        let key = self
            .secret_key_of(session.account_id())
            .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
        match session.verify_master_password(password, &key) {
            Ok(proof) => {
                if grant {
                    self.quick_unlock.grant_reauth(proof);
                } else {
                    self.quick_unlock.clear_reauth_failures();
                }
                self.note_user_verification();
                Ok(())
            }
            Err(SessionError::UnlockFailed) => {
                drop(slot);
                Err(self.count_wrong_master())
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Counts a wrong master password towards the five-failure lock (the fifth
    /// locks the vault and requires the password for the next unlock) and
    /// returns the error to answer with.
    pub(crate) fn count_wrong_master(&self) -> IpcError {
        if self.quick_unlock.reauth_failed() >= MAX_FAILED_REAUTH {
            let _ = self.lock_with(LockReason::Manual, true);
        }
        IpcError::new(ErrorCode::WrongCredentials)
    }

    /// The master password for a change that needs it: a non-empty
    /// `password` is checked ([`Backend::reauthenticate`]); `null` passes
    /// only within two minutes of a right one.
    ///
    /// # Errors
    /// `reauthRequired`, and those of [`Backend::reauthenticate`].
    pub(crate) fn require_reauth(&self, password: Option<SecretIn>) -> IpcResult<()> {
        match password {
            Some(password) if !password.expose_secret().is_empty() => self.reauthenticate(password),
            _ if self.quick_unlock.reauth_granted() => Ok(()),
            _ => Err(IpcError::new(ErrorCode::ReauthRequired)),
        }
    }

    /// Takes the re-authentication grant for one action that spends it
    /// (leaving a hardware-protected vault, adding a security key): the
    /// atomic version of [`Backend::require_reauth`] with no password.
    ///
    /// # Errors
    /// `reauthRequired` without a grant within two minutes (or when a
    /// parallel command took it).
    pub(crate) fn take_reauth_grant(&self) -> IpcResult<crate::quick_unlock::Grant> {
        self.quick_unlock
            .take_reauth()
            .ok_or(IpcError::new(ErrorCode::ReauthRequired))
    }

    /// Writes the Emergency Kit PDF to a path the user picks.
    ///
    /// The password is required once the kit was verified (or the vault
    /// locked) since account creation; during onboarding it is not. Nothing
    /// is rendered until the user has chosen a path, and the PDF lives in a
    /// zeroizing buffer.
    ///
    /// # Errors
    /// `locked`, `reauthRequired` (no or empty password), `wrongCredentials`
    /// (the vault locks after five in a row), `secretKeyRequired`, `invalidInput`
    /// (labels, or a path that cannot be written), `internal` if no dialog
    /// can be shown.
    pub fn save_emergency_kit(&self, request: SaveKitRequest) -> IpcResult<KitSaved> {
        request.labels.validate()?;
        let account = self.slot()?.unlocked()?.account_id();
        if *guard(&self.kit_pending)? != Some(account) {
            let password = request
                .password
                .ok_or(IpcError::new(ErrorCode::ReauthRequired))?;
            if password.expose_secret().is_empty() {
                return Err(IpcError::new(ErrorCode::ReauthRequired));
            }
            self.reauthenticate(password)?;
        }
        self.touch();
        let Some(path) = self
            .dialogs
            .save_file(emergency_kit::SUGGESTED_FILE_NAME, crate::dialogs::PDF)?
        else {
            return Ok(KitSaved::Cancelled);
        };
        let kit = {
            let slot = self.slot()?;
            if slot.unlocked()?.account_id() != account {
                return Err(IpcError::LOCKED);
            }
            let key = self
                .secret_key_of(account)
                .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
            let display = key.to_display()?;
            let email = self.account().and_then(|a| a.email).unwrap_or_default();
            emergency_kit::render(
                &request.labels,
                &KitValues {
                    email: &email,
                    account_id: &account.to_string(),
                    secret_key: display.expose_secret(),
                },
            )?
        };
        emergency_kit::write(&path, &kit)?;
        Ok(KitSaved::Saved)
    }
}

/// Takes the locked session out of the slot; the slot is unchanged on error.
pub(crate) fn take_locked(slot: &mut Slot) -> IpcResult<LockedSession> {
    match std::mem::replace(slot, Slot::Taken) {
        Slot::Locked(locked) => Ok(locked),
        other => {
            let error = match &other {
                Slot::Unlocked(_) => IpcError::new(ErrorCode::AlreadyUnlocked),
                Slot::Unavailable(code) => IpcError::new(*code),
                Slot::Locked(_) | Slot::Taken => IpcError::new(ErrorCode::Busy),
            };
            *slot = other;
            Err(error)
        }
    }
}
