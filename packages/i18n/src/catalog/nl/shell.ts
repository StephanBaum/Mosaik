/**
 * Dutch UI strings: shell. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { shell as en } from "../en/shell.ts";

export const shell = {
  "shell.emergencyKit.title": "Mosaic Noodkit",
  "shell.emergencyKit.intro":
    "Bewaar deze pagina op een veilige plek, zoals een afgesloten la. Je Geheime Sleutel en je hoofdwachtwoord ontgrendelen samen je kluis op een nieuw apparaat. Niemand, ook Mosaic niet, kan een van beide voor je herstellen.",
  "shell.emergencyKit.email": "E-mail van het account",
  "shell.emergencyKit.accountId": "Account-ID",
  "shell.emergencyKit.signInUrl": "Inlogadres",
  "shell.emergencyKit.signInUrlLocal":
    "Alleen dit apparaat (nog geen synchronisatieserver ingesteld)",
  "shell.emergencyKit.secretKey": "Geheime Sleutel",
  "shell.emergencyKit.masterPassword": "Hoofdwachtwoord",
  "shell.emergencyKit.masterPasswordHint":
    "Schrijf het alleen met de hand op de regel als deze pagina op een veilige plek blijft. Onthoud het anders in je hoofd.",
  "shell.emergencyKit.footer":
    "Mosaic is software vóór een audit: het is nog niet beoordeeld door een externe beveiligingsauditor.",
  "shell.shortcut.conflict":
    "Een andere app gebruikt {shortcut} al. Sluit die app of wijzig zijn sneltoets en start Mosaic daarna opnieuw om snel zoeken te gebruiken.",
  "keys.ctrl": "Ctrl",
  "keys.shift": "Shift",
  "keys.alt": "Alt",
  "keys.space": "Spatie",
  "keys.esc": "Esc",
  "keys.enter": "Enter",
} satisfies Record<keyof typeof en, string>;
