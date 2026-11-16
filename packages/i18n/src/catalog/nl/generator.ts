/**
 * Dutch UI strings: generator. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { generator as en } from "../en/generator.ts";

export const generator = {
  "generator.title": "Wachtwoordgenerator",
  "generator.mode": "Type",
  "generator.mode.password": "Tekens",
  "generator.mode.passphrase": "Woorden",
  "generator.length": "Lengte",
  "generator.words": "Woorden",
  "generator.lowercase": "Kleine letters",
  "generator.uppercase": "Hoofdletters",
  "generator.digits": "Cijfers",
  "generator.symbols": "Symbolen",
  "generator.excludeAmbiguous": "Op elkaar lijkende tekens weglaten",
  "generator.capitalize": "Woorden met hoofdletter",
  "generator.appendDigit": "Een cijfer toevoegen",
  "generator.result": "Gegenereerd wachtwoord",
  "generator.entropy": "{bits} bits entropie",
  "generator.strength.veryWeak": "Zeer zwak",
  "generator.strength.weak": "Zwak",
  "generator.strength.fair": "Redelijk",
  "generator.strength.strong": "Sterk",
  "generator.strength.veryStrong": "Zeer sterk",
  "generator.strengthLabel": "Sterkte: {strength}",
  "generator.strengthShort": "{strength} · {bits} bits",
  "generator.minorHint": "Cijfers en symbolen in grijs",
  "generator.lengthValue": "{length} tekens",
  "generator.wordsValue": "{length} woorden",
  "generator.characters": "Tekens",
} satisfies Record<keyof typeof en, string>;
