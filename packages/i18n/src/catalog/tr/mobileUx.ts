// UNTRANSLATED: copy of English, replace every value
/**
 * Turkish UI strings: mobileUx. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { mobileUx as en } from "../en/mobileUx.ts";

export const mobileUx = {
  "mobile.sync.warningsBanner": "Sync found a problem. Open Settings to check it.",
  "mobile.provider.errorTitle": "Mosaic can't continue",
  "mobile.onboarding.details.keyWiped":
    "Mosaic removed the scanned Secret Key when you left the app. Scan the code again or type the key.",
  "mobile.common.showField": "Show {field}",
  "mobile.common.hideField": "Hide {field}",
  "mobile.error.storeInUse":
    "Mosaic is busy elsewhere on this phone. Wait a moment, then try again.",
  "mobile.error.noAccount": "Mosaic isn't set up on this phone yet. Open Mosaic to set it up.",
  "mobile.error.wrongCredentials":
    "That master password or Secret Key didn't work. Check for typos and the keyboard language, then try again.",
  "mobile.error.invalidInput":
    "Something you entered isn't valid. Check your entries and try again.",
  "mobile.error.vaultUnavailable":
    "This vault can't be opened right now. Sync and try again, or unlock Mosaic again.",
  "mobile.unlock.quickOptionTouchId": "Unlock with Touch ID next time",
  "mobile.detail.readCode": "Read code",
  "mobile.detail.labelValue": "{label}: {value}",
  "mobile.autofill.warningWithName": "Check the address first | {name}",
  "mobile.autofill.linkEntry": "Link {login} and fill",
  "mobile.link.imported": "Imported for this app",
  "mobile.link.confirmTitle": "Link {login} to {app}?",
  "mobile.link.confirmSubtitle": "App ID {appId} · Signature {fingerprint}",
  "mobile.link.confirmFallbackHint": "After linking, tap the field again to fill it.",
  "mobile.link.importedNotVerified": "Imported login. The website has not confirmed this app.",
} satisfies Record<keyof typeof en, string>;
