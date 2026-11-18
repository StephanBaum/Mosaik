/**
 * Dutch UI strings: files. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { files as en } from "../en/files.ts";

export const files = {
  "files.title": "Bestanden",
  "files.add": "Bestand toevoegen…",
  "files.empty": "Nog geen bestanden.",
  "files.previewLoading": "Voorvertoning voorbereiden…",
  "files.size.b": "{value} B",
  "files.size.kb": "{value} KB",
  "files.size.mb": "{value} MB",
  "files.addedAt": "Toegevoegd op {at, date, medium}",
  "files.preview": "Voorvertoning van {name}",
  "files.saveAs": "{name} bewaren als…",
  "files.saveAsShort": "Bewaren als…",
  "files.remove": "{name} verwijderen",
  "files.close": "Sluiten",
  "files.previewAlt": "Voorvertoning van {name}",
  "files.previewOnlyImages":
    "Voorvertoning werkt voor PNG- en JPEG-afbeeldingen. Gebruik Bewaren als om andere bestanden te openen.",
  "files.notDownloaded": "Dit bestand staat nog niet op dit apparaat.",
  "files.editorNote": "Bestanden worden bewaard zodra je ze toevoegt of verwijdert.",
  "files.removeConfirm.title": "{name} verwijderen?",
  "files.removeConfirm.body":
    "Het bestand wordt van dit item verwijderd op elk apparaat dat het synchroniseert. Je kunt dit niet ongedaan maken.",
  "files.removeConfirm.confirm": "Bestand verwijderen",
  "files.removeConfirm.keep": "Bestand behouden",
  "toast.fileAdded": "Bestand toegevoegd.",
  "toast.fileSaved": "Bestand bewaard.",
  "toast.fileRemoved": "Bestand verwijderd.",
} satisfies Record<keyof typeof en, string>;
