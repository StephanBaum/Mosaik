/**
 * Japanese UI strings: generator. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { generator as en } from "../en/generator.ts";

export const generator = {
  "generator.title": "パスワードジェネレーター",
  "generator.mode": "タイプ",
  "generator.mode.password": "文字",
  "generator.mode.passphrase": "単語",
  "generator.length": "長さ",
  "generator.words": "単語数",
  "generator.lowercase": "英小文字",
  "generator.uppercase": "英大文字",
  "generator.digits": "数字",
  "generator.symbols": "記号",
  "generator.excludeAmbiguous": "紛らわしい文字を除く",
  "generator.capitalize": "単語の先頭を大文字にする",
  "generator.appendDigit": "数字を1つ追加",
  "generator.result": "生成されたパスワード",
  "generator.entropy": "エントロピー {bits} ビット",
  "generator.strength.veryWeak": "非常に弱い",
  "generator.strength.weak": "弱い",
  "generator.strength.fair": "普通",
  "generator.strength.strong": "強い",
  "generator.strength.veryStrong": "非常に強い",
  "generator.strengthLabel": "強度: {strength}",
  "generator.strengthShort": "{strength} · {bits} ビット",
  "generator.minorHint": "数字と記号はグレーで表示",
  "generator.lengthValue": "{length} 文字",
  "generator.wordsValue": "{length} 語",
  "generator.characters": "文字",
} satisfies Record<keyof typeof en, string>;
