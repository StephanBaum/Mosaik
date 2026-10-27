//! Import commands: not built yet (import track, on top of `mosaic-import`).
//! The native file dialog runs from Rust, so no dialog or fs permission is
//! granted to the webview.

use tauri::State;

use crate::contract::errors::{IpcError, IpcResult};
use crate::contract::import::{
    ImportCancelRequest, ImportChooseRequest, ImportCommitRequest, ImportFileChoice, ImportPreview,
    ImportPreviewRequest, ImportResult,
};
use crate::state::AppState;

/// `import_choose_file`.
#[tauri::command]
pub async fn import_choose_file(
    state: State<'_, AppState>,
    request: ImportChooseRequest,
) -> IpcResult<ImportFileChoice> {
    let _ = (state, request);
    Err(IpcError::NOT_IMPLEMENTED)
}

/// `import_preview`.
#[tauri::command]
pub async fn import_preview(
    state: State<'_, AppState>,
    request: ImportPreviewRequest,
) -> IpcResult<ImportPreview> {
    let _ = (state, request);
    Err(IpcError::NOT_IMPLEMENTED)
}

/// `import_commit`.
#[tauri::command]
pub async fn import_commit(
    state: State<'_, AppState>,
    request: ImportCommitRequest,
) -> IpcResult<ImportResult> {
    let _ = (state, request);
    Err(IpcError::NOT_IMPLEMENTED)
}

/// `import_cancel`.
#[tauri::command]
pub async fn import_cancel(
    state: State<'_, AppState>,
    request: ImportCancelRequest,
) -> IpcResult<()> {
    let _ = (state, request);
    Err(IpcError::NOT_IMPLEMENTED)
}
