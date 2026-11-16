/**
 * Polish UI strings: files. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { files as en } from "../en/files.ts";

export const files = {
  "files.title": "Pliki",
  "files.add": "Dodaj plik…",
  "files.empty": "Brak plików.",
  "files.previewLoading": "Przygotowywanie podglądu…",
  "files.size.b": "{value} B",
  "files.size.kb": "{value} KB",
  "files.size.mb": "{value} MB",
  "files.addedAt": "Dodano {at, date, medium}",
  "files.preview": "Podgląd: {name}",
  "files.saveAs": "Zapisz {name} jako…",
  "files.saveAsShort": "Zapisz jako…",
  "files.remove": "Usuń {name}",
  "files.close": "Zamknij",
  "files.previewAlt": "Podgląd: {name}",
  "files.previewOnlyImages":
    "Podgląd działa dla obrazów PNG i JPEG. Użyj Zapisz jako, aby otworzyć inne pliki.",
  "files.notDownloaded": "Tego pliku nie ma jeszcze na tym urządzeniu.",
  "files.editorNote": "Pliki są zapisywane od razu po dodaniu lub usunięciu.",
  "files.removeConfirm.title": "Usunąć {name}?",
  "files.removeConfirm.body":
    "Plik zostanie usunięty z tego elementu na każdym urządzeniu, które go synchronizuje. Nie można tego cofnąć.",
  "files.removeConfirm.confirm": "Usuń plik",
  "files.removeConfirm.keep": "Zachowaj plik",
  "toast.fileAdded": "Dodano plik.",
  "toast.fileSaved": "Zapisano plik.",
  "toast.fileRemoved": "Usunięto plik.",
} satisfies Record<keyof typeof en, string>;
