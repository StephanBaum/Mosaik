/**
 * Italian UI strings: unlock. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { unlock as en } from "../en/unlock.ts";

export const unlock = {
  "lock.unlockTitle": "Sblocca Mosaic",
  "lock.masterPassword": "Password principale",
  "lock.wrongPassword":
    "Quella password principale non ha funzionato. Controlla il Blocco maiuscole e il layout della tastiera, poi riprova.",
  "lock.locked": "Mosaic è bloccato.",
  "session.unlocked": "Mosaic è sbloccato.",
  "lock.secretKey": "Chiave segreta",
  "lock.secretKeyHint":
    "Questo dispositivo non ha ancora la tua Chiave segreta. Digitala dal tuo Kit di emergenza e Mosaic la ricorderà su questo dispositivo.",
  "lock.submit": "Sblocca",
  "lock.working": "Sblocco in corso…",
  "lock.reason.manual": "Hai bloccato Mosaic.",
  "lock.reason.idle": "Mosaic si è bloccato da solo dopo un po' di inattività.",
  "lock.reason.sleep": "Mosaic si è bloccato quando il computer è andato in stop.",
  "lock.reason.screenLock": "Mosaic si è bloccato insieme allo schermo.",
  "lock.or": "oppure",
  "lock.quick.biometricWindows": "Usa Windows Hello",
  "lock.quick.biometricMac": "Usa Touch ID",
  "lock.quick.biometricLinux": "Usa la biometria",
  "lock.quick.hardwareKey": "Usa la chiave di sicurezza",
  "lock.autoLockInfo":
    "Si blocca automaticamente dopo {minutes, plural, one {# minuto} other {# minuti}} di inattività.",
  "lock.forgot.title": "Hai dimenticato la password principale?",
  "lock.forgot.body":
    "Mosaic non può reimpostarla, perché la conosci solo tu. Controlla se l'hai scritta sul tuo Kit di emergenza. Altrimenti, crea un nuovo account.",
  "lock.quick.waitingWindows": "In attesa di Windows Hello…",
  "lock.quick.waitingMac": "In attesa di Touch ID…",
  "lock.quick.touchKey": "Inserisci la tua chiave e toccala quando lampeggia.",
  "lock.quick.pin": "PIN della chiave di sicurezza",
  "lock.quick.pinSubmit": "Continua",
  "lock.quick.cancel": "Annulla",
} satisfies Record<keyof typeof en, string>;
