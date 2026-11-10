//! The commands for a folder backend (ADR-0043, ADR-0034): choosing the
//! folder (the webview gets a `choiceId` and a display path, never a path to
//! send back), connecting this device's account, joining an account from a
//! folder, the folder's device list, and the master password change (a new
//! account file). Also the pieces both joins share and the master password
//! change command.

use std::sync::MutexGuard;

use mosaic_core::{AccountId, InstallId, MasterPassword, PasswordParams, SecretKey};
use mosaic_sync::folder::FolderTransport;
use mosaic_sync::folder::{
    self, FolderContents, FolderError, JoinError, JoinFailure, init, inspect,
};
use mosaic_sync::http::{StoredToken, hosted};
use mosaic_sync::transport::{Rejection, RemoteDevice, Transport, TransportError};
use mosaic_sync::wire::SessionCrypto;
use mosaic_vault::session::{
    LockedSession, PasswordChanged, PreparedPasswordChange, SyncConfig, SyncKind, UnlockedSession,
};

use super::access::BackendSession;
use super::hosted_flow::{code_of, device_name};
use super::open_folder;
use crate::commands::session::{master_password, take_locked};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::sync::{
    ChangePasswordRequest, FolderChoice, FolderConnectRequest, FolderState, JoinFolderRequest,
    JoinOutcome, RecoverPasswordRequest, SyncOverview,
};
use crate::state::{Backend, Slot};

/// The longest `choiceId` the commands accept (the ones Rust hands out are
/// 39 characters).
const MAX_CHOICE_ID: usize = 64;

fn choice_id(id: &str) -> IpcResult<&str> {
    if id.len() > MAX_CHOICE_ID {
        return Err(IpcError::new(ErrorCode::InvalidInput));
    }
    Ok(id)
}

impl Backend {
    /// `sync_pick_folder`: asks the user for a folder (a native dialog), looks
    /// into it and remembers it for five minutes under a random `choiceId`.
    /// `None` when the user cancels.
    ///
    /// # Errors
    /// The store's code when it is unavailable; `internal` if no dialog can
    /// be shown.
    pub fn sync_pick_folder(&self) -> IpcResult<Option<FolderChoice>> {
        {
            let slot = self.slot()?;
            match &*slot {
                Slot::Unavailable(code) => return Err(IpcError::new(*code)),
                Slot::Taken => return Err(IpcError::new(ErrorCode::Busy)),
                Slot::Locked(_) | Slot::Unlocked(_) => {}
            }
        }
        let Some(path) = self.dialogs.pick_folder()? else {
            return Ok(None);
        };
        let (contents, accounts) = match inspect(&path) {
            FolderContents::Empty => (FolderState::Empty, Vec::new()),
            FolderContents::Mosaic { accounts } => (FolderState::Mosaic, accounts),
            FolderContents::Missing | FolderContents::OtherVersion | FolderContents::Foreign => {
                (FolderState::Other, Vec::new())
            }
        };
        let choice_id = self.sync.remember_choice(path.clone())?;
        Ok(Some(FolderChoice {
            choice_id,
            display_path: path.display().to_string(),
            contents,
            accounts: accounts.into_iter().map(Into::into).collect(),
        }))
    }

    /// `sync_folder_connect`: uses the chosen folder for this device's
    /// account: an empty folder is initialized, a Mosaic folder is joined as
    /// one more account (a team folder holds several side by side), and the
    /// account file is written once.
    ///
    /// # Errors
    /// `syncChoiceExpired`, `syncFolderNotEmpty`, `syncFolderUnavailable`,
    /// `syncAlreadyConfigured`, `invalidInput` (device name), `locked`.
    pub fn sync_folder_connect(&self, request: &FolderConnectRequest) -> IpcResult<SyncOverview> {
        let device = device_name(&request.device_name)?;
        let root = self.sync.choice(choice_id(&request.choice_id)?)?;
        let (account, install, bootstrap) = self.with_unlocked(|session| {
            if session.sync_config()?.is_some() {
                return Err(IpcError::new(ErrorCode::SyncAlreadyConfigured));
            }
            Ok((
                session.account_id(),
                session.install_id(),
                session.account_bootstrap()?,
            ))
        })?;
        init(&root).map_err(folder_code)?;
        let mut transport = open_folder(self.sync.session()?, &root, account, install, &device)?;
        let wrote_first = transport.highest_account_version()? == 0;
        // The version to record once the old backend's state is cleared.
        let known = if wrote_first {
            transport.write_account_file(&bootstrap.params, &bootstrap.wrapped_ak, 1)?;
            Some(1)
        } else {
            // A folder that already holds this account: a password this
            // device retired (or an older one) is replaced by this device's
            // own, a newer one is left for the recovery (the folder thread
            // flags it). The stored version belongs to the old backend.
            self.reconcile_folder_account(&mut transport, Some(0))?
                .version()
        };
        drop(transport);
        self.with_unlocked(|session| {
            session.set_sync_config(Some(&SyncConfig {
                kind: SyncKind::Folder,
                server_url: None,
                email: None,
                folder_path: Some(root.clone()),
                device_name: device.clone(),
            }))?;
            session.reset_sync_state()?;
            if let Some(version) = known {
                session.raise_folder_account_version(version)?;
            }
            Ok(())
        })?;
        self.sync.forget_choice(&request.choice_id);
        self.sync_restart();
        self.sync_status()
    }

    /// `join_folder`: joins an account that a folder holds from a new
    /// device: the account file is opened with the master password and the
    /// Secret Key and its MAC is checked. A file whose MAC fails
    /// (`syncFolderDamaged`) was written by someone without the account key:
    /// the new store is deleted.
    ///
    /// # Errors
    /// `syncChoiceExpired`, `syncFolderNoAccount`, `syncFolderUnavailable`,
    /// `syncFolderDamaged`, `wrongCredentials`, `accountExists`,
    /// `alreadyUnlocked`, `secretKeyMalformed`, `secretKeyMistyped`,
    /// `emptyPassword`.
    pub fn join_folder(&self, request: JoinFolderRequest) -> IpcResult<JoinOutcome> {
        let device = device_name(&request.device_name)?;
        let root = self.sync.choice(choice_id(&request.choice_id)?)?;
        let secret_key = SecretKey::parse(request.secret_key.expose_secret())?;
        let password = master_password(request.password);
        if password.expose_secret().is_empty() {
            return Err(IpcError::new(ErrorCode::EmptyPassword));
        }
        self.join_prerequisites()?;
        let mut slot = self.slot()?;
        let locked = take_locked(&mut slot)?;
        match folder::join_from_folder(
            &root,
            request.account_id.get(),
            locked,
            &password,
            &secret_key,
        ) {
            Ok(session) => {
                let config = SyncConfig {
                    kind: SyncKind::Folder,
                    server_url: None,
                    email: None,
                    folder_path: Some(root),
                    device_name: device,
                };
                self.complete_join(slot, session, &secret_key, None, &config, None)?;
                self.sync.forget_choice(&request.choice_id);
                Ok(JoinOutcome::Unlocked)
            }
            Err(failure) => {
                let JoinFailure { error, session } = *failure;
                match session {
                    Some(session) => *slot = Slot::Locked(session),
                    None => self.discard_new_store(&mut slot),
                }
                Err(join_code(error))
            }
        }
    }

    /// Finishes a join: the device is set up as if it had held the account
    /// from the start. The Secret Key goes to the keystore, the hosted token
    /// (if any) too, the backend choice into the store, the settings account
    /// block records the account with `kitVerified` (the Secret Key was typed
    /// from the kit), and the session is unlocked. No `reset_sync_state`: the
    /// store is new.
    pub(super) fn complete_join(
        &self,
        mut slot: MutexGuard<'_, Slot>,
        mut session: UnlockedSession,
        secret_key: &SecretKey,
        email: Option<&str>,
        config: &SyncConfig,
        token: Option<&StoredToken>,
    ) -> IpcResult<()> {
        let account_id = session.account_id();
        let setup = session
            .set_sync_config(Some(config))
            .map_err(IpcError::from)
            .and_then(|()| self.secret_keys.save(account_id, secret_key))
            .and_then(|()| token.map_or(Ok(()), |t| self.sync.save_token(self, account_id, t)));
        if let Err(error) = setup {
            // The account exists in the store; the user unlocks it with the
            // typed Secret Key.
            *slot = Slot::Locked(session.lock());
            return Err(error);
        }
        *slot = Slot::Unlocked(Box::new(session));
        self.settings_unlocked(&mut slot);
        self.remember_account(&mut slot, account_id, email.map(str::to_owned));
        let _ = self.update_stored_in(&mut slot, |stored| {
            if let Some(account) = stored.account.as_mut() {
                account.kit_verified = true;
            }
        });
        self.after_unlock(&slot);
        self.touch();
        self.reconcile_slot(&mut slot);
        Ok(())
    }

    /// Deletes the store a failed join created (a folder whose account file
    /// does not verify) and opens an empty one in its place.
    fn discard_new_store(&self, slot: &mut Slot) {
        match &self.store_dir {
            Some(dir) => {
                let base = dir.join(crate::STORE_FILE);
                for suffix in ["", "-wal", "-shm"] {
                    let mut name = base.clone().into_os_string();
                    name.push(suffix);
                    let _ = std::fs::remove_file(name);
                }
                let mut attachments = base.into_os_string();
                attachments.push("-attachments");
                let _ = std::fs::remove_dir_all(attachments);
                *slot = crate::open_store(dir);
            }
            None => *slot = Slot::Unavailable(ErrorCode::StoreUnavailable),
        }
    }

    /// The devices a folder lists (every device directory with its sealed
    /// info and checkpoint).
    pub(super) fn folder_devices(
        &self,
        config: &SyncConfig,
        account: AccountId,
        install: InstallId,
    ) -> IpcResult<Vec<RemoteDevice>> {
        let root = config
            .folder_path
            .as_deref()
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        let mut transport = open_folder(
            self.sync.session()?,
            root,
            account,
            install,
            &config.device_name,
        )?;
        Ok(transport.devices()?)
    }

    /// Compares the folder's highest verified account file with this
    /// device's stored parameters and wrapping (CRYPTO_SPEC §10.1,
    /// "Reconnecting to a folder"). `stored` overrides the stored folder
    /// account version (a connect passes 0: the version belongs to the
    /// backend that is being left). Writes nothing to the store; the caller
    /// records [`FolderAccount::version`].
    ///
    /// # Errors
    /// The transport's codes, `locked`.
    pub(crate) fn reconcile_folder_account(
        &self,
        transport: &mut FolderTransport<SessionCrypto<BackendSession>>,
        stored: Option<u64>,
    ) -> IpcResult<FolderAccount> {
        let Some(file) = transport.verified_account_file()? else {
            return Ok(FolderAccount::InSync(None));
        };
        let (local, retired, stored_version) = self.with_unlocked(|session| {
            Ok((
                session.account_bootstrap()?,
                session.salt_retired(&file.salt)?,
                session.folder_account_version()?,
            ))
        })?;
        let same = local.wrapped_ak.as_bytes() == file.wrapped_ak.as_bytes()
            && local.params.salt == file.salt
            && local.params.profile == file.profile;
        if same {
            return Ok(FolderAccount::InSync(Some(file.account_version)));
        }
        let stored = stored.unwrap_or(stored_version);
        if retired || file.account_version <= stored {
            let next = transport.next_account_version()?;
            transport.write_account_file(&local.params, &local.wrapped_ak, next)?;
            return Ok(FolderAccount::Rewrote(next));
        }
        Ok(FolderAccount::Changed)
    }

    // ---- master password -----------------------------------------------------------

    /// `change_master_password`: re-wraps the account key under the new
    /// password. With a hosted sync account the server changes first (it
    /// needs to be reachable: `syncPasswordNeedsServer`, nothing has changed
    /// then); with a folder the new account file is written first. Quick
    /// unlock is removed, as before M6 (CRYPTO_SPEC §6.2).
    ///
    /// # Errors
    /// `locked`, `emptyPassword`, `wrongCredentials` (counts towards the lock),
    /// `secretKeyRequired`, `syncPasswordNeedsServer`, `syncTwoFactorRequired`,
    /// and the transport's `sync*` codes.
    pub fn change_master_password(&self, request: &ChangePasswordRequest) -> IpcResult<()> {
        let current = MasterPassword::new(request.current_password.expose_secret().to_owned());
        let new = MasterPassword::new(request.new_password.expose_secret().to_owned());
        if current.expose_secret().is_empty() || new.expose_secret().is_empty() {
            return Err(IpcError::new(ErrorCode::EmptyPassword));
        }
        self.verify_master_only(&current)?;
        // From here until the local commit a lock waits (at most three
        // minutes): the device must not lock between the backend's change
        // and its own.
        let _change = self.sync.begin_change()?;
        let (account, install, config, prepared) = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            let key = self
                .secret_key_of(session.account_id())
                .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
            (
                session.account_id(),
                session.install_id(),
                session.sync_config()?,
                session.prepare_master_password_change(&current, &new, &key)?,
            )
        };
        drop((current, new));
        let changed = self.apply_credential_change(
            config,
            account,
            install,
            request.code.as_ref(),
            prepared,
        )?;
        self.remove_platform_key(&changed);
        Ok(())
    }

    /// `sync_recover_password`: finishes a master password change that the
    /// server (or the folder) holds and this device missed: this device was
    /// locked, closed or killed between the backend's change and its own
    /// commit, or another device changed the password. The user types the new
    /// password; the device derives the keys under the backend's parameters
    /// and adopts them only if they open the backend's wrapping to this
    /// session's own account key
    /// ([`UnlockedSession::adopt_remote_password`], CRYPTO_SPEC §9.4). A wrong
    /// password counts towards the five-failure lock.
    ///
    /// # Errors
    /// `locked`, `syncNotConfigured`, `emptyPassword`, `invalidInput` (code),
    /// `wrongCredentials`, `secretKeyRequired`, `syncTwoFactorRequired`,
    /// `syncTwoFactorInvalid`, `syncFolderNoAccount`, and the transport's
    /// `sync*` codes.
    pub fn sync_recover_password(
        &self,
        request: &RecoverPasswordRequest,
    ) -> IpcResult<SyncOverview> {
        // A Secret Key from the newest kit: the keys were renewed elsewhere (M7).
        if let Some(typed) = request.secret_key.as_ref() {
            return self.recover_with_new_key(request, typed);
        }
        let new = MasterPassword::new(request.new_password.expose_secret().to_owned());
        if new.expose_secret().is_empty() {
            return Err(IpcError::new(ErrorCode::EmptyPassword));
        }
        let code = code_of(request.code.as_ref())?;
        let epoch = self.sync.epoch();
        let (account, install, config) = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            (
                session.account_id(),
                session.install_id(),
                session
                    .sync_config()?
                    .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?,
            )
        };
        let key = self
            .secret_key_of(account)
            .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
        let changed = match config.kind {
            SyncKind::Hosted => {
                self.recover_hosted(&config, account, install, &new, &key, code, epoch)?
            }
            SyncKind::Folder => {
                self.recover_folder(&config, account, install, &new, &key, epoch)?
            }
        };
        drop(new);
        self.quick_unlock.clear_reauth_failures();
        self.remove_platform_key(&changed);
        self.sync.update_view(|view| {
            view.password_changed = false;
            true
        });
        self.emit_sync_state();
        self.sync_status()
    }

    /// The vault's answer to an adoption: a wrong password or another account
    /// key counts like a wrong password anywhere else.
    fn counted(&self, error: IpcError) -> IpcError {
        if error.code == ErrorCode::WrongCredentials {
            self.count_wrong_master()
        } else {
            error
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one value per thing the recovery reads from the stored account"
    )]
    pub(super) fn recover_hosted(
        &self,
        config: &SyncConfig,
        account: AccountId,
        install: InstallId,
        new: &MasterPassword,
        key: &SecretKey,
        code: Option<&str>,
        epoch: u64,
    ) -> IpcResult<PasswordChanged> {
        let url = config
            .server_url
            .as_deref()
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        let email = config.email.clone().unwrap_or_default();
        // The login ends this device's older session.
        self.sync.stop();
        let result =
            hosted::recover_password(url, &email, new, key, install, &config.device_name, code);
        let outcome = (|| {
            let recovered = match result {
                Ok(recovered) => recovered,
                Err(TransportError::Rejected(Rejection::LoginFailed)) => {
                    return Err(self.count_wrong_master());
                }
                Err(error) => return Err(error.into()),
            };
            self.sync.check_epoch(epoch)?;
            let hosted::Recovered {
                keys,
                account: remote,
                grant,
            } = recovered;
            let params = *keys.params();
            let adopted = self
                .with_unlocked(|session| {
                    Ok(session.adopt_remote_password(keys, &remote.wrapped_ak, params)?)
                })
                .map_err(|e| self.counted(e))?;
            let token = StoredToken::from_session(&grant.token);
            if self.sync.save_token(self, account, &token).is_err() {
                let _ = self.sync_tokens.delete(account);
                self.sync_force_signed_out();
            }
            Ok(adopted)
        })();
        self.sync_restart();
        outcome
    }

    pub(super) fn recover_folder(
        &self,
        config: &SyncConfig,
        account: AccountId,
        install: InstallId,
        new: &MasterPassword,
        key: &SecretKey,
        epoch: u64,
    ) -> IpcResult<PasswordChanged> {
        let root = config
            .folder_path
            .as_deref()
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        let transport = open_folder(
            self.sync.session()?,
            root,
            account,
            install,
            &config.device_name,
        )?;
        // Only a file whose MAC verifies under this account's own key counts.
        let file = transport
            .verified_account_file()?
            .ok_or(IpcError::new(ErrorCode::SyncFolderNoAccount))?;
        drop(transport);
        // A file at or below the version this device wrote or adopted last is
        // an older password offered again (CRYPTO_SPEC 9.4).
        let last = self.with_unlocked(|session| Ok(session.folder_account_version()?))?;
        if file.account_version <= last {
            return Err(IpcError::new(ErrorCode::SyncPasswordRollback));
        }
        let params = PasswordParams {
            account_id: file.account_id,
            salt: file.salt,
            profile: file.profile,
        };
        let keys = LockedSession::derive_join_keys(&params, new, key)?;
        self.sync.check_epoch(epoch)?;
        self.with_unlocked(|session| {
            let changed = session.adopt_remote_password(keys, &file.wrapped_ak, params)?;
            session.raise_folder_account_version(file.account_version)?;
            Ok(changed)
        })
        .map_err(|e| self.counted(e))
    }

    fn change_password_folder(
        &self,
        config: &SyncConfig,
        account: AccountId,
        install: InstallId,
        prepared: PreparedPasswordChange,
    ) -> IpcResult<PasswordChanged> {
        let root = config
            .folder_path
            .as_deref()
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        let mut transport = open_folder(
            self.sync.session()?,
            root,
            account,
            install,
            &config.device_name,
        )?;
        let version = transport.next_account_version()?;
        transport.write_account_file(prepared.new_params(), prepared.new_wrapped_ak(), version)?;
        drop(transport);
        self.with_unlocked(|session| {
            let changed = session.commit_master_password_change(prepared)?;
            session.raise_folder_account_version(version)?;
            Ok(changed)
        })
    }

    /// The platform key of a quick-unlock enrollment the change deleted.
    fn remove_platform_key(&self, changed: &PasswordChanged) {
        self.remove_platform_enrollment(changed.removed_quick_unlock.as_ref());
    }

    /// Applies a prepared credential change on the backend that holds the
    /// account (the server first, the folder's account file first, or just the
    /// store) and commits it here.
    pub(super) fn apply_credential_change(
        &self,
        config: Option<SyncConfig>,
        account: AccountId,
        install: InstallId,
        code: Option<&crate::contract::secret::SecretIn>,
        prepared: PreparedPasswordChange,
    ) -> IpcResult<PasswordChanged> {
        let changed = match config {
            Some(config) if config.kind == SyncKind::Hosted => {
                let ctx = self.hosted_ctx()?;
                self.change_password_hosted(&ctx, code, prepared)
            }
            Some(config) if config.kind == SyncKind::Folder => {
                self.change_password_folder(&config, account, install, prepared)
            }
            _ => self.with_unlocked(|session| Ok(session.commit_master_password_change(prepared)?)),
        };
        if changed.is_ok() {
            // The vault committed new credentials (password, Secret Key or
            // profile): every CLI session is bound to the old ones.
            self.delete_cli_sessions();
        }
        changed
    }
}

/// What [`Backend::reconcile_folder_account`] found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FolderAccount {
    /// The folder holds this device's parameters (at that version), or no
    /// account file at all.
    InSync(Option<u64>),
    /// The folder held a retired or older password: this device's own was
    /// written at that version.
    Rewrote(u64),
    /// Another device of the folder changed the password (a newer file that
    /// is not retired): the recovery finishes it.
    Changed,
}

impl FolderAccount {
    /// The folder account version to record, if there is one.
    pub(crate) const fn version(self) -> Option<u64> {
        match self {
            Self::InSync(v) => v,
            Self::Rewrote(v) => Some(v),
            Self::Changed => None,
        }
    }
}

fn folder_code(error: FolderError) -> IpcError {
    IpcError::new(match error {
        FolderError::NotEmpty => ErrorCode::SyncFolderNotEmpty,
        FolderError::Unsupported => ErrorCode::SyncServerIncompatible,
        _ => ErrorCode::SyncFolderUnavailable,
    })
}

fn join_code(error: JoinError) -> IpcError {
    match error {
        JoinError::Folder(e) => folder_code(e),
        JoinError::NoAccountFile => IpcError::new(ErrorCode::SyncFolderNoAccount),
        JoinError::WrongCredentials => IpcError::new(ErrorCode::WrongCredentials),
        JoinError::FolderDamaged => IpcError::new(ErrorCode::SyncFolderDamaged),
        JoinError::Session(e) => e.into(),
        _ => IpcError::INTERNAL,
    }
}
