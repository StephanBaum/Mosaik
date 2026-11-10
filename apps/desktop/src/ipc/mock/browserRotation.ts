/**
 * Mock password-change dialogs of the browser extension (M7b, `&credRotation=browserBegin|
 * browserResult|addOrigin`). Like Rust (`src-tauri/src/browser/rotation_prompt.rs`): nothing is
 * stored before the answer, the generated password is fetched with `browser_rotation_reveal`,
 * "generate another" restarts the 1 s Allow guard, and keeping the old password or cancelling is
 * never delayed.
 */
import type { BrowserRotationDecision, BrowserRotationKind, BrowserRotationRequest } from "../contract/index.ts";
import { closeAttempt, refreshDetail, startAttempt } from "./credentialRotation.ts";
import { fail, requireUnlocked, type MockHandlers, type MockState } from "./state.ts";

const ALLOW_GUARD_MS = 1000;

const ROTATION_REQUEST_ID = "7c6b5a4d3e2f1a0b9c8d7e6f5a4b3c2d";
const ROTATION_TTL_MS = 300_000;
/** The related address the `addOrigin` variant asks to add (GitHub's). */
const RELATED_ORIGIN = "https://gist.github.com";
const GENERATED_PASSWORDS: readonly string[] = [
  "q7#Vt2!mZp9@Lx4$Nw6&",
  "Hd3!kP8$wQ5#zR2&mT7@",
  "Bn6@xC9!vL4$jF2#sY8&",
];

export function rotationRequest(state: MockState, kind: BrowserRotationKind): BrowserRotationRequest | null {
  const github = state.items.find((item) => item.detail.summary.title === "GitHub");
  if (github === undefined) {
    return null;
  }
  const requestedAt = state.now();
  return {
    requestId: ROTATION_REQUEST_ID,
    pairingLabel: "Chrome",
    kind,
    vaultId: github.detail.summary.vaultId,
    itemId: github.detail.summary.itemId,
    title: github.detail.summary.title,
    host: kind === "addOrigin" ? new URL(RELATED_ORIGIN).hostname : "github.com",
    unicodeHost: null,
    outcome: kind === "result" ? "succeeded" : null,
    newOrigin: kind === "addOrigin" ? RELATED_ORIGIN : null,
    attempt: kind === "result" ? { startedAt: requestedAt - 20 * 60_000, stage: "delivered", otherOpen: 1 } : null,
    requestedAt,
    expiresAt: requestedAt + ROTATION_TTL_MS,
  };
}

/** The password Rust "generated" for the open request: the sample list in turn. */
function generatedPassword(state: MockState): string {
  return GENERATED_PASSWORDS[state.browser.rotationGenerated % GENERATED_PASSWORDS.length] ?? "";
}

/** Whether the decision fits the kind of the open request (`cancel` fits all). */
function rotationDecisionFits(open: BrowserRotationRequest, decision: BrowserRotationDecision): boolean {
  switch (decision.kind) {
    case "cancel":
      return true;
    case "allow":
    case "regenerate":
      return open.kind === "begin";
    case "confirm":
      return open.kind === "result" && open.outcome === "succeeded";
    case "keepOld":
      return open.kind === "result";
    case "addOrigin":
      return open.kind === "addOrigin";
  }
}

/** Writes what a decision writes: the attempt, the closed attempt or the added address. */
function commitRotation(state: MockState, open: BrowserRotationRequest, decision: BrowserRotationDecision): void {
  const item = state.items.find((each) => each.detail.summary.itemId === open.itemId);
  if (item === undefined) {
    fail("itemNotFound");
  }
  const attempt = item.detail.rotationAttempts[0];
  if (decision.kind === "allow") {
    if (item.detail.rotationAttempts.length >= 4) {
      fail("rotationLimit");
    }
    startAttempt(state, item, generatedPassword(state), "prepared");
  } else if (decision.kind === "confirm" || decision.kind === "keepOld") {
    if (attempt === undefined) {
      fail("rotationNotFound");
    }
    closeAttempt(state, item, attempt.attemptId, decision.kind === "confirm" ? "succeeded" : "failed");
  } else if (decision.kind === "addOrigin" && open.newOrigin !== null) {
    item.detail.urls.push({
      elementId: `${item.detail.summary.itemId}-url-${String(item.detail.urls.length)}`,
      url: open.newOrigin,
      matchMode: "exactOrigin",
      allowHttp: false,
    });
    refreshDetail(item);
  }
}

export const rotationHandlers: MockHandlers = {
  browser_rotation_respond: (state, request) => {
    requireUnlocked(state);
    const browser = state.browser;
    const open = browser.rotation;
    if (open === null || open.requestId !== request.requestId) {
      fail("browserRequestExpired");
    }
    const decision = request.decision;
    if (!rotationDecisionFits(open, decision)) {
      fail("invalidInput");
    }
    if (decision.kind === "regenerate") {
      // A new password restarts the Allow guard, like in Rust.
      browser.rotationGenerated += 1;
      browser.rotationShownAt = state.now();
      return null;
    }
    // Cancel and keeping the old password are never delayed.
    if (decision.kind !== "cancel" && decision.kind !== "keepOld") {
      if (state.now() - browser.rotationShownAt < ALLOW_GUARD_MS) {
        fail("browserAllowTooSoon");
      }
    }
    if (decision.kind !== "cancel") {
      commitRotation(state, open, decision);
    }
    browser.rotation = null;
    state.emit({ type: "browserRotationClosed", requestId: open.requestId, reason: "answered" });
    return null;
  },

  browser_rotation_reveal: (state, request) => {
    requireUnlocked(state);
    const open = state.browser.rotation;
    if (open === null || open.requestId !== request.requestId) {
      fail("browserRequestExpired");
    }
    if (open.kind !== "begin") {
      fail("invalidInput");
    }
    return { value: generatedPassword(state), hideAfterSeconds: 15 };
  },
};

/** The browser dialog a credential rotation variant asks for. */
export function rotationOnStartOf(variant: string | undefined): BrowserRotationKind | null {
  switch (variant) {
    case "browserBegin":
      return "begin";
    case "browserResult":
      return "result";
    case "addOrigin":
      return "addOrigin";
    default:
      return null;
  }
}
