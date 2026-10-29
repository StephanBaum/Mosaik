//! Session commands: state, account creation, Emergency Kit check, unlock,
//! quick unlock, lock, and the event subscription. Also the Emergency Kit
//! PDF and re-authentication, which the other commands use.

use mosaic_core::{AccountId, MasterPassword, SecretKey};
use mosaic_vault::session::{LockedSession, UnlockError};
use tauri::State;
use tauri::ipc::Channel;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::{AppEvent, LockReason};
use crate::contract::secret::{SecretIn, SecretOut};
use crate::contract::session::{
    AppStatus, CreateAccountRequest, CreatedAccount, EmergencyKit, KitSaved, KitVerification,
    QuickUnlockRequest, SaveKitRequest, SessionState, ShortcutState, UnlockRequest,
    VerifyKitRequest,
};
use crate::emergency_kit::{self, KitValues};
use crate::persist::{AccountInfo, valid_email};
use crate::shortcut::ShortcutStatus;
use crate::state::{AppState, Backend, Slot, guard};

/// Longest accepted vault name, in bytes.
pub(crate) const MAX_VAULT_NAME_LEN: usize = 256;

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

/// `unlock_quick`.
#[tauri::command]
pub async fn unlock_quick(
    state: State<'_, AppState>,
    request: QuickUnlockRequest,
) -> IpcResult<SessionState> {
    state.run(move |b| b.unlock_quick(&request)).await
}

/// `lock`.
#[tauri::command]
pub async fn lock(state: State<'_, AppState>) -> IpcResult<SessionState> {
    state.run(|b| b.lock(LockReason::Manual)).await
}

/// `subscribe_events`: the webview hands over a channel for [`AppEvent`]s.
#[tauri::command]
pub async fn subscribe_events(
    state: State<'_, AppState>,
    channel: Channel<AppEvent>,
) -> IpcResult<()> {
    state.backend().subscribe(channel)
}

fn master_password(secret: SecretIn) -> MasterPassword {
    // The copy moves into the zeroizing `MasterPassword`; the `SecretIn`
    // buffer zeroizes when it drops at the end of this function.
    let password = MasterPassword::new(secret.expose_secret().to_owned());
    drop(secret);
    password
}

fn validate_text(value: &str, max_len: usize) -> IpcResult<&str> {
    let value = value.trim();
    if value.is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
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

    fn secret_key_of(&self, account: AccountId) -> Option<SecretKey> {
        self.secret_keys.load(account).ok().flatten()
    }

    fn state_of(&self, slot: &Slot) -> IpcResult<SessionState> {
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
                        quick_unlock: self.quick_unlock.offered(),
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

    /// Starts the SSH agent after an unlock if the user turned it on. A
    /// failure shows up as `error` in `ssh_agent_status`.
    fn start_ssh_if_enabled(&self, slot: &Slot) {
        if self.stored().ssh_agent_enabled
            && let Ok(session) = slot.unlocked()
        {
            let _ = self.adapters.ssh.start(session, self.event_sink());
        }
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
    fn remember_account(&self, account_id: AccountId, email: Option<String>) {
        let _ = self.update_stored(|stored| {
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
        self.remember_account(account_id, Some(email.clone()));
        self.start_ssh_if_enabled(&slot);
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
    /// `locked`; `secretKeyRequired` if this device holds no Secret Key.
    pub fn verify_emergency_kit(&self, request: &VerifyKitRequest) -> IpcResult<KitVerification> {
        let account = self.slot()?.unlocked()?.account_id();
        let key = self
            .secret_key_of(account)
            .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
        let matches = key.matches_last_group(request.last_group.expose_secret());
        if matches {
            *guard(&self.kit_pending)? = None;
            self.update_stored(|s| {
                if let Some(account) = s.account.as_mut() {
                    account.kit_verified = true;
                }
            })?;
        }
        Ok(KitVerification { matches })
    }

    /// Unlocks with the master password and the stored or typed Secret Key.
    /// A typed Secret Key that unlocks is stored for next time.
    ///
    /// # Errors
    /// `wrongCredentials`, `secretKeyRequired`, `secretKeyMalformed`,
    /// `secretKeyMistyped`, `alreadyUnlocked`, `noAccount`, or a store error.
    pub fn unlock(&self, request: UnlockRequest) -> IpcResult<SessionState> {
        let typed = request.secret_key.is_some();
        let secret_key = match request.secret_key {
            Some(typed) => SecretKey::parse(typed.expose_secret())?,
            None => self
                .account()
                .and_then(|a| self.secret_key_of(a.account_id.get()))
                .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?,
        };
        let password = master_password(request.password);

        let mut slot = self.slot()?;
        let locked = take_locked(&mut slot)?;
        match locked.unlock(&password, &secret_key) {
            Ok(session) => {
                let account_id = session.account_id();
                *slot = Slot::Unlocked(Box::new(session));
                if typed {
                    self.secret_keys.save(account_id, &secret_key)?;
                }
                self.remember_account(account_id, None);
                self.start_ssh_if_enabled(&slot);
                self.touch();
                self.state_of(&slot)
            }
            Err(UnlockError { session, error }) => {
                *slot = Slot::Locked(session);
                Err(error.into())
            }
        }
    }

    /// Quick unlock (CRYPTO_SPEC §6.2). Answers `quickUnlockUnavailable`
    /// without any OS prompt until `mosaic-vault` can unlock with a
    /// quick-unlock wrapping (ADR-0020 item 7, PROGRESS M3-R6).
    ///
    /// # Errors
    /// `alreadyUnlocked`, `quickUnlockUnavailable`, the store's code.
    pub fn unlock_quick(&self, request: &QuickUnlockRequest) -> IpcResult<SessionState> {
        {
            let slot = self.slot()?;
            match &*slot {
                Slot::Unlocked(_) => return Err(IpcError::new(ErrorCode::AlreadyUnlocked)),
                Slot::Unavailable(code) => return Err(IpcError::new(*code)),
                Slot::Taken => return Err(IpcError::new(ErrorCode::Busy)),
                Slot::Locked(_) => {}
            }
        }
        // With vault support: release the platform secret
        // (`self.quick_unlock.platform(method)`, the OS prompt), derive W in
        // mosaic-core, and unlock with the method-0x02 wrapping. Until then
        // no OS call is made.
        let _ = request.method;
        Err(IpcError::new(ErrorCode::QuickUnlockUnavailable))
    }

    /// Locks: drops every key and decrypted item (they zeroize), then
    /// [`Backend::after_lock`]. Locking a locked vault is a no-op.
    ///
    /// # Errors
    /// The store's code when it is unavailable.
    pub fn lock(&self, reason: LockReason) -> IpcResult<SessionState> {
        let mut slot = self.slot()?;
        let mut locked_now = false;
        if matches!(*slot, Slot::Unlocked(_))
            && let Slot::Unlocked(session) = std::mem::replace(&mut *slot, Slot::Taken)
        {
            *slot = Slot::Locked(session.lock());
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
    /// and adapter state are dropped, the clipboard is cleared if it still
    /// holds what Mosaic copied, the lock hooks run (the onboarding kit window
    /// closes) and the webview is told.
    pub fn after_lock(&self, reason: LockReason) {
        if let Ok(mut pending) = self.kit_pending.lock() {
            *pending = None;
        }
        if let Ok(mut generated) = self.last_generated.lock() {
            *generated = None;
        }
        if let Ok(mut files) = self.import_files.lock() {
            files.clear();
        }
        self.adapters.import.clear();
        self.adapters.ssh.stop();
        let _ = self.clipboard.clear_if_ours();
        self.run_lock_hooks(reason);
        self.emit(&AppEvent::Locked { reason });
    }

    /// Checks the master password while unlocked (UX_GUIDELINES §8: turning
    /// a protection off needs it again).
    ///
    /// `mosaic-vault` has no verify method yet, so this re-derives the keys
    /// the only way the frozen API allows: lock, then unlock with
    /// `password` and the stored Secret Key. A wrong password therefore
    /// leaves the vault **locked** (`wrongCredentials` plus the `locked`
    /// event), which is the conservative outcome; a session method
    /// `verify_master_password` is proposed to keep it unlocked instead.
    ///
    /// # Errors
    /// `locked`, `secretKeyRequired`, `wrongCredentials`, `emptyPassword`.
    pub fn reauthenticate(&self, password: SecretIn) -> IpcResult<()> {
        let password = master_password(password);
        let mut slot = self.slot()?;
        let account = slot.unlocked()?.account_id();
        let key = self
            .secret_key_of(account)
            .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
        let Slot::Unlocked(session) = std::mem::replace(&mut *slot, Slot::Taken) else {
            return Err(IpcError::INTERNAL);
        };
        match session.lock().unlock(&password, &key) {
            Ok(session) => {
                *slot = Slot::Unlocked(Box::new(session));
                Ok(())
            }
            Err(UnlockError { session, error }) => {
                *slot = Slot::Locked(session);
                drop(slot);
                self.after_lock(LockReason::Manual);
                Err(error.into())
            }
        }
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
    /// (the vault is then locked), `secretKeyRequired`, `invalidInput`
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
fn take_locked(slot: &mut Slot) -> IpcResult<LockedSession> {
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
