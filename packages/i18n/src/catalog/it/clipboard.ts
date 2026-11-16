// UNTRANSLATED: copy of English, replace every value
/**
 * Italian UI strings: clipboard. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { clipboard as en } from "../en/clipboard.ts";

export const clipboard = {
  "clipboard.copied": "Copied. Clears in {seconds, plural, one {# second} other {# seconds}}.",
  "clipboard.cleared": "Clipboard cleared.",
  "toast.copied": "Copied. Clears in {seconds, plural, one {# second} other {# seconds}}.",
  "toast.copiedKept": "Copied. Mosaic won't clear the clipboard.",
  "toast.clipboardCleared": "Clipboard cleared.",
  "toast.saved": "Saved.",
  "toast.trashed": "Moved to Trash.",
  "toast.restored": "Restored.",
  "toast.purged": "Deleted for good.",
  "clipboard.clearNow": "Clear now",
  "toast.dismiss": "Dismiss",
  "toast.region": "Notifications",
} satisfies Record<keyof typeof en, string>;
