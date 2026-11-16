/**
 * Polish UI strings: importGuide. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { importGuide as en } from "../en/importGuide.ts";

export const importGuide = {
  "import.grid.title": "Gdzie są teraz Twoje hasła?",
  "import.grid.browsers": "Przeglądarki",
  "import.grid.managers": "Menedżery haseł",
  "import.grid.other": "Coś innego",
  "import.source.chrome": "Chrome",
  "import.source.edge": "Edge",
  "import.source.brave": "Brave",
  "import.source.opera": "Opera",
  "import.source.vivaldi": "Vivaldi",
  "import.source.arc": "Arc",
  "import.source.firefox": "Firefox",
  "import.source.safari": "Safari",
  "import.source.applePasswords": "Apple Passwords",
  "import.source.onePassword": "1Password",
  "import.source.bitwarden": "Bitwarden",
  "import.source.keepass": "KeePass",
  "import.source.keepassxcCsv": "KeePassXC (CSV)",
  "import.source.enpass": "Enpass",
  "import.source.lastpass": "LastPass",
  "import.source.dashlane": "Dashlane",
  "import.source.keeper": "Keeper",
  "import.source.nordpass": "NordPass",
  "import.source.protonPass": "Proton Pass",
  "import.source.roboform": "RoboForm",
  "import.source.otherFile": "Inny plik CSV lub inny plik",
  "import.guide.chrome.1":
    "Otwórz ten adres w Chrome: skopiuj go i wklej w pasku adresu.",
  "import.guide.chrome.2":
    "Obok „Eksportuj hasła” kliknij „Pobierz plik” i potwierdź hasłem do komputera.",
  "import.guide.chrome.3": "Chrome zapisuje „Chrome Passwords.csv” w folderze Pobrane.",
  "import.guide.edge.1": "Otwórz ten adres w Edge: skopiuj go i wklej w pasku adresu.",
  "import.guide.edge.2":
    "Kliknij „⋯” obok „Zapisane hasła”, potem „Eksportuj hasła” i potwierdź.",
  "import.guide.edge.3": "Edge zapisuje plik CSV w folderze Pobrane.",
  "import.guide.brave.1": "Otwórz ten adres w Brave: skopiuj go i wklej w pasku adresu.",
  "import.guide.brave.2":
    "Obok „Eksportuj hasła” kliknij „Pobierz plik” i potwierdź hasłem do komputera.",
  "import.guide.brave.3": "Brave zapisuje plik CSV w folderze Pobrane.",
  "import.guide.opera.1": "Otwórz ten adres w Operze: skopiuj go i wklej w pasku adresu.",
  "import.guide.opera.2":
    "Obok „Eksportuj hasła” kliknij „Pobierz plik” i potwierdź hasłem do komputera.",
  "import.guide.opera.3": "Opera zapisuje plik CSV w folderze Pobrane.",
  "import.guide.vivaldi.1":
    "Otwórz ten adres w Vivaldi: skopiuj go i wklej w pasku adresu.",
  "import.guide.vivaldi.2":
    "Obok „Eksportuj hasła” kliknij „Pobierz plik” i potwierdź hasłem do komputera.",
  "import.guide.vivaldi.3": "Vivaldi zapisuje plik CSV w folderze Pobrane.",
  "import.guide.arc.1": "Otwórz ten adres w Arc: skopiuj go i wklej w pasku adresu.",
  "import.guide.arc.2":
    "Obok „Eksportuj hasła” kliknij „Pobierz plik” i potwierdź hasłem do komputera.",
  "import.guide.arc.3": "Arc zapisuje plik CSV w folderze Pobrane.",
  "import.guide.firefox.1":
    "Otwórz ten adres w Firefox: skopiuj go i wklej w pasku adresu.",
  "import.guide.firefox.2":
    "Kliknij „⋯” w prawym górnym rogu, potem „Eksportuj hasła…” i potwierdź.",
  "import.guide.firefox.3": "Zapisz „logins.csv” w folderze Pobrane.",
  "import.guide.safari.1":
    "Safari → Ustawienia → Hasła (macOS 14 i starsze; w macOS 15 i nowszych użyj Apple Passwords).",
  "import.guide.safari.2": "Kliknij „⋯” pod listą, potem „Eksportuj wszystkie hasła…”.",
  "import.guide.safari.3": "Zapisz plik CSV w folderze Pobrane.",
  "import.guide.applePasswords.1": "Otwórz aplikację Hasła.",
  "import.guide.applePasswords.2": "Plik → „Eksportuj wszystkie hasła do pliku…”.",
  "import.guide.applePasswords.3": "Zapisz plik CSV w folderze Pobrane.",
  "import.guide.onePassword.1": "Otwórz aplikację 1Password i odblokuj ją.",
  "import.guide.onePassword.2": "Plik → Eksportuj → Twoje konto, a potem wpisz hasło.",
  "import.guide.onePassword.3": "Wybierz „1PUX” i kliknij „Eksportuj dane”.",
  "import.guide.bitwarden.1": "W sejfie w przeglądarce lub w aplikacji: Narzędzia → Eksportuj sejf.",
  "import.guide.bitwarden.2": "Wybierz format pliku „.json” (nie „zaszyfrowany”).",
  "import.guide.bitwarden.3": "Potwierdź hasłem głównym.",
  "import.guide.keepass.1": "Użyj pliku bazy danych (.kdbx) bez zmian.",
  "import.guide.keepass.2":
    "Hasło do niego wpiszesz w następnym kroku. Zostaje na tym urządzeniu.",
  "import.guide.keepassxcCsv.1": "KeePassXC: Baza danych → Eksportuj → Plik CSV…",
  "import.guide.keepassxcCsv.2":
    "Zapisz w folderze Pobrane. Plik .kdbx zachowuje więcej, więc jeśli możesz, wybierz raczej „KeePass”.",
  "import.guide.enpass.1": "Enpass: Menu → Plik → Eksportuj.",
  "import.guide.enpass.2":
    "Wybierz sejf, format „.json” i wpisz hasło główne.",
  "import.guide.enpass.3": "Zapisz w folderze Pobrane.",
  "import.guide.lastpass.1":
    "Zaloguj się na lastpass.com i otwórz Opcje zaawansowane → Eksportuj.",
  "import.guide.lastpass.2": "Wpisz hasło główne. Plik CSV zostanie pobrany.",
  "import.guide.lastpass.3":
    "Jeśli przeglądarka pokaże zamiast tego tekst, zapisz stronę jako „lastpass.csv”.",
  "import.guide.dashlane.1":
    "Otwórz Dashlane w przeglądarce → Moje konto → Ustawienia → Eksportuj dane.",
  "import.guide.dashlane.2": "Wybierz „CSV”. Dashlane pobiera plik ZIP.",
  "import.guide.dashlane.3": "Zostaw plik ZIP bez zmian.",
  "import.guide.keeper.1": "Keeper Web Vault → Ustawienia → Eksportuj.",
  "import.guide.keeper.2":
    "Wybierz „JSON” (zachowuje pola niestandardowe i kody 2FA) i wyeksportuj.",
  "import.guide.nordpass.1": "NordPass → Ustawienia → Eksportuj elementy.",
  "import.guide.nordpass.2":
    "Potwierdź hasłem głównym i zapisz plik CSV w folderze Pobrane.",
  "import.guide.protonPass.1": "Proton Pass → Ustawienia → Eksportuj.",
  "import.guide.protonPass.2":
    "Wybierz „ZIP” i zostaw szyfrowanie wyłączone. Mosaic nie odczyta eksportu zaszyfrowanego PGP.",
  "import.guide.protonPass.3": "Odblokuj i zapisz w folderze Pobrane.",
  "import.guide.roboform.1": "RoboForm → Ustawienia → Konto i dane → Eksportuj.",
  "import.guide.roboform.2": "Wybierz „CSV” i „Logowania”, a potem zapisz w folderze Pobrane.",
  "import.guide.otherFile.1":
    "Wyeksportuj plik CSV z wierszem nagłówka lub jeden z formatów powyżej.",
  "import.guide.otherFile.2": "Wybierz plik. Mosaic dopasuje kolumny.",
  "import.guide.copyUrl": "Kopiuj adres",
  "import.guide.urlCopied": "Skopiowano adres.",
  "import.guide.choose": "Wybierz plik eksportu",
  "import.guide.dropHint": "…lub upuść tu jeden plik. Jest odczytywany tylko na tym urządzeniu.",
  "import.guide.back": "Wstecz",
  "import.detect.detected": "Rozpoznano: {source}",
  "import.detect.mismatch":
    "Ten plik wygląda na eksport z {source}. Mosaic odczyta go jako taki.",
  "import.detected.chromium": "Chrome, Edge lub inna przeglądarka Chromium",
  "import.detected.firefox": "Firefox",
  "import.detected.safari": "Safari lub Apple Passwords",
  "import.detected.bitwardenCsv": "Bitwarden (CSV)",
  "import.detected.bitwarden": "Bitwarden",
  "import.detected.enpass": "Enpass",
  "import.detected.keepass": "KeePass",
  "import.detected.keepassxcCsv": "KeePassXC (CSV)",
  "import.detected.onePassword": "1Password",
  "import.detected.cxf": "plik FIDO Credential Exchange",
  "import.detected.lastpass": "LastPass",
  "import.detected.dashlane": "Dashlane",
  "import.detected.keeper": "Keeper",
  "import.detected.nordpass": "NordPass",
  "import.detected.protonPass": "Proton Pass",
  "import.detected.roboform": "RoboForm",
  "import.detected.csv": "plik CSV",
  "import.lost.title": "Nie zostanie przeniesione",
  "import.lost.passkeys":
    "Klucze dostępu: {source} nie umieszcza ich w eksporcie. Zachowaj {source}, dopóki nie skonfigurujesz nowych kluczy dostępu.",
  "import.lost.attachments":
    "Załączniki: {source} nie umieszcza ich w eksporcie. Dodaj je ręcznie.",
  "import.lost.history": "Stare hasła (historia): ten eksport ich nie zawiera.",
  "import.lost.roboformExtras":
    "Tożsamości, bezpieczne notatki i zakładki: wyeksportuj je osobno. Nie są częścią pliku z logowaniami.",
  "import.lost.entriesSkipped":
    "{count, plural, one {# wpisu nie udało się odczytać i został pominięty.} few {# wpisów nie udało się odczytać i zostały pominięte.} many {# wpisów nie udało się odczytać i zostały pominięte.} other {# wpisu nie udało się odczytać i został pominięty.}}",
  "import.lost.fieldsDropped":
    "Niektóre elementy mają więcej pól, niż zachowuje Mosaic. Nadmiarowe pola są pomijane.",
  "import.lost.notesTruncated":
    "Niektóre notatki są dłuższe, niż może pomieścić jeden element. Część, która się nie zmieściła, jest pomijana.",
  "import.lost.attachmentsSkipped":
    "Niektórych załączników brakuje, są zbyt duże lub nie mieszczą się w miejscu na załączniki. Dodaj je ręcznie.",
  "import.cleanup.title": "Porządkowanie",
  "import.cleanup.delete": "Usuń teraz {name}",
  "import.cleanup.deleted": "{name} został nadpisany i usunięty.",
  "import.cleanup.deleteFailed": "Nie udało się usunąć {name}. Usuń go samodzielnie.",
  "import.cleanup.keepFile":
    "Niektóre elementy nie zostały zaimportowane lub nie zostały przeniesione w całości. Zachowaj {name}, dopóki nie sprawdzisz, czego brakuje, a potem usuń go samodzielnie.",
  "import.cleanup.ssdNote":
    "Na dyskach SSD i w synchronizowanych folderach mogą przetrwać starsze kopie: w historii wersji, kopiach zapasowych lub koszu. Sprawdź także te miejsca.",
  "import.cleanup.synced":
    "Ten plik znajduje się w folderze {service}. {service} może przechowywać wcześniejsze wersje; usuń je także tam.",
  "import.synced.oneDrive": "OneDrive",
  "import.synced.dropbox": "Dropbox",
  "import.synced.googleDrive": "Google Drive",
  "import.synced.icloud": "iCloud",
  "import.synced.box": "Box",
  "import.checklist.title": "Zanim opuścisz starą aplikację",
  "import.checklist.browserSaving":
    "Wyłącz zapisywanie haseł i automatyczne logowanie w {source}, aby nie proponowała zapisu obok Mosaic.",
  "import.checklist.firefoxSaving":
    "Otwórz about:preferences#privacy i odznacz „Pytaj o zapisywanie loginów i haseł”, aby Firefox nie proponował zapisu obok Mosaic.",
  "import.checklist.safariSaving":
    "Safari → Ustawienia → Autouzupełnianie: odznacz „Nazwy użytkowników i hasła”, aby Safari nie wypełniało obok Mosaic.",
  "import.checklist.oldExtension":
    "Wyłącz rozszerzenie przeglądarki {source}, aby wypełniała tylko jedna aplikacja.",
  "import.checklist.deleteOld":
    "Usuń swoje dane w {source}, gdy wszystko sprawdzisz w Mosaic.",
} satisfies Record<keyof typeof en, string>;
