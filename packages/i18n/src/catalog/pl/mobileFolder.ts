/**
 * Polish UI strings: mobileFolder. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { mobileFolder as en } from "../en/mobileFolder.ts";

export const mobileFolder = {
  "mobile.sync.never": "Jeszcze nie zsynchronizowano",
  "mobile.sync.last": "Zsynchronizowano {time}",
  "mobile.sync.localOnly":
    "Ten sejf jest tylko na tym telefonie. Synchronizacja nie jest skonfigurowana.",
  "mobile.sync.pending":
    "{count, plural, one {# zmiana czeka na wysłanie} few {# zmiany czekają na wysłanie} many {# zmian czeka na wysłanie} other {# zmiany czeka na wysłanie}}",
  "mobile.sync.now": "Synchronizuj teraz",
  "mobile.sync.working": "Synchronizowanie",
  "mobile.sync.warnings": "Ostrzeżenia synchronizacji",
  "mobile.sync.warning.freezeSuspected":
    "Zmiany z innego urządzenia nie docierają od jakiegoś czasu. Sprawdź, czy drugie urządzenie jest włączone i online.",
  "mobile.sync.warning.noCommonView":
    "Ten telefon i inne urządzenie nie widziały tych samych danych serwera od dwóch dni. Serwer może pokazywać im różne wersje. Zsynchronizuj drugie urządzenie, a potem tutaj.",
  "mobile.sync.warning.rollbackDetected":
    "Dane synchronizacji wyglądają na starsze niż na tym telefonie. Otwórz Mosaic na innym urządzeniu i sprawdź, czy są tam Twoje najnowsze zmiany. Potem wybierz „Sprawdzono”.",
  "mobile.sync.warning.identityDuplicated":
    "Inna kopia Mosaic zapisuje dane pod tożsamością tego telefonu, na przykład po przywróceniu kopii zapasowej. Synchronizacja jest wstrzymana. Upewnij się, że używana jest tylko jedna kopia, a potem wybierz „Sprawdzono”.",
  "mobile.sync.warning.serverInconsistent":
    "Serwer pokazał temu telefonowi inne dane niż innemu urządzeniu. Otwórz Mosaic na innym urządzeniu i sprawdź, czy są tam Twoje najnowsze zmiany. Potem wybierz „Sprawdzono”.",
  "mobile.sync.warning.serverRollback":
    "Kopia jednego z Twoich sejfów na serwerze jest starsza niż ta na telefonie. Otwórz Mosaic na innym urządzeniu i sprawdź, czy są tam Twoje najnowsze zmiany. Potem wybierz „Sprawdzono”.",
  "mobile.sync.warning.sharingAttention":
    "Udostępniony sejf wymaga uwagi: czeka zmiana klucza lub nie można było sprawdzić członka. Otwórz Mosaic na komputerze, aby zobaczyć, o co chodzi. Potem wybierz „Sprawdzono”.",
  "mobile.sync.warning.undecryptable":
    "Nie można było odczytać na tym telefonie: {count, plural, one {# odebrany element} few {# odebrane elementy} many {# odebranych elementów} other {# odebranego elementu}}. Zaktualizuj Mosaic na wszystkich urządzeniach, a potem zsynchronizuj ponownie.",
  "mobile.sync.warning.accountRollback":
    "Serwer pokazuje starszy klucz konta niż ten na telefonie. Nic nie zostało zmienione. Otwórz Mosaic na innym urządzeniu i sprawdź, czy są tam Twoje najnowsze zmiany. Potem wybierz „Sprawdzono”.",
  "mobile.sync.acknowledge": "Sprawdzono…",
  "mobile.sync.acknowledgeTitle": "Wyczyścić ostrzeżenia synchronizacji?",
  "mobile.sync.acknowledgeBody":
    "Kontynuuj dopiero po samodzielnym sprawdzeniu serwera. To czyści wszystkie ostrzeżenia, a synchronizacja jest kontynuowana. Jeśli Twoje dane naprawdę zostały cofnięte lub zmienione, Mosaic nie będzie potem w stanie tego stwierdzić. Zrób to na każdym ze swoich urządzeń.",
  "mobile.sync.acknowledgeConfirm": "Sprawdzono",
  "mobile.folder.intro": "To konto synchronizuje się przez Twój folder {provider}.",
  "mobile.folder.signIn": "Zaloguj się do {provider}",
  "mobile.folder.scope":
    "{provider} poprosi o dostęp do wszystkich Twoich plików. Mosaic otwiera tylko wybrany folder i niczego innego nie dotyka.",
  "mobile.folder.waiting": "Dokończ logowanie w przeglądarce, a potem wróć tutaj.",
  "mobile.folder.serverLabel": "Adres Nextcloud lub WebDAV",
  "mobile.folder.serverHint": "https://cloud.example.com",
  "mobile.folder.serverConfirm":
    "Mosaic zaloguje się na {server}. Kontynuuj tylko wtedy, gdy to Twój własny serwer.",
  "mobile.folder.locating": "Szukanie folderu Mosaic…",
  "mobile.folder.browseTitle": "Wybierz folder Mosaic",
  "mobile.folder.browseHint":
    "To folder, który synchronizuje Twój komputer, z plikiem o nazwie mosaic-sync.json.",
  "mobile.folder.currentFolder": "Bieżący folder: {path}",
  "mobile.folder.noSubfolders": "Tu nie ma folderów.",
  "mobile.folder.useThis": "Użyj tego folderu",
  "mobile.folder.up": "Poziom wyżej",
  "mobile.folder.chooseProvider": "Gdzie synchronizuje się Twoje konto?",
  "mobile.folder.noDrive":
    "Google Drive i iCloud Drive nie działają jeszcze na Androidzie. Użyj Dropbox, OneDrive, Nextcloud lub synchronizacji Mosaic.",
  "mobile.folder.useFolder": "Synchronizuj zamiast tego przez folder w chmurze",
  "mobile.folder.providerDropbox": "Dropbox",
  "mobile.folder.providerOneDrive": "OneDrive",
  "mobile.folder.providerWebDav": "Nextcloud lub WebDAV",
  "mobile.folder.appPasswordToggle": "Użyj zamiast tego hasła aplikacji",
  "mobile.folder.davLabel": "Adres folderu WebDAV",
  "mobile.folder.loginLabel": "Nazwa użytkownika",
  "mobile.folder.appPasswordLabel": "Hasło aplikacji",
  "mobile.folder.appPasswordHint":
    "Utwórz hasło aplikacji w ustawieniach zabezpieczeń serwera. Nie używaj hasła głównego.",
  "mobile.folder.appPasswordAction": "Zaloguj się hasłem aplikacji",
  "mobile.folder.pickAccount": "Do którego konta dołącza ten telefon?",
  "mobile.folder.accountOption": "Konto {id}",
  "mobile.folder.linked": "Zalogowano. Synchronizacja działa ponownie.",
  "mobile.folder.settingsLine": "Synchronizuje przez {provider}: {path}",
  "mobile.folder.relink": "Zaloguj się ponownie",
  "mobile.folder.msRevokeHint":
    "Aby całkowicie usunąć dostęp Mosaic, usuń go na stronie konta Microsoft w sekcji Aplikacje.",
  "mobile.reset.revokeDropbox":
    "Konto zostało usunięte. Nie udało się zakończyć dostępu Mosaic w Dropbox: usuń go w Dropbox w Ustawienia, Połączone aplikacje.",
  "mobile.reset.revokeWebdav":
    "Konto zostało usunięte. Nie udało się usunąć hasła aplikacji na Twoim serwerze: usuń je tam w Ustawienia, Bezpieczeństwo, Urządzenia i sesje.",
  "mobile.reset.title": "Usuń to konto z tego telefonu",
  "mobile.reset.body":
    "To usuwa kopię sejfu, Klucz tajny i logowanie do chmury zapisane na tym telefonie. Twoje dane w folderze w chmurze lub na serwerze pozostają. Możesz później dołączyć ponownie z Kluczem tajnym i hasłem głównym.",
  "mobile.reset.action": "Usuń konto…",
  "mobile.reset.confirm": "Usuń konto",
  "mobile.reset.phraseLabel": "Potwierdzenie",
  "mobile.reset.typePhrase": "Wpisz {phrase}, aby potwierdzić",
} satisfies Record<keyof typeof en, string>;
