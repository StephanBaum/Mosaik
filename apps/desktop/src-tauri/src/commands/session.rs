//! Session commands: state, account creation, Emergency Kit check, unlock,
//! quick unlock, lock, and the event subscription.

use mosaic_core::{MasterPassword, SecretKey};
use mosaic_vault::session::{LockedSession, UnlockError};
use tauri::State;
use tauri::ipc::Channel;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::{AppEvent, LockReason};
use crate::contract::secret::{SecretIn, SecretOut};
use crate::contract::session::{
    CreateAccountRequest, CreatedAccount, EmergencyKit, KitVerification, QuickUnlockRequest,
    SessionState, UnlockRequest, VerifyKitRequest,
};
use crate::state::{AppState, Backend, Slot, guard};

/// Longest accepted account email (RFC 5321 path limit).
const MAX_EMAIL_LEN: usize = 254;
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

/// `unlock_quick`: not built yet (unlock track: OS keystore, biometrics,
/// FIDO2, CRYPTO_SPEC §6.2 and §6.3).
#[tauri::command]
pub async fn unlock_quick(
    state: State<'_, AppState>,
    request: QuickUnlockRequest,
) -> IpcResult<SessionState> {
    let _ = (state, request.method);
    Err(IpcError::new(ErrorCode::QuickUnlockUnavailable))
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

    fn state_of(&self, slot: &Slot) -> IpcResult<SessionState> {
        let email = guard(&self.email)?.clone();
        match slot {
            Slot::Unavailable(code) => Err(IpcError::new(*code)),
            Slot::Taken => Err(IpcError::new(ErrorCode::Busy)),
            Slot::Locked(locked) => {
                if locked.has_account()? {
                    Ok(SessionState::Locked {
                        email,
                        secret_key_stored: self.secret_keys.load()?.is_some(),
                        quick_unlock: Vec::new(),
                    })
                } else {
                    Ok(SessionState::NoAccount)
                }
            }
            Slot::Unlocked(session) => Ok(SessionState::Unlocked {
                account_id: session.account_id().into(),
                email,
            }),
        }
    }

    /// Creates the account and its first vault, stores the Secret Key and
    /// returns the Emergency Kit.
    ///
    /// # Errors
    /// `accountExists`, `alreadyUnlocked`, `invalidInput` (email or vault
    /// name), `emptyPassword`, or a store error.
    pub fn create_account(&self, request: CreateAccountRequest) -> IpcResult<CreatedAccount> {
        let email = validate_text(&request.email, MAX_EMAIL_LEN)?.to_owned();
        if !email.contains('@') {
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
        // so the user can retry the vault or the keystore from the app.
        let vault = session.create_vault(&vault_name);
        let account_id = session.account_id();
        *slot = Slot::Unlocked(Box::new(session));
        vault?;
        self.secret_keys.save(&created.secret_key)?;
        *guard(&self.email)? = Some(email.clone());
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

    /// Whether `request.last_group` matches the stored Secret Key.
    ///
    /// # Errors
    /// `secretKeyRequired` if this device holds no Secret Key.
    pub fn verify_emergency_kit(&self, request: &VerifyKitRequest) -> IpcResult<KitVerification> {
        let key = self
            .secret_keys
            .load()?
            .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
        Ok(KitVerification {
            matches: key.matches_last_group(request.last_group.expose_secret()),
        })
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
                .secret_keys
                .load()?
                .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?,
        };
        let password = master_password(request.password);

        let mut slot = self.slot()?;
        let locked = take_locked(&mut slot)?;
        match locked.unlock(&password, &secret_key) {
            Ok(session) => {
                *slot = Slot::Unlocked(Box::new(session));
                if typed {
                    self.secret_keys.save(&secret_key)?;
                }
                self.state_of(&slot)
            }
            Err(UnlockError { session, error }) => {
                *slot = Slot::Locked(session);
                Err(error.into())
            }
        }
    }

    /// Locks: drops every key and decrypted item (they zeroize), the last
    /// generated password, and tells the webview. Locking a locked vault is
    /// a no-op.
    ///
    /// # Errors
    /// The store's code when it is unavailable.
    pub fn lock(&self, reason: LockReason) -> IpcResult<SessionState> {
        let mut slot = self.slot()?;
        if matches!(*slot, Slot::Unlocked(_))
            && let Slot::Unlocked(session) = std::mem::replace(&mut *slot, Slot::Taken)
        {
            *slot = Slot::Locked(session.lock());
            *guard(&self.last_generated)? = None;
            self.emit(&AppEvent::Locked { reason });
        }
        self.state_of(&slot)
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
