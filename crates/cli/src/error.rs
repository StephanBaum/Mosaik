//! Errors of the CLI: a stable code, an exit code (ADR-0075) and a localized message. No error
//! carries a secret or an OS message that could quote content.

use mosaic_vault::session::Error as VaultError;
use mosaic_vault::storage::Error as StorageError;

use crate::i18n::Msg;

/// Exit code: other error.
pub const EXIT_OTHER: u8 = 1;
/// Exit code: usage error.
pub const EXIT_USAGE: u8 = 2;
/// Exit code: authentication failed or input required.
pub const EXIT_AUTH: u8 = 3;
/// Exit code: not found.
pub const EXIT_NOT_FOUND: u8 = 4;
/// Exit code: store in use.
pub const EXIT_STORE_IN_USE: u8 = 5;
/// Exit code: sync halted or sync error.
pub const EXIT_SYNC: u8 = 6;
/// Exit code: ambiguous reference.
pub const EXIT_AMBIGUOUS: u8 = 7;
/// Exit code: a vault part is locked (hardware-protected or unavailable).
pub const EXIT_VAULT_LOCKED: u8 = 8;
/// Exit code: reveal required.
pub const EXIT_REVEAL: u8 = 9;
/// Exit code: rotation in progress.
pub const EXIT_ROTATION: u8 = 10;
/// Exit code: `run` could not start its command.
pub const EXIT_SPAWN: u8 = 125;

/// A failed command.
#[derive(Debug)]
pub struct CliError {
    /// The stable code of the JSON error (`storeInUse`, ...).
    pub code: &'static str,
    /// The process exit code.
    pub exit: u8,
    /// The message.
    pub msg: Msg,
    /// Its placeholders.
    pub args: Vec<(&'static str, String)>,
    /// Extra JSON members of the error (candidates of an ambiguous reference): never a secret.
    pub extra: Option<serde_json::Value>,
}

/// `Result` with [`CliError`].
pub type Result<T> = core::result::Result<T, CliError>;

impl CliError {
    /// An error with `code`, `exit` and `msg`.
    #[must_use]
    pub const fn new(code: &'static str, exit: u8, msg: Msg) -> Self {
        Self {
            code,
            exit,
            msg,
            args: Vec::new(),
            extra: None,
        }
    }

    /// Adds a placeholder value.
    #[must_use]
    pub fn with(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.args.push((name, value.into()));
        self
    }

    /// A usage error (exit 2).
    #[must_use]
    pub const fn usage(msg: Msg) -> Self {
        Self::new("usage", EXIT_USAGE, msg)
    }

    /// An option that would put a secret on the command line (exit 2).
    #[must_use]
    pub fn secret_in_argv(flag: &str) -> Self {
        Self::new("secretInArgv", EXIT_USAGE, Msg::ErrSecretInArgv).with("flag", flag)
    }

    /// Anything unexpected; says nothing about the cause.
    #[must_use]
    pub const fn internal() -> Self {
        Self::new("internal", EXIT_OTHER, Msg::ErrInternal)
    }

    /// `--no-input` met a prompt (exit 3).
    #[must_use]
    pub const fn input_required() -> Self {
        Self::new("inputRequired", EXIT_AUTH, Msg::ErrInputRequired)
    }

    /// The master password or the Secret Key is wrong (exit 3).
    #[must_use]
    pub const fn wrong_credentials() -> Self {
        Self::new("wrongCredentials", EXIT_AUTH, Msg::ErrWrongCredentials)
    }

    /// The store is open in another process (exit 5).
    #[must_use]
    pub const fn store_in_use() -> Self {
        Self::new("storeInUse", EXIT_STORE_IN_USE, Msg::ErrStoreInUse)
    }

    /// Nothing matches `what` (exit 4).
    #[must_use]
    pub fn not_found(what: &str) -> Self {
        Self::new("notFound", EXIT_NOT_FOUND, Msg::ErrNotFound).with("ref", what)
    }

    /// A key rotation job is open (exit 10).
    #[must_use]
    pub const fn rotation() -> Self {
        Self::new(
            "rotationInProgress",
            EXIT_ROTATION,
            Msg::ErrRotationInProgress,
        )
    }

    /// A feature this CLI does not have (exit 1).
    #[must_use]
    pub fn not_supported(what: &str) -> Self {
        Self::new("notSupported", EXIT_OTHER, Msg::ErrNotSupported).with("what", what)
    }

    /// A file could not be read.
    #[must_use]
    pub fn file_read(path: &std::path::Path) -> Self {
        Self::new("fileRead", EXIT_OTHER, Msg::ErrFileRead).with("path", path.display().to_string())
    }

    /// A file could not be written.
    #[must_use]
    pub fn file_write(path: &std::path::Path) -> Self {
        Self::new("fileWrite", EXIT_OTHER, Msg::ErrFileWrite)
            .with("path", path.display().to_string())
    }
}

impl From<StorageError> for CliError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::StoreInUse => Self::store_in_use(),
            StorageError::NoAccount => Self::new("noAccount", EXIT_NOT_FOUND, Msg::ErrNoAccount),
            _ => Self::internal(),
        }
    }
}

impl From<VaultError> for CliError {
    fn from(e: VaultError) -> Self {
        match e {
            VaultError::Store(inner) => inner.into(),
            VaultError::UnlockFailed => Self::wrong_credentials(),
            VaultError::NoAccount => Self::new("noAccount", EXIT_NOT_FOUND, Msg::ErrNoAccount),
            VaultError::VaultUnavailable => Self::new(
                "vaultUnavailable",
                EXIT_VAULT_LOCKED,
                Msg::ErrVaultUnavailable,
            ),
            VaultError::CliSessionRejected => {
                Self::new("sessionExpired", EXIT_AUTH, Msg::ErrSessionExpired)
            }
            VaultError::NoOtp => Self::new("noOtp", EXIT_NOT_FOUND, Msg::ErrNoOtp),
            VaultError::UnknownVault | VaultError::UnknownItem | VaultError::ItemDeleted => {
                Self::new("notFound", EXIT_NOT_FOUND, Msg::ErrNotFound).with("ref", "")
            }
            _ => Self::internal(),
        }
    }
}
