/**
 * Mock join of a new device (`?mock=noAccount`): `join_hosted`, `join_hosted_two_factor`,
 * `join_folder`, and `change_master_password`. A join accepts the sample account's email,
 * master password and Secret Key, then behaves like an unlock of that account.
 */
import type { JoinOutcome } from "../contract/index.ts";
import { startBrowser } from "./browser.ts";
import { sampleAccount, sampleAttachments, sampleItems, sampleVaults } from "./data.ts";
import { normalizeKey } from "./session.ts";
import { startAgent } from "./ssh.ts";
import { fail, requireUnlocked, type MockHandlers, type MockState } from "./state.ts";
import { MOCK_TOTP_CODE, normalizeServer, settle } from "./syncData.ts";

function requireNoAccount(state: MockState): void {
  if (state.account !== null) {
    fail("accountExists");
  }
}

function checkSecrets(password: string, secretKey: string): void {
  const typed = normalizeKey(secretKey);
  if (!typed.startsWith("M1") || typed.length !== 30) {
    fail("secretKeyMalformed");
  }
  if (password === "") {
    fail("emptyPassword");
  }
  if (typed !== normalizeKey(sampleAccount.secretKey) || password !== sampleAccount.password) {
    fail("wrongCredentials");
  }
}

/** The new device now holds the account: unlocked, with the sample vaults and items. */
function finishJoin(
  state: MockState,
  deviceName: string,
  backend: { kind: "hosted"; serverUrl: string } | { kind: "folder"; path: string },
): JoinOutcome {
  state.account = { ...sampleAccount };
  state.secretKeyStored = true;
  state.kitVerified = true;
  state.unlocked = true;
  state.quick.passwordUsed = true;
  state.vaults = sampleVaults();
  state.items = sampleItems();
  state.attachments = sampleAttachments();
  if (backend.kind === "hosted") {
    state.sync.registered = true;
    state.sync.totp = state.sync.join2fa;
    settle(state, { kind: "hosted", serverUrl: backend.serverUrl, deviceName });
  } else {
    settle(state, { kind: "folder", path: backend.path, deviceName });
  }
  startAgent(state);
  startBrowser(state);
  return { state: "unlocked" };
}

export const syncJoinHandlers: MockHandlers = {
  join_hosted: (state, request) => {
    requireNoAccount(state);
    const serverUrl = normalizeServer(request.serverUrl);
    if (request.email.trim().toLowerCase() !== sampleAccount.email) {
      fail("wrongCredentials");
    }
    checkSecrets(request.password, request.secretKey);
    if (state.sync.join2fa) {
      state.sync.pendingTwoFactor = { kind: "join", serverUrl, deviceName: request.deviceName };
      return { state: "twoFactorRequired" };
    }
    return finishJoin(state, request.deviceName, { kind: "hosted", serverUrl });
  },

  join_hosted_two_factor: (state, request) => {
    requireNoAccount(state);
    const pending = state.sync.pendingTwoFactor;
    if (pending?.kind !== "join") {
      fail("syncNotConfigured");
    }
    if (request.code !== MOCK_TOTP_CODE) {
      fail("syncTwoFactorInvalid");
    }
    return finishJoin(state, pending.deviceName, { kind: "hosted", serverUrl: pending.serverUrl });
  },

  join_folder: (state, request) => {
    requireNoAccount(state);
    const choice = state.sync.choices.get(request.choiceId) ?? fail("syncChoiceExpired");
    if (choice.contents !== "mosaic" || !choice.accounts.includes(request.accountId)) {
      fail("syncFolderNoAccount");
    }
    checkSecrets(request.password, request.secretKey);
    state.sync.choices.delete(request.choiceId);
    return finishJoin(state, request.deviceName, { kind: "folder", path: choice.displayPath });
  },

  change_master_password: (state, request) => {
    requireUnlocked(state);
    const account = state.account ?? fail("noAccount");
    if (request.currentPassword !== account.password) {
      fail("wrongCredentials");
    }
    if (request.newPassword === "") {
      fail("emptyPassword");
    }
    const { config, totp } = state.sync;
    if (config?.kind === "hosted" && totp && request.code !== MOCK_TOTP_CODE) {
      fail(request.code === null ? "syncTwoFactorRequired" : "syncTwoFactorInvalid");
    }
    account.password = request.newPassword;
    return null;
  },
};
