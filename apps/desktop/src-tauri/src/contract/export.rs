//! Export (`docs/DESKTOP_IPC.md` §3): passkeys to a FIDO CXF v1.0 JSON file.
//! Rust opens the native save dialog and writes the file itself; the file
//! holds private keys unencrypted, so the command needs the master password
//! again (ADR-0041).

use serde::{Deserialize, Serialize};

use super::ids::WireVaultId;
use super::secret::SecretIn;

/// `export_passkeys_cxf`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportPasskeysRequest {
    /// The vault whose ES256 passkeys are exported.
    pub vault_id: WireVaultId,
    /// The master password, checked now; or `null` within two minutes of a
    /// right one (`reauthenticate`). A success spends the re-authentication.
    pub password: Option<SecretIn>,
}

/// The `export_passkeys_cxf` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PasskeysExported {
    /// The file was written to the chosen path.
    Saved {
        /// How many passkeys it holds.
        count: u32,
    },
    /// The user cancelled the dialog. Nothing was written.
    Cancelled,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn requests() -> Vec<ExportPasskeysRequest> {
        vec![ExportPasskeysRequest {
            vault_id: ids::vault(1),
            password: None,
        }]
    }

    pub(crate) fn results() -> Vec<PasskeysExported> {
        vec![
            PasskeysExported::Saved { count: 2 },
            PasskeysExported::Cancelled,
        ]
    }
}
