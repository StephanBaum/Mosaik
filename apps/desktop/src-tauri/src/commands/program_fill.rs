//! Program fill commands of the main window (ADR-0164): the status and the block list. Picks,
//! links and the picker state belong to the two picker windows (`commands::picker`).

use tauri::State;

use crate::contract::errors::IpcResult;
use crate::contract::program_fill::{ProgramFillStatus, ProgramFillUnblockRequest};
use crate::state::AppState;

/// `program_fill_status`.
#[tauri::command]
pub async fn program_fill_status(state: State<'_, AppState>) -> IpcResult<ProgramFillStatus> {
    state.run(|b| Ok(b.program_fill_status())).await
}

/// `program_fill_unblock`.
#[tauri::command]
pub async fn program_fill_unblock(
    state: State<'_, AppState>,
    request: ProgramFillUnblockRequest,
) -> IpcResult<ProgramFillStatus> {
    state.run(move |b| b.program_fill_unblock(&request)).await
}
