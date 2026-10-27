//! Vaults: list, create, rename.

use serde::{Deserialize, Serialize};

use super::ids::WireVaultId;

/// A vault in the sidebar.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultSummary {
    /// The vault.
    pub vault_id: WireVaultId,
    /// Its name; `null` if the metadata is missing or unreadable (the UI
    /// shows a localized placeholder).
    pub name: Option<String>,
    /// Live items, in and outside the trash.
    pub item_count: u32,
    /// Whether this session holds the vault's key. A hardware-protected TOTP
    /// vault (CRYPTO_SPEC §7.4) is unavailable until its key is unlocked.
    pub available: bool,
}

/// `create_vault`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateVaultRequest {
    /// The name (user content, stored encrypted).
    pub name: String,
}

/// `rename_vault`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameVaultRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The new name.
    pub name: String,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn summaries() -> Vec<VaultSummary> {
        vec![
            VaultSummary {
                vault_id: ids::vault(1),
                name: Some("Personal".into()),
                item_count: 42,
                available: true,
            },
            VaultSummary {
                vault_id: ids::vault(2),
                name: None,
                item_count: 0,
                available: false,
            },
        ]
    }
}
