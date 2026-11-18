/**
 * Simplified Chinese UI strings: generator. Same keys as English (checked by the type).
 * ICU plural categories of Simplified Chinese: other (CLDR; `other` is required).
 */
import type { generator as en } from "../en/generator.ts";

export const generator = {
  "generator.title": "密码生成器",
  "generator.mode": "类型",
  "generator.mode.password": "字符",
  "generator.mode.passphrase": "单词",
  "generator.length": "长度",
  "generator.words": "单词",
  "generator.lowercase": "小写字母",
  "generator.uppercase": "大写字母",
  "generator.digits": "数字",
  "generator.symbols": "符号",
  "generator.excludeAmbiguous": "排除易混淆的字符",
  "generator.capitalize": "单词首字母大写",
  "generator.appendDigit": "添加一个数字",
  "generator.result": "生成的密码",
  "generator.entropy": "{bits} 位熵",
  "generator.strength.veryWeak": "非常弱",
  "generator.strength.weak": "弱",
  "generator.strength.fair": "一般",
  "generator.strength.strong": "强",
  "generator.strength.veryStrong": "非常强",
  "generator.strengthLabel": "强度：{strength}",
  "generator.strengthShort": "{strength} · {bits} 位",
  "generator.minorHint": "数字和符号以灰色显示",
  "generator.lengthValue": "{length} 个字符",
  "generator.wordsValue": "{length} 个单词",
  "generator.characters": "字符",
} satisfies Record<keyof typeof en, string>;
