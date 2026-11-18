/**
 * Polish UI strings: ext. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { ext as en } from "../en/ext.ts";

export const ext = {
  "ext.manifest.name": "Mosaic",
  "ext.manifest.description":
    "Wypełniaj logowania z sejfu Mosaic. Hasło główne zostaje w aplikacji na komputerze.",
  "ext.command.fill": "Wypełnij logowanie przez Mosaic",
  "ext.command.generate": "Wygeneruj hasło w Mosaic",
  "ext.save.badgeTitle": "Zapisać logowanie?",
  "ext.loading": "Wczytywanie…",
  "ext.site.secure": "Bieżąca witryna, bezpieczne połączenie (https): {host}",
  "ext.site.insecure": "Bieżąca witryna, bez szyfrowania (http): {host}",
  "ext.site.none": "Ta karta nie ma witryny",
  "ext.nav.label": "Widoki",
  "ext.nav.logins": "Logowania i wyszukiwanie",
  "ext.nav.generator": "Generator haseł",
  "ext.footer.open": "Otwórz Mosaic",
  "ext.footer.more": "Więcej opcji",
  "ext.footer.connected": "Połączono z Mosaic",
  "ext.footer.locked": "Mosaic jest zablokowany",
  "ext.footer.offline": "Niepołączony",
  "ext.more.saveOffers": "Proponuj zapisywanie logowań",
  "ext.more.saveOffersHint":
    "Wymaga dostępu do wszystkich witryn, aby rozszerzenie mogło zauważyć przesłane logowanie. Nic nie jest zapisywane, dopóki nie potwierdzisz w aplikacji Mosaic.",
  "ext.passkeys.toggle": "Używaj Mosaic dla kluczy dostępu",
  "ext.passkeys.hint":
    "Wymaga dostępu do wszystkich witryn, aby odpowiadać na żądania kluczy dostępu. Każde potwierdzasz w aplikacji Mosaic.",
  "ext.passkeys.permissionDenied":
    "Mosaic potrzebuje dostępu do witryn, aby odpowiadać na żądania kluczy dostępu, a nie został on przyznany.",
  "ext.more.disconnect": "Rozłącz z Mosaic",
  "ext.more.disconnectTitle": "Rozłączyć tę przeglądarkę?",
  "ext.more.disconnectBody":
    "Ta przeglądarka nie będzie mogła wypełniać logowań, dopóki nie połączysz jej ponownie. Nowe połączenie zatwierdzasz w aplikacji Mosaic.",
  "ext.action.cancel": "Anuluj",
  "ext.neverSave.menu": "Witryny, w których nie proponować zapisu…",
  "ext.neverSave.title": "Nigdy nie proponuj zapisu",
  "ext.neverSave.intro":
    "Mosaic nie proponuje zapisywania logowań w tych witrynach. Usuń witrynę, aby znów być tam pytanym.",
  "ext.neverSave.empty":
    "Brak witryn. Możesz dodać witrynę przez „Nigdy dla tej witryny”, gdy Mosaic zaproponuje zapisanie logowania.",
  "ext.neverSave.remove": "Usuń",
  "ext.neverSave.removeLabel": "Usuń {origin} z listy",
  "ext.neverSave.removed":
    "Usunięto {origin}. Mosaic może tam znów proponować zapisywanie logowań.",
  "ext.neverSave.close": "Zamknij",
  "ext.connect.checking": "Łączenie z Mosaic…",
  "ext.connect.hostMissing.title": "Mosaic nie jest skonfigurowany dla tej przeglądarki",
  "ext.connect.hostMissing.body":
    "Zainstaluj aplikację Mosaic na tym komputerze, a potem uruchom przeglądarkę ponownie.",
  "ext.connect.hostMissing.dev":
    "Dla programistów: uruchom {command}, a potem uruchom przeglądarkę ponownie.",
  "ext.connect.hostForbidden.title": "Ta przeglądarka nie ma zgody na połączenie",
  "ext.connect.hostForbidden.body":
    "Mosaic komunikuje się tylko z oficjalnym rozszerzeniem Mosaic. Zainstaluj je ze sklepu swojej przeglądarki.",
  "ext.connect.desktopNotRunning.title": "Otwórz Mosaic na tym komputerze",
  "ext.connect.desktopNotRunning.body":
    "Rozszerzenie działa przez aplikację Mosaic. Uruchom aplikację, a potem sprawdź ponownie.",
  "ext.connect.check": "Sprawdź ponownie",
  "ext.connect.notPaired.title": "Połącz z Mosaic",
  "ext.connect.notPaired.body":
    "Połącz tę przeglądarkę z aplikacją Mosaic na tym komputerze. Zatwierdzasz to w aplikacji.",
  "ext.connect.start": "Połącz",
  "ext.connect.pairing.title": "Potwierdź w Mosaic",
  "ext.connect.pairing.body":
    "Sprawdź, czy Mosaic pokazuje ten sam kod, a potem kliknij tam Zezwól.",
  "ext.connect.pairing.code": "Kod",
  "ext.connect.pairing.waiting": "Czekanie na odpowiedź w aplikacji Mosaic…",
  "ext.connect.protocolMismatch.title": "Zaktualizuj Mosaic",
  "ext.connect.protocolMismatch.body":
    "Rozszerzenie i aplikacja Mosaic mają różne wersje. Zaktualizuj oba do najnowszej wersji.",
  "ext.connect.error.title": "Nie można połączyć się z Mosaic",
  "ext.connect.error.body": "Sprawdź, czy aplikacja Mosaic działa, a potem spróbuj ponownie.",
  "ext.locked.title": "Mosaic jest zablokowany",
  "ext.locked.body": "Odblokuj go w aplikacji Mosaic, aby wypełniać w {host}.",
  "ext.locked.bodyNoSite": "Odblokuj go w aplikacji Mosaic, aby używać go tutaj.",
  "ext.locked.open": "Otwórz Mosaic",
  "ext.locked.note": "Hasło główne wpisujesz wyłącznie w aplikacji Mosaic.",
  "ext.logins.forSite": "Dla {host}",
  "ext.logins.count":
    "{count, plural, one {# logowanie} few {# logowania} many {# logowań} other {# logowania}}",
  "ext.logins.sameSite": "ta sama witryna",
  "ext.logins.none": "Brak zapisanego logowania dla {host}.",
  "ext.logins.noneHint":
    "Mosaic porównuje pełny adres, więc witryny podszywające się nigdy nie dostają dopasowania.",
  "ext.logins.unsupported":
    "Mosaic nie może wypełniać na tej stronie. Przeszukaj sejf, aby skopiować logowanie.",
  "ext.logins.shortcut": "{shortcut} wypełnia pierwsze dopasowanie.",
  "ext.logins.frameNote":
    "Mosaic wypełnia tylko w {host}, nigdy wewnątrz osadzonych ramek z innych witryn.",
  "ext.item.untitled": "Bez tytułu",
  "ext.item.noUsername": "Brak nazwy użytkownika",
  "ext.item.twoFactor": "2FA",
  "ext.item.fill": "Wypełnij",
  "ext.item.fillLabel": "Wypełnij {name}",
  "ext.item.copyMenu": "Opcje kopiowania dla {name}",
  "ext.item.copyUsername": "Kopiuj nazwę użytkownika",
  "ext.item.copyPassword": "Kopiuj hasło",
  "ext.item.copyCode": "Kopiuj kod",
  "ext.item.blocked":
    "Zablokowano: ta strona nie jest szyfrowana, a to logowanie na to nie pozwala.",
  "ext.search.label": "Inne elementy",
  "ext.search.placeholder": "Przeszukaj sejf",
  "ext.search.note":
    "Wyniki zostają w tym oknie. Strona otrzymuje zawsze tylko jedno logowanie, które wypełniasz.",
  "ext.search.none": "Nic nie znaleziono.",
  "ext.search.copy": "Kopiuj",
  "ext.search.copyOnly": "Tylko kopiowanie: niezapisane dla tej witryny",
  "ext.shortcut.copyUser": "{platform, select, mac {⌘C} other {Ctrl+C}}",
  "ext.shortcut.copyPassword": "{platform, select, mac {⇧⌘C} other {Ctrl+Shift+C}}",
  "ext.shortcut.copyCode": "{platform, select, mac {⌥⌘C} other {Ctrl+Alt+C}}",
  "ext.search.results":
    "{count, plural, one {# wynik} few {# wyniki} many {# wyników} other {# wyniku}}",
  "ext.frame.cross":
    "Aktywne pole znajduje się w ramce z innej witryny. Mosaic tam nie wypełnia.",
  "ext.frameConfirm.title": "Wypełnić wewnątrz osadzonej ramki?",
  "ext.frameConfirm.body":
    "Pole znajduje się w ramce w {host}, a nie na samej stronie. Wypełniaj tutaj tylko wtedy, gdy ufasz tej ramce.",
  "ext.frameConfirm.confirm": "Wypełnij w ramce",
  "ext.warning.idn.title": "Sprawdź adres",
  "ext.warning.idn.body":
    "Ten adres zawiera znaki specjalne i może wyglądać jak inna witryna. Sprawdź pasek adresu, zanim się zalogujesz.",
  "ext.warning.lookalike.body":
    "Ta witryna to nie {saved}. Wygląda podobnie, co jest częstą sztuczką phishingu. Zamknij tę kartę lub wpisz {saved} samodzielnie.",
  "ext.warning.youAreOn": "Jesteś na",
  "ext.warning.writtenAs": "Zapisane jako",
  "ext.warning.savedFor": "Zapisane dla",
  "ext.warning.insecure.title": "Ta strona nie jest szyfrowana",
  "ext.warning.insecure.body":
    "Strona używa http. Inni w sieci mogliby odczytać Twoje hasło. Mosaic wypełnia tutaj tylko logowania, dla których na to zezwolono.",
  "ext.otp.title": "Wypełnij kod jednorazowy",
  "ext.otp.next": "Wypełniono hasło dla {name}. Dalej: kod dwuskładnikowy.",
  "ext.otp.for": "Kod dla {name}",
  "ext.otp.fill": "Wypełnij kod",
  "ext.otp.copy": "Kopiuj kod",
  "ext.otp.note":
    "Kod trafia z Mosaic prosto do pola. To okno nigdy go nie pokazuje.",
  "ext.save.titleNew": "Zapisać to logowanie dla {host}?",
  "ext.save.titleUpdate": "Zaktualizować zapisane logowanie dla {host}?",
  "ext.save.generated": "Hasło wygenerowane przez Mosaic dla {host}.",
  "ext.save.updates": "Aktualizuje „{title}”.",
  "ext.save.username": "Nazwa użytkownika",
  "ext.save.noUsername": "Brak",
  "ext.save.password": "Hasło",
  "ext.save.passwordHidden": "Ukryte. Zobaczysz je w aplikacji Mosaic.",
  "ext.save.insecure": "Ta strona nie jest szyfrowana (http).",
  "ext.save.action": "Zapisz w Mosaic…",
  "ext.save.notNow": "Nie teraz",
  "ext.save.never": "Nigdy dla tej witryny",
  "ext.save.waiting": "Potwierdź w aplikacji Mosaic. Nic nie zostanie zapisane, dopóki tego nie zrobisz.",
  "ext.save.saved": "Zapisano w Mosaic.",
  "ext.save.updated": "Zaktualizowano w Mosaic.",
  "ext.save.cancelled": "Nie zapisano.",
  "ext.save.dismissed": "Odrzucono. Hasło zostało zapomniane.",
  "ext.save.neverDone": "Mosaic nie będzie już proponować zapisu w {host}.",
  "ext.save.permissionAsk":
    "Mosaic prosi przeglądarkę o dostęp do witryn. Zezwól, aby to włączyć.",
  "ext.generator.title": "Wygeneruj hasło dla {host}",
  "ext.generator.titleGeneric": "Wygeneruj hasło",
  "ext.generator.value": "Wygenerowane hasło",
  "ext.generator.regenerate": "Wygeneruj inne hasło",
  "ext.generator.type": "Rodzaj hasła",
  "ext.generator.random": "Losowe",
  "ext.generator.passphrase": "Fraza hasłowa",
  "ext.generator.include": "Uwzględnij",
  "ext.generator.rules.page": "Zgodne z zasadami haseł tej strony",
  "ext.generator.rules.siteQuirks": "Zgodne ze znanymi zasadami tej witryny",
  "ext.generator.fill": "Wypełnij",
  "ext.generator.fillHint":
    "Aby wypełnić, kliknij pole nowego hasła na stronie, a potem otwórz to okno ponownie.",
  "ext.generator.copy": "Kopiuj",
  "ext.generator.expired": "Tego hasła już nie przechowywano, więc wygenerowano nowe.",
  "ext.generator.filled": "Wypełniono. Mosaic zaproponuje jego zapisanie.",
  "ext.copied":
    "Skopiowano. Zostanie wyczyszczone za {seconds, plural, one {# sekundę} few {# sekundy} many {# sekund} other {# sekundy}}.",
  "ext.copiedNoClear": "Skopiowano.",
  "ext.filled": "Wypełniono.",
  "ext.filledCode": "Kod wypełniono.",
  "ext.error.invalidRequest":
    "Mosaic nie zrozumiał tego żądania. Zaktualizuj rozszerzenie i aplikację Mosaic.",
  "ext.error.notPaired": "Ta przeglądarka nie jest połączona z Mosaic. Połącz ją ponownie.",
  "ext.error.protocolVersion":
    "Rozszerzenie i aplikacja Mosaic mają różne wersje. Zaktualizuj oba.",
  "ext.error.busy": "Mosaic jest zajęty innym żądaniem. Spróbuj ponownie za chwilę.",
  "ext.error.locked": "Mosaic jest zablokowany. Odblokuj go w aplikacji Mosaic.",
  "ext.error.rateLimited":
    "Zbyt wiele żądań w krótkim czasie. Chwilę poczekaj, a potem spróbuj ponownie.",
  "ext.error.originMismatch":
    "To logowanie nie jest zapisane dla tej witryny, więc Mosaic nie wypełni go tutaj.",
  "ext.error.insecureOrigin":
    "Ta strona nie jest szyfrowana, a logowanie nie pozwala na wypełnianie na niej. Nic nie wypełniono.",
  "ext.error.crossOriginFrame":
    "To pole znajduje się w ramce z innej witryny. Mosaic tam nie wypełnia.",
  "ext.error.itemNotFound": "Ten element już nie istnieje. Mógł zostać przeniesiony lub usunięty.",
  "ext.error.vaultUnavailable": "Ten sejf jest zamknięty. Najpierw otwórz go w aplikacji Mosaic.",
  "ext.error.noOtp": "Ten element nie ma kodu jednorazowego.",
  "ext.error.otpNotFillable":
    "Ten kod rośnie z każdym użyciem (HOTP), więc nie można go stąd wypełnić ani skopiować. Użyj go w aplikacji Mosaic.",
  "ext.error.fieldEmpty": "To pole jest puste, więc nie ma czego wypełniać ani kopiować.",
  "ext.error.clipboardUnavailable":
    "Mosaic nie mógł użyć schowka. Spróbuj ponownie lub skopiuj w aplikacji Mosaic.",
  "ext.error.cancelled": "Anulowano w aplikacji Mosaic. Spróbuj ponownie, gdy będziesz gotowy.",
  "ext.error.passkeyNotFound": "Mosaic nie ma klucza dostępu dla tej witryny.",
  "ext.error.passkeyExcluded": "Masz już w Mosaic klucz dostępu dla tego konta.",
  "ext.error.rpIdMismatch":
    "Ta witryna nie może użyć klucza dostępu dla tej domeny, więc Mosaic nie odpowiedział.",
  "ext.error.passkeyUnsupported":
    "Mosaic nie może utworzyć klucza dostępu z opcjami, o które poprosiła ta witryna.",
  "ext.error.pairingDenied":
    "Połączenie odrzucono w aplikacji Mosaic. Wybierz Połącz, aby spróbować ponownie, a potem wybierz Zezwól.",
  "ext.error.pairingExpired":
    "Żądanie połączenia wygasło. Wybierz Połącz, aby zacząć od nowa.",
  "ext.error.pairingConflict":
    "Otwarte było inne żądanie połączenia. Wybierz Połącz, aby spróbować ponownie.",
  "ext.error.pairingBlocked":
    "Odrzucono zbyt wiele połączeń. Odczekaj dziesięć minut, a potem spróbuj ponownie.",
  "ext.error.pairingLimit":
    "Ten komputer ma zbyt wiele połączonych przeglądarek. Rozłącz jedną w ustawieniach Mosaic.",
  "ext.error.internal":
    "Mosaic napotkał nieoczekiwany problem. Spróbuj ponownie, a jeśli się powtórzy, uruchom aplikację Mosaic ponownie.",
  "ext.error.desktopNotRunning":
    "Mosaic nie działa na tym komputerze. Otwórz go, a potem spróbuj ponownie.",
  "ext.error.desktopNotAuthentic":
    "Aplikacja, która odpowiedziała, nie jest aplikacją Mosaic, z którą ta przeglądarka jest połączona. Nic nie wysłano.",
  "ext.error.timeout":
    "Mosaic nie odpowiedział na czas. Jeśli w aplikacji Mosaic jest otwarte okno dialogowe, odpowiedz w nim.",
  "ext.error.frameTooLarge": "To zbyt wiele danych do wysłania do Mosaic.",
  "ext.error.hostMissing":
    "Pomocnik Mosaic nie jest zainstalowany dla tej przeglądarki. Zainstaluj aplikację Mosaic, a potem uruchom przeglądarkę ponownie.",
  "ext.error.hostForbidden": "Ta przeglądarka lub rozszerzenie nie ma zgody na komunikację z Mosaic.",
  "ext.error.hostExited": "Połączenie z Mosaic zostało zamknięte. Spróbuj ponownie.",
  "ext.error.unsupportedPage": "Mosaic nie może wypełniać na tym rodzaju strony.",
  "ext.error.noFields":
    "Nie znaleziono tu pól logowania. Kliknij pole, a potem spróbuj ponownie.",
  "ext.error.noMatch": "Mosaic nie ma zapisanego logowania dla tej witryny.",
  "ext.error.frameNeedsConfirmation":
    "Pole znajduje się wewnątrz ramki. Otwórz okno Mosaic i potwierdź ramkę, aby je wypełnić.",
  "ext.error.frameOpaque": "Pole znajduje się w ramce z piaskownicy. Mosaic tam nie wypełnia.",
  "ext.error.originChanged":
    "Strona zmieniła się w trakcie wypełniania przez Mosaic. Nic nie wypełniono. Spróbuj ponownie.",
  "ext.error.pageHidden": "Strona nie jest widoczna, więc Mosaic nie wypełnił.",
  "ext.error.permissionDenied":
    "Mosaic potrzebuje dostępu do witryn, aby zauważać logowania, a nie został on przyznany.",
  "ext.error.generatedExpired": "Mosaic już nie przechowuje tego hasła. Wygeneruj nowe.",
  "ext.rotation.card.title": "Zmiana hasła",
  "ext.rotation.card.for": "Dla {name}",
  "ext.rotation.card.started": "Rozpoczęto {when}",
  "ext.rotation.card.delivered": "Nowe hasło zostało już raz wypełnione.",
  "ext.rotation.card.note":
    "Nowe hasło jest pokazywane tylko w aplikacji Mosaic. Mosaic nigdy nie wysyła formularza za Ciebie.",
  "ext.rotation.card.list": "Trwające zmiany haseł",
  "ext.rotation.change": "Zmień hasło…",
  "ext.rotation.changeLabel": "Zmień hasło… dla {name}",
  "ext.rotation.openPage": "Otwórz stronę zmiany",
  "ext.rotation.fillChange": "Wypełnij formularz zmiany",
  "ext.rotation.fillPending": "Wypełnij nowe hasło, aby się zalogować",
  "ext.rotation.worked": "Udało się",
  "ext.rotation.failed": "Nie udało się",
  "ext.rotation.fresh": "Zacznij od nowa z nowym hasłem",
  "ext.rotation.waiting": "Czekanie na Ciebie w aplikacji Mosaic…",
  "ext.rotation.started": "Rozpoczęto. W aplikacji Mosaic wybierz Użyj tego hasła.",
  "ext.rotation.startCancelled": "Nic nie rozpoczęto.",
  "ext.rotation.filled": "Formularz zmiany wypełniono. Sprawdź go, a potem wyślij samodzielnie.",
  "ext.rotation.opened": "Otwarto stronę zmiany.",
  "ext.rotation.confirmed": "Gotowe. Nowe hasło jest teraz zapisane w Mosaic.",
  "ext.rotation.keptOld": "Mosaic zachował stare hasło.",
  "ext.rotation.resultCancelled": "Nic się nie zmieniło. Zmiana nadal jest otwarta.",
  "ext.rotation.offer.title": "Powiązany adres",
  "ext.rotation.offer.body":
    "Ta strona jest w {host}, który nie jest zapisany dla {title}. Dodać go, aby Mosaic mógł tam wypełniać?",
  "ext.rotation.offer.add": "Dodaj {host}",
  "ext.rotation.offer.added": "Dodano. Zmiana jest teraz tu widoczna.",
  "ext.rotation.offer.cancelled": "Nie dodano.",
  "ext.error.rotationNotFound":
    "Ta zmiana hasła nie jest już otwarta. Rozpocznij nową z poziomu logowania.",
  "ext.error.originNotRelated":
    "Ten adres nie należy do tej samej witryny co to logowanie, więc Mosaic go nie dodał.",
  "ext.error.rotationLimit":
    "To logowanie ma już cztery niedokończone zmiany. Najpierw zakończ lub anuluj jedną w aplikacji Mosaic.",
  "ext.error.changeFormNotFound":
    "Mosaic nie znalazł tu formularza zmiany hasła. Otwórz stronę hasła swojego konta, a potem spróbuj ponownie.",
  "ext.error.changeFormAmbiguous":
    "Mosaic nie mógł ustalić, które pole jest które. Skopiuj nowe hasło z Mosaic i wklej je samodzielnie.",
} satisfies Record<keyof typeof en, string>;
