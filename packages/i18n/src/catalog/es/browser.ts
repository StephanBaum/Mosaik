/**
 * Spanish UI strings: browser. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { browser as en } from "../en/browser.ts";

export const browser = {
  "browser.pair.title": "Un navegador quiere conectarse con Mosaic",
  "browser.pair.description":
    "Permítelo solo si acabas de hacer clic en Conectar en la extensión de Mosaic y esta muestra este código.",
  "browser.pair.code": "Código",
  "browser.pair.browser": "Navegador",
  "browser.pair.label": "Nombre para este navegador",
  "browser.pair.labelHint": "Verás este nombre en Ajustes.",
  "browser.pair.deny": "Denegar",
  "browser.pair.allow": "Permitir",
  "browser.expires":
    "Se cierra automáticamente en {seconds, plural, one {# segundo} other {# segundos}}",
  "browser.escDenies": "para denegar",
  "browser.hint.chrome": "Chrome",
  "browser.hint.edge": "Edge",
  "browser.hint.brave": "Brave",
  "browser.hint.opera": "Opera",
  "browser.hint.vivaldi": "Vivaldi",
  "browser.hint.chromium": "Chromium",
  "browser.hint.firefox": "Firefox",
  "browser.hint.other": "Navegador",
  "browser.family.chromium": "Navegador basado en Chromium",
  "browser.family.firefox": "Firefox",
  "browser.escCancels": "para cancelar",
  "browser.save.titleNew": "¿Guardar este inicio de sesión en Mosaic?",
  "browser.save.titleUpdate": "¿Actualizar este inicio de sesión en Mosaic?",
  "browser.save.description":
    "{browser} ofrece guardar un inicio de sesión. Mosaic lo escribe solo si eliges Guardar.",
  "browser.save.descriptionUpdate":
    "{browser} ofrece una contraseña nueva para este inicio de sesión. Mosaic la cambia solo si eliges Actualizar contraseña.",
  "browser.save.site": "Sitio web",
  "browser.save.username": "Nombre de usuario",
  "browser.save.noUsername": "Ninguno",
  "browser.save.password": "Contraseña",
  "browser.save.passwordHidden": "Oculta",
  "browser.save.target": "Guardar como",
  "browser.save.updateOption": "Actualizar “{title}”",
  "browser.save.newOption": "Un inicio de sesión nuevo",
  "browser.save.itemTitle": "Título",
  "browser.save.vault": "Caja fuerte",
  "browser.save.allowHttp": "Permitir rellenar en esta página sin cifrar",
  "browser.save.allowHttpHint":
    "El sitio no usa HTTPS. Cualquiera en la red podría leer lo que rellenes.",
  "browser.save.cancel": "Cancelar",
  "browser.save.saveNew": "Guardar",
  "browser.save.update": "Actualizar contraseña",
  "browser.passkey.secure": "Conexión segura (https)",
  "browser.passkey.getTitle": "Iniciar sesión en {rpId} con una llave de acceso",
  "browser.passkey.createTitle": "Guardar una llave de acceso nueva",
  "browser.passkey.excludedTitle": "Llave de acceso ya guardada",
  "browser.passkey.createBody": "¿Crear una llave de acceso para {rpId} en Mosaic?",
  "browser.passkey.excludedBody": "Ya tienes una llave de acceso para {rpId} en Mosaic.",
  "browser.passkey.uvNote":
    "El sitio pidió verificar al usuario. Escribe tu contraseña maestra para confirmar que eres tú.",
  "browser.passkey.password": "Contraseña maestra",
  "browser.passkey.account": "Cuenta",
  "browser.passkey.noName": "Sin nombre",
  "browser.passkey.vault": "Caja fuerte",
  "browser.passkey.signIn": "Iniciar sesión con llave de acceso",
  "browser.passkey.save": "Guardar llave de acceso",
  "browser.passkey.anotherDevice": "Usar otro dispositivo o llave de seguridad",
  "browser.passkey.cancel": "Cancelar",
  "browser.passkey.ok": "Entendido",
  "browser.passkey.escCloses": "para cerrar",
  "browser.passkey.passwordNeeded":
    "Escribe tu contraseña maestra para confirmar que eres tú.",
  "browser.settings.title": "Navegadores",
  "browser.settings.body":
    "Extensiones de navegador que rellenan inicios de sesión desde esta app. Apruebas cada conexión aquí.",
  "browser.settings.empty":
    "Aún no hay ningún navegador conectado. Instala la extensión de Mosaic y elige Conectar en ella.",
  "browser.settings.connected": "Conectado ahora",
  "browser.settings.lastUsed": "Último uso: {when}",
  "browser.settings.neverUsed": "Aún sin usar",
  "browser.settings.disconnect": "Desconectar",
  "browser.settings.disconnectNamed": "Desconectar {name}",
  "browser.settings.disconnectTitle": "¿Desconectar “{name}”?",
  "browser.settings.disconnectBody":
    "Este navegador no podrá rellenar inicios de sesión hasta que lo vuelvas a conectar desde la extensión.",
  "browser.settings.endpointInUse":
    "Otro programa, u otra ventana de Mosaic, ya usa la conexión para navegadores. Ciérralo y reinicia Mosaic.",
  "browser.settings.endpointUnavailable":
    "Mosaic no pudo abrir la conexión para navegadores. Reinicia Mosaic. Si sigue ocurriendo, comprueba que se pueda escribir en tu carpeta de usuario.",
  "browser.settings.registryUnreadable":
    "No se pueden leer las conexiones de navegador guardadas. Mosaic no las modificará hasta que se solucione.",
  "browser.connector.title": "La conexión de un navegador apunta a otro lugar",
  "browser.connector.body":
    "La conexión de {browser} inicia un programa distinto de Mosaic: {path}. Puede ser un Mosaic anterior u otra cosa que se registró por su cuenta. Reparar la vuelve a dirigir a este Mosaic.",
  "browser.connector.item": "{browser}: {path}",
  "browser.connector.repair": "Reparar",
  "browser.connector.repaired": "Conexión del navegador reparada.",
} satisfies Record<keyof typeof en, string>;
