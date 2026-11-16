/**
 * Spanish UI strings: importGuide. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { importGuide as en } from "../en/importGuide.ts";

export const importGuide = {
  "import.grid.title": "¿Dónde están ahora tus contraseñas?",
  "import.grid.browsers": "Navegadores",
  "import.grid.managers": "Gestores de contraseñas",
  "import.grid.other": "Otro origen",
  "import.source.chrome": "Chrome",
  "import.source.edge": "Edge",
  "import.source.brave": "Brave",
  "import.source.opera": "Opera",
  "import.source.vivaldi": "Vivaldi",
  "import.source.arc": "Arc",
  "import.source.firefox": "Firefox",
  "import.source.safari": "Safari",
  "import.source.applePasswords": "Contraseñas de Apple",
  "import.source.onePassword": "1Password",
  "import.source.bitwarden": "Bitwarden",
  "import.source.keepass": "KeePass",
  "import.source.keepassxcCsv": "KeePassXC (CSV)",
  "import.source.enpass": "Enpass",
  "import.source.lastpass": "LastPass",
  "import.source.dashlane": "Dashlane",
  "import.source.keeper": "Keeper",
  "import.source.nordpass": "NordPass",
  "import.source.protonPass": "Proton Pass",
  "import.source.roboform": "RoboForm",
  "import.source.otherFile": "Otro CSV u otro archivo",
  "import.guide.chrome.1":
    "Abre esta dirección en Chrome: cópiala y pégala en la barra de direcciones.",
  "import.guide.chrome.2":
    "Junto a \"Exportar contraseñas\", haz clic en \"Descargar archivo\" y confirma con la contraseña de tu equipo.",
  "import.guide.chrome.3": "Chrome guarda \"Chrome Passwords.csv\" en Descargas.",
  "import.guide.edge.1":
    "Abre esta dirección en Edge: cópiala y pégala en la barra de direcciones.",
  "import.guide.edge.2":
    "Haz clic en \"⋯\" junto a \"Contraseñas guardadas\", luego en \"Exportar contraseñas\" y confirma.",
  "import.guide.edge.3": "Edge guarda el archivo CSV en Descargas.",
  "import.guide.brave.1":
    "Abre esta dirección en Brave: cópiala y pégala en la barra de direcciones.",
  "import.guide.brave.2":
    "Junto a \"Exportar contraseñas\", haz clic en \"Descargar archivo\" y confirma con la contraseña de tu equipo.",
  "import.guide.brave.3": "Brave guarda el archivo CSV en Descargas.",
  "import.guide.opera.1":
    "Abre esta dirección en Opera: cópiala y pégala en la barra de direcciones.",
  "import.guide.opera.2":
    "Junto a \"Exportar contraseñas\", haz clic en \"Descargar archivo\" y confirma con la contraseña de tu equipo.",
  "import.guide.opera.3": "Opera guarda el archivo CSV en Descargas.",
  "import.guide.vivaldi.1":
    "Abre esta dirección en Vivaldi: cópiala y pégala en la barra de direcciones.",
  "import.guide.vivaldi.2":
    "Junto a \"Exportar contraseñas\", haz clic en \"Descargar archivo\" y confirma con la contraseña de tu equipo.",
  "import.guide.vivaldi.3": "Vivaldi guarda el archivo CSV en Descargas.",
  "import.guide.arc.1": "Abre esta dirección en Arc: cópiala y pégala en la barra de direcciones.",
  "import.guide.arc.2":
    "Junto a \"Exportar contraseñas\", haz clic en \"Descargar archivo\" y confirma con la contraseña de tu equipo.",
  "import.guide.arc.3": "Arc guarda el archivo CSV en Descargas.",
  "import.guide.firefox.1":
    "Abre esta dirección en Firefox: cópiala y pégala en la barra de direcciones.",
  "import.guide.firefox.2":
    "Haz clic en \"⋯\" arriba a la derecha, luego en \"Exportar contraseñas…\" y confirma.",
  "import.guide.firefox.3": "Guarda \"logins.csv\" en Descargas.",
  "import.guide.safari.1":
    "Safari → Ajustes → Contraseñas (macOS 14 y anteriores; en macOS 15 y posteriores usa Contraseñas de Apple).",
  "import.guide.safari.2":
    "Haz clic en \"⋯\" debajo de la lista y luego en \"Exportar todas las contraseñas…\".",
  "import.guide.safari.3": "Guarda el archivo CSV en Descargas.",
  "import.guide.applePasswords.1": "Abre la app Contraseñas.",
  "import.guide.applePasswords.2": "Archivo → \"Exportar todas las contraseñas a un archivo…\".",
  "import.guide.applePasswords.3": "Guarda el archivo CSV en Descargas.",
  "import.guide.onePassword.1": "Abre la app 1Password y desbloquéala.",
  "import.guide.onePassword.2":
    "Archivo → Exportar → tu cuenta y luego escribe tu contraseña.",
  "import.guide.onePassword.3": "Elige \"1PUX\" y haz clic en \"Exportar datos\".",
  "import.guide.bitwarden.1":
    "En la caja fuerte web o en la app: Herramientas → Exportar caja fuerte.",
  "import.guide.bitwarden.2": "Elige el formato de archivo \".json\" (no \"cifrado\").",
  "import.guide.bitwarden.3": "Confirma con tu contraseña maestra.",
  "import.guide.keepass.1": "Usa tu archivo de base de datos (.kdbx) tal cual.",
  "import.guide.keepass.2":
    "Escribirás su contraseña en el siguiente paso. Se queda en este dispositivo.",
  "import.guide.keepassxcCsv.1": "KeePassXC: Base de datos → Exportar → Archivo CSV…",
  "import.guide.keepassxcCsv.2":
    "Guárdalo en Descargas. El archivo .kdbx conserva más datos, así que elige \"KeePass\" si puedes.",
  "import.guide.enpass.1": "Enpass: Menú → Archivo → Exportar.",
  "import.guide.enpass.2":
    "Elige la caja fuerte, el formato \".json\" y escribe tu contraseña maestra.",
  "import.guide.enpass.3": "Guárdalo en Descargas.",
  "import.guide.lastpass.1":
    "Inicia sesión en lastpass.com y abre Opciones avanzadas → Exportar.",
  "import.guide.lastpass.2": "Escribe tu contraseña maestra. Se descarga el archivo CSV.",
  "import.guide.lastpass.3":
    "Si el navegador muestra texto en su lugar, guarda la página como \"lastpass.csv\".",
  "import.guide.dashlane.1":
    "Abre Dashlane en tu navegador → Mi cuenta → Ajustes → Exportar datos.",
  "import.guide.dashlane.2": "Elige \"CSV\". Dashlane descarga un archivo ZIP.",
  "import.guide.dashlane.3": "Deja el archivo ZIP como está.",
  "import.guide.keeper.1": "Keeper Web Vault → Ajustes → Exportar.",
  "import.guide.keeper.2":
    "Elige \"JSON\" (conserva los campos personalizados y los códigos 2FA) y exporta.",
  "import.guide.nordpass.1": "NordPass → Ajustes → Exportar elementos.",
  "import.guide.nordpass.2":
    "Confirma con tu contraseña maestra y guarda el archivo CSV en Descargas.",
  "import.guide.protonPass.1": "Proton Pass → Ajustes → Exportar.",
  "import.guide.protonPass.2":
    "Elige \"ZIP\" y deja el cifrado desactivado. Mosaic no puede leer la exportación cifrada con PGP.",
  "import.guide.protonPass.3": "Desbloquea y guárdalo en Descargas.",
  "import.guide.roboform.1": "RoboForm → Ajustes → Cuenta y datos → Exportar.",
  "import.guide.roboform.2":
    "Elige \"CSV\" e \"Inicios de sesión\" y guárdalo en Descargas.",
  "import.guide.otherFile.1":
    "Exporta un archivo CSV con una fila de encabezado o uno de los formatos anteriores.",
  "import.guide.otherFile.2": "Elige el archivo. Mosaic asigna las columnas.",
  "import.guide.copyUrl": "Copiar dirección",
  "import.guide.urlCopied": "Dirección copiada.",
  "import.guide.choose": "Elige el archivo de exportación",
  "import.guide.dropHint": "…o suelta un archivo aquí. Solo se lee en este dispositivo.",
  "import.guide.back": "Atrás",
  "import.detect.detected": "Reconocido: {source}",
  "import.detect.mismatch":
    "Este archivo parece una exportación de {source}. Mosaic lo leerá como tal.",
  "import.detected.chromium": "Chrome, Edge u otro navegador basado en Chromium",
  "import.detected.firefox": "Firefox",
  "import.detected.safari": "Safari o Contraseñas de Apple",
  "import.detected.bitwardenCsv": "Bitwarden (CSV)",
  "import.detected.bitwarden": "Bitwarden",
  "import.detected.enpass": "Enpass",
  "import.detected.keepass": "KeePass",
  "import.detected.keepassxcCsv": "KeePassXC (CSV)",
  "import.detected.onePassword": "1Password",
  "import.detected.cxf": "un archivo de FIDO Credential Exchange",
  "import.detected.lastpass": "LastPass",
  "import.detected.dashlane": "Dashlane",
  "import.detected.keeper": "Keeper",
  "import.detected.nordpass": "NordPass",
  "import.detected.protonPass": "Proton Pass",
  "import.detected.roboform": "RoboForm",
  "import.detected.csv": "un archivo CSV",
  "import.lost.title": "No se importará",
  "import.lost.passkeys":
    "Llaves de acceso: {source} no las incluye en su exportación. Conserva {source} hasta que hayas configurado llaves de acceso nuevas.",
  "import.lost.attachments":
    "Adjuntos: {source} no los incluye en su exportación. Añádelos a mano.",
  "import.lost.history": "Contraseñas antiguas (historial): esta exportación no las contiene.",
  "import.lost.roboformExtras":
    "Identidades, notas seguras y marcadores: expórtalos por separado. No forman parte del archivo de inicios de sesión.",
  "import.lost.entriesSkipped":
    "{count, plural, one {# entrada no se pudo leer y se omite.} other {# entradas no se pudieron leer y se omiten.}}",
  "import.lost.fieldsDropped":
    "Algunos elementos tienen más campos de los que Mosaic conserva. Los campos sobrantes se omiten.",
  "import.lost.notesTruncated":
    "Algunas notas son más largas de lo que cabe en un elemento. La parte que no cupo se omite.",
  "import.lost.attachmentsSkipped":
    "Faltan algunos adjuntos, son demasiado grandes o no caben en tu almacenamiento de adjuntos. Añádelos a mano.",
  "import.cleanup.title": "Limpieza",
  "import.cleanup.delete": "Eliminar {name} ahora",
  "import.cleanup.deleted": "{name} se sobrescribió y se eliminó.",
  "import.cleanup.deleteFailed": "No se pudo eliminar {name}. Elimínalo tú mismo.",
  "import.cleanup.keepFile":
    "Algunos elementos no se importaron o no se importaron por completo. Conserva {name} hasta que hayas comprobado qué falta y luego elimínalo tú mismo.",
  "import.cleanup.ssdNote":
    "En los SSD y en las carpetas sincronizadas pueden sobrevivir copias antiguas: en el historial de versiones, en copias de seguridad o en la papelera de reciclaje. Revisa esos lugares también.",
  "import.cleanup.synced":
    "Este archivo está en tu carpeta de {service}. {service} puede conservar versiones anteriores; elimínalas allí también.",
  "import.synced.oneDrive": "OneDrive",
  "import.synced.dropbox": "Dropbox",
  "import.synced.googleDrive": "Google Drive",
  "import.synced.icloud": "iCloud",
  "import.synced.box": "Box",
  "import.checklist.title": "Antes de dejar tu app anterior",
  "import.checklist.browserSaving":
    "Desactiva en {source} el guardado de contraseñas y el inicio de sesión automático, para que no ofrezca guardar junto con Mosaic.",
  "import.checklist.firefoxSaving":
    "Abre about:preferences#privacy y desmarca \"Preguntar si se guardan las contraseñas\", para que Firefox no ofrezca guardar junto con Mosaic.",
  "import.checklist.safariSaving":
    "Safari → Ajustes → Autocompletar: desmarca \"Nombres de usuario y contraseñas\", para que Safari no rellene junto con Mosaic.",
  "import.checklist.oldExtension":
    "Desactiva la extensión del navegador de {source} para que solo rellene una app.",
  "import.checklist.deleteOld":
    "Elimina tus datos en {source} cuando hayas comprobado todo en Mosaic.",
} satisfies Record<keyof typeof en, string>;
