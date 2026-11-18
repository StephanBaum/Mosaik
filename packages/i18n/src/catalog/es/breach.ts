/**
 * Spanish UI strings: breach. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { breach as en } from "../en/breach.ts";

export const breach = {
  "breach.card.title": "Comprobación de filtraciones",
  "breach.card.off":
    "Desactivada. Mosaic nunca se conecta con Have I Been Pwned a menos que lo actives.",
  "breach.card.about":
    "Comprueba tus contraseñas con filtraciones de datos conocidas. De cada contraseña solo sale de este dispositivo un fragmento de 5 caracteres de un código codificado.",
  "breach.mode.label": "Comprobar con filtraciones de datos conocidas",
  "breach.mode.off": "Desactivada",
  "breach.mode.manual": "Solo cuando yo lo decida",
  "breach.mode.weekly": "Cada semana, tras desbloquear",
  "breach.card.checkNow": "Comprobar ahora",
  "breach.card.stop": "Detener comprobación",
  "breach.card.turnOff": "Desactivar",
  "breach.progress.starting": "Comprobando tus contraseñas…",
  "breach.progress.step": "Comprobando… {done} de {total}",
  "breach.result.never": "Aún no se ha comprobado.",
  "breach.result.last":
    "Última comprobación: {date, date, medium}. {found, plural, =0 {No se encontró ninguna contraseña en una filtración.} one {Se encontró # contraseña en una filtración.} other {Se encontraron # contraseñas en una filtración.}}",
  "breach.result.finished":
    "Se {checked, plural, one {comprobó # contraseña} other {comprobaron # contraseñas}}. {found, plural, =0 {No se encontró ninguna en una filtración.} one {Se encontró # en una filtración.} other {Se encontraron # en una filtración.}}",
  "breach.result.upToDate": "Todo se comprobó hace poco. No hay nada nuevo que consultar.",
  "breach.error.offline":
    "No se pudo conectar con Have I Been Pwned. Se muestran los últimos resultados. Vuelve a intentarlo cuando tengas conexión.",
  "breach.error.rateLimited":
    "Have I Been Pwned pidió a Mosaic que vaya más despacio. Vuelve a intentarlo en unos minutos.",
  "breach.error.protocol":
    "Have I Been Pwned envió una respuesta en la que Mosaic no confía, así que algunas contraseñas no se comprobaron. Vuelve a intentarlo más tarde.",
  "breach.error.cancelled":
    "Se detuvo la comprobación. Los resultados ya terminados se conservan.",
  "breach.consent.title": "¿Comprobar tus contraseñas con filtraciones de datos conocidas?",
  "breach.consent.body":
    "Mosaic convierte cada contraseña en un código codificado que no se puede revertir (un hash) y envía solo los primeros 5 caracteres de ese código a Have I Been Pwned (api.pwnedpasswords.com). El servicio conoce tu dirección IP y esos fragmentos de 5 caracteres. Nunca ve una contraseña, tu correo ni para qué sitio es cada contraseña. Los resultados se quedan en este dispositivo.",
  "breach.consent.detail":
    "Muchas otras contraseñas comparten cada fragmento de 5 caracteres, así que la respuesta no revela la tuya. Aun así, es una solicitud a otro servicio, por eso queda desactivada hasta que aceptes. Puedes desactivarla en cualquier momento.",
  "breach.consent.agree": "Activar y comprobar ahora",
  "breach.consent.decline": "Ahora no",
} satisfies Record<keyof typeof en, string>;
