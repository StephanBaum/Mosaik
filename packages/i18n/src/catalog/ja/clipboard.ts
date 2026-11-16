/**
 * Japanese UI strings: clipboard. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { clipboard as en } from "../en/clipboard.ts";

export const clipboard = {
  "clipboard.copied": "コピーしました。{seconds, plural, other {#秒}}後に消去されます。",
  "clipboard.cleared": "クリップボードを消去しました。",
  "toast.copied": "コピーしました。{seconds, plural, other {#秒}}後に消去されます。",
  "toast.copiedKept": "コピーしました。Mosaic はクリップボードを消去しません。",
  "toast.clipboardCleared": "クリップボードを消去しました。",
  "toast.saved": "保存しました。",
  "toast.trashed": "ゴミ箱に移動しました。",
  "toast.restored": "復元しました。",
  "toast.purged": "完全に削除しました。",
  "clipboard.clearNow": "今すぐ消去",
  "toast.dismiss": "閉じる",
  "toast.region": "通知",
} satisfies Record<keyof typeof en, string>;
