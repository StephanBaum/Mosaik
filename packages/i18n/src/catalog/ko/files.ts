// UNTRANSLATED: copy of English, replace every value
/**
 * Korean UI strings: files. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { files as en } from "../en/files.ts";

export const files = {
  "files.title": "Files",
  "files.add": "Add file…",
  "files.empty": "No files yet.",
  "files.previewLoading": "Preparing preview…",
  "files.size.b": "{value} B",
  "files.size.kb": "{value} KB",
  "files.size.mb": "{value} MB",
  "files.addedAt": "Added {at, date, medium}",
  "files.preview": "Preview {name}",
  "files.saveAs": "Save {name} as…",
  "files.saveAsShort": "Save as…",
  "files.remove": "Remove {name}",
  "files.close": "Close",
  "files.previewAlt": "Preview of {name}",
  "files.previewOnlyImages":
    "Preview works for PNG and JPEG images. Use Save as to open other files.",
  "files.notDownloaded": "This file isn't on this device yet.",
  "files.editorNote": "Files are saved as soon as you add or remove them.",
  "files.removeConfirm.title": "Remove {name}?",
  "files.removeConfirm.body":
    "The file is deleted from this item on every device that syncs it. You can't undo this.",
  "files.removeConfirm.confirm": "Remove file",
  "files.removeConfirm.keep": "Keep file",
  "toast.fileAdded": "File added.",
  "toast.fileSaved": "File saved.",
  "toast.fileRemoved": "File removed.",
} satisfies Record<keyof typeof en, string>;
