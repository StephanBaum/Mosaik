//! The setup code for a phone (`docs/DESKTOP_IPC.md` §3, ADR-0096): Rust builds
//! the code from the account's server, email and Secret Key, renders it with
//! `qrcodegen`, and hands the webview the module matrix to draw. The matrix
//! encodes the Secret Key, so this is the one response besides [`SecretOut`]
//! that carries a secret: it only follows a right master password and the
//! webview hides it again after 60 seconds, on blur and on lock.
//!
//! [`SecretOut`]: super::secret::SecretOut

use std::fmt;

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use super::ids::UnixMillis;
use super::secret::SecretIn;

/// How long the code stays up, in milliseconds.
pub const SETUP_CODE_VISIBLE_MS: i64 = 60_000;

/// `setup_code_show`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupCodeShowRequest {
    /// The master password, checked now (counts towards the five-failure
    /// lock). It earns no re-authentication grant for other commands.
    pub password: SecretIn,
}

/// The `setup_code_show` response. The matrix encodes the Secret Key, so it is
/// not `Clone`, its `Debug` prints no modules, and `Drop` wipes them
/// (M10-G2 M4). The QR library's own buffers are not wiped (CRYPTO_SPEC §6.9.7).
#[derive(PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupCodeShown {
    /// `modules[y][x]` is `true` for a dark module. `size` rows of `size`
    /// columns, without a quiet zone (the webview adds one).
    pub modules: Vec<Vec<bool>>,
    /// The side length in modules.
    pub size: u32,
    /// When the code must be hidden: 60 s after it was made.
    pub expires_at: UnixMillis,
}

impl fmt::Debug for SetupCodeShown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetupCodeShown")
            .field("size", &self.size)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

impl Drop for SetupCodeShown {
    fn drop(&mut self) {
        for row in &mut self.modules {
            row.zeroize();
        }
        self.modules.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_prints_no_modules() {
        let shown = SetupCodeShown {
            modules: vec![vec![true, false], vec![false, true]],
            size: 2,
            expires_at: 7,
        };
        let text = format!("{shown:?}");
        assert!(text.contains("size: 2"));
        assert!(text.contains("expires_at: 7"));
        assert!(!text.contains("modules"));
        assert!(!text.contains("true"));
    }
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;

    pub(crate) fn requests() -> Vec<SetupCodeShowRequest> {
        vec![SetupCodeShowRequest {
            password: SecretIn::test("correct horse battery staple"),
        }]
    }

    pub(crate) fn results() -> Vec<SetupCodeShown> {
        vec![SetupCodeShown {
            modules: vec![vec![true, false], vec![false, true]],
            size: 2,
            expires_at: 1_762_689_660_000,
        }]
    }
}
