/** Settings contract: mirrors `src-tauri/src/contract/settings.rs`. */

export const themes = ["system", "light", "dark"] as const;
export type Theme = (typeof themes)[number];

export const languages = ["system", "en", "de"] as const;
export type Language = (typeof languages)[number];

/** The choices of the quick-unlock max-age setting. */
export const quickUnlockMaxAgeChoices = [1, 3, 7, 14, 30] as const;

/** The opt-in breach check: `off` (the default), `manual` or `weekly`. */
export const breachModes = ["off", "manual", "weekly"] as const;
export type BreachMode = (typeof breachModes)[number];

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
  /** Under the settings MAC: a settings file edited outside Mosaic reads as `off`. */
  breachCheck: BreachMode;
  /** When the user agreed to the check (Unix ms); `null` exactly when `breachCheck` is `off`. */
  breachConsentAt: number | null;
  /** Show "n passwords to change" after an unlock (default on). */
  rotationReminders: boolean;
}

export interface UpdateSettingsRequest {
  settings: Settings;
  /** Required for weakening changes, else `reauthRequired`. */
  password: string | null;
}
