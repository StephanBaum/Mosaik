/**
 * French UI strings: clipboard. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { clipboard as en } from "../en/clipboard.ts";

export const clipboard = {
  "clipboard.copied": "Copié. Effacement dans {seconds, plural, one {# seconde} other {# secondes}}.",
  "clipboard.cleared": "Presse-papiers effacé.",
  "toast.copied": "Copié. Effacement dans {seconds, plural, one {# seconde} other {# secondes}}.",
  "toast.copiedKept": "Copié. Mosaic n’effacera pas le presse-papiers.",
  "toast.clipboardCleared": "Presse-papiers effacé.",
  "toast.saved": "Enregistré.",
  "toast.trashed": "Déplacé dans la corbeille.",
  "toast.restored": "Restauré.",
  "toast.purged": "Supprimé définitivement.",
  "clipboard.clearNow": "Effacer maintenant",
  "toast.dismiss": "Fermer",
  "toast.region": "Notifications",
} satisfies Record<keyof typeof en, string>;
