/**
 * Korean UI strings: palette. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { palette as en } from "../en/palette.ts";

export const palette = {
  "palette.title": "명령 팔레트",
  "palette.search": "항목과 명령 검색",
  "palette.items": "항목",
  "palette.commands": "명령",
  "palette.empty": "일치하는 항목이 없어요. 다른 단어로 검색해 보세요.",
  "palette.cmd.newItem": "새 항목",
  "palette.cmd.generator": "생성기 열기",
  "palette.cmd.security": "보안 보고서 열기",
  "palette.cmd.settings": "설정 열기",
  "palette.cmd.import": "다른 앱에서 가져오기",
  "palette.cmd.share": "보관함 공유…",
  "palette.cmd.invites": "초대",
  "palette.cmd.lock": "Mosaic 잠금",
  "palette.open": "명령 팔레트 열기",
  "palette.hint.move": "이동하려면",
  "palette.hint.open": "열려면",
  "palette.hint.close": "닫으려면",
} satisfies Record<keyof typeof en, string>;
