/**
 * Italian UI strings: generator. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { generator as en } from "../en/generator.ts";

export const generator = {
  "generator.title": "Generatore di password",
  "generator.mode": "Tipo",
  "generator.mode.password": "Caratteri",
  "generator.mode.passphrase": "Parole",
  "generator.length": "Lunghezza",
  "generator.words": "Parole",
  "generator.lowercase": "Lettere minuscole",
  "generator.uppercase": "Lettere maiuscole",
  "generator.digits": "Cifre",
  "generator.symbols": "Simboli",
  "generator.excludeAmbiguous": "Escludi i caratteri simili tra loro",
  "generator.capitalize": "Parole con l'iniziale maiuscola",
  "generator.appendDigit": "Aggiungi una cifra",
  "generator.result": "Password generata",
  "generator.entropy": "{bits} bit di entropia",
  "generator.strength.veryWeak": "Molto debole",
  "generator.strength.weak": "Debole",
  "generator.strength.fair": "Discreta",
  "generator.strength.strong": "Forte",
  "generator.strength.veryStrong": "Molto forte",
  "generator.strengthLabel": "Robustezza: {strength}",
  "generator.strengthShort": "{strength} · {bits} bit",
  "generator.minorHint": "Cifre e simboli in grigio",
  "generator.lengthValue": "{length} caratteri",
  "generator.wordsValue": "{length} parole",
  "generator.characters": "Caratteri",
} satisfies Record<keyof typeof en, string>;
