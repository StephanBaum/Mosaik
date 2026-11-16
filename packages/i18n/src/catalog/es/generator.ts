/**
 * Spanish UI strings: generator. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { generator as en } from "../en/generator.ts";

export const generator = {
  "generator.title": "Generador de contraseñas",
  "generator.mode": "Tipo",
  "generator.mode.password": "Caracteres",
  "generator.mode.passphrase": "Palabras",
  "generator.length": "Longitud",
  "generator.words": "Palabras",
  "generator.lowercase": "Letras minúsculas",
  "generator.uppercase": "Letras mayúsculas",
  "generator.digits": "Dígitos",
  "generator.symbols": "Símbolos",
  "generator.excludeAmbiguous": "Omitir caracteres que se parecen",
  "generator.capitalize": "Poner las palabras en mayúscula inicial",
  "generator.appendDigit": "Añadir un dígito",
  "generator.result": "Contraseña generada",
  "generator.entropy": "{bits} bits de entropía",
  "generator.strength.veryWeak": "Muy débil",
  "generator.strength.weak": "Débil",
  "generator.strength.fair": "Aceptable",
  "generator.strength.strong": "Fuerte",
  "generator.strength.veryStrong": "Muy fuerte",
  "generator.strengthLabel": "Seguridad: {strength}",
  "generator.strengthShort": "{strength} · {bits} bits",
  "generator.minorHint": "Dígitos y símbolos en gris",
  "generator.lengthValue": "{length} caracteres",
  "generator.wordsValue": "{length} palabras",
  "generator.characters": "Caracteres",
} satisfies Record<keyof typeof en, string>;
