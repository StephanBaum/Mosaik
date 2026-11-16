/**
 * French UI strings: unlock. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { unlock as en } from "../en/unlock.ts";

export const unlock = {
  "lock.unlockTitle": "Déverrouiller Mosaic",
  "lock.masterPassword": "Mot de passe principal",
  "lock.wrongPassword":
    "Ce mot de passe principal n’a pas fonctionné. Vérifiez la touche Verr. maj et la disposition de votre clavier, puis réessayez.",
  "lock.locked": "Mosaic est verrouillé.",
  "session.unlocked": "Mosaic est déverrouillé.",
  "lock.secretKey": "Clé secrète",
  "lock.secretKeyHint":
    "Cet appareil n’a pas encore votre clé secrète. Saisissez-la depuis votre kit d’urgence, et Mosaic la retiendra sur cet appareil.",
  "lock.submit": "Déverrouiller",
  "lock.working": "Déverrouillage…",
  "lock.reason.manual": "Vous avez verrouillé Mosaic.",
  "lock.reason.idle": "Mosaic s’est verrouillé après un moment d’inactivité.",
  "lock.reason.sleep": "Mosaic s’est verrouillé quand l’ordinateur est passé en veille.",
  "lock.reason.screenLock": "Mosaic s’est verrouillé en même temps que votre écran.",
  "lock.or": "ou",
  "lock.quick.biometricWindows": "Utiliser Windows Hello",
  "lock.quick.biometricMac": "Utiliser Touch ID",
  "lock.quick.biometricLinux": "Utiliser la biométrie",
  "lock.quick.hardwareKey": "Utiliser une clé de sécurité",
  "lock.autoLockInfo":
    "Verrouillage automatique après {minutes, plural, one {# minute} other {# minutes}} d’inactivité.",
  "lock.forgot.title": "Mot de passe principal oublié ?",
  "lock.forgot.body":
    "Mosaic ne peut pas le réinitialiser, car vous seul le connaissez. Vérifiez si vous l’avez noté sur votre kit d’urgence. Sinon, créez un nouveau compte.",
  "lock.quick.waitingWindows": "En attente de Windows Hello…",
  "lock.quick.waitingMac": "En attente de Touch ID…",
  "lock.quick.touchKey": "Insérez votre clé et touchez-la quand elle clignote.",
  "lock.quick.pin": "Code PIN de la clé de sécurité",
  "lock.quick.pinSubmit": "Continuer",
  "lock.quick.cancel": "Annuler",
} satisfies Record<keyof typeof en, string>;
