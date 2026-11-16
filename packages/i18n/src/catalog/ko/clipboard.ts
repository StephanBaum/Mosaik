/**
 * Korean UI strings: clipboard. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { clipboard as en } from "../en/clipboard.ts";

export const clipboard = {
  "clipboard.copied": "복사했어요. {seconds, plural, other {#초}} 후 지워져요.",
  "clipboard.cleared": "클립보드를 지웠어요.",
  "toast.copied": "복사했어요. {seconds, plural, other {#초}} 후 지워져요.",
  "toast.copiedKept": "복사했어요. Mosaic이 클립보드를 지우지 않아요.",
  "toast.clipboardCleared": "클립보드를 지웠어요.",
  "toast.saved": "저장했어요.",
  "toast.trashed": "휴지통으로 이동했어요.",
  "toast.restored": "복원했어요.",
  "toast.purged": "완전히 삭제했어요.",
  "clipboard.clearNow": "지금 지우기",
  "toast.dismiss": "닫기",
  "toast.region": "알림",
} satisfies Record<keyof typeof en, string>;
