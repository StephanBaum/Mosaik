/** Mock settings, with the re-authentication rule for weakening changes. */
import type { Settings } from "../contract/index.ts";
import { fail, type MockHandlers } from "./state.ts";

export const defaultSettings: Settings = {
  autoLockMinutes: 10,
  lockOnSleep: true,
  lockOnScreenLock: true,
  clipboardClearSeconds: 30,
  theme: "system",
  language: "system",
};

export const settingsHandlers: MockHandlers = {
  get_settings: (state) => state.settings,

  update_settings: (state, request) => {
    const next = request.settings;
    const weakens =
      (next.autoLockMinutes === null && state.settings.autoLockMinutes !== null) ||
      (next.clipboardClearSeconds === null && state.settings.clipboardClearSeconds !== null);
    if (weakens && request.password !== state.account?.password) {
      fail("reauthRequired");
    }
    state.settings = { ...next };
    return state.settings;
  },
};
