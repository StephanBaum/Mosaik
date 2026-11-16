/**
 * Korean UI strings: generator. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { generator as en } from "../en/generator.ts";

export const generator = {
  "generator.title": "비밀번호 생성기",
  "generator.mode": "유형",
  "generator.mode.password": "문자",
  "generator.mode.passphrase": "단어",
  "generator.length": "길이",
  "generator.words": "단어",
  "generator.lowercase": "소문자",
  "generator.uppercase": "대문자",
  "generator.digits": "숫자",
  "generator.symbols": "기호",
  "generator.excludeAmbiguous": "비슷하게 보이는 문자 제외",
  "generator.capitalize": "단어를 대문자로 시작",
  "generator.appendDigit": "숫자 추가",
  "generator.result": "생성된 비밀번호",
  "generator.entropy": "엔트로피 {bits}비트",
  "generator.strength.veryWeak": "매우 약함",
  "generator.strength.weak": "약함",
  "generator.strength.fair": "보통",
  "generator.strength.strong": "강함",
  "generator.strength.veryStrong": "매우 강함",
  "generator.strengthLabel": "강도: {strength}",
  "generator.strengthShort": "{strength} · {bits}비트",
  "generator.minorHint": "숫자와 기호는 회색으로 표시",
  "generator.lengthValue": "{length}자",
  "generator.wordsValue": "단어 {length}개",
  "generator.characters": "문자",
} satisfies Record<keyof typeof en, string>;
