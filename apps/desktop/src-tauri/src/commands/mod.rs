//! The IPC commands, one module per area of `docs/DESKTOP_IPC.md`. Each
//! command is a thin async wrapper that runs a `Backend` method on the
//! blocking pool; the methods hold the logic and are testable without
//! Tauri.
//!
//! The wrappers' errors are the `Backend` methods' (each has an `# Errors`
//! section) and are listed per command in `docs/DESKTOP_IPC.md`.
#![allow(clippy::missing_errors_doc)]

pub mod clipboard;
pub mod draft;
pub mod generator;
pub mod import;
pub mod insights;
pub mod items;
/// The command list, shared with `build.rs` (which `include!`s it).
pub mod names;
pub mod session;
pub mod settings;
pub mod ssh;
pub mod totp;
pub mod vaults;

/// The invoke handler with every command of [`names::COMMANDS`], in the
/// same order. `tests::every_command_is_registered` checks the two agree.
pub fn handler<R: tauri::Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static
{
    tauri::generate_handler![
        session::session_state,
        session::create_account,
        session::save_emergency_kit,
        session::verify_emergency_kit,
        session::unlock,
        session::unlock_quick,
        session::lock,
        session::subscribe_events,
        session::app_status,
        vaults::list_vaults,
        vaults::create_vault,
        vaults::rename_vault,
        items::list_items,
        items::get_item,
        items::reveal_secret,
        items::add_item,
        items::update_item,
        items::set_favorite,
        items::trash_item,
        items::restore_item,
        items::purge_item,
        items::move_item,
        clipboard::copy_to_clipboard,
        clipboard::clear_clipboard,
        generator::generate_password,
        totp::totp_code,
        settings::get_settings,
        settings::update_settings,
        insights::security_report,
        import::import_choose_file,
        import::import_preview,
        import::import_commit,
        import::import_cancel,
        ssh::ssh_agent_status,
        ssh::ssh_agent_set_enabled,
        ssh::ssh_respond,
    ]
}
