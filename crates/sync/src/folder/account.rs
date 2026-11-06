//! The account file `.macct` and joining from a folder (ADR-0043 "Join from
//! folder", CRYPTO_SPEC §10.1).
//!
//! The current account file is the one with the highest `account_version`
//! (a tie goes to the lowest device ID). A device that joins reads it before
//! it holds `AK`: the wrapped key's AEAD authenticates the parameters, and
//! the file's MAC is checked right after the unlock.

use std::path::Path;

use mosaic_core::{AccountId, MasterPassword, PasswordParams, SecretKey, WrappedKey};
use mosaic_vault::session::{Error as SessionError, LockedSession, UnlockedSession};

use super::init::{FolderContents, FolderError, inspect};
use super::layout::Layout;
use super::{FolderTransport, fsutil};
use crate::transport::TransportError;
use crate::wire::SyncCrypto;
use crate::wire::account_file::{self, AccountFile, AccountFilePath};
use crate::wire::folder_names::{self, FolderName};
use mosaic_core::InstallId;

/// The account files of a folder, best first: highest version, then lowest
/// device ID. Each comes with the path IDs and the file's bytes.
fn candidates(
    layout: &Layout,
    account: AccountId,
) -> Result<Vec<(AccountFilePath, Vec<u8>)>, TransportError> {
    let mut found: Vec<(u64, [u8; 16])> = fsutil::list(&layout.account_files())?
        .iter()
        .filter(|e| !e.is_dir)
        .filter_map(|e| match folder_names::parse(&e.name) {
            Some(FolderName::Account { version, device }) => Some((version, device)),
            _ => None,
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut out = Vec::new();
    for (version, device) in found {
        let Ok(device_id) = InstallId::from_bytes(device) else {
            continue;
        };
        let name = folder_names::account_name(version, &device);
        let Some(bytes) =
            fsutil::read_capped(&layout.account_files().join(name), account_file::LEN as u64)?
        else {
            continue;
        };
        out.push((
            AccountFilePath {
                account_id: account,
                account_version: version,
                device_id,
            },
            bytes,
        ));
    }
    Ok(out)
}

/// The current account file of `account` in the folder `root` (the best
/// candidate that parses), with its bytes; `None` if there is none.
///
/// # Errors
/// [`TransportError`] for a folder that cannot be read.
pub fn current_account_file(
    root: &Path,
    account: AccountId,
) -> Result<Option<(AccountFile, Vec<u8>)>, TransportError> {
    let layout = Layout::new(root, &account);
    for (path, bytes) in candidates(&layout, account)? {
        if let Ok(file) = AccountFile::decode_for_join(&bytes, &path) {
            return Ok(Some((file, bytes)));
        }
    }
    Ok(None)
}

/// Why joining from a folder failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum JoinError {
    /// The folder is not usable ([`FolderError`]).
    #[error("folder unavailable")]
    Folder(#[source] FolderError),
    /// The folder has no such account, or no account file that parses.
    #[error("no account file")]
    NoAccountFile,
    /// The master password or Secret Key does not open the account.
    #[error("wrong master password or Secret Key")]
    WrongCredentials,
    /// The unlock worked but the account file's MAC does not verify: the
    /// file was written by someone without the account key
    /// (`syncFolderDamaged`). The session is dropped; the caller deletes the
    /// new store.
    #[error("account file damaged")]
    FolderDamaged,
    /// Another session error.
    #[error("session error")]
    Session(#[source] SessionError),
}

/// A failed join: the error, and the locked session back where it can be
/// reused (not after a MAC failure).
#[derive(Debug)]
pub struct JoinFailure {
    /// Why.
    pub error: JoinError,
    /// The session, if the join left it unchanged.
    pub session: Option<LockedSession>,
}

#[expect(
    clippy::unnecessary_box_returns,
    reason = "the error is boxed so the Result stays small"
)]
fn fail(error: JoinError, session: Option<LockedSession>) -> Box<JoinFailure> {
    Box::new(JoinFailure { error, session })
}

/// Joins the account `account` of the folder `root` with the master
/// password and Secret Key: reads the current account file, derives the
/// keys under its parameters, opens the wrapped `AK`, then checks the file's
/// MAC.
///
/// # Errors
/// [`JoinFailure`] with a [`JoinError`].
pub fn join_from_folder(
    root: &Path,
    account: AccountId,
    session: LockedSession,
    password: &MasterPassword,
    secret_key: &SecretKey,
) -> Result<UnlockedSession, Box<JoinFailure>> {
    match inspect(root) {
        FolderContents::Mosaic { accounts } if accounts.contains(&account) => {}
        FolderContents::Mosaic { .. } => return Err(fail(JoinError::NoAccountFile, Some(session))),
        FolderContents::Missing | FolderContents::Empty | FolderContents::Foreign => {
            return Err(fail(
                JoinError::Folder(FolderError::Unavailable),
                Some(session),
            ));
        }
        FolderContents::OtherVersion => {
            return Err(fail(
                JoinError::Folder(FolderError::Unsupported),
                Some(session),
            ));
        }
    }
    let current = match current_account_file(root, account) {
        Ok(Some(c)) => c,
        Ok(None) => return Err(fail(JoinError::NoAccountFile, Some(session))),
        Err(_) => {
            return Err(fail(JoinError::Folder(FolderError::Io), Some(session)));
        }
    };
    let (file, bytes) = current;
    let params = PasswordParams {
        account_id: file.account_id,
        salt: file.salt,
        profile: file.profile,
    };
    let keys = match LockedSession::derive_join_keys(&params, password, secret_key) {
        Ok(k) => k,
        Err(e) => return Err(fail(JoinError::Session(e), Some(session))),
    };
    let unlocked = match session.join_account_with_keys(&file.wrapped_ak, keys) {
        Ok(u) => u,
        Err(failed) => {
            let error = match failed.error {
                SessionError::UnlockFailed => JoinError::WrongCredentials,
                other => JoinError::Session(other),
            };
            return Err(fail(error, Some(failed.session)));
        }
    };
    match AccountFile::verify_mac(&bytes, &unlocked) {
        Ok(true) => Ok(unlocked),
        Ok(false) | Err(_) => Err(fail(JoinError::FolderDamaged, None)),
    }
}

impl<C: SyncCrypto + Send> FolderTransport<C> {
    /// Writes this install's account file for `version` (the first one is
    /// 1; a master password change writes the next) and removes this
    /// install's account files of lower versions.
    ///
    /// # Errors
    /// [`TransportError`].
    pub fn write_account_file(
        &mut self,
        params: &PasswordParams,
        wrapped_ak: &WrappedKey,
        version: u64,
    ) -> Result<(), TransportError> {
        let file = AccountFile {
            account_id: self.account,
            account_version: version,
            device_id: self.install,
            salt: params.salt,
            profile: params.profile,
            wrapped_ak: wrapped_ak.clone(),
        };
        let bytes = file
            .encode(&self.crypto)
            .map_err(|_| TransportError::Protocol)?;
        let dir = self.layout.account_files();
        fsutil::write_atomic(
            &dir,
            &folder_names::account_name(version, self.install.as_bytes()),
            &bytes,
            true,
        )?;
        for entry in fsutil::list(&dir)? {
            if let Some(FolderName::Account { version: v, device }) =
                folder_names::parse(&entry.name)
                && v < version
                && device == *self.install.as_bytes()
            {
                fsutil::remove(&dir.join(&entry.name))?;
            }
        }
        Ok(())
    }

    /// The highest account version in the folder (0 if there is none), by
    /// file name.
    ///
    /// # Errors
    /// [`TransportError`].
    pub fn highest_account_version(&self) -> Result<u64, TransportError> {
        Ok(fsutil::list(&self.layout.account_files())?
            .iter()
            .filter_map(|e| match folder_names::parse(&e.name) {
                Some(FolderName::Account { version, .. }) => Some(version),
                _ => None,
            })
            .max()
            .unwrap_or(0))
    }
}
