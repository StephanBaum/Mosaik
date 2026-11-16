/**
 * Polish UI strings: settings. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { settings as en } from "../en/settings.ts";

export const settings = {
  "settings.title": "Ustawienia",
  "settings.security": "Bezpieczeństwo",
  "settings.autoLock": "Blokuj po bezczynności",
  "settings.autoLock.never": "Nigdy",
  "settings.autoLock.minutes":
    "{minutes, plural, one {# minuta} few {# minuty} many {# minut} other {# minuty}}",
  "settings.lockOnSleep": "Blokuj, gdy komputer przechodzi w stan uśpienia",
  "settings.lockOnScreenLock": "Blokuj, gdy ekran się blokuje",
  "settings.screenLock.unsupported":
    "Mosaic nie może wykryć blokady ekranu w tym systemie. Nadal blokuje się po bezczynności i po wybudzeniu komputera.",
  "settings.lockTrigger.required":
    "Zostaw włączoną co najmniej jedną blokadę. Mosaic nie może wykryć zablokowanego ekranu w tym systemie.",
  "settings.clipboard": "Czyść schowek po",
  "settings.clipboard.never": "Nigdy",
  "settings.clipboard.seconds":
    "{seconds, plural, one {# sekundzie} few {# sekundach} many {# sekundach} other {# sekundy}}",
  "settings.appearance": "Wygląd",
  "settings.theme": "Motyw",
  "settings.theme.system": "Zgodnie z systemem",
  "settings.theme.light": "Jasny",
  "settings.theme.dark": "Ciemny",
  "settings.language": "Język",
  "settings.language.system": "Zgodnie z systemem",
  "settings.language.machineNote":
    "Przetłumaczono z pomocą AI. Jeśli coś brzmi źle, daj nam znać.",
  "settings.import": "Import",
  "settings.import.body":
    "Przenieś swoje logowania z przeglądarki lub innego menedżera haseł. Mosaic poprowadzi Cię przez eksport i pomoże usunąć plik po wszystkim.",
  "settings.import.open": "Importuj hasła",
  "settings.ssh": "Agent SSH",
  "settings.ssh.enable": "Używaj moich kluczy SSH z Mosaic",
  "settings.ssh.hint":
    "Domyślnie wyłączone. Agent działa tylko wtedy, gdy Mosaic jest odblokowany, i pyta przed każdym użyciem.",
  "settings.ssh.status":
    "{count, plural, =0 {Brak udostępnianych kluczy} one {Udostępniany # klucz} few {Udostępniane # klucze} many {Udostępnianych # kluczy} other {Udostępnianego # klucza}}",
  "settings.ssh.endpoint": "Ustaw SSH_AUTH_SOCK na",
  "settings.ssh.skipped": "Klucze, które nie są udostępniane",
  "settings.kit": "Zestaw awaryjny",
  "settings.kit.save": "Zapisz Zestaw awaryjny ponownie",
  "settings.saved": "Zapisano ustawienia.",
  "reauth.title": "Potwierdź hasłem głównym",
  "reauth.body": "Ta zmiana zmniejsza bezpieczeństwo Mosaic, więc wymaga hasła głównego.",
  "reauth.bodyKit":
    "Twój Klucz tajny zostanie pokazany w pliku, więc Mosaic potrzebuje hasła głównego.",
  "reauth.password": "Hasło główne",
  "reauth.submit": "Potwierdź",
  "settings.intro":
    "Jak Mosaic blokuje się, czyści sekrety i chroni Twoje klucze na tym urządzeniu.",
  "settings.section.autoLock": "Automatyczne blokowanie",
  "settings.section.autoLock.body": "Kiedy Mosaic blokuje się sam, bez Twojej prośby.",
  "settings.section.clipboard": "Schowek",
  "settings.section.clipboard.body": "Skopiowane sekrety są czyszczone po tym czasie.",
  "settings.appearance.body": "Motyw i język Mosaic.",
  "settings.ssh.body":
    "Pozwól programom logować się kluczami SSH z Twojego sejfu. Każde użycie zatwierdzasz.",
  "settings.kit.body":
    "Zapisz nową kopię Zestawu awaryjnego. Wymaga to hasła głównego.",
  "settings.quick": "Szybkie odblokowanie",
  "settings.quick.body": "Szybsze sposoby powrotu, oprócz hasła głównego.",
  "settings.quick.windowsHello": "Windows Hello",
  "settings.quick.touchId": "Touch ID",
  "settings.quick.biometric.body":
    "Odblokuj twarzą, odciskiem palca lub PIN-em. Po ponownym uruchomieniu nadal potrzebne jest hasło główne.",
  "settings.quick.biometric.bodyMac":
    "Odblokuj odciskiem palca. Po ponownym uruchomieniu nadal potrzebne jest hasło główne.",
  "settings.quick.expires": "Mosaic poprosi ponownie o hasło główne {date, date, medium}.",
  "settings.quick.maxAge": "Pytaj o hasło główne co najmniej co",
  "settings.quick.maxAgeDays":
    "{days, plural, one {# dzień} few {# dni} many {# dni} other {# dnia}}",
  "settings.quick.biometricLinux": "Odblokowanie biometryczne nie jest dostępne w systemie Linux.",
  "settings.quick.biometricUnavailable": "Nie skonfigurowano na tym urządzeniu.",
  "settings.quick.keys": "Klucze bezpieczeństwa",
  "settings.quick.keysCount":
    "{count, plural, =0 {Brak zarejestrowanych} one {# zarejestrowany} few {# zarejestrowane} many {# zarejestrowanych} other {# zarejestrowanego}}",
  "settings.quick.keysUnsupported":
    "Klucze bezpieczeństwa nie są jeszcze obsługiwane w tym systemie.",
  "settings.quick.keyRow": "Klucz bezpieczeństwa · {tag} · dodano {addedAt, date, medium}",
  "settings.quick.addKey": "Dodaj klucz bezpieczeństwa",
  "settings.quick.addKeyBody":
    "Włóż klucz, który chcesz dodać, i wpisz jego PIN. Potem dotknij go, gdy zacznie migać.",
  "settings.quick.manageKeys": "Zarządzaj",
  "settings.quick.removeKey": "Usuń",
  "settings.quick.removeKeyNamed": "Usuń klucz bezpieczeństwa {tag}",
  "settings.quick.removeKeyConfirm": "Usunąć klucz bezpieczeństwa {tag}?",
  "settings.quick.keyRisk":
    "Każdy, kto ma ten klucz i jego PIN, może otworzyć Twój sejf bez hasła głównego.",
  "reauth.bodyQuick":
    "To ułatwia dostęp do Mosaic, więc wymaga hasła głównego.",
  "reauth.bodyKey": "Dodanie klucza do tego sejfu wymaga hasła głównego.",
  "settings.passkeyExport": "Eksportuj klucze dostępu",
  "settings.passkeyExport.body":
    "Zapisz klucze dostępu z sejfu w pliku FIDO CXF, aby przenieść je do innej aplikacji.",
  "settings.passkeyExport.vault": "Sejf",
  "settings.passkeyExport.warning":
    "Plik zawiera Twoje klucze dostępu bez szyfrowania. Każdy, kto ma plik, może logować się jako Ty. Usuń go po zaimportowaniu.",
  "settings.passkeyExport.none": "Ten sejf nie ma kluczy dostępu do wyeksportowania.",
  "settings.passkeyExport.button": "Eksportuj klucze dostępu",
  "settings.passkeyExport.reauth":
    "Plik będzie zawierać Twoje klucze dostępu bez szyfrowania, więc eksport wymaga hasła głównego.",
  "settings.passkeyExport.saved":
    "{count, plural, one {Wyeksportowano # klucz dostępu.} few {Wyeksportowano # klucze dostępu.} many {Wyeksportowano # kluczy dostępu.} other {Wyeksportowano # klucza dostępu.}} Usuń plik po zaimportowaniu.",
  "reauth.bodyMove":
    "Przeniesienie elementu z sejfu chronionego kluczem bezpieczeństwa wymaga hasła głównego.",
  "toast.settingsReset":
    "Ustawienia bezpieczeństwa zostały zmienione poza Mosaic i przywrócono bezpieczne wartości domyślne.",
  "settings.shortcut.title": "Skrót szybkiego wyszukiwania",
  "settings.shortcut.body":
    "Naciśnij tę kombinację w dowolnym programie, aby otworzyć wyszukiwanie Mosaic.",
  "settings.shortcut.label": "Skrót",
  "settings.shortcut.hint":
    "Kliknij pole, a potem naciśnij wybrane klawisze: Ctrl, Alt lub ⌘ (na Macu) oraz jeszcze jeden klawisz.",
  "settings.shortcut.save": "Zapisz skrót",
  "settings.shortcut.reset": "Użyj domyślnego",
  "settings.shortcut.saved": "Zapisano skrót: {shortcut}.",
  "settings.shortcut.invalid":
    "Mosaic nie może użyć tego skrótu. Naciśnij Ctrl, Alt lub ⌘ (na Macu) razem z jeszcze jednym klawiszem.",
  "settings.shortcut.unsupported":
    "Twój system nie pozwala aplikacjom ustawiać globalnego skrótu, więc nie można go tu zmienić.",
} satisfies Record<keyof typeof en, string>;
