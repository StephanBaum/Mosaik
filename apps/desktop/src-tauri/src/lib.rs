//! The Mosaic desktop app (M3): a Tauri v2 shell that holds the vault
//! session in Rust and serves the React UI over a typed IPC contract
//! (`docs/DESKTOP_IPC.md`).
//!
//! Security baseline (ADR-0019): the webview gets only the commands in
//! [`commands::names::COMMANDS`] (no shell, fs, http or dialog plugin, no
//! `core:*` permission), a strict CSP without remote origins or
//! `unsafe-eval`, a frozen `Object.prototype`, no devtools in release
//! builds, and one running instance per user. Keys and decrypted items stay
//! in Rust; secrets reach JavaScript only through the contract's `SecretOut`
//! (explicit reveals, generated passwords, one-time codes).
#![forbid(unsafe_code)]

pub mod commands;
pub mod contract;
pub mod keystore;
pub mod state;

use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use tauri::Manager;

use crate::contract::errors::{ErrorCode, IpcError};
use crate::contract::events::LockReason;
use crate::keystore::MemoryKeystore;
use crate::state::{AppState, Backend, Slot};

/// The store's file name in the app data directory.
pub const STORE_FILE: &str = "vault.sqlite3";

/// Opens (or creates) the store in `dir`.
#[must_use]
pub fn open_store(dir: &std::path::Path) -> Slot {
    if std::fs::create_dir_all(dir).is_err() {
        return Slot::Unavailable(ErrorCode::StoreUnavailable);
    }
    match Store::open(dir.join(STORE_FILE)) {
        Ok(store) => Slot::Locked(LockedSession::open(store)),
        Err(error) => Slot::Unavailable(IpcError::from(error).code),
    }
}

fn focus_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        // Best effort: a window that cannot be raised stays where it is.
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Runs the app until the last window closes.
///
/// # Errors
/// Tauri's error if the app cannot start.
pub fn run() -> tauri::Result<()> {
    let app = tauri::Builder::default()
        // First, so a second launch hands over before anything else starts
        // (one store session per install, CORE_API).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            focus_main_window(app);
        }))
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let backend = Backend::new(
                open_store(&dir),
                Box::new(MemoryKeystore::default()),
                mosaic_core::kdf::calibrate,
            );
            app.manage(AppState::new(backend));
            Ok(())
        })
        .invoke_handler(commands::handler())
        .build(tauri::generate_context!())?;
    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event
            && let Some(state) = handle.try_state::<AppState>()
        {
            // Drop keys and decrypted items before the process ends.
            let _ = state.backend().lock(LockReason::Manual);
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests;
