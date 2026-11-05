/** Browser extension contract: mirrors `src-tauri/src/contract/browser.rs`. */
import type { ItemId, UnixMillis, VaultId } from "./common.ts";

export const browserEndpointStates = ["running", "inUse", "unavailable"] as const;
export type BrowserEndpointState = (typeof browserEndpointStates)[number];

/** Whether the saved list of paired browsers could be read; `unreadable` makes it read-only. */
export const browserRegistryStates = ["ok", "unreadable"] as const;
export type BrowserRegistryState = (typeof browserRegistryStates)[number];

export const browserFamilies = ["chromium", "firefox"] as const;
export type BrowserFamily = (typeof browserFamilies)[number];

export const browserHints = [
  "chrome",
  "edge",
  "brave",
  "opera",
  "vivaldi",
  "chromium",
  "firefox",
  "other",
] as const;
export type BrowserHint = (typeof browserHints)[number];

export const pairingDecisions = ["allow", "deny"] as const;
export type PairingDecision = (typeof pairingDecisions)[number];

export const browserCloseReasons = [
  "answered",
  "expired",
  "conflict",
  "locked",
  "disconnected",
] as const;
export type BrowserCloseReason = (typeof browserCloseReasons)[number];

export const browserSaveKinds = ["new", "update"] as const;
export type BrowserSaveKind = (typeof browserSaveKinds)[number];

export interface BrowserPairing {
  /** 32 lowercase hex digits. */
  pairingId: string;
  label: string;
  browser: BrowserFamily;
  createdAt: UnixMillis;
  lastUsedAt: UnixMillis | null;
  /** A session is open right now. */
  connected: boolean;
}

export interface BrowserStatus {
  endpoint: BrowserEndpointState;
  /** `unreadable`: the saved file is damaged or locked; nothing is added or removed until fixed. */
  registry: BrowserRegistryState;
  pairings: BrowserPairing[];
}

export interface BrowserRemovePairingRequest {
  pairingId: string;
}

export interface BrowserPairingRequest {
  requestId: string;
  browser: BrowserFamily;
  hint: BrowserHint;
  /** Six symbols of Crockford base32; shown as `XXX-XXX`. */
  code: string;
  requestedAt: UnixMillis;
  expiresAt: UnixMillis;
}

export interface BrowserPairingRespondRequest {
  requestId: string;
  decision: PairingDecision;
  /** 1 to 64 characters; `null` takes the name from the hint. */
  label: string | null;
}

export interface BrowserSaveCandidate {
  vaultId: VaultId;
  itemId: ItemId;
  title: string | null;
  username: string | null;
}

/** What would be saved. The password stays in Rust and is not in this type. */
export interface BrowserSaveRequest {
  requestId: string;
  pairingLabel: string;
  origin: string;
  host: string;
  username: string | null;
  kind: BrowserSaveKind;
  candidates: BrowserSaveCandidate[];
  suggestedTitle: string;
  /** The page is plain HTTP: offer the fill override. */
  insecure: boolean;
  requestedAt: UnixMillis;
  expiresAt: UnixMillis;
}

export type BrowserSaveDecision =
  | { kind: "saveNew"; vaultId: VaultId; title: string; allowHttp: boolean }
  | { kind: "update"; vaultId: VaultId; itemId: ItemId }
  | { kind: "cancel" };

export interface BrowserSaveRespondRequest {
  requestId: string;
  decision: BrowserSaveDecision;
}
