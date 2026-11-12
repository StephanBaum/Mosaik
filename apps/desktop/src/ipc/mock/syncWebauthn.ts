/**
 * Mock security keys for the sync account (M9): the same rules as
 * `src-tauri/src/sync/webauthn_flow.rs` where the UI can see them. A browser step ends on its
 * second status poll (the user "touches the key"); `&syncHandoff=expired|failed` ends it badly.
 * `&syncWebauthn=both|only` gives the account a key (next to a code, or instead of one).
 */
import { sampleAccount } from "./data.ts";
import { fail, requireUnlocked, type MockHandlers, type MockState } from "./state.ts";
import { finishJoin } from "./syncJoin.ts";
import { MOCK_TOTP_CODE, publish, settle } from "./syncData.ts";

const MATCH_CODE = "482913";
const HANDOFF_MS = 300_000;
const CONFIRM_MS = 300_000;
const MAX_KEYS = 10;

function begin(state: MockState) {
  return { matchCode: MATCH_CODE, expiresAt: state.now() + HANDOFF_MS };
}

/** The sync account's key count into the overview. */
function syncKeyCount(state: MockState): void {
  const { config } = state.sync;
  if (config?.kind === "hosted") {
    state.sync.config = { ...config, webauthnKeys: state.sync.keys.length };
  }
}

function checkPassword(state: MockState, password: string): void {
  if (password !== (state.account?.password ?? sampleAccount.password)) {
    fail("wrongCredentials");
  }
}

/** The state a handoff reports: `done` only after the second poll, unless the URL says otherwise. */
function poll(state: MockState): "waiting" | "done" | "failed" | "expired" {
  const { handoff } = state.sync;
  if (handoff === null) {
    return fail("syncHandoffExpired");
  }
  handoff.polls += 1;
  if (state.sync.handoffEnd !== "done") {
    state.sync.handoff = null;
    return state.sync.handoffEnd;
  }
  return handoff.polls >= 2 ? "done" : "waiting";
}

/** Starts an account flow: a login step first when the account asks for a second factor. */
function startAccountFlow(
  state: MockState,
  then: "register" | "confirm",
  label: string | null,
  code: string | null,
) {
  const { sync } = state;
  const fresh = state.now() < sync.confirmedUntil;
  const needsFactor = (sync.totp || sync.keys.length > 0) && !fresh;
  if (needsFactor && code !== null && code !== "") {
    if (code !== MOCK_TOTP_CODE) {
      fail("syncTwoFactorInvalid");
    }
  } else if (needsFactor) {
    if (sync.keys.length === 0) {
      fail("syncTwoFactorRequired");
    }
    sync.handoff = { step: "login", polls: 0, then, label };
    return { step: "login" as const, ...begin(state) };
  }
  sync.handoff = { step: then, polls: 0, then: null, label };
  return { step: then, ...begin(state) };
}

export const syncWebauthnHandlers: MockHandlers = {
  sync_login_methods: (state) => {
    const pending = state.sync.pendingTwoFactor;
    if (pending === null) {
      fail("syncSignedOut");
    }
    // A joining device does not know the account yet: the seed says what the account has.
    return pending.kind === "join"
      ? { totp: state.sync.webauthnMode !== "only", webauthn: state.sync.webauthnMode !== "none" }
      : { totp: state.sync.totp, webauthn: state.sync.keys.length > 0 };
  },

  sync_hosted_webauthn_begin: (state) => {
    const pending = state.sync.pendingTwoFactor;
    if (pending?.kind !== "connect") {
      fail("syncSignedOut");
    }
    if (state.sync.keys.length === 0) {
      fail("syncWebauthnUnavailable");
    }
    state.sync.handoff = { step: "login", polls: 0, then: null, label: null };
    return begin(state);
  },

  join_hosted_webauthn_begin: (state) => {
    if (state.account !== null) {
      fail("accountExists");
    }
    const pending = state.sync.pendingTwoFactor;
    if (pending?.kind !== "join") {
      fail("syncSignedOut");
    }
    if (state.sync.webauthnMode === "none") {
      fail("syncWebauthnUnavailable");
    }
    state.sync.handoff = { step: "login", polls: 0, then: null, label: null };
    return begin(state);
  },

  sync_webauthn_status: (state) => {
    const pending = state.sync.pendingTwoFactor;
    if (pending === null) {
      fail("syncSignedOut");
    }
    const answer = poll(state);
    if (answer !== "done") {
      return { state: answer };
    }
    state.sync.handoff = null;
    if (pending.kind === "connect") {
      settle(state, { kind: "hosted", serverUrl: pending.serverUrl, deviceName: pending.deviceName });
    } else {
      finishJoin(state, pending.deviceName, { kind: "hosted", serverUrl: pending.serverUrl });
    }
    return { state: "done" };
  },

  sync_webauthn_reopen: (state) => {
    if (state.sync.handoff === null) {
      fail("syncHandoffExpired");
    }
    return null;
  },

  sync_webauthn_list: (state) => {
    requireUnlocked(state);
    if (state.sync.config?.kind !== "hosted") {
      fail("syncNotConfigured");
    }
    return { totp: state.sync.totp, keys: state.sync.keys };
  },

  sync_webauthn_add_begin: (state, request) => {
    requireUnlocked(state);
    if (state.sync.config?.kind !== "hosted") {
      fail("syncNotConfigured");
    }
    const label = request.label.trim();
    if (label === "" || label.length > 64) {
      fail("invalidInput");
    }
    checkPassword(state, request.password);
    if (state.sync.keys.length >= MAX_KEYS) {
      fail("syncWebauthnLimit");
    }
    return startAccountFlow(state, "register", label, request.code);
  },

  sync_webauthn_confirm_begin: (state, request) => {
    requireUnlocked(state);
    if (state.sync.config?.kind !== "hosted") {
      fail("syncNotConfigured");
    }
    checkPassword(state, request.password);
    return startAccountFlow(state, "confirm", null, null);
  },

  sync_webauthn_add_status: (state) => {
    requireUnlocked(state);
    const { sync } = state;
    const step = sync.handoff?.step ?? "register";
    const answer = poll(state);
    if (answer !== "done" || sync.handoff === null) {
      return { state: answer, step, matchCode: null, expiresAt: null };
    }
    const { handoff } = sync;
    if (handoff.step === "login" && handoff.then !== null) {
      sync.handoff = { step: handoff.then, polls: 0, then: null, label: handoff.label };
      return { state: "waiting", step: handoff.then, matchCode: MATCH_CODE, expiresAt: state.now() + HANDOFF_MS };
    }
    sync.handoff = null;
    if (handoff.step === "register") {
      const suffix = (sync.keys.length + 1).toString(16).padStart(2, "0");
      sync.keys = [
        ...sync.keys,
        {
          credentialRef: `${suffix}${"ab".repeat(15)}`,
          label: handoff.label ?? "Security key",
          createdAt: state.now(),
          lastUsedAt: null,
        },
      ];
      syncKeyCount(state);
      publish(state);
    } else {
      sync.confirmedUntil = state.now() + CONFIRM_MS;
    }
    return { state: "done", step: handoff.step, matchCode: null, expiresAt: null };
  },

  sync_webauthn_remove: (state, request) => {
    requireUnlocked(state);
    const { sync } = state;
    if (!/^[0-9a-f]{32}$/.test(request.credentialRef)) {
      fail("invalidInput");
    }
    const key = sync.keys.find((k) => k.credentialRef === request.credentialRef);
    if (key === undefined) {
      fail("invalidInput");
    }
    if (request.code !== null && request.code !== "") {
      if (request.code !== MOCK_TOTP_CODE) {
        fail("syncTwoFactorInvalid");
      }
    } else if (state.now() >= sync.confirmedUntil) {
      fail("syncTwoFactorRequired");
    }
    sync.keys = sync.keys.filter((k) => k !== key);
    syncKeyCount(state);
    publish(state);
    return null;
  },
};
