/**
 * Mock browser extension endpoint (M4): one paired browser, and with `&browserPair=1` or
 * `&browserSave=new|update` one pairing or save request 500 ms after the app subscribes (and
 * is unlocked). The 1 s Allow guard is enforced here too, like in Rust.
 */
import type {
  BrowserPairing,
  BrowserPairingRequest,
  BrowserSaveRequest,
  BrowserStatus,
} from "../contract/index.ts";
import { fail, requireUnlocked, type MockHandlers, type MockState } from "./state.ts";

const REQUEST_DELAY_MS = 500;
const PAIR_TTL_MS = 120_000;
const SAVE_TTL_MS = 300_000;
const ALLOW_GUARD_MS = 1000;
const PAIR_REQUEST_ID = "a1b2c3d4e5f60718293a4b5c6d7e8f90";
const SAVE_REQUEST_ID = "0f9e8d7c6b5a49382716f5e4d3c2b1a0";

export interface MockBrowser {
  endpoint: BrowserStatus["endpoint"];
  pairings: BrowserPairing[];
  /** Emit a pairing request after the first subscription (`&browserPair=1`). */
  pairOnStart: boolean;
  /** Emit a save request after the first subscription (`&browserSave=`). */
  saveOnStart: "new" | "update" | null;
  pairing: BrowserPairingRequest | null;
  save: BrowserSaveRequest | null;
}

export function defaultBrowser(now: () => number): MockBrowser {
  return {
    endpoint: "running",
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
    pairing: null,
    save: null,
  };
}

function statusOf(state: MockState): BrowserStatus {
  return { endpoint: state.browser.endpoint, pairings: state.browser.pairings };
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

/**
 * Called after the app subscribes and after an unlock: emits the requests the URL asked for,
 * once, 500 ms later, if the vault is unlocked by then.
 */
export function startBrowser(state: MockState): void {
  const browser = state.browser;
  if (!browser.pairOnStart && browser.saveOnStart === null) {
    return;
  }
  const pair = browser.pairOnStart;
  const save = browser.saveOnStart;
  browser.pairOnStart = false;
  browser.saveOnStart = null;
  setTimeout(() => {
    if (!state.unlocked) {
      browser.pairOnStart = pair;
      browser.saveOnStart = save;
      return;
    }
    if (pair) {
      browser.pairing = pairingRequest(state);
      state.emit({ type: "browserPairingRequested", request: browser.pairing });
    }
    if (save !== null) {
      browser.save = saveRequest(state, save);
      state.emit({ type: "browserSaveRequested", request: browser.save });
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
};
