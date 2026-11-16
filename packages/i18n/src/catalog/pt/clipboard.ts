/**
 * Brazilian Portuguese UI strings: clipboard. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { clipboard as en } from "../en/clipboard.ts";

export const clipboard = {
  "clipboard.copied":
    "Copiado. Será limpo em {seconds, plural, one {# segundo} other {# segundos}}.",
  "clipboard.cleared": "Área de transferência limpa.",
  "toast.copied": "Copiado. Será limpo em {seconds, plural, one {# segundo} other {# segundos}}.",
  "toast.copiedKept": "Copiado. O Mosaic não limpará a área de transferência.",
  "toast.clipboardCleared": "Área de transferência limpa.",
  "toast.saved": "Salvo.",
  "toast.trashed": "Movido para a Lixeira.",
  "toast.restored": "Restaurado.",
  "toast.purged": "Excluído permanentemente.",
  "clipboard.clearNow": "Limpar agora",
  "toast.dismiss": "Dispensar",
  "toast.region": "Notificações",
} satisfies Record<keyof typeof en, string>;
