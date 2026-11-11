//! The error of every `Core` call: unit variants only, so an error carries no
//! message, ID or value (CRYPTO_SPEC §6.9.5). Each variant has a camelCase code
//! that the apps map to the i18n key `error.<code>`; the codes of
//! `docs/DESKTOP_IPC.md` keep their meaning here.

use mosaic_sync::engine::SyncError;
use mosaic_sync::transport::{Rejection, RevokedReason, TransportError};
use mosaic_vault::session::Error as SessionError;
use mosaic_vault::storage::Error as StoreError;

macro_rules! mobile_errors {
    ($($(#[$meta:meta])* $variant:ident => $code:literal),* $(,)?) => {
        /// Why a call failed. See the module documentation.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
        #[non_exhaustive]
        pub enum MobileError {
            $($(#[$meta])* #[error($code)] $variant),*
        }

        impl MobileError {
            /// Every error, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];

            /// The camelCase code of this error.
            #[must_use]
            pub const fn code(self) -> &'static str {
                match self {
                    $(Self::$variant => $code),*
                }
            }
        }
    };
}

mobile_errors! {
    // Store and session (shared with the desktop).
    /// Something that cannot be explained to the user failed.
    Internal => "internal",
    /// A long call is running; try again.
    Busy => "busy",
    /// Another process holds the store.
    StoreInUse => "storeInUse",
    /// The store cannot be read or written.
    StoreUnavailable => "storeUnavailable",
    /// There is no account on this phone yet.
    NoAccount => "noAccount",
    /// There already is an account on this phone.
    AccountExists => "accountExists",
    /// The vault is locked.
    Locked => "locked",
    // Credentials and the Secret Key.
    /// The master password is empty.
    EmptyPassword => "emptyPassword",
    /// The password, the Secret Key or the email is wrong.
    WrongCredentials => "wrongCredentials",
    /// The Secret Key is needed and not stored.
    SecretKeyRequired => "secretKeyRequired",
    /// The Secret Key is not in the `M1-` format.
    SecretKeyMalformed => "secretKeyMalformed",
    /// The Secret Key's checksum does not match.
    SecretKeyMistyped => "secretKeyMistyped",
    // Quick unlock and re-authentication.
    /// The master password is needed (no boot marker, five failures, or no enrolment).
    PasswordRequired => "passwordRequired",
    /// The enrolment is older than its maximum age.
    QuickUnlockExpired => "quickUnlockExpired",
    /// The platform key no longer opens the wrapping.
    QuickUnlockInvalidated => "quickUnlockInvalidated",
    /// The master password must be given again.
    ReauthRequired => "reauthRequired",
    // Items.
    /// No such item, vault or pending entry.
    NotFound => "notFound",
    /// The input is not acceptable.
    InvalidInput => "invalidInput",
    /// The vault's key is not available in this session.
    VaultUnavailable => "vaultUnavailable",
    /// A hardware-protected vault takes one-time-password items only.
    ItemTypeNotAllowed => "itemTypeNotAllowed",
    // One-time passwords.
    /// The QR code is not an `otpauth://` code.
    OtpQrNotOtp => "otpQrNotOtp",
    /// The QR code is a Google Authenticator export.
    OtpQrMigration => "otpQrMigration",
    /// More than one code in the picture.
    OtpQrMultiple => "otpQrMultiple",
    /// The `otpauth://` code is invalid.
    OtpQrInvalid => "otpQrInvalid",
    /// The `otpauth://` text is invalid.
    OtpUriInvalid => "otpUriInvalid",
    /// The typed key is not valid base32.
    OtpKeyInvalid => "otpKeyInvalid",
    // Sync.
    /// Sync is not set up.
    SyncNotConfigured => "syncNotConfigured",
    /// The server ended the session or none is stored.
    SyncSignedOut => "syncSignedOut",
    /// The account needs a second factor.
    SyncTwoFactorRequired => "syncTwoFactorRequired",
    /// The second-factor code is wrong.
    SyncTwoFactorInvalid => "syncTwoFactorInvalid",
    /// The security-key handoff ran out.
    SyncHandoffExpired => "syncHandoffExpired",
    /// The server asks to slow down.
    SyncRateLimited => "syncRateLimited",
    /// The master password was changed on another device.
    SyncPasswordChanged => "syncPasswordChanged",
    /// The account was re-keyed on another device.
    SyncAccountRekeyed => "syncAccountRekeyed",
    /// Sync failed for another reason.
    SyncFailed => "syncFailed",
    /// The server cannot be reached.
    SyncServerUnreachable => "syncServerUnreachable",
    /// The server address or its answer is invalid.
    SyncServerInvalid => "syncServerInvalid",
    /// The server address is not `https`.
    SyncServerInsecure => "syncServerInsecure",
    /// The server is not the one it says it is.
    SyncServerMismatch => "syncServerMismatch",
    /// The server speaks another protocol version.
    SyncServerIncompatible => "syncServerIncompatible",
    // New on mobile: context and onboarding.
    /// The master password is entered in the app, not in the extension.
    PasswordInApp => "passwordInApp",
    /// The account syncs through a folder; the phone apps support hosted accounts only.
    FolderAccountUnsupported => "folderAccountUnsupported",
    /// The setup code is not valid.
    SetupCodeInvalid => "setupCodeInvalid",
    /// The phone has too little free memory to unlock.
    NotEnoughMemory => "notEnoughMemory",
    // Autofill.
    /// Nothing matches.
    NoMatch => "noMatch",
    /// The page is not served over a secure origin.
    InsecureOrigin => "insecureOrigin",
    /// The app is not linked to the login.
    AppNotLinked => "appNotLinked",
    /// The field is in a foreign frame.
    ForeignFrame => "foreignFrame",
    // Passkeys.
    /// The site's asset-links file cannot be fetched.
    AssetLinksUnavailable => "assetLinksUnavailable",
    /// The site does not list this app.
    AppNotVerified => "appNotVerified",
    /// The relying-party ID does not fit the origin.
    RpIdInvalid => "rpIdInvalid",
    /// No such passkey.
    PasskeyNotFound => "passkeyNotFound",
    /// An excluded credential exists.
    PasskeyExcluded => "passkeyExcluded",
    /// No requested algorithm is supported.
    PasskeyUnsupportedAlgorithm => "passkeyUnsupportedAlgorithm",
    /// The passkey request ran out.
    PasskeySessionExpired => "passkeySessionExpired",
    // Other.
    /// No QR code in the picture.
    QrNotFound => "qrNotFound",
    /// The platform's secure storage failed.
    StorageUnavailable => "storageUnavailable",
}

/// The result of a `Core` call.
pub type Result<T> = core::result::Result<T, MobileError>;

/// Every code, for `mobile_error_codes()`.
#[must_use]
pub fn error_codes() -> Vec<&'static str> {
    MobileError::ALL.iter().map(|e| e.code()).collect()
}

impl From<StoreError> for MobileError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::StoreInUse => Self::StoreInUse,
            StoreError::NoAccount => Self::NoAccount,
            StoreError::AccountExists => Self::AccountExists,
            _ => Self::StoreUnavailable,
        }
    }
}

impl From<mosaic_core::Error> for MobileError {
    fn from(e: mosaic_core::Error) -> Self {
        match e {
            mosaic_core::Error::EmptyPassword => Self::EmptyPassword,
            mosaic_core::Error::SecretKeyMistyped => Self::SecretKeyMistyped,
            mosaic_core::Error::SecretKeyMalformed => Self::SecretKeyMalformed,
            mosaic_core::Error::Malformed | mosaic_core::Error::SizeLimit => Self::InvalidInput,
            _ => Self::Internal,
        }
    }
}

impl From<SessionError> for MobileError {
    fn from(e: SessionError) -> Self {
        match e {
            SessionError::Core(c) => c.into(),
            SessionError::Store(s) => s.into(),
            SessionError::UnlockFailed => Self::WrongCredentials,
            SessionError::NoAccount => Self::NoAccount,
            SessionError::UnknownVault | SessionError::UnknownItem | SessionError::ItemDeleted => {
                Self::NotFound
            }
            SessionError::VaultUnavailable => Self::VaultUnavailable,
            SessionError::QuickUnlockExpired => Self::QuickUnlockExpired,
            SessionError::QuickUnlockInvalidated => Self::QuickUnlockInvalidated,
            SessionError::NoQuickUnlock | SessionError::PasswordRequired => Self::PasswordRequired,
            SessionError::ReauthenticationRequired => Self::ReauthRequired,
            SessionError::Otp(_) | SessionError::InvalidInput => Self::InvalidInput,
            SessionError::ItemTypeNotAllowed => Self::ItemTypeNotAllowed,
            SessionError::NoOtp => Self::NotFound,
            _ => Self::Internal,
        }
    }
}

impl From<TransportError> for MobileError {
    fn from(e: TransportError) -> Self {
        match e {
            TransportError::Unreachable | TransportError::Timeout => Self::SyncServerUnreachable,
            TransportError::Revoked(RevokedReason::PasswordChange) => Self::SyncPasswordChanged,
            TransportError::Revoked(RevokedReason::Rekey) => Self::SyncAccountRekeyed,
            TransportError::Unauthorized | TransportError::Revoked(_) => Self::SyncSignedOut,
            TransportError::TwoFactorRequired | TransportError::SecondFactorUnsupported => {
                Self::SyncTwoFactorRequired
            }
            TransportError::RateLimited { .. } => Self::SyncRateLimited,
            TransportError::ServerMismatch => Self::SyncServerMismatch,
            TransportError::Incompatible => Self::SyncServerIncompatible,
            TransportError::Protocol | TransportError::InvalidUrl => Self::SyncServerInvalid,
            TransportError::InsecureUrl => Self::SyncServerInsecure,
            TransportError::Locked => Self::Locked,
            TransportError::Rejected(r) => match r {
                Rejection::LoginFailed => Self::WrongCredentials,
                Rejection::TwoFactorInvalid => Self::SyncTwoFactorInvalid,
                Rejection::HandoffExpired => Self::SyncHandoffExpired,
                Rejection::InvalidEmail | Rejection::BadRequest => Self::InvalidInput,
                Rejection::DeviceRevoked => Self::SyncSignedOut,
                Rejection::AccountExists => Self::AccountExists,
                _ => Self::SyncFailed,
            },
            _ => Self::SyncFailed,
        }
    }
}

impl From<SyncError> for MobileError {
    fn from(e: SyncError) -> Self {
        match e {
            SyncError::Locked => Self::Locked,
            SyncError::Transport(t) => t.into(),
            SyncError::Session(s) => s.into(),
            SyncError::AccountRekeyed => Self::SyncAccountRekeyed,
            _ => Self::SyncFailed,
        }
    }
}

impl From<mosaic_totp::Error> for MobileError {
    fn from(_: mosaic_totp::Error) -> Self {
        Self::InvalidInput
    }
}

impl From<mosaic_generator::Error> for MobileError {
    fn from(e: mosaic_generator::Error) -> Self {
        match e {
            mosaic_generator::Error::Randomness | mosaic_generator::Error::Wordlist => {
                Self::Internal
            }
            _ => Self::InvalidInput,
        }
    }
}
