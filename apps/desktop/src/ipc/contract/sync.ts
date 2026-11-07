/** Sync contract: mirrors `src-tauri/src/contract/sync.rs` (docs/DESKTOP_IPC.md §3 "Sync"). */
import type { AccountId, UnixMillis } from "./common.ts";
import type { SyncState } from "./events.ts";

/** Who may create an account on a server. */
export const registrationModes = ["open", "invite", "closed"] as const;
export type RegistrationMode = (typeof registrationModes)[number];

/** What `sync_hosted_connect` does on the server. */
export const hostedModes = ["register", "signIn"] as const;
export type HostedMode = (typeof hostedModes)[number];

/** What a chosen folder holds. */
export const folderStates = ["empty", "mosaic", "other"] as const;
export type FolderState = (typeof folderStates)[number];

export const devicePlatforms = ["windows", "macos", "linux"] as const;
export type DevicePlatform = (typeof devicePlatforms)[number];

/** The configured backend, without secrets. */
export type SyncConfigView =
  | {
      kind: "hosted";
      serverUrl: string;
      email: string;
      deviceName: string;
      totpEnabled: boolean;
    }
  | { kind: "folder"; displayPath: string; deviceName: string };

/** Something the user should know about; the UI text says what to do. */
export type SyncWarningView =
  | { kind: "freezeSuspected"; deviceName: string | null; since: UnixMillis }
  | { kind: "missingFiles"; deviceName: string | null; since: UnixMillis }
  | { kind: "noCommonView"; deviceName: string | null; since: UnixMillis }
  | { kind: "rollbackDetected" }
  | { kind: "identityDuplicated" }
  | { kind: "serverInconsistent"; vaultName: string | null }
  | { kind: "serverRollback"; vaultName: string | null }
  | { kind: "undecryptable"; count: number }
  | { kind: "hardwareVaultsLocal"; count: number }
  | { kind: "attachmentsNotRepaired"; count: number };

export type SyncWarningKind = SyncWarningView["kind"];

/** The `sync_status` response (the spec's "SyncStatus"), and what most sync commands return. */
export interface SyncOverview {
  config: SyncConfigView | null;
  state: SyncState;
  warnings: SyncWarningView[];
  /** The server (or the folder) holds a master password this device does not know. */
  passwordChanged: boolean;
}

export interface CheckServerRequest {
  serverUrl: string;
}

export interface ServerCheck {
  serverUrl: string;
  registration: RegistrationMode;
}

export interface HostedConnectRequest {
  serverUrl: string;
  mode: HostedMode;
  password: string;
  inviteCode: string | null;
  deviceName: string;
}

/** A request that carries one code (`sync_hosted_two_factor`, `sync_totp_confirm`, ...). */
export interface CodeRequest {
  code: string;
}

/** The `sync_pick_folder` response (`null` when the user cancels). */
export interface FolderChoice {
  choiceId: string;
  displayPath: string;
  contents: FolderState;
  accounts: AccountId[];
}

export interface FolderConnectRequest {
  choiceId: string;
  deviceName: string;
}

export interface SyncDevice {
  deviceId: string;
  name: string | null;
  platform: DevicePlatform | null;
  current: boolean;
  lastSeenAt: UnixMillis | null;
  canRemove: boolean;
}

export interface SyncDevices {
  devices: SyncDevice[];
}

export interface DeviceRenameRequest {
  deviceId: string;
  name: string;
}

export interface DeviceRemoveRequest {
  deviceId: string;
  password: string;
  code: string | null;
}

export interface TotpBeginRequest {
  password: string;
}

export interface TotpSetupInfo {
  secret: string;
  otpauthUri: string;
}

export interface TotpDisableRequest {
  password: string;
  code: string;
}

export interface JoinHostedRequest {
  serverUrl: string;
  email: string;
  password: string;
  secretKey: string;
  deviceName: string;
}

export interface JoinFolderRequest {
  choiceId: string;
  accountId: AccountId;
  password: string;
  secretKey: string;
  deviceName: string;
}

export type JoinOutcome = { state: "unlocked" } | { state: "twoFactorRequired" };

/** `sync_recover_password`: the master password the server or folder holds now. */
export interface RecoverPasswordRequest {
  newPassword: string;
  code: string | null;
}

/** `sync_acknowledge`: no fields; Rust shows the native confirmation with its own texts. */
export type AcknowledgeRequest = Record<string, never>;

export interface ChangePasswordRequest {
  currentPassword: string;
  newPassword: string;
  code: string | null;
}
