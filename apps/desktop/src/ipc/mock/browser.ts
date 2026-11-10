/**
 * Mock browser extension endpoint (M4, M5): one paired browser, and with `&browserPair=1`,
 * `&browserSave=new|update` or `&browserPasskey=get|create|excluded|uv` one pairing, save or
 * passkey request 500 ms after the app subscribes (and is unlocked). `uv` is a sign-in that asks
 * for the master password (`mosaic`). The 1 s Allow guard is enforced here too, like in Rust.
 */
import type {
  BrowserPairing,
  BrowserPairingRequest,
  BrowserPasskeyRequest,
  BrowserRotationKind,
  BrowserRotationRequest,
  BrowserSaveRequest,
  BrowserStatus,
  RequestOf,
} from "../contract/index.ts";
import { rotationHandlers, rotationRequest } from "./browserRotation.ts";
import { passkeyItem } from "./data.ts";
import { fail, requireUnlocked, type MockHandlers, type MockState } from "./state.ts";

/** The `&browserPasskey=` variants. */
/** `uv` asks for the password at once; `uvExpires` is a sign-in whose verification window runs out while the dialog is open. */
export type PasskeyVariant = "get" | "create" | "excluded" | "uv" | "uvExpires";

const REQUEST_DELAY_MS = 500;
const PAIR_TTL_MS = 120_000;
const SAVE_TTL_MS = 300_000;
const PASSKEY_TTL_MS = 300_000;
const ALLOW_GUARD_MS = 1000;
const PAIR_REQUEST_ID = "a1b2c3d4e5f60718293a4b5c6d7e8f90";
const SAVE_REQUEST_ID = "0f9e8d7c6b5a49382716f5e4d3c2b1a0";
const PASSKEY_REQUEST_ID = "5a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d";

export interface MockBrowser {
  endpoint: BrowserStatus["endpoint"];
  registry: BrowserStatus["registry"];
  pairings: BrowserPairing[];
  /** Emit a pairing request after the first subscription (`&browserPair=1`). */
  pairOnStart: boolean;
  /** Emit a save request after the first subscription (`&browserSave=`). */
  saveOnStart: "new" | "update" | null;
  /** Emit a passkey request after the first subscription (`&browserPasskey=`). */
  passkeyOnStart: PasskeyVariant | null;
  /** Emit a password-change request after the first subscription (`&credRotation=browserBegin|browserResult|addOrigin`). */
  rotationOnStart: BrowserRotationKind | null;
  pairing: BrowserPairingRequest | null;
  save: BrowserSaveRequest | null;
  passkey: BrowserPasskeyRequest | null;
  /** The open password-change request, the password Rust generated for it and when the Allow guard started. */
  rotation: BrowserRotationRequest | null;
  rotationGenerated: number;
  rotationShownAt: number;
  /** The open passkey request needs the master password now (the window ran out). */
  passkeyUvExpired: boolean;
}

export function defaultBrowser(now: () => number): MockBrowser {
  return {
    endpoint: "running",
    registry: "ok",
    pairings: [
      {
        pairingId: "0123456789abcdef0123456789abcdef",
        label: "Chrome",
        browser: "chromium",
        createdAt: now() - 3 * 86_400_000,
        lastUsedAt: now() - 2 * 3_600_000,
        connected: false,
      },
    ],
    pairOnStart: false,
    saveOnStart: null,
    passkeyOnStart: null,
    rotationOnStart: null,
    pairing: null,
    save: null,
    passkey: null,
    rotation: null,
    rotationGenerated: 0,
    rotationShownAt: 0,
    passkeyUvExpired: false,
  };
}

function statusOf(state: MockState): BrowserStatus {
  return {
    endpoint: state.browser.endpoint,
    registry: state.browser.registry,
    pairings: state.browser.pairings,
  };
}

function pairingRequest(state: MockState): BrowserPairingRequest {
  const requestedAt = state.now();
  return {
    requestId: PAIR_REQUEST_ID,
    browser: "chromium",
    hint: "edge",
    code: "4F7K2Q",
    requestedAt,
    expiresAt: requestedAt + PAIR_TTL_MS,
  };
}

function saveRequest(state: MockState, kind: "new" | "update"): BrowserSaveRequest {
  const requestedAt = state.now();
  const login = state.items.find((item) => item.detail.summary.kind === "login");
  return {
    requestId: SAVE_REQUEST_ID,
    pairingLabel: "Chrome",
    origin: kind === "new" ? "http://example.com" : "https://example.com",
    host: "example.com",
    username: "alex@example.com",
    kind,
    candidates:
      kind === "update" && login !== undefined
        ? [
            {
              vaultId: login.detail.summary.vaultId,
              itemId: login.detail.summary.itemId,
              title: login.detail.summary.title,
              username: login.detail.summary.subtitle,
            },
          ]
        : [],
    suggestedTitle: "example.com",
    insecure: kind === "new",
    requestedAt,
    expiresAt: requestedAt + SAVE_TTL_MS,
  };
}

function passkeyRequest(state: MockState, variant: PasskeyVariant): BrowserPasskeyRequest {
  const requestedAt = state.now();
  const base = {
    requestId: PASSKEY_REQUEST_ID,
    pairingLabel: "Chrome",
    origin: "https://login.example.com",
    rpId: "example.com",
    requestedAt,
    expiresAt: requestedAt + PASSKEY_TTL_MS,
  };
  state.browser.passkeyUvExpired = variant === "uvExpires";
  if (variant === "get" || variant === "uv" || variant === "uvExpires") {
    return {
      ...base,
      kind: "get",
      userName: null,
      userDisplayName: null,
      candidates: state.items
        .filter((item) => item.detail.passkey !== null)
        .map((item) => ({
          vaultId: item.detail.summary.vaultId,
          itemId: item.detail.summary.itemId,
          vaultName:
            state.vaults.find((vault) => vault.vaultId === item.detail.summary.vaultId)?.name ?? "",
          userName: item.detail.passkey?.userName ?? "",
          userDisplayName: item.detail.passkey?.userDisplayName ?? "",
        })),
      excluded: false,
      needsPassword: variant === "uv",
      userVerification: variant === "uv" || variant === "uvExpires" ? "required" : "preferred",
    };
  }
  return {
    ...base,
    kind: "create",
    userName: "alex@example.com",
    userDisplayName: "Alex Example",
    candidates: [],
    excluded: variant === "excluded",
    needsPassword: false,
    userVerification: variant === "excluded" ? "discouraged" : "preferred",
  };
}

/**
 * Called after the app subscribes and after an unlock: emits the requests the URL asked for,
 * once, 500 ms later, if the vault is unlocked by then.
 */
export function startBrowser(state: MockState): void {
  const browser = state.browser;
  if (
    !browser.pairOnStart &&
    browser.saveOnStart === null &&
    browser.passkeyOnStart === null &&
    browser.rotationOnStart === null
  ) {
    return;
  }
  const pair = browser.pairOnStart;
  const save = browser.saveOnStart;
  const passkey = browser.passkeyOnStart;
  const rotation = browser.rotationOnStart;
  browser.pairOnStart = false;
  browser.saveOnStart = null;
  browser.passkeyOnStart = null;
  browser.rotationOnStart = null;
  setTimeout(() => {
    if (!state.unlocked) {
      browser.pairOnStart = pair;
      browser.saveOnStart = save;
      browser.passkeyOnStart = passkey;
      browser.rotationOnStart = rotation;
      return;
    }
    if (rotation !== null) {
      browser.rotation = rotationRequest(state, rotation);
      browser.rotationShownAt = state.now();
      if (browser.rotation !== null) {
        state.emit({ type: "browserRotationRequested", request: browser.rotation });
      }
    }
    if (pair) {
      browser.pairing = pairingRequest(state);
      state.emit({ type: "browserPairingRequested", request: browser.pairing });
    }
    if (save !== null) {
      browser.save = saveRequest(state, save);
      state.emit({ type: "browserSaveRequested", request: browser.save });
    }
    if (passkey !== null) {
      browser.passkey = passkeyRequest(state, passkey);
      state.emit({ type: "browserPasskeyRequested", request: browser.passkey });
    }
  }, REQUEST_DELAY_MS);
}

export const browserHandlers: MockHandlers = {
  browser_status: (state) => statusOf(state),

  browser_remove_pairing: (state, request) => {
    const browser = state.browser;
    if (!browser.pairings.some((p) => p.pairingId === request.pairingId)) {
      fail("browserPairingNotFound");
    }
    browser.pairings = browser.pairings.filter((p) => p.pairingId !== request.pairingId);
    state.emit({ type: "browserPairingsChanged" });
    return statusOf(state);
  },

  browser_pairing_respond: (state, request) => {
    requireUnlocked(state);
    const browser = state.browser;
    const open = browser.pairing;
    if (open === null || open.requestId !== request.requestId) {
      fail("browserRequestExpired");
    }
    if (request.decision === "allow") {
      if (state.now() - open.requestedAt < ALLOW_GUARD_MS) {
        fail("browserAllowTooSoon");
      }
      const label = request.label ?? "Edge";
      if (label.length < 1 || label.length > 64) {
        fail("invalidInput");
      }
      browser.pairings = [
        ...browser.pairings,
        {
          pairingId: "fedcba9876543210fedcba9876543210",
          label,
          browser: open.browser,
          createdAt: state.now(),
          lastUsedAt: null,
          connected: false,
        },
      ];
      state.emit({ type: "browserPairingsChanged" });
    }
    browser.pairing = null;
    state.emit({ type: "browserPairingClosed", requestId: open.requestId, reason: "answered" });
    return null;
  },

  browser_save_respond: (state, request) => {
    requireUnlocked(state);
    const browser = state.browser;
    const open = browser.save;
    if (open === null || open.requestId !== request.requestId) {
      fail("browserRequestExpired");
    }
    if (request.decision.kind !== "cancel") {
      if (state.now() - open.requestedAt < ALLOW_GUARD_MS) {
        fail("browserAllowTooSoon");
      }
      if (request.decision.kind === "saveNew" && request.decision.title.trim() === "") {
        fail("invalidInput");
      }
    }
    browser.save = null;
    state.emit({ type: "browserSaveClosed", requestId: open.requestId, reason: "answered" });
    return null;
  },

  ...rotationHandlers,

  browser_passkey_respond: (state, request) => {
    requireUnlocked(state);
    const browser = state.browser;
    const open = browser.passkey;
    if (open === null || open.requestId !== request.requestId) {
      fail("browserRequestExpired");
    }
    const decision = request.decision;
    if (!decisionFits(open, decision)) {
      fail("invalidInput");
    }
    if (decision.kind === "create" || decision.kind === "assert") {
      commitPasskey(state, open, request);
    } else if (
      decision.kind === "acknowledge" &&
      state.now() - open.requestedAt < ALLOW_GUARD_MS
    ) {
      fail("browserAllowTooSoon");
    }
    browser.passkey = null;
    state.emit({ type: "browserPasskeyClosed", requestId: open.requestId, reason: "answered" });
    return null;
  },
};

/** Whether the answer fits the request (Rust: `invalidInput` otherwise). */
function decisionFits(
  open: BrowserPasskeyRequest,
  decision: RequestOf<"browser_passkey_respond">["decision"],
): boolean {
  switch (decision.kind) {
    case "create":
      return open.kind === "create" && !open.excluded;
    case "assert":
      return open.kind === "get";
    case "acknowledge":
      return open.excluded;
    case "useAnotherDevice":
      return !open.excluded;
    case "cancel":
      return true;
  }
}

/** The Allow guard and, when the site wants it, the master password. */
function checkUser(
  state: MockState,
  open: BrowserPasskeyRequest,
  request: RequestOf<"browser_passkey_respond">,
): void {
  if (state.now() - open.requestedAt < ALLOW_GUARD_MS) {
    fail("browserAllowTooSoon");
  }
  const typed = request.password !== null && request.password !== "";
  if (open.needsPassword || state.browser.passkeyUvExpired) {
    if (!typed) {
      fail("reauthRequired");
    }
  }
  if (typed && (open.needsPassword || state.browser.passkeyUvExpired)) {
    if (request.password !== state.account?.password) {
      fail("wrongCredentials");
    }
    state.browser.passkeyUvExpired = false;
  }
}

/** The checks of Rust's `respond_passkey` for a create or a sign-in, then the write. */
function commitPasskey(
  state: MockState,
  open: BrowserPasskeyRequest,
  request: RequestOf<"browser_passkey_respond">,
): void {
  const decision = request.decision;
  checkUser(state, open, request);
  if (decision.kind === "create") {
    const vault = state.vaults.find((each) => each.vaultId === decision.vaultId);
    if (vault === undefined) {
      fail("vaultNotFound");
    }
    if (!vault.available || vault.protection !== "accountKey") {
      fail("vaultUnavailable");
    }
    state.items = [
      ...state.items,
      passkeyItem(
        {
          vaultId: vault.vaultId,
          itemId: `019a2b3c-4d5e-7020-8000-0000000000${String(state.items.length).padStart(2, "0")}`,
          userName: open.userName ?? "",
          displayName: open.userDisplayName ?? "",
        },
        { rpId: open.rpId, createdAt: state.now() },
      ),
    ];
  } else if (decision.kind === "assert") {
    const offered = open.candidates.some(
      (c) => c.vaultId === decision.vaultId && c.itemId === decision.itemId,
    );
    if (!offered) {
      fail("itemNotFound");
    }
  }
}
