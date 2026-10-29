/** English UI strings: unlock. ICU MessageFormat. */
export const unlock = {
  "lock.unlockTitle": "Unlock Mosaic",
  "lock.masterPassword": "Master password",
  "lock.wrongPassword":
    "That master password didn't work. Check Caps Lock and your keyboard layout, then try again.",
  "lock.locked": "Mosaic is locked.",
  "lock.secretKey": "Secret Key",
  "lock.secretKeyHint":
    "This device doesn't have your Secret Key yet. Type it from your Emergency Kit, and Mosaic remembers it on this device.",
  "lock.submit": "Unlock",
  "lock.working": "Unlocking…",
  "lock.reason.manual": "You locked Mosaic.",
  "lock.reason.idle": "Mosaic locked itself after a while without activity.",
  "lock.reason.sleep": "Mosaic locked when the computer went to sleep.",
  "lock.reason.screenLock": "Mosaic locked together with your screen.",
} as const satisfies Record<string, string>;
