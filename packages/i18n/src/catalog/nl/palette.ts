/**
 * Dutch UI strings: palette. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { palette as en } from "../en/palette.ts";

export const palette = {
  "palette.title": "Opdrachtenpalet",
  "palette.search": "Zoek items en opdrachten",
  "palette.items": "Items",
  "palette.commands": "Opdrachten",
  "palette.empty": "Niets komt overeen. Probeer een ander woord.",
  "palette.cmd.newItem": "Nieuw item",
  "palette.cmd.generator": "De generator openen",
  "palette.cmd.security": "Het beveiligingsrapport openen",
  "palette.cmd.settings": "Instellingen openen",
  "palette.cmd.import": "Importeren uit een andere app",
  "palette.cmd.share": "Kluis delen…",
  "palette.cmd.invites": "Uitnodigingen",
  "palette.cmd.lock": "Mosaic vergrendelen",
  "palette.open": "Opdrachtenpalet openen",
  "palette.hint.move": "om te bewegen",
  "palette.hint.open": "om te openen",
  "palette.hint.close": "om te sluiten",
} satisfies Record<keyof typeof en, string>;
