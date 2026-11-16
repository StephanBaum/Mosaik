/**
 * Polish UI strings: palette. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { palette as en } from "../en/palette.ts";

export const palette = {
  "palette.title": "Paleta poleceń",
  "palette.search": "Szukaj elementów i poleceń",
  "palette.items": "Elementy",
  "palette.commands": "Polecenia",
  "palette.empty": "Nic nie pasuje. Spróbuj innego słowa.",
  "palette.cmd.newItem": "Nowy element",
  "palette.cmd.generator": "Otwórz generator",
  "palette.cmd.security": "Otwórz raport bezpieczeństwa",
  "palette.cmd.settings": "Otwórz ustawienia",
  "palette.cmd.import": "Importuj z innej aplikacji",
  "palette.cmd.share": "Udostępnij sejf…",
  "palette.cmd.invites": "Zaproszenia",
  "palette.cmd.lock": "Zablokuj Mosaic",
  "palette.open": "Otwórz paletę poleceń",
  "palette.hint.move": "aby się poruszać",
  "palette.hint.open": "aby otworzyć",
  "palette.hint.close": "aby zamknąć",
} satisfies Record<keyof typeof en, string>;
