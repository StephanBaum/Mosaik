/**
 * Spanish UI strings: shell. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { shell as en } from "../en/shell.ts";

export const shell = {
  "shell.emergencyKit.title": "Kit de emergencia de Mosaic",
  "shell.emergencyKit.intro":
    "Guarda esta página en un lugar seguro, por ejemplo en un cajón con llave. Tu Clave secreta y tu contraseña maestra juntas desbloquean tu caja fuerte en un dispositivo nuevo. Nadie, ni siquiera Mosaic, puede recuperar ninguna de las dos por ti.",
  "shell.emergencyKit.email": "Correo de la cuenta",
  "shell.emergencyKit.accountId": "ID de la cuenta",
  "shell.emergencyKit.signInUrl": "Dirección de inicio de sesión",
  "shell.emergencyKit.signInUrlLocal":
    "Solo este dispositivo (aún no hay un servidor de sincronización configurado)",
  "shell.emergencyKit.secretKey": "Clave secreta",
  "shell.emergencyKit.masterPassword": "Contraseña maestra",
  "shell.emergencyKit.masterPasswordHint":
    "Escríbela a mano en la línea solo si esta página se queda en un lugar seguro. Si no, memorízala.",
  "shell.emergencyKit.footer":
    "Mosaic es software previo a una auditoría: aún no lo ha revisado ningún auditor de seguridad externo.",
  "shell.shortcut.conflict":
    "Otra app ya usa {shortcut}. Cierra esa app o cambia su atajo y reinicia Mosaic para usar la búsqueda rápida.",
  "keys.ctrl": "Ctrl",
  "keys.shift": "Mayús",
  "keys.alt": "Alt",
  "keys.space": "Espacio",
  "keys.esc": "Esc",
  "keys.enter": "Intro",
} satisfies Record<keyof typeof en, string>;
