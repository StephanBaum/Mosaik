/**
 * Dutch UI strings: unlock. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { unlock as en } from "../en/unlock.ts";

export const unlock = {
  "lock.unlockTitle": "Mosaic ontgrendelen",
  "lock.masterPassword": "Hoofdwachtwoord",
  "lock.wrongPassword":
    "Dat hoofdwachtwoord werkte niet. Controleer Caps Lock en je toetsenbordindeling en probeer het opnieuw.",
  "lock.locked": "Mosaic is vergrendeld.",
  "session.unlocked": "Mosaic is ontgrendeld.",
  "lock.secretKey": "Geheime Sleutel",
  "lock.secretKeyHint":
    "Dit apparaat heeft je Geheime Sleutel nog niet. Typ hem over van je Noodkit en Mosaic onthoudt hem op dit apparaat.",
  "lock.submit": "Ontgrendelen",
  "lock.working": "Ontgrendelen…",
  "lock.reason.manual": "Je hebt Mosaic vergrendeld.",
  "lock.reason.idle": "Mosaic vergrendelde zichzelf na een tijd zonder activiteit.",
  "lock.reason.sleep": "Mosaic vergrendelde toen de computer in slaapstand ging.",
  "lock.reason.screenLock": "Mosaic vergrendelde samen met je scherm.",
  "lock.or": "of",
  "lock.quick.biometricWindows": "Windows Hello gebruiken",
  "lock.quick.biometricMac": "Touch ID gebruiken",
  "lock.quick.biometricLinux": "Biometrie gebruiken",
  "lock.quick.hardwareKey": "Beveiligingssleutel gebruiken",
  "lock.autoLockInfo":
    "Vergrendelt automatisch na {minutes, plural, one {# minuut} other {# minuten}} inactiviteit.",
  "lock.forgot.title": "Je hoofdwachtwoord vergeten?",
  "lock.forgot.body":
    "Mosaic kan het niet resetten, want alleen jij kent het. Controleer of je het op je Noodkit hebt geschreven. Maak anders een nieuw account aan.",
  "lock.quick.waitingWindows": "Wachten op Windows Hello…",
  "lock.quick.waitingMac": "Wachten op Touch ID…",
  "lock.quick.touchKey": "Steek je sleutel in en raak hem aan wanneer hij knippert.",
  "lock.quick.pin": "Pincode van de beveiligingssleutel",
  "lock.quick.pinSubmit": "Doorgaan",
  "lock.quick.cancel": "Annuleren",
} satisfies Record<keyof typeof en, string>;
