//! One-time code command: not built yet (TOTP track: `otp_code` of the
//! session, HOTP counter handling, hardware-protected vaults).

use tauri::State;

use crate::contract::errors::{IpcError, IpcResult};
use crate::contract::items::ItemRequest;
use crate::contract::totp::TotpCode;
use crate::state::AppState;

/// `totp_code`.
#[tauri::command]
pub async fn totp_code(state: State<'_, AppState>, request: ItemRequest) -> IpcResult<TotpCode> {
    let _ = (state, request);
    Err(IpcError::NOT_IMPLEMENTED)
}
