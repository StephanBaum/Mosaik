/**
 * Spanish UI strings: settings. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { settings as en } from "../en/settings.ts";

export const settings = {
  "settings.title": "Ajustes",
  "settings.security": "Seguridad",
  "settings.autoLock": "Bloquear tras inactividad",
  "settings.autoLock.never": "Nunca",
  "settings.autoLock.minutes": "{minutes, plural, one {# minuto} other {# minutos}}",
  "settings.lockOnSleep": "Bloquear cuando el equipo entre en suspensión",
  "settings.lockOnScreenLock": "Bloquear cuando se bloquee la pantalla",
  "settings.screenLock.unsupported":
    "Mosaic no puede saber cuándo se bloquea la pantalla en este sistema. Aun así, se bloquea tras un periodo de inactividad y cuando el equipo se reactiva.",
  "settings.lockTrigger.required":
    "Deja al menos un bloqueo activado. Mosaic no puede detectar una pantalla bloqueada en este sistema.",
  "settings.clipboard": "Borrar el portapapeles después de",
  "settings.clipboard.never": "Nunca",
  "settings.clipboard.seconds": "{seconds, plural, one {# segundo} other {# segundos}}",
  "settings.appearance": "Apariencia",
  "settings.theme": "Tema",
  "settings.theme.system": "Seguir el sistema",
  "settings.theme.light": "Claro",
  "settings.theme.dark": "Oscuro",
  "settings.language": "Idioma",
  "settings.language.system": "Seguir el sistema",
  "settings.language.machineNote":
    "Traducido con ayuda de IA. Si algo suena mal, avísanos.",
  "settings.import": "Importar",
  "settings.import.body":
    "Trae tus inicios de sesión desde un navegador u otro gestor de contraseñas. Mosaic te guía en la exportación y te ayuda a eliminar el archivo después.",
  "settings.import.open": "Importar contraseñas",
  "settings.ssh": "Agente SSH",
  "settings.ssh.enable": "Usar mis claves SSH desde Mosaic",
  "settings.ssh.hint":
    "Desactivado de forma predeterminada. El agente funciona solo mientras Mosaic está desbloqueado y te pregunta antes de cada uso.",
  "settings.ssh.status":
    "{count, plural, =0 {Ninguna clave ofrecida} one {# clave ofrecida} other {# claves ofrecidas}}",
  "settings.ssh.endpoint": "Establece SSH_AUTH_SOCK en",
  "settings.ssh.skipped": "Claves que no se ofrecen",
  "settings.kit": "Kit de emergencia",
  "settings.kit.save": "Guardar de nuevo el Kit de emergencia",
  "settings.saved": "Ajustes guardados.",
  "reauth.title": "Confirma con tu contraseña maestra",
  "reauth.body":
    "Este cambio hace que Mosaic sea menos seguro, por eso necesita tu contraseña maestra.",
  "reauth.bodyKit":
    "Tu Clave secreta está a punto de mostrarse en un archivo, por eso Mosaic necesita tu contraseña maestra.",
  "reauth.password": "Contraseña maestra",
  "reauth.submit": "Confirmar",
  "settings.intro":
    "Cómo se bloquea Mosaic, cómo borra secretos y cómo protege tus claves en este dispositivo.",
  "settings.section.autoLock": "Bloqueo automático",
  "settings.section.autoLock.body": "Cuándo se bloquea Mosaic solo, sin que lo pidas.",
  "settings.section.clipboard": "Portapapeles",
  "settings.section.clipboard.body": "Los secretos copiados se borran pasado este tiempo.",
  "settings.appearance.body": "El tema y el idioma de Mosaic.",
  "settings.ssh.body":
    "Permite que los programas inicien sesión con claves SSH de tu caja fuerte. Apruebas cada uso.",
  "settings.kit.body":
    "Guarda una copia nueva de tu Kit de emergencia. Requiere tu contraseña maestra.",
  "settings.quick": "Desbloqueo rápido",
  "settings.quick.body": "Formas más rápidas de volver a entrar, además de tu contraseña maestra.",
  "settings.quick.windowsHello": "Windows Hello",
  "settings.quick.touchId": "Touch ID",
  "settings.quick.biometric.body":
    "Desbloquea con tu rostro, tu huella o tu PIN. Tu contraseña maestra sigue haciendo falta tras un reinicio.",
  "settings.quick.biometric.bodyMac":
    "Desbloquea con tu huella. Tu contraseña maestra sigue haciendo falta tras un reinicio.",
  "settings.quick.expires":
    "Mosaic volverá a pedir tu contraseña maestra el {date, date, medium}.",
  "settings.quick.maxAge": "Pedir la contraseña maestra al menos cada",
  "settings.quick.maxAgeDays": "{days, plural, one {# día} other {# días}}",
  "settings.quick.biometricLinux": "El desbloqueo biométrico no está disponible en Linux.",
  "settings.quick.biometricUnavailable": "No está configurado en este dispositivo.",
  "settings.quick.keys": "Llaves de seguridad",
  "settings.quick.keysCount":
    "{count, plural, =0 {Ninguna registrada} one {# registrada} other {# registradas}}",
  "settings.quick.keysUnsupported": "Las llaves de seguridad aún no se admiten en este sistema.",
  "settings.quick.keyRow": "Llave de seguridad · {tag} · añadida el {addedAt, date, medium}",
  "settings.quick.addKey": "Añadir llave de seguridad",
  "settings.quick.addKeyBody":
    "Inserta la llave que quieres añadir y escribe su PIN. Luego tócala cuando parpadee.",
  "settings.quick.manageKeys": "Administrar",
  "settings.quick.removeKey": "Quitar",
  "settings.quick.removeKeyNamed": "Quitar la llave de seguridad {tag}",
  "settings.quick.removeKeyConfirm": "¿Quitar la llave de seguridad {tag}?",
  "settings.quick.keyRisk":
    "Cualquiera que tenga esta llave y su PIN puede abrir tu caja fuerte sin tu contraseña maestra.",
  "reauth.bodyQuick":
    "Esto facilita el acceso a Mosaic, por eso necesita tu contraseña maestra.",
  "reauth.bodyKey": "Añadir una llave a esta caja fuerte requiere tu contraseña maestra.",
  "settings.passkeyExport": "Exportar llaves de acceso",
  "settings.passkeyExport.body":
    "Guarda las llaves de acceso de una caja fuerte en un archivo FIDO CXF para moverlas a otra app.",
  "settings.passkeyExport.vault": "Caja fuerte",
  "settings.passkeyExport.warning":
    "El archivo contiene tus llaves de acceso sin cifrar. Cualquiera que lo tenga puede iniciar sesión como tú. Elimínalo después de importarlo.",
  "settings.passkeyExport.none": "Esta caja fuerte no tiene llaves de acceso que exportar.",
  "settings.passkeyExport.button": "Exportar llaves de acceso",
  "settings.passkeyExport.reauth":
    "El archivo contendrá tus llaves de acceso sin cifrar, por eso exportar requiere tu contraseña maestra.",
  "settings.passkeyExport.saved":
    "{count, plural, one {# llave de acceso exportada.} other {# llaves de acceso exportadas.}} Elimina el archivo después de importarlo.",
  "reauth.bodyMove":
    "Mover un elemento fuera de una caja fuerte protegida por una llave de seguridad requiere tu contraseña maestra.",
  "toast.settingsReset":
    "Tus ajustes de seguridad se cambiaron fuera de Mosaic y se restablecieron a valores seguros.",
  "settings.shortcut.title": "Atajo de búsqueda rápida",
  "settings.shortcut.body":
    "Pulsa esta combinación en cualquier programa para abrir la búsqueda de Mosaic.",
  "settings.shortcut.label": "Atajo",
  "settings.shortcut.hint":
    "Haz clic en el campo y pulsa las teclas que quieras: Ctrl, Alt o ⌘ (en un Mac), más otra tecla.",
  "settings.shortcut.save": "Guardar atajo",
  "settings.shortcut.reset": "Usar el predeterminado",
  "settings.shortcut.saved": "Atajo guardado: {shortcut}.",
  "settings.shortcut.invalid":
    "Ese atajo no lo puede usar Mosaic. Pulsa Ctrl, Alt o ⌘ (en un Mac) junto con otra tecla.",
  "settings.shortcut.unsupported":
    "Tu sistema no permite que las apps definan un atajo global, por lo que no se puede cambiar aquí.",
} satisfies Record<keyof typeof en, string>;
