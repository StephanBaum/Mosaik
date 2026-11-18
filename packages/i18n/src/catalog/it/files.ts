/**
 * Italian UI strings: files. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { files as en } from "../en/files.ts";

export const files = {
  "files.title": "File",
  "files.add": "Aggiungi file…",
  "files.empty": "Ancora nessun file.",
  "files.previewLoading": "Preparazione dell'anteprima…",
  "files.size.b": "{value} B",
  "files.size.kb": "{value} KB",
  "files.size.mb": "{value} MB",
  "files.addedAt": "Aggiunto il {at, date, medium}",
  "files.preview": "Anteprima di {name}",
  "files.saveAs": "Salva {name} con nome…",
  "files.saveAsShort": "Salva con nome…",
  "files.remove": "Rimuovi {name}",
  "files.close": "Chiudi",
  "files.previewAlt": "Anteprima di {name}",
  "files.previewOnlyImages":
    "L'anteprima funziona per le immagini PNG e JPEG. Usa Salva con nome per aprire gli altri file.",
  "files.notDownloaded": "Questo file non è ancora su questo dispositivo.",
  "files.editorNote": "I file vengono salvati appena li aggiungi o li rimuovi.",
  "files.removeConfirm.title": "Rimuovere {name}?",
  "files.removeConfirm.body":
    "Il file viene eliminato da questo elemento su ogni dispositivo che lo sincronizza. Non puoi annullare.",
  "files.removeConfirm.confirm": "Rimuovi file",
  "files.removeConfirm.keep": "Conserva file",
  "toast.fileAdded": "File aggiunto.",
  "toast.fileSaved": "File salvato.",
  "toast.fileRemoved": "File rimosso.",
} satisfies Record<keyof typeof en, string>;
