/**
 * Polish UI strings: clipboard. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { clipboard as en } from "../en/clipboard.ts";

export const clipboard = {
  "clipboard.copied":
    "Skopiowano. Zostanie wyczyszczone za {seconds, plural, one {# sekundę} few {# sekundy} many {# sekund} other {# sekundy}}.",
  "clipboard.cleared": "Schowek wyczyszczony.",
  "toast.copied":
    "Skopiowano. Zostanie wyczyszczone za {seconds, plural, one {# sekundę} few {# sekundy} many {# sekund} other {# sekundy}}.",
  "toast.copiedKept": "Skopiowano. Mosaic nie wyczyści schowka.",
  "toast.clipboardCleared": "Schowek wyczyszczony.",
  "toast.saved": "Zapisano.",
  "toast.trashed": "Przeniesiono do kosza.",
  "toast.restored": "Przywrócono.",
  "toast.purged": "Usunięto na zawsze.",
  "clipboard.clearNow": "Wyczyść teraz",
  "toast.dismiss": "Zamknij",
  "toast.region": "Powiadomienia",
} satisfies Record<keyof typeof en, string>;
