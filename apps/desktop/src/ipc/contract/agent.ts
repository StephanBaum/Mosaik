/**
 * Agent access contract: mirrors `src-tauri/src/contract/agent.rs` (bindings) and
 * `agent_access.rs` (clients, dialogs, grants, log); ADR-0159, ADR-0160.
 */
import type { ItemId, UnixMillis, VaultId } from "./common.ts";
import type { PairingDecision } from "./browser.ts";
import type { SecretRef } from "./items.ts";

export const agentInjectKinds = ["bearer", "header", "basic", "query"] as const;
export type AgentInjectKind = (typeof agentInjectKinds)[number];

export const agentBasicUserKinds = ["literal", "field", "secretIsUser"] as const;
export type AgentBasicUserKind = (typeof agentBasicUserKinds)[number];

export interface AgentBasicUserView {
  kind: AgentBasicUserKind;
  /** The literal or the text field key; `null` for `secretIsUser`. */
  value: string | null;
}

/** How a binding places the value. Only the members of `kind` are set. */
export interface AgentInjectView {
  kind: AgentInjectKind;
  headerName: string | null;
  headerPrefix: string | null;
  basicUser: AgentBasicUserView | null;
  queryName: string | null;
}

export interface AgentBindingSnapshot {
  origins: string[];
  inject: AgentInjectKind;
}

/** One binding of an item. Never a value. */
export interface AgentBindingView {
  elementId: string;
  /** A `field` or a `customField`. */
  field: SecretRef;
  origins: string[];
  inject: AgentInjectView;
  env: string | null;
  /** The user confirmed exactly this binding on this device. */
  confirmed: boolean;
  /** The last confirmed state; `null` if never confirmed here. */
  lastConfirmed: AgentBindingSnapshot | null;
  /** An agent could use it now. */
  usable: boolean;
}

export type AgentBindingChange =
  | { op: "add"; field: SecretRef; origins: string[]; inject: AgentInjectView; env: string | null }
  | {
      op: "update";
      elementId: string;
      field: SecretRef;
      origins: string[];
      inject: AgentInjectView;
      env: string | null;
    }
  | { op: "remove"; elementId: string };

export interface AgentBindingConfirmRequest {
  vaultId: VaultId;
  itemId: ItemId;
  elementId: string;
}

export const agentEndpointStates = ["running", "stopped", "unavailable"] as const;
export type AgentEndpointState = (typeof agentEndpointStates)[number];

export const agentGrantStates = ["once", "until", "untilLock"] as const;
export type AgentGrantState = (typeof agentGrantStates)[number];

export const agentAccessDecisions = ["deny", "allowOnce", "allow15m", "allow1h", "allowUntilLock"] as const;
export type AgentAccessDecision = (typeof agentAccessDecisions)[number];

export const agentCloseReasons = ["answered", "expired", "locked", "disconnected", "off"] as const;
export type AgentCloseReason = (typeof agentCloseReasons)[number];

export const agentLogKinds = [
  "paired",
  "pairingRemoved",
  "accessGranted",
  "accessDenied",
  "request",
  "refused",
  "revoked",
  "stoppedAll",
  "bindingConfirmed",
] as const;
export type AgentLogKind = (typeof agentLogKinds)[number];

export const agentRefusalCodes = [
  "desktopNotRunning",
  "locked",
  "agentAccessOff",
  "notPaired",
  "notAgentUsable",
  "needsReview",
  "ambiguousRef",
  "noGrant",
  "denied",
  "destinationNotAllowed",
  "methodNotAllowed",
  "headerNotAllowed",
  "requestTooLarge",
  "responseTooLarge",
  "timeout",
  "tlsFailed",
  "rateLimited",
  "busy",
  "invalidRef",
  "invalidRequest",
  "requestFailed",
  "internal",
] as const;
export type AgentRefusalCode = (typeof agentRefusalCodes)[number];

export interface AgentPairingView {
  /** 32 lowercase hex digits. */
  pairingId: string;
  /** The name the client gave at pairing. Not verified. */
  name: string;
  pairedAt: UnixMillis;
  lastUsedAt: UnixMillis | null;
}

export interface AgentGrantView {
  grantId: number;
  pairingId: string;
  pairingName: string;
  itemTitle: string;
  vaultName: string;
  fieldLabel: string;
  state: AgentGrantState;
  /** Set when `state` is `until`. */
  expiresAt: UnixMillis | null;
}

export interface AgentAccessStatus {
  enabled: boolean;
  endpoint: AgentEndpointState;
  pairings: AgentPairingView[];
  grants: AgentGrantView[];
  /** Requests being sent right now. */
  inFlight: number;
  /** Where the `mosaic` program is; `null` if this build has none next to it. */
  cliPath: string | null;
}

export interface AgentSetEnabledRequest {
  enabled: boolean;
}

export interface AgentPairingRequest {
  requestId: string;
  /** The name the client gave. Not verified. */
  name: string;
  /** The six-symbol code the terminal shows. */
  code: string;
  requestedAt: UnixMillis;
  expiresAt: UnixMillis;
}

export interface AgentPairingRespondRequest {
  requestId: string;
  decision: PairingDecision;
}

export interface AgentPairingRemoveRequest {
  pairingId: string;
}

/** Facts first; `reason` is the agent's own words (at most 200 characters), shown as such. */
export interface AgentAccessRequest {
  requestId: string;
  /** The name the client gave at pairing. Not verified. */
  pairingName: string;
  itemTitle: string;
  vaultName: string;
  fieldLabel: string;
  origins: string[];
  inject: AgentInjectKind;
  reason: string | null;
  /** Requests waiting behind this one. */
  waiting: number;
  requestedAt: UnixMillis;
  /** Refused automatically then; the queue time counts. */
  expiresAt: UnixMillis;
}

export interface AgentAccessRespondRequest {
  requestId: string;
  decision: AgentAccessDecision;
}

export interface AgentGrantRevokeRequest {
  grantId: number;
}

export interface AgentLogRequest {
  pairingId: string | null;
  itemId: ItemId | null;
  /** Only entries before this time; `null`: the newest. */
  beforeMs: UnixMillis | null;
  /** At most this many (capped at 200). */
  limit: number;
}

/** Facts only: never a value, a header, a body or a query string. */
export interface AgentLogView {
  atMs: UnixMillis;
  kind: AgentLogKind;
  pairingId: string | null;
  pairingName: string | null;
  itemId: string | null;
  itemTitle: string | null;
  fieldLabel: string | null;
  method: string | null;
  origin: string | null;
  path: string | null;
  status: number | null;
  bytes: number | null;
  masked: number | null;
  code: AgentRefusalCode | null;
  grant: AgentGrantState | null;
  origins: string[] | null;
  inject: AgentInjectKind | null;
}

export interface AgentLogPage {
  entries: AgentLogView[];
  /** Entries that could not be opened, skipped. */
  skipped: number;
}
