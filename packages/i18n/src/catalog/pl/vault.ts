/**
 * Polish UI strings: vault. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { vault as en } from "../en/vault.ts";

export const vault = {
  "vault.itemCount":
    "{count, plural, =0 {Brak elementów} one {# element} few {# elementy} many {# elementów} other {# elementu}}",
  "vault.deleteConfirm": "Usunąć „{title}”? Możesz go przywrócić z kosza w ciągu 30 dni.",
  "vault.defaultName": "Osobisty",
  "vault.unnamed": "Sejf bez nazwy",
  "vault.vaults": "Sejfy",
  "vault.allItems": "Wszystkie elementy",
  "vault.items": "Elementy",
  "vault.search": "Przeszukaj sejf",
  "vault.untitled": "Bez tytułu",
  "vault.noResults":
    "Nic nie pasuje do „{query}”. Sprawdź pisownię lub wyszukaj zamiast tego witrynę.",
  "vault.new": "Nowy element",
  "vault.selectItem": "Wybierz element, aby zobaczyć jego szczegóły.",
  "vault.trashEmpty": "Kosz jest pusty. Usunięte elementy zostają tu przez 30 dni.",
  "vault.favorite": "Ulubione",
  "vault.unfavorite": "Usuń z ulubionych",
  "vault.moveTo": "Przenieś do sejfu",
  "vault.allVaults": "Wszystkie sejfy",
  "vault.category.favorites": "Ulubione",
  "vault.category.logins": "Logowania",
  "vault.category.passkeys": "Klucze dostępu",
  "vault.category.codes": "Kody jednorazowe",
  "vault.category.cards": "Karty",
  "vault.category.notes": "Bezpieczne notatki",
  "vault.category.ssh": "Klucze SSH",
  "vault.flag.otp": "2FA",
  "vault.moveToNamed": "Przenieś do: {name}",
  "vault.emptyTitle": "Twój sejf jest pusty",
  "vault.emptyBody":
    "Dodaj logowanie, klucz dostępu, kartę lub notatkę. Wszystko jest szyfrowane na tym urządzeniu, zanim zostanie zsynchronizowane.",
  "vault.addFirst": "Dodaj pierwszy element",
  "vault.paletteTip.before": "Naciśnij",
  "vault.paletteTip.after": "w dowolnej chwili, aby wyszukać lub uruchomić polecenie.",
  "trash.retention": "Usunięte elementy są przechowywane przez 30 dni.",
  "trash.rowMeta":
    "Usunięto {deletedAt, date, medium} · pozostało {days, plural, one {# dzień} few {# dni} many {# dni} other {# dnia}}",
  "trash.detailMeta": "{kind} · {vault} · Usunięto {deletedAt, date, medium}",
  "trash.purgeConfirm.title": "Usunąć „{title}” na zawsze?",
  "trash.purgeConfirm.body": "Nie można tego cofnąć.",
  "vault.hardware.placeholderName": "Kody (klucz bezpieczeństwa)",
  "vault.hardware.title": "Ten sejf wymaga klucza bezpieczeństwa",
  "vault.hardware.body":
    "Kody tutaj pozostają bezpieczne, nawet jeśli ktoś pozna Twoje hasło główne i Klucz tajny, ale nie zobaczysz ich bez klucza bezpieczeństwa.",
  "vault.hardware.open": "Użyj klucza bezpieczeństwa",
  "vault.hardware.lockedNote": "Zablokowany. Pozostałe sejfy zostają odblokowane.",
  "vault.hardware.lockNow": "Zablokuj ten sejf",
  "vault.hardware.closesIn": "Zablokuje się za {time}",
  "vault.hardware.backupTitle": "Zarejestruj zapasowy klucz",
  "vault.hardware.backupBody":
    "Zarejestrowano 1 klucz. Zalecamy dwa: jeśli utracisz jedyny klucz, nie otworzysz już tego sejfu.",
  "vault.hardware.createOption": "Chroń kluczem bezpieczeństwa (tylko kody)",
  "vault.hardware.tradeoff":
    "Kody w tym sejfie za każdym razem wymagają klucza bezpieczeństwa, więc samo skradzione hasło główne nie wystarczy, ale utrata wszystkich zarejestrowanych kluczy oznacza utratę tych kodów.",
  "vault.hardware.removeNote":
    "Usunięcie klucza nie zmienia klucza tego sejfu. Jeśli klucz został skradziony, przenieś kody do nowego sejfu.",
  "vault.hardware.moveNote":
    "Przeniesienie do tego sejfu kodu, którego już używasz, nie chroni wcześniejszych kopii: starego elementu, jego historii, kopii zapasowych i eksportów. Jeśli możesz, wygeneruj nowy sekret u dostawcy i dodaj go zamiast tego.",
  "vault.hardware.pinBody":
    "Wpisz PIN klucza, jeśli go ma, a potem dotknij klucza, gdy zacznie migać.",
  "vault.hardware.keys": "Klucze",
  "vault.hardware.keysLabel": "Klucze tego sejfu",
  "vault.hardware.eyebrow": "Kody chronione sprzętowo",
  "vault.hardware.selfLocked":
    "Ten sejf zablokował się sam. Użyj klucza bezpieczeństwa, aby otworzyć go ponownie.",
  "vault.hardware.closingSoon":
    "Ten sejf zablokuje się za mniej niż 30 sekund. Potem użyj klucza bezpieczeństwa, aby otworzyć go ponownie.",
  "vault.hardware.keysTitle": "Klucze tego sejfu",
  "vault.hardware.codesOnly": "Tylko kody jednorazowe",
  "vault.create": "Nowy sejf",
  "vault.create.name": "Nazwa sejfu",
  "vault.create.submit": "Utwórz sejf",
  "vault.menu": "Działania sejfu",
  "vaultKey.menu": "Wymień klucz sejfu…",
  "vaultKey.title": "Wymień klucz sejfu {vault}",
  "vaultKey.body":
    "Mosaic tworzy nowy klucz dla tego sejfu i przekazuje go każdemu urządzeniu i każdej osobie z dostępem. Kopie, które już mają, pozostają dla nich czytelne; nowy klucz chroni to, co zmieni się od teraz. Użyj tego, gdy ktoś utracił dostęp lub urządzenie zostało zgubione.",
  "vaultKey.attachments": "Wymień też klucze załączników",
  "vaultKey.attachments.hint":
    "Nadaje nowy klucz także każdemu plikowi w tym sejfie. Przy wielu lub dużych plikach trwa to dłużej.",
  "vaultKey.reauth": "Wymiana klucza sejfu wymaga hasła głównego.",
  "vaultKey.submit": "Wymień klucz",
  "vaultKey.working": "Wymiana klucza…",
  "vaultKey.progress": "Wymiana kluczy załączników: {done} z {total}.",
  "vaultKey.done": "Klucz sejfu został wymieniony.",
  "vaultKey.doneAttachments": "Klucz sejfu został wymieniony, klucze załączników również.",
  "vaultKey.unfinished":
    "Klucz sejfu został wymieniony. Wymiana kluczy załączników będzie kontynuowana później. Jeśli Mosaic zamknie się wcześniej, następnym razem wybierz Kontynuuj wymianę kluczy.",
  "vaultKey.unreachable":
    "{count, plural, one {# załącznika nie ma jeszcze na tym urządzeniu i zachowuje stary klucz.} few {# załączników nie ma jeszcze na tym urządzeniu i zachowują stary klucz.} many {# załączników nie ma jeszcze na tym urządzeniu i zachowują stary klucz.} other {# załącznika nie ma jeszcze na tym urządzeniu i zachowują stary klucz.}}",
  "vaultKey.close": "Zamknij",
  "vault.meta.title": "Szczegóły sejfu zostały skrócone",
  "vault.meta.body":
    "Dwa urządzenia zmieniły {fields} sejfu „{vault}” w tym samym czasie, a razem były zbyt długie, aby je zapisać. Mosaic zachował jedną wersję lub zostawił pole puste. Sprawdź je i w razie potrzeby wpisz ponownie.",
  "vault.edit.title": "Edytuj sejf",
  "vault.meta.edit": "Edytuj sejf",
  "vault.meta.field.name": "nazwę",
  "vault.meta.field.icon": "ikonę",
  "vault.meta.field.description": "opis",
  "vault.meta.dismiss": "Odrzuć",
} satisfies Record<keyof typeof en, string>;
