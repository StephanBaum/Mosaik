/**
 * Polish UI strings: errors. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { errors as en } from "../en/errors.ts";

export const errors = {
  "error.notImplemented": "To jeszcze nie jest dostępne. Pojawi się w jednej z kolejnych wersji Mosaic.",
  "error.noAccount":
    "Na tym urządzeniu nie ma jeszcze konta. Utwórz je lub zaloguj się za pomocą Zestawu awaryjnego.",
  "error.accountExists": "To urządzenie ma już konto Mosaic. Odblokuj je zamiast tworzyć nowe.",
  "error.locked": "Mosaic jest zablokowany. Odblokuj go, aby kontynuować.",
  "error.alreadyUnlocked": "Mosaic jest już odblokowany. Możesz kontynuować od miejsca, w którym skończono.",
  "error.wrongCredentials":
    "Hasło główne lub Klucz tajny nie zadziałały. Sprawdź Caps Lock i układ klawiatury, a potem spróbuj ponownie.",
  "error.secretKeyRequired":
    "To urządzenie nie ma Twojego Klucza tajnego. Wpisz go z Zestawu awaryjnego.",
  "error.secretKeyMalformed":
    "To nie jest Klucz tajny. Twój zaczyna się od „M1-” i znajduje się w Zestawie awaryjnym.",
  "error.secretKeyMistyped":
    "W tym Kluczu tajnym jest literówka. Porównaj go z Zestawem awaryjnym, znak po znaku.",
  "error.emptyPassword": "Hasło główne jest puste. Wpisz je, aby kontynuować.",
  "error.quickUnlockUnavailable":
    "Szybkie odblokowanie nie jest skonfigurowane na tym urządzeniu. Odblokuj hasłem głównym.",
  "error.storeInUse":
    "Mosaic jest już otwarty na tym komputerze. Zamknij drugie okno, a potem spróbuj ponownie.",
  "error.storeUnavailable":
    "Mosaic nie mógł otworzyć pliku sejfu na tym urządzeniu. Sprawdź, czy dysk jest dostępny i ma wolne miejsce, a potem uruchom Mosaic ponownie.",
  "error.vaultNotFound": "Ten sejf już nie istnieje. Mógł zostać usunięty na innym urządzeniu.",
  "error.vaultUnavailable":
    "Tego sejfu nie można otworzyć w tej sesji. Odblokuj go kluczem sprzętowym lub zsynchronizuj i spróbuj ponownie.",
  "error.itemNotFound":
    "Ten element już nie istnieje. Mógł zostać przeniesiony lub usunięty na innym urządzeniu.",
  "error.itemDeleted":
    "Ten element został usunięty na zawsze, więc nie można go otworzyć. Wybierz inny element.",
  "error.sameVault": "Element jest już w tym sejfie. Wybierz inny sejf.",
  "error.noOtp": "Ten element nie ma kodu jednorazowego. Edytuj element, aby go dodać.",
  "error.fieldEmpty":
    "To pole jest puste, więc nie ma czego kopiować ani pokazywać. Edytuj element, aby je wypełnić.",
  "error.invalidInput":
    "Coś w tym formularzu jest nieprawidłowe. Sprawdź wyróżnione pola i spróbuj ponownie.",
  "error.sshKeyInvalid":
    "To nie jest klucz prywatny OpenSSH. Wklej cały klucz, razem z wierszami BEGIN i END.",
  "error.reauthRequired": "Ta zmiana wymaga hasła głównego. Wpisz je, aby potwierdzić.",
  "error.attachmentTooLarge": "Ten plik jest większy niż 50 MiB. Wybierz mniejszy plik.",
  "error.attachmentQuota":
    "Twoje pliki zajęłyby więcej niż dostępny 1 GiB. Usuń część załączników, a potem spróbuj ponownie.",
  "error.attachmentNotFound": "Ten plik nie jest już częścią elementu. Wczytaj element ponownie.",
  "error.attachmentUnavailable":
    "Tego pliku nie można otworzyć na tym urządzeniu. Może jeszcze nie został zsynchronizowany lub jest uszkodzony.",
  "error.attachmentFileUnreadable": "Nie można odczytać tego pliku. Wybierz inny plik.",
  "error.attachmentSaveFailed": "Nie udało się zapisać pliku w tym miejscu. Wybierz inny folder.",
  "error.attachmentPreviewUnsupported":
    "Ten plik nie ma podglądu. Użyj Zapisz jako, aby go otworzyć.",
  "error.attachmentPreviewTooLarge":
    "Ten obraz jest zbyt duży na podgląd. Użyj Zapisz jako, aby go otworzyć.",
  "error.clipboardUnavailable":
    "Mosaic nie mógł użyć schowka. Zamknij aplikacje, które mogą go blokować, a potem spróbuj ponownie.",
  "error.importFileUnreadable":
    "Mosaic nie mógł odczytać tego pliku. Sprawdź, czy nadal istnieje i nie jest otwarty w innej aplikacji, a potem spróbuj ponownie.",
  "error.importDeleteNotAllowed":
    "Najpierw zaimportuj plik. Mosaic usuwa eksport dopiero wtedy, gdy jego elementy są w Twoim sejfie.",
  "error.importDeleteFailed":
    "Nie udało się usunąć pliku. Może być otwarty w innej aplikacji lub tylko do odczytu. Usuń go samodzielnie, a potem opróżnij kosz.",
  "error.importFormatInvalid":
    "Ten plik nie ma wybranego formatu. Wybierz pasujący format lub wyeksportuj plik ponownie.",
  "error.importPreviewExpired": "Ten import został anulowany. Wybierz plik ponownie, aby zacząć od nowa.",
  "error.sshRequestExpired":
    "To żądanie SSH już się zakończyło. Jeśli nadal tego potrzeba, uruchom polecenie ponownie.",
  "error.sshAgentUnavailable":
    "Agent SSH nie mógł się uruchomić. Sprawdź, czy nie działa inny agent SSH, a potem włącz go ponownie.",
  "error.unlockDelayed": "Zbyt wiele błędnych prób z rzędu. Odczekaj kilka sekund, a potem spróbuj ponownie.",
  "error.tooManyAttempts":
    "Zbyt wiele błędnych odpowiedzi. Mosaic został zablokowany. Odblokuj go hasłem głównym, aby kontynuować.",
  "error.sshAllowTooSoon":
    "To było zbyt szybko, aby przeczytać żądanie. Sprawdź klucz i program, a potem ponownie wybierz Zezwól.",
  "error.browserRequestExpired":
    "To żądanie przeglądarki już się zakończyło. Jeśli nadal tego potrzeba, poproś ponownie z poziomu rozszerzenia.",
  "error.browserAllowTooSoon":
    "To było zbyt szybko, aby przeczytać żądanie. Sprawdź szczegóły, a potem wybierz ponownie.",
  "error.browserPairingNotFound":
    "Ta przeglądarka nie jest już połączona. Otwórz Ustawienia ponownie, aby zobaczyć aktualną listę.",
  "error.browserHostMissing":
    "Ta kopia Mosaic nie ma zainstalowanego konektora przeglądarki, więc nie ma czego naprawiać. Zainstaluj Mosaic za pomocą instalatora, aby połączyć przeglądarkę.",
  "error.agentOriginInvalid":
    "Agenci mogą wysłać klucz tylko do witryny, której adres zaczyna się od https:// i zawiera wyłącznie nazwę witryny, na przykład https://api.example.com. Sprawdź adres.",
  "error.agentBindingInvalid":
    "Mosaic nie może użyć tego ustawienia dla agentów. Sprawdź pole, witryny i sposób wysyłania klucza.",
  "error.agentValueUnusable":
    "Ta wartość jest zbyt krótka lub zawiera podział wiersza, więc agenci nie mogą jej używać. Klucz musi mieć co najmniej 12 znaków w jednym wierszu.",
  "error.agentVaultShared":
    "Agenci mogą używać tylko elementów z sejfu, który należy do Ciebie i nie jest udostępniony. Najpierw przenieś element do jednego z własnych sejfów.",
  "error.agentAccessOff":
    "Dostęp agentów jest wyłączony. Włącz go w Ustawieniach, w sekcji Agenci.",
  "error.agentRequestExpired":
    "To żądanie agenta już się zakończyło. Agent może poprosić ponownie, jeśli nadal tego potrzebuje.",
  "error.agentAllowTooSoon":
    "To było zbyt szybko, aby przeczytać żądanie. Sprawdź szczegóły, a potem wybierz ponownie.",
  "error.agentPairingNotFound":
    "Ten agent nie jest już połączony. Otwórz Ustawienia ponownie, aby zobaczyć aktualną listę.",
  "error.agentEndpointUnavailable":
    "Mosaic nie mógł otworzyć połączenia używanego przez agentów. Zamknij inne kopie Mosaic i spróbuj ponownie.",
  "error.kitNotVerified":
    "Najpierw zapisz nowy Zestaw awaryjny i potwierdź jego ostatnią grupę. Do tego czasu Mosaic niczego nie zmienia.",
  "error.kitGroupMismatch":
    "To nie jest ostatnia grupa nowego Klucza tajnego. Sprawdź nowy Zestaw awaryjny.",
  "error.rotationInProgress":
    "Inna wymiana kluczy jest nadal otwarta. Dokończ ją przyciskiem Kontynuuj wymianę kluczy lub najpierw ją odrzuć.",
  "error.rotationNone": "Nie ma wymiany kluczy do kontynuowania.",
  "error.rotationNotFound":
    "Ta zmiana hasła nie jest już otwarta. Mogła zostać zamknięta na innym urządzeniu lub w przeglądarce. Otwórz element ponownie, aby zobaczyć jej stan.",
  "error.rotationLimit":
    "Cztery zmiany hasła tego elementu są już otwarte. Najpierw zakończ lub zamknij jedną w elemencie.",
  "error.rotationOpen":
    "To logowanie ma otwartą zmianę hasła. Przywróć je z kosza, a potem w elemencie wybierz Udało się lub Nie udało się. Po tym można je usunąć na zawsze.",
  "error.rotationNotSupported":
    "Mosaic nie może poprowadzić tej zmiany. Potrzebne jest logowanie z adresem https. Zmień hasło w witrynie samodzielnie.",
  "error.historyNotFound":
    "Tego wcześniejszego hasła nie ma już w historii. Otwórz element ponownie, aby zobaczyć aktualną listę.",
  "error.originNotRelated":
    "Ta strona nie należy do witryny tego logowania, więc Mosaic jej nie dodał. Edytuj adresy elementu, jeśli powinna tam być.",
  "error.breachConsentRequired":
    "Sprawdzanie wycieków jest wyłączone. Najpierw włącz je i zgódź się na to, co wysyła.",
  "error.breachRunning":
    "Sprawdzanie wycieków już trwa. Poczekaj na jego zakończenie lub najpierw je zatrzymaj.",
  "error.rotationTooFar":
    "Ta zmiana dotarła już do Twojego konta, więc nie można jej odrzucić. Kontynuuj ją, aby ją zakończyć.",
  "error.rotationJumpTooLarge":
    "To urządzenie jest zbyt daleko w tyle za innymi urządzeniami, aby nadrobić zaległości. Skonfiguruj je ponownie: usuń konto z tego urządzenia i dołącz ponownie za pomocą Zestawu awaryjnego.",
  "error.rotationNotApplied":
    "Zmiana nigdy nie dotarła do Twojego konta, więc nic nie zostało zmienione. Zacznij od nowa lub odrzuć ją.",
  "error.secretKeyUnchanged":
    "Ten Klucz tajny już nie działa. Użyj tego z Zestawu awaryjnego zapisanego na urządzeniu, które wykonało zmianę.",
  "error.passwordUnchanged": "Nowe hasło główne musi różnić się od obecnego.",
  "error.profileDowngrade":
    "Ten poziom nie jest silniejszy niż ten, którego teraz używa Twoje konto. Wybierz wyższy.",
  "error.profileTooSlow":
    "Odblokowanie na tym poziomie zajęłoby temu urządzeniu ponad dwie sekundy. Wybierz niższy poziom lub mimo to zmień.",
  "error.rekeyNeedsCleanSync":
    "Wymiana kluczy wymaga aktualnej synchronizacji bez ostrzeżeń. Zsynchronizuj teraz, sprawdź ewentualne ostrzeżenia, a potem spróbuj ponownie.",
  "error.rekeyMemberKeyMismatch":
    "Kody bezpieczeństwa wszystkich osób w udostępnionym sejfie nie zostały jeszcze porównane, więc tego sejfu nie można odnowić. Porównaj ich kody w ustawieniach udostępniania sejfu, a potem spróbuj ponownie.",
  "error.rekeyBusy": "Twoje sejfy wciąż się zmieniały, gdy Mosaic pracował. Chwilę poczekaj i spróbuj ponownie.",
  "error.rekeyOutcomeUnknown":
    "Mosaic nie mógł potwierdzić, czy Twoje konto przyjęło nowe klucze. Wybierz Kontynuuj wymianę kluczy, aby się dowiedzieć.",
  "error.rekeyNeedsServer":
    "Wymiana kluczy wymaga Twojego serwera lub folderu synchronizacji, a nie można się z nim połączyć. Sprawdź połączenie i spróbuj ponownie.",
  "error.rekeyNoAccountFile":
    "Nic w Twoim folderze synchronizacji nie otwiera się tym hasłem i Kluczem tajnym. Sprawdź nowy Zestaw awaryjny.",
  "error.syncAccountRekeyed":
    "Klucze Twojego konta zostały odnowione na innym urządzeniu. Aby kontynuować, użyj Klucza tajnego z nowego Zestawu awaryjnego.",
  "error.syncEpochRollback":
    "Twój backend synchronizacji pokazuje starsze klucze konta niż to urządzenie. Nic nie zostało zmienione.",
  "error.syncRekeyRefused":
    "Klucze oferowane przez Twój backend synchronizacji nie są następnymi po kluczach tego urządzenia. Nic nie zostało zmienione. Sprawdź inne urządzenia.",
  "error.pickerStale": "Ta lista jest nieaktualna. Kliknij pole ponownie.",
  "error.pickerTooSoon": "Chwilę poczekaj, a potem potwierdź ponownie.",
  "error.programFillUnavailable": "Wypełnianie w programach nie jest dostępne na tym urządzeniu.",
  "error.busy": "Mosaic nadal kończy poprzedni krok. Chwilę poczekaj, a potem spróbuj ponownie.",
  "error.internal":
    "Coś nieoczekiwanego przerwało to działanie. Spróbuj ponownie, a jeśli problem się powtarza, uruchom Mosaic ponownie.",
  "error.importTooLarge":
    "Ten plik jest zbyt duży lub zbyt złożony, aby go zaimportować. Eksportuj naraz mniej elementów, a potem spróbuj ponownie.",
  "error.importUnsupported":
    "Mosaic nie może odczytać tej wersji formatu. Wyeksportuj ponownie w aktualnym formacie (dla KeePass: KDBX 4 z AES lub ChaCha20), a potem spróbuj ponownie.",
  "error.importEncrypted":
    "Ten eksport jest chroniony hasłem. Wyeksportuj ponownie bez szyfrowania, zaimportuj go, a potem usuń plik.",
  "error.sshAgentEndpointInUse":
    "Działa już inny agent SSH (w systemie Windows usługa OpenSSH Authentication Agent). Zatrzymaj go, a potem ponownie włącz agenta SSH Mosaic.",
  "error.cancelled": "Anulowano. Nic się nie zmieniło.",
  "error.quickUnlockExpired":
    "Szybkie odblokowanie wygasło. Odblokuj hasłem głównym, a potem włącz je ponownie w Ustawieniach.",
  "error.quickUnlockInvalidated":
    "Szybkie odblokowanie przestało działać na tym urządzeniu i zostało wyłączone. Odblokuj hasłem głównym.",
  "error.passwordRequired":
    "Tym razem wpisz hasło główne. Potem szybkie odblokowanie znów będzie działać.",
  "error.hardwareKeyUnsupported": "Tego klucza bezpieczeństwa lub systemu nie można używać z Mosaic.",
  "error.hardwareKeyNotFound": "Nie znaleziono klucza bezpieczeństwa. Włóż go i spróbuj ponownie.",
  "error.hardwareKeyTimeout": "Klucz nie został dotknięty na czas. Spróbuj ponownie.",
  "error.hardwareKeyPinRequired": "Wpisz PIN klucza bezpieczeństwa.",
  "error.hardwareKeyPinInvalid": "Błędny PIN. Po kilku błędnych próbach klucz sam się blokuje.",
  "error.hardwareKeyPinBlocked":
    "Ten klucz jest zablokowany po zbyt wielu błędnych PIN-ach. Zresetuj go narzędziem producenta; jego rejestracje zostaną wtedy utracone.",
  "error.hardwareKeyNoPin": "Najpierw ustaw PIN na tym kluczu za pomocą narzędzia producenta.",
  "error.hardwareKeyNotRecognized":
    "Ten klucz nie jest tu zarejestrowany. Spróbuj innego klucza lub hasła głównego.",
  "error.hardwareKeyLimit": "Zarejestrowano maksymalną dozwoloną liczbę kluczy (8). Najpierw usuń jeden.",
  "error.hardwareKeyLast":
    "To jedyny klucz tego sejfu. Zarejestruj inny, zanim go usuniesz.",
  "error.itemTypeNotAllowed": "Ten sejf przechowuje tylko kody jednorazowe.",
  "error.otpScanUnsupported":
    "Skanowanie ekranu nie jest dostępne w tym systemie. Wklej zamiast tego zrzut ekranu.",
  "error.otpQrNotFound":
    "Nie znaleziono kodu QR. Upewnij się, że cały kod jest widoczny, a potem spróbuj ponownie.",
  "error.otpQrNotOtp":
    "Ten kod QR nie jest kodem konfiguracji dwuskładnikowej. Poszukaj kodu na stronie konfiguracji 2FA w witrynie.",
  "error.otpQrMigration":
    "To kod eksportu Google Authenticator, którego Mosaic jeszcze nie odczyta. Skonfiguruj każde konto ponownie w jego witrynie lub wpisz jego klucz konfiguracji.",
  "error.otpQrMultiple":
    "Widocznych jest więcej niż jeden kod konfiguracji. Pokaż tylko ten, który ma być użyty, a potem spróbuj ponownie.",
  "error.otpQrInvalid":
    "Ten kod konfiguracji używa ustawień, których Mosaic nie obsługuje. Wpisz zamiast tego klucz konfiguracji.",
  "error.otpUriInvalid": "To nie jest prawidłowy link otpauth://. Skopiuj cały link ponownie.",
  "error.otpKeyInvalid":
    "Ten klucz jest nieprawidłowy. Używa tylko liter od A do Z i cyfr od 2 do 7; sprawdź, czy nie ma literówek.",
  "error.otpImageUnreadable": "Mosaic nie może odczytać tego obrazu. Użyj pliku PNG lub JPEG.",
  "error.otpImageTooLarge": "Ten obraz jest zbyt duży. Użyj zrzutu ekranu samego kodu QR.",
  "error.otpClipboardNoImage":
    "W schowku nie ma obrazu. Zrób zrzut ekranu kodu QR, a potem wklej go tutaj.",
  "error.otpPendingExpired": "Zeskanowany kod nie jest już dostępny. Zeskanuj go ponownie.",
  "error.syncNotConfigured":
    "Synchronizacja nie jest jeszcze skonfigurowana na tym urządzeniu. Najpierw skonfiguruj ją w Ustawieniach.",
  "error.syncAlreadyConfigured":
    "Synchronizacja jest już skonfigurowana. Aby użyć innego serwera lub folderu, najpierw rozłącz.",
  "error.syncServerUnreachable":
    "Mosaic nie mógł połączyć się z serwerem. Sprawdź adres i połączenie z internetem, a potem spróbuj ponownie.",
  "error.syncServerInvalid":
    "To nie wygląda na serwer Mosaic. Sprawdź adres; powinien wyglądać jak https://sync.example.com.",
  "error.syncServerInsecure":
    "Mosaic łączy się z serwerami tylko przez https, aby nikt po drodze nie mógł odczytać ani zmienić Twoich danych. Użyj adresu https://.",
  "error.syncServerMismatch":
    "To nie jest serwer, z którym to konto się synchronizuje. Sprawdź adres.",
  "error.syncServerIncompatible":
    "Ten serwer działa w wersji, która nie pasuje do Twojego Mosaic. Zaktualizuj Mosaic lub poproś właściciela serwera o aktualizację.",
  "error.syncRegistrationClosed":
    "Ten serwer nie przyjmuje nowych kont synchronizacji. Zaloguj się na istniejące lub zapytaj właściciela serwera.",
  "error.syncInviteInvalid":
    "Ten kod zaproszenia jest nieprawidłowy lub został już użyty. Sprawdź go lub poproś właściciela serwera o nowy.",
  "error.syncAccountExists":
    "Masz już konto synchronizacji na tym serwerze. Wybierz zamiast tego Zaloguj się.",
  "error.syncAccountMismatch":
    "Konto synchronizacji na tym serwerze należy do innego konta Mosaic. Sprawdź adres i swój e-mail.",
  "error.syncTwoFactorRequired":
    "To konto synchronizacji wymaga kodu z aplikacji uwierzytelniającej. Wpisz 6-cyfrowy kod.",
  "error.syncTwoFactorInvalid":
    "Ten kod jest nieprawidłowy lub wygasł. Wpisz aktualny 6-cyfrowy kod z aplikacji uwierzytelniającej.",
  "error.syncSignedOut":
    "Serwer wylogował to urządzenie. Zaloguj się ponownie, aby dalej synchronizować. Twoje zmiany są bezpieczne na tym urządzeniu.",
  "error.syncRateLimited": "Zbyt wiele prób. Odczekaj kilka minut, a potem spróbuj ponownie.",
  "error.syncQuotaExceeded":
    "Na serwerze lub w folderze nie ma już miejsca na Twoje dane. Zwolnij miejsce lub poproś właściciela serwera o więcej.",
  "error.syncFolderUnavailable":
    "Mosaic nie może użyć tego folderu. Sprawdź, czy istnieje, czy usługa chmurowa zakończyła jego synchronizację i czy Mosaic może w nim zapisywać.",
  "error.syncFolderNotEmpty":
    "Ten folder zawiera inne pliki, więc Mosaic nie będzie w nim zapisywać. Wybierz pusty folder lub utwórz nowy.",
  "error.syncFolderNoAccount":
    "Ten folder nie zawiera danych synchronizacji dla tego konta. Wybierz folder, z którym synchronizuje się Twoje drugie urządzenie.",
  "error.syncFolderDamaged":
    "Niektóre pliki synchronizacji w tym folderze są uszkodzone. Poczekaj, aż usługa chmurowa zakończy synchronizację, lub przywróć folder z kopii zapasowej, a potem spróbuj ponownie.",
  "error.syncPasswordOutcomeUnknown":
    "Nie udało się potwierdzić, czy serwer zmienił Twoje hasło główne. Zachowaj oba hasła, dopóki synchronizacja nie potwierdzi, które obowiązuje.",
  "error.syncWebauthnUnavailable":
    "Twój serwer nie może teraz użyć klucza bezpieczeństwa do tego celu lub to konto go nie ma. Użyj zamiast tego kodu z aplikacji uwierzytelniającej.",
  "error.syncWebauthnLimit":
    "To konto ma już maksymalną liczbę kluczy bezpieczeństwa dozwoloną przez serwer. Usuń klucz, którego już nie używasz, a potem spróbuj ponownie.",
  "error.syncHandoffExpired": "Krok w przeglądarce przekroczył limit czasu. Zacznij go od nowa.",
  "error.syncHandoffFailed":
    "Zbyt wiele błędnych kodów lub nieudanych weryfikacji w przeglądarce. Zacznij ten krok od nowa.",
  "error.syncPasswordNeedsServer":
    "Zmiana hasła głównego wymaga Twojego serwera synchronizacji, a nie można się z nim połączyć. Sprawdź połączenie z internetem i spróbuj ponownie.",
  "error.syncDeviceIsCurrent": "Nie można usunąć używanego urządzenia. Użyj zamiast tego Rozłącz.",
  "error.syncChoiceExpired": "Wybór folderu wygasł. Wybierz folder ponownie.",
  "error.syncFreshLoginExpired": "Sprawdzanie hasła przekroczyło limit czasu. Wpisz hasło główne ponownie.",
  "error.syncPasswordRollback": "Serwer zaproponował starsze hasło główne. Sprawdź serwer.",
  "error.syncPasswordChanged":
    "Twoje hasło główne zostało zmienione na serwerze lub w folderze. Wpisz poniżej nowe.",
  "error.sharingNotHosted":
    "Udostępnianie wymaga serwera Mosaic. Najpierw połącz go w ustawieniach synchronizacji.",
  "error.setupCodeNeedsServer": "Telefon może dołączyć tylko do konta, które się synchronizuje.",
  "error.sharingContactUnverified":
    "Najpierw porównaj kod bezpieczeństwa tej osoby. Mosaic udostępnia tylko osobom, które zostały zweryfikowane.",
  "error.sharingKeyChanged":
    "Klucz tej osoby się zmienił lub kod się nie zgadza. Wyszukaj ją ponownie i porównaj nowy kod, zanim przejdziesz dalej.",
  "error.sharingRoleForbidden":
    "Twoja rola w tym sejfie na to nie pozwala. Zapytaj właściciela lub menedżera.",
  "error.sharingInviteExists":
    "Zaproszenie dla tej osoby i tego sejfu już czeka. Najpierw je wycofaj, jeśli chcesz wysłać nowe.",
  "error.sharingInviteStale":
    "Zaproszenie zmieniło się w międzyczasie, na przykład po zmianie klucza. Otwórz je ponownie.",
  "error.sharingInviteExpired":
    "To zaproszenie wygasło lub zostało wycofane. Poproś nadawcę o ponowne zaproszenie.",
  "error.sharingInviteUnreadable":
    "Tego zaproszenia nie można otworzyć. Może być uszkodzone lub nie być przeznaczone dla tego konta. Poproś nadawcę o ponowne zaproszenie.",
  "error.sharingInviteNotInRoster":
    "Lista członków sejfu nie pasuje do tego zaproszenia, więc Mosaic go nie otworzył. Poproś nadawcę o ponowne zaproszenie.",
  "error.sharingOwnerCannotLeave":
    "Właściciel nie może opuścić sejfu. Zamiast tego usuń pozostałe osoby.",
  "error.sharingAlreadyMember": "Ta osoba jest już członkiem sejfu.",
  "error.sharingReinviteBlocked":
    "Ta osoba została niedawno usunięta. Możesz zaprosić ją ponownie, gdy klucz sejfu się zmieni. Zsynchronizuj, a potem spróbuj ponownie.",
  "error.sharingInviteLimit":
    "Zbyt wiele zaproszeń czeka. Poczekaj na odpowiedzi lub wycofaj część z nich.",
  "error.sharingVaultKnown":
    "To urządzenie ma już ten sejf, więc Mosaic nie przyjął dla niego klucza. Zapytaj nadawcę, który sejf udostępnił.",
  "error.syncHasSharedVaults":
    "Nie można zmienić serwera, gdy na tym urządzeniu są udostępnione sejfy. Najpierw opuść udostępnione sejfy lub poproś ich właścicieli o usunięcie Cię.",
  "error.emergencyWaiting":
    "Czeka żądanie, więc teraz nie można tego zmienić. Odrzuć je lub poczekaj na nie.",
  "error.emergencyCooldown":
    "Ostatnie żądanie zostało odrzucone. Możesz poprosić ponownie po okresie oczekiwania wynoszącym jeden dzień.",
  "error.emergencyNotGranted":
    "Nie ma czego odebrać: żadne żądanie nie jest otwarte lub osoba nie przyznała Ci dostępu.",
  "error.shortcutUnavailable":
    "Inny program już używa tego skrótu lub Mosaic nie może go zarejestrować. Wybierz inną kombinację.",
  "error.shortcutUnsupported":
    "Twój system nie pozwala aplikacjom ustawiać globalnego skrótu. Otwieraj Mosaic z jego okna.",
} satisfies Record<keyof typeof en, string>;
