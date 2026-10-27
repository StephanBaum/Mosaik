//! The one error type every command returns (`docs/DESKTOP_IPC.md`,
//! "Errors"). It carries a code and nothing else: no message, no ID, no
//! value from the vault, so it can never leak a secret. The webview maps
//! every code to the i18n key `error.<code>`, which says what happened and
//! what to do next (UX_GUIDELINES §5).

use mosaic_vault::session::Error as SessionError;
use mosaic_vault::storage::Error as StoreError;
use serde::Serialize;

use super::macros::wire_enum;

wire_enum!(
    /// Why a command failed.
    pub enum ErrorCode {
        /// The command is part of the contract but not built yet.
        NotImplemented,
        /// There is no account on this device yet.
        NoAccount,
        /// This device already has an account.
        AccountExists,
        /// The vault is locked; unlock first.
        Locked,
        /// The vault is already unlocked.
        AlreadyUnlocked,
        /// The master password or the Secret Key is wrong (never which one,
        /// CRYPTO_SPEC §5.4).
        WrongCredentials,
        /// This device does not hold the Secret Key; the user enters it from
        /// the Emergency Kit.
        SecretKeyRequired,
        /// The Secret Key is not in the `M1-` format.
        SecretKeyMalformed,
        /// The Secret Key's checksum does not match: a typo.
        SecretKeyMistyped,
        /// The master password is empty.
        EmptyPassword,
        /// The quick-unlock method is not set up or not available.
        QuickUnlockUnavailable,
        /// Another Mosaic process holds the local vault.
        StoreInUse,
        /// The local vault file could not be opened or read.
        StoreUnavailable,
        /// The vault does not exist.
        VaultNotFound,
        /// The vault's key is not available (hardware-protected or damaged).
        VaultUnavailable,
        /// The item does not exist.
        ItemNotFound,
        /// The item was deleted.
        ItemDeleted,
        /// Source and target vault of a move are the same.
        SameVault,
        /// The item has no one-time password.
        NoOtp,
        /// The field is empty, so there is nothing to copy or reveal.
        FieldEmpty,
        /// A request value is out of range or malformed.
        InvalidInput,
        /// The change needs the master password again (UX_GUIDELINES §8).
        ReauthRequired,
        /// The file is larger than 50 MiB.
        AttachmentTooLarge,
        /// The account's attachments would exceed 1 GiB.
        AttachmentQuota,
        /// The system clipboard could not be written.
        ClipboardUnavailable,
        /// The import file could not be read.
        ImportFileUnreadable,
        /// The import file is not in the chosen format.
        ImportFormatInvalid,
        /// The import preview is gone (cancelled, locked or too old).
        ImportPreviewExpired,
        /// The SSH approval request was already answered or expired.
        SshRequestExpired,
        /// The SSH agent could not start.
        SshAgentUnavailable,
        /// Another operation that needs the vault is still running.
        Busy,
        /// An unexpected internal failure; details are not shown.
        Internal,
    }
);

/// A failed command, as the webview receives it: `{ "code": "locked" }`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, thiserror::Error)]
#[error("IPC command failed: {code:?}")]
pub struct IpcError {
    /// What happened.
    pub code: ErrorCode,
}

impl IpcError {
    /// An error with this code.
    #[must_use]
    pub const fn new(code: ErrorCode) -> Self {
        Self { code }
    }

    /// The command is not built yet.
    pub const NOT_IMPLEMENTED: Self = Self::new(ErrorCode::NotImplemented);
    /// The vault is locked.
    pub const LOCKED: Self = Self::new(ErrorCode::Locked);
    /// An unexpected internal failure.
    pub const INTERNAL: Self = Self::new(ErrorCode::Internal);
}

impl From<ErrorCode> for IpcError {
    fn from(code: ErrorCode) -> Self {
        Self::new(code)
    }
}

/// The result of every command.
pub type IpcResult<T> = Result<T, IpcError>;

impl From<SessionError> for IpcError {
    fn from(error: SessionError) -> Self {
        let code = match error {
            SessionError::UnlockFailed => ErrorCode::WrongCredentials,
            SessionError::NoAccount => ErrorCode::NoAccount,
            SessionError::UnknownVault => ErrorCode::VaultNotFound,
            SessionError::VaultUnavailable => ErrorCode::VaultUnavailable,
            SessionError::UnknownItem => ErrorCode::ItemNotFound,
            SessionError::ItemDeleted => ErrorCode::ItemDeleted,
            SessionError::SameVault => ErrorCode::SameVault,
            SessionError::NoOtp => ErrorCode::NoOtp,
            SessionError::AttachmentTooLarge => ErrorCode::AttachmentTooLarge,
            SessionError::AttachmentQuota => ErrorCode::AttachmentQuota,
            SessionError::Store(store) => return store.into(),
            SessionError::Core(core) => return core.into(),
            _ => ErrorCode::Internal,
        };
        Self::new(code)
    }
}

impl From<StoreError> for IpcError {
    fn from(error: StoreError) -> Self {
        Self::new(match error {
            StoreError::StoreInUse => ErrorCode::StoreInUse,
            StoreError::AccountExists => ErrorCode::AccountExists,
            StoreError::NoAccount => ErrorCode::NoAccount,
            StoreError::UnknownVault => ErrorCode::VaultNotFound,
            StoreError::AttachmentTooLarge => ErrorCode::AttachmentTooLarge,
            StoreError::Database(_)
            | StoreError::Io(_)
            | StoreError::InvalidPath
            | StoreError::NotAMosaicStore
            | StoreError::SchemaTooNew { .. }
            | StoreError::Corrupt => ErrorCode::StoreUnavailable,
            _ => ErrorCode::Internal,
        })
    }
}

impl From<mosaic_core::Error> for IpcError {
    fn from(error: mosaic_core::Error) -> Self {
        Self::new(match error {
            mosaic_core::Error::SecretKeyMalformed => ErrorCode::SecretKeyMalformed,
            mosaic_core::Error::SecretKeyMistyped => ErrorCode::SecretKeyMistyped,
            mosaic_core::Error::EmptyPassword => ErrorCode::EmptyPassword,
            _ => ErrorCode::Internal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_serialize_as_a_bare_code() {
        let json = serde_json::to_string(&IpcError::LOCKED).unwrap();
        assert_eq!(json, r#"{"code":"locked"}"#);
        let json = serde_json::to_string(&IpcError::new(ErrorCode::SecretKeyMistyped)).unwrap();
        assert_eq!(json, r#"{"code":"secretKeyMistyped"}"#);
    }

    #[test]
    fn wrong_password_and_wrong_secret_key_look_the_same() {
        assert_eq!(
            IpcError::from(SessionError::UnlockFailed).code,
            ErrorCode::WrongCredentials
        );
    }
}
