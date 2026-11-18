/**
 * Spanish UI strings: onboarding. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { onboarding as en } from "../en/onboarding.ts";

export const onboarding = {
  "onboarding.secretKey.title": "Tu Clave secreta",
  "onboarding.secretKey.body":
    "Tu Clave secreta y tu contraseña maestra juntas desbloquean tu caja fuerte. No podemos recuperar ninguna de las dos.",
  "onboarding.secretKey.verify":
    "Escribe el último grupo de tu Clave secreta para confirmar que la guardaste.",
  "onboarding.secretKey.mistyped":
    "No coincide. Revisa el último grupo en tu Kit de emergencia.",
  "onboarding.step": "Paso {current} de {total}",
  "onboarding.create.title": "Crea tu cuenta",
  "onboarding.create.body":
    "Tu contraseña maestra y una Clave secreta que generamos en este dispositivo desbloquean juntas tu caja fuerte. Ninguna de las dos se envía nunca al servidor.",
  "onboarding.create.email": "Correo electrónico",
  "onboarding.create.password": "Contraseña maestra",
  "onboarding.create.passwordHint":
    "Cuatro palabras sin relación entre sí funcionan bien. Úsala solo para Mosaic.",
  "onboarding.create.confirm": "Confirmar contraseña maestra",
  "onboarding.create.mismatch":
    "Las dos contraseñas no coinciden. Escribe la misma contraseña maestra en ambos campos.",
  "onboarding.create.submit": "Crear cuenta",
  "onboarding.create.working": "Creando tu cuenta. Tarda unos segundos.",
  "onboarding.kit.accountId": "ID de la cuenta",
  "onboarding.kit.secretKey": "Clave secreta",
  "onboarding.kit.lastGroup": "Último grupo de tu Clave secreta",
  "onboarding.kit.confirm": "Confirmar",
  "kit.title": "Guarda tu Kit de emergencia",
  "kit.body":
    "El kit es un PDF con tu Clave secreta. Junto con tu contraseña maestra abre tu caja fuerte en un dispositivo nuevo. Guárdalo en un lugar seguro, lejos de este equipo.",
  "kit.secretKey": "Tu Clave secreta",
  "kit.save": "Guardar Kit de emergencia (PDF)",
  "kit.saved": "Guardado. Conserva el archivo en un lugar seguro.",
  "kit.cancelled": "Aún no se ha guardado. Guarda el kit para continuar.",
  "kit.recover.alreadySaved": "Ya lo guardé",
  "kit.recover.title": "Guarda tu Kit de emergencia de nuevo",
  "kit.recover.body":
    "Aún no has confirmado tu Kit de emergencia. Escribe tu contraseña maestra para guardarlo de nuevo y luego confirma el último grupo de tu Clave secreta.",
  "onboarding.import.title": "Trae tus contraseñas",
  "onboarding.import.body":
    "Desde Chrome, Edge, Firefox, Safari y otros navegadores, o desde 1Password, Bitwarden, LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm, KeePass y Enpass. Lleva unos minutos. También puedes hacerlo más tarde en Ajustes.",
  "onboarding.import.start": "Importar desde otra app",
  "onboarding.import.skip": "Omitir por ahora",
  "onboarding.done.title": "Todo listo",
  "onboarding.done.body":
    "Tu caja fuerte está lista. Bloquéala cuando quieras con el botón Bloquear.",
  "onboarding.browser.title": "Instala la extensión del navegador",
  "onboarding.browser.body":
    "Rellena inicios de sesión en tu navegador. La extensión solo rellena cuando haces clic en ella o usas su atajo de teclado.",
  "onboarding.browser.steps": "Cómo conectar un navegador",
  "onboarding.browser.step1": "Instala la extensión de Mosaic en tu navegador.",
  "onboarding.browser.step2": "Abre la extensión y elige Conectar.",
  "onboarding.browser.step3":
    "Comprueba que el código coincide y elige Permitir en la ventana que se abre aquí.",
  "onboarding.browser.notInStores":
    "La extensión aún no está en las tiendas de los navegadores. Puedes conectarla más tarde en Ajustes → Navegadores.",
  "onboarding.browser.getFor": "Obtener para {browser}",
  "onboarding.browser.connectedList": "Navegadores conectados",
  "onboarding.browser.waiting":
    "Esperando a la extensión. Aún no hay ningún navegador conectado.",
  "onboarding.browser.connected": "{name} conectado",
  "onboarding.browser.done": "Listo",
  "onboarding.browser.skip": "Omitir por ahora",
  "onboarding.progress": "Progreso de la configuración",
  "onboarding.stepName.create": "Crear cuenta",
  "onboarding.stepName.kit": "Kit de emergencia",
  "onboarding.stepName.verify": "Confirmar tu kit",
  "onboarding.stepName.import": "Importar (opcional)",
  "onboarding.stepName.browser": "Extensión del navegador (opcional)",
  "kit.preview": "Vista previa del Kit de emergencia",
  "kit.continueHint": "Guarda el kit para continuar.",
  "kit.warning":
    "Sin este kit y tu contraseña maestra, nadie puede recuperar tu caja fuerte, ni siquiera nosotros.",
  "onboarding.join.link": "Ya uso Mosaic",
  "onboarding.join.title": "Añade este dispositivo a tu cuenta",
  "onboarding.join.body":
    "Inicia sesión con tu contraseña maestra y la Clave secreta de tu Kit de emergencia. Después, tus cajas fuertes se descargan y se descifran en este dispositivo.",
  "onboarding.join.tabs": "¿Dónde están tus datos de sincronización?",
  "onboarding.join.tab.server": "Servidor de Mosaic",
  "onboarding.join.tab.folder": "Carpeta en la nube",
  "onboarding.join.serverUrl": "Dirección del servidor",
  "onboarding.join.email": "Correo de la cuenta de sincronización",
  "onboarding.join.password": "Contraseña maestra",
  "onboarding.join.secretKey": "Clave secreta",
  "onboarding.join.secretKeyHint": "De tu Kit de emergencia. Empieza por M1.",
  "onboarding.join.deviceName": "Nombre de este dispositivo",
  "onboarding.join.chooseFolder": "Elegir carpeta…",
  "onboarding.join.folder": "Carpeta",
  "onboarding.join.noAccounts":
    "Esta carpeta no contiene ninguna cuenta de Mosaic. Elige la carpeta con la que se sincroniza tu otro dispositivo.",
  "onboarding.join.account": "Cuenta",
  "onboarding.join.accountOption": "Cuenta {id}",
  "onboarding.join.submit": "Añadir este dispositivo",
  "onboarding.join.working": "Iniciando sesión y descargando tus cajas fuertes…",
  "onboarding.join.twoFactor.title": "Escribe tu código",
  "onboarding.join.twoFactor.body":
    "Tu cuenta de sincronización usa verificación en dos pasos. Escribe el código de 6 dígitos de tu app de autenticación.",
  "onboarding.join.twoFactor.code": "Código de 6 dígitos",
  "onboarding.join.twoFactor.submit": "Verificar",
  "onboarding.join.back": "Crear una cuenta nueva",
} satisfies Record<keyof typeof en, string>;
