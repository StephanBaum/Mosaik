/** German UI strings: shell. Informal "du". Same keys as English (checked by the type). */
import type { shell as en } from "../en/shell.ts";

export const shell = {
  "shell.emergencyKit.title": "Mosaic Emergency Kit",
  "shell.emergencyKit.intro":
    "Bewahre dieses Blatt sicher auf, zum Beispiel in einer abschließbaren Schublade. Dein Secret Key und dein Master-Passwort entsperren gemeinsam deinen Tresor auf einem neuen Gerät. Niemand, auch nicht Mosaic, kann eins von beiden für dich wiederherstellen.",
  "shell.emergencyKit.email": "E-Mail des Kontos",
  "shell.emergencyKit.accountId": "Konto-ID",
  "shell.emergencyKit.signInUrl": "Anmeldeadresse",
  "shell.emergencyKit.signInUrlLocal": "Nur dieses Gerät (noch kein Sync-Server eingerichtet)",
  "shell.emergencyKit.secretKey": "Secret Key",
  "shell.emergencyKit.masterPassword": "Master-Passwort",
  "shell.emergencyKit.masterPasswordHint":
    "Schreib es nur dann von Hand auf die Linie, wenn dieses Blatt sicher aufbewahrt bleibt. Sonst behalte es im Kopf.",
  "shell.emergencyKit.footer":
    "Mosaic ist Pre-Audit-Software: Sie wurde noch nicht von einem externen Sicherheitsprüfer begutachtet.",
  "shell.shortcut.conflict":
    "Eine andere App nutzt {shortcut} schon. Schließ diese App oder ändere ihr Tastenkürzel und starte Mosaic dann neu, um die Schnellsuche zu nutzen.",
  "keys.ctrl": "Strg",
  "keys.shift": "Umschalt",
  "keys.alt": "Alt",
  "keys.space": "Leertaste",
} satisfies Record<keyof typeof en, string>;
