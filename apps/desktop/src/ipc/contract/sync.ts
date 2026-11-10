/** Sync contract: mirrors `src-tauri/src/contract/sync.rs` (docs/DESKTOP_IPC.md §3 "Sync"). */
import type { AccountId, UnixMillis } from "./common.ts";
import type { SyncState } from "./events.ts";
import type { SharingFindingView } from "./sharing.ts";

/** Who may create an account on a server. */
export const registrationModes = ["open", "invite", "closed"] as const;
export type RegistrationMode = (typeof registrationModes)[number];

/** What `sync_hosted_connect` does on the server. */
export const hostedModes = ["register", "signIn"] as const;
export type HostedMode = (typeof hostedModes)[number];

/** Where a browser handoff stands (M9). */
export const handoffStates = ["waiting", "done", "failed", "expired"] as const;
export type HandoffState = (typeof handoffStates)[number];

/** Which browser step an account handoff is at (M9). */
export const handoffSteps = ["login", "register", "confirm"] as const;
export type HandoffStep = (typeof handoffSteps)[number];

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
      /** How many security keys the sync account has (M9). */
      webauthnKeys: number;
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
  | { kind: "attachmentsNotRepaired"; count: number }
  | { kind: "rotationBlocked"; vaultName: string | null }
  /** A folder account file this device cannot verify; `conflict`: it names this device's own epoch (M7). */
  | { kind: "unverifiedAccountFile"; name: string; sha256: string; conflict: boolean };

export type SyncWarningKind = SyncWarningView["kind"];

/** The `sync_status` response (the spec's "SyncStatus"), and what most sync commands return. */
export interface SyncOverview {
  config: SyncConfigView | null;
  state: SyncState;
  warnings: SyncWarningView[];
  /** What sharing wants the user to look at; `sync_acknowledge` clears the findings. */
  sharingFindings: SharingFindingView[];
  /** The server (or the folder) holds a master password this device does not know. */
  passwordChanged: boolean;
  /** A verified re-key marker says the keys were renewed on another device; sync is paused (M7). */
  rekeyPending: boolean;
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

/** `sync_login_methods`: the second factors the waiting login can use. */
export interface LoginMethods {
  totp: boolean;
  webauthn: boolean;
}

/** `sync_hosted_webauthn_begin` and `join_hosted_webauthn_begin`. The code is typed on the server's page. */
export interface WebauthnBegin {
  matchCode: string;
  expiresAt: UnixMillis;
}

export interface WebauthnStatus {
  state: HandoffState;
}

/** `sync_webauthn_add_begin` and `sync_webauthn_confirm_begin`. */
export interface WebauthnAccountBegin {
  step: HandoffStep;
  matchCode: string;
  expiresAt: UnixMillis;
}

export interface WebauthnAccountStatus {
  state: HandoffState;
  step: HandoffStep;
  matchCode: string | null;
  expiresAt: UnixMillis | null;
}

export interface WebauthnKeyView {
  /** 32 hex digits. */
  credentialRef: string;
  label: string;
  createdAt: UnixMillis;
  lastUsedAt: UnixMillis | null;
}

export interface WebauthnKeys {
  /** The account also has an authenticator-app second factor. */
  totp: boolean;
  keys: WebauthnKeyView[];
}

export interface WebauthnAddRequest {
  password: string;
  code: string | null;
  label: string;
}

export interface WebauthnConfirmRequest {
  password: string;
}

export interface WebauthnRemoveRequest {
  credentialRef: string;
  code: string | null;
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
  /** The Secret Key from the newest kit when the keys were renewed elsewhere (M7); `null`: this device's own. */
  secretKey: string | null;
  code: string | null;
}

/** `sync_acknowledge`: no fields; Rust shows the native confirmation with its own texts. */
export type AcknowledgeRequest = Record<string, never>;

export interface ChangePasswordRequest {
  currentPassword: string;
  newPassword: string;
  code: string | null;
}

/** The security-key commands of the sync account (M9); `Commands` in `commands.ts` extends it. */
export interface SyncWebauthnCommands {
  sync_login_methods: { request: null; response: LoginMethods };
  sync_hosted_webauthn_begin: { request: null; response: WebauthnBegin };
  join_hosted_webauthn_begin: { request: null; response: WebauthnBegin };
  sync_webauthn_status: { request: null; response: WebauthnStatus };
  sync_webauthn_reopen: { request: null; response: null };
  sync_webauthn_list: { request: null; response: WebauthnKeys };
  sync_webauthn_add_begin: { request: WebauthnAddRequest; response: WebauthnAccountBegin };
  sync_webauthn_add_status: { request: null; response: WebauthnAccountStatus };
  sync_webauthn_confirm_begin: { request: WebauthnConfirmRequest; response: WebauthnAccountBegin };
  sync_webauthn_remove: { request: WebauthnRemoveRequest; response: null };
}

export type SyncWebauthnCommandName = keyof SyncWebauthnCommands;

/** The names of [`SyncWebauthnCommands`], for the command list. */
export const syncWebauthnCommandNames: readonly SyncWebauthnCommandName[] = [
  "sync_login_methods",
  "sync_hosted_webauthn_begin",
  "join_hosted_webauthn_begin",
  "sync_webauthn_status",
  "sync_webauthn_reopen",
  "sync_webauthn_list",
  "sync_webauthn_add_begin",
  "sync_webauthn_add_status",
  "sync_webauthn_confirm_begin",
  "sync_webauthn_remove",
];
