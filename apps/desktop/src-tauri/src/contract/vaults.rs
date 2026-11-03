//! Vaults: list, create, rename.

use serde::{Deserialize, Serialize};

use super::ids::{UnixMillis, WireVaultId};
use super::macros::wire_enum;

wire_enum!(
    /// How a vault's key is protected (CRYPTO_SPEC section 7).
    pub enum VaultProtection {
        /// Wrapped under the account key: open whenever the app is unlocked.
        AccountKey,
        /// A one-time-code vault (section 7.4) opened with a security key
        /// for five minutes at a time.
        HardwareKey,
    }
);

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
    /// How the vault's key is protected.
    pub protection: VaultProtection,
    /// When an open hardware-protected vault closes by itself (`null`
    /// otherwise). A wall-clock projection of a monotonic deadline, for
    /// display only.
    pub open_until: Option<UnixMillis>,
}

/// A request that names a vault (`close_hardware_vault`,
/// `hardware_vault_keys`).
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultRequest {
    /// The vault.
    pub vault_id: WireVaultId,
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
                protection: VaultProtection::AccountKey,
                open_until: None,
            },
            VaultSummary {
                vault_id: ids::vault(2),
                name: None,
                item_count: 0,
                available: false,
                protection: VaultProtection::HardwareKey,
                open_until: None,
            },
            VaultSummary {
                vault_id: ids::vault(3),
                name: Some("Authenticators".into()),
                item_count: 5,
                available: true,
                protection: VaultProtection::HardwareKey,
                open_until: Some(1_761_480_300_000),
            },
        ]
    }
}
