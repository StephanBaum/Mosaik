/** Mock settings, with the re-authentication rule for weakening changes. */
import type { Settings } from "../contract/index.ts";
import { SCREEN_LOCK_DETECTION } from "./app.ts";
import { fail, type MockHandlers } from "./state.ts";

export const defaultSettings: Settings = {
  autoLockMinutes: 10,
  lockOnSleep: true,
  lockOnScreenLock: true,
  clipboardClearSeconds: 30,
  theme: "system",
  language: "system",
};

/** Like Rust `has_lock_trigger`: screen lock counts only where the system detects it. */
function hasLockTrigger(s: Settings): boolean {
  return s.autoLockMinutes !== null || s.lockOnSleep || (s.lockOnScreenLock && SCREEN_LOCK_DETECTION);
}

/** Like Rust `weakens`: turning a protection off needs the master password. */
function weakens(old: Settings, next: Settings): boolean {
  return (
    (next.autoLockMinutes === null && old.autoLockMinutes !== null) ||
    (next.clipboardClearSeconds === null && old.clipboardClearSeconds !== null) ||
    (!next.lockOnSleep && old.lockOnSleep) ||
    (!next.lockOnScreenLock && old.lockOnScreenLock)
  );
}

export const settingsHandlers: MockHandlers = {
  get_settings: (state) => state.settings,

  update_settings: (state, request) => {
    const next = request.settings;
    if (!hasLockTrigger(next)) {
      fail("invalidInput");
    }
    if (weakens(state.settings, next) && request.password !== state.account?.password) {
      fail("reauthRequired");
    }
    state.settings = { ...next };
    return state.settings;
  },
};
