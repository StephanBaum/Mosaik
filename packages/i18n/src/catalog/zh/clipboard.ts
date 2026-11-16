/**
 * Simplified Chinese UI strings: clipboard. Same keys as English (checked by the type).
 * ICU plural categories of Simplified Chinese: other (CLDR; `other` is required).
 */
import type { clipboard as en } from "../en/clipboard.ts";

export const clipboard = {
  "clipboard.copied": "已复制。{seconds, plural, other {# 秒}}后清除。",
  "clipboard.cleared": "剪贴板已清除。",
  "toast.copied": "已复制。{seconds, plural, other {# 秒}}后清除。",
  "toast.copiedKept": "已复制。Mosaic 不会清除剪贴板。",
  "toast.clipboardCleared": "剪贴板已清除。",
  "toast.saved": "已保存。",
  "toast.trashed": "已移到回收站。",
  "toast.restored": "已恢复。",
  "toast.purged": "已永久删除。",
  "clipboard.clearNow": "立即清除",
  "toast.dismiss": "关闭",
  "toast.region": "通知",
} satisfies Record<keyof typeof en, string>;
