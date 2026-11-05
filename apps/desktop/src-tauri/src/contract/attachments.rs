//! Attachments (`docs/DESKTOP_IPC.md` §3): add a file through a native
//! dialog, preview an image, save a file through a native dialog, remove.
//! Requests carry IDs only; a path never crosses the boundary in either
//! direction (ADR-0034).

use serde::{Deserialize, Serialize};

use super::ids::{WireAttachmentId, WireItemId, WireVaultId};
use super::items::ItemDetail;

/// `attachment_preview`, `attachment_save`, `attachment_remove`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttachmentRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// The attachment.
    pub attachment_id: WireAttachmentId,
}

/// `attachment_add`: the user chose a file, or cancelled the dialog.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AttachmentAdded {
    /// The file is encrypted and part of the item.
    Added {
        /// The item with the new file.
        item: Box<ItemDetail>,
    },
    /// The user cancelled the dialog. Nothing changed.
    Cancelled,
}

/// `attachment_save`: the file was written, or the user cancelled.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AttachmentSaved {
    /// The decrypted file was written to the chosen path.
    Saved,
    /// The user cancelled the dialog. Nothing was decrypted.
    Cancelled,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn request() -> AttachmentRequest {
        AttachmentRequest {
            vault_id: ids::vault(1),
            item_id: ids::item(1),
            attachment_id: ids::attachment(1),
        }
    }
}
