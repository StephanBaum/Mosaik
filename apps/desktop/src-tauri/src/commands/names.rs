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
    // Clipboard
    "copy_to_clipboard",
    "clear_clipboard",
    // Generator and one-time codes
    "generate_password",
    "totp_code",
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
    // SSH agent
    "ssh_agent_status",
    "ssh_agent_set_enabled",
    "ssh_respond",
];
