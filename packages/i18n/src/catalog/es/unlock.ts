/**
 * Spanish UI strings: unlock. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { unlock as en } from "../en/unlock.ts";

export const unlock = {
  "lock.unlockTitle": "Desbloquear Mosaic",
  "lock.masterPassword": "Contraseña maestra",
  "lock.wrongPassword":
    "Esa contraseña maestra no funcionó. Revisa las mayúsculas y la distribución del teclado y vuelve a intentarlo.",
  "lock.locked": "Mosaic está bloqueado.",
  "session.unlocked": "Mosaic está desbloqueado.",
  "lock.secretKey": "Clave secreta",
  "lock.secretKeyHint":
    "Este dispositivo aún no tiene tu Clave secreta. Escríbela desde tu Kit de emergencia y Mosaic la recordará en este dispositivo.",
  "lock.submit": "Desbloquear",
  "lock.working": "Desbloqueando…",
  "lock.reason.manual": "Bloqueaste Mosaic.",
  "lock.reason.idle": "Mosaic se bloqueó solo tras un rato sin actividad.",
  "lock.reason.sleep": "Mosaic se bloqueó cuando el equipo entró en suspensión.",
  "lock.reason.screenLock": "Mosaic se bloqueó junto con tu pantalla.",
  "lock.or": "o",
  "lock.quick.biometricWindows": "Usar Windows Hello",
  "lock.quick.biometricMac": "Usar Touch ID",
  "lock.quick.biometricLinux": "Usar biometría",
  "lock.quick.hardwareKey": "Usar llave de seguridad",
  "lock.autoLockInfo":
    "Se bloquea tras {minutes, plural, one {# minuto} other {# minutos}} de inactividad.",
  "lock.forgot.title": "¿Olvidaste tu contraseña maestra?",
  "lock.forgot.body":
    "Mosaic no puede restablecerla, porque solo tú la conoces. Comprueba si la anotaste en tu Kit de emergencia. Si no, crea una cuenta nueva.",
  "lock.quick.waitingWindows": "Esperando a Windows Hello…",
  "lock.quick.waitingMac": "Esperando a Touch ID…",
  "lock.quick.touchKey": "Inserta tu llave y tócala cuando parpadee.",
  "lock.quick.pin": "PIN de la llave de seguridad",
  "lock.quick.pinSubmit": "Continuar",
  "lock.quick.cancel": "Cancelar",
} satisfies Record<keyof typeof en, string>;
