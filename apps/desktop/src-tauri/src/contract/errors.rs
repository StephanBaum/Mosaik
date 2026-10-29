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
        /// The import file is too large or too complex.
        ImportTooLarge,
        /// The export's format version or cipher is not supported.
        ImportUnsupported,
        /// The export is password-protected in a way Mosaic cannot read.
        ImportEncrypted,
        /// The SSH approval request was already answered or expired.
        SshRequestExpired,
        /// The SSH agent could not start.
        SshAgentUnavailable,
        /// Another SSH agent already uses the endpoint.
        SshAgentEndpointInUse,
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
    fn every_code_round_trips_through_its_wire_name() {
        let mut seen = std::collections::HashSet::new();
        for &code in ErrorCode::ALL {
            let json = serde_json::to_string(&IpcError::new(code)).unwrap();
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            let name = value["code"].as_str().unwrap().to_owned();
            assert_eq!(value.as_object().unwrap().len(), 1, "only a code: {json}");
            assert!(name.chars().all(|c| c.is_ascii_alphanumeric()), "{name}");
            assert!(name.starts_with(|c: char| c.is_ascii_lowercase()), "{name}");
            let back: ErrorCode = serde_json::from_value(value["code"].clone()).unwrap();
            assert_eq!(back, code);
            assert!(seen.insert(name), "duplicate wire name for {code:?}");
        }
        assert!(serde_json::from_str::<ErrorCode>("\"nonsense\"").is_err());
    }

    #[test]
    fn display_names_the_code_and_nothing_secret() {
        assert_eq!(IpcError::LOCKED.to_string(), "IPC command failed: Locked");
        assert_eq!(IpcError::from(ErrorCode::Busy).code, ErrorCode::Busy);
        assert_eq!(IpcError::NOT_IMPLEMENTED.code, ErrorCode::NotImplemented);
        assert_eq!(IpcError::INTERNAL.code, ErrorCode::Internal);
    }

    #[test]
    fn session_errors_map_to_the_codes_the_ui_acts_on() {
        let cases = [
            (SessionError::NoAccount, ErrorCode::NoAccount),
            (SessionError::UnknownVault, ErrorCode::VaultNotFound),
            (SessionError::VaultUnavailable, ErrorCode::VaultUnavailable),
            (SessionError::UnknownItem, ErrorCode::ItemNotFound),
            (SessionError::ItemDeleted, ErrorCode::ItemDeleted),
            (SessionError::SameVault, ErrorCode::SameVault),
            (SessionError::NoOtp, ErrorCode::NoOtp),
            (
                SessionError::AttachmentTooLarge,
                ErrorCode::AttachmentTooLarge,
            ),
            (SessionError::AttachmentQuota, ErrorCode::AttachmentQuota),
            (SessionError::UnknownAttachment, ErrorCode::Internal),
            (
                SessionError::Core(mosaic_core::Error::Decryption),
                ErrorCode::Internal,
            ),
            (
                SessionError::Core(mosaic_core::Error::EmptyPassword),
                ErrorCode::EmptyPassword,
            ),
            (
                SessionError::Store(StoreError::StoreInUse),
                ErrorCode::StoreInUse,
            ),
        ];
        for (error, code) in cases {
            let shown = format!("{error:?}");
            assert_eq!(IpcError::from(error).code, code, "{shown}");
        }
    }

    #[test]
    fn store_errors_map_and_unreadable_files_all_look_alike() {
        let cases = [
            (StoreError::StoreInUse, ErrorCode::StoreInUse),
            (StoreError::AccountExists, ErrorCode::AccountExists),
            (StoreError::NoAccount, ErrorCode::NoAccount),
            (StoreError::UnknownVault, ErrorCode::VaultNotFound),
            (
                StoreError::AttachmentTooLarge,
                ErrorCode::AttachmentTooLarge,
            ),
            (StoreError::InvalidPath, ErrorCode::StoreUnavailable),
            (StoreError::NotAMosaicStore, ErrorCode::StoreUnavailable),
            (StoreError::Corrupt, ErrorCode::StoreUnavailable),
            (
                StoreError::SchemaTooNew {
                    found: 99,
                    supported: 1,
                },
                ErrorCode::StoreUnavailable,
            ),
            (
                StoreError::Io(std::io::Error::other("secret path")),
                ErrorCode::StoreUnavailable,
            ),
            (StoreError::AccountMismatch, ErrorCode::Internal),
        ];
        for (error, code) in cases {
            let shown = format!("{error:?}");
            assert_eq!(IpcError::from(error).code, code, "{shown}");
        }
    }

    #[test]
    fn core_errors_only_surface_what_the_user_can_fix() {
        let cases = [
            (
                mosaic_core::Error::SecretKeyMalformed,
                ErrorCode::SecretKeyMalformed,
            ),
            (
                mosaic_core::Error::SecretKeyMistyped,
                ErrorCode::SecretKeyMistyped,
            ),
            (mosaic_core::Error::EmptyPassword, ErrorCode::EmptyPassword),
            (mosaic_core::Error::Decryption, ErrorCode::Internal),
            (mosaic_core::Error::Malformed, ErrorCode::Internal),
        ];
        for (error, code) in cases {
            assert_eq!(IpcError::from(error).code, code, "{error}");
        }
    }

    #[test]
    fn wrong_password_and_wrong_secret_key_look_the_same() {
        assert_eq!(
            IpcError::from(SessionError::UnlockFailed).code,
            ErrorCode::WrongCredentials
        );
    }
}
