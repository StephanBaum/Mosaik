//! The OS-facing parts, behind `mosaic-platform`: keystores, the session store, the clipboard,
//! the terminal and the clock. With the `test-util` feature and `MOSAIC_TEST_DIR` set, file-based
//! stand-ins replace them (`testing.rs`); otherwise the real ones are used.

use mosaic_platform::PlatformError;
use mosaic_platform::clipboard::{Clipboard, SystemClipboard};
use mosaic_platform::keystore::{
    CliSessionStore, OsCliSessions, OsKeystore, OsSyncTokens, SecretKeyStore, SyncTokenStore,
};

use crate::input::{RealTerminal, Terminal};

/// Everything the commands read from or write to the OS.
pub struct Backends {
    /// The Secret Key store (shared with the desktop).
    pub keys: Box<dyn SecretKeyStore>,
    /// The session store; `Err` when there is none (no `$XDG_RUNTIME_DIR` on Linux).
    pub sessions: core::result::Result<Box<dyn CliSessionStore>, PlatformError>,
    /// The CLI's hosted sync tokens.
    pub tokens: Box<dyn SyncTokenStore>,
}

/// The backends of this run.
#[must_use]
pub fn open() -> Backends {
    #[cfg(feature = "test-util")]
    if let Some(backends) = crate::testing::backends() {
        return backends;
    }
    Backends {
        keys: Box::new(OsKeystore::for_app()),
        sessions: OsCliSessions::new().map(|s| Box::new(s) as Box<dyn CliSessionStore>),
        tokens: Box::new(OsSyncTokens::for_cli()),
    }
}

/// The clipboard.
///
/// # Errors
/// [`PlatformError::Clipboard`] when there is none.
pub fn clipboard() -> core::result::Result<Box<dyn Clipboard>, PlatformError> {
    #[cfg(feature = "test-util")]
    if let Some(clipboard) = crate::testing::clipboard() {
        return Ok(clipboard);
    }
    SystemClipboard::new().map(|c| Box::new(c) as Box<dyn Clipboard>)
}

/// The terminal.
#[must_use]
pub fn terminal() -> Box<dyn Terminal> {
    #[cfg(feature = "test-util")]
    if let Some(terminal) = crate::testing::terminal() {
        return terminal;
    }
    Box::new(RealTerminal)
}

/// Unix milliseconds now.
#[must_use]
pub fn now_ms() -> u64 {
    #[cfg(feature = "test-util")]
    if let Some(now) = crate::testing::now_ms() {
        return now;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}
