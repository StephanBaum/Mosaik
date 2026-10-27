//! Clipboard commands: not built yet (vault UI track). The implementation
//! writes the system clipboard from Rust, marks the entry as concealed /
//! excluded from history where the OS supports it, clears it after
//! `Settings::clipboard_clear_seconds` unless the user copied something
//! else meanwhile, and emits `AppEvent::ClipboardCleared`.

use tauri::State;

use crate::contract::clipboard::{CopyReceipt, CopyRequest};
use crate::contract::errors::{IpcError, IpcResult};
use crate::state::AppState;

/// `copy_to_clipboard`.
#[tauri::command]
pub async fn copy_to_clipboard(
    state: State<'_, AppState>,
    request: CopyRequest,
) -> IpcResult<CopyReceipt> {
    let _ = (state, request);
    Err(IpcError::NOT_IMPLEMENTED)
}

/// `clear_clipboard`: clears it now if it still holds what Mosaic copied.
#[tauri::command]
pub async fn clear_clipboard(state: State<'_, AppState>) -> IpcResult<()> {
    let _ = state;
    Err(IpcError::NOT_IMPLEMENTED)
}
