/**
 * Japanese UI strings: palette. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { palette as en } from "../en/palette.ts";

export const palette = {
  "palette.title": "コマンドパレット",
  "palette.search": "項目とコマンドを検索",
  "palette.items": "項目",
  "palette.commands": "コマンド",
  "palette.empty": "一致するものがありません。別の言葉をお試しください。",
  "palette.cmd.newItem": "新しい項目",
  "palette.cmd.generator": "ジェネレーターを開く",
  "palette.cmd.security": "セキュリティレポートを開く",
  "palette.cmd.settings": "設定を開く",
  "palette.cmd.import": "他のアプリから読み込む",
  "palette.cmd.share": "保管庫を共有…",
  "palette.cmd.invites": "招待",
  "palette.cmd.lock": "Mosaic をロック",
  "palette.open": "コマンドパレットを開く",
  "palette.hint.move": "で移動",
  "palette.hint.open": "で開く",
  "palette.hint.close": "で閉じる",
} satisfies Record<keyof typeof en, string>;
