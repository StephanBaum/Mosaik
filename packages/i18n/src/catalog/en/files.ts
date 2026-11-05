/** English UI strings: attachments (files on an item). ICU MessageFormat. */
export const files = {
  "files.title": "Files",
  "files.add": "Add file…",
  "files.empty": "No files yet.",
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
} as const satisfies Record<string, string>;
