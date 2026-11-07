// Shared by the crate (`commands::names`) and `build.rs` (`include!`), so it
// holds nothing but the list. Keep it in the order of `docs/DESKTOP_IPC.md`.

/// Every IPC command, by its Tauri name. `build.rs` generates one
/// `allow-<command>` permission per entry, `capabilities/main.json` grants
/// exactly these, and the contract fixtures list them for the TypeScript
/// parity test.
pub const COMMANDS: &[&str] = &[
    // Session
    "session_state",
    "create_account",
    "save_emergency_kit",
    "verify_emergency_kit",
    "unlock",
    "lock",
    "reauthenticate",
    "subscribe_events",
    "app_status",
    // Quick unlock
    "unlock_quick",
    "quick_unlock_status",
    "enroll_biometric",
    "remove_biometric",
    "enroll_hardware_key",
    "remove_hardware_key",
    "hardware_key_cancel",
    // Vaults
    "list_vaults",
    "create_vault",
    "rename_vault",
    "create_hardware_vault",
    "open_hardware_vault",
    "close_hardware_vault",
    "hardware_vault_keys",
    "add_hardware_vault_key",
    "remove_hardware_vault_key",
    // Items
    "list_items",
    "get_item",
    "reveal_secret",
    "add_item",
    "update_item",
    "set_favorite",
    "trash_item",
    "restore_item",
    "purge_item",
    "move_item",
    // Attachments
    "attachment_add",
    "attachment_preview",
    "attachment_save",
    "attachment_remove",
    // Clipboard
    "copy_to_clipboard",
    "clear_clipboard",
    // Generator and one-time codes
    "generate_password",
    "totp_code",
    // One-time code setup
    "otp_scan",
    "otp_parse",
    "otp_pending_code",
    "otp_pending_discard",
    // Settings
    "get_settings",
    "update_settings",
    // Security insights
    "security_report",
    // Import
    "import_choose_file",
    "import_preview",
    "import_commit",
    "import_cancel",
    // Export
    "export_passkeys_cxf",
    // SSH agent
    "ssh_agent_status",
    "ssh_agent_set_enabled",
    "ssh_respond",
    // Browser extension
    "browser_status",
    "browser_remove_pairing",
    "browser_pairing_respond",
    "browser_save_respond",
    "browser_passkey_respond",
    // Sync
    "sync_status",
    "sync_check_server",
    "sync_hosted_connect",
    "sync_hosted_two_factor",
    "sync_pick_folder",
    "sync_folder_connect",
    "sync_now",
    "sync_disconnect",
    "sync_acknowledge",
    "sync_devices",
    "sync_device_rename",
    "sync_device_remove",
    "sync_totp_begin",
    "sync_totp_confirm",
    "sync_totp_disable",
    "join_hosted",
    "join_hosted_two_factor",
    "join_folder",
    "change_master_password",
];
