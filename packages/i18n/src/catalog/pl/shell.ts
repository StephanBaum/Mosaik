/**
 * Polish UI strings: shell. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { shell as en } from "../en/shell.ts";

export const shell = {
  "shell.emergencyKit.title": "Zestaw awaryjny Mosaic",
  "shell.emergencyKit.intro":
    "Przechowuj tę stronę w bezpiecznym miejscu, na przykład w zamykanej szufladzie. Klucz tajny i hasło główne razem odblokowują Twój sejf na nowym urządzeniu. Nikt, nawet Mosaic, nie może odzyskać żadnego z nich za Ciebie.",
  "shell.emergencyKit.email": "E-mail konta",
  "shell.emergencyKit.accountId": "ID konta",
  "shell.emergencyKit.signInUrl": "Adres logowania",
  "shell.emergencyKit.signInUrlLocal": "Tylko to urządzenie (serwer synchronizacji nie jest jeszcze skonfigurowany)",
  "shell.emergencyKit.secretKey": "Klucz tajny",
  "shell.emergencyKit.masterPassword": "Hasło główne",
  "shell.emergencyKit.masterPasswordHint":
    "Wpisz je ręcznie w linii tylko wtedy, gdy ta strona pozostaje w bezpiecznym miejscu. W przeciwnym razie zapamiętaj je.",
  "shell.emergencyKit.footer":
    "Mosaic to oprogramowanie przed audytem: nie zostało jeszcze sprawdzone przez zewnętrznego audytora bezpieczeństwa.",
  "shell.shortcut.conflict":
    "Inna aplikacja już używa {shortcut}. Zamknij tę aplikację lub zmień jej skrót, a potem uruchom Mosaic ponownie, aby użyć szybkiego wyszukiwania.",
  "keys.ctrl": "Ctrl",
  "keys.shift": "Shift",
  "keys.alt": "Alt",
  "keys.space": "Spacja",
  "keys.esc": "Esc",
  "keys.enter": "Enter",
} satisfies Record<keyof typeof en, string>;
