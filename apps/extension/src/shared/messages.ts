/**
 * Messages inside the extension: popup and background (port `popup`), background and the
 * content script (`tabs.sendMessage`). None of these is ever sent by, or accepted from, a
 * web page: the router and the content script check the sender (docs/EXTENSION_SECURITY.md).
 */
import { isErrorCode, type ErrorCode } from "./errors.ts";
import type {
  CopySource,
  GenerateRequest,
  LoginMatch,
  OriginWarning,
  RotationMatch,
  SearchHit,
} from "./protocol.ts";
import {
  bool,
  copySource,
  exactly,
  int,
  isGenerateRequest,
  lit,
  nullable,
  obj,
  str,
  tagged,
  type Guard,
} from "./validate.ts";

/** The name of the port the popup connects on. */
export const POPUP_PORT = "popup";

/** What the popup sees of the link to the desktop. */
export type ConnectionState =
  | "checking"
  | "hostMissing"
  | "hostForbidden"
  | "desktopNotRunning"
  | "notPaired"
  | "pairing"
  | "locked"
  | "unlocked"
  | "protocolMismatch"
  | "error";

export type FrameClass = "top" | "sameOriginFrame" | "crossOriginFrame" | "none";

export interface TabInfo {
  origin: string;
  host: string;
  unicodeHost: string | null;
  secure: boolean;
  supported: boolean;
}

/** A frame as the popup showed it: what a frame confirmation is bound to. */
export interface FrameRef {
  frameId: number;
  /** Null where the browser gives none (Firefox): then only the frame id is compared. */
  documentId: string | null;
}

export interface PendingSaveInfo {
  host: string;
  unicodeHost: string | null;
  username: string | null;
  /** `null`: not asked yet (the desktop was locked); the popup asks when it opens. */
  kind: "new" | "update" | null;
  itemTitle: string | null;
  source: "capture" | "generated";
  insecure: boolean;
}

export interface NextOtp {
  vaultId: string;
  itemId: string;
  title: string | null;
  username: string | null;
}

/**
 * The page is on a related address of a login (the same registrable domain) that the extension
 * sent the tab to: the popup offers to add it. The desktop decides.
 */
export interface RotationTabOffer {
  host: string;
  unicodeHost: string | null;
  title: string | null;
}

export interface PopupState {
  connection: ConnectionState;
  pairCode: { code: string; expiresInSeconds: number } | null;
  tab: TabInfo | null;
  frame: FrameClass;
  /** The frame behind `frame` (a fill with a confirmation names it), or null. */
  frameTarget: FrameRef | null;
  /**
   * The frame that holds the change form of an open password change, as the change fill will pick
   * it (not the sign-in form `frame` is about), or null: the popup confirms this frame for
   * `fillChange`.
   */
  changeFrame: { frame: FrameClass; target: FrameRef } | null;
  matches: LoginMatch[];
  warnings: OriginWarning[];
  pendingSave: PendingSaveInfo | null;
  nextOtp: NextOtp | null;
  lastError: ErrorCode | null;
  saveOffers: boolean;
  /** "Use Mosaic for passkeys" is on: the passkey scripts are registered and the site access is granted. */
  passkeys: boolean;
  generatorFill: boolean;
  /** The open password changes of logins saved for exactly this page (at most 5); empty off https. */
  rotations: RotationMatch[];
  /** A related address the tab was sent to (`rotationOpenPage`): offer to add it, or null. */
  rotationTabOffer: RotationTabOffer | null;
}

export type SaveAction = "save" | "dismiss" | "never";

/** Popup operations. A frame other than the top frame is filled only with the `confirmFrame` the popup got from `getState`. */
export type PopupRequest =
  | { op: "getState" }
  | { op: "pair" }
  | { op: "focusDesktop" }
  | { op: "search"; query: string }
  | { op: "fill"; vaultId: string; itemId: string; confirmFrame: FrameRef | null }
  | { op: "fillOtp"; vaultId: string; itemId: string; confirmFrame: FrameRef | null }
  | { op: "copy"; source: CopySource }
  | { op: "generate"; request: GenerateRequest }
  | { op: "fillGenerated"; confirmFrame: FrameRef | null }
  | { op: "savePending"; action: SaveAction }
  | { op: "saveOffersChanged"; enabled: boolean }
  | { op: "setPasskeys"; enabled: boolean }
  | { op: "neverSaveList" }
  | { op: "neverSaveRemove"; origin: string }
  | { op: "rotationBegin"; vaultId: string; itemId: string }
  | { op: "rotationOpenPage"; vaultId: string; itemId: string; attemptId: string }
  | { op: "fillChange"; vaultId: string; itemId: string; attemptId: string; confirmFrame: FrameRef | null }
  | { op: "fillPending"; vaultId: string; itemId: string; attemptId: string; confirmFrame: FrameRef | null }
  | { op: "rotationResult"; vaultId: string; itemId: string; attemptId: string; outcome: "succeeded" | "failed" }
  | { op: "rotationAddOrigin" }
  | { op: "unpair" };

export interface PopupResults {
  getState: PopupState;
  pair: Record<string, never>;
  focusDesktop: Record<string, never>;
  search: { items: SearchHit[] };
  fill: FillSummary;
  fillOtp: FillSummary;
  copy: { clearAfterSeconds: number | null };
  generate: { value: string; entropyBits: number; strength: string; rulesSource: "page" | "siteQuirks" | null };
  fillGenerated: FillSummary;
  savePending: { outcome: "saved" | "updated" | "cancelled" | "dismissed" | "never" };
  saveOffersChanged: { saveOffers: boolean };
  setPasskeys: { passkeys: boolean };
  neverSaveList: { origins: string[] };
  neverSaveRemove: { origins: string[] };
  rotationBegin: { outcome: "started" | "cancelled" };
  rotationOpenPage: Record<string, never>;
  fillChange: FillSummary;
  fillPending: FillSummary;
  rotationResult: { outcome: "confirmed" | "keptOld" | "cancelled" };
  rotationAddOrigin: { outcome: "added" | "cancelled" };
  unpair: Record<string, never>;
}

/** What a fill reports back: counts only, never values. */
export interface FillSummary {
  username: number;
  password: number;
  otp: number;
  newPassword: number;
}

export interface PopupEnvelope {
  id: number;
  request: PopupRequest;
}

export type PopupReply =
  | { id: number; ok: true; result: unknown }
  | { id: number; ok: false; error: ErrorCode };

/** Pushed to every open popup port. */
export type PopupEvent =
  | { event: "connection"; connection: ConnectionState }
  | { event: "pairCode"; code: string; expiresInSeconds: number };

const id = str(128);
const attemptId = (value: unknown): value is string =>
  typeof value === "string" && /^[A-Za-z0-9._-]{1,64}$/.test(value);
const frameRef: Guard<FrameRef> = obj({ frameId: int(0, 2_147_483_647), documentId: nullable(str(64)) });
const confirmFrame = nullable(frameRef);

const popupRequest: Guard<PopupRequest> = tagged<PopupRequest>("op", {
  getState: obj({ op: exactly("getState" as const) }),
  pair: obj({ op: exactly("pair" as const) }),
  focusDesktop: obj({ op: exactly("focusDesktop" as const) }),
  search: obj({ op: exactly("search" as const), query: str(128) }),
  fill: obj({ op: exactly("fill" as const), vaultId: id, itemId: id, confirmFrame }),
  fillOtp: obj({ op: exactly("fillOtp" as const), vaultId: id, itemId: id, confirmFrame }),
  copy: obj({ op: exactly("copy" as const), source: copySource }),
  generate: obj({ op: exactly("generate" as const), request: isGenerateRequest }),
  fillGenerated: obj({ op: exactly("fillGenerated" as const), confirmFrame }),
  savePending: obj({ op: exactly("savePending" as const), action: lit("save", "dismiss", "never") }),
  saveOffersChanged: obj({ op: exactly("saveOffersChanged" as const), enabled: bool }),
  setPasskeys: obj({ op: exactly("setPasskeys" as const), enabled: bool }),
  neverSaveList: obj({ op: exactly("neverSaveList" as const) }),
  neverSaveRemove: obj({ op: exactly("neverSaveRemove" as const), origin: str(2048) }),
  rotationBegin: obj({ op: exactly("rotationBegin" as const), vaultId: id, itemId: id }),
  rotationOpenPage: obj({ op: exactly("rotationOpenPage" as const), vaultId: id, itemId: id, attemptId }),
  fillChange: obj({ op: exactly("fillChange" as const), vaultId: id, itemId: id, attemptId, confirmFrame }),
  fillPending: obj({ op: exactly("fillPending" as const), vaultId: id, itemId: id, attemptId, confirmFrame }),
  rotationResult: obj({
    op: exactly("rotationResult" as const),
    vaultId: id,
    itemId: id,
    attemptId,
    outcome: lit("succeeded", "failed"),
  }),
  rotationAddOrigin: obj({ op: exactly("rotationAddOrigin" as const) }),
  unpair: obj({ op: exactly("unpair" as const) }),
});

const requestNumber = (value: unknown): value is number =>
  typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= 4_294_967_295;

export const isPopupEnvelope: Guard<PopupEnvelope> = obj({ id: requestNumber, request: popupRequest });

// --- background and content script ---

/** One frame as the content script reports it (docs/EXTENSION_SECURITY.md, "frames"). */
export interface ProbeResult {
  origin: string;
  ancestorsSameOrigin: boolean;
  frameVisible: boolean;
  /** Ancestor count: 0 for the top frame. */
  depth: number;
  activeField: boolean;
  counts: { username: number; password: number; newPassword: number; otp: number };
  passwordRules: string | null;
  visible: boolean;
  /**
   * What the change-form heuristic made of the form (`content/change.ts`): decided before the
   * desktop is asked for a secret. `none`: no password change form; `ambiguous`: more or fewer
   * fields than a change form has, or fields that cannot be told apart.
   */
  changeShape: ChangeFormShape;
}

export type ChangeFormShape = "none" | "ambiguous" | "currentAndNew" | "newOnly";

/** A probe plus what the browser knows of the frame. */
export interface FrameProbe extends ProbeResult {
  frameId: number;
  documentId: string | null;
}

export interface FillMessage {
  type: "fill";
  fillId: string;
  expectedOrigin: string;
  username: string | null;
  password: string | null;
  otp: string | null;
}

export interface FillGeneratedMessage {
  type: "fillGenerated";
  fillId: string;
  expectedOrigin: string;
  password: string;
}

/**
 * The two values of **one** login's password change, for one frame and one document: the current
 * password (null for a form with new-password fields only) and the new one. The content script
 * classifies the form again and fills only if it is the form the probe saw.
 */
export interface FillChangeMessage {
  type: "fillChange";
  fillId: string;
  expectedOrigin: string;
  shape: "currentAndNew" | "newOnly";
  current: string | null;
  new: string;
}

export type ContentMessage = FillMessage | FillGeneratedMessage | FillChangeMessage;

export type FillReply =
  | { ok: true; filled: FillSummary; usernameValue: string | null }
  | { ok: false; error: ErrorCode };

export const isContentMessage: Guard<ContentMessage> = tagged<ContentMessage>("type", {
  fill: obj({
    type: exactly("fill" as const),
    fillId: str(64),
    expectedOrigin: str(2048),
    username: nullable(str(256)),
    password: nullable(str(1024)),
    otp: nullable(str(32)),
  }),
  fillGenerated: obj({
    type: exactly("fillGenerated" as const),
    fillId: str(64),
    expectedOrigin: str(2048),
    password: str(1024),
  }),
  fillChange: obj({
    type: exactly("fillChange" as const),
    fillId: str(64),
    expectedOrigin: str(2048),
    shape: lit("currentAndNew", "newOnly"),
    current: nullable(str(1024)),
    new: str(1024),
  }),
});

const count = int(0, 10_000);

export const isProbeResult: Guard<ProbeResult> = obj({
  origin: str(2048),
  ancestorsSameOrigin: bool,
  frameVisible: bool,
  depth: int(0, 64),
  activeField: bool,
  counts: obj({ username: count, password: count, newPassword: count, otp: count }),
  passwordRules: nullable(str(4096)),
  visible: bool,
  changeShape: lit("none", "ambiguous", "currentAndNew", "newOnly"),
});

const isFillOk = obj({
  ok: exactly(true as const),
  filled: obj({ username: count, password: count, otp: count, newPassword: count }),
  usernameValue: nullable(str(1024)),
});
const isFillFailed = obj({ ok: exactly(false as const), error: isErrorCode });

export const isFillReply = (value: unknown): value is FillReply => isFillOk(value) || isFillFailed(value);

/**
 * A save capture from the opt-in capture script (`capture.js`). `password` is what was typed in
 * the form; `newPassword` is set only for a change form (current plus new password), and is
 * then the secret to save. The sender's origin and tab come from the browser, never from here.
 */
export interface CapturedMessage {
  type: "captured";
  username: string | null;
  password: string;
  newPassword: string | null;
  /** The script's own claim about its frame; the background checks the rest. */
  ancestorsSameOrigin: boolean;
}

const secret = (value: unknown): value is string =>
  typeof value === "string" && value.length >= 1 && value.length <= 1024;

export const isCapturedMessage: Guard<CapturedMessage> = obj({
  type: exactly("captured" as const),
  username: nullable(str(256)),
  password: secret,
  newPassword: nullable(secret),
  ancestorsSameOrigin: bool,
});
