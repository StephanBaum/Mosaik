/** German UI strings: generator. Informal "du". Same keys as English (checked by the type). */
import type { generator as en } from "../en/generator.ts";

export const generator = {
  "generator.title": "Passwort-Generator",
  "generator.mode": "Art",
  "generator.mode.password": "Zeichen",
  "generator.mode.passphrase": "Wörter",
  "generator.length": "Länge",
  "generator.words": "Wörter",
  "generator.lowercase": "Kleinbuchstaben",
  "generator.uppercase": "Großbuchstaben",
  "generator.digits": "Ziffern",
  "generator.symbols": "Sonderzeichen",
  "generator.excludeAmbiguous": "Ähnlich aussehende Zeichen weglassen",
  "generator.capitalize": "Wörter großschreiben",
  "generator.appendDigit": "Ziffer anhängen",
  "generator.result": "Erzeugtes Passwort",
  "generator.entropy": "{bits} Bit Entropie",
  "generator.strength.veryWeak": "Sehr schwach",
  "generator.strength.weak": "Schwach",
  "generator.strength.fair": "Mittel",
  "generator.strength.strong": "Stark",
  "generator.strength.veryStrong": "Sehr stark",
  "generator.strengthLabel": "Stärke: {strength}",
} satisfies Record<keyof typeof en, string>;
