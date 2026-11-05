/** German UI strings: attachments. Informal "du". Same keys as English (checked by the type). */
import type { files as en } from "../en/files.ts";

export const files = {
  "files.title": "Dateien",
  "files.add": "Datei hinzufügen…",
  "files.empty": "Noch keine Dateien.",
  "files.previewLoading": "Vorschau wird vorbereitet…",
  "files.size.b": "{value} B",
  "files.size.kb": "{value} KB",
  "files.size.mb": "{value} MB",
  "files.addedAt": "Hinzugefügt am {at, date, medium}",
  "files.preview": "Vorschau von {name}",
  "files.saveAs": "{name} speichern unter…",
  "files.saveAsShort": "Speichern unter…",
  "files.remove": "{name} entfernen",
  "files.close": "Schließen",
  "files.previewAlt": "Vorschau von {name}",
  "files.previewOnlyImages":
    "Die Vorschau gibt es für PNG- und JPEG-Bilder. Andere Dateien öffnest du über Speichern unter.",
  "files.notDownloaded": "Diese Datei ist noch nicht auf diesem Gerät.",
  "files.editorNote": "Dateien werden sofort gespeichert, wenn du sie hinzufügst oder entfernst.",
  "files.removeConfirm.title": "{name} entfernen?",
  "files.removeConfirm.body":
    "Die Datei wird auf allen synchronisierten Geräten aus diesem Eintrag gelöscht. Das lässt sich nicht rückgängig machen.",
  "files.removeConfirm.confirm": "Datei entfernen",
  "files.removeConfirm.keep": "Datei behalten",
  "toast.fileAdded": "Datei hinzugefügt.",
  "toast.fileSaved": "Datei gespeichert.",
  "toast.fileRemoved": "Datei entfernt.",
} satisfies Record<keyof typeof en, string>;
