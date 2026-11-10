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
  BrowserRotationCommands,
  BrowserSaveRespondRequest,
  BrowserStatus,
} from "./browser.ts";
import type { CopyReceipt, CopyRequest } from "./clipboard.ts";
import type { CredentialRotationCommands } from "./credentialRotation.ts";
import type { ExportPasskeysRequest, PasskeysExported } from "./export.ts";
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
import type {
  DismissFileRequest,
  ProfileUpgradeRequest,
  RekeyCommitRequest,
  RekeyMarkerView,
  RekeyPrepareRequest,
  RekeyReviewView,
  RotateCommitRequest,
  RotatePrepareRequest,
  RotationDone,
  RotationResumeRequest,
  RotationStatus,
} from "./rotation.ts";
import type { Settings, UpdateSettingsRequest } from "./settings.ts";
import type {
  ContactCodeRequest,
  ContactList,
  EmergencyClaimRequest,
  EmergencyClaimed,
  EmergencyGrantRequest,
  EmergencyOverview,
  EmergencyRequestRequest,
  EmergencyRequested,
  EmergencyRespondRequest,
  EmergencyRevokeRequest,
  InviteAcceptRequest,
  InviteDeclineRequest,
  InviteList,
  InvitePreviewView,
  InviteRequest,
  InviteSent,
  LookupRequest,
  LookupResult,
  MemberRequest,
  RemoveOutcome,
  SetRoleRequest,
  ShareVaultRequest,
  SharingIdentity,
  VaultMembersView,
} from "./sharing.ts";
import type { SshAgentStatus, SshRespondRequest, SshSetEnabledRequest } from "./ssh.ts";
import type {
  AcknowledgeRequest,
  ChangePasswordRequest,
  CheckServerRequest,
  CodeRequest,
  DeviceRemoveRequest,
  DeviceRenameRequest,
  FolderChoice,
  FolderConnectRequest,
  HostedConnectRequest,
  JoinFolderRequest,
  JoinHostedRequest,
  JoinOutcome,
  RecoverPasswordRequest,
  ServerCheck,
  SyncDevices,
  SyncOverview,
  TotpBeginRequest,
  TotpDisableRequest,
  TotpSetupInfo,
} from "./sync.ts";
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

export interface Commands extends CredentialRotationCommands, BrowserRotationCommands {
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
  // Export
  export_passkeys_cxf: Command<ExportPasskeysRequest, PasskeysExported>;
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
  // Sync
  sync_status: Command<null, SyncOverview>;
  sync_check_server: Command<CheckServerRequest, ServerCheck>;
  sync_hosted_connect: Command<HostedConnectRequest, SyncOverview>;
  sync_hosted_two_factor: Command<CodeRequest, SyncOverview>;
  sync_pick_folder: Command<null, FolderChoice | null>;
  sync_folder_connect: Command<FolderConnectRequest, SyncOverview>;
  sync_now: Command<null, SyncOverview>;
  sync_disconnect: Command<null, SyncOverview>;
  sync_acknowledge: Command<AcknowledgeRequest, SyncOverview>;
  sync_recover_password: Command<RecoverPasswordRequest, SyncOverview>;
  sync_devices: Command<null, SyncDevices>;
  sync_device_rename: Command<DeviceRenameRequest, null>;
  sync_device_remove: Command<DeviceRemoveRequest, null>;
  sync_totp_begin: Command<TotpBeginRequest, TotpSetupInfo>;
  sync_totp_confirm: Command<CodeRequest, null>;
  sync_totp_disable: Command<TotpDisableRequest, null>;
  join_hosted: Command<JoinHostedRequest, JoinOutcome>;
  join_hosted_two_factor: Command<CodeRequest, JoinOutcome>;
  join_folder: Command<JoinFolderRequest, JoinOutcome>;
  change_master_password: Command<ChangePasswordRequest, null>;
  sync_dismiss_account_file: Command<DismissFileRequest, SyncOverview>;
  // Account keys
  rotation_status: Command<null, RotationStatus>;
  secret_key_rotate_prepare: Command<RotatePrepareRequest, null>;
  secret_key_rotate_commit: Command<RotateCommitRequest, RotationDone>;
  profile_upgrade: Command<ProfileUpgradeRequest, RotationDone>;
  rekey_review: Command<null, RekeyReviewView>;
  rekey_prepare: Command<RekeyPrepareRequest, null>;
  rekey_commit: Command<RekeyCommitRequest, RotationDone>;
  rotation_resume: Command<RotationResumeRequest, RotationDone>;
  rotation_abandon: Command<null, null>;
  rotation_abandon_committed: Command<null, null>;
  sync_rekey_markers: Command<null, RekeyMarkerView[]>;
  sync_dismiss_rekey_marker: Command<DismissFileRequest, SyncOverview>;
  sync_ignored_rekey_markers: Command<null, RekeyMarkerView[]>;
  sync_restore_rekey_marker: Command<DismissFileRequest, SyncOverview>;
  // Sharing and emergency access
  sharing_identity: Command<null, SharingIdentity>;
  sharing_lookup: Command<LookupRequest, LookupResult>;
  sharing_verify_contact: Command<ContactCodeRequest, null>;
  sharing_accept_key_change: Command<ContactCodeRequest, null>;
  sharing_contacts: Command<null, ContactList>;
  vault_share: Command<ShareVaultRequest, InviteSent>;
  vault_members: Command<VaultRequest, VaultMembersView>;
  vault_member_set_role: Command<SetRoleRequest, null>;
  vault_member_remove: Command<MemberRequest, RemoveOutcome>;
  vault_invite_cancel: Command<InviteRequest, null>;
  vault_leave: Command<VaultRequest, null>;
  invites_list: Command<null, InviteList>;
  invite_preview: Command<InviteRequest, InvitePreviewView>;
  invite_accept: Command<InviteAcceptRequest, null>;
  invite_decline: Command<InviteDeclineRequest, null>;
  emergency_overview: Command<null, EmergencyOverview>;
  emergency_grant: Command<EmergencyGrantRequest, null>;
  emergency_revoke: Command<EmergencyRevokeRequest, null>;
  emergency_request: Command<EmergencyRequestRequest, EmergencyRequested>;
  emergency_respond: Command<EmergencyRespondRequest, null>;
  emergency_claim: Command<EmergencyClaimRequest, EmergencyClaimed>;
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
  export_passkeys_cxf: true,
  ssh_agent_status: true,
  ssh_agent_set_enabled: true,
  ssh_respond: true,
  browser_status: true,
  browser_remove_pairing: true,
  browser_pairing_respond: true,
  browser_save_respond: true,
  browser_passkey_respond: true,
  browser_rotation_respond: true,
  browser_rotation_reveal: true,
  sync_status: true,
  sync_check_server: true,
  sync_hosted_connect: true,
  sync_hosted_two_factor: true,
  sync_pick_folder: true,
  sync_folder_connect: true,
  sync_now: true,
  sync_disconnect: true,
  sync_acknowledge: true,
  sync_recover_password: true,
  sync_devices: true,
  sync_device_rename: true,
  sync_device_remove: true,
  sync_totp_begin: true,
  sync_totp_confirm: true,
  sync_totp_disable: true,
  join_hosted: true,
  join_hosted_two_factor: true,
  join_folder: true,
  change_master_password: true,
  sync_dismiss_account_file: true,
  rotation_status: true,
  secret_key_rotate_prepare: true,
  secret_key_rotate_commit: true,
  profile_upgrade: true,
  rekey_review: true,
  rekey_prepare: true,
  rekey_commit: true,
  rotation_resume: true,
  rotation_abandon: true,
  rotation_queue: true,
  rotation_begin: true,
  rotation_close: true,
  rotation_reveal: true,
  rotation_copy: true,
  password_history_restore: true,
  rotation_snooze: true,
  rotation_dismiss: true,
  rotation_set_policy: true,
  breach_scan_start: true,
  breach_scan_cancel: true,
  rotation_abandon_committed: true,
  sync_rekey_markers: true,
  sync_dismiss_rekey_marker: true,
  sync_ignored_rekey_markers: true,
  sync_restore_rekey_marker: true,
  sharing_identity: true,
  sharing_lookup: true,
  sharing_verify_contact: true,
  sharing_accept_key_change: true,
  sharing_contacts: true,
  vault_share: true,
  vault_members: true,
  vault_member_set_role: true,
  vault_member_remove: true,
  vault_invite_cancel: true,
  vault_leave: true,
  invites_list: true,
  invite_preview: true,
  invite_accept: true,
  invite_decline: true,
  emergency_overview: true,
  emergency_grant: true,
  emergency_revoke: true,
  emergency_request: true,
  emergency_respond: true,
  emergency_claim: true,
} as const satisfies Record<CommandName, true>;

export const commandNames = Object.keys(commandSet) as CommandName[];
