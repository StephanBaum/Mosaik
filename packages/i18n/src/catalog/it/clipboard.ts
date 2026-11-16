/**
 * Italian UI strings: clipboard. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { clipboard as en } from "../en/clipboard.ts";

export const clipboard = {
  "clipboard.copied":
    "Copiato. Si cancella tra {seconds, plural, one {# secondo} other {# secondi}}.",
  "clipboard.cleared": "Appunti cancellati.",
  "toast.copied": "Copiato. Si cancella tra {seconds, plural, one {# secondo} other {# secondi}}.",
  "toast.copiedKept": "Copiato. Mosaic non cancellerà gli appunti.",
  "toast.clipboardCleared": "Appunti cancellati.",
  "toast.saved": "Salvato.",
  "toast.trashed": "Spostato nel Cestino.",
  "toast.restored": "Ripristinato.",
  "toast.purged": "Eliminato definitivamente.",
  "clipboard.clearNow": "Cancella ora",
  "toast.dismiss": "Chiudi",
  "toast.region": "Notifiche",
} satisfies Record<keyof typeof en, string>;
