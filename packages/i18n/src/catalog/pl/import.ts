/**
 * Polish UI strings: import. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { importMessages as en } from "../en/import.ts";

export const importMessages = {
  "import.format.enpass": "Enpass (.json)",
  "import.format.bitwarden": "Bitwarden (.json, niezaszyfrowany)",
  "import.format.keepass": "KeePass (.kdbx)",
  "import.format.onePassword": "1Password (.1pux)",
  "import.format.cxf": "FIDO Credential Exchange (.json): klucze dostępu i hasła",
  "import.format.dashlane": "Dashlane (.zip)",
  "import.format.keeper": "Keeper (.json)",
  "import.format.keeperCsv": "Keeper (.csv)",
  "import.format.protonPass": "Proton Pass (.zip lub .json, niezaszyfrowany)",
  "import.format.csv": "Plik CSV (.csv)",
  "import.keepassPassword": "Hasło bazy danych KeePass",
  "import.preview.ready":
    "{count, plural, one {# element jest gotowy do importu.} few {# elementy są gotowe do importu.} many {# elementów jest gotowych do importu.} other {# elementu jest gotowych do importu.}}",
  "import.preview.attachments":
    "{count, plural, one {# załącznik} few {# załączniki} many {# załączników} other {# załącznika}}",
  "import.preview.duplicate":
    "Masz już podobny element. Jest odznaczony, więc nie zostanie zaimportowany dwa razy.",
  "import.preview.duplicateInFile":
    "Ten plik zawiera ten sam element dwa razy. Kopia jest odznaczona.",
  "import.warning.unmappedFields":
    "Niektóre pola nie mają odpowiednika w Mosaic. Zachowano je jako pola niestandardowe.",
  "import.warning.attachmentsSkipped":
    "Niektóre załączniki są większe niż 50 MiB, nie mieszczą się w Twoim 1 GiB miejsca na załączniki lub brakuje ich w eksporcie. Dodaj je ręcznie po imporcie.",
  "import.warning.entriesSkipped":
    "Niektórych wpisów nie udało się odczytać i zostaną pominięte. Sprawdź je w swoim starym menedżerze haseł.",
  "import.warning.otpUnreadable":
    "Niektóre ustawienia kodów jednorazowych nie zostały rozpoznane. Zachowano je jako ukryte pola, więc skonfiguruj kod ponownie na ich podstawie.",
  "import.warning.otpWeak":
    "Niektóre kody jednorazowe mają sekret krótszy niż 80 bitów, więc łatwiej je odgadnąć. Mimo to zostały zaimportowane; gdy to możliwe, skonfiguruj je ponownie z nowym sekretem.",
  "import.warning.passkeysNotImported":
    "Ten format eksportu nie zawiera kluczy prywatnych kluczy dostępu, więc nie można ich zaimportować. Reszta tych elementów zostaje zaimportowana.",
  "import.warning.passkeyUnreadable":
    "Niektórych kluczy dostępu nie udało się odczytać i zostały pominięte.",
  "import.warning.passkeyUnsupported":
    "Niektóre klucze dostępu używają typu klucza, którego Mosaic nie obsługuje (działa tylko ES256), i zostały pominięte.",
  "import.warning.passkeysSkipped":
    "Niektóre klucze dostępu pominięto, ponieważ jest ich zbyt wiele (najwyżej 100 na element i 10 000 na plik).",
  "import.warning.passkeysKeepOldApp":
    "Zachowaj starą aplikację, dopóki nie przeniesiesz kluczy dostępu.",
  "import.entry.unmappedFields":
    "Niektóre pola nie mają odpowiednika w Mosaic. Zachowano je jako pola niestandardowe.",
  "import.entry.otpUnreadable":
    "Ustawienie kodu jednorazowego nie zostało rozpoznane. Zachowano je jako ukryte pole, więc skonfiguruj kod ponownie na jego podstawie.",
  "import.entry.otpWeak":
    "Sekret kodu jednorazowego jest krótszy niż 80 bitów, więc łatwiej go odgadnąć. Mimo to został zaimportowany; gdy to możliwe, skonfiguruj go ponownie z nowym sekretem.",
  "import.entry.attachmentTooLarge":
    "Załącznik jest większy niż 50 MiB i został pominięty. Dodaj go ręcznie po imporcie.",
  "import.entry.attachmentMissing":
    "Załącznika brakuje w eksporcie. Dodaj go ręcznie po imporcie.",
  "import.entry.attachmentQuota":
    "Załącznik nie mieści się w Twoim 1 GiB miejsca na załączniki i został pominięty.",
  "import.entry.passkeysNotImported":
    "Ten format eksportu nie zawiera klucza prywatnego klucza dostępu tego elementu, więc nie można go zaimportować. Zachowaj starą aplikację, dopóki go nie przeniesiesz.",
  "import.entry.passkeyUnreadable":
    "Klucza dostępu tego elementu nie udało się odczytać i został pominięty. Zachowaj starą aplikację, dopóki go nie przeniesiesz.",
  "import.entry.passkeyUnsupported":
    "Klucz dostępu tego elementu używa typu klucza, którego Mosaic nie obsługuje (działa tylko ES256), i został pominięty. Zachowaj starą aplikację, dopóki go nie przeniesiesz.",
  "import.entry.historyTruncated": "Zachowano tylko 32 najnowsze stare hasła.",
  "import.entry.tagsDropped": "Puste lub zbyt długie tagi zostały pominięte.",
  "import.entry.archived": "Zarchiwizowane w starej aplikacji. Otrzymuje tag „archived”.",
  "import.entry.unknownCategory": "Nieznany typ elementu. Zostaje zaimportowany jako bezpieczna notatka.",
  "import.entry.titleCleaned":
    "Tytuł zawierał niewidoczne znaki lub znaki odwracające tekst. Zostały usunięte.",
  "import.entry.passkeysLimit":
    "Ten element ma więcej kluczy dostępu, niż przyjmuje jeden import (100 na element, 10 000 na plik). Nadmiarowe klucze dostępu zostały pominięte. Zachowaj starą aplikację, dopóki ich nie przeniesiesz.",
  "import.entry.fieldsDropped":
    "Ten element ma więcej pól, niż zachowuje Mosaic. Nadmiarowe pola są pomijane.",
  "import.entry.notesTruncated":
    "Notatki tego elementu są dłuższe, niż może pomieścić jeden element. Część, która się nie zmieściła, została pominięta.",
  "import.entry.idnHost":
    "Ten adres używa znaków specjalnych w nazwie domeny i może wyglądać jak inna witryna. Sprawdź go, zanim na nim polegasz.",
  "import.entry.urlKeptAsText":
    "Tego adresu internetowego nie można było użyć jako linku. Zachowano go jako tekst.",
  "import.skip.malformed": "Ten wpis jest uszkodzony i nie można go odczytać.",
  "import.skip.empty": "Ten wpis jest pusty.",
  "import.skip.tooLarge": "Ten wpis jest większy niż 1 MiB, czyli maksimum dla jednego elementu.",
  "import.result":
    "{imported, plural, one {Zaimportowano # element.} few {Zaimportowano # elementy.} many {Zaimportowano # elementów.} other {Zaimportowano # elementu.}} {skipped, plural, =0 {} one {Pominięto #.} few {Pominięto #.} many {Pominięto #.} other {Pominięto #.}}",
  "import.flow.title": "Import",
  "import.flow.formatStep": "Skąd importujesz?",
  "import.flow.chooseFile": "Wybierz plik eksportu",
  "import.flow.fileChosen": "Plik: {name}",
  "import.flow.password": "Hasło bazy danych KeePass",
  "import.flow.preview": "Podgląd",
  "import.flow.previewBusy": "Odczytywanie pliku. To może chwilę potrwać.",
  "import.flow.selectAll": "Zaznacz wszystko",
  "import.flow.target": "Importuj do",
  "import.flow.commit":
    "Importuj {count, plural, one {# element} few {# elementy} many {# elementów} other {# elementu}}",
  "import.flow.entries": "Wpisy",
  "import.flow.skipped": "Pominięte",
  "import.flow.attachments":
    "{count, plural, one {# załącznik} few {# załączniki} many {# załączników} other {# załącznika}}",
  "import.flow.cancel": "Anuluj import",
  "import.flow.noneSelected": "Zaznacz co najmniej jeden element do importu.",
  "import.flow.resultFailed":
    "{failed, plural, =0 {} one {Nie udało się zapisać # elementu.} few {Nie udało się zapisać # elementów.} many {Nie udało się zapisać # elementów.} other {Nie udało się zapisać # elementu.}}",
  "import.flow.deleteReminder":
    "Usuń teraz plik eksportu. Zawiera Twoje hasła zapisane zwykłym tekstem.",
  "import.flow.dropHint":
    "Wybierz plik eksportu. Wszystko jest odczytywane i szyfrowane na tym urządzeniu.",
} satisfies Record<keyof typeof en, string>;
