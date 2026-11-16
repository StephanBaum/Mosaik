/**
 * Spanish UI strings: clipboard. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { clipboard as en } from "../en/clipboard.ts";

export const clipboard = {
  "clipboard.copied": "Copiado. Se borra en {seconds, plural, one {# segundo} other {# segundos}}.",
  "clipboard.cleared": "Portapapeles borrado.",
  "toast.copied": "Copiado. Se borra en {seconds, plural, one {# segundo} other {# segundos}}.",
  "toast.copiedKept": "Copiado. Mosaic no borrará el portapapeles.",
  "toast.clipboardCleared": "Portapapeles borrado.",
  "toast.saved": "Guardado.",
  "toast.trashed": "Movido a la papelera.",
  "toast.restored": "Restaurado.",
  "toast.purged": "Eliminado definitivamente.",
  "clipboard.clearNow": "Borrar ahora",
  "toast.dismiss": "Descartar",
  "toast.region": "Notificaciones",
} satisfies Record<keyof typeof en, string>;
