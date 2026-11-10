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
use super::{FolderCrypto, FolderTransport, fsutil, lock_or};
use crate::transport::TransportError;
use crate::wire::account_file::{self, AccountFile, AccountFilePath};
use crate::wire::folder_names::{self, FolderName};
use mosaic_core::InstallId;
use mosaic_core::sync_auth::FolderMacPurpose;

/// The most account files a join tries: each costs one Argon2id derivation,
/// and anyone with write access to the folder can plant files.
const MAX_JOIN_CANDIDATES: usize = 8;

/// The most newer account files a join checks against the key that opened an
/// older one (see the superseded-password rule).
const MAX_SUPERSEDED_CHECKS: usize = 8;

/// The most account files read from a folder (newest first).
const MAX_ACCOUNT_FILES_READ: usize = 64;

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
    found.truncate(MAX_ACCOUNT_FILES_READ);
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

/// Whether an account file of a higher version than candidate `index` (at
/// most [`MAX_SUPERSEDED_CHECKS`]) carries a MAC of the account key that
/// opened the candidate: the password was changed since, and the password
/// that opened the older file is superseded.
fn superseded(
    session: &LockedSession,
    all: &[(AccountFilePath, Vec<u8>)],
    index: usize,
    file: &AccountFile,
    keys: &mosaic_vault::session::JoinKeys,
) -> bool {
    let Some((path, _)) = all.get(index) else {
        return false;
    };
    all.iter()
        .take(index)
        .filter(|(p, _)| p.account_version > path.account_version)
        .take(MAX_SUPERSEDED_CHECKS)
        .filter_map(|(_, bytes)| bytes.split_last_chunk::<32>())
        .any(|(body, mac)| {
            session
                .verify_join_folder_mac(
                    &file.wrapped_ak,
                    keys,
                    FolderMacPurpose::Account,
                    body,
                    mac,
                )
                .unwrap_or(false)
        })
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
    let Ok(all) = candidates(&Layout::new(root, &account), account) else {
        return Err(fail(JoinError::Folder(FolderError::Io), Some(session)));
    };
    // Candidates in order (highest version, lowest device) until one unlocks
    // and its MAC verifies. A file somebody planted (another version of the
    // wrapped key, a copy with a bumped version) must not lock a device
    // out, so a candidate that does not open is skipped, and one that
    // opens but fails its MAC is remembered and skipped too.
    let mut parsed = false;
    let mut damaged = false;
    for (index, (path, bytes)) in all.iter().enumerate().take(MAX_JOIN_CANDIDATES) {
        let Ok(file) = AccountFile::decode_for_join(bytes, path) else {
            continue;
        };
        parsed = true;
        let params = PasswordParams {
            account_id: file.account_id,
            salt: file.salt,
            profile: file.profile,
        };
        let keys = match LockedSession::derive_join_keys(&params, password, secret_key) {
            Ok(k) => k,
            Err(e) => return Err(fail(JoinError::Session(e), Some(session))),
        };
        let Some((body, mac)) = bytes.split_last_chunk::<32>() else {
            continue;
        };
        match session.verify_join_folder_mac(
            &file.wrapped_ak,
            &keys,
            FolderMacPurpose::Account,
            body,
            mac,
        ) {
            Err(SessionError::UnlockFailed) => continue,
            Err(other) => return Err(fail(JoinError::Session(other), Some(session))),
            Ok(false) => {
                damaged = true;
                continue;
            }
            Ok(true) => {}
        }
        if superseded(&session, &all, index, &file, &keys) {
            return Err(fail(JoinError::WrongCredentials, Some(session)));
        }
        // The device starts at the epoch of the verified file (review H2).
        let unlocked =
            match session.join_account_with_keys_at(&file.wrapped_ak, keys, file.ak_epoch) {
                Ok(u) => u,
                Err(failed) => {
                    let error = match failed.error {
                        SessionError::UnlockFailed => JoinError::WrongCredentials,
                        other => JoinError::Session(other),
                    };
                    return Err(fail(error, Some(failed.session)));
                }
            };
        // The same check once more on the unlocked session.
        return match AccountFile::verify_mac(bytes, &unlocked) {
            Ok(true) => Ok(unlocked),
            Ok(false) | Err(_) => Err(fail(JoinError::FolderDamaged, None)),
        };
    }
    if damaged {
        // The store was not touched: the session is still usable.
        Err(fail(JoinError::FolderDamaged, Some(session)))
    } else if parsed {
        Err(fail(JoinError::WrongCredentials, Some(session)))
    } else {
        Err(fail(JoinError::NoAccountFile, Some(session)))
    }
}

impl<C: FolderCrypto> FolderTransport<C> {
    /// Writes this install's account file for `version` (the first one is
    /// 1; a master password change writes the next; a re-key writes the next
    /// one at the new epoch, which the account file carries) and removes this
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
            ak_epoch: self
                .crypto
                .ak_epoch()
                .map_err(|e| lock_or(e, TransportError::Protocol))?,
            device_id: self.install,
            salt: params.salt,
            profile: params.profile,
            wrapped_ak: wrapped_ak.clone(),
        };
        let bytes = file
            .encode(&self.crypto)
            .map_err(|e| lock_or(e, TransportError::Protocol))?;
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

    /// The highest account version in the folder whose file verifies (0 if
    /// there is none). A planted file does not count, however high its name.
    ///
    /// # Errors
    /// [`TransportError`].
    pub fn highest_account_version(&self) -> Result<u64, TransportError> {
        let mut found: Vec<(u64, [u8; 16])> = fsutil::list(&self.layout.account_files())?
            .iter()
            .filter(|e| !e.is_dir)
            .filter_map(|e| match folder_names::parse(&e.name) {
                Some(FolderName::Account { version, device }) => Some((version, device)),
                _ => None,
            })
            .collect();
        found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        Ok(self.highest_verified_account_version(&found)?.unwrap_or(0))
    }

    /// The highest-version account file whose MAC verifies under this
    /// session's account key (`None` if there is none): what a device that
    /// missed a master password change finds in the folder
    /// ([`mosaic_vault::session::UnlockedSession::adopt_remote_password`]).
    ///
    /// # Errors
    /// [`TransportError`].
    pub fn verified_account_file(&self) -> Result<Option<AccountFile>, TransportError> {
        let mut found: Vec<(u64, [u8; 16])> = fsutil::list(&self.layout.account_files())?
            .iter()
            .filter(|e| !e.is_dir)
            .filter_map(|e| match folder_names::parse(&e.name) {
                Some(FolderName::Account { version, device }) => Some((version, device)),
                _ => None,
            })
            .collect();
        found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        self.highest_verified_account_file(&found)
    }

    /// The version the next master password change writes: one above the
    /// highest verified version.
    ///
    /// # Errors
    /// [`TransportError::Protocol`] if the counter would overflow.
    pub fn next_account_version(&self) -> Result<u64, TransportError> {
        self.highest_account_version()?
            .checked_add(1)
            .ok_or(TransportError::Protocol)
    }
}
