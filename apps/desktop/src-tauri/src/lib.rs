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
//! - [`quick_unlock`]: the platform half of quick unlock (Windows Hello,
//!   Touch ID, security keys behind a trait);
//! - [`adapters`]: the import and SSH agent services;
//! - [`browser`]: the endpoint for the browser extension's native messaging
//!   host (M4, ADR-0035 to ADR-0037).
//!
//! CRYPTO_SPEC §11 floor: the auto-lock triggers (idle, sleep, screen lock)
//! cannot all be disabled. `update_settings` refuses such a request with
//! `invalidInput`, and a state file that says so is read as the default idle
//! timeout ([`persist::DEFAULT_AUTO_LOCK_MINUTES`]).
//!
#![forbid(unsafe_code)]

pub mod adapters;
pub mod attachment_files;
pub mod autolock;
pub mod browser;
pub mod clipboard;
pub mod commands;
pub mod contract;
pub mod dialogs;
pub mod emergency_kit;
pub mod events;
pub mod keystore;
pub mod navigation;
pub mod otp_pending;
pub mod passkey_export;
pub mod persist;
pub mod quick_unlock;
pub mod screen_capture;
pub mod settings_integrity;
pub mod shortcut;
pub mod state;
pub mod sync;
pub mod unlock_throttle;

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
use crate::quick_unlock::QuickUnlockService;
use crate::screen_capture::NativeScreenCapture;
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

/// The app context. Generated in one place: on macOS `generate_context!`
/// embeds the Info.plist under a fixed symbol, so a second expansion in the
/// same crate (the tests) fails to link.
pub(crate) fn context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

/// Runs the app until the last window closes.
///
/// # Errors
/// Tauri's error if the app cannot start.
#[expect(
    clippy::too_many_lines,
    reason = "the composition root lists every service of the app once"
)]
pub fn run() -> tauri::Result<()> {
    let app = tauri::Builder::default()
        // First, so a second launch hands over before anything else starts
        // (one store session per install, CORE_API).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            shortcut::focus_main(app);
        }))
        // No webview leaves the bundled frontend (the CSP does not cover
        // top-level navigations).
        .plugin(navigation::guard())
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
            // The Secret Key of a rotation in progress: its own entries, so a
            // crash between the kit and the commit cannot touch the one in use.
            let pending = LayeredKeystore::new(Box::new(OsKeystore::new(&format!(
                "{}.pending-secret-key",
                app.config().identifier
            ))));
            let local_dir = app.path().app_local_data_dir()?;
            let backend = Backend::new(slot, Box::new(keystore), mosaic_core::kdf::calibrate)
                .with_pending_secret_keys(Box::new(pending))
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
                .with_sync_storage(&app.config().identifier, dir.clone())
                .with_clipboard(SystemClipboard::factory())
                .with_dialogs(Box::new(NativeDialogs::new(app.handle().clone())))
                .with_screen_capture(Box::new(NativeScreenCapture::new(app.handle().clone())))
                .with_focus({
                    let handle = app.handle().clone();
                    Arc::new(move || shortcut::focus_main(&handle))
                })
                .with_quick_unlock(QuickUnlockService::for_this_os());
            // Removes an expired or orphaned quick-unlock enrollment and its
            // platform key before the first unlock screen.
            backend.reconcile_quick_unlock();
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
            // `weekly` breach check: 60 seconds after an unlock, if it is due.
            // With the setting `off` (the default) nothing is ever started.
            let weak = state.downgrade();
            state.backend().on_unlock(Box::new(move || {
                Backend::breach_weekly_after_unlock(weak.clone());
            }));
            // After the state exists: the endpoint reaches the vault through it.
            state.backend().browser.start(&dir, state.downgrade());

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
        .build(context())?;
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
                // Closes the sessions and, on Windows, deletes `endpoint.json`.
                state.backend().browser.stop();
                let _ = state.backend().lock(LockReason::Manual);
                state.backend().clipboard.shutdown();
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod browser_tests;
#[cfg(test)]
mod credential_rotation_tests;
#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod handler_tests;
#[cfg(test)]
mod otp_tests;
#[cfg(test)]
mod quick_unlock_tests;
#[cfg(test)]
mod redteam_surface_tests;
#[cfg(test)]
mod redteam_tests;
#[cfg(test)]
mod rotation_handler_tests;
#[cfg(test)]
mod settings_integrity_tests;
#[cfg(test)]
mod sharing_tests;
#[cfg(test)]
mod sync_handler_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
