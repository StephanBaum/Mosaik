/**
 * Spanish UI strings: otp. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { otp as en } from "../en/otp.ts";

export const otp = {
  "otp.section.label": "Código de un solo uso (2FA)",
  "otp.section.empty":
    "Añade el código de la página de configuración de la verificación en dos pasos del sitio.",
  "otp.section.required": "Añade un código de un solo uso para guardar este elemento.",
  "otp.source.screen": "Escanear pantalla",
  "otp.source.clipboard": "Pegar captura",
  "otp.source.file": "Elegir imagen…",
  "otp.source.manual": "¿No puedes escanear? Escribe la clave",
  "otp.source.screenHint":
    "Mosaic se aparta un momento, mira tu pantalla una vez y solo se queda con el código.",
  "otp.source.clipboardHint":
    "{platform, select, mac {Pulsa Cmd+Ctrl+Mayús+4, selecciona el código QR y pégalo aquí.} windows {Pulsa Win+Mayús+S, selecciona el código QR y pégalo aquí.} other {Copia una captura de pantalla del código QR y pégala aquí.}}",
  "otp.dialog.title": "Configurar un código de un solo uso",
  "otp.working.screen": "Buscando un código QR en tu pantalla…",
  "otp.working.image": "Leyendo el código QR…",
  "otp.manual.title": "Escribe la clave de configuración",
  "otp.manual.input": "Clave de configuración o enlace otpauth://",
  "otp.manual.inputHint":
    "El sitio la muestra junto al código QR, a menudo bajo \"¿No puedes escanear?\" o \"Introducir manualmente\". Los espacios no importan.",
  "otp.manual.issuer": "Servicio",
  "otp.manual.account": "Cuenta",
  "otp.manual.advanced": "Avanzado",
  "otp.manual.algorithm": "Algoritmo",
  "otp.manual.digits": "Dígitos",
  "otp.manual.period": "Código nuevo cada",
  "otp.manual.periodValue": "{seconds} segundos",
  "otp.manual.advancedHint":
    "Cambia esto solo si el sitio lo indica. La mayoría de los sitios usan SHA-1, 6 dígitos y 30 segundos.",
  "otp.manual.continue": "Continuar",
  "otp.confirm.title": "¿Es este el código correcto?",
  "otp.confirm.issuer": "Servicio",
  "otp.confirm.account": "Cuenta",
  "otp.confirm.details":
    "{scheme, select, hotp {Basado en contador, {digits} dígitos} steam {Steam Guard, 5 caracteres} other {{digits} dígitos, código nuevo cada {period} segundos}}",
  "otp.confirm.algorithmValue":
    "{algorithm, select, sha256 {SHA-256} sha512 {SHA-512} other {SHA-1}}",
  "otp.confirm.codeLabel": "Código actual",
  "otp.confirm.codeHint":
    "Si el sitio pide un código para terminar la configuración, escribe este.",
  "otp.confirm.hotpNote":
    "Los códigos basados en contador aparecen después de guardar, para que el contador siga sincronizado con el sitio.",
  "otp.confirm.weak":
    "Este secreto es inusualmente corto. Funciona, pero la verificación en dos pasos de este sitio es más débil de lo normal.",
  "otp.confirm.use": "Usar este código",
  "otp.confirm.back": "Escanear de nuevo",
  "otp.confirm.copy": "Copiar código",
  "otp.summary.noIssuer": "Código de un solo uso",
  "otp.summary.pending": "Se guarda junto con este elemento.",
  "otp.summary.removed": "El código de un solo uso se eliminará al guardar.",
  "otp.action.replace": "Reemplazar",
  "otp.action.remove": "Quitar código de un solo uso",
  "otp.action.keep": "Conservarlo",
  "otp.action.cancelSetup": "No añadir",
  "otp.storage.label": "Dónde guardar el código",
  "otp.storage.item": "Con este elemento",
  "otp.storage.itemTradeoff":
    "Es cómodo, pero quien abra esta caja fuerte obtiene tu contraseña y tu segundo factor a la vez.",
  "otp.storage.hardware": "En {vault} (requiere tu llave de seguridad)",
  "otp.storage.hardwareClosed": "Abre primero {vault} con tu llave de seguridad.",
  "otp.toast.clipboardCleared": "Captura leída. Mosaic borró el portapapeles.",
  "otp.toast.savedInVault": "Código guardado en {vault}.",
  "otp.clipboardCopies":
    "{platform, select, windows {Si el historial del portapapeles o la Herramienta Recortes guardó una copia de la captura, elimínala allí.} mac {Si también guardaste la captura como archivo, elimínalo.} other {Si un gestor del portapapeles guardó una copia de la captura, elimínala allí.}}",
} satisfies Record<keyof typeof en, string>;
