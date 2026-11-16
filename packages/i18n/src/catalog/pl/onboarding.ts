/**
 * Polish UI strings: onboarding. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { onboarding as en } from "../en/onboarding.ts";

export const onboarding = {
  "onboarding.secretKey.title": "Twój Klucz tajny",
  "onboarding.secretKey.body":
    "Klucz tajny i hasło główne razem odblokowują Twój sejf. Nie możemy odzyskać żadnego z nich.",
  "onboarding.secretKey.verify":
    "Wpisz ostatnią grupę Klucza tajnego, aby potwierdzić jego zapisanie.",
  "onboarding.secretKey.mistyped":
    "To się nie zgadza. Sprawdź ostatnią grupę w Zestawie awaryjnym.",
  "onboarding.step": "Krok {current} z {total}",
  "onboarding.create.title": "Utwórz konto",
  "onboarding.create.body":
    "Hasło główne i Klucz tajny wygenerowany na tym urządzeniu razem odblokowują Twój sejf. Żadne z nich nigdy nie jest wysyłane na serwer.",
  "onboarding.create.email": "E-mail",
  "onboarding.create.password": "Hasło główne",
  "onboarding.create.passwordHint":
    "Dobrze sprawdzają się cztery niepowiązane słowa. Używaj go tylko dla Mosaic.",
  "onboarding.create.confirm": "Potwierdź hasło główne",
  "onboarding.create.mismatch":
    "Oba hasła się nie zgadzają. Wpisz to samo hasło główne w obu polach.",
  "onboarding.create.submit": "Utwórz konto",
  "onboarding.create.working": "Tworzenie konta. To potrwa kilka sekund.",
  "onboarding.kit.accountId": "ID konta",
  "onboarding.kit.secretKey": "Klucz tajny",
  "onboarding.kit.lastGroup": "Ostatnia grupa Klucza tajnego",
  "onboarding.kit.confirm": "Potwierdź",
  "kit.title": "Zapisz Zestaw awaryjny",
  "kit.body":
    "Zestaw to plik PDF z Twoim Kluczem tajnym. Razem z hasłem głównym otwiera sejf na nowym urządzeniu. Przechowuj go w bezpiecznym miejscu, z dala od tego komputera.",
  "kit.secretKey": "Twój Klucz tajny",
  "kit.save": "Zapisz Zestaw awaryjny (PDF)",
  "kit.saved": "Zapisano. Przechowuj plik w bezpiecznym miejscu.",
  "kit.cancelled": "Jeszcze nie zapisano. Zapisz zestaw, aby kontynuować.",
  "kit.recover.alreadySaved": "Już zapisano",
  "kit.recover.title": "Zapisz Zestaw awaryjny ponownie",
  "kit.recover.body":
    "Zestaw awaryjny nie został jeszcze potwierdzony. Wpisz hasło główne, aby zapisać go ponownie, a potem potwierdź ostatnią grupę Klucza tajnego.",
  "onboarding.import.title": "Przenieś swoje hasła",
  "onboarding.import.body":
    "Z Chrome, Edge, Firefox, Safari i innych przeglądarek albo z 1Password, Bitwarden, LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm, KeePass i Enpass. Zajmie to kilka minut. Możesz to też zrobić później w Ustawieniach.",
  "onboarding.import.start": "Importuj z innej aplikacji",
  "onboarding.import.skip": "Pomiń na razie",
  "onboarding.done.title": "Wszystko gotowe",
  "onboarding.done.body": "Twój sejf jest gotowy. Zablokujesz go w każdej chwili przyciskiem Zablokuj.",
  "onboarding.browser.title": "Zainstaluj rozszerzenie przeglądarki",
  "onboarding.browser.body":
    "Wypełniaj logowania w przeglądarce. Rozszerzenie wypełnia tylko po kliknięciu lub użyciu jego skrótu klawiszowego.",
  "onboarding.browser.steps": "Jak połączyć przeglądarkę",
  "onboarding.browser.step1": "Zainstaluj rozszerzenie Mosaic w przeglądarce.",
  "onboarding.browser.step2": "Otwórz rozszerzenie i wybierz Połącz.",
  "onboarding.browser.step3":
    "Sprawdź, czy kod się zgadza, a potem wybierz Zezwól w oknie, które otworzy się tutaj.",
  "onboarding.browser.notInStores":
    "Rozszerzenia nie ma jeszcze w sklepach przeglądarek. Możesz połączyć je później w Ustawienia → Przeglądarki.",
  "onboarding.browser.getFor": "Pobierz dla: {browser}",
  "onboarding.browser.connectedList": "Połączone przeglądarki",
  "onboarding.browser.waiting": "Czekanie na rozszerzenie. Żadna przeglądarka nie jest jeszcze połączona.",
  "onboarding.browser.connected": "Połączono: {name}",
  "onboarding.browser.done": "Gotowe",
  "onboarding.browser.skip": "Pomiń na razie",
  "onboarding.progress": "Postęp konfiguracji",
  "onboarding.stepName.create": "Utwórz konto",
  "onboarding.stepName.kit": "Zestaw awaryjny",
  "onboarding.stepName.verify": "Potwierdź zestaw",
  "onboarding.stepName.import": "Import (opcjonalnie)",
  "onboarding.stepName.browser": "Rozszerzenie przeglądarki (opcjonalnie)",
  "kit.preview": "Podgląd Zestawu awaryjnego",
  "kit.continueHint": "Zapisz zestaw, aby kontynuować.",
  "kit.warning":
    "Bez tego zestawu i hasła głównego nikt nie może odzyskać Twojego sejfu, nawet my.",
  "onboarding.join.link": "Już używam Mosaic",
  "onboarding.join.title": "Dodaj to urządzenie do swojego konta",
  "onboarding.join.body":
    "Zaloguj się hasłem głównym i Kluczem tajnym z Zestawu awaryjnego. Twoje sejfy zostaną następnie pobrane i odszyfrowane na tym urządzeniu.",
  "onboarding.join.tabs": "Gdzie są Twoje dane synchronizacji?",
  "onboarding.join.tab.server": "Serwer Mosaic",
  "onboarding.join.tab.folder": "Folder w chmurze",
  "onboarding.join.serverUrl": "Adres serwera",
  "onboarding.join.email": "E-mail konta synchronizacji",
  "onboarding.join.password": "Hasło główne",
  "onboarding.join.secretKey": "Klucz tajny",
  "onboarding.join.secretKeyHint": "Z Zestawu awaryjnego. Zaczyna się od M1.",
  "onboarding.join.deviceName": "Nazwa tego urządzenia",
  "onboarding.join.chooseFolder": "Wybierz folder…",
  "onboarding.join.folder": "Folder",
  "onboarding.join.noAccounts":
    "Ten folder nie zawiera konta Mosaic. Wybierz folder, z którym synchronizuje się Twoje drugie urządzenie.",
  "onboarding.join.account": "Konto",
  "onboarding.join.accountOption": "Konto {id}",
  "onboarding.join.submit": "Dodaj to urządzenie",
  "onboarding.join.working": "Logowanie i pobieranie sejfów…",
  "onboarding.join.twoFactor.title": "Wpisz kod",
  "onboarding.join.twoFactor.body":
    "Twoje konto synchronizacji używa weryfikacji dwuetapowej. Wpisz 6-cyfrowy kod z aplikacji uwierzytelniającej.",
  "onboarding.join.twoFactor.code": "6-cyfrowy kod",
  "onboarding.join.twoFactor.submit": "Zweryfikuj",
  "onboarding.join.back": "Utwórz zamiast tego nowe konto",
} satisfies Record<keyof typeof en, string>;
