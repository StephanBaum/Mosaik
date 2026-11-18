/**
 * Korean UI strings: files. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { files as en } from "../en/files.ts";

export const files = {
  "files.title": "파일",
  "files.add": "파일 추가…",
  "files.empty": "아직 파일이 없어요.",
  "files.previewLoading": "미리 보기를 준비하는 중…",
  "files.size.b": "{value} B",
  "files.size.kb": "{value} KB",
  "files.size.mb": "{value} MB",
  "files.addedAt": "{at, date, medium}에 추가됨",
  "files.preview": "{name} 미리 보기",
  "files.saveAs": "{name}을(를) 다른 이름으로 저장…",
  "files.saveAsShort": "다른 이름으로 저장…",
  "files.remove": "{name} 제거",
  "files.close": "닫기",
  "files.previewAlt": "{name} 미리 보기",
  "files.previewOnlyImages":
    "미리 보기는 PNG와 JPEG 이미지에서만 가능해요. 다른 파일은 다른 이름으로 저장을 사용해 여세요.",
  "files.notDownloaded": "이 파일은 아직 이 기기에 없어요.",
  "files.editorNote": "파일은 추가하거나 제거하는 즉시 저장돼요.",
  "files.removeConfirm.title": "{name}을(를) 제거할까요?",
  "files.removeConfirm.body":
    "동기화하는 모든 기기에서 이 항목의 파일이 삭제돼요. 되돌릴 수 없어요.",
  "files.removeConfirm.confirm": "파일 제거",
  "files.removeConfirm.keep": "파일 유지",
  "toast.fileAdded": "파일을 추가했어요.",
  "toast.fileSaved": "파일을 저장했어요.",
  "toast.fileRemoved": "파일을 제거했어요.",
} satisfies Record<keyof typeof en, string>;
