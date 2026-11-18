/**
 * French UI strings: generator. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { generator as en } from "../en/generator.ts";

export const generator = {
  "generator.title": "Générateur de mots de passe",
  "generator.mode": "Type",
  "generator.mode.password": "Caractères",
  "generator.mode.passphrase": "Mots",
  "generator.length": "Longueur",
  "generator.words": "Mots",
  "generator.lowercase": "Lettres minuscules",
  "generator.uppercase": "Lettres majuscules",
  "generator.digits": "Chiffres",
  "generator.symbols": "Symboles",
  "generator.excludeAmbiguous": "Exclure les caractères qui se ressemblent",
  "generator.capitalize": "Mettre les mots en majuscule",
  "generator.appendDigit": "Ajouter un chiffre",
  "generator.result": "Mot de passe généré",
  "generator.entropy": "{bits} bits d’entropie",
  "generator.strength.veryWeak": "Très faible",
  "generator.strength.weak": "Faible",
  "generator.strength.fair": "Moyen",
  "generator.strength.strong": "Fort",
  "generator.strength.veryStrong": "Très fort",
  "generator.strengthLabel": "Robustesse : {strength}",
  "generator.strengthShort": "{strength} · {bits} bits",
  "generator.minorHint": "Chiffres et symboles en gris",
  "generator.lengthValue": "{length} caractères",
  "generator.wordsValue": "{length} mots",
  "generator.characters": "Caractères",
} satisfies Record<keyof typeof en, string>;
