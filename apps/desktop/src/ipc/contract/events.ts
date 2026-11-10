/** Events contract: mirrors `src-tauri/src/contract/events.rs`. */
import type {
  BrowserCloseReason,
  BrowserPairingRequest,
  BrowserPasskeyRequest,
  BrowserRotationRequest,
  BrowserSaveRequest,
} from "./browser.ts";
import type { ErrorCode, ItemId, UnixMillis, VaultId } from "./common.ts";
import type { BreachScanError } from "./credentialRotation.ts";
import type { RotationKind, RotationPhaseName } from "./rotation.ts";
import type { SshApprovalRequest, SshCloseReason, SshSignFailReason } from "./ssh.ts";

export const lockReasons = ["manual", "idle", "sleep", "screenLock"] as const;
export type LockReason = (typeof lockReasons)[number];

export const syncStatuses = [
  "notConfigured",
  "offline",
  "syncing",
  "upToDate",
  "error",
  "signedOut",
] as const;
export type SyncStatus = (typeof syncStatuses)[number];

export interface SyncState {
  status: SyncStatus;
  lastSyncedAt: UnixMillis | null;
  error: ErrorCode | null;
}

export type AppEvent =
  | { type: "locked"; reason: LockReason }
  | { type: "syncStateChanged"; state: SyncState }
  | { type: "rotationProgress"; kind: RotationKind; phase: RotationPhaseName; done: number; total: number }
  | { type: "clipboardCleared" }
  | { type: "sshApprovalRequested"; request: SshApprovalRequest }
  | { type: "sshApprovalClosed"; requestId: string; reason: SshCloseReason }
  | {
      type: "sshSignFailed";
      requestId: string;
      vaultId: VaultId;
      itemId: ItemId;
      reason: SshSignFailReason;
    }
  | { type: "quickSearchRequested" }
  /** A hardware vault closed itself (timer or lock): drop revealed codes. */
  | { type: "hardwareVaultClosed"; vaultId: VaultId }
  /** Protection settings in the settings file failed their integrity check and were reset. */
  | { type: "settingsReset" }
  /** A browser asks to be paired: show the code and wait for the user. */
  | { type: "browserPairingRequested"; request: BrowserPairingRequest }
  | { type: "browserPairingClosed"; requestId: string; reason: BrowserCloseReason }
  /** The extension offers to save a login (the password stays in Rust). */
  | { type: "browserSaveRequested"; request: BrowserSaveRequest }
  | { type: "browserSaveClosed"; requestId: string; reason: BrowserCloseReason }
  /** The extension asks to create or use a passkey; the desktop dialog decides. */
  | { type: "browserPasskeyRequested"; request: BrowserPasskeyRequest }
  | { type: "browserPasskeyClosed"; requestId: string; reason: BrowserCloseReason }
  /** The extension starts a password change, reports its result or asks to add an address. */
  | { type: "browserRotationRequested"; request: BrowserRotationRequest }
  | { type: "browserRotationClosed"; requestId: string; reason: BrowserCloseReason }
  /** A browser was paired, removed, connected or disconnected. */
  | { type: "browserPairingsChanged" }
  /** A sync run changed these vaults: read their items again. */
  | { type: "syncDataChanged"; vaultIds: VaultId[] }
  /** The invites or emergency requests waiting for this account changed. */
  | { type: "sharingChanged"; invites: number; emergencyRequests: number }
  /** A breach scan answered another prefix. */
  | { type: "breachScanProgress"; done: number; total: number }
  /** A breach scan ended (also when cancelled or the vault locked). */
  | { type: "breachScanFinished"; checked: number; found: number; error: BreachScanError | null }
  /** A sync run merged or repaired something the user may want to look at. */
  | { type: "syncAttention"; conflicts: number; restored: number; replacedUnreadable: number };
