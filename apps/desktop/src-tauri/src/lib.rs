//! The Mosaic desktop app (M3): a Tauri v2 shell that holds the vault
//! session in Rust and serves the React UI over a typed IPC contract
//! (`docs/DESKTOP_IPC.md`).
//!
//! Security baseline (ADR-0019): the webview gets only the commands in
//! [`commands::names::COMMANDS`] (no shell, fs, http or dialog plugin, no
//! `core:*` permission), a strict CSP without remote origins or
//! `unsafe-eval`, a frozen `Object.prototype`, no devtools in release
//! builds, windows excluded from screen capture, and one running instance
//! per user. Keys and decrypted items stay in Rust; secrets reach
//! JavaScript only through the contract's `SecretOut` (explicit reveals,
//! generated passwords, one-time codes, the Secret Key once at account
//! creation).
//!
//! What the shell adds around the session:
//!
//! - [`keystore`]: the Secret Key in the OS keystore;
//! - [`clipboard`]: copy from Rust with history exclusion and auto-clear;
//! - [`autolock`]: idle, sleep and screen-lock locking;
//! - [`persist`]: settings and the account reference in a local file;
//! - [`shortcut`]: the global shortcut (focuses the main window);
//! - [`emergency_kit`] and [`dialogs`]: the Emergency Kit PDF, saved where
//!   the user chooses;
//! - [`quick_unlock`]: the platform half of quick unlock (Windows Hello);
//! - [`adapters`]: the import and SSH agent services.
//!
//! CRYPTO_SPEC §11 floor: the auto-lock triggers (idle, sleep, screen lock)
//! cannot all be disabled. `update_settings` refuses such a request with
//! `invalidInput`, and a state file that says so is read as the default idle
//! timeout ([`persist::DEFAULT_AUTO_LOCK_MINUTES`]).
//!
#![forbid(unsafe_code)]

pub mod adapters;
pub mod autolock;
pub mod clipboard;
pub mod commands;
pub mod contract;
pub mod dialogs;
pub mod emergency_kit;
pub mod events;
pub mod keystore;
pub mod persist;
pub mod quick_unlock;
pub mod shortcut;
pub mod state;

use std::sync::{Arc, Mutex};

use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use tauri::Manager;

use crate::adapters::{Adapters, ImportService, SshAgentService};
use crate::autolock::{Monitor, TICK, spawn_os_sources};
use crate::clipboard::SystemClipboard;
use crate::contract::errors::{ErrorCode, IpcError};
use crate::contract::events::LockReason;
use crate::dialogs::NativeDialogs;
use crate::keystore::{LayeredKeystore, OsKeystore};
use crate::persist::Persistence;
use crate::quick_unlock::QuickUnlock;
use crate::state::{AppState, Backend, Slot, guard};

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

/// The auto-lock monitor, kept alive for the life of the app.
struct MonitorHandle(Mutex<Monitor>);

/// Runs the app until the last window closes.
///
/// # Errors
/// Tauri's error if the app cannot start.
pub fn run() -> tauri::Result<()> {
    let app = tauri::Builder::default()
        // First, so a second launch hands over before anything else starts
        // (one store session per install, CORE_API).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            shortcut::focus_main(app);
        }))
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let slot = open_store(&dir);
            let persistence = if dir.is_dir() {
                Persistence::in_dir(&dir)
            } else {
                Persistence::memory()
            };
            let keystore =
                LayeredKeystore::new(Box::new(OsKeystore::new(&app.config().identifier)));
            let local_dir = app.path().app_local_data_dir()?;
            let backend = Backend::new(slot, Box::new(keystore), mosaic_core::kdf::calibrate)
                .with_adapters(Adapters {
                    import: ImportService::default(),
                    ssh: SshAgentService::for_this_os(&local_dir),
                })
                .with_attention({
                    let handle = app.handle().clone();
                    Arc::new(move || {
                        if let Some(window) = handle.get_webview_window("main") {
                            // Best effort; the window is never focused.
                            let _ = window.request_user_attention(Some(
                                tauri::UserAttentionType::Informational,
                            ));
                        }
                    })
                })
                .with_persistence(persistence)
                .with_clipboard(SystemClipboard::factory())
                .with_dialogs(Box::new(NativeDialogs::new(app.handle().clone())))
                .with_quick_unlock(QuickUnlock::for_this_os());
            let state = AppState::new(backend);

            let weak = state.downgrade();
            let sources = spawn_os_sources(
                &(Arc::new(move |event| {
                    if let Some(backend) = weak.upgrade() {
                        backend.on_os_event(event);
                    }
                }) as Arc<dyn Fn(autolock::OsEvent) + Send + Sync>),
            );
            if let Ok(mut slot) = guard(&state.backend().os_sources) {
                *slot = sources;
            }
            app.manage(MonitorHandle(Mutex::new(Monitor::spawn(
                state.downgrade(),
                TICK,
            ))));
            app.manage(state.clone());

            let status = shortcut::install(app.handle());
            if let Ok(mut slot) = guard(&state.backend().shortcut_status) {
                *slot = Some(status);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(true) = event
                && let Some(state) = window.app_handle().try_state::<AppState>()
            {
                state.backend().touch();
            }
        })
        .invoke_handler(commands::handler())
        .build(tauri::generate_context!())?;
    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            if let Some(monitor) = handle.try_state::<MonitorHandle>()
                && let Ok(mut monitor) = monitor.0.lock()
            {
                monitor.stop();
            }
            if let Some(state) = handle.try_state::<AppState>() {
                // Drop keys and decrypted items, and clear what Mosaic put on
                // the clipboard, before the process ends.
                state.backend().adapters.ssh.shutdown();
                let _ = state.backend().lock(LockReason::Manual);
                state.backend().clipboard.shutdown();
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod handler_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
