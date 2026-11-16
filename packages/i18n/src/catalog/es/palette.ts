// UNTRANSLATED: copy of English, replace every value
/**
 * Spanish UI strings: palette. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { palette as en } from "../en/palette.ts";

export const palette = {
  "palette.title": "Command palette",
  "palette.search": "Search items and commands",
  "palette.items": "Items",
  "palette.commands": "Commands",
  "palette.empty": "Nothing matches. Try another word.",
  "palette.cmd.newItem": "New item",
  "palette.cmd.generator": "Open the generator",
  "palette.cmd.security": "Open the security report",
  "palette.cmd.settings": "Open settings",
  "palette.cmd.import": "Import from another app",
  "palette.cmd.share": "Share vault…",
  "palette.cmd.invites": "Invites",
  "palette.cmd.lock": "Lock Mosaic",
  "palette.open": "Open command palette",
  "palette.hint.move": "to move",
  "palette.hint.open": "to open",
  "palette.hint.close": "to close",
} satisfies Record<keyof typeof en, string>;
