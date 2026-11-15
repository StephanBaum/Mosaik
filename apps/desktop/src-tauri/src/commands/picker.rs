//! The commands of the two picker windows (ADR-0164): the chip (`picker-chip`) and the list
//! (`picker`). Each window has its own capability that grants only its own commands, and every
//! command checks the calling window's label again in Rust, so a capability mistake cannot hand
//! the list's commands to the chip or the picker commands to another window. Nothing here
//! returns a secret: logins travel as ID, title, user name, vault name and host.

use tauri::ipc::Channel;
use tauri::{Runtime, State, Webview};

use super::names::{CHIP_COMMANDS, LIST_COMMANDS};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::picker::{
    LinkConfirmResult, LinkStep, PickerEvent, PickerGenerationRequest, PickerItemRequest,
    PickerLogin, PickerSearchRequest, PickerState,
};
use crate::program_fill::PickerLabel;
use crate::state::{AppState, Backend};

/// The label of the chip window.
pub const CHIP_LABEL: &str = "picker-chip";
/// The label of the list window.
pub const LIST_LABEL: &str = "picker";

/// Which picker window `label` is, when it may call `command`.
///
/// # Errors
/// `invalidInput` for any other window or command.
pub fn check_window(label: &str, command: &str) -> IpcResult<PickerLabel> {
    let (which, allowed) = match label {
        CHIP_LABEL => (PickerLabel::Chip, CHIP_COMMANDS),
        LIST_LABEL => (PickerLabel::List, LIST_COMMANDS),
        _ => return Err(IpcError::new(ErrorCode::InvalidInput)),
    };
    if allowed.contains(&command) {
        Ok(which)
    } else {
        Err(IpcError::new(ErrorCode::InvalidInput))
    }
}

/// `picker_subscribe`: the window hands over the channel of its `changed` events (the latest
/// subscription of a window wins).
#[tauri::command]
pub async fn picker_subscribe<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, AppState>,
    channel: Channel<PickerEvent>,
) -> IpcResult<()> {
    let which = check_window(webview.label(), "picker_subscribe")?;
    state.backend().program_fill.subscribe(which, channel);
    Ok(())
}

/// `picker_state`.
#[tauri::command]
pub async fn picker_state<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, AppState>,
) -> IpcResult<PickerState> {
    check_window(webview.label(), "picker_state")?;
    state.run(Backend::picker_state).await
}

/// `picker_chip_click`.
#[tauri::command]
pub async fn picker_chip_click<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, AppState>,
    request: PickerGenerationRequest,
) -> IpcResult<()> {
    check_window(webview.label(), "picker_chip_click")?;
    state.run(move |b| b.picker_chip_click(&request)).await
}

/// `picker_search`.
#[tauri::command]
pub async fn picker_search<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, AppState>,
    request: PickerSearchRequest,
) -> IpcResult<Vec<PickerLogin>> {
    check_window(webview.label(), "picker_search")?;
    state.run(move |b| b.picker_search(&request)).await
}

/// `picker_choose`.
#[tauri::command]
pub async fn picker_choose<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, AppState>,
    request: PickerItemRequest,
) -> IpcResult<()> {
    check_window(webview.label(), "picker_choose")?;
    state.run(move |b| b.picker_choose(&request)).await
}

/// `picker_link_begin`.
#[tauri::command]
pub async fn picker_link_begin<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, AppState>,
    request: PickerItemRequest,
) -> IpcResult<LinkStep> {
    check_window(webview.label(), "picker_link_begin")?;
    state.run(move |b| b.picker_link_begin(&request)).await
}

/// `picker_link_confirm`.
#[tauri::command]
pub async fn picker_link_confirm<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, AppState>,
    request: PickerGenerationRequest,
) -> IpcResult<LinkConfirmResult> {
    check_window(webview.label(), "picker_link_confirm")?;
    state.run(move |b| b.picker_link_confirm(&request)).await
}

/// `picker_block`.
#[tauri::command]
pub async fn picker_block<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, AppState>,
    request: PickerGenerationRequest,
) -> IpcResult<()> {
    check_window(webview.label(), "picker_block")?;
    state.run(move |b| b.picker_block(&request)).await
}

/// `picker_dismiss`.
#[tauri::command]
pub async fn picker_dismiss<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, AppState>,
    request: PickerGenerationRequest,
) -> IpcResult<()> {
    check_window(webview.label(), "picker_dismiss")?;
    state.run(move |b| b.picker_dismiss(&request)).await
}
