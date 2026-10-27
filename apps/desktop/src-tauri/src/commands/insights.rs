//! Security dashboard command: not built yet (insights track, on top of
//! `mosaic-insights`).

use tauri::State;

use crate::contract::errors::{IpcError, IpcResult};
use crate::contract::insights::SecurityReport;
use crate::state::AppState;

/// `security_report`.
#[tauri::command]
pub async fn security_report(state: State<'_, AppState>) -> IpcResult<SecurityReport> {
    let _ = state;
    Err(IpcError::NOT_IMPLEMENTED)
}
