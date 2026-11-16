/**
 * Japanese UI strings: files. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { files as en } from "../en/files.ts";

export const files = {
  "files.title": "ファイル",
  "files.add": "ファイルを追加…",
  "files.empty": "ファイルはまだありません。",
  "files.previewLoading": "プレビューを準備しています…",
  "files.size.b": "{value} B",
  "files.size.kb": "{value} KB",
  "files.size.mb": "{value} MB",
  "files.addedAt": "追加日: {at, date, medium}",
  "files.preview": "{name} をプレビュー",
  "files.saveAs": "{name} を別名で保存…",
  "files.saveAsShort": "別名で保存…",
  "files.remove": "{name} を削除",
  "files.close": "閉じる",
  "files.previewAlt": "{name} のプレビュー",
  "files.previewOnlyImages":
    "プレビューできるのは PNG と JPEG の画像のみです。その他のファイルは「別名で保存」から開いてください。",
  "files.notDownloaded": "このファイルはまだこのデバイスにありません。",
  "files.editorNote": "ファイルは追加または削除した時点で保存されます。",
  "files.removeConfirm.title": "{name} を削除しますか？",
  "files.removeConfirm.body":
    "同期しているすべてのデバイスで、この項目からファイルが削除されます。この操作は元に戻せません。",
  "files.removeConfirm.confirm": "ファイルを削除",
  "files.removeConfirm.keep": "ファイルを残す",
  "toast.fileAdded": "ファイルを追加しました。",
  "toast.fileSaved": "ファイルを保存しました。",
  "toast.fileRemoved": "ファイルを削除しました。",
} satisfies Record<keyof typeof en, string>;
