/**
 * Dutch UI strings: clipboard. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { clipboard as en } from "../en/clipboard.ts";

export const clipboard = {
  "clipboard.copied":
    "Gekopieerd. Wordt gewist over {seconds, plural, one {# seconde} other {# seconden}}.",
  "clipboard.cleared": "Klembord gewist.",
  "toast.copied":
    "Gekopieerd. Wordt gewist over {seconds, plural, one {# seconde} other {# seconden}}.",
  "toast.copiedKept": "Gekopieerd. Mosaic wist het klembord niet.",
  "toast.clipboardCleared": "Klembord gewist.",
  "toast.saved": "Bewaard.",
  "toast.trashed": "Naar de prullenmand verplaatst.",
  "toast.restored": "Teruggezet.",
  "toast.purged": "Definitief verwijderd.",
  "clipboard.clearNow": "Nu wissen",
  "toast.dismiss": "Sluiten",
  "toast.region": "Meldingen",
} satisfies Record<keyof typeof en, string>;
