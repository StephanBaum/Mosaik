/** Events contract: mirrors `src-tauri/src/contract/events.rs`. */
import type { ErrorCode, UnixMillis } from "./common.ts";
import type { SshApprovalRequest, SshCloseReason } from "./ssh.ts";

export const lockReasons = ["manual", "idle", "sleep", "screenLock"] as const;
export type LockReason = (typeof lockReasons)[number];

export const syncStatuses = ["notConfigured", "offline", "syncing", "upToDate", "error"] as const;
export type SyncStatus = (typeof syncStatuses)[number];

export interface SyncState {
  status: SyncStatus;
  lastSyncedAt: UnixMillis | null;
  error: ErrorCode | null;
}

export type AppEvent =
  | { type: "locked"; reason: LockReason }
  | { type: "syncStateChanged"; state: SyncState }
  | { type: "clipboardCleared" }
  | { type: "sshApprovalRequested"; request: SshApprovalRequest }
  | { type: "sshApprovalClosed"; requestId: string; reason: SshCloseReason };
