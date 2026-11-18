/**
 * Spanish UI strings: sync. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { sync as en } from "../en/sync.ts";

export const sync = {
  "sync.status.signedOut": "Sesión cerrada",
  "sync.platform.windows": "Windows",
  "sync.platform.macos": "macOS",
  "sync.platform.linux": "Linux",
  "settings.sync.title": "Sincronización",
  "settings.sync.body":
    "Mantén tus cajas fuertes al día en todos tus dispositivos. Todo se cifra en este dispositivo antes de salir de él.",
  "settings.sync.notSetUp":
    "La sincronización está desactivada. Tus cajas fuertes se guardan solo en este dispositivo.",
  "settings.sync.deviceName": "Nombre de este dispositivo",
  "settings.sync.deviceName.default": "Mosaic en {platform}",
  "settings.sync.password": "Contraseña maestra",
  "settings.sync.code": "Código de 6 dígitos",
  "settings.sync.server.title": "Servidor de Mosaic",
  "settings.sync.server.body":
    "Sincroniza a través de un servidor de Mosaic que administras tú o alguien de tu confianza.",
  "settings.sync.server.url": "Dirección del servidor",
  "settings.sync.server.urlHint": "Por ejemplo, https://sync.example.com",
  "settings.sync.server.check": "Comprobar servidor",
  "settings.sync.server.checking": "Comprobando el servidor…",
  "settings.sync.server.registration.open":
    "Este servidor permite que cualquiera cree una cuenta de sincronización.",
  "settings.sync.server.registration.invite":
    "Este servidor necesita un código de invitación para crear una cuenta de sincronización.",
  "settings.sync.server.registration.closed":
    "Este servidor no acepta cuentas de sincronización nuevas. Aún puedes iniciar sesión en una existente.",
  "settings.sync.server.choice.label": "Cuenta de sincronización",
  "settings.sync.server.choice.signIn": "Tengo una cuenta de sincronización",
  "settings.sync.server.choice.create": "Crear una",
  "settings.sync.server.invite": "Código de invitación",
  "settings.sync.server.create": "Crear cuenta de sincronización",
  "settings.sync.server.signIn": "Iniciar sesión",
  "settings.sync.server.signInAgain": "Iniciar sesión de nuevo",
  "settings.sync.server.working": "Conectando…",
  "settings.sync.server.sees":
    "El servidor solo ve datos cifrados, el correo de tu cuenta y cuándo sincronizas. Nunca ve tu contraseña maestra ni tu Clave secreta.",
  "settings.sync.twoFactor.title": "Escribe tu código",
  "settings.sync.twoFactor.body":
    "Esta cuenta de sincronización usa verificación en dos pasos. Escribe el código de 6 dígitos de tu app de autenticación.",
  "settings.sync.twoFactor.submit": "Verificar",
  "settings.sync.folder.title": "Tu propia carpeta en la nube",
  "settings.sync.folder.body":
    "Sincroniza a través de una carpeta que tu servicio en la nube (Dropbox, OneDrive, iCloud Drive y otros) mantiene actualizada.",
  "settings.sync.folder.choose": "Elegir carpeta…",
  "settings.sync.folder.chooseOther": "Elegir otra carpeta…",
  "settings.sync.folder.path": "Carpeta",
  "settings.sync.folder.contents.empty": "La carpeta está vacía. Mosaic puede usarla.",
  "settings.sync.folder.contents.mosaic":
    "Esta carpeta ya contiene datos de sincronización de Mosaic.",
  "settings.sync.folder.contents.other":
    "Esta carpeta contiene otros archivos. Mosaic no escribe nada allí. Elige una carpeta vacía o crea una nueva.",
  "settings.sync.folder.connect": "Conectar carpeta",
  "settings.sync.folder.working": "Conectando…",
  "settings.sync.folder.sees":
    "Tu proveedor de la nube y cualquiera con acceso a la carpeta ven solo datos cifrados, el ID de la cuenta y cuándo cambian los archivos. Nunca ven tu contraseña maestra ni tu Clave secreta.",
  "settings.sync.status.label": "Estado",
  "settings.sync.lastSynced": "Última sincronización {when}",
  "settings.sync.neverSynced": "Aún sin sincronizar",
  "settings.sync.offline":
    "No tienes conexión. Tus cambios están guardados en este dispositivo y se sincronizarán cuando vuelvas a tener conexión.",
  "settings.sync.now": "Sincronizar ahora",
  "settings.sync.backend.server": "Servidor",
  "settings.sync.backend.account": "Cuenta de sincronización",
  "settings.sync.backend.folder": "Carpeta",
  "settings.sync.backend.device": "Este dispositivo",
  "settings.sync.signedOut.title": "La sesión de este dispositivo está cerrada",
  "settings.sync.signedOut.body":
    "El servidor terminó la sesión de este dispositivo. Inicia sesión de nuevo para seguir sincronizando. Tus cambios están a salvo en este dispositivo.",
  "settings.sync.recover.title": "Se cambió tu contraseña maestra",
  "settings.sync.recover.body":
    "El servidor o la carpeta tiene una contraseña maestra más reciente que este dispositivo. Escribe la nueva contraseña maestra para poner este dispositivo al día. Tus cajas fuertes y tu Clave secreta siguen igual.",
  "settings.sync.recover.password": "Nueva contraseña maestra",
  "settings.sync.recover.submit": "Actualizar este dispositivo",
  "settings.sync.disconnect": "Desconectar…",
  "settings.sync.disconnect.title": "¿Desconectar la sincronización?",
  "settings.sync.disconnect.body":
    "Este dispositivo deja de sincronizar y cierra sesión en el servidor. Tus cajas fuertes se quedan en este dispositivo y tus otros dispositivos conservan sus propias copias.",
  "settings.sync.disconnect.confirm": "Desconectar",
  "settings.sync.devices.title": "Dispositivos",
  "settings.sync.devices.thisDevice": "Este dispositivo",
  "settings.sync.devices.unnamed": "Dispositivo sin nombre",
  "settings.sync.devices.lastSeen": "Visto por última vez {when}",
  "settings.sync.devices.neverSeen": "Aún no se ha visto",
  "settings.sync.devices.rename": "Cambiar nombre",
  "settings.sync.devices.renameNamed": "Cambiar el nombre de {name}",
  "settings.sync.devices.renameLabel": "Nombre nuevo para este dispositivo",
  "settings.sync.devices.remove": "Quitar",
  "settings.sync.devices.removeNamed": "Quitar {name}",
  "settings.sync.devices.removeTitle": "¿Quitar {name}?",
  "settings.sync.devices.removeBody":
    "Quitar un dispositivo cierra su sesión de sincronización. No le quita la copia de tus cajas fuertes que ya tiene. Para dejar fuera un dispositivo de forma definitiva, cambia tu contraseña maestra y tu Clave secreta.",
  "settings.sync.devices.removeConfirm": "Quitar dispositivo",
  "settings.sync.devices.folderNote":
    "Para dejar fuera un dispositivo, cambia tu contraseña maestra y tu Clave secreta.",
  "settings.sync.phone.title": "Configurar un teléfono",
  "settings.sync.phone.body":
    "Muestra un código que tu teléfono escanea, para que no tengas que escribir tu Clave secreta. El teléfono sigue necesitando tu contraseña maestra.",
  "settings.sync.phone.button": "Configurar un teléfono",
  "settings.sync.phone.reauth":
    "El código contiene tu Clave secreta, por eso necesita tu contraseña maestra.",
  "settings.sync.phone.dialogTitle": "Escanea este código con tu teléfono",
  "settings.sync.phone.steps":
    "En tu teléfono, abre Mosaic, elige configurarlo y escanea este código.",
  "settings.sync.phone.hint":
    "El código se oculta a los 60 segundos o cuando sales de esta ventana. No compartas una foto de él.",
  "settings.sync.phone.canvas": "Código de configuración para tu teléfono",
  "settings.sync.phone.hidden.time": "Pasaron los 60 segundos, así que el código está oculto.",
  "settings.sync.phone.hidden.away": "Saliste de esta ventana, así que el código está oculto.",
  "settings.sync.phone.again": "Mostrar de nuevo",
  "settings.sync.phone.folderHint":
    "Tu teléfono inicia sesión en tu almacenamiento en la nube y abre esta misma carpeta.",
  "settings.sync.totp.title": "App de autenticación",
  "settings.sync.totp.body":
    "Pide un código de una app de autenticación cuando alguien inicia sesión en tu cuenta de sincronización. Protege la cuenta del servidor, no tus cajas fuertes: estas siguen cifradas con tu contraseña maestra y tu Clave secreta.",
  "settings.sync.totp.on": "Los códigos de una app de autenticación están activados.",
  "settings.sync.totp.off": "Los códigos de una app de autenticación están desactivados.",
  "settings.sync.totp.enable": "Activar…",
  "settings.sync.totp.disable": "Desactivar…",
  "settings.sync.totp.beginTitle": "Activar los códigos de una app de autenticación",
  "settings.sync.totp.beginBody": "Escribe tu contraseña maestra para empezar.",
  "settings.sync.totp.begin": "Continuar",
  "settings.sync.totp.setupBody":
    "Añade esta clave a tu app de autenticación y luego escribe el código de 6 dígitos que muestra.",
  "settings.sync.totp.secret": "Clave de configuración",
  "settings.sync.totp.confirm": "Activar",
  "settings.sync.totp.disableTitle": "Desactivar los códigos de una app de autenticación",
  "settings.sync.totp.disableBody": "Escribe tu contraseña maestra y un código actual.",
  "settings.sync.totp.disableConfirm": "Desactivar",
  "settings.sync.totp.enabledToast": "Los códigos de una app de autenticación están activados.",
  "settings.sync.totp.disabledToast": "Los códigos de una app de autenticación están desactivados.",
  "settings.sync.key.title": "Llaves de seguridad",
  "settings.sync.key.body":
    "Una llave de seguridad puede ser un segundo paso al iniciar sesión en tu cuenta de sincronización. Protege la cuenta en el servidor, no tus cajas fuertes. Añadir o usar una llave abre tu navegador.",
  "settings.sync.key.none": "No se ha añadido ninguna llave de seguridad.",
  "settings.sync.key.added": "Añadida {when}",
  "settings.sync.key.lastUsed": "Último uso {when}",
  "settings.sync.key.neverUsed": "Aún sin usar",
  "settings.sync.key.add": "Añadir una llave de seguridad…",
  "settings.sync.key.addTitle": "Añadir una llave de seguridad",
  "settings.sync.key.addBody":
    "Ponle un nombre a la llave y escribe tu contraseña maestra. Después Mosaic abre tu navegador, donde usas la llave.",
  "settings.sync.key.label": "Nombre de la llave",
  "settings.sync.key.labelHint": "Por ejemplo, “Llave del escritorio”. Hasta 64 caracteres.",
  "settings.sync.key.addConfirm": "Continuar",
  "settings.sync.key.addedToast": "Llave de seguridad añadida.",
  "settings.sync.key.noBackup":
    "Una llave de seguridad sería tu único segundo paso. Si la pierdes, puede que no puedas volver a iniciar sesión en tu cuenta de sincronización. Recomendamos una copia de respaldo: una segunda llave o un código de una app de autenticación.",
  "settings.sync.key.recommend":
    "Recomendamos dos métodos, por ejemplo dos llaves, o una llave y un código de una app de autenticación.",
  "settings.sync.key.remove": "Quitar",
  "settings.sync.key.removeLabel": "Quitar {name}",
  "settings.sync.key.removeTitle": "¿Quitar {name}?",
  "settings.sync.key.removeBody":
    "Confirmas una vez con una llave de seguridad en tu navegador. Después, esta llave deja de servir para iniciar sesión.",
  "settings.sync.key.removeLast":
    "Este es tu último segundo paso. La verificación en dos pasos se desactiva y tu cuenta de sincronización quedará protegida solo por tu contraseña maestra y tu Clave secreta.",
  "settings.sync.key.removeCode": "Código de la app de autenticación",
  "settings.sync.key.removeCodeHint":
    "Escribe un código actual para quitar la llave de inmediato. Déjalo vacío para confirmar con una llave de seguridad en tu navegador.",
  "settings.sync.key.removeConfirm": "Quitar llave",
  "settings.sync.key.removedToast": "Llave de seguridad quitada.",
  "settings.sync.key.confirmTitle": "Confirma con una llave de seguridad",
  "settings.sync.key.confirmBody":
    "Quitar una llave requiere un segundo paso. Escribe tu contraseña maestra y confirma con una llave en tu navegador.",
  "settings.sync.key.step.methods": "Comprobando cómo inicia sesión esta cuenta…",
  "settings.sync.key.step.useKey": "Usar llave de seguridad (abre tu navegador)",
  "settings.sync.key.step.keyOnlyTitle": "Usa tu llave de seguridad",
  "settings.sync.key.step.keyOnlyBody":
    "Esta cuenta está protegida por una llave de seguridad. Mosaic abre una página en tu navegador, donde la usas.",
  "settings.sync.key.step.or": "O usa un código",
  "settings.sync.key.wait.login": "Inicia sesión con tu llave de seguridad",
  "settings.sync.key.wait.register": "Añade tu llave de seguridad",
  "settings.sync.key.wait.confirm": "Confirma con tu llave de seguridad",
  "settings.sync.key.wait.lead":
    "Tu navegador abrió una página de Mosaic de tu servidor de sincronización. Escribe allí este código y luego usa tu llave de seguridad.",
  "settings.sync.key.wait.codeLabel": "Código para escribir en tu navegador",
  "settings.sync.key.wait.codeSpoken": "Código para escribir en tu navegador: {digits}",
  "settings.sync.key.wait.expires": "Este paso caduca a las {time}.",
  "settings.sync.key.wait.waiting": "Esperando tu llave de seguridad…",
  "settings.sync.key.wait.done": "Listo.",
  "settings.sync.key.wait.expired": "Este paso agotó su tiempo. Vuelve a empezar.",
  "settings.sync.key.wait.failed":
    "El paso falló: el código fue incorrecto demasiadas veces o no se aceptó la llave de seguridad. No se cambió nada. Vuelve a empezar.",
  "settings.sync.key.wait.reopen": "Abrir de nuevo la página del navegador",
  "settings.sync.key.wait.retry": "Volver a empezar",
  "sync.warning.region": "Avisos de sincronización",
  "sync.warning.unknownDevice": "otro dispositivo",
  "sync.warning.unknownVault": "una de tus cajas fuertes",
  "sync.warning.freezeSuspected":
    "Los cambios de {device} llevan más de un día sin llegar. Comprueba que el otro dispositivo esté encendido y con conexión.",
  "sync.warning.missingFiles":
    "Faltan en la carpeta archivos de {device}. Deja que tu servicio en la nube termine de sincronizar y vuelve a sincronizar.",
  "sync.warning.noCommonView":
    "Este dispositivo y {device} llevan dos días sin ver los mismos datos del servidor. Puede que el servidor les muestre versiones distintas.",
  "sync.warning.rollbackDetected":
    "Tus datos de sincronización parecen más antiguos que los de este dispositivo. Revisa el servidor o la carpeta y luego elige “Ya lo comprobé”.",
  "sync.warning.identityDuplicated":
    "Otra copia de Mosaic está escribiendo con la identidad de este dispositivo, por ejemplo tras restaurar una copia de seguridad. La sincronización está en pausa. Asegúrate de que solo se use una copia y luego elige “Ya lo comprobé”.",
  "sync.warning.serverInconsistent":
    "El servidor mostró a este dispositivo datos distintos a los de otro dispositivo. Revisa el servidor y luego elige “Ya lo comprobé”.",
  "sync.warning.serverRollback":
    "La copia que el servidor tiene de {vault} es más antigua que la de este dispositivo. Revisa el servidor y luego elige “Ya lo comprobé”.",
  "sync.warning.undecryptable":
    "{count, plural, one {No se pudo leer en este dispositivo # elemento recibido.} other {No se pudieron leer en este dispositivo # elementos recibidos.}} Actualiza Mosaic en todos tus dispositivos y vuelve a sincronizar.",
  "sync.warning.hardwareVaultsLocal":
    "Las cajas fuertes protegidas por hardware se quedan en este dispositivo. {count, plural, one {# caja fuerte no se sincroniza.} other {# cajas fuertes no se sincronizan.}}",
  "sync.warning.attachmentsNotRepaired":
    "{count, plural, one {# archivo del servidor o de la carpeta no se pudo reparar desde este dispositivo.} other {# archivos del servidor o de la carpeta no se pudieron reparar desde este dispositivo.}} Abre el elemento en un dispositivo que aún tenga el archivo y añádelo de nuevo.",
  "sync.warning.acknowledge": "Ya lo comprobé…",
  "sync.acknowledge.title": "¿Borrar los avisos de sincronización que requieren una comprobación?",
  "sync.acknowledge.body":
    "Continúa solo después de haber comprobado tú mismo el servidor o la carpeta. Esto borra todos los avisos que requieren una comprobación y la sincronización continúa. Si tus datos realmente se revirtieron o se modificaron, Mosaic no podrá detectarlo después.",
  "sync.acknowledge.eachDevice": "Haz esto en cada uno de tus dispositivos.",
  "toast.syncAttention":
    "La sincronización combinó o reparó algunos elementos de otro dispositivo. Echa un vistazo a lo que cambió.",
  "sync.revoked.rekey":
    "Las claves de esta cuenta se renovaron en otro dispositivo: inicia sesión con la nueva Clave secreta de tu Kit de emergencia más reciente.",
  "sync.revoked.passwordChange":
    "Tu contraseña maestra se cambió en otro dispositivo: inicia sesión con la nueva contraseña.",
  "sync.revoked.deviceRemoved":
    "Este dispositivo se quitó de tu cuenta desde otro dispositivo. Inicia sesión de nuevo para volver a añadirlo.",
  "sync.revoked.operator":
    "El operador del servidor terminó esta sesión. Inicia sesión de nuevo para seguir sincronizando.",
} satisfies Record<keyof typeof en, string>;
