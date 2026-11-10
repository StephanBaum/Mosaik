/** Mock settings, with the re-authentication rule for weakening changes. */
import { quickUnlockMaxAgeChoices, type Settings } from "../contract/index.ts";
import { SCREEN_LOCK_DETECTION } from "./app.ts";
import { fail, type MockHandlers, type MockState } from "./state.ts";

export const defaultSettings: Settings = {
  autoLockMinutes: 10,
  lockOnSleep: true,
  lockOnScreenLock: true,
  clipboardClearSeconds: 30,
  theme: "system",
  language: "system",
  quickUnlockMaxAgeDays: 14,
  breachCheck: "off",
  breachConsentAt: null,
  rotationReminders: true,
  globalShortcut: "Ctrl+Shift+Space",
};

/** Like Rust `parse_accelerator`: modifiers plus one key, at least one of Ctrl, Alt, Cmd or Super. */
export function validAccelerator(text: string): boolean {
  const parts = text.trim().split("+").map((part) => part.trim());
  const key = parts.pop() ?? "";
  const mods = parts.map((part) => part.toLowerCase());
  const known = ["ctrl", "control", "alt", "option", "shift", "cmd", "command", "super"];
  const strong = ["ctrl", "control", "alt", "option", "cmd", "command", "super"];
  return (
    text.length <= 64 &&
    /^[A-Za-z0-9]$|^F([1-9]|1\d|2[0-4])$|^(Space|Enter|Tab|Escape|Backspace|Delete|Home|End|PageUp|PageDown|Up|Down|Left|Right)$/i.test(key) &&
    mods.every((mod) => known.includes(mod)) &&
    mods.some((mod) => strong.includes(mod))
  );
}

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
    (!next.lockOnScreenLock && old.lockOnScreenLock) ||
    next.quickUnlockMaxAgeDays > old.quickUnlockMaxAgeDays
  );
}

/** Like Rust `apply_global_shortcut`: only a changed shortcut is checked and registered. */
function checkShortcut(state: MockState, next: Settings): void {
  if (next.globalShortcut === state.settings.globalShortcut) {
    return;
  }
  if (state.options.shortcut === "unsupported") {
    fail("shortcutUnsupported");
  }
  if (!validAccelerator(next.globalShortcut)) {
    fail("invalidInput");
  }
  // `&shortcut=conflict`: another program holds every combination but the current one.
  if (state.options.shortcut !== "registered") {
    fail("shortcutUnavailable");
  }
}

export const settingsHandlers: MockHandlers = {
  get_settings: (state) => state.settings,

  update_settings: (state, request) => {
    const next = request.settings;
    if (!hasLockTrigger(next) || !quickUnlockMaxAgeChoices.some((days) => days === next.quickUnlockMaxAgeDays)) {
      fail("invalidInput");
    }
    checkShortcut(state, next);
    if (weakens(state.settings, next) && request.password !== state.account?.password) {
      fail("reauthRequired");
    }
    // Like Rust `normalize`: the mode and the consent go together, and turning the check off withdraws it.
    const consented = next.breachCheck !== "off" && next.breachConsentAt !== null;
    state.settings = {
      ...next,
      breachCheck: consented ? next.breachCheck : "off",
      breachConsentAt: consented ? next.breachConsentAt : null,
    };
    return state.settings;
  },
};
