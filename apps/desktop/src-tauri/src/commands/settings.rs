//! Settings commands. Settings live in memory for now; the settings track
//! persists them and enforces re-authentication for weakening changes
//! (`reauthRequired`).

use tauri::State;

use crate::contract::errors::{IpcError, IpcResult};
use crate::contract::settings::{Settings, UpdateSettingsRequest};
use crate::state::{AppState, guard};

/// `get_settings`.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> IpcResult<Settings> {
    Ok(guard(&state.backend().settings)?.clone())
}

/// `update_settings`: not built yet (settings track).
#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    request: UpdateSettingsRequest,
) -> IpcResult<Settings> {
    let _ = (state, request);
    Err(IpcError::NOT_IMPLEMENTED)
}
