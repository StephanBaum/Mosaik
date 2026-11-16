/**
 * Polish UI strings: browser. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { browser as en } from "../en/browser.ts";

export const browser = {
  "browser.pair.title": "Przeglądarka chce połączyć się z Mosaic",
  "browser.pair.description":
    "Zezwól tylko wtedy, gdy właśnie kliknięto Połącz w rozszerzeniu Mosaic i pokazuje ono ten kod.",
  "browser.pair.code": "Kod",
  "browser.pair.browser": "Przeglądarka",
  "browser.pair.label": "Nazwa tej przeglądarki",
  "browser.pair.labelHint": "Ta nazwa będzie widoczna w Ustawieniach.",
  "browser.pair.deny": "Odmów",
  "browser.pair.allow": "Zezwól",
  "browser.expires":
    "Zamknie się automatycznie za {seconds, plural, one {# sekundę} few {# sekundy} many {# sekund} other {# sekundy}}",
  "browser.escDenies": "aby odmówić",
  "browser.hint.chrome": "Chrome",
  "browser.hint.edge": "Edge",
  "browser.hint.brave": "Brave",
  "browser.hint.opera": "Opera",
  "browser.hint.vivaldi": "Vivaldi",
  "browser.hint.chromium": "Chromium",
  "browser.hint.firefox": "Firefox",
  "browser.hint.other": "Przeglądarka",
  "browser.family.chromium": "Przeglądarka oparta na Chromium",
  "browser.family.firefox": "Firefox",
  "browser.escCancels": "aby anulować",
  "browser.save.titleNew": "Zapisać to logowanie w Mosaic?",
  "browser.save.titleUpdate": "Zaktualizować to logowanie w Mosaic?",
  "browser.save.description":
    "{browser} proponuje zapisanie logowania. Mosaic zapisze je tylko po wybraniu Zapisz.",
  "browser.save.descriptionUpdate":
    "{browser} proponuje nowe hasło do tego logowania. Mosaic zmieni je tylko po wybraniu Zaktualizuj hasło.",
  "browser.save.site": "Witryna",
  "browser.save.username": "Nazwa użytkownika",
  "browser.save.noUsername": "Brak",
  "browser.save.password": "Hasło",
  "browser.save.passwordHidden": "Ukryte",
  "browser.save.target": "Zapisz jako",
  "browser.save.updateOption": "Zaktualizuj „{title}”",
  "browser.save.newOption": "Nowe logowanie",
  "browser.save.itemTitle": "Tytuł",
  "browser.save.vault": "Sejf",
  "browser.save.allowHttp": "Zezwól na wypełnianie na tej niezaszyfrowanej stronie",
  "browser.save.allowHttpHint":
    "Witryna nie używa HTTPS. Każdy w tej sieci mógłby odczytać wpisywane dane.",
  "browser.save.cancel": "Anuluj",
  "browser.save.saveNew": "Zapisz",
  "browser.save.update": "Zaktualizuj hasło",
  "browser.passkey.secure": "Bezpieczne połączenie (https)",
  "browser.passkey.getTitle": "Zaloguj się do {rpId} kluczem dostępu",
  "browser.passkey.createTitle": "Zapisz nowy klucz dostępu",
  "browser.passkey.excludedTitle": "Klucz dostępu jest już zapisany",
  "browser.passkey.createBody": "Utworzyć w Mosaic klucz dostępu dla {rpId}?",
  "browser.passkey.excludedBody": "Masz już w Mosaic klucz dostępu dla {rpId}.",
  "browser.passkey.uvNote":
    "Witryna zażądała weryfikacji użytkownika. Wpisz hasło główne, aby potwierdzić, że to Ty.",
  "browser.passkey.password": "Hasło główne",
  "browser.passkey.account": "Konto",
  "browser.passkey.noName": "Bez nazwy",
  "browser.passkey.vault": "Sejf",
  "browser.passkey.signIn": "Zaloguj się kluczem dostępu",
  "browser.passkey.save": "Zapisz klucz dostępu",
  "browser.passkey.anotherDevice": "Użyj innego urządzenia lub klucza bezpieczeństwa",
  "browser.passkey.cancel": "Anuluj",
  "browser.passkey.ok": "Rozumiem",
  "browser.passkey.escCloses": "aby zamknąć",
  "browser.passkey.passwordNeeded": "Wpisz hasło główne, aby potwierdzić, że to Ty.",
  "browser.settings.title": "Przeglądarki",
  "browser.settings.body":
    "Rozszerzenia przeglądarek, które wypełniają logowania z tej aplikacji. Każde połączenie zatwierdzasz tutaj.",
  "browser.settings.empty":
    "Żadna przeglądarka nie jest jeszcze połączona. Zainstaluj rozszerzenie Mosaic, a potem wybierz w nim Połącz.",
  "browser.settings.connected": "Połączona teraz",
  "browser.settings.lastUsed": "Ostatnio użyta {when}",
  "browser.settings.neverUsed": "Jeszcze nie użyta",
  "browser.settings.disconnect": "Rozłącz",
  "browser.settings.disconnectNamed": "Rozłącz {name}",
  "browser.settings.disconnectTitle": "Rozłączyć „{name}”?",
  "browser.settings.disconnectBody":
    "Ta przeglądarka nie będzie mogła wypełniać logowań, dopóki nie połączysz jej ponownie z poziomu rozszerzenia.",
  "browser.settings.endpointInUse":
    "Inny program lub inne okno Mosaic już używa połączenia dla przeglądarek. Zamknij je, a potem uruchom Mosaic ponownie.",
  "browser.settings.endpointUnavailable":
    "Mosaic nie mógł otworzyć połączenia dla przeglądarek. Uruchom Mosaic ponownie. Jeśli problem się powtarza, sprawdź, czy Twój folder użytkownika jest zapisywalny.",
  "browser.settings.registryUnreadable":
    "Nie można odczytać zapisanych połączeń z przeglądarkami. Mosaic nie zmieni ich, dopóki to nie zostanie naprawione.",
  "browser.connector.title": "Połączenie z przeglądarką wskazuje gdzie indziej",
  "browser.connector.body":
    "Połączenie dla {browser} uruchamia inny program zamiast Mosaic: {path}. Może to być starsza wersja Mosaic lub coś innego, co samo się zarejestrowało. Naprawa ponownie wskaże to Mosaic.",
  "browser.connector.item": "{browser}: {path}",
  "browser.connector.repair": "Napraw",
  "browser.connector.repaired": "Połączenie z przeglądarką naprawiono.",
} satisfies Record<keyof typeof en, string>;
