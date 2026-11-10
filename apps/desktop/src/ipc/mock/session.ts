/** Mock session commands: the same state machine as `src-tauri/src/commands/session.rs`. */
import type { SessionState } from "../contract/index.ts";
import {
  closeHardwareVault,
  MOCK_REAUTH_MS,
  quickMethods,
  verifyQuickUnlock,
} from "./quickUnlock.ts";
import { clearMockPending } from "./otp.ts";
import { startBrowser } from "./browser.ts";
import { startAgent } from "./ssh.ts";
import { announceSync } from "./syncData.ts";
import { fail, randomIndex, requireUnlocked, type MockAsyncHandlers,
  type MockHandlers,
  type MockState,
} from "./state.ts";

const SECRET_KEY_ALPHABET = "23456789ABCDEFGHJKLMNPQRSTVWXYZ";

export function newSecretKey(): string {
  const groups = Array.from({ length: 7 }, () =>
    Array.from({ length: 4 }, () => SECRET_KEY_ALPHABET[randomIndex(SECRET_KEY_ALPHABET.length)]).join(
      "",
    ),
  );
  return ["M1", ...groups].join("-");
}

export function normalizeKey(key: string): string {
  return key.toUpperCase().replace(/[\s-]/g, "");
}

export function sessionState(state: MockState): SessionState {
  if (state.account === null) {
    return { state: "noAccount" };
  }
  if (!state.unlocked) {
    return {
      state: "locked",
      email: state.account.email,
      secretKeyStored: state.secretKeyStored,
      quickUnlock: quickMethods(state),
    };
  }
  return {
    state: "unlocked",
    accountId: state.account.accountId,
    email: state.account.email,
    kitVerified: state.kitVerified,
  };
}

export const sessionHandlers: MockHandlers = {
  session_state: (state) => sessionState(state),

  create_account: (state, request) => {
    if (state.account !== null) {
      fail("accountExists");
    }
    const email = request.email.trim();
    if (!email.includes("@") || request.defaultVaultName.trim() === "") {
      fail("invalidInput");
    }
    if (request.password === "") {
      fail("emptyPassword");
    }
    state.account = {
      accountId: state.nextId(),
      email,
      password: request.password,
      secretKey: newSecretKey(),
    };
    state.secretKeyStored = true;
    state.unlocked = true;
    state.kitVerified = false;
    state.vaults = [
      {
        vaultId: state.nextId(),
        name: request.defaultVaultName.trim(),
        itemCount: 0,
        available: true,
        protection: "accountKey",
        openUntil: null,
      },
    ];
    state.items = [];
    // A fresh account is unlocked: the browser request the URL asked for may follow (onboarding D06).
    startBrowser(state);
    return {
      emergencyKit: {
        accountId: state.account.accountId,
        email,
        secretKey: state.account.secretKey,
      },
      session: sessionState(state),
    };
  },

  save_emergency_kit: (state, request) => {
    requireUnlocked(state);
    if (request.pending) {
      // The new kit of a rotation: no password, the key is new.
      return state.rotation.pendingKey === null ? fail("rotationNone") : { kind: "saved" };
    }
    if (state.kitVerified && request.password !== state.account?.password) {
      fail(request.password === null ? "reauthRequired" : "wrongCredentials");
    }
    return { kind: "saved" };
  },

  verify_emergency_kit: (state, request) => {
    if (request.pending) {
      const pending = state.rotation.pendingKey ?? fail("rotationNone");
      const matches = normalizeKey(request.lastGroup) === normalizeKey(pending).slice(-4);
      state.rotation.kitVerified = state.rotation.kitVerified || matches;
      return { matches };
    }
    if (state.account === null || !state.secretKeyStored) {
      fail("secretKeyRequired");
    }
    const lastGroup = state.account.secretKey.split("-").at(-1) ?? "";
    const matches = normalizeKey(request.lastGroup) === lastGroup;
    state.kitVerified = state.kitVerified || matches;
    return { matches };
  },

  unlock: (state, request) => {
    const account = state.account ?? fail("noAccount");
    if (state.unlocked) {
      fail("alreadyUnlocked");
    }
    if (request.secretKey === null && !state.secretKeyStored) {
      fail("secretKeyRequired");
    }
    if (request.secretKey !== null) {
      const typed = normalizeKey(request.secretKey);
      if (!typed.startsWith("M1") || typed.length !== 30) {
        fail("secretKeyMalformed");
      }
      if (typed !== normalizeKey(account.secretKey)) {
        fail("wrongCredentials");
      }
    }
    if (request.password !== account.password) {
      fail("wrongCredentials");
    }
    state.unlocked = true;
    state.secretKeyStored = true;
    state.quick.passwordUsed = true;
    startAgent(state);
    startBrowser(state);
    announceSync(state);
    return sessionState(state);
  },

  reauthenticate: (state, request) => {
    requireUnlocked(state);
    if (request.password !== state.account?.password) {
      fail("wrongCredentials");
    }
    state.quick.reauthUntil = state.now() + MOCK_REAUTH_MS;
    return null;
  },

  lock: (state) => {
    if (state.unlocked) {
      state.unlocked = false;
      state.quick.reauthUntil = 0;
      for (const vault of state.hardwareVaults) {
        closeHardwareVault(state, vault, "lock");
      }
      state.lastGenerated = null;
      clearMockPending(state);
      state.heldImport = null;
      state.emit({ type: "locked", reason: "manual" });
    }
    return sessionState(state);
  },
};

export const sessionAsyncHandlers: MockAsyncHandlers = {
  unlock_quick: async (state, request) => {
    await verifyQuickUnlock(state, request);
    state.unlocked = true;
    startAgent(state);
    startBrowser(state);
    announceSync(state);
    return sessionState(state);
  },
};
