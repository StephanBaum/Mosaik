/**
 * English UI strings (source locale). ICU MessageFormat syntax.
 * Every key here must exist in every other locale (enforced by the type of `de`).
 */
export const en = {
  "app.name": "Mosaic",
  "app.preAudit": "Pre-audit software: not yet reviewed by an external security auditor.",

  "action.cancel": "Cancel",
  "action.continue": "Continue",
  "action.retry": "Retry",
  "action.copy": "Copy",
  "action.reveal": "Reveal",
  "action.hide": "Hide",
  "action.lock": "Lock",
  "action.details": "Details",

  "lock.unlockTitle": "Unlock Mosaic",
  "lock.masterPassword": "Master password",
  "lock.wrongPassword":
    "That master password didn't work. Check Caps Lock and your keyboard layout, then try again.",
  "lock.locked": "Mosaic is locked.",

  "onboarding.secretKey.title": "Your Secret Key",
  "onboarding.secretKey.body":
    "Your Secret Key and master password together unlock your vault. We can't recover either of them. Save your Emergency Kit now.",
  "onboarding.secretKey.verify": "Type the last group of your Secret Key to confirm you saved it.",
  "onboarding.secretKey.mistyped": "That doesn't match. Check the last group in your Emergency Kit.",

  "clipboard.copied": "Copied. Clears in {seconds, plural, one {# second} other {# seconds}}.",
  "clipboard.cleared": "Clipboard cleared.",

  "sync.offline":
    "Couldn't reach your sync server. Your changes are saved on this device and will sync when you're back online.",

  "vault.itemCount": "{count, plural, =0 {No items} one {# item} other {# items}}",
  "vault.deleteConfirm": "Delete “{title}”? You can restore it from Trash for 30 days.",
} as const satisfies Record<string, string>;

export type MessageKey = keyof typeof en;
