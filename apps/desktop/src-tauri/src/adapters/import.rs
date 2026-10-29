//! Import (`mosaic-import`, ADR-0021). Rust reads the export into a
//! zeroizing buffer and parses it without holding the session; the webview
//! sees counts, titles, subtitles, warnings and duplicate hints. The parsed
//! data is held under a preview ID and dropped on commit, cancel, a new
//! preview or lock.

use std::fs::File;
use std::io::Read as _;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use mosaic_core::SecretString;
use mosaic_core::model::ItemType;
use mosaic_import::{EntryWarning, Format, MAX_FILE_LEN, SkipReason};
use mosaic_vault::session::{ItemFilter, UnlockedSession};
use zeroize::Zeroizing;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::import::{
    ImportCommitRequest, ImportCount, ImportDuplicate, ImportEntry, ImportEntryWarning,
    ImportFormat, ImportPreview, ImportResult, ImportSkipReason, ImportSkipped, ImportWarning,
};
use crate::contract::items::ItemKind;
use crate::state::guard;

/// The parsed export kept between `import_preview` and `import_commit`.
struct Held {
    preview_id: String,
    // Dropping the import wipes every value from the file.
    import: mosaic_import::Import,
}

/// Parses exports and holds one preview at a time.
#[derive(Default)]
pub struct ImportService {
    held: Mutex<Option<Held>>,
    next: AtomicU64,
}

impl ImportService {
    /// Reads and parses `file`. No session lock is held while this runs
    /// (the KDBX key derivation can take seconds).
    ///
    /// # Errors
    /// `importFileUnreadable`, `importTooLarge`, `importFormatInvalid`,
    /// `importUnsupported`, `importEncrypted`, `wrongCredentials`.
    pub fn parse(
        &self,
        file: &Path,
        format: ImportFormat,
        password: Option<&SecretString>,
    ) -> IpcResult<mosaic_import::Import> {
        let bytes = read_export(file)?;
        mosaic_import::parse(format_of(format), &bytes, password).map_err(map_error)
    }

    /// Runs the duplicate pass against the session and keeps the import,
    /// replacing an older one.
    ///
    /// # Errors
    /// `internal` on a poisoned mutex.
    pub fn hold(
        &self,
        import: mosaic_import::Import,
        session: &UnlockedSession,
    ) -> IpcResult<ImportPreview> {
        let preview = import.preview(&session.items(&ItemFilter::default()));
        let id = format!("preview-{}", self.next.fetch_add(1, Ordering::Relaxed) + 1);
        let wire = to_wire(&id, wire_format(import.format), &preview);
        *guard(&self.held)? = Some(Held {
            preview_id: id,
            import,
        });
        Ok(wire)
    }

    /// Writes the held entries (minus `skip`) into the target vault and
    /// drops the import.
    ///
    /// # Errors
    /// `importPreviewExpired`, `vaultNotFound`, `vaultUnavailable`.
    pub fn commit(
        &self,
        request: &ImportCommitRequest,
        session: &mut UnlockedSession,
    ) -> IpcResult<ImportResult> {
        let held = guard(&self.held)?
            .take_if(|h| h.preview_id == request.preview_id)
            .ok_or(IpcError::new(ErrorCode::ImportPreviewExpired))?;
        let report = mosaic_import::commit(
            held.import,
            session,
            request.target_vault_id.get(),
            &request.skip,
        )?;
        Ok(ImportResult {
            imported: report.imported,
            skipped: report.skipped,
            failed: report.failed,
            attachments: report.attachments,
            attachments_failed: report.attachments_failed,
        })
    }

    /// Drops the import if its ID matches (no-op otherwise).
    pub fn cancel(&self, preview_id: &str) {
        if let Ok(mut held) = self.held.lock() {
            let _ = held.take_if(|h| h.preview_id == preview_id);
        }
    }

    /// Drops the import (the vault locked).
    pub fn clear(&self) {
        if let Ok(mut held) = self.held.lock() {
            *held = None;
        }
    }
}

/// Reads an export into a zeroizing buffer of exactly its size, so the
/// buffer never reallocates and leaves a stale copy behind.
fn read_export(path: &Path) -> IpcResult<Zeroizing<Vec<u8>>> {
    let unreadable = || IpcError::new(ErrorCode::ImportFileUnreadable);
    let len = std::fs::metadata(path).map_err(|_| unreadable())?.len();
    if len > MAX_FILE_LEN as u64 {
        return Err(IpcError::new(ErrorCode::ImportTooLarge));
    }
    let len = usize::try_from(len).map_err(|_| unreadable())?;
    let mut buf = Zeroizing::new(Vec::with_capacity(len.saturating_add(1)));
    File::open(path)
        .map_err(|_| unreadable())?
        .take(len as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|_| unreadable())?;
    if buf.len() > len {
        // The file grew while it was read.
        return Err(unreadable());
    }
    Ok(buf)
}

const fn format_of(format: ImportFormat) -> Format {
    match format {
        ImportFormat::Enpass => Format::Enpass,
        ImportFormat::Bitwarden => Format::Bitwarden,
        ImportFormat::Keepass => Format::Keepass,
        ImportFormat::OnePassword => Format::OnePassword,
        ImportFormat::Csv => Format::Csv,
    }
}

const fn wire_format(format: Format) -> ImportFormat {
    match format {
        Format::Enpass => ImportFormat::Enpass,
        Format::Bitwarden => ImportFormat::Bitwarden,
        Format::Keepass => ImportFormat::Keepass,
        Format::OnePassword => ImportFormat::OnePassword,
        Format::Csv => ImportFormat::Csv,
    }
}

fn map_error(error: mosaic_import::Error) -> IpcError {
    use mosaic_import::Error as E;
    IpcError::new(match error {
        E::TooLarge | E::LimitExceeded => ErrorCode::ImportTooLarge,
        E::Malformed => ErrorCode::ImportFormatInvalid,
        E::UnsupportedVersion | E::Unsupported => ErrorCode::ImportUnsupported,
        E::Encrypted => ErrorCode::ImportEncrypted,
        E::PasswordRequired | E::WrongCredentials => ErrorCode::WrongCredentials,
        _ => ErrorCode::Internal,
    })
}

fn entry_warning(warning: EntryWarning) -> Option<ImportEntryWarning> {
    Some(match warning {
        EntryWarning::UnmappedFields => ImportEntryWarning::UnmappedFields,
        EntryWarning::OtpUnreadable => ImportEntryWarning::OtpUnreadable,
        EntryWarning::AttachmentTooLarge => ImportEntryWarning::AttachmentTooLarge,
        EntryWarning::AttachmentMissing => ImportEntryWarning::AttachmentMissing,
        EntryWarning::TagsDropped => ImportEntryWarning::TagsDropped,
        EntryWarning::HistoryTruncated => ImportEntryWarning::HistoryTruncated,
        EntryWarning::PasskeysNotImported => ImportEntryWarning::PasskeysNotImported,
        EntryWarning::UnknownCategory => ImportEntryWarning::UnknownCategory,
        EntryWarning::FieldsDropped => ImportEntryWarning::FieldsDropped,
        EntryWarning::Archived => ImportEntryWarning::Archived,
        EntryWarning::AttachmentQuota => ImportEntryWarning::AttachmentQuota,
        _ => return None,
    })
}

fn warning(warning: mosaic_import::ImportWarning) -> Option<ImportWarning> {
    use mosaic_import::ImportWarning as W;
    Some(match warning {
        W::UnmappedFields => ImportWarning::UnmappedFields,
        W::AttachmentsSkipped => ImportWarning::AttachmentsSkipped,
        W::EntriesSkipped => ImportWarning::EntriesSkipped,
        W::OtpUnreadable => ImportWarning::OtpUnreadable,
        W::PasskeysNotImported => ImportWarning::PasskeysNotImported,
        _ => return None,
    })
}

const fn skip_reason(reason: SkipReason) -> ImportSkipReason {
    match reason {
        SkipReason::Empty => ImportSkipReason::Empty,
        SkipReason::TooLarge => ImportSkipReason::TooLarge,
        _ => ImportSkipReason::Malformed,
    }
}

fn kind_of(item_type: &ItemType) -> ItemKind {
    ItemKind::from_model(item_type)
}

fn to_wire(id: &str, format: ImportFormat, p: &mosaic_import::Preview) -> ImportPreview {
    ImportPreview {
        preview_id: id.to_owned(),
        format,
        entries: p
            .entries
            .iter()
            .map(|e| ImportEntry {
                index: e.index,
                kind: kind_of(&e.item_type),
                title: e.title.as_ref().map(|t| t.as_str().to_owned()),
                subtitle: e.subtitle.as_ref().map(|t| t.as_str().to_owned()),
                duplicate_of: e.duplicate_of.map(|d| ImportDuplicate {
                    vault_id: d.vault_id.into(),
                    item_id: d.item_id.into(),
                }),
                duplicate_in_file: e.duplicate_in_file,
                attachments: e.attachments,
                warnings: e
                    .warnings
                    .iter()
                    .copied()
                    .filter_map(entry_warning)
                    .collect(),
            })
            .collect(),
        skipped: p
            .skipped
            .iter()
            .map(|s| ImportSkipped {
                index: s.index,
                reason: skip_reason(s.reason),
            })
            .collect(),
        counts: p
            .counts
            .iter()
            .map(|(t, count)| ImportCount {
                kind: kind_of(t),
                count: *count,
            })
            .collect(),
        attachments: p.attachments,
        attachment_bytes: p.attachment_bytes,
        warnings: p.warnings.iter().copied().filter_map(warning).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_map_to_codes() {
        use mosaic_import::Error as E;
        let code = |e| map_error(e).code;
        assert_eq!(code(E::TooLarge), ErrorCode::ImportTooLarge);
        assert_eq!(code(E::LimitExceeded), ErrorCode::ImportTooLarge);
        assert_eq!(code(E::Malformed), ErrorCode::ImportFormatInvalid);
        assert_eq!(code(E::UnsupportedVersion), ErrorCode::ImportUnsupported);
        assert_eq!(code(E::Unsupported), ErrorCode::ImportUnsupported);
        assert_eq!(code(E::Encrypted), ErrorCode::ImportEncrypted);
        assert_eq!(code(E::PasswordRequired), ErrorCode::WrongCredentials);
        assert_eq!(code(E::WrongCredentials), ErrorCode::WrongCredentials);
        assert_eq!(code(E::Allocation), ErrorCode::Internal);
    }

    #[test]
    fn formats_round_trip() {
        for &f in ImportFormat::ALL {
            assert_eq!(wire_format(format_of(f)), f);
        }
    }

    #[test]
    fn a_missing_file_is_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let error = read_export(&dir.path().join("none.json")).unwrap_err();
        assert_eq!(error.code, ErrorCode::ImportFileUnreadable);
    }

    #[test]
    fn a_file_over_the_limit_is_refused_before_reading() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.json");
        let file = File::create(&path).unwrap();
        file.set_len(MAX_FILE_LEN as u64 + 1).unwrap();
        drop(file);
        assert_eq!(
            read_export(&path).unwrap_err().code,
            ErrorCode::ImportTooLarge
        );
    }

    #[test]
    fn cancel_and_clear_drop_nothing_that_is_absent() {
        let service = ImportService::default();
        service.cancel("preview-1");
        service.clear();
    }
}
