//! Session: the state machine (no account → locked ⇄ unlocked), account
//! creation with the Emergency Kit, unlock and lock.

use serde::{Deserialize, Serialize};

use super::ids::WireAccountId;
use super::macros::wire_enum;
use super::secret::{SecretIn, SecretOut};

wire_enum!(
    /// One-gesture unlock methods (CRYPTO_SPEC §6.2, §6.3).
    pub enum QuickUnlockKind {
        /// The OS biometric prompt (Windows Hello, Touch ID, Linux fprintd)
        /// releasing a key from the OS keystore.
        Biometric,
        /// A FIDO2 security key with the hmac-secret / PRF extension.
        HardwareKey,
    }
);

/// Where the session is. Every session command returns the new state.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SessionState {
    /// No account on this device: onboarding.
    NoAccount,
    /// An account, locked.
    Locked {
        /// The account email, shown on the unlock screen (`null` if unknown).
        email: Option<String>,
        /// Whether this device holds the Secret Key; if not, the unlock
        /// screen asks for it (from the Emergency Kit).
        secret_key_stored: bool,
        /// Quick-unlock methods set up on this device, in display order.
        quick_unlock: Vec<QuickUnlockKind>,
    },
    /// Unlocked: the vault UI.
    Unlocked {
        /// The account.
        account_id: WireAccountId,
        /// The account email (`null` if unknown).
        email: Option<String>,
    },
}

/// `create_account`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateAccountRequest {
    /// The account email (for the Emergency Kit and, from M6, sync).
    pub email: String,
    /// The new master password.
    pub password: SecretIn,
    /// The name of the first vault, in the user's language.
    pub default_vault_name: String,
}

/// What the Emergency Kit shows (CRYPTO_SPEC §5.2). Shown once.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyKit {
    /// The account.
    pub account_id: WireAccountId,
    /// The account email.
    pub email: String,
    /// The Secret Key in its display form (`M1-XXXX-…`).
    pub secret_key: SecretOut,
}

/// The `create_account` response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedAccount {
    /// Print or save it, then verify with [`VerifyKitRequest`].
    pub emergency_kit: EmergencyKit,
    /// The new state (unlocked).
    pub session: SessionState,
}

/// `verify_emergency_kit`: the user types the last group of the Secret Key
/// to confirm they saved the kit (UX_GUIDELINES §8).
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerifyKitRequest {
    /// What the user typed.
    pub last_group: SecretIn,
}

/// The `verify_emergency_kit` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KitVerification {
    /// Whether the typed group matches (compared in constant time).
    pub matches: bool,
}

/// `unlock`: master password, plus the Secret Key when this device does
/// not hold it (`null` to use the stored one).
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnlockRequest {
    /// The master password.
    pub password: SecretIn,
    /// The Secret Key as typed from the Emergency Kit, or `null`.
    pub secret_key: Option<SecretIn>,
}

/// `unlock_quick`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuickUnlockRequest {
    /// Which method the user chose.
    pub method: QuickUnlockKind,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn states() -> Vec<SessionState> {
        let all = vec![
            SessionState::NoAccount,
            SessionState::Locked {
                email: Some("alex@example.com".into()),
                secret_key_stored: true,
                quick_unlock: QuickUnlockKind::ALL.to_vec(),
            },
            SessionState::Locked {
                email: None,
                secret_key_stored: false,
                quick_unlock: vec![],
            },
            SessionState::Unlocked {
                account_id: ids::account(),
                email: Some("alex@example.com".into()),
            },
        ];
        for state in &all {
            // Adding a variant fails to compile here until it has a sample.
            match state {
                SessionState::NoAccount
                | SessionState::Locked { .. }
                | SessionState::Unlocked { .. } => {}
            }
        }
        all
    }

    pub(crate) fn created_account() -> CreatedAccount {
        CreatedAccount {
            emergency_kit: EmergencyKit {
                account_id: ids::account(),
                email: "alex@example.com".into(),
                secret_key: SecretOut::test("M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2"),
            },
            session: SessionState::Unlocked {
                account_id: ids::account(),
                email: Some("alex@example.com".into()),
            },
        }
    }

    pub(crate) fn create_account_request() -> CreateAccountRequest {
        CreateAccountRequest {
            email: "alex@example.com".into(),
            password: SecretIn::test("correct horse battery staple"),
            default_vault_name: "Personal".into(),
        }
    }

    pub(crate) fn unlock_request() -> UnlockRequest {
        UnlockRequest {
            password: SecretIn::test("correct horse battery staple"),
            secret_key: Some(SecretIn::test("M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2")),
        }
    }
}
