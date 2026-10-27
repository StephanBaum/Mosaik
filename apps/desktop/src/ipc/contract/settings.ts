/** Settings contract: mirrors `src-tauri/src/contract/settings.rs`. */

export const themes = ["system", "light", "dark"] as const;
export type Theme = (typeof themes)[number];

export const languages = ["system", "en", "de"] as const;
export type Language = (typeof languages)[number];

export interface Settings {
  /** 1 to 480; `null`: never (needs re-authentication). */
  autoLockMinutes: number | null;
  lockOnSleep: boolean;
  lockOnScreenLock: boolean;
  /** 10 to 300; `null`: never (needs re-authentication, shows a warning). */
  clipboardClearSeconds: number | null;
  theme: Theme;
  language: Language;
}

export interface UpdateSettingsRequest {
  settings: Settings;
  /** Required for weakening changes, else `reauthRequired`. */
  password: string | null;
}
