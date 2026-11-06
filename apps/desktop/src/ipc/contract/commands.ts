/**
 * Every IPC command with its request and response type (docs/DESKTOP_IPC.md). Mirrors
 * `src-tauri/src/commands/names.rs`. `subscribe_events` is not here: its argument is a
 * Channel, and `MosaicIpc.subscribe` wraps it.
 */
import type { AttachmentAdded, AttachmentRequest, AttachmentSaved } from "./attachments.ts";
import type {
  BrowserPairingRespondRequest,
  BrowserPasskeyRespondRequest,
  BrowserRemovePairingRequest,
  BrowserSaveRespondRequest,
  BrowserStatus,
} from "./browser.ts";
import type { CopyReceipt, CopyRequest } from "./clipboard.ts";
import type { Generated, GenerateRequest } from "./generator.ts";
import type {
  ImportCancelRequest,
  ImportChooseRequest,
  ImportCommitRequest,
  ImportFileChoice,
  ImportPreview,
  ImportPreviewRequest,
  ImportResult,
} from "./import.ts";
import type { SecurityReport } from "./insights.ts";
import type {
  OtpParseRequest,
  OtpPending,
  OtpPendingRequest,
  OtpScanRequest,
  OtpScanResult,
} from "./otp.ts";
import type {
  EnrollBiometricRequest,
  EnrollHardwareKeyRequest,
  HardwareVaultKeys,
  QuickUnlockStatus,
  RemoveHardwareKeyRequest,
} from "./quickUnlock.ts";
import type {
  AddItemRequest,
  ItemDetail,
  ItemRequest,
  ItemSummary,
  ListItemsRequest,
  MovedItem,
  MoveItemRequest,
  RevealedSecret,
  RevealRequest,
  SetFavoriteRequest,
  UpdateItemRequest,
} from "./items.ts";
import type {
  CreateAccountRequest,
  CreatedAccount,
  AppStatus,
  KitSaved,
  KitVerification,
  QuickUnlockRequest,
  ReauthenticateRequest,
  SaveKitRequest,
  SessionState,
  UnlockRequest,
  VerifyKitRequest,
} from "./session.ts";
import type { Settings, UpdateSettingsRequest } from "./settings.ts";
import type { SshAgentStatus, SshRespondRequest, SshSetEnabledRequest } from "./ssh.ts";
import type { TotpCode } from "./totp.ts";
import type {
  CreateHardwareVaultRequest,
  CreateVaultRequest,
  HardwareVaultKeyRequest,
  HardwareVaultRequest,
  RenameVaultRequest,
  VaultRequest,
  VaultSummary,
} from "./vaults.ts";

interface Command<Req, Res> {
  request: Req;
  response: Res;
}

export interface Commands {
  // Session
  session_state: Command<null, SessionState>;
  create_account: Command<CreateAccountRequest, CreatedAccount>;
  save_emergency_kit: Command<SaveKitRequest, KitSaved>;
  verify_emergency_kit: Command<VerifyKitRequest, KitVerification>;
  unlock: Command<UnlockRequest, SessionState>;
  unlock_quick: Command<QuickUnlockRequest, SessionState>;
  lock: Command<null, SessionState>;
  reauthenticate: Command<ReauthenticateRequest, null>;
  app_status: Command<null, AppStatus>;
  // Quick unlock
  quick_unlock_status: Command<null, QuickUnlockStatus>;
  enroll_biometric: Command<EnrollBiometricRequest, QuickUnlockStatus>;
  remove_biometric: Command<null, QuickUnlockStatus>;
  enroll_hardware_key: Command<EnrollHardwareKeyRequest, QuickUnlockStatus>;
  remove_hardware_key: Command<RemoveHardwareKeyRequest, QuickUnlockStatus>;
  hardware_key_cancel: Command<null, null>;
  // Vaults
  list_vaults: Command<null, VaultSummary[]>;
  create_vault: Command<CreateVaultRequest, VaultSummary>;
  rename_vault: Command<RenameVaultRequest, VaultSummary>;
  create_hardware_vault: Command<CreateHardwareVaultRequest, VaultSummary>;
  open_hardware_vault: Command<HardwareVaultRequest, VaultSummary>;
  close_hardware_vault: Command<VaultRequest, null>;
  hardware_vault_keys: Command<VaultRequest, HardwareVaultKeys>;
  add_hardware_vault_key: Command<HardwareVaultRequest, HardwareVaultKeys>;
  remove_hardware_vault_key: Command<HardwareVaultKeyRequest, HardwareVaultKeys>;
  // Items
  list_items: Command<ListItemsRequest, ItemSummary[]>;
  get_item: Command<ItemRequest, ItemDetail>;
  reveal_secret: Command<RevealRequest, RevealedSecret>;
  add_item: Command<AddItemRequest, ItemDetail>;
  update_item: Command<UpdateItemRequest, ItemDetail>;
  set_favorite: Command<SetFavoriteRequest, ItemSummary>;
  trash_item: Command<ItemRequest, null>;
  restore_item: Command<ItemRequest, null>;
  purge_item: Command<ItemRequest, null>;
  move_item: Command<MoveItemRequest, MovedItem>;
  // Attachments
  attachment_add: Command<ItemRequest, AttachmentAdded>;
  /** Binary response: `[width u32 LE][height u32 LE][width * height * 4 RGBA bytes]`. */
  attachment_preview: Command<AttachmentRequest, ArrayBuffer>;
  attachment_save: Command<AttachmentRequest, AttachmentSaved>;
  attachment_remove: Command<AttachmentRequest, ItemDetail>;
  // Clipboard
  copy_to_clipboard: Command<CopyRequest, CopyReceipt>;
  clear_clipboard: Command<null, null>;
  // Generator and one-time codes
  generate_password: Command<GenerateRequest, Generated>;
  totp_code: Command<ItemRequest, TotpCode>;
  // One-time code setup
  otp_scan: Command<OtpScanRequest, OtpScanResult>;
  otp_parse: Command<OtpParseRequest, OtpPending>;
  otp_pending_code: Command<OtpPendingRequest, TotpCode>;
  otp_pending_discard: Command<OtpPendingRequest, null>;
  // Settings
  get_settings: Command<null, Settings>;
  update_settings: Command<UpdateSettingsRequest, Settings>;
  // Security insights
  security_report: Command<null, SecurityReport>;
  // Import
  import_choose_file: Command<ImportChooseRequest, ImportFileChoice>;
  import_preview: Command<ImportPreviewRequest, ImportPreview>;
  import_commit: Command<ImportCommitRequest, ImportResult>;
  import_cancel: Command<ImportCancelRequest, null>;
  // SSH agent
  ssh_agent_status: Command<null, SshAgentStatus>;
  ssh_agent_set_enabled: Command<SshSetEnabledRequest, SshAgentStatus>;
  ssh_respond: Command<SshRespondRequest, null>;
  // Browser extension
  browser_status: Command<null, BrowserStatus>;
  browser_remove_pairing: Command<BrowserRemovePairingRequest, BrowserStatus>;
  browser_pairing_respond: Command<BrowserPairingRespondRequest, null>;
  browser_save_respond: Command<BrowserSaveRespondRequest, null>;
  browser_passkey_respond: Command<BrowserPasskeyRespondRequest, null>;
}

export type CommandName = keyof Commands;
export type RequestOf<K extends CommandName> = Commands[K]["request"];
export type ResponseOf<K extends CommandName> = Commands[K]["response"];

/**
 * Commands whose response is binary (an `ArrayBuffer`), so the Rust fixtures hold no JSON
 * response sample for them. `satisfies` keeps the list inside `Commands`.
 */
export const binaryCommands = ["attachment_preview"] as const satisfies readonly CommandName[];
export type BinaryCommandName = (typeof binaryCommands)[number];

/** Commands whose argument is a Channel rather than a request. */
export const channelCommands = ["subscribe_events"] as const;

/** Every command at runtime; `satisfies` keeps it equal to `Commands`. */
const commandSet = {
  session_state: true,
  create_account: true,
  save_emergency_kit: true,
  verify_emergency_kit: true,
  unlock: true,
  unlock_quick: true,
  lock: true,
  reauthenticate: true,
  app_status: true,
  quick_unlock_status: true,
  enroll_biometric: true,
  remove_biometric: true,
  enroll_hardware_key: true,
  remove_hardware_key: true,
  hardware_key_cancel: true,
  list_vaults: true,
  create_vault: true,
  rename_vault: true,
  create_hardware_vault: true,
  open_hardware_vault: true,
  close_hardware_vault: true,
  hardware_vault_keys: true,
  add_hardware_vault_key: true,
  remove_hardware_vault_key: true,
  list_items: true,
  get_item: true,
  reveal_secret: true,
  add_item: true,
  update_item: true,
  set_favorite: true,
  trash_item: true,
  restore_item: true,
  purge_item: true,
  move_item: true,
  attachment_add: true,
  attachment_preview: true,
  attachment_save: true,
  attachment_remove: true,
  copy_to_clipboard: true,
  clear_clipboard: true,
  generate_password: true,
  totp_code: true,
  otp_scan: true,
  otp_parse: true,
  otp_pending_code: true,
  otp_pending_discard: true,
  get_settings: true,
  update_settings: true,
  security_report: true,
  import_choose_file: true,
  import_preview: true,
  import_commit: true,
  import_cancel: true,
  ssh_agent_status: true,
  ssh_agent_set_enabled: true,
  ssh_respond: true,
  browser_status: true,
  browser_remove_pairing: true,
  browser_pairing_respond: true,
  browser_save_respond: true,
  browser_passkey_respond: true,
} as const satisfies Record<CommandName, true>;

export const commandNames = Object.keys(commandSet) as CommandName[];
