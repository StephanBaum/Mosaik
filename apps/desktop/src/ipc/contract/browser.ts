/** Browser extension contract: mirrors `src-tauri/src/contract/browser.rs`. */
import type { ItemId, UnixMillis, VaultId } from "./common.ts";
import type { AttemptStage, RotationOutcomeKey } from "./credentialRotation.ts";
import type { RevealedSecret } from "./items.ts";

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
  "abortedByPage",
] as const;
export type BrowserCloseReason = (typeof browserCloseReasons)[number];

export const browserSaveKinds = ["new", "update"] as const;
export type BrowserSaveKind = (typeof browserSaveKinds)[number];

/** What a rotation request asks the user to decide (M7b). */
export const browserRotationKinds = ["begin", "result", "addOrigin"] as const;
export type BrowserRotationKind = (typeof browserRotationKinds)[number];

export const browserPasskeyKinds = ["create", "get"] as const;
export type BrowserPasskeyKind = (typeof browserPasskeyKinds)[number];

/** The site's user verification preference. */
export const passkeyVerifications = ["required", "preferred", "discouraged"] as const;
export type PasskeyVerification = (typeof passkeyVerifications)[number];

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

/** A passkey the user may sign in with. No key material. */
export interface BrowserPasskeyCandidate {
  vaultId: VaultId;
  itemId: ItemId;
  vaultName: string;
  /** May be empty. */
  userName: string;
  /** May be empty. */
  userDisplayName: string;
}

/** A passkey request: what the site asked for, checked by Rust. No challenge, no key. */
export interface BrowserPasskeyRequest {
  /**
   * The desktop's own handle for this prompt: 32 lowercase hex digits, made for each prompt and
   * never the extension's request ID.
   */
  requestId: string;
  pairingLabel: string;
  kind: BrowserPasskeyKind;
  /** The page's origin, from the browser. */
  origin: string;
  /** The relying party ID Rust accepted for the origin. */
  rpId: string;
  /** Create only: sanitized, at most 64 bytes. `null` for a sign-in. */
  userName: string | null;
  userDisplayName: string | null;
  /** Sign-in only (never empty); empty for a create. */
  candidates: BrowserPasskeyCandidate[];
  /** Create only: Mosaic already holds a passkey the site excludes. Informs only. */
  excluded: boolean;
  /** The site wants user verification and none happened in five minutes: ask for the password. */
  needsPassword: boolean;
  userVerification: PasskeyVerification;
  requestedAt: UnixMillis;
  expiresAt: UnixMillis;
}

export type BrowserPasskeyDecision =
  | { kind: "create"; vaultId: VaultId }
  | { kind: "assert"; vaultId: VaultId; itemId: ItemId }
  | { kind: "useAnotherDevice" }
  | { kind: "cancel" }
  | { kind: "acknowledge" };

export interface BrowserPasskeyRespondRequest {
  /** The prompt's handle, from the event. */
  requestId: string;
  decision: BrowserPasskeyDecision;
  /**
   * The master password when the user typed one: the dialog asked for it (`needsPassword`) or
   * Rust answered `reauthRequired` because the verification ran out while the dialog was open;
   * `null` otherwise.
   */
  password: string | null;
}

/**
 * The extension starts a password change (`begin`), reports its result (`result`) or asks to add a
 * related address (`addOrigin`). No password: the generated one is fetched with
 * `browser_rotation_reveal`.
 */
export interface BrowserRotationAttempt {
  startedAt: UnixMillis;
  stage: AttemptStage;
  /** How many other changes of the login are open. */
  otherOpen: number;
}

export interface BrowserRotationRequest {
  /** The desktop's handle for this prompt: 32 lowercase hex digits. */
  requestId: string;
  pairingLabel: string;
  kind: BrowserRotationKind;
  vaultId: VaultId;
  itemId: ItemId;
  title: string | null;
  /** The page's host (punycode stays punycode); for `addOrigin` the host to add. */
  host: string;
  /** The Unicode form of `host` when a label is punycode. */
  unicodeHost: string | null;
  /** `result` only: what the user said in the popup. */
  outcome: RotationOutcomeKey | null;
  /** `addOrigin` only: the origin to add. */
  newOrigin: string | null;
  /** `result` only: which password change it closes (a login can have up to four open). */
  attempt: BrowserRotationAttempt | null;
  requestedAt: UnixMillis;
  expiresAt: UnixMillis;
}

/**
 * `allow` and `regenerate` fit a `begin` (`regenerate` keeps the dialog open with a new password),
 * `confirm` a `result` the user reported as succeeded, `keepOld` a `result`, `addOrigin` an
 * `addOrigin`; `cancel` fits all.
 */
export type BrowserRotationDecision =
  | { kind: "allow" }
  | { kind: "regenerate" }
  | { kind: "confirm" }
  | { kind: "keepOld" }
  | { kind: "addOrigin" }
  | { kind: "cancel" };

export interface BrowserRotationRespondRequest {
  requestId: string;
  decision: BrowserRotationDecision;
}

export interface BrowserRotationRevealRequest {
  requestId: string;
}

/** The generated password of an open `begin` request, shown in the dialog for a moment. */
export type BrowserRotationRevealed = RevealedSecret;

/** The commands of the password-change dialog (`Commands` in `commands.ts` extends it). */
export interface BrowserRotationCommands {
  browser_rotation_respond: { request: BrowserRotationRespondRequest; response: null };
  browser_rotation_reveal: { request: BrowserRotationRevealRequest; response: BrowserRotationRevealed };
}
