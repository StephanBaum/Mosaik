/**
 * Polish UI strings: sync. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { sync as en } from "../en/sync.ts";

export const sync = {
  "sync.status.signedOut": "Wylogowano",
  "sync.platform.windows": "Windows",
  "sync.platform.macos": "macOS",
  "sync.platform.linux": "Linux",
  "settings.sync.title": "Synchronizacja",
  "settings.sync.body":
    "Utrzymuj sejfy aktualne na wszystkich urządzeniach. Wszystko jest szyfrowane na tym urządzeniu, zanim je opuści.",
  "settings.sync.notSetUp": "Synchronizacja jest wyłączona. Sejfy są przechowywane tylko na tym urządzeniu.",
  "settings.sync.deviceName": "Nazwa tego urządzenia",
  "settings.sync.deviceName.default": "Mosaic na {platform}",
  "settings.sync.password": "Hasło główne",
  "settings.sync.code": "6-cyfrowy kod",
  "settings.sync.server.title": "Serwer Mosaic",
  "settings.sync.server.body":
    "Synchronizuj przez serwer Mosaic, który prowadzisz Ty lub ktoś, komu ufasz.",
  "settings.sync.server.url": "Adres serwera",
  "settings.sync.server.urlHint": "Na przykład https://sync.example.com",
  "settings.sync.server.check": "Sprawdź serwer",
  "settings.sync.server.checking": "Sprawdzanie serwera…",
  "settings.sync.server.registration.open": "Ten serwer pozwala każdemu utworzyć konto synchronizacji.",
  "settings.sync.server.registration.invite":
    "Ten serwer wymaga kodu zaproszenia, aby utworzyć konto synchronizacji.",
  "settings.sync.server.registration.closed":
    "Ten serwer nie przyjmuje nowych kont synchronizacji. Nadal możesz zalogować się na istniejące.",
  "settings.sync.server.choice.label": "Konto synchronizacji",
  "settings.sync.server.choice.signIn": "Mam konto synchronizacji",
  "settings.sync.server.choice.create": "Utwórz konto",
  "settings.sync.server.invite": "Kod zaproszenia",
  "settings.sync.server.create": "Utwórz konto synchronizacji",
  "settings.sync.server.signIn": "Zaloguj się",
  "settings.sync.server.signInAgain": "Zaloguj się ponownie",
  "settings.sync.server.working": "Łączenie…",
  "settings.sync.server.sees":
    "Serwer widzi tylko zaszyfrowane dane, e-mail konta i moment synchronizacji. Nigdy nie widzi hasła głównego ani Klucza tajnego.",
  "settings.sync.twoFactor.title": "Wpisz kod",
  "settings.sync.twoFactor.body":
    "To konto synchronizacji używa weryfikacji dwuetapowej. Wpisz 6-cyfrowy kod z aplikacji uwierzytelniającej.",
  "settings.sync.twoFactor.submit": "Zweryfikuj",
  "settings.sync.folder.title": "Własny folder w chmurze",
  "settings.sync.folder.body":
    "Synchronizuj przez folder, który Twoja usługa w chmurze (Dropbox, OneDrive, iCloud Drive i inne) utrzymuje aktualnym.",
  "settings.sync.folder.choose": "Wybierz folder…",
  "settings.sync.folder.chooseOther": "Wybierz inny folder…",
  "settings.sync.folder.path": "Folder",
  "settings.sync.folder.contents.empty": "Folder jest pusty. Mosaic może go użyć.",
  "settings.sync.folder.contents.mosaic": "Ten folder zawiera już dane synchronizacji Mosaic.",
  "settings.sync.folder.contents.other":
    "Ten folder zawiera inne pliki. Mosaic niczego tam nie zapisuje. Wybierz pusty folder lub utwórz nowy.",
  "settings.sync.folder.connect": "Połącz folder",
  "settings.sync.folder.working": "Łączenie…",
  "settings.sync.folder.sees":
    "Dostawca chmury i każdy z dostępem do folderu widzą tylko zaszyfrowane dane, ID konta i moment zmiany plików. Nigdy nie widzą hasła głównego ani Klucza tajnego.",
  "settings.sync.status.label": "Stan",
  "settings.sync.lastSynced": "Ostatnia synchronizacja: {when}",
  "settings.sync.neverSynced": "Jeszcze nie zsynchronizowano",
  "settings.sync.offline":
    "Brak połączenia. Zmiany są zapisane na tym urządzeniu i zostaną zsynchronizowane po odzyskaniu połączenia.",
  "settings.sync.now": "Synchronizuj teraz",
  "settings.sync.backend.server": "Serwer",
  "settings.sync.backend.account": "Konto synchronizacji",
  "settings.sync.backend.folder": "Folder",
  "settings.sync.backend.device": "To urządzenie",
  "settings.sync.signedOut.title": "To urządzenie jest wylogowane",
  "settings.sync.signedOut.body":
    "Serwer zakończył sesję tego urządzenia. Zaloguj się ponownie, aby dalej synchronizować. Twoje zmiany są bezpieczne na tym urządzeniu.",
  "settings.sync.recover.title": "Hasło główne zostało zmienione",
  "settings.sync.recover.body":
    "Serwer lub folder ma nowsze hasło główne niż to urządzenie. Wpisz nowe hasło główne, aby zaktualizować to urządzenie. Sejfy i Klucz tajny pozostają bez zmian.",
  "settings.sync.recover.password": "Nowe hasło główne",
  "settings.sync.recover.submit": "Zaktualizuj to urządzenie",
  "settings.sync.disconnect": "Rozłącz…",
  "settings.sync.disconnect.title": "Rozłączyć synchronizację?",
  "settings.sync.disconnect.body":
    "To urządzenie przestaje synchronizować i wylogowuje się z serwera. Sejfy pozostają na tym urządzeniu, a pozostałe urządzenia zachowują własne kopie.",
  "settings.sync.disconnect.confirm": "Rozłącz",
  "settings.sync.devices.title": "Urządzenia",
  "settings.sync.devices.thisDevice": "To urządzenie",
  "settings.sync.devices.unnamed": "Urządzenie bez nazwy",
  "settings.sync.devices.lastSeen": "Ostatnio widziane {when}",
  "settings.sync.devices.neverSeen": "Jeszcze niewidziane",
  "settings.sync.devices.rename": "Zmień nazwę",
  "settings.sync.devices.renameNamed": "Zmień nazwę: {name}",
  "settings.sync.devices.renameLabel": "Nowa nazwa tego urządzenia",
  "settings.sync.devices.remove": "Usuń",
  "settings.sync.devices.removeNamed": "Usuń {name}",
  "settings.sync.devices.removeTitle": "Usunąć {name}?",
  "settings.sync.devices.removeBody":
    "Usunięcie urządzenia wylogowuje je z synchronizacji. Nie odbiera kopii sejfów, którą już ma. Aby trwale zablokować urządzenie, zmień hasło główne i Klucz tajny.",
  "settings.sync.devices.removeConfirm": "Usuń urządzenie",
  "settings.sync.devices.folderNote":
    "Aby zablokować urządzenie, zmień hasło główne i Klucz tajny.",
  "settings.sync.phone.title": "Skonfiguruj telefon",
  "settings.sync.phone.body":
    "Pokaż kod, który telefon zeskanuje, aby nie trzeba było wpisywać Klucza tajnego. Telefon nadal potrzebuje hasła głównego.",
  "settings.sync.phone.button": "Skonfiguruj telefon",
  "settings.sync.phone.reauth":
    "Kod zawiera Twój Klucz tajny, więc wymaga hasła głównego.",
  "settings.sync.phone.dialogTitle": "Zeskanuj ten kod telefonem",
  "settings.sync.phone.steps":
    "Na telefonie otwórz Mosaic i wybierz konfigurację, a potem zeskanuj ten kod.",
  "settings.sync.phone.hint":
    "Kod ukrywa się po 60 sekundach lub po opuszczeniu tego okna. Nie udostępniaj jego zdjęcia.",
  "settings.sync.phone.canvas": "Kod konfiguracji dla telefonu",
  "settings.sync.phone.hidden.time": "Minęło 60 sekund, więc kod jest ukryty.",
  "settings.sync.phone.hidden.away": "Opuszczono to okno, więc kod jest ukryty.",
  "settings.sync.phone.again": "Pokaż ponownie",
  "settings.sync.phone.folderHint":
    "Telefon loguje się do Twojego magazynu w chmurze i otwiera ten sam folder.",
  "settings.sync.totp.title": "Aplikacja uwierzytelniająca",
  "settings.sync.totp.body":
    "Prosi o kod z aplikacji uwierzytelniającej, gdy ktoś loguje się na Twoje konto synchronizacji. Chroni konto na serwerze, a nie sejfy: te pozostają zaszyfrowane hasłem głównym i Kluczem tajnym.",
  "settings.sync.totp.on": "Kody z aplikacji uwierzytelniającej są włączone.",
  "settings.sync.totp.off": "Kody z aplikacji uwierzytelniającej są wyłączone.",
  "settings.sync.totp.enable": "Włącz…",
  "settings.sync.totp.disable": "Wyłącz…",
  "settings.sync.totp.beginTitle": "Włącz kody z aplikacji uwierzytelniającej",
  "settings.sync.totp.beginBody": "Wpisz hasło główne, aby zacząć.",
  "settings.sync.totp.begin": "Dalej",
  "settings.sync.totp.setupBody":
    "Dodaj ten klucz do aplikacji uwierzytelniającej, a potem wpisz 6-cyfrowy kod, który pokazuje.",
  "settings.sync.totp.secret": "Klucz konfiguracji",
  "settings.sync.totp.confirm": "Włącz",
  "settings.sync.totp.disableTitle": "Wyłącz kody z aplikacji uwierzytelniającej",
  "settings.sync.totp.disableBody": "Wpisz hasło główne i bieżący kod.",
  "settings.sync.totp.disableConfirm": "Wyłącz",
  "settings.sync.totp.enabledToast": "Kody z aplikacji uwierzytelniającej są włączone.",
  "settings.sync.totp.disabledToast": "Kody z aplikacji uwierzytelniającej są wyłączone.",
  "settings.sync.key.title": "Klucze bezpieczeństwa",
  "settings.sync.key.body":
    "Klucz bezpieczeństwa może być drugim krokiem przy logowaniu na konto synchronizacji. Chroni konto na serwerze, a nie sejfy. Dodanie lub użycie klucza otwiera przeglądarkę.",
  "settings.sync.key.none": "Nie dodano kluczy bezpieczeństwa.",
  "settings.sync.key.added": "Dodano {when}",
  "settings.sync.key.lastUsed": "Ostatnio użyty {when}",
  "settings.sync.key.neverUsed": "Jeszcze nie użyty",
  "settings.sync.key.add": "Dodaj klucz bezpieczeństwa…",
  "settings.sync.key.addTitle": "Dodaj klucz bezpieczeństwa",
  "settings.sync.key.addBody":
    "Nazwij klucz i wpisz hasło główne. Mosaic otworzy wtedy przeglądarkę, w której użyjesz klucza.",
  "settings.sync.key.label": "Nazwa klucza",
  "settings.sync.key.labelHint": "Na przykład „Klucz z biurka”. Do 64 znaków.",
  "settings.sync.key.addConfirm": "Dalej",
  "settings.sync.key.addedToast": "Dodano klucz bezpieczeństwa.",
  "settings.sync.key.noBackup":
    "Klucz bezpieczeństwa byłby Twoim jedynym drugim krokiem. Jeśli go utracisz, możesz nie być w stanie ponownie zalogować się na konto synchronizacji. Zalecamy zapasową metodę: drugi klucz lub kod z aplikacji uwierzytelniającej.",
  "settings.sync.key.recommend":
    "Zalecamy dwie metody, na przykład dwa klucze albo klucz i kod z aplikacji uwierzytelniającej.",
  "settings.sync.key.remove": "Usuń",
  "settings.sync.key.removeLabel": "Usuń {name}",
  "settings.sync.key.removeTitle": "Usunąć {name}?",
  "settings.sync.key.removeBody":
    "Potwierdzasz raz kluczem bezpieczeństwa w przeglądarce. Potem ten klucz przestaje działać przy logowaniu.",
  "settings.sync.key.removeLast":
    "To Twój ostatni drugi krok. Weryfikacja dwuetapowa zostaje wyłączona, a konto synchronizacji jest wtedy chronione wyłącznie hasłem głównym i Kluczem tajnym.",
  "settings.sync.key.removeCode": "Kod z aplikacji uwierzytelniającej",
  "settings.sync.key.removeCodeHint":
    "Wpisz bieżący kod, aby od razu usunąć klucz. Zostaw puste, aby zamiast tego potwierdzić kluczem bezpieczeństwa w przeglądarce.",
  "settings.sync.key.removeConfirm": "Usuń klucz",
  "settings.sync.key.removedToast": "Klucz bezpieczeństwa usunięty.",
  "settings.sync.key.confirmTitle": "Potwierdź kluczem bezpieczeństwa",
  "settings.sync.key.confirmBody":
    "Usunięcie klucza wymaga drugiego kroku. Wpisz hasło główne, a potem potwierdź kluczem w przeglądarce.",
  "settings.sync.key.step.methods": "Sprawdzanie, jak to konto się loguje…",
  "settings.sync.key.step.useKey": "Użyj klucza bezpieczeństwa (otwiera przeglądarkę)",
  "settings.sync.key.step.keyOnlyTitle": "Użyj klucza bezpieczeństwa",
  "settings.sync.key.step.keyOnlyBody":
    "To konto jest chronione kluczem bezpieczeństwa. Mosaic otwiera stronę w przeglądarce, w której go użyjesz.",
  "settings.sync.key.step.or": "Lub użyj kodu",
  "settings.sync.key.wait.login": "Zaloguj się kluczem bezpieczeństwa",
  "settings.sync.key.wait.register": "Dodaj klucz bezpieczeństwa",
  "settings.sync.key.wait.confirm": "Potwierdź kluczem bezpieczeństwa",
  "settings.sync.key.wait.lead":
    "Przeglądarka otworzyła stronę Mosaic z Twojego serwera synchronizacji. Wpisz tam ten kod, a potem użyj klucza bezpieczeństwa.",
  "settings.sync.key.wait.codeLabel": "Kod do wpisania w przeglądarce",
  "settings.sync.key.wait.codeSpoken": "Kod do wpisania w przeglądarce: {digits}",
  "settings.sync.key.wait.expires": "Ten krok wygasa o {time}.",
  "settings.sync.key.wait.waiting": "Czekanie na klucz bezpieczeństwa…",
  "settings.sync.key.wait.done": "Gotowe.",
  "settings.sync.key.wait.expired": "Ten krok przekroczył limit czasu. Zacznij go od nowa.",
  "settings.sync.key.wait.failed":
    "Krok nie powiódł się: kod był zbyt często błędny lub klucz bezpieczeństwa nie został zaakceptowany. Nic nie zmieniono. Zacznij od nowa.",
  "settings.sync.key.wait.reopen": "Otwórz stronę w przeglądarce ponownie",
  "settings.sync.key.wait.retry": "Zacznij od nowa",
  "sync.warning.region": "Ostrzeżenia synchronizacji",
  "sync.warning.unknownDevice": "inne urządzenie",
  "sync.warning.unknownVault": "jeden z Twoich sejfów",
  "sync.warning.freezeSuspected":
    "Zmiany z: {device} nie docierają od ponad doby. Sprawdź, czy drugie urządzenie jest włączone i online.",
  "sync.warning.missingFiles":
    "Brakuje w folderze plików z: {device}. Poczekaj, aż usługa w chmurze zakończy synchronizację, a potem zsynchronizuj ponownie.",
  "sync.warning.noCommonView":
    "To urządzenie i {device} nie widziały tych samych danych serwera od dwóch dni. Serwer może pokazywać im różne wersje.",
  "sync.warning.rollbackDetected":
    "Dane synchronizacji wyglądają na starsze niż na tym urządzeniu. Sprawdź serwer lub folder, a potem wybierz „Sprawdzono”.",
  "sync.warning.identityDuplicated":
    "Inna kopia Mosaic zapisuje dane pod tożsamością tego urządzenia, na przykład po przywróceniu kopii zapasowej. Synchronizacja jest wstrzymana. Upewnij się, że używana jest tylko jedna kopia, a potem wybierz „Sprawdzono”.",
  "sync.warning.serverInconsistent":
    "Serwer pokazał temu urządzeniu inne dane niż innemu urządzeniu. Sprawdź serwer, a potem wybierz „Sprawdzono”.",
  "sync.warning.serverRollback":
    "Kopia {vault} na serwerze jest starsza niż ta na tym urządzeniu. Sprawdź serwer, a potem wybierz „Sprawdzono”.",
  "sync.warning.undecryptable":
    "Nie można było odczytać na tym urządzeniu: {count, plural, one {# odebrany element} few {# odebrane elementy} many {# odebranych elementów} other {# odebranego elementu}}. Zaktualizuj Mosaic na wszystkich urządzeniach, a potem zsynchronizuj ponownie.",
  "sync.warning.hardwareVaultsLocal":
    "Sejfy chronione sprzętowo pozostają na tym urządzeniu. Niezsynchronizowane: {count, plural, one {# sejf} few {# sejfy} many {# sejfów} other {# sejfu}}.",
  "sync.warning.attachmentsNotRepaired":
    "Nie udało się naprawić z tego urządzenia na serwerze lub w folderze: {count, plural, one {# plik} few {# pliki} many {# plików} other {# pliku}}. Otwórz element na urządzeniu, które nadal ma plik, i dodaj go ponownie.",
  "sync.warning.acknowledge": "Sprawdzono…",
  "sync.acknowledge.title": "Wyczyścić ostrzeżenia synchronizacji wymagające sprawdzenia?",
  "sync.acknowledge.body":
    "Kontynuuj dopiero po samodzielnym sprawdzeniu serwera lub folderu. To czyści każde ostrzeżenie wymagające sprawdzenia, a synchronizacja jest kontynuowana. Jeśli Twoje dane naprawdę zostały cofnięte lub zmienione, Mosaic nie będzie potem w stanie tego stwierdzić.",
  "sync.acknowledge.eachDevice": "Zrób to na każdym ze swoich urządzeń.",
  "toast.syncAttention":
    "Synchronizacja scaliła lub naprawiła niektóre elementy z innego urządzenia. Sprawdź, co się zmieniło.",
  "sync.revoked.rekey":
    "Klucze tego konta zostały odnowione na innym urządzeniu: zaloguj się nowym Kluczem tajnym z najnowszego Zestawu awaryjnego.",
  "sync.revoked.passwordChange":
    "Hasło główne zostało zmienione na innym urządzeniu: zaloguj się nowym hasłem.",
  "sync.revoked.deviceRemoved":
    "To urządzenie zostało usunięte z konta na innym urządzeniu. Zaloguj się ponownie, aby dodać je z powrotem.",
  "sync.revoked.operator":
    "Operator serwera zakończył tę sesję. Zaloguj się ponownie, aby dalej synchronizować.",
} satisfies Record<keyof typeof en, string>;
