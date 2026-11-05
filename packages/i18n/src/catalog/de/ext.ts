/** German UI strings: browser extension (manifest and menu strings). Informal "du". Same keys as English (checked by the type). */
import type { ext as en } from "../en/ext.ts";

export const ext = {
  "ext.manifest.name": "Mosaic",
  "ext.manifest.description":
    "Füllt Logins aus deinem Mosaic-Tresor aus. Dein Master-Passwort bleibt in der Desktop-App.",
  "ext.command.fill": "Login mit Mosaic ausfüllen",
  "ext.command.generate": "Passwort mit Mosaic erzeugen",
} satisfies Record<keyof typeof en, string>;
