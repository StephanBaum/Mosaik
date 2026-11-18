/**
 * Spanish UI strings: files. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { files as en } from "../en/files.ts";

export const files = {
  "files.title": "Archivos",
  "files.add": "Añadir archivo…",
  "files.empty": "Aún no hay archivos.",
  "files.previewLoading": "Preparando la vista previa…",
  "files.size.b": "{value} B",
  "files.size.kb": "{value} KB",
  "files.size.mb": "{value} MB",
  "files.addedAt": "Añadido el {at, date, medium}",
  "files.preview": "Vista previa de {name}",
  "files.saveAs": "Guardar {name} como…",
  "files.saveAsShort": "Guardar como…",
  "files.remove": "Quitar {name}",
  "files.close": "Cerrar",
  "files.previewAlt": "Vista previa de {name}",
  "files.previewOnlyImages":
    "La vista previa funciona con imágenes PNG y JPEG. Usa Guardar como para abrir otros archivos.",
  "files.notDownloaded": "Este archivo aún no está en este dispositivo.",
  "files.editorNote": "Los archivos se guardan en cuanto los añades o los quitas.",
  "files.removeConfirm.title": "¿Quitar {name}?",
  "files.removeConfirm.body":
    "El archivo se elimina de este elemento en todos los dispositivos que lo sincronizan. No se puede deshacer.",
  "files.removeConfirm.confirm": "Quitar archivo",
  "files.removeConfirm.keep": "Conservar archivo",
  "toast.fileAdded": "Archivo añadido.",
  "toast.fileSaved": "Archivo guardado.",
  "toast.fileRemoved": "Archivo quitado.",
} satisfies Record<keyof typeof en, string>;
