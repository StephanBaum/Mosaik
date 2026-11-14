//! Vault commands: list, create, rename, and the hardware-protected
//! one-time-code vaults (CRYPTO_SPEC §7.4; ADR-0028): create, open with a
//! security key for five minutes, close, and manage the vault's keys.
//!
//! The device calls run without the session held (see
//! `commands/quick_unlock.rs`); the deadline of an open vault is a
//! monotonic `Instant` in the quick-unlock service, and the auto-lock tick
//! closes the vault when it passes.

use std::time::{Instant, SystemTime};

use mosaic_core::unlock::new_prf_salt;
use mosaic_core::{AccountId, VaultId};
use mosaic_vault::session::{
    DEFAULT_RP_ID, MAX_HARDWARE_KEYS, NewHardwareKey, UnlockedSession,
    VaultProtection as SessionProtection,
};
use tauri::State;

use super::quick_unlock::{changed_while_waiting, parse_method_id, summaries};
use super::session::MAX_VAULT_NAME_LEN;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::contract::quick_unlock::{
    CreateHardwareVaultRequest, HardwareVaultKeyRequest, HardwareVaultKeys, HardwareVaultRequest,
};
use crate::contract::secret::SecretIn;
use crate::contract::session::ReauthPurpose;
use crate::contract::vaults::{
    CreateVaultRequest, RenameVaultRequest, VaultProtection, VaultRequest, VaultSummary,
};
use crate::state::{AppState, Backend, Slot};

/// `list_vaults`.
#[tauri::command]
pub async fn list_vaults(state: State<'_, AppState>) -> IpcResult<Vec<VaultSummary>> {
    state.run(Backend::list_vaults).await
}

/// `create_vault`.
#[tauri::command]
pub async fn create_vault(
    state: State<'_, AppState>,
    request: CreateVaultRequest,
) -> IpcResult<VaultSummary> {
    state.run(move |b| b.create_vault(&request)).await
}

/// `rename_vault`.
#[tauri::command]
pub async fn rename_vault(
    state: State<'_, AppState>,
    request: RenameVaultRequest,
) -> IpcResult<VaultSummary> {
    state.run(move |b| b.rename_vault(&request)).await
}

/// `create_hardware_vault`.
#[tauri::command]
pub async fn create_hardware_vault(
    state: State<'_, AppState>,
    request: CreateHardwareVaultRequest,
) -> IpcResult<VaultSummary> {
    state
        .run(move |b| {
            let mut request = request;
            b.create_hardware_vault(&mut request)
        })
        .await
}

/// `open_hardware_vault`.
#[tauri::command]
pub async fn open_hardware_vault(
    state: State<'_, AppState>,
    request: HardwareVaultRequest,
) -> IpcResult<VaultSummary> {
    state
        .run(move |b| {
            let mut request = request;
            b.open_hardware_vault(&mut request)
        })
        .await
}

/// `close_hardware_vault`.
#[tauri::command]
pub async fn close_hardware_vault(
    state: State<'_, AppState>,
    request: VaultRequest,
) -> IpcResult<()> {
    state.run(move |b| b.close_hardware_vault(&request)).await
}

/// `hardware_vault_keys`.
#[tauri::command]
pub async fn hardware_vault_keys(
    state: State<'_, AppState>,
    request: VaultRequest,
) -> IpcResult<HardwareVaultKeys> {
    state.run(move |b| b.hardware_vault_keys(&request)).await
}

/// `add_hardware_vault_key`.
#[tauri::command]
pub async fn add_hardware_vault_key(
    state: State<'_, AppState>,
    request: HardwareVaultRequest,
) -> IpcResult<HardwareVaultKeys> {
    state
        .run(move |b| {
            let mut request = request;
            b.add_hardware_vault_key(&mut request)
        })
        .await
}

/// `remove_hardware_vault_key`.
#[tauri::command]
pub async fn remove_hardware_vault_key(
    state: State<'_, AppState>,
    request: HardwareVaultKeyRequest,
) -> IpcResult<HardwareVaultKeys> {
    state
        .run(move |b| b.remove_hardware_vault_key(&request))
        .await
}

fn vault_name(name: &str) -> IpcResult<&str> {
    let name = name.trim();
    if name.is_empty()
        || name.len() > MAX_VAULT_NAME_LEN
        || name
            .chars()
            .any(|c| c.is_control() || super::is_spoofing_char(c))
    {
        return Err(IpcError::new(ErrorCode::InvalidInput));
    }
    Ok(name)
}

impl Backend {
    /// A vault as the webview sees it. `openUntil` is the wall-clock
    /// projection of the monotonic deadline of an open hardware vault.
    fn vault_to_wire(
        &self,
        session: &UnlockedSession,
        vault: &mosaic_vault::session::VaultSummary<'_>,
    ) -> VaultSummary {
        let hardware = self
            .quick_unlock
            .vault_left(vault.vault_id, Instant::now(), SystemTime::now())
            .filter(|_| vault.available);
        let account_key = session
            .vault_protection(vault.vault_id)
            .map_or(true, |p| p == SessionProtection::AccountKey);
        VaultSummary {
            vault_id: vault.vault_id.into(),
            name: vault.name.map(str::to_owned),
            item_count: u32::try_from(vault.item_count).unwrap_or(u32::MAX),
            available: vault.available,
            protection: if account_key {
                VaultProtection::AccountKey
            } else {
                VaultProtection::HardwareKey
            },
            open_until: hardware.map(|left| {
                self.quick_unlock
                    .now_ms()
                    .saturating_add(i64::try_from(left.as_millis()).unwrap_or(i64::MAX))
            }),
        }
    }

    fn summary(&self, session: &UnlockedSession, vault_id: VaultId) -> IpcResult<VaultSummary> {
        session
            .vaults()
            .iter()
            .find(|v| v.vault_id == vault_id)
            .map(|v| self.vault_to_wire(session, v))
            .ok_or(IpcError::new(ErrorCode::VaultNotFound))
    }

    /// Every vault.
    ///
    /// # Errors
    /// `locked`.
    pub fn list_vaults(&self) -> IpcResult<Vec<VaultSummary>> {
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        Ok(session
            .vaults()
            .iter()
            .map(|v| self.vault_to_wire(session, v))
            .collect())
    }

    /// Creates a vault.
    ///
    /// # Errors
    /// `locked`, `invalidInput`, or a store error.
    pub fn create_vault(&self, request: &CreateVaultRequest) -> IpcResult<VaultSummary> {
        self.touch();
        let name = vault_name(&request.name)?;
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        let vault_id = session.create_vault(name)?;
        self.sync_poke();
        self.summary(session, vault_id)
    }

    /// Renames a vault.
    ///
    /// # Errors
    /// `locked`, `invalidInput`, `vaultNotFound`, `vaultUnavailable`.
    pub fn rename_vault(&self, request: &RenameVaultRequest) -> IpcResult<VaultSummary> {
        self.touch();
        let name = vault_name(&request.name)?;
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        session.rename_vault(request.vault_id.get(), name)?;
        self.sync_poke();
        self.summary(session, request.vault_id.get())
    }

    /// The session of `account` after a device call; `locked` (or the
    /// store's code) if the vault locked meanwhile.
    fn session_after_device(
        slot: &mut Slot,
        account: AccountId,
    ) -> IpcResult<&mut UnlockedSession> {
        if !matches!(&*slot, Slot::Unlocked(session) if session.account_id() == account) {
            return Err(changed_while_waiting(slot));
        }
        slot.unlocked_mut()
    }

    /// Registers a key on the device, dropping (zeroizing) the PIN as soon
    /// as the call is over.
    fn register_with_pin(
        &self,
        hardware: &dyn crate::quick_unlock::HardwareKeyPlatform,
        salt: &[u8; 32],
        pin: Option<SecretIn>,
    ) -> IpcResult<(Vec<u8>, mosaic_core::unlock::PrfOutput)> {
        let registered = self.device_op(|cancel| {
            hardware.register(
                DEFAULT_RP_ID,
                salt,
                pin.as_ref().map(SecretIn::as_secret),
                cancel,
            )
        });
        drop(pin);
        Ok(registered??)
    }

    fn require_hardware(&self) -> IpcResult<()> {
        if self.quick_unlock.hardware().is_supported() {
            Ok(())
        } else {
            Err(IpcError::new(ErrorCode::HardwareKeyUnsupported))
        }
    }

    /// Creates a hardware-protected one-time-code vault: registers a
    /// security key (touch, PIN) for it and leaves the vault open for five
    /// minutes.
    ///
    /// # Errors
    /// `locked`, `invalidInput` (the name), `hardwareKeyUnsupported`,
    /// `busy`, `cancelled`, the device codes, or a store error.
    pub fn create_hardware_vault(
        &self,
        request: &mut CreateHardwareVaultRequest,
    ) -> IpcResult<VaultSummary> {
        self.touch();
        let name = vault_name(&request.name)?;
        let account = self.slot()?.unlocked()?.account_id();
        self.require_hardware()?;
        let salt = new_prf_salt()?;
        let hardware = self.quick_unlock.hardware();
        let (credential_id, prf) = self.register_with_pin(hardware, &salt, request.pin.take())?;
        let key = NewHardwareKey::new(credential_id, DEFAULT_RP_ID.to_owned(), salt)?;

        let mut slot = self.slot()?;
        let session = Self::session_after_device(&mut slot, account)?;
        let created = session.create_hardware_vault(name, &key, &prf);
        drop(prf);
        let vault_id = created?;
        self.quick_unlock
            .note_vault_open(vault_id, Instant::now(), SystemTime::now());
        self.summary(session, vault_id)
    }

    /// Opens a hardware-protected vault with one of its security keys. It
    /// closes again after five minutes, on lock, or by hand.
    ///
    /// # Errors
    /// `locked`, `hardwareKeyUnsupported`, `vaultNotFound`, `invalidInput`
    /// (not a hardware-protected vault), `hardwareKeyNotRecognized`, `busy`,
    /// `cancelled`, the device codes, or a store error.
    pub fn open_hardware_vault(
        &self,
        request: &mut HardwareVaultRequest,
    ) -> IpcResult<VaultSummary> {
        self.touch();
        let vault_id = request.vault_id.get();
        let account = self.slot()?.unlocked()?.account_id();
        self.require_hardware()?;
        let keys = self.slot()?.unlocked()?.hardware_vault_keys(vault_id)?;
        let candidates: Vec<_> = keys
            .iter()
            .map(|key| crate::quick_unlock::HardwareCandidate {
                rp_id: &key.rp_id,
                credential_id: &key.credential_id,
                prf_salt: &key.prf_salt,
            })
            .collect();
        let hardware = self.quick_unlock.hardware();
        // The PIN is dropped (zeroized) as soon as the device call is over.
        let pin = request.pin.take();
        let evaluated = self.device_op(|cancel| {
            hardware.evaluate(&candidates, pin.as_ref().map(SecretIn::as_secret), cancel)
        });
        drop(pin);
        let (index, prf) = evaluated??;
        let credential = keys
            .get(index)
            .map(|key| key.credential_id.as_slice())
            .ok_or(IpcError::INTERNAL)?;

        let mut slot = self.slot()?;
        let session = Self::session_after_device(&mut slot, account)?;
        let opened = session.open_hardware_vault(vault_id, credential, &prf);
        drop(prf);
        opened?;
        self.quick_unlock
            .note_vault_open(vault_id, Instant::now(), SystemTime::now());
        self.summary(session, vault_id)
    }

    /// Closes a hardware-protected vault now (a closed one stays closed).
    ///
    /// # Errors
    /// `locked`, `vaultNotFound`, `invalidInput` (not a hardware-protected
    /// vault).
    pub fn close_hardware_vault(&self, request: &VaultRequest) -> IpcResult<()> {
        self.touch();
        let vault_id = request.vault_id.get();
        let mut slot = self.slot()?;
        slot.unlocked_mut()?.close_hardware_vault(vault_id)?;
        self.quick_unlock.note_vault_closed(vault_id);
        Ok(())
    }

    /// The security keys registered for a hardware-protected vault (works
    /// while it is closed).
    ///
    /// # Errors
    /// `locked`, `vaultNotFound`, `invalidInput` (not a hardware-protected
    /// vault).
    pub fn hardware_vault_keys(&self, request: &VaultRequest) -> IpcResult<HardwareVaultKeys> {
        let slot = self.slot()?;
        let keys = slot
            .unlocked()?
            .hardware_vault_keys(request.vault_id.get())?;
        Ok(HardwareVaultKeys {
            keys: summaries(&keys),
        })
    }

    /// Registers another security key for an open hardware-protected vault.
    ///
    /// # Errors
    /// `locked`, `hardwareKeyUnsupported`, `reauthRequired` (no right
    /// `reauthenticate` in the last two minutes), `vaultNotFound`,
    /// `invalidInput`, `vaultUnavailable` (the vault is closed),
    /// `hardwareKeyLimit`, `busy`,
    /// `cancelled`, the device codes, or a store error.
    pub fn add_hardware_vault_key(
        &self,
        request: &mut HardwareVaultRequest,
    ) -> IpcResult<HardwareVaultKeys> {
        self.touch();
        let vault_id = request.vault_id.get();
        self.slot()?.unlocked()?;
        self.require_hardware()?;
        // A new key opens the vault later: like enrolling one, it needs a
        // right master password within the last two minutes. The grant is
        // taken (of two parallel calls one gets it), spent by a success and
        // handed back by a failure.
        let grant = self.take_reauth_grant(ReauthPurpose::HardwareVaultKey)?;
        let added = self.add_hardware_vault_key_now(vault_id, request);
        if added.is_err() {
            self.quick_unlock.restore_reauth(grant);
        }
        added
    }

    fn add_hardware_vault_key_now(
        &self,
        vault_id: VaultId,
        request: &mut HardwareVaultRequest,
    ) -> IpcResult<HardwareVaultKeys> {
        let account = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            let keys = session.hardware_vault_keys(vault_id)?;
            if session.vault_protection(vault_id)?
                == (SessionProtection::HardwareKey { open: false })
            {
                return Err(IpcError::new(ErrorCode::VaultUnavailable));
            }
            if keys.len() >= MAX_HARDWARE_KEYS {
                return Err(IpcError::new(ErrorCode::HardwareKeyLimit));
            }
            session.account_id()
        };
        let salt = new_prf_salt()?;
        let hardware = self.quick_unlock.hardware();
        let (credential_id, prf) = self.register_with_pin(hardware, &salt, request.pin.take())?;
        let key = NewHardwareKey::new(credential_id, DEFAULT_RP_ID.to_owned(), salt)?;

        let mut slot = self.slot()?;
        let session = Self::session_after_device(&mut slot, account)?;
        let added = session.add_hardware_vault_key(vault_id, &key, &prf);
        drop(prf);
        added?;
        // One re-authentication, one new key (ADR-0028 item 8): the grant
        // was taken at the start and is not put back.
        Ok(HardwareVaultKeys {
            keys: summaries(&session.hardware_vault_keys(vault_id)?),
        })
    }

    /// Removes a security key from an open hardware-protected vault. The
    /// last key cannot be removed.
    ///
    /// # Errors
    /// `locked`, `invalidInput` (the key ID, or not a hardware-protected
    /// vault), `vaultNotFound`, `vaultUnavailable` (closed),
    /// `hardwareKeyLast`, `hardwareKeyNotRecognized`, or a store error.
    pub fn remove_hardware_vault_key(
        &self,
        request: &HardwareVaultKeyRequest,
    ) -> IpcResult<HardwareVaultKeys> {
        self.touch();
        let vault_id = request.vault_id.get();
        let method_id =
            parse_method_id(&request.method_id).ok_or(IpcError::new(ErrorCode::InvalidInput))?;
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        if !session.remove_hardware_vault_key(vault_id, &method_id)? {
            return Err(IpcError::new(ErrorCode::HardwareKeyNotRecognized));
        }
        Ok(HardwareVaultKeys {
            keys: summaries(&session.hardware_vault_keys(vault_id)?),
        })
    }

    /// Closes the hardware-protected vaults that are open past their five
    /// minutes and tells the webview (`hardwareVaultClosed`). Called by the
    /// auto-lock tick with the monotonic clock and the wall clock; a vault
    /// is due when **either** deadline passed. After a suspend (`slept`)
    /// every open hardware vault closes, whatever the lock-on-sleep
    /// setting says. An open vault without a deadline (it should not exist)
    /// closes too. A deadline is forgotten only after the close succeeded,
    /// so a failed close is retried on the next tick.
    pub fn close_expired_hardware_vaults(&self, now: Instant, wall: SystemTime, slept: bool) {
        let mut closed = Vec::new();
        if let Ok(mut slot) = self.slot()
            && let Slot::Unlocked(session) = &mut *slot
        {
            let due = self.quick_unlock.expired_vaults(now, wall);
            let open: Vec<VaultId> = session
                .vaults()
                .iter()
                .map(|v| v.vault_id)
                .filter(|id| {
                    matches!(
                        session.vault_protection(*id),
                        Ok(SessionProtection::HardwareKey { open: true })
                    )
                })
                .collect();
            for vault_id in &open {
                if (slept || due.contains(vault_id) || !self.quick_unlock.vault_tracked(*vault_id))
                    && session.close_hardware_vault(*vault_id).is_ok()
                {
                    self.quick_unlock.note_vault_closed(*vault_id);
                    closed.push(*vault_id);
                }
            }
            // A due deadline of a vault that is not open any more is stale.
            for vault_id in due {
                if !open.contains(&vault_id) {
                    self.quick_unlock.note_vault_closed(vault_id);
                }
            }
        }
        for vault_id in closed {
            self.emit(&AppEvent::HardwareVaultClosed {
                vault_id: vault_id.into(),
            });
        }
    }
}
