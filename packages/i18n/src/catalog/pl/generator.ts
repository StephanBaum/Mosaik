/**
 * Polish UI strings: generator. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { generator as en } from "../en/generator.ts";

export const generator = {
  "generator.title": "Generator haseł",
  "generator.mode": "Rodzaj",
  "generator.mode.password": "Znaki",
  "generator.mode.passphrase": "Słowa",
  "generator.length": "Długość",
  "generator.words": "Słowa",
  "generator.lowercase": "Małe litery",
  "generator.uppercase": "Wielkie litery",
  "generator.digits": "Cyfry",
  "generator.symbols": "Symbole",
  "generator.excludeAmbiguous": "Pomijaj podobnie wyglądające znaki",
  "generator.capitalize": "Wielkie litery na początku słów",
  "generator.appendDigit": "Dodaj cyfrę",
  "generator.result": "Wygenerowane hasło",
  "generator.entropy": "{bits} bitów entropii",
  "generator.strength.veryWeak": "Bardzo słabe",
  "generator.strength.weak": "Słabe",
  "generator.strength.fair": "Przeciętne",
  "generator.strength.strong": "Mocne",
  "generator.strength.veryStrong": "Bardzo mocne",
  "generator.strengthLabel": "Siła: {strength}",
  "generator.strengthShort": "{strength} · {bits} bitów",
  "generator.minorHint": "Cyfry i symbole na szaro",
  "generator.lengthValue": "{length} znaków",
  "generator.wordsValue": "{length} słów",
  "generator.characters": "Znaki",
} satisfies Record<keyof typeof en, string>;
