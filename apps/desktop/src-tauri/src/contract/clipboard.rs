//! Clipboard: Rust reads the value and writes the system clipboard itself,
//! so the secret never enters JavaScript. The clipboard is cleared after
//! the configured timeout (default 30 s, UX_GUIDELINES §8) and excluded
//! from clipboard history where the OS supports it.

use serde::{Deserialize, Serialize};

use super::ids::{UnixMillis, WireItemId, WireVaultId};
use super::items::{SecretFieldKey, TextFieldKey};

/// Which value of an item to copy.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CopyField {
    /// The login user name.
    Username,
    /// The login password.
    Password,
    /// The current one-time code (HOTP advances the counter).
    Totp,
    /// A plain text field.
    Text {
        /// The field.
        field: TextFieldKey,
    },
    /// A secret field.
    Secret {
        /// The field.
        field: SecretFieldKey,
    },
    /// A custom field (any kind; an `otp` field copies its current code).
    CustomField {
        /// The element.
        element_id: String,
    },
}

/// What to copy.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CopySource {
    /// A value of an item.
    Item {
        /// The vault.
        vault_id: WireVaultId,
        /// The item.
        item_id: WireItemId,
        /// The value.
        field: CopyField,
    },
    /// The password most recently returned by `generate_password`, which
    /// Rust keeps until the next generation or lock.
    Generated,
}

/// `copy_to_clipboard`.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyRequest {
    /// What to copy.
    pub source: CopySource,
}

/// The `copy_to_clipboard` response, for the "Copied, clears in 30 seconds"
/// toast and its live region (UX_GUIDELINES §7).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyReceipt {
    /// When Rust clears the clipboard (`null`: auto-clear is off).
    pub clears_at: Option<UnixMillis>,
    /// The timeout in seconds (`null`: auto-clear is off).
    pub clear_after_seconds: Option<u32>,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn requests() -> Vec<CopyRequest> {
        let item = |field| CopyRequest {
            source: CopySource::Item {
                vault_id: ids::vault(1),
                item_id: ids::item(1),
                field,
            },
        };
        vec![
            item(CopyField::Username),
            item(CopyField::Password),
            item(CopyField::Totp),
            item(CopyField::Text {
                field: TextFieldKey::Email,
            }),
            item(CopyField::Secret {
                field: SecretFieldKey::CardNumber,
            }),
            item(CopyField::CustomField {
                element_id: "5-019a2b3c4d5e7000".into(),
            }),
            CopyRequest {
                source: CopySource::Generated,
            },
        ]
    }

    pub(crate) fn receipts() -> Vec<CopyReceipt> {
        vec![
            CopyReceipt {
                clears_at: Some(1_761_480_030_000),
                clear_after_seconds: Some(30),
            },
            CopyReceipt {
                clears_at: None,
                clear_after_seconds: None,
            },
        ]
    }
}
