//! Import (mosaic-import): Rust opens the native file dialog and reads the
//! file itself, so the export's plaintext never enters JavaScript. The
//! webview sees a preview with titles and duplicate hints, then commits or
//! cancels; parsed data is dropped (zeroized) on commit, cancel or lock.

use serde::{Deserialize, Serialize};

use super::ids::{WireItemId, WireVaultId};
use super::items::ItemKind;
use super::macros::wire_enum;
use super::secret::SecretIn;

wire_enum!(
    /// Supported export formats (brief §5).
    pub enum ImportFormat {
        /// Enpass JSON (priority).
        Enpass,
        /// Bitwarden JSON (unencrypted export).
        Bitwarden,
        /// KeePass KDBX 4.
        Keepass,
        /// 1Password 1PUX.
        OnePassword,
        /// Generic CSV with a column mapping.
        Csv,
    }
);

wire_enum!(
    /// Something the user should know about the preview.
    pub enum ImportWarning {
        /// Some entries have fields Mosaic keeps as custom fields.
        UnmappedFields,
        /// Some attachments were skipped (over 50 MiB or the quota).
        AttachmentsSkipped,
        /// Some entries could not be read and are left out.
        EntriesSkipped,
    }
);

/// `import_choose_file`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportChooseRequest {
    /// The format, which picks the dialog's file filter.
    pub format: ImportFormat,
}

/// The `import_choose_file` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ImportFileChoice {
    /// A file was chosen; Rust keeps its path behind the token.
    Chosen {
        /// Opaque handle for `import_preview`.
        file_token: String,
        /// The file name, for display.
        file_name: String,
    },
    /// The dialog was cancelled.
    Cancelled,
}

/// `import_preview`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportPreviewRequest {
    /// From `import_choose_file`.
    pub file_token: String,
    /// The format.
    pub format: ImportFormat,
    /// The file's password (KeePass), or `null`.
    pub password: Option<SecretIn>,
}

/// One entry of the preview.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportEntry {
    /// Position in the file, for `skip`.
    pub index: u32,
    /// The type it becomes.
    pub kind: ItemKind,
    /// Its title (`null` if empty).
    pub title: Option<String>,
    /// User name or similar (`null` if none).
    pub subtitle: Option<String>,
    /// An existing item that looks the same (same type, title, user name and
    /// URL), suggested for skipping.
    pub duplicate_of: Option<ImportDuplicate>,
}

/// An existing item an entry duplicates.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDuplicate {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
}

/// The `import_preview` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    /// Handle for `import_commit` and `import_cancel`.
    pub preview_id: String,
    /// Every entry.
    pub entries: Vec<ImportEntry>,
    /// Warnings to show above the list.
    pub warnings: Vec<ImportWarning>,
}

/// `import_commit`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportCommitRequest {
    /// From `import_preview`.
    pub preview_id: String,
    /// The vault to import into.
    pub target_vault_id: WireVaultId,
    /// Entry indexes to leave out.
    pub skip: Vec<u32>,
}

/// `import_cancel`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportCancelRequest {
    /// From `import_preview`.
    pub preview_id: String,
}

/// The `import_commit` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    /// Items added.
    pub imported: u32,
    /// Entries left out.
    pub skipped: u32,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn choices() -> Vec<ImportFileChoice> {
        vec![
            ImportFileChoice::Chosen {
                file_token: "f-1".into(),
                file_name: "enpass-export.json".into(),
            },
            ImportFileChoice::Cancelled,
        ]
    }

    pub(crate) fn preview() -> ImportPreview {
        ImportPreview {
            preview_id: "p-1".into(),
            entries: vec![
                ImportEntry {
                    index: 0,
                    kind: ItemKind::Login,
                    title: Some("GitHub".into()),
                    subtitle: Some("alex@example.com".into()),
                    duplicate_of: Some(ImportDuplicate {
                        vault_id: ids::vault(1),
                        item_id: ids::item(1),
                    }),
                },
                ImportEntry {
                    index: 1,
                    kind: ItemKind::SecureNote,
                    title: None,
                    subtitle: None,
                    duplicate_of: None,
                },
            ],
            warnings: ImportWarning::ALL.to_vec(),
        }
    }
}
