/**
 * Spanish UI strings: mobileUx. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { mobileUx as en } from "../en/mobileUx.ts";

export const mobileUx = {
  "mobile.sync.warningsBanner":
    "La sincronización encontró un problema. Abre Ajustes para revisarlo.",
  "mobile.provider.errorTitle": "Mosaic no puede continuar",
  "mobile.onboarding.details.keyWiped":
    "Mosaic eliminó la Clave secreta escaneada al salir de la app. Escanea el código de nuevo o escribe la clave.",
  "mobile.common.showField": "Mostrar {field}",
  "mobile.common.hideField": "Ocultar {field}",
  "mobile.error.storeInUse":
    "Mosaic está ocupado en otro lugar de este teléfono. Espera un momento y vuelve a intentarlo.",
  "mobile.error.noAccount":
    "Mosaic aún no está configurado en este teléfono. Abre Mosaic para configurarlo.",
  "mobile.error.wrongCredentials":
    "Esa contraseña maestra o Clave secreta no funcionó. Revisa que no haya erratas ni un idioma de teclado distinto y vuelve a intentarlo.",
  "mobile.error.invalidInput":
    "Algo de lo que escribiste no es válido. Revisa los datos e inténtalo de nuevo.",
  "mobile.error.vaultUnavailable":
    "Esta caja fuerte no se puede abrir ahora. Sincroniza e inténtalo de nuevo, o vuelve a desbloquear Mosaic.",
  "mobile.unlock.quickOptionTouchId": "Desbloquear con Touch ID la próxima vez",
  "mobile.detail.readCode": "Leer código",
  "mobile.detail.labelValue": "{label}: {value}",
  "mobile.autofill.warningWithName": "Comprueba primero la dirección | {name}",
  "mobile.autofill.linkEntry": "Vincular {login} y rellenar",
  "mobile.link.imported": "Importado para esta app",
  "mobile.link.confirmTitle": "¿Vincular {login} con {app}?",
  "mobile.link.confirmSubtitle": "ID de la app {appId} · Firma {fingerprint}",
  "mobile.link.confirmFallbackHint": "Tras vincular, toca el campo de nuevo para rellenarlo.",
  "mobile.link.importedNotVerified":
    "Inicio de sesión importado. El sitio web no ha confirmado esta app.",
} satisfies Record<keyof typeof en, string>;
