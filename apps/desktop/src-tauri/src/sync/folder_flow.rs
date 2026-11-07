//! The commands for a folder backend (ADR-0043, ADR-0034): choosing the
//! folder (the webview gets a `choiceId` and a display path, never a path to
//! send back), connecting this device's account, joining an account from a
//! folder, the folder's device list, and the master password change (a new
//! account file). Also the pieces both joins share and the master password
//! change command.

use std::sync::MutexGuard;

use mosaic_core::{AccountId, InstallId, MasterPassword, SecretKey};
use mosaic_sync::folder::{
    self, FolderContents, FolderError, JoinError, JoinFailure, init, inspect,
};
use mosaic_sync::http::StoredToken;
use mosaic_sync::transport::{RemoteDevice, Transport};
use mosaic_vault::session::{
    PasswordChanged, PreparedPasswordChange, SyncConfig, SyncKind, UnlockedSession,
};

use super::hosted_flow::device_name;
use super::open_folder;
use crate::commands::session::{master_password, take_locked};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::sync::{
    ChangePasswordRequest, FolderChoice, FolderConnectRequest, FolderState, JoinFolderRequest,
    JoinOutcome, SyncOverview,
};
use crate::state::{Backend, Slot};

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
        let root = self.sync.choice(&request.choice_id)?;
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
        if transport.highest_account_version()? == 0 {
            transport.write_account_file(&bootstrap.params, &bootstrap.wrapped_ak, 1)?;
        }
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
        let root = self.sync.choice(&request.choice_id)?;
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
            .and_then(|()| token.map_or(Ok(()), |t| self.sync_tokens.save(account_id, t)));
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
        let changed = match config {
            Some(config) if config.kind == SyncKind::Hosted => {
                let ctx = self.hosted_ctx()?;
                self.change_password_hosted(&ctx, request, prepared)?
            }
            Some(config) if config.kind == SyncKind::Folder => {
                self.change_password_folder(&config, account, install, prepared)?
            }
            _ => {
                self.with_unlocked(|session| Ok(session.commit_master_password_change(prepared)?))?
            }
        };
        self.remove_platform_key(&changed);
        Ok(())
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
        self.with_unlocked(|session| Ok(session.commit_master_password_change(prepared)?))
    }

    /// The platform key of a quick-unlock enrollment the change deleted.
    fn remove_platform_key(&self, changed: &PasswordChanged) {
        if let Some(enrollment) = &changed.removed_quick_unlock
            && let Some(platform) = self.quick_unlock.biometric()
        {
            let _ = platform.remove(&enrollment.platform_params);
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
