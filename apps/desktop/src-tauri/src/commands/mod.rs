//! The IPC commands, one module per area of `docs/DESKTOP_IPC.md`. Each
//! command is a thin async wrapper that runs a `Backend` method on the
//! blocking pool; the methods hold the logic and are testable without
//! Tauri.
//!
//! The wrappers' errors are the `Backend` methods' (each has an `# Errors`
//! section) and are listed per command in `docs/DESKTOP_IPC.md`.
#![expect(
    clippy::missing_errors_doc,
    reason = "errors are documented on the Backend methods and in DESKTOP_IPC.md"
)]

pub mod attachments;
pub mod browser;
pub mod clipboard;
pub mod draft;
pub mod export;
pub mod generator;
pub mod import;
pub mod insights;
pub mod items;
/// The command list, shared with `build.rs` (which `include!`s it).
pub mod names;
pub mod otp;
pub mod quick_unlock;
pub mod rotation;
pub mod session;
pub mod settings;
pub mod sharing;
pub mod ssh;
pub mod sync;
pub mod totp;
pub mod vaults;

/// Whether `c` can make a name look like another one: the Unicode format
/// characters (Cf) and default-ignorable code points, which hide or reorder
/// text (`mosaic_core::is_spoofing_char`). Names and titles that contain one
/// are refused (`invalidInput`).
#[must_use]
pub(crate) const fn is_spoofing_char(c: char) -> bool {
    mosaic_core::is_spoofing_char(c)
}

/// The invoke handler with every command of [`names::COMMANDS`], in the
/// same order. `tests::every_command_is_registered` checks the two agree.
#[expect(
    clippy::too_many_lines,
    reason = "one line per command, in the order of docs/DESKTOP_IPC.md"
)]
pub fn handler<R: tauri::Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static
{
    tauri::generate_handler![
        session::session_state,
        session::create_account,
        session::save_emergency_kit,
        session::verify_emergency_kit,
        session::unlock,
        session::lock,
        session::reauthenticate,
        session::subscribe_events,
        session::app_status,
        quick_unlock::unlock_quick,
        quick_unlock::quick_unlock_status,
        quick_unlock::enroll_biometric,
        quick_unlock::remove_biometric,
        quick_unlock::enroll_hardware_key,
        quick_unlock::remove_hardware_key,
        quick_unlock::hardware_key_cancel,
        vaults::list_vaults,
        vaults::create_vault,
        vaults::rename_vault,
        vaults::create_hardware_vault,
        vaults::open_hardware_vault,
        vaults::close_hardware_vault,
        vaults::hardware_vault_keys,
        vaults::add_hardware_vault_key,
        vaults::remove_hardware_vault_key,
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
        attachments::attachment_add,
        attachments::attachment_preview,
        attachments::attachment_save,
        attachments::attachment_remove,
        clipboard::copy_to_clipboard,
        clipboard::clear_clipboard,
        generator::generate_password,
        totp::totp_code,
        otp::otp_scan,
        otp::otp_parse,
        otp::otp_pending_code,
        otp::otp_pending_discard,
        settings::get_settings,
        settings::update_settings,
        insights::security_report,
        import::import_choose_file,
        import::import_preview,
        import::import_commit,
        import::import_cancel,
        export::export_passkeys_cxf,
        ssh::ssh_agent_status,
        ssh::ssh_agent_set_enabled,
        ssh::ssh_respond,
        browser::browser_status,
        browser::browser_remove_pairing,
        browser::browser_pairing_respond,
        browser::browser_save_respond,
        browser::browser_passkey_respond,
        sync::sync_status,
        sync::sync_check_server,
        sync::sync_hosted_connect,
        sync::sync_hosted_two_factor,
        sync::sync_pick_folder,
        sync::sync_folder_connect,
        sync::sync_now,
        sync::sync_disconnect,
        sync::sync_acknowledge,
        sync::sync_recover_password,
        sync::sync_devices,
        sync::sync_device_rename,
        sync::sync_device_remove,
        sync::sync_totp_begin,
        sync::sync_totp_confirm,
        sync::sync_totp_disable,
        sync::join_hosted,
        sync::join_hosted_two_factor,
        sync::join_folder,
        sync::change_master_password,
        rotation::sync_dismiss_account_file,
        rotation::rotation_status,
        rotation::secret_key_rotate_prepare,
        rotation::secret_key_rotate_commit,
        rotation::profile_upgrade,
        rotation::rekey_review,
        rotation::rekey_prepare,
        rotation::rekey_commit,
        rotation::rotation_resume,
        rotation::rotation_abandon,
        sharing::sharing_identity,
        sharing::sharing_lookup,
        sharing::sharing_verify_contact,
        sharing::sharing_accept_key_change,
        sharing::sharing_contacts,
        sharing::vault_share,
        sharing::vault_members,
        sharing::vault_member_set_role,
        sharing::vault_member_remove,
        sharing::vault_invite_cancel,
        sharing::vault_leave,
        sharing::invites_list,
        sharing::invite_preview,
        sharing::invite_accept,
        sharing::invite_decline,
        sharing::emergency_overview,
        sharing::emergency_grant,
        sharing::emergency_revoke,
        sharing::emergency_request,
        sharing::emergency_respond,
        sharing::emergency_claim,
    ]
}
