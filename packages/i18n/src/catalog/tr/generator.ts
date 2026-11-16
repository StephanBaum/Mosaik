/**
 * Turkish UI strings: generator. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { generator as en } from "../en/generator.ts";

export const generator = {
  "generator.title": "Parola oluşturucu",
  "generator.mode": "Tür",
  "generator.mode.password": "Karakterler",
  "generator.mode.passphrase": "Sözcükler",
  "generator.length": "Uzunluk",
  "generator.words": "Sözcükler",
  "generator.lowercase": "Küçük harfler",
  "generator.uppercase": "Büyük harfler",
  "generator.digits": "Rakamlar",
  "generator.symbols": "Simgeler",
  "generator.excludeAmbiguous": "Birbirine benzeyen karakterleri dışarıda bırak",
  "generator.capitalize": "Sözcüklerin baş harfini büyüt",
  "generator.appendDigit": "Bir rakam ekle",
  "generator.result": "Oluşturulan parola",
  "generator.entropy": "{bits} bit entropi",
  "generator.strength.veryWeak": "Çok zayıf",
  "generator.strength.weak": "Zayıf",
  "generator.strength.fair": "Orta",
  "generator.strength.strong": "Güçlü",
  "generator.strength.veryStrong": "Çok güçlü",
  "generator.strengthLabel": "Güç: {strength}",
  "generator.strengthShort": "{strength} · {bits} bit",
  "generator.minorHint": "Rakamlar ve simgeler gri",
  "generator.lengthValue": "{length} karakter",
  "generator.wordsValue": "{length} sözcük",
  "generator.characters": "Karakterler",
} satisfies Record<keyof typeof en, string>;
