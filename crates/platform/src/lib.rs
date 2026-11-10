//! Platform glue shared by the desktop app and the CLI (ADR-0074): the OS
//! keystore with the one set of entry names, the clipboard backend with the
//! history-exclusion hints, and the data directories.
//!
//! No secret is ever logged or put into an error; secret values live in
//! zeroizing types.

#![forbid(unsafe_code)]

pub mod clipboard;
pub mod keystore;
pub mod paths;
pub mod session;

/// What can go wrong. Carries no secret and no OS message (which could quote
/// content).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PlatformError {
    /// The OS keystore cannot be used right now.
    #[error("the OS keystore is unavailable")]
    Keystore,
    /// A lock of an in-process store is poisoned.
    #[error("internal state unavailable")]
    Poisoned,
    /// The clipboard cannot be used right now.
    #[error("the clipboard is unavailable")]
    Clipboard,
    /// No home or data directory could be determined.
    #[error("no data directory for this user")]
    NoDirectory,
    /// Linux without `$XDG_RUNTIME_DIR`: no session can be cached (there is no
    /// persistent fallback, CRYPTO_SPEC §6.8).
    #[error("no runtime directory for session files ($XDG_RUNTIME_DIR is not set)")]
    NoRuntimeDir,
    /// A session file or its directory cannot be read or written.
    #[error("a session file cannot be accessed")]
    File,
    /// A key could not be encoded or decoded.
    #[error(transparent)]
    Core(#[from] mosaic_core::Error),
}

/// `Result` with [`PlatformError`].
pub type Result<T> = core::result::Result<T, PlatformError>;
