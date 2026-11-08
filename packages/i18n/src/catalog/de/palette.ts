/** German UI strings: palette. Informal "du". Same keys as English (checked by the type). */
import type { palette as en } from "../en/palette.ts";

export const palette = {
  "palette.title": "Befehlspalette",
  "palette.search": "Einträge und Befehle suchen",
  "palette.items": "Einträge",
  "palette.commands": "Befehle",
  "palette.empty": "Nichts passt. Versuche ein anderes Wort.",
  "palette.cmd.newItem": "Neuer Eintrag",
  "palette.cmd.generator": "Generator öffnen",
  "palette.cmd.security": "Sicherheitsbericht öffnen",
  "palette.cmd.settings": "Einstellungen öffnen",
  "palette.cmd.import": "Aus einer anderen App importieren",
  "palette.cmd.share": "Tresor teilen…",
  "palette.cmd.invites": "Einladungen",
  "palette.cmd.lock": "Mosaic sperren",
  "palette.open": "Befehlspalette öffnen",
  "palette.hint.move": "bewegen",
  "palette.hint.open": "öffnen",
  "palette.hint.close": "schließen",
} satisfies Record<keyof typeof en, string>;
