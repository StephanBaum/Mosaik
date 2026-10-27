/** SSH agent contract: mirrors `src-tauri/src/contract/ssh.rs`. */
import type { ItemId, UnixMillis, VaultId } from "./common.ts";

export const sshDecisions = ["allowOnce", "allowUntilLock", "deny"] as const;
export type SshDecision = (typeof sshDecisions)[number];

export const sshCloseReasons = ["answered", "expired", "locked"] as const;
export type SshCloseReason = (typeof sshCloseReasons)[number];

export interface SshAgentStatus {
  enabled: boolean;
  running: boolean;
  endpoint: string | null;
  keyCount: number;
}

export interface SshSetEnabledRequest {
  enabled: boolean;
}

export interface SshClient {
  processName: string | null;
  pid: number | null;
}

export interface SshApprovalRequest {
  requestId: string;
  vaultId: VaultId;
  itemId: ItemId;
  keyTitle: string | null;
  fingerprint: string;
  client: SshClient;
  expiresAt: UnixMillis;
}

export interface SshRespondRequest {
  requestId: string;
  decision: SshDecision;
}
