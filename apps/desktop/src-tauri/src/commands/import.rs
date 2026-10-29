//! Import commands. The shell opens the native file dialog and keeps the
//! chosen path behind an opaque token, so no dialog or fs permission is
//! granted to the webview; parsing, preview and commit are the import
//! adapter's (`adapters::ImportAdapter`, `mosaic-import`). Tokens and
//! previews are dropped on lock.

use tauri::State;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::import::{
    ImportCancelRequest, ImportChooseRequest, ImportCommitRequest, ImportFileChoice, ImportFormat,
    ImportPreview, ImportPreviewRequest, ImportResult,
};
use crate::dialogs::FileFilter;
use crate::state::{AppState, Backend, guard};

/// `import_choose_file`.
#[tauri::command]
pub async fn import_choose_file(
    state: State<'_, AppState>,
    request: ImportChooseRequest,
) -> IpcResult<ImportFileChoice> {
    state.run(move |b| b.import_choose_file(request)).await
}

/// `import_preview`.
#[tauri::command]
pub async fn import_preview(
    state: State<'_, AppState>,
    request: ImportPreviewRequest,
) -> IpcResult<ImportPreview> {
    state.run(move |b| b.import_preview(request)).await
}

/// `import_commit`.
#[tauri::command]
pub async fn import_commit(
    state: State<'_, AppState>,
    request: ImportCommitRequest,
) -> IpcResult<ImportResult> {
    state.run(move |b| b.import_commit(&request)).await
}

/// `import_cancel`.
#[tauri::command]
pub async fn import_cancel(
    state: State<'_, AppState>,
    request: ImportCancelRequest,
) -> IpcResult<()> {
    state.run(move |b| b.import_cancel(&request)).await
}

/// The dialog filter of each format.
#[must_use]
pub const fn filter(format: ImportFormat) -> FileFilter {
    match format {
        ImportFormat::Enpass => FileFilter {
            name: "Enpass JSON",
            extensions: &["json"],
        },
        ImportFormat::Bitwarden => FileFilter {
            name: "Bitwarden JSON",
            extensions: &["json"],
        },
        ImportFormat::Keepass => FileFilter {
            name: "KeePass KDBX",
            extensions: &["kdbx"],
        },
        ImportFormat::OnePassword => FileFilter {
            name: "1Password 1PUX",
            extensions: &["1pux"],
        },
        ImportFormat::Csv => FileFilter {
            name: "CSV",
            extensions: &["csv"],
        },
    }
}

impl Backend {
    /// Opens the file dialog (while unlocked) and returns a token.
    ///
    /// # Errors
    /// `locked`, `internal` if no dialog can be shown.
    pub fn import_choose_file(&self, request: ImportChooseRequest) -> IpcResult<ImportFileChoice> {
        self.touch();
        self.slot()?.unlocked()?;
        let Some(path) = self.dialogs.open_file(filter(request.format))? else {
            return Ok(ImportFileChoice::Cancelled);
        };
        // The vault may have locked while the dialog was open: keep no token then.
        let slot = self.slot()?;
        slot.unlocked()?;
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let file_token = guard(&self.import_files)?.insert(path);
        drop(slot);
        Ok(ImportFileChoice::Chosen {
            file_token,
            file_name,
        })
    }

    /// Parses the chosen file through the import adapter.
    ///
    /// # Errors
    /// `locked`, `importFileUnreadable` (unknown token), the adapter's
    /// errors.
    pub fn import_preview(&self, request: ImportPreviewRequest) -> IpcResult<ImportPreview> {
        self.touch();
        let path = guard(&self.import_files)?
            .get(&request.file_token)
            .cloned()
            .ok_or(IpcError::new(ErrorCode::ImportFileUnreadable))?;
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        self.adapters.import.preview(
            &path,
            request.format,
            request
                .password
                .map(crate::contract::secret::SecretIn::into_secret),
            session,
        )
    }

    /// Commits a preview through the import adapter.
    ///
    /// # Errors
    /// `locked`, the adapter's errors.
    pub fn import_commit(&self, request: &ImportCommitRequest) -> IpcResult<ImportResult> {
        self.touch();
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        self.adapters.import.commit(request, session)
    }

    /// Drops a preview.
    ///
    /// # Errors
    /// None today (the signature leaves room for one).
    pub fn import_cancel(&self, request: &ImportCancelRequest) -> IpcResult<()> {
        self.touch();
        self.adapters.import.cancel(&request.preview_id);
        Ok(())
    }
}
