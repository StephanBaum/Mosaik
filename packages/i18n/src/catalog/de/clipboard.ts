/** German UI strings: clipboard. Informal "du". Same keys as English (checked by the type). */
import type { clipboard as en } from "../en/clipboard.ts";

export const clipboard = {
  "clipboard.copied":
    "Kopiert. Wird in {seconds, plural, one {# Sekunde} other {# Sekunden}} geleert.",
  "clipboard.cleared": "Zwischenablage geleert.",
  "toast.copied":
    "Kopiert. Wird in {seconds, plural, one {# Sekunde} other {# Sekunden}} gelöscht.",
  "toast.copiedKept": "Kopiert. Mosaic leert die Zwischenablage nicht.",
  "toast.clipboardCleared": "Zwischenablage geleert.",
  "toast.saved": "Gespeichert.",
  "toast.trashed": "In den Papierkorb verschoben.",
  "toast.restored": "Wiederhergestellt.",
  "toast.purged": "Endgültig gelöscht.",
  "toast.dismiss": "Ausblenden",
  "toast.region": "Hinweise",
} satisfies Record<keyof typeof en, string>;
