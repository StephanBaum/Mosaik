/**
 * Every IPC command with its request and response type (docs/DESKTOP_IPC.md). Mirrors
 * `src-tauri/src/commands/names.rs`. `subscribe_events` is not here: its argument is a
 * Channel, and `MosaicIpc.subscribe` wraps it.
 */
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
  KitVerification,
  QuickUnlockRequest,
  SessionState,
  UnlockRequest,
  VerifyKitRequest,
} from "./session.ts";
import type { Settings, UpdateSettingsRequest } from "./settings.ts";
import type { SshAgentStatus, SshRespondRequest, SshSetEnabledRequest } from "./ssh.ts";
import type { TotpCode } from "./totp.ts";
import type { CreateVaultRequest, RenameVaultRequest, VaultSummary } from "./vaults.ts";

interface Command<Req, Res> {
  request: Req;
  response: Res;
}

export interface Commands {
  // Session
  session_state: Command<null, SessionState>;
  create_account: Command<CreateAccountRequest, CreatedAccount>;
  verify_emergency_kit: Command<VerifyKitRequest, KitVerification>;
  unlock: Command<UnlockRequest, SessionState>;
  unlock_quick: Command<QuickUnlockRequest, SessionState>;
  lock: Command<null, SessionState>;
  // Vaults
  list_vaults: Command<null, VaultSummary[]>;
  create_vault: Command<CreateVaultRequest, VaultSummary>;
  rename_vault: Command<RenameVaultRequest, VaultSummary>;
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
  // Clipboard
  copy_to_clipboard: Command<CopyRequest, CopyReceipt>;
  clear_clipboard: Command<null, null>;
  // Generator and one-time codes
  generate_password: Command<GenerateRequest, Generated>;
  totp_code: Command<ItemRequest, TotpCode>;
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
}

export type CommandName = keyof Commands;
export type RequestOf<K extends CommandName> = Commands[K]["request"];
export type ResponseOf<K extends CommandName> = Commands[K]["response"];

/** Commands whose argument is a Channel rather than a request. */
export const channelCommands = ["subscribe_events"] as const;

/** Every command at runtime; `satisfies` keeps it equal to `Commands`. */
const commandSet = {
  session_state: true,
  create_account: true,
  verify_emergency_kit: true,
  unlock: true,
  unlock_quick: true,
  lock: true,
  list_vaults: true,
  create_vault: true,
  rename_vault: true,
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
  copy_to_clipboard: true,
  clear_clipboard: true,
  generate_password: true,
  totp_code: true,
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
} as const satisfies Record<CommandName, true>;

export const commandNames = Object.keys(commandSet) as CommandName[];
