/**
 * Polish UI strings: rotation. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { rotation as en } from "../en/rotation.ts";

export const rotation = {
  "rotation.section.title": "Hasło i klucze",
  "rotation.section.body":
    "Zmień hasło główne, uzyskaj nowy Klucz tajny lub przenieś całe konto na nowe klucze. Każda zmiana pokazuje, co się stanie, zanim cokolwiek zostanie ruszone.",
  "rotation.action.password": "Zmień hasło główne",
  "rotation.action.password.body":
    "Wybierz nowe hasło główne. Klucz tajny i sejfy pozostają bez zmian.",
  "rotation.action.password.button": "Zmień…",
  "rotation.action.secretKey": "Wymień Klucz tajny",
  "rotation.action.secretKey.body":
    "Uzyskaj nowy Klucz tajny i nowy Zestaw awaryjny. Użyj tego, jeśli zestaw zginął lub ktoś mógł go zobaczyć.",
  "rotation.action.secretKey.button": "Wymień…",
  "rotation.action.profile": "Zwiększ ochronę",
  "rotation.action.profile.body":
    "Utrudnij odgadywanie hasła głównego, używając więcej pamięci przy każdej próbie. Odblokowanie trwa nieco dłużej.",
  "rotation.action.profile.button": "Zwiększ…",
  "rotation.action.rekey": "Wymień klucze konta",
  "rotation.action.rekey.body":
    "Odnów wszystkie klucze konta: nowy Klucz tajny, nowe klucze sejfów i nowy kod bezpieczeństwa. Użyj tego, jeśli ktoś mógł skopiować dane sejfu razem z Twoim hasłem i Kluczem tajnym.",
  "rotation.action.rekey.button": "Wymień klucze…",
  "rotation.action.rekey.needsSync":
    "Wymiana kluczy odnawia kopię na serwerze lub w folderze synchronizacji, więc wymaga synchronizacji. Najpierw skonfiguruj synchronizację.",
  "rotation.title.password": "Zmień hasło główne",
  "rotation.title.secretKey": "Wymień Klucz tajny",
  "rotation.title.profile": "Zwiększ ochronę",
  "rotation.title.rekey": "Wymień klucze konta",
  "rotation.step": "Krok {current} z {total}",
  "rotation.back": "Wstecz",
  "rotation.stop.title": "Przerwać tę zmianę?",
  "rotation.stop.body": "Właśnie zapisany Zestaw awaryjny nie zadziała. Zachowaj stary zestaw.",
  "rotation.stop.confirm": "Przerwij zmianę",
  "rotation.stop.keep": "Kontynuuj",
  "rotation.next": "Dalej",
  "rotation.close": "Zamknij",
  "rotation.review.title.password": "Zanim zmienisz hasło",
  "rotation.review.title.secretKey": "Zanim wymienisz Klucz tajny",
  "rotation.review.title.profile": "Zanim zwiększysz ochronę",
  "rotation.review.title.rekey": "Czy coś zostało naruszone?",
  "rotation.review.changes": "Co się zmienia",
  "rotation.review.stays": "Co pozostaje bez zmian",
  "rotation.review.password.changes":
    "Twoje hasło główne. Szybkie odblokowanie na tym urządzeniu zostaje wyłączone, a potem możesz je skonfigurować ponownie.",
  "rotation.review.password.stays":
    "Twój Klucz tajny, sejfy i elementy, kod bezpieczeństwa oraz kontakty awaryjne.",
  "rotation.review.password.devices":
    "Pozostałe urządzenia zostają wylogowane z synchronizacji. Na każdym z nich wpisz raz nowe hasło główne, a synchronizacja będzie kontynuowana.",
  "rotation.review.secretKey.changes":
    "Twój Klucz tajny i Zestaw awaryjny. Szybkie odblokowanie na tym urządzeniu zostaje wyłączone, a potem możesz je skonfigurować ponownie.",
  "rotation.review.secretKey.stays":
    "Twoje hasło główne, sejfy i elementy, kod bezpieczeństwa oraz kontakty awaryjne. Zarejestrowane klucze bezpieczeństwa nadal działają.",
  "rotation.review.secretKey.devices":
    "Pozostałe urządzenia zostają wylogowane z synchronizacji. Na każdym z nich wpisz raz hasło i nowy Klucz tajny z nowego zestawu.",
  "rotation.review.secretKey.limit":
    "To uniemożliwia logowanie starym Kluczem tajnym. Każdy, kto już skopiował dane Twojego sejfu wraz ze starym hasłem i Kluczem tajnym, nadal może je odczytać. Wymień klucze konta, aby to powstrzymać.",
  "rotation.review.secretKey.compromised": "Coś zostało naruszone: wymień zamiast tego klucze konta",
  "rotation.review.profile.changes":
    "Ile pamięci potrzebuje każda próba odblokowania, dla Ciebie i dla każdego, kto próbuje zgadywać. Szybkie odblokowanie na tym urządzeniu zostaje wyłączone, a potem możesz je skonfigurować ponownie.",
  "rotation.review.profile.stays":
    "Twoje hasło główne, Klucz tajny, sejfy i kod bezpieczeństwa. Zarejestrowane klucze bezpieczeństwa nadal działają.",
  "rotation.review.profile.devices":
    "Pozostałe urządzenia zostają wylogowane z synchronizacji. Na każdym z nich wpisz raz hasło główne, a synchronizacja będzie kontynuowana. Wolniejsze urządzenia będą odblokowywać się nieco dłużej.",
  "rotation.review.rekey.intro":
    "Wymiana kluczy konta jest przeznaczona na wypadek, gdy ktoś mógł zdobyć dane Twojego sejfu oraz hasło i Klucz tajny. Jeśli utracono tylko urządzenie, wystarczy wymienić Klucz tajny lub zmienić hasło.",
  "rotation.review.rekey.yes": "Tak, wymień klucze mojego konta",
  "rotation.review.rekey.no": "Nie, wymień zamiast tego tylko Klucz tajny",
  "rotation.review.rekey.limit":
    "Wymiana kluczy nie cofa tego, co ktoś już skopiował. Stare kopie na serwerach i w kopiach zapasowych pozostają zaszyfrowane starymi kluczami, a stare klucze nadal je otwierają.",
  "rotation.review.rekey.title": "Co robi wymiana kluczy konta",
  "rotation.review.rekey.vaults":
    "{count, plural, one {# sejf używany tylko przez Ciebie dostaje nowy klucz.} few {# sejfy używane tylko przez Ciebie dostają nowe klucze.} many {# sejfów używanych tylko przez Ciebie dostaje nowe klucze.} other {# sejfu używanego tylko przez Ciebie dostaje nowe klucze.}}",
  "rotation.review.rekey.keys":
    "Nowy Klucz tajny, nowy Zestaw awaryjny, nowy kod bezpieczeństwa i nowy klucz konta.",
  "rotation.review.rekey.devices":
    "Pozostałe urządzenia zostają wylogowane i nie mogą synchronizować, dopóki nie wpiszesz na każdym nowego Klucza tajnego i nowego hasła głównego.",
  "rotation.review.rekey.shared":
    "{count, plural, one {Udostępniasz # sejf.} few {Udostępniasz # sejfy.} many {Udostępniasz # sejfów.} other {Udostępniasz # sejfu.}} Każdy z nich dostaje nowy klucz.",
  "rotation.review.rekey.sharedRow":
    "{name}, {count, plural, one {# członek} few {# członków} many {# członków} other {# członka}}",
  "rotation.review.rekey.sharedMembers":
    "Poproś każdą osobę, której udostępniasz, o porównanie Twojego nowego kodu bezpieczeństwa. Dopóki tego nie zrobi, nie dostanie od Ciebie nowych kluczy i nie otworzy odnowionego sejfu.",
  "rotation.review.rekey.memberVaults":
    "{count, plural, one {# sejf należy do kogoś innego.} few {# sejfy należą do innych osób.} many {# sejfów należy do innych osób.} other {# sejfu należy do innych osób.}} Ich właściciele zostaną poproszeni o odnowienie. Dopóki tego nie zrobią, Twoje stare klucze nadal mogą je odczytać.",
  "rotation.review.rekey.grants":
    "{count, plural, one {# osoba ma dostęp awaryjny do Twoich sejfów.} few {# osoby mają dostęp awaryjny do Twoich sejfów.} many {# osób ma dostęp awaryjny do Twoich sejfów.} other {# osoby ma dostęp awaryjny do Twoich sejfów.}} Ich dostęp zostanie odnowiony dla nowych kluczy. Każda z nich musi też porównać Twój nowy kod bezpieczeństwa.",
  "rotation.review.rekey.quick":
    "Szybkie odblokowanie na tym urządzeniu zostaje wyłączone. Potem możesz je skonfigurować ponownie.",
  "rotation.review.rekey.hardwareKeys":
    "{count, plural, one {# zarejestrowany klucz bezpieczeństwa do odblokowywania zostaje wyłączony.} few {# zarejestrowane klucze bezpieczeństwa do odblokowywania zostają wyłączone.} many {# zarejestrowanych kluczy bezpieczeństwa do odblokowywania zostaje wyłączonych.} other {# zarejestrowanego klucza bezpieczeństwa do odblokowywania zostaje wyłączone.}} Potem możesz zarejestrować je ponownie.",
  "rotation.review.rekey.hardwareVaults":
    "{count, plural, one {# sejf chroniony sprzętowo nie jest dotknięty.} few {# sejfy chronione sprzętowo nie są dotknięte.} many {# sejfów chronionych sprzętowo nie jest dotkniętych.} other {# sejfu chronionego sprzętowo nie jest dotknięte.}}",
  "rotation.review.rekey.attachments":
    "{count, plural, one {# załącznika nie ma na tym urządzeniu.} few {# załączników nie ma na tym urządzeniu.} many {# załączników nie ma na tym urządzeniu.} other {# załącznika nie ma na tym urządzeniu.}} Zostaną najpierw pobrane, aby można je było ponownie zabezpieczyć. Te, do których nie można się dostać, zachowują stary klucz.",
  "rotation.review.rekey.folder":
    "Twój folder synchronizacji dostaje nowy plik konta. Inne urządzenia znajdą tam informację, że klucze się zmieniły.",
  "rotation.review.rekey.unavailable":
    "Wymiana kluczy konta nie jest dostępna: synchronizacja nie jest skonfigurowana.",
  "rotation.review.loading": "Sprawdzanie, czego dotyczy wymiana kluczy…",
  "rotation.review.rekey.needsClean":
    "Wymiana kluczy wymaga aktualnej synchronizacji bez ostrzeżeń. Zsynchronizuj teraz, sprawdź ewentualne ostrzeżenia, a potem spróbuj ponownie.",
  "rotation.credentials.title": "Potwierdź, że to Ty",
  "rotation.credentials.password.title": "Twoje hasła główne",
  "rotation.credentials.current": "Obecne hasło główne",
  "rotation.credentials.new": "Nowe hasło główne",
  "rotation.credentials.confirm": "Wpisz nowe hasło główne ponownie",
  "rotation.credentials.mismatch": "Oba hasła się różnią.",
  "rotation.credentials.same": "Nowe hasło główne musi różnić się od obecnego.",
  "rotation.credentials.hint":
    "Długa fraza hasłowa jest lepsza niż krótkie skomplikowane hasło. Nie można go odzyskać: Mosaic go nie zna.",
  "rotation.credentials.code": "6-cyfrowy kod z aplikacji uwierzytelniającej",
  "rotation.credentials.profile": "Poziom ochrony",
  "rotation.credentials.profile.standard": "Standardowy: 256 MB pamięci na próbę",
  "rotation.credentials.profile.strong": "Silny: 512 MB pamięci na próbę",
  "rotation.credentials.profile.max": "Maksymalny: 1 GB pamięci na próbę",
  "rotation.credentials.profile.slow":
    "Odblokowanie na tym poziomie zajmuje temu urządzeniu ponad dwie sekundy. Inne, starsze urządzenia będą jeszcze wolniejsze.",
  "rotation.credentials.profile.force": "Mimo to zwiększ",
  "rotation.kit.title": "Zapisz nowy Zestaw awaryjny",
  "rotation.kit.body":
    "Twój nowy Klucz tajny jest w tym zestawie. Najpierw go zapisz: Mosaic zmienia konto dopiero po potwierdzeniu, że zestaw jest zapisany, ponieważ bez niego nie zalogujesz się na nowym urządzeniu.",
  "rotation.kit.save": "Zapisz nowy Zestaw awaryjny",
  "rotation.kit.saved": "Zapisano. Przechowuj go w bezpiecznym miejscu, osobno od urządzeń.",
  "rotation.kit.cancelled": "Jeszcze nic nie zapisano.",
  "rotation.kit.verify": "Wpisz ostatnią grupę nowego Klucza tajnego z zapisanego zestawu.",
  "rotation.kit.confirm": "Potwierdź zestaw",
  "rotation.kit.mistyped": "To nie jest koniec nowego Klucza tajnego. Sprawdź zapisany zestaw.",
  "rotation.kit.verified": "Zestaw potwierdzony.",
  "rotation.confirm.title.password": "Zmień hasło główne",
  "rotation.confirm.title.secretKey": "Wymień Klucz tajny",
  "rotation.confirm.title.profile": "Zwiększ ochronę",
  "rotation.confirm.title.rekey": "Wymień klucze konta",
  "rotation.confirm.body.rekey":
    "To wylogowuje pozostałe urządzenia i odnawia wszystkie klucze. Pozostaw Mosaic otwarty do zakończenia.",
  "rotation.confirm.body.secretKey":
    "To wylogowuje pozostałe urządzenia z synchronizacji. Pozostaw Mosaic otwarty do zakończenia.",
  "rotation.confirm.verb.password": "Zmień hasło",
  "rotation.confirm.verb.secretKey": "Wymień Klucz tajny",
  "rotation.confirm.verb.profile": "Zwiększ ochronę",
  "rotation.confirm.verb.rekey": "Wymień klucze konta",
  "rotation.run.title": "Odnawianie kluczy…",
  "rotation.run.keepOpen":
    "Pozostaw Mosaic otwarty. Przy wielu załącznikach może to potrwać kilka minut.",
  "rotation.progress.label": "Postęp",
  "rotation.progress.count": "{done} z {total}",
  "rotation.progress.done": "gotowe",
  "rotation.phase.prepared": "Przygotowywanie",
  "rotation.phase.backendStarted": "Prośba do konta o przełączenie na nowe klucze",
  "rotation.phase.backendAccepted": "Konto przyjęło nowe klucze",
  "rotation.phase.folderWritten": "Folder synchronizacji ma nowy plik konta",
  "rotation.phase.localCommitted": "To urządzenie używa teraz nowych kluczy",
  "rotation.phase.attachments": "Ponowne zabezpieczanie załączników",
  "rotation.phase.uploading": "Wysyłanie zabezpieczonych danych",
  "rotation.phase.done": "Zakończono",
  "rotation.done.title.password": "Hasło główne zmienione",
  "rotation.done.title.secretKey": "Klucz tajny wymieniony",
  "rotation.done.title.profile": "Ochrona zwiększona",
  "rotation.done.title.rekey": "Klucze konta wymienione",
  "rotation.done.body.password":
    "Od teraz używaj nowego hasła głównego. Pozostałe urządzenia zapytają o nie raz.",
  "rotation.done.body.secretKey":
    "Przechowuj nowy Zestaw awaryjny w bezpiecznym miejscu, a stary usuń. Pozostałe urządzenia zapytają raz o nowy Klucz tajny.",
  "rotation.done.body.profile":
    "Odblokowanie wymaga teraz więcej pamięci. Pozostałe urządzenia zapytają raz o hasło.",
  "rotation.done.body.rekey":
    "Przechowuj nowy Zestaw awaryjny w bezpiecznym miejscu, a stary usuń. Pozostałe urządzenia zapytają raz o nowy Klucz tajny i nowe hasło główne.",
  "rotation.done.members":
    "Poproś wszystkich, z którymi udostępniasz sejf, o porównanie Twojego nowego kodu bezpieczeństwa. Dopóki tego nie zrobią, nie dostaną od Ciebie nowych kluczy.",
  "rotation.done.quick": "Skonfiguruj szybkie odblokowanie ponownie",
  "rotation.done.quick.body": "Szybkie odblokowanie zostało wyłączone.",
  "rotation.done.hardware": "Zarejestruj klucze bezpieczeństwa ponownie",
  "rotation.done.hardware.body":
    "{count, plural, one {# klucz bezpieczeństwa został wyłączony dla odblokowywania.} few {# klucze bezpieczeństwa zostały wyłączone dla odblokowywania.} many {# kluczy bezpieczeństwa zostało wyłączonych dla odblokowywania.} other {# klucza bezpieczeństwa zostało wyłączone dla odblokowywania.}}",
  "rotation.done.unreachable.note":
    "Tych plików nie udało się odczytać i nie zostały ponownie zaszyfrowane. Zachowują stary klucz pliku, dopóki ich nie usuniesz lub nie dodasz ponownie, więc każdy, kto ma Twoje stare klucze i kopię pliku, nadal mógłby go otworzyć.",
  "rotation.done.unreachable.file": "{item}: {file}",
  "rotation.done.unreachable.unknownItem": "Nieznany element",
  "rotation.done.unreachable.unknownFile": "nieznany plik",
  "rotation.done.unreachable.more":
    "{count, plural, one {i jeszcze # plik} few {i jeszcze # pliki} many {i jeszcze # plików} other {i jeszcze # pliku}}",
  "rotation.done.unfinished":
    "Ostatnia część będzie kontynuowana później. Jeśli Mosaic zamknie się przed jej zakończeniem, następnym razem wybierz Kontynuuj wymianę kluczy.",
  "rotation.resume.title": "Wymiana kluczy niedokończona",
  "rotation.resume.body.password":
    "Zmiana kluczy została przerwana. Nic nie zostało utracone. Wybierz Dalej, aby ponownie wpisać hasła, lub odrzuć zmianę.",
  "rotation.resume.body.secretKey":
    "Wymiana Klucza tajnego została przerwana. Nic nie zostało utracone. Kontynuuj, aby ją dokończyć.",
  "rotation.resume.body.rekey":
    "Wymiana kluczy konta została przerwana. Nic nie zostało utracone. Kontynuuj, aby ją dokończyć.",
  "rotation.resume.body.profile":
    "Zwiększanie ochrony zostało przerwane. Nic nie zostało utracone. Kontynuuj, aby je dokończyć.",
  "rotation.resume.continue": "Kontynuuj wymianę kluczy",
  "rotation.resume.discard": "Odrzuć…",
  "rotation.resume.running": "Kontynuowanie…",
  "rotation.resume.progress": "Gotowe: {done} z {total}. Pozostaw Mosaic otwarty do zakończenia.",
  "rotation.resume.dialog.title": "Kontynuuj wymianę kluczy",
  "rotation.resume.dialog.password": "Obecne hasło główne",
  "rotation.resume.dialog.group": "Ostatnia grupa nowego Klucza tajnego",
  "rotation.resume.dialog.body":
    "Klucze Twojego konta mogły już się zmienić. Aby upewnić się, że to Ty, wpisz hasło główne i ostatnią grupę nowego Klucza tajnego z nowego Zestawu awaryjnego.",
  "rotation.resume.dialog.submit": "Dalej",
  "rotation.resume.discard.title": "Odrzucić tę zmianę kluczy?",
  "rotation.resume.discard.body":
    "Nowy Klucz tajny i nowy Zestaw awaryjny z tej próby zostaną usunięte. Konto pozostaje bez zmian.",
  "rotation.resume.discard.confirm": "Odrzuć zmianę kluczy",
  "rotation.resume.abandon": "Zrezygnuj z reszty…",
  "rotation.resume.stalled":
    "Kontynuowanie nie zakończyło się. Jeśli wciąż się nie udaje, możesz tu poprzestać.",
  "rotation.abandon.title": "Przerwać ponowne szyfrowanie plików?",
  "rotation.abandon.body":
    "Twoje nowe klucze są już używane. Mosaic przerywa ponowne szyfrowanie załączników, które nadal czekają. Zachowują stary klucz pliku, więc każdy, kto ma Twoje stare klucze i kopię pliku, nadal mógłby go otworzyć.",
  "rotation.abandon.stays":
    "Wszystko, co zostało już ponownie zaszyfrowane, pozostaje bez zmian. Usuń lub dodaj ponownie czekające pliki później, aby zabezpieczyć je nowym kluczem.",
  "rotation.abandon.confirm": "Przerwij ponowne szyfrowanie",
  "rotation.abandon.keep": "Próbuj dalej",
  "rotation.marker.title": "Niepotwierdzona informacja o wymianie kluczy",
  "rotation.marker.hint":
    "Jeśli nie odnawiano kluczy na innym urządzeniu, ta informacja może nie być prawdziwa. Możesz ją zignorować.",
  "rotation.marker.open": "Zignoruj tę informację…",
  "rotation.marker.open.label": "Zignoruj tę informację: {name}",
  "rotation.marker.confirm":
    "Mosaic znalazł informację o wymianie kluczy, której nie mógł potwierdzić. Jeśli nie wymieniano kluczy konta na innym urządzeniu, możesz ją zignorować.",
  "rotation.marker.ignore": "Zignoruj informację",
  "rotation.marker.keep": "Zachowaj informację",
  "rotation.ignored.title": "Zignorowane informacje o wymianie kluczy",
  "rotation.ignored.body":
    "Informacje, które zdecydowano się zignorować. Przywróć jedną, jeśli klucze konta rzeczywiście wymieniono na innym urządzeniu.",
  "rotation.ignored.row": "Informacja o wymianie kluczy z Twojego folderu synchronizacji",
  "rotation.ignored.restore": "Przywróć",
  "rotation.ignored.restore.label": "Przywróć informację {name}",
  "rotation.resume.done": "Wymiana kluczy zakończona.",
  "rotation.recovery.title": "Klucze Twojego konta zostały odnowione na innym urządzeniu",
  "rotation.recovery.body":
    "Jedno z Twoich urządzeń odnowiło klucze tego konta. Aby dalej synchronizować, wpisz hasło główne i Klucz tajny z nowego Zestawu awaryjnego.",
  "rotation.recovery.password": "Hasło główne (nowe, jeśli zmieniono je tam)",
  "rotation.recovery.secretKey": "Klucz tajny z nowego Zestawu awaryjnego",
  "rotation.recovery.submit": "Użyj nowych kluczy",
  "rotation.recovery.warning":
    "Klucze nie zostały odnowione przez Ciebie? Wtedy mógł to zrobić ktoś, kto ma Twoje stare hasło i Klucz tajny. Nic tu nie wpisuj; najpierw sprawdź inne urządzenia.",
  "rotation.recovery.notice": "Synchronizacja jest wstrzymana, dopóki nie zrobisz tego na tym urządzeniu.",
  "rotation.recovery.open": "Użyj nowych kluczy…",
  "rotation.recovery.keyHint":
    "Jeśli klucze odnowiono na innym urządzeniu, wpisz także nowy Klucz tajny z jego Zestawu awaryjnego.",
  "rotation.recovery.done": "To urządzenie używa teraz nowych kluczy.",
  "rotation.recovery.optional":
    "Klucz tajny z najnowszego Zestawu awaryjnego (tylko jeśli go wymieniono)",
  "rotation.file.unknown":
    "W folderze synchronizacji pojawił się nieznany plik konta. Nie można go sprawdzić Twoimi kluczami: może być pozostałością lub ktoś zmienił Twój folder. Twoje dane nie są zagrożone.",
  "rotation.file.dismiss": "Odrzuć",
  "rotation.file.conflict.title": "Dwa urządzenia odnowiły klucze w tym samym czasie",
  "rotation.file.conflict":
    "Inne urządzenie odnowiło klucze konta w tej samej chwili co to, więc oba przestały być zgodne. Wymień klucze ponownie z tego urządzenia, aby je zgrać: klucze tego urządzenia wygrywają, a drugie zapyta o nowy Klucz tajny.",
  "rotation.file.conflict.rekey": "Wymień klucze ponownie z tego urządzenia",
  "rotation.file.conflict.keep": "Ukryj informację",
  "rotation.resume.body.vault":
    "Ponowne szyfrowanie załączników sejfu zostało przerwane. Nic nie zostało utracone. Kontynuuj, aby je dokończyć.",
} satisfies Record<keyof typeof en, string>;
