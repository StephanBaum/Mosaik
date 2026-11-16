/**
 * Simplified Chinese UI strings: files. Same keys as English (checked by the type).
 * ICU plural categories of Simplified Chinese: other (CLDR; `other` is required).
 */
import type { files as en } from "../en/files.ts";

export const files = {
  "files.title": "文件",
  "files.add": "添加文件…",
  "files.empty": "还没有文件。",
  "files.previewLoading": "正在准备预览…",
  "files.size.b": "{value} B",
  "files.size.kb": "{value} KB",
  "files.size.mb": "{value} MB",
  "files.addedAt": "添加于 {at, date, medium}",
  "files.preview": "预览 {name}",
  "files.saveAs": "将 {name} 另存为…",
  "files.saveAsShort": "另存为…",
  "files.remove": "移除 {name}",
  "files.close": "关闭",
  "files.previewAlt": "{name} 的预览",
  "files.previewOnlyImages": "仅支持预览 PNG 和 JPEG 图片。其他文件请用“另存为”打开。",
  "files.notDownloaded": "此文件还不在这台设备上。",
  "files.editorNote": "添加或移除文件后会立即保存。",
  "files.removeConfirm.title": "移除 {name}？",
  "files.removeConfirm.body": "该文件会从所有同步此项目的设备上删除。此操作无法撤销。",
  "files.removeConfirm.confirm": "移除文件",
  "files.removeConfirm.keep": "保留文件",
  "toast.fileAdded": "文件已添加。",
  "toast.fileSaved": "文件已保存。",
  "toast.fileRemoved": "文件已移除。",
} satisfies Record<keyof typeof en, string>;
