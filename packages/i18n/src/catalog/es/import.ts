/**
 * Spanish UI strings: import. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { importMessages as en } from "../en/import.ts";

export const importMessages = {
  "import.format.enpass": "Enpass (.json)",
  "import.format.bitwarden": "Bitwarden (.json, sin cifrar)",
  "import.format.keepass": "KeePass (.kdbx)",
  "import.format.onePassword": "1Password (.1pux)",
  "import.format.cxf": "FIDO Credential Exchange (.json): llaves de acceso y contraseñas",
  "import.format.dashlane": "Dashlane (.zip)",
  "import.format.keeper": "Keeper (.json)",
  "import.format.keeperCsv": "Keeper (.csv)",
  "import.format.protonPass": "Proton Pass (.zip o .json, sin cifrar)",
  "import.format.csv": "Archivo CSV (.csv)",
  "import.keepassPassword": "Contraseña de la base de datos de KeePass",
  "import.preview.ready":
    "{count, plural, one {# elemento está listo para importar.} other {# elementos están listos para importar.}}",
  "import.preview.attachments": "{count, plural, one {# adjunto} other {# adjuntos}}",
  "import.preview.duplicate":
    "Ya tienes un elemento como este. Está desmarcado para que no se importe dos veces.",
  "import.preview.duplicateInFile":
    "Este archivo tiene el mismo elemento dos veces. La copia está desmarcada.",
  "import.warning.unmappedFields":
    "Algunos campos no tienen un campo equivalente en Mosaic. Se conservaron como campos personalizados.",
  "import.warning.attachmentsSkipped":
    "Algunos adjuntos superan los 50 MiB, no caben en tu 1 GiB de almacenamiento de adjuntos o faltan en la exportación. Añádelos a mano después de importar.",
  "import.warning.entriesSkipped":
    "Algunas entradas no se pudieron leer y se omitirán. Revísalas en tu antiguo gestor de contraseñas.",
  "import.warning.otpUnreadable":
    "No se reconocieron algunos ajustes de códigos de un solo uso. Se conservaron como campos ocultos, así que vuelve a configurar el código a partir de ellos.",
  "import.warning.otpWeak":
    "Algunos códigos de un solo uso tienen un secreto de menos de 80 bits, por lo que son más fáciles de adivinar. Se importaron igualmente; vuelve a configurarlos con un secreto nuevo en cuanto puedas.",
  "import.warning.passkeysNotImported":
    "Este formato de exportación no incluye las claves privadas de las llaves de acceso, por lo que no se pueden importar. El resto de esos elementos sí se importa.",
  "import.warning.passkeyUnreadable":
    "Algunas llaves de acceso no se pudieron leer y se omitieron.",
  "import.warning.passkeyUnsupported":
    "Algunas llaves de acceso usan un tipo de clave que Mosaic no admite (solo funciona ES256) y se omitieron.",
  "import.warning.passkeysSkipped":
    "Algunas llaves de acceso se omitieron porque hay demasiadas (como máximo 100 por elemento y 10 000 por archivo).",
  "import.warning.passkeysKeepOldApp":
    "Conserva tu antigua app hasta que hayas movido las llaves de acceso.",
  "import.entry.unmappedFields":
    "Algunos campos no tienen un campo equivalente en Mosaic. Se conservaron como campos personalizados.",
  "import.entry.otpUnreadable":
    "No se reconoció el ajuste del código de un solo uso. Se conservó como un campo oculto, así que vuelve a configurar el código a partir de él.",
  "import.entry.otpWeak":
    "El secreto del código de un solo uso tiene menos de 80 bits, por lo que el código es más fácil de adivinar. Se importó igualmente; vuelve a configurarlo con un secreto nuevo en cuanto puedas.",
  "import.entry.attachmentTooLarge":
    "Un adjunto supera los 50 MiB y se omitió. Añádelo a mano después de importar.",
  "import.entry.attachmentMissing":
    "Falta un adjunto en la exportación. Añádelo a mano después de importar.",
  "import.entry.attachmentQuota":
    "Un adjunto no cabe en tu 1 GiB de almacenamiento de adjuntos y se omitió.",
  "import.entry.passkeysNotImported":
    "Este formato de exportación no incluye la clave privada de la llave de acceso de este elemento, por lo que no se puede importar. Conserva tu antigua app hasta que la hayas movido.",
  "import.entry.passkeyUnreadable":
    "No se pudo leer una llave de acceso de este elemento y se omitió. Conserva tu antigua app hasta que la hayas movido.",
  "import.entry.passkeyUnsupported":
    "Una llave de acceso de este elemento usa un tipo de clave que Mosaic no admite (solo funciona ES256) y se omitió. Conserva tu antigua app hasta que la hayas movido.",
  "import.entry.historyTruncated": "Solo se conservaron las 32 contraseñas antiguas más recientes.",
  "import.entry.tagsDropped": "Se omitieron las etiquetas vacías o demasiado largas.",
  "import.entry.archived": "Archivado en tu antigua app. Recibe la etiqueta \"archived\".",
  "import.entry.unknownCategory": "Tipo de elemento desconocido. Se importa como nota segura.",
  "import.entry.titleCleaned":
    "El título tenía caracteres invisibles o que invierten el texto. Se eliminaron.",
  "import.entry.passkeysLimit":
    "Este elemento tiene más llaves de acceso de las que admite una importación (100 por elemento, 10 000 por archivo). Las llaves de acceso sobrantes se omitieron. Conserva tu antigua app hasta que las hayas movido.",
  "import.entry.fieldsDropped":
    "Este elemento tiene más campos de los que Mosaic conserva. Los campos sobrantes se omiten.",
  "import.entry.notesTruncated":
    "Las notas de este elemento son más largas de lo que cabe en un elemento. La parte que no cupo se omitió.",
  "import.entry.idnHost":
    "Esta dirección usa caracteres especiales en su nombre de dominio y puede parecerse a otro sitio. Compruébala antes de fiarte de ella.",
  "import.entry.urlKeptAsText":
    "Esta dirección web no se pudo usar como enlace. Se conservó como texto.",
  "import.skip.malformed": "Esta entrada está dañada y no se puede leer.",
  "import.skip.empty": "Esta entrada está vacía.",
  "import.skip.tooLarge": "Esta entrada supera 1 MiB, el máximo que cabe en un elemento.",
  "import.result":
    "{imported, plural, one {# elemento importado.} other {# elementos importados.}} {skipped, plural, =0 {} one {# omitido.} other {# omitidos.}}",
  "import.flow.title": "Importar",
  "import.flow.formatStep": "¿Desde dónde importas?",
  "import.flow.chooseFile": "Elige el archivo de exportación",
  "import.flow.fileChosen": "Archivo: {name}",
  "import.flow.password": "Contraseña de la base de datos de KeePass",
  "import.flow.preview": "Vista previa",
  "import.flow.previewBusy": "Leyendo el archivo. Puede tardar un momento.",
  "import.flow.selectAll": "Seleccionar todo",
  "import.flow.target": "Importar en",
  "import.flow.commit": "Importar {count, plural, one {# elemento} other {# elementos}}",
  "import.flow.entries": "Entradas",
  "import.flow.skipped": "Omitidas",
  "import.flow.attachments": "{count, plural, one {# adjunto} other {# adjuntos}}",
  "import.flow.cancel": "Cancelar importación",
  "import.flow.noneSelected": "Selecciona al menos un elemento para importar.",
  "import.flow.resultFailed":
    "{failed, plural, =0 {} one {No se pudo guardar # elemento.} other {No se pudieron guardar # elementos.}}",
  "import.flow.deleteReminder":
    "Elimina ahora el archivo de exportación. Contiene tus contraseñas en texto sin cifrar.",
  "import.flow.dropHint":
    "Elige tu archivo de exportación. Todo se lee y se cifra en este dispositivo.",
} satisfies Record<keyof typeof en, string>;
