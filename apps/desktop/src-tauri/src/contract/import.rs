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
        /// Some one-time-password settings could not be read.
        OtpUnreadable,
        /// Passkeys are left out (they come with M5).
        PasskeysNotImported,
    }
);

wire_enum!(
    /// Something the user should know about one entry.
    pub enum ImportEntryWarning {
        /// Fields without a Mosaic equivalent became custom fields.
        UnmappedFields,
        /// The one-time-password setting could not be read.
        OtpUnreadable,
        /// An attachment is over the size limit and is left out.
        AttachmentTooLarge,
        /// An attachment's data is missing from the file.
        AttachmentMissing,
        /// Tags were dropped.
        TagsDropped,
        /// Older versions were dropped.
        HistoryTruncated,
        /// A passkey is left out.
        PasskeysNotImported,
        /// The category is unknown; the entry became a secure note.
        UnknownCategory,
        /// Some fields were over a limit and are left out.
        FieldsDropped,
        /// The entry was archived; it carries the tag `archived`.
        Archived,
        /// An attachment would exceed the import quota and is left out.
        AttachmentQuota,
    }
);

wire_enum!(
    /// Why an entry of the file is not imported.
    pub enum ImportSkipReason {
        /// The entry is not well-formed.
        Malformed,
        /// The entry holds no data.
        Empty,
        /// The entry is over the item size limit.
        TooLarge,
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
    /// An earlier entry of the same file that looks the same.
    pub duplicate_in_file: Option<u32>,
    /// Attachment files it carries.
    pub attachments: u32,
    /// What the user should know about this entry.
    pub warnings: Vec<ImportEntryWarning>,
}

/// An entry of the file that is left out.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSkipped {
    /// Position in the file.
    pub index: u32,
    /// Why.
    pub reason: ImportSkipReason,
}

/// How many entries become a type.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCount {
    /// The type.
    pub kind: ItemKind,
    /// How many.
    pub count: u32,
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
    /// The file's format.
    pub format: ImportFormat,
    /// Every entry that becomes an item.
    pub entries: Vec<ImportEntry>,
    /// Entries of the file that are left out.
    pub skipped: Vec<ImportSkipped>,
    /// Entries per type.
    pub counts: Vec<ImportCount>,
    /// Attachment files in all.
    pub attachments: u32,
    /// Their size in bytes.
    pub attachment_bytes: u64,
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
    /// Entries the vault refused.
    pub failed: u32,
    /// Attachment files added.
    pub attachments: u32,
    /// Attachment files the vault refused.
    pub attachments_failed: u32,
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
            format: ImportFormat::Enpass,
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
                    duplicate_in_file: None,
                    attachments: 1,
                    warnings: ImportEntryWarning::ALL.to_vec(),
                },
                ImportEntry {
                    index: 1,
                    kind: ItemKind::SecureNote,
                    title: None,
                    subtitle: None,
                    duplicate_of: None,
                    duplicate_in_file: Some(0),
                    attachments: 0,
                    warnings: vec![],
                },
            ],
            skipped: ImportSkipReason::ALL
                .iter()
                .zip(2_u32..)
                .map(|(&reason, index)| ImportSkipped { index, reason })
                .collect(),
            counts: vec![
                ImportCount {
                    kind: ItemKind::Login,
                    count: 1,
                },
                ImportCount {
                    kind: ItemKind::SecureNote,
                    count: 1,
                },
            ],
            attachments: 1,
            attachment_bytes: 2048,
            warnings: ImportWarning::ALL.to_vec(),
        }
    }
}
