/**
 * Spanish UI strings: mobileFolder. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { mobileFolder as en } from "../en/mobileFolder.ts";

export const mobileFolder = {
  "mobile.sync.never": "Aún sin sincronizar",
  "mobile.sync.last": "Sincronizado {time}",
  "mobile.sync.localOnly":
    "Esta caja fuerte solo está en este teléfono. La sincronización no está configurada.",
  "mobile.sync.pending":
    "{count, plural, one {# cambio en espera de subirse} other {# cambios en espera de subirse}}",
  "mobile.sync.now": "Sincronizar ahora",
  "mobile.sync.working": "Sincronizando",
  "mobile.sync.warnings": "Avisos de sincronización",
  "mobile.sync.warning.freezeSuspected":
    "Hace un tiempo que no llegan cambios de otro dispositivo. Comprueba que el otro dispositivo esté encendido y con conexión.",
  "mobile.sync.warning.noCommonView":
    "Este teléfono y otro dispositivo llevan dos días sin ver los mismos datos del servidor. Puede que el servidor les muestre versiones distintas. Sincroniza tu otro dispositivo y luego sincroniza aquí.",
  "mobile.sync.warning.rollbackDetected":
    "Tus datos de sincronización parecen más antiguos que los de este teléfono. Abre Mosaic en otro dispositivo y comprueba que tus últimos cambios están ahí. Después elige “Ya lo comprobé”.",
  "mobile.sync.warning.identityDuplicated":
    "Otra copia de Mosaic está escribiendo con la identidad de este teléfono, por ejemplo tras restaurar una copia de seguridad. La sincronización está en pausa. Asegúrate de que solo se use una copia y luego elige “Ya lo comprobé”.",
  "mobile.sync.warning.serverInconsistent":
    "El servidor mostró a este teléfono datos distintos a los de otro dispositivo. Abre Mosaic en otro dispositivo y comprueba que tus últimos cambios están ahí. Después elige “Ya lo comprobé”.",
  "mobile.sync.warning.serverRollback":
    "La copia que el servidor tiene de una de tus cajas fuertes es más antigua que la de este teléfono. Abre Mosaic en otro dispositivo y comprueba que tus últimos cambios están ahí. Después elige “Ya lo comprobé”.",
  "mobile.sync.warning.sharingAttention":
    "Una caja fuerte compartida requiere atención: hay un cambio de clave en espera o no se pudo comprobar a un miembro. Abre Mosaic en tu equipo para ver cuál es. Después elige “Ya lo comprobé”.",
  "mobile.sync.warning.undecryptable":
    "{count, plural, one {No se pudo leer en este teléfono # elemento recibido.} other {No se pudieron leer en este teléfono # elementos recibidos.}} Actualiza Mosaic en todos tus dispositivos y vuelve a sincronizar.",
  "mobile.sync.warning.accountRollback":
    "El servidor muestra una clave de cuenta más antigua que la de este teléfono. No se cambió nada. Abre Mosaic en otro dispositivo y comprueba que tus últimos cambios están ahí. Después elige “Ya lo comprobé”.",
  "mobile.sync.acknowledge": "Ya lo comprobé…",
  "mobile.sync.acknowledgeTitle": "¿Borrar los avisos de sincronización?",
  "mobile.sync.acknowledgeBody":
    "Continúa solo después de haber comprobado tú mismo el servidor. Esto borra todos los avisos y la sincronización continúa. Si tus datos realmente se revirtieron o se modificaron, Mosaic no podrá detectarlo después. Haz esto en cada uno de tus dispositivos.",
  "mobile.sync.acknowledgeConfirm": "Ya lo comprobé",
  "mobile.folder.intro": "Esta cuenta se sincroniza a través de tu carpeta de {provider}.",
  "mobile.folder.signIn": "Iniciar sesión en {provider}",
  "mobile.folder.scope":
    "{provider} pedirá acceso a todos tus archivos. Mosaic solo abre la carpeta que elijas y nunca toca nada más.",
  "mobile.folder.waiting": "Termina de iniciar sesión en tu navegador y vuelve aquí.",
  "mobile.folder.serverLabel": "Dirección de Nextcloud o WebDAV",
  "mobile.folder.serverHint": "https://cloud.example.com",
  "mobile.folder.serverConfirm":
    "Mosaic iniciará sesión en {server}. Continúa solo si es tu propio servidor.",
  "mobile.folder.locating": "Buscando tu carpeta de Mosaic…",
  "mobile.folder.browseTitle": "Elige tu carpeta de Mosaic",
  "mobile.folder.browseHint":
    "Es la carpeta que sincroniza tu equipo, con un archivo llamado mosaic-sync.json.",
  "mobile.folder.currentFolder": "Carpeta actual: {path}",
  "mobile.folder.noSubfolders": "Aquí no hay carpetas.",
  "mobile.folder.useThis": "Usar esta carpeta",
  "mobile.folder.up": "Subir un nivel",
  "mobile.folder.chooseProvider": "¿Dónde se sincroniza tu cuenta?",
  "mobile.folder.noDrive":
    "Google Drive e iCloud Drive aún no funcionan en Android. Usa Dropbox, OneDrive, Nextcloud o la sincronización de Mosaic.",
  "mobile.folder.useFolder": "Sincronizar a través de una carpeta en la nube",
  "mobile.folder.providerDropbox": "Dropbox",
  "mobile.folder.providerOneDrive": "OneDrive",
  "mobile.folder.providerWebDav": "Nextcloud o WebDAV",
  "mobile.folder.appPasswordToggle": "Usar una contraseña de aplicación",
  "mobile.folder.davLabel": "Dirección de la carpeta WebDAV",
  "mobile.folder.loginLabel": "Nombre de usuario",
  "mobile.folder.appPasswordLabel": "Contraseña de aplicación",
  "mobile.folder.appPasswordHint":
    "Crea una contraseña de aplicación en los ajustes de seguridad de tu servidor. No uses tu contraseña principal.",
  "mobile.folder.appPasswordAction": "Iniciar sesión con la contraseña de aplicación",
  "mobile.folder.pickAccount": "¿A qué cuenta se une este teléfono?",
  "mobile.folder.accountOption": "Cuenta {id}",
  "mobile.folder.linked": "Sesión iniciada. Sincronizando de nuevo.",
  "mobile.folder.settingsLine": "Se sincroniza a través de {provider}: {path}",
  "mobile.folder.relink": "Iniciar sesión de nuevo",
  "mobile.folder.msRevokeHint":
    "Para quitar por completo el acceso de Mosaic, quítalo en la página de tu cuenta de Microsoft, en Aplicaciones.",
  "mobile.reset.revokeDropbox":
    "Se quitó la cuenta. No pudimos terminar el acceso de Mosaic en Dropbox: quítalo en Dropbox, en Ajustes, Aplicaciones conectadas.",
  "mobile.reset.revokeWebdav":
    "Se quitó la cuenta. No pudimos eliminar la contraseña de aplicación en tu servidor: quítala allí, en Ajustes, Seguridad, Dispositivos y sesiones.",
  "mobile.reset.title": "Quitar esta cuenta de este teléfono",
  "mobile.reset.body":
    "Esto elimina la copia de la caja fuerte, la Clave secreta y el inicio de sesión en la nube guardados en este teléfono. Tus datos en la carpeta de la nube o en el servidor se conservan. Puedes volver a unirte más tarde con tu Clave secreta y tu contraseña maestra.",
  "mobile.reset.action": "Quitar cuenta…",
  "mobile.reset.confirm": "Quitar cuenta",
  "mobile.reset.phraseLabel": "Confirmación",
  "mobile.reset.typePhrase": "Escribe {phrase} para confirmar",
} satisfies Record<keyof typeof en, string>;
