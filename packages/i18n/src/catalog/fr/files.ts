/**
 * French UI strings: files. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { files as en } from "../en/files.ts";

export const files = {
  "files.title": "Fichiers",
  "files.add": "Ajouter un fichier…",
  "files.empty": "Aucun fichier pour l’instant.",
  "files.previewLoading": "Préparation de l’aperçu…",
  "files.size.b": "{value} o",
  "files.size.kb": "{value} Ko",
  "files.size.mb": "{value} Mo",
  "files.addedAt": "Ajouté le {at, date, medium}",
  "files.preview": "Aperçu de {name}",
  "files.saveAs": "Enregistrer {name} sous…",
  "files.saveAsShort": "Enregistrer sous…",
  "files.remove": "Retirer {name}",
  "files.close": "Fermer",
  "files.previewAlt": "Aperçu de {name}",
  "files.previewOnlyImages":
    "L’aperçu fonctionne pour les images PNG et JPEG. Utilisez Enregistrer sous pour ouvrir d’autres fichiers.",
  "files.notDownloaded": "Ce fichier n’est pas encore sur cet appareil.",
  "files.editorNote": "Les fichiers sont enregistrés dès que vous les ajoutez ou les retirez.",
  "files.removeConfirm.title": "Retirer {name} ?",
  "files.removeConfirm.body":
    "Le fichier est supprimé de cet élément sur tous les appareils qui le synchronisent. Vous ne pourrez pas annuler.",
  "files.removeConfirm.confirm": "Retirer le fichier",
  "files.removeConfirm.keep": "Garder le fichier",
  "toast.fileAdded": "Fichier ajouté.",
  "toast.fileSaved": "Fichier enregistré.",
  "toast.fileRemoved": "Fichier retiré.",
} satisfies Record<keyof typeof en, string>;
