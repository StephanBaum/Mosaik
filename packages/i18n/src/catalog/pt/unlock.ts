/**
 * Brazilian Portuguese UI strings: unlock. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { unlock as en } from "../en/unlock.ts";

export const unlock = {
  "lock.unlockTitle": "Desbloquear o Mosaic",
  "lock.masterPassword": "Senha mestra",
  "lock.wrongPassword":
    "Essa senha mestra não funcionou. Verifique o Caps Lock e o layout do teclado e tente de novo.",
  "lock.locked": "O Mosaic está bloqueado.",
  "session.unlocked": "O Mosaic está desbloqueado.",
  "lock.secretKey": "Chave Secreta",
  "lock.secretKeyHint":
    "Este dispositivo ainda não tem a sua Chave Secreta. Digite-a a partir do seu Kit de Emergência e o Mosaic a lembra neste dispositivo.",
  "lock.submit": "Desbloquear",
  "lock.working": "Desbloqueando…",
  "lock.reason.manual": "Você bloqueou o Mosaic.",
  "lock.reason.idle": "O Mosaic se bloqueou após um tempo sem atividade.",
  "lock.reason.sleep": "O Mosaic foi bloqueado quando o computador entrou em suspensão.",
  "lock.reason.screenLock": "O Mosaic foi bloqueado junto com a sua tela.",
  "lock.or": "ou",
  "lock.quick.biometricWindows": "Usar o Windows Hello",
  "lock.quick.biometricMac": "Usar o Touch ID",
  "lock.quick.biometricLinux": "Usar biometria",
  "lock.quick.hardwareKey": "Usar chave de segurança",
  "lock.autoLockInfo":
    "Bloqueia automaticamente após {minutes, plural, one {# minuto} other {# minutos}} de inatividade.",
  "lock.forgot.title": "Esqueceu a sua senha mestra?",
  "lock.forgot.body":
    "O Mosaic não pode redefini-la, porque só você a conhece. Verifique se você a anotou no seu Kit de Emergência. Caso contrário, crie uma nova conta.",
  "lock.quick.waitingWindows": "Aguardando o Windows Hello…",
  "lock.quick.waitingMac": "Aguardando o Touch ID…",
  "lock.quick.touchKey": "Insira a sua chave e toque nela quando piscar.",
  "lock.quick.pin": "PIN da chave de segurança",
  "lock.quick.pinSubmit": "Continuar",
  "lock.quick.cancel": "Cancelar",
} satisfies Record<keyof typeof en, string>;
