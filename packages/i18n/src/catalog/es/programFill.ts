/**
 * Spanish UI strings: programFill. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { programFill as en } from "../en/programFill.ts";

export const programFill = {
  "programFill.chip.label": "Rellenar con Mosaic",
  "programFill.chip.locked": "Desbloquea Mosaic para rellenar",
  "programFill.list.title": "Inicios de sesión para {program}",
  "programFill.list.none": "Aún no hay un inicio de sesión para {program}",
  "programFill.list.signerChanged": "Cambió el firmante de este programa. ¿Vincular de nuevo?",
  "programFill.list.fileChanged": "Este programa cambió desde que lo vinculaste. ¿Vincular de nuevo?",
  "programFill.list.suggestions": "Sugerencias",
  "programFill.list.search": "Buscar inicios de sesión",
  "programFill.list.link": "Vincular un inicio de sesión…",
  "programFill.list.block": "No ofrecer para este programa",
  "programFill.link.packaged": "{name} · App de la tienda · Editor {publisher}",
  "programFill.link.packagedNoPublisher": "{name} · App de la tienda",
  "programFill.link.signed": "{exe} · Firmado por {signer}, comprobado por Windows",
  "programFill.link.unsigned": "{exe} · Sin firmar · {path}",
  "programFill.link.unsignedNote":
    "Mosaic rellenará solo este archivo exacto. Si cambia, Mosaic volverá a preguntar.",
  "programFill.link.lookalike":
    "Cuidado: este nombre se parece a {similar}, que vinculaste antes.",
  "programFill.link.hashChange": "Archivo vinculado {old}…, este archivo {new}…",
  "programFill.link.login": "Inicio de sesión: {title}",
  "programFill.link.confirm": "Vincular e iniciar sesión",
  "programFill.link.confirmFill": "Vincular y rellenar",
  "programFill.link.confirmUnsigned": "Vincular este programa sin firmar",
  "programFill.link.confirmAgain": "Confirma de nuevo para vincular",
  "programFill.notice.elevated":
    "Mosaic no puede rellenar programas que se ejecutan como administrador. Copia la contraseña desde Mosaic.",
  "programFill.notice.noField": "Mosaic no ve aquí ningún campo de inicio de sesión.",
  "programFill.notice.aborted":
    "Mosaic dejó de rellenar porque la ventana cambió. Revisa el campo.",
  "programFill.notice.abortedCleared":
    "Mosaic dejó de rellenar porque la ventana cambió y borró lo que había escrito.",
  "programFill.notice.modifiersHeld":
    "Suelta Mayús, Ctrl, Alt y la tecla de Windows, y vuelve a elegir el inicio de sesión.",
  "programFill.notice.notResponding":
    "Este programa no respondió a tiempo. No se rellenó nada más.",
  "programFill.notice.close": "Cerrar",
  "programFill.settings.title": "Rellenar en programas",
  "programFill.settings.description":
    "Mosaic observa qué campo tiene el foco en otros programas de este dispositivo para ofrecerte tus inicios de sesión allí. No se guarda ni se envía nada.",
  "programFill.settings.enable": "Ofrecer inicios de sesión en programas",
  "programFill.settings.submit": "Iniciar sesión tras rellenar",
  "programFill.settings.hotkey": "Atajo para abrir la lista",
  "programFill.settings.hotkeyNone": "Sin atajo",
  "programFill.settings.hotkeySave": "Guardar atajo de la lista",
  "programFill.settings.blocked": "Programas para los que Mosaic no ofrece inicios de sesión",
  "programFill.settings.blockedNone": "Ninguno",
  "programFill.settings.unblock": "Volver a ofrecer",
  "programFill.settings.notWindows": "Por ahora solo en Windows.",
  "programFill.settings.mosaicElevated":
    "Mosaic se ejecuta como administrador, por lo que no rellena programas. Inícialo de forma normal.",
  "programFill.settings.hostFailed":
    "Mosaic no pudo empezar a observar los programas. Reinicia Mosaic para volver a intentarlo.",
  "programFill.card.title": "Rellena también inicios de sesión en programas",
  "programFill.card.turnOn": "Activar",
  "programFill.card.notNow": "Ahora no",
  "programFill.item.program": "Programa",
  "programFill.item.storeApp": "App de la tienda",
  "programFill.item.signedBy": "Firmado por {signer}",
  "programFill.item.unsigned": "Sin firmar: {path}",
  "programFill.item.submit": "Iniciar sesión tras rellenar",
  "programFill.item.unlink": "Desvincular",
} satisfies Record<keyof typeof en, string>;
