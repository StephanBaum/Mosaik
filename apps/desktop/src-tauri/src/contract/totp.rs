//! The current one-time code of an item, for display next to the item. The
//! UI refreshes it when `secondsRemaining` runs out; copying goes through
//! `copy_to_clipboard` instead.

use serde::Serialize;

use super::secret::SecretOut;

/// The `totp_code` response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotpCode {
    /// The code.
    pub code: SecretOut,
    /// Seconds until it changes (`null` for HOTP, which changes on use).
    pub seconds_remaining: Option<u32>,
    /// The time step in seconds (`null` for HOTP).
    pub period: Option<u32>,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;

    pub(crate) fn codes() -> Vec<TotpCode> {
        vec![
            TotpCode {
                code: SecretOut::test("492039"),
                seconds_remaining: Some(17),
                period: Some(30),
            },
            TotpCode {
                code: SecretOut::test("755224"),
                seconds_remaining: None,
                period: None,
            },
        ]
    }
}
