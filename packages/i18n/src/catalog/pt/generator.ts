/**
 * Brazilian Portuguese UI strings: generator. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { generator as en } from "../en/generator.ts";

export const generator = {
  "generator.title": "Gerador de senhas",
  "generator.mode": "Tipo",
  "generator.mode.password": "Caracteres",
  "generator.mode.passphrase": "Palavras",
  "generator.length": "Tamanho",
  "generator.words": "Palavras",
  "generator.lowercase": "Letras minúsculas",
  "generator.uppercase": "Letras maiúsculas",
  "generator.digits": "Dígitos",
  "generator.symbols": "Símbolos",
  "generator.excludeAmbiguous": "Omitir caracteres parecidos",
  "generator.capitalize": "Capitalizar palavras",
  "generator.appendDigit": "Adicionar um dígito",
  "generator.result": "Senha gerada",
  "generator.entropy": "{bits} bits de entropia",
  "generator.strength.veryWeak": "Muito fraca",
  "generator.strength.weak": "Fraca",
  "generator.strength.fair": "Razoável",
  "generator.strength.strong": "Forte",
  "generator.strength.veryStrong": "Muito forte",
  "generator.strengthLabel": "Força: {strength}",
  "generator.strengthShort": "{strength} · {bits} bits",
  "generator.minorHint": "Dígitos e símbolos em cinza",
  "generator.lengthValue": "{length} caracteres",
  "generator.wordsValue": "{length} palavras",
  "generator.characters": "Caracteres",
} satisfies Record<keyof typeof en, string>;
