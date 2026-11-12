//! The setup code for a phone (`docs/DESKTOP_IPC.md` §3, ADR-0096): Rust builds
//! the code from the account's server, email and Secret Key, renders it with
//! `qrcodegen`, and hands the webview the module matrix to draw. The matrix
//! encodes the Secret Key, so this is the one response besides [`SecretOut`]
//! that carries a secret: it only follows a right master password and the
//! webview hides it again after 60 seconds, on blur and on lock.
//!
//! [`SecretOut`]: super::secret::SecretOut

use serde::{Deserialize, Serialize};

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

/// The `setup_code_show` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
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
