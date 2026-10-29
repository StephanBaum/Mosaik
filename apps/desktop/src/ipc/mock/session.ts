/** Mock session commands: the same state machine as `src-tauri/src/commands/session.rs`. */
import type { SessionState } from "../contract/index.ts";
import { startAgent } from "./ssh.ts";
import { fail, randomIndex, requireUnlocked, type MockHandlers, type MockState } from "./state.ts";

const SECRET_KEY_ALPHABET = "23456789ABCDEFGHJKLMNPQRSTVWXYZ";

function newSecretKey(): string {
  const groups = Array.from({ length: 7 }, () =>
    Array.from({ length: 4 }, () => SECRET_KEY_ALPHABET[randomIndex(SECRET_KEY_ALPHABET.length)]).join(
      "",
    ),
  );
  return ["M1", ...groups].join("-");
}

function normalizeKey(key: string): string {
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
      quickUnlock: [],
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
      { vaultId: state.nextId(), name: request.defaultVaultName.trim(), itemCount: 0, available: true },
    ];
    state.items = [];
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
    if (state.kitVerified && request.password !== state.account?.password) {
      fail(request.password === null ? "reauthRequired" : "wrongCredentials");
    }
    return { kind: "saved" };
  },

  verify_emergency_kit: (state, request) => {
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
    startAgent(state);
    return sessionState(state);
  },

  unlock_quick: () => fail("quickUnlockUnavailable"),

  lock: (state) => {
    if (state.unlocked) {
      state.unlocked = false;
      state.lastGenerated = null;
      state.heldImport = null;
      state.emit({ type: "locked", reason: "manual" });
    }
    return sessionState(state);
  },
};
