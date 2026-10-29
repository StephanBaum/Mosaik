/** SSH agent contract: mirrors `src-tauri/src/contract/ssh.rs`. */
import type { ErrorCode, ItemId, UnixMillis, VaultId } from "./common.ts";

export const sshDecisions = ["allowOnce", "allowFor15Minutes", "deny"] as const;
export type SshDecision = (typeof sshDecisions)[number];

export const sshCloseReasons = ["answered", "expired", "locked"] as const;
export type SshCloseReason = (typeof sshCloseReasons)[number];

export const sshSkipReasons = ["unsupported", "malformed", "duplicate"] as const;
export type SshSkipReason = (typeof sshSkipReasons)[number];

export const sshSignFailReasons = [
  "passphraseRequired",
  "wrongPassphrase",
  "keyMalformed",
  "kdfTooExpensive",
  "publicKeyMismatch",
  "unsupported",
  "internal",
] as const;
export type SshSignFailReason = (typeof sshSignFailReasons)[number];

export interface SshSkippedKey {
  vaultId: VaultId;
  itemId: ItemId;
  reason: SshSkipReason;
}

export interface SshAgentStatus {
  enabled: boolean;
  running: boolean;
  endpoint: string | null;
  keyCount: number;
  skipped: SshSkippedKey[];
  /** Why the agent could not start. */
  error: ErrorCode | null;
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
  requestedAt: UnixMillis;
  expiresAt: UnixMillis;
  /** Whether "Allow for 15 minutes" is offered. */
  canRemember: boolean;
}

export interface SshRespondRequest {
  requestId: string;
  decision: SshDecision;
}
