/** Settings contract: mirrors `src-tauri/src/contract/settings.rs`. */

export const themes = ["system", "light", "dark"] as const;
export type Theme = (typeof themes)[number];

export const languages = ["system", "en", "de"] as const;
export type Language = (typeof languages)[number];

/** The choices of the quick-unlock max-age setting. */
export const quickUnlockMaxAgeChoices = [1, 3, 7, 14, 30] as const;

export interface Settings {
  /** 1 to 480; `null`: never (needs re-authentication). */
  autoLockMinutes: number | null;
  lockOnSleep: boolean;
  lockOnScreenLock: boolean;
  /** 10 to 300; `null`: never (needs re-authentication, shows a warning). */
  clipboardClearSeconds: number | null;
  theme: Theme;
  language: Language;
  /** Days until quick unlock asks for the master password again: 1, 3, 7, 14 or 30. */
  quickUnlockMaxAgeDays: number;
}

export interface UpdateSettingsRequest {
  settings: Settings;
  /** Required for weakening changes, else `reauthRequired`. */
  password: string | null;
}
