/**
 * Polish UI strings: unlock. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { unlock as en } from "../en/unlock.ts";

export const unlock = {
  "lock.unlockTitle": "Odblokuj Mosaic",
  "lock.masterPassword": "Hasło główne",
  "lock.wrongPassword":
    "Hasło główne nie zadziałało. Sprawdź Caps Lock i układ klawiatury, a potem spróbuj ponownie.",
  "lock.locked": "Mosaic jest zablokowany.",
  "session.unlocked": "Mosaic jest odblokowany.",
  "lock.secretKey": "Klucz tajny",
  "lock.secretKeyHint":
    "To urządzenie nie ma jeszcze Twojego Klucza tajnego. Wpisz go z Zestawu awaryjnego, a Mosaic zapamięta go na tym urządzeniu.",
  "lock.submit": "Odblokuj",
  "lock.working": "Odblokowywanie…",
  "lock.reason.manual": "Mosaic został zablokowany.",
  "lock.reason.idle": "Mosaic zablokował się sam po pewnym czasie bez aktywności.",
  "lock.reason.sleep": "Mosaic zablokował się, gdy komputer przeszedł w stan uśpienia.",
  "lock.reason.screenLock": "Mosaic zablokował się razem z ekranem.",
  "lock.or": "lub",
  "lock.quick.biometricWindows": "Użyj Windows Hello",
  "lock.quick.biometricMac": "Użyj Touch ID",
  "lock.quick.biometricLinux": "Użyj biometrii",
  "lock.quick.hardwareKey": "Użyj klucza bezpieczeństwa",
  "lock.autoLockInfo":
    "Blokuje się automatycznie po {minutes, plural, one {# minucie} few {# minutach} many {# minutach} other {# minuty}} bezczynności.",
  "lock.forgot.title": "Nie pamiętasz hasła głównego?",
  "lock.forgot.body":
    "Mosaic nie może go zresetować, ponieważ zna je tylko Twoja osoba. Sprawdź, czy zapisano je w Zestawie awaryjnym. W przeciwnym razie załóż nowe konto.",
  "lock.quick.waitingWindows": "Czekanie na Windows Hello…",
  "lock.quick.waitingMac": "Czekanie na Touch ID…",
  "lock.quick.touchKey": "Włóż klucz i dotknij go, gdy zacznie migać.",
  "lock.quick.pin": "PIN klucza bezpieczeństwa",
  "lock.quick.pinSubmit": "Dalej",
  "lock.quick.cancel": "Anuluj",
} satisfies Record<keyof typeof en, string>;
