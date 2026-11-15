//! Import commands. The shell opens the native file dialog and keeps the
//! chosen path behind an opaque token, so no dialog or fs permission is
//! granted to the webview; parsing, preview and commit are the import
//! service's (`adapters::ImportService`, `mosaic-import`). Parsing runs
//! without the session lock. Tokens and previews are dropped on lock.

use tauri::State;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::import::{
    ImportCancelRequest, ImportCommitRequest, ImportDeleteRequest, ImportDetectRequest,
    ImportDetected, ImportFileChoice, ImportPreview, ImportPreviewRequest, ImportResult,
};
use crate::dialogs::FileFilter;
use crate::state::{AppState, Backend, guard};

/// `import_choose_file`.
#[tauri::command]
pub async fn import_choose_file(state: State<'_, AppState>) -> IpcResult<ImportFileChoice> {
    state.run(Backend::import_choose_file).await
}

/// `import_detect`.
#[tauri::command]
pub async fn import_detect(
    state: State<'_, AppState>,
    request: ImportDetectRequest,
) -> IpcResult<ImportDetected> {
    state.run(move |b| b.import_detect(&request)).await
}

/// `import_delete_file`.
#[tauri::command]
pub async fn import_delete_file(
    state: State<'_, AppState>,
    request: ImportDeleteRequest,
) -> IpcResult<()> {
    state.run(move |b| b.import_delete_file(&request)).await
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

/// The import dialog's file filter: every export type Mosaic reads. The
/// content, not the extension, decides the format (`import_detect`).
pub const EXPORT_FILES: FileFilter = FileFilter {
    name: "Export files",
    extensions: &["csv", "json", "zip", "1pux", "kdbx"],
};

impl Backend {
    /// Opens the file dialog (while unlocked) and returns a token.
    ///
    /// # Errors
    /// `locked`, `internal` if no dialog can be shown.
    pub fn import_choose_file(&self) -> IpcResult<ImportFileChoice> {
        self.touch();
        self.slot()?.unlocked()?;
        let Some(path) = self.dialogs.open_import_file(EXPORT_FILES)? else {
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

    /// Says what the chosen file is (product and format), from its content.
    ///
    /// # Errors
    /// `locked`, `importFileUnreadable` (unknown token), `importTooLarge`.
    pub fn import_detect(&self, request: &ImportDetectRequest) -> IpcResult<ImportDetected> {
        self.touch();
        let path = guard(&self.import_files)?
            .get(&request.file_token)
            .cloned()
            .ok_or(IpcError::new(ErrorCode::ImportFileUnreadable))?;
        self.slot()?.unlocked()?;
        self.adapters.import.detect(&path)
    }

    /// Parses the chosen file (outside the session lock) and previews it
    /// against the unlocked vault.
    ///
    /// # Errors
    /// `locked`, `importFileUnreadable` (unknown token), `importTooLarge`,
    /// `importFormatInvalid`, `importUnsupported`, `importEncrypted`,
    /// `wrongCredentials`.
    pub fn import_preview(&self, request: ImportPreviewRequest) -> IpcResult<ImportPreview> {
        self.touch();
        let path = guard(&self.import_files)?
            .get(&request.file_token)
            .cloned()
            .ok_or(IpcError::new(ErrorCode::ImportFileUnreadable))?;
        // Fail fast; the guard is dropped before the parse starts.
        self.slot()?.unlocked()?;
        let password = request
            .password
            .map(crate::contract::secret::SecretIn::into_secret);
        let import = self
            .adapters
            .import
            .parse(&path, request.format, password.as_ref())?;
        // If the vault locked meanwhile this is `locked`, and the parsed
        // import is dropped (wiped) here.
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        self.adapters
            .import
            .hold(import, session, &request.file_token)
    }

    /// Commits a preview.
    ///
    /// # Errors
    /// `locked`, `importPreviewExpired`, `vaultNotFound`, `vaultUnavailable`.
    pub fn import_commit(&self, request: &ImportCommitRequest) -> IpcResult<ImportResult> {
        self.touch();
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        let (result, file_token) = self.adapters.import.commit(request, session)?;
        self.items_changed(session);
        // The file may be deleted only when everything it held that was not
        // left out on purpose is in the vault.
        if result.failed == 0 && result.attachments_failed == 0 {
            guard(&self.import_files)?.mark_committed(&file_token);
        }
        Ok(result)
    }

    /// Overwrites the imported export file once and deletes it. Only a file
    /// this session imported successfully (by token) is touched; the path
    /// never comes from the webview.
    ///
    /// # Errors
    /// `locked`, `importFileUnreadable` (unknown token),
    /// `importDeleteNotAllowed` (not imported yet), `importDeleteFailed`
    /// (the file is still there; the token stays for a retry).
    pub fn import_delete_file(&self, request: &ImportDeleteRequest) -> IpcResult<()> {
        self.touch();
        self.slot()?.unlocked()?;
        let path = {
            let files = guard(&self.import_files)?;
            let path = files
                .get(&request.file_token)
                .cloned()
                .ok_or(IpcError::new(ErrorCode::ImportFileUnreadable))?;
            if !files.is_committed(&request.file_token) {
                return Err(IpcError::new(ErrorCode::ImportDeleteNotAllowed));
            }
            path
        };
        match crate::export_cleanup::overwrite_and_delete(&path) {
            Ok(()) => {}
            // Already gone (the user deleted it): nothing is left to delete.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(IpcError::new(ErrorCode::ImportDeleteFailed)),
        }
        guard(&self.import_files)?.remove(&request.file_token);
        Ok(())
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
