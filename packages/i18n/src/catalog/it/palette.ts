/**
 * Italian UI strings: palette. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { palette as en } from "../en/palette.ts";

export const palette = {
  "palette.title": "Tavolozza dei comandi",
  "palette.search": "Cerca elementi e comandi",
  "palette.items": "Elementi",
  "palette.commands": "Comandi",
  "palette.empty": "Nessuna corrispondenza. Prova un'altra parola.",
  "palette.cmd.newItem": "Nuovo elemento",
  "palette.cmd.generator": "Apri il generatore",
  "palette.cmd.security": "Apri il rapporto di sicurezza",
  "palette.cmd.settings": "Apri le impostazioni",
  "palette.cmd.import": "Importa da un'altra app",
  "palette.cmd.share": "Condividi cassaforte…",
  "palette.cmd.invites": "Inviti",
  "palette.cmd.lock": "Blocca Mosaic",
  "palette.open": "Apri la tavolozza dei comandi",
  "palette.hint.move": "per spostarti",
  "palette.hint.open": "per aprire",
  "palette.hint.close": "per chiudere",
} satisfies Record<keyof typeof en, string>;
