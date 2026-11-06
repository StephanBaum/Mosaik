/**
 * The extension to host envelope and the bodies of every op (docs/NATIVE_MESSAGING.md §5).
 * Rules of DESKTOP_IPC §1.4: camelCase, every key present, `null` for no value, unknown keys
 * rejected (`validate.ts` enforces them on what comes back). Secrets are plain strings here;
 * keep them in short-lived locals and never log them.
 */
import type { WireErrorCode } from "./errors.ts";

export const PROTOCOL_VERSION = 1;
/** Frame cap on the stdio link, in bytes. */
export const MAX_FRAME_BYTES = 262_144;
/** The host waits 15 s for the desktop; the extension waits a little longer. */
export const REQUEST_TIMEOUT_MS = 20_000;
/** A save prompt stays open for 300 s on the desktop (SAVE_PROMPT_TIMEOUT). */
export const SAVE_PROMPT_TIMEOUT_MS = 305_000;
/** A passkey prompt stays open for 300 s on the desktop (PASSKEY_PROMPT_TIMEOUT). */
export const PASSKEY_PROMPT_TIMEOUT_MS = 305_000;
/** A pairing request stays open for 120 s on the desktop. */
export const PAIR_TIMEOUT_MS = 125_000;
/** The port is closed after this long with nothing in flight. */
export const IDLE_DISCONNECT_MS = 60_000;
export const MAX_IN_FLIGHT = 4;

export type Hint = "chrome" | "edge" | "brave" | "opera" | "vivaldi" | "chromium" | "firefox" | "other";
export type LockState = "locked" | "unlocked";
export type DesktopState = "notRunning" | LockState;
export type EmptyBody = Record<string, never>;

export interface Pairing {
  /** 32 lowercase hex digits. */
  pairingId: string;
  /** 64 lowercase hex digits. A secret. */
  key: string;
}

export interface LoginMatch {
  vaultId: string;
  itemId: string;
  title: string | null;
  username: string | null;
  host: string | null;
  hasOtp: boolean;
  favorite: boolean;
  matchedBy: "exactOrigin" | "etld1";
  blocked: "insecure" | null;
}

export interface SearchHit {
  vaultId: string;
  itemId: string;
  title: string;
  username: string | null;
  host: string | null;
  hasOtp: boolean;
  favorite: boolean;
  fillable: boolean;
}

export type OriginWarning =
  | { kind: "idn"; unicodeHost: string }
  | { kind: "insecure" }
  | { kind: "lookalike"; savedHost: string };

export type CopySource =
  | { kind: "item"; vaultId: string; itemId: string; field: "username" | "password" | "totp" }
  | { kind: "generated" };

/** The desktop's `GenerateRequest`; the extension only forwards it (`forSite` carries `rules`). */
export type GenerateRequest = { kind: string } & Readonly<Record<string, unknown>>;

export interface FillWant {
  login: boolean;
  otp: boolean;
}

export interface SaveBody {
  origin: string;
  topOrigin: string;
  username: string | null;
  password: string;
}

export type UserVerification = "required" | "preferred" | "discouraged";

/**
 * `passkeyCreate` (docs/NATIVE_MESSAGING.md §5.2). Binary values are base64url without padding.
 * `origin` and `topOrigin` come from the browser, never from the page.
 */
export interface PasskeyCreateBody {
  /** 32 lowercase hex digits, chosen by the extension. */
  requestId: string;
  origin: string;
  topOrigin: string;
  rpId: string | null;
  userId: string;
  userName: string;
  userDisplayName: string;
  challenge: string;
  algorithms: number[];
  excludeCredentials: string[];
  userVerification: UserVerification;
}

export interface PasskeyGetBody {
  requestId: string;
  origin: string;
  topOrigin: string;
  rpId: string | null;
  challenge: string;
  allowCredentials: string[];
  userVerification: UserVerification;
}

/** The credential a `passkeyCreate` made. The only key material is the public key. */
export interface CreatedCredential {
  credentialId: string;
  clientDataJson: string;
  attestationObject: string;
  authenticatorData: string;
  publicKeySpki: string;
  publicKeyAlgorithm: number;
  vaultId: string;
  itemId: string;
}

export interface AssertionBody {
  credentialId: string;
  clientDataJson: string;
  authenticatorData: string;
  signature: string;
  userHandle: string;
}

export interface RequestMap {
  hello: { pairing: Pairing | null; hint: Hint };
  pair: { hint: Hint };
  status: EmptyBody;
  focus: EmptyBody;
  listForOrigin: { origin: string; topOrigin: string };
  search: { query: string; origin: string | null };
  fill: {
    vaultId: string;
    itemId: string;
    origin: string;
    topOrigin: string;
    frame: "top" | "sameOriginFrame";
    want: FillWant;
  };
  copy: { source: CopySource };
  generate: { request: GenerateRequest };
  checkSave: SaveBody;
  proposeSave: SaveBody;
  unpair: EmptyBody;
  passkeyCreate: PasskeyCreateBody;
  passkeyGet: PasskeyGetBody;
  passkeyCancel: { requestId: string };
}

export interface ResultMap {
  hello: { desktop: DesktopState; paired: boolean };
  pair: Pairing;
  status: { state: LockState };
  focus: EmptyBody;
  listForOrigin: { items: LoginMatch[]; warnings: OriginWarning[] };
  search: { items: SearchHit[] };
  fill: { username: string | null; password: string | null; otp: string | null };
  copy: { clearAfterSeconds: number | null };
  generate: {
    value: string;
    entropyBits: number;
    strength: string;
    rulesSource: "page" | "siteQuirks" | null;
  };
  checkSave: { kind: "new" | "update" | "unchanged"; itemTitle: string | null };
  proposeSave: {
    outcome: "saved" | "updated" | "cancelled";
    vaultId: string | null;
    itemId: string | null;
  };
  unpair: EmptyBody;
  passkeyCreate: { outcome: "created" | "useAnotherDevice"; credential: CreatedCredential | null };
  passkeyGet: { outcome: "asserted" | "useAnotherDevice"; assertion: AssertionBody | null };
  passkeyCancel: EmptyBody;
}

export type Op = keyof RequestMap;

/** Every op, in the order of the protocol document. */
export const OPS = [
  "hello",
  "pair",
  "status",
  "focus",
  "listForOrigin",
  "search",
  "fill",
  "copy",
  "generate",
  "checkSave",
  "proposeSave",
  "unpair",
  "passkeyCreate",
  "passkeyGet",
  "passkeyCancel",
] as const satisfies readonly Op[];

export interface RequestEnvelope<O extends Op = Op> {
  v: typeof PROTOCOL_VERSION;
  id: number;
  op: O;
  body: RequestMap[O];
}

export type HostEvent =
  | { v: 1; event: "state"; state: LockState }
  | { v: 1; event: "unpaired" }
  | { v: 1; event: "disconnected" }
  | { v: 1; event: "pairCode"; requestId: number; code: string; expiresInSeconds: number };

export type HostMessage =
  | { kind: "event"; event: HostEvent }
  | { kind: "response"; id: number; ok: true; body: unknown }
  | { kind: "response"; id: number; ok: false; error: WireErrorCode };
