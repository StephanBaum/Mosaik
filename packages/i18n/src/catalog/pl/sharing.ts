/**
 * Polish UI strings: sharing. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { sharing as en } from "../en/sharing.ts";

export const sharing = {
  "sharing.title": "Udostępnianie",
  "sharing.body":
    "Udostępniaj sejfy osobom, które znasz. Wszystko pozostaje zaszyfrowane: udostępniony sejf mogą odczytać tylko zaproszone osoby, a Mosaic udostępnia tylko osobom, których kod bezpieczeństwa został porównany.",
  "sharing.notHosted.none":
    "Udostępnianie wymaga serwera Mosaic. Najpierw włącz synchronizację z serwerem Mosaic w ustawieniach synchronizacji.",
  "sharing.notHosted.folder":
    "Udostępnianie nie jest dostępne z folderem synchronizacji. Folder nie wie, kto może odczytać sejf, więc Mosaic trzyma sejfy w folderze prywatnie dla Ciebie. Aby udostępniać, rozłącz folder i połącz serwer Mosaic w ustawieniach synchronizacji.",
  "sharing.notHosted":
    "Udostępnianie wymaga serwera Mosaic. Najpierw połącz go w ustawieniach synchronizacji.",
  "sharing.syncSettings": "Otwórz ustawienia synchronizacji",
  "sharing.ownCode.title": "Twój kod bezpieczeństwa",
  "sharing.ownCode.body":
    "Przeczytaj ten kod osobie, której udostępniasz, osobiście lub przez telefon. Jeśli zgadza się z tym, co ona widzi dla Ciebie, nikt nie stoi między Wami.",
  "sharing.ownCode.pending":
    "Twój kod bezpieczeństwa pojawi się po pierwszej synchronizacji z serwerem.",
  "sharing.contacts.title": "Osoby, które wyszukano",
  "sharing.contacts.empty": "Nikogo jeszcze nie wyszukano.",
  "sharing.contacts.verified": "Zweryfikowano",
  "sharing.contacts.notVerified": "Niezweryfikowano",
  "sharing.contacts.changed": "Wymaga ponownego sprawdzenia",
  "sharing.contacts.verify": "Porównaj kod…",
  "sharing.contacts.verifyOf": "Porównaj kod z {email}",
  "sharing.contacts.review": "Sprawdź zmianę…",
  "sharing.contacts.reviewOf": "Sprawdź zmianę dla {email}",
  "sharing.share.open": "Udostępnij…",
  "sharing.share.title": "Udostępnij {vault}",
  "sharing.share.dialog": "Udostępnianie: {vault}",
  "sharing.share.roleStep": "Wybierz rolę dla {email}",
  "sharing.lookup.email": "Adres e-mail tej osoby",
  "sharing.lookup.find": "Wyszukaj",
  "sharing.lookup.searching": "Wyszukiwanie…",
  "sharing.lookup.notFoundHint":
    "Mosaic nie może powiedzieć, czy dany adres ma konto. Jeśli kod powyżej nie zgadza się z tym, co widzi ta osoba, nie udostępniaj.",
  "sharing.code.title": "Porównaj kody bezpieczeństwa",
  "sharing.code.compare":
    "Porównaj ten kod z {email}, osobiście lub przez telefon. Nie używaj e-maila ani czatu.",
  "sharing.code.compareNew":
    "Porównaj nowy kod z {email}, osobiście lub przez telefon. Nie używaj e-maila ani czatu.",
  "sharing.code.shown": "Kod pokazany przez Mosaic dla {email}",
  "sharing.code.typed": "Kod odczytany przez tę osobę (opcjonalnie)",
  "sharing.code.typedMatch": "To jest kod powyżej.",
  "sharing.code.attest":
    "Kod porównano z {email} przez telefon lub osobiście i jest taki sam.",
  "sharing.code.matches": "Kody się zgadzają",
  "sharing.code.mismatch":
    "Kody się różnią. Nie udostępniaj. Ktoś może stać między Tobą a {email}.",
  "sharing.code.mismatchHelp":
    "Sprawdź adres i poproś {email} o ponowne odczytanie kodu. Jeśli nadal się różni, anuluj i powiadom tę osobę w inny sposób.",
  "sharing.code.verified": "Zweryfikowano: {email}.",
  "sharing.code.status.new": "Nowy kontakt",
  "sharing.code.status.pinned": "Jeszcze niezweryfikowano",
  "sharing.code.status.verified": "Zweryfikowano",
  "sharing.code.status.changed": "Klucz się zmienił",
  "sharing.code.status.conflicting": "Urządzenia się nie zgadzają",
  "sharing.code.conflicting":
    "Twoje urządzenia widziały różne klucze dla {email}. Porównaj kod z tą osobą ponownie, zanim udostępnisz.",
  "sharing.verify.title": "Zweryfikuj {email}",
  "sharing.role.label": "Rola",
  "sharing.role.owner": "Właściciel",
  "sharing.role.manage": "Menedżer",
  "sharing.role.write": "Edytor",
  "sharing.role.read": "Czytelnik",
  "sharing.role.desc.manage": "Może zapraszać i usuwać osoby oraz zmieniać role.",
  "sharing.role.desc.write": "Może dodawać i zmieniać elementy.",
  "sharing.role.desc.read": "Może tylko czytać.",
  "sharing.role.honesty":
    "Każdy, kto może odczytać element, może go skopiować. Czytelnik nie może zmieniać sejfu, ale nic nie stoi na przeszkodzie, aby zachował to, co widzi, a późniejsze usunięcie osoby nie cofa tego, co już pobrała.",
  "sharing.invite.send": "Wyślij zaproszenie",
  "sharing.invite.sent":
    "Zaproszenie wysłano do {email}. Ta osoba musi otworzyć Mosaic i je przyjąć.",
  "sharing.invite.sentTitle": "Zaproszenie wysłane",
  "sharing.invite.pending": "Czekanie na {email}",
  "sharing.invite.expires": "Wygasa {when}",
  "sharing.invite.cancel": "Wycofaj zaproszenie",
  "sharing.invite.cancelOf": "Wycofaj zaproszenie dla {email}",
  "sharing.invite.cancelled": "Zaproszenie wycofane.",
  "sharing.inbox.title": "Zaproszenia",
  "sharing.inbox.nav": "Zaproszenia",
  "sharing.inbox.navCount": "Zaproszenia, oczekujące: {count}",
  "sharing.inbox.intro":
    "Osoby, które zaprosiły Cię do udostępnionego sejfu. Mosaic sprawdza każde zaproszenie, zanim Ci je pokaże.",
  "sharing.inbox.empty":
    "Teraz nie ma zaproszeń. Gdy ktoś zaprosi Cię do udostępnionego sejfu, pojawi się tutaj.",
  "sharing.inbox.from": "Od: {email}",
  "sharing.inbox.review": "Sprawdź",
  "sharing.inbox.reviewOf": "Sprawdź zaproszenie od {email}",
  "sharing.inbox.refresh": "Odśwież",
  "sharing.inbox.preview.title": "Dołączyć do {vault}?",
  "sharing.inbox.preview.body": "{email} zaprasza Cię jako: {role}.",
  "sharing.inbox.preview.unnamed": "udostępniony sejf",
  "sharing.inbox.preview.owner": "Sejf należy do {email}.",
  "sharing.inbox.verifySender": "Zanim dołączysz, porównaj kod bezpieczeństwa {email}.",
  "sharing.inbox.verifyOwner":
    "{sender} nie jest właścicielem. Zanim dołączysz, porównaj także kod bezpieczeństwa właściciela: to on może zmieniać klucz sejfu.",
  "sharing.inbox.ownerCode": "Kod właściciela, {email}",
  "sharing.inbox.accept": "Dołącz do sejfu",
  "sharing.inbox.readyTitle": "Wszystko sprawdzone. Dołączyć do tego sejfu?",
  "sharing.inbox.decline": "Odrzuć",
  "sharing.inbox.block": "Odrzuć i zablokuj nadawcę",
  "sharing.inbox.block.title": "Odrzucić i zablokować tego nadawcę?",
  "sharing.inbox.block.body":
    "Mosaic odrzuca zaproszenie i od teraz ignoruje każde zaproszenie od {email}. Nie można tego cofnąć w Mosaic.",
  "sharing.inbox.unreadable":
    "Tego zaproszenia nie można otworzyć. Może być uszkodzone lub nie być przeznaczone dla Ciebie. Poproś {email} o ponowne wysłanie.",
  "sharing.inbox.joined": "Dołączono do sejfu. Za chwilę pojawi się na liście sejfów.",
  "sharing.inbox.declined": "Zaproszenie odrzucone.",
  "sharing.inbox.outgoing": "Wysłane zaproszenia",
  "sharing.inbox.outgoing.empty": "Żadne z Twoich zaproszeń nie czeka na odpowiedź.",
  "sharing.inbox.unknownVault": "sejf",
  "sharing.members.title": "Osoby z dostępem",
  "sharing.members.you": "Ty",
  "sharing.members.unverified": "Niezweryfikowano",
  "sharing.members.unconfirmed": "Nie ma na liście członków",
  "sharing.members.unconfirmed.help":
    "Serwer wymienia tę osobę, ale własna lista członków sejfu jej nie zawiera. Następna zmiana klucza ją zablokuje. Jeśli jej nie znasz, usuń ją.",
  "sharing.members.dropped": "Nie ma już na serwerze",
  "sharing.members.dropped.help":
    "Ta osoba jest na liście członków sejfu, ale serwer już jej nie wymienia. Mogła odejść. Sprawdź, czy tego oczekiwano.",
  "sharing.members.invited": "Zaproszono",
  "sharing.members.removing": "W trakcie usuwania",
  "sharing.members.verified": "Zweryfikowano",
  "sharing.members.verify": "Zweryfikuj…",
  "sharing.members.verifyOf": "Zweryfikuj {email}",
  "sharing.members.remove": "Usuń",
  "sharing.members.removeOf": "Usuń {email}",
  "sharing.members.remove.title": "Usunąć {email}?",
  "sharing.members.remove.confirm":
    "{email} zachowuje wszystko, co już pobrano. Mosaic zmienia teraz klucz sejfu i ta osoba nie odczyta niczego nowego.",
  "sharing.members.remove.confirmManager":
    "{email} zachowuje wszystko, co już pobrano. Serwer przestaje teraz wysyłać tej osobie aktualizacje. Urządzenie właściciela sejfu zmieni klucz sejfu przy następnej synchronizacji, a do tego czasu {email} zatrzymuje poza sejfem tylko serwer.",
  "sharing.members.cancel": "Anuluj",
  "sharing.members.removed": "Usunięto.",
  "sharing.members.leave": "Opuść sejf",
  "sharing.members.leave.title": "Opuścić ten sejf?",
  "sharing.members.leave.confirm":
    "Kopia na tym urządzeniu pozostaje czytelna, ale przestaje się aktualizować. Aby dołączyć ponownie, potrzebne jest nowe zaproszenie.",
  "sharing.members.left": "Sejf został opuszczony.",
  "sharing.members.roleOf": "Rola: {email}",
  "sharing.members.changeRole": "Zmień rolę",
  "sharing.members.changeRoleOf": "Zmień rolę: {email}",
  "sharing.members.invite": "Zaproś kogoś…",
  "sharing.members.empty": "Na razie tylko Ty możesz otworzyć ten sejf.",
  "sharing.members.unknownEmail": "Nieznany adres",
  "sharing.members.managerNote":
    "Możesz zapraszać i usuwać osoby. Klucz sejfu zmienia tylko urządzenie właściciela, więc usunięcie jest zakończone dopiero po synchronizacji urządzenia właściciela.",
  "sharing.members.readerNote":
    "Możesz czytać ten sejf, ale nie zmieniać, kto ma dostęp.",
  "sharing.rotation.running":
    "Usunięcie czeka, aż urządzenie właściciela zmieni klucz sejfu. Do czasu synchronizacji usuniętą osobę zatrzymuje poza sejfem tylko serwer.",
  "sharing.rotation.unprotected":
    "Liczba plików, które zachowują starą ochronę, ponieważ to urządzenie ich nie ma: {count}. Usunięta osoba nadal może je otworzyć, jeśli zachowała stary klucz. Dodaj je ponownie z urządzenia, które je ma.",
  "sharing.keyChanged.title": "Kod bezpieczeństwa {email} się zmienił",
  "sharing.keyChanged.body":
    "Serwer pokazuje teraz inny klucz dla {email}. Zdarza się to, gdy ta osoba ponownie skonfiguruje Mosaic, ale tak samo wyglądałby atak na serwer. Nic nie jest udostępniane z nowym kluczem, dopóki nie porównasz kodów z tą osobą, osobiście lub przez telefon.",
  "sharing.keyChanged.old": "Kod porównany wcześniej",
  "sharing.keyChanged.new": "Nowy kod",
  "sharing.keyChanged.accept": "Nowy kod się zgadza",
  "sharing.finding.region": "Ostrzeżenia udostępniania",
  "sharing.finding.summary":
    "{count, plural, one {# ostrzeżenie udostępniania} few {# ostrzeżenia udostępniania} many {# ostrzeżeń udostępniania} other {# ostrzeżenia udostępniania}}",
  "sharing.finding.review": "Sprawdź",
  "sharing.finding.collapse": "Zwiń",
  "sharing.finding.dismiss": "Odrzuć",
  "sharing.finding.unknownEmail": "nieznana osoba",
  "sharing.finding.verify": "Zweryfikuj…",
  "sharing.finding.remove": "Usuń",
  "sharing.finding.removeOf": "Usuń {email} z {vault}",
  "sharing.finding.identityMismatch":
    "Serwer pokazuje inny klucz udostępniania dla Twojego konta niż ten utworzony przez to urządzenie, więc udostępnianie jest wstrzymane. Niczego nie udostępniaj i sprawdź serwer oraz pozostałe urządzenia. Potem wybierz „Sprawdzono…”.",
  "sharing.finding.rosterInvalid":
    "Lista członków {vault} jest uszkodzona lub ktoś zmienił ją bez uprawnień. Nie dodawaj osób, dopóki nie sprawdzisz, kto ma dostęp. Potem wybierz „Sprawdzono…”.",
  "sharing.finding.shareUntrusted":
    "Odrzucono nowy klucz dla {vault}: wysłał go {sender}, ale ta osoba nie ma prawa zmieniać klucza tego sejfu. Sejf pozostaje na starym kluczu. Zapytaj właściciela, czy go zmieniał. Potem wybierz „Sprawdzono…”.",
  "sharing.finding.memberUnverified":
    "Nie zweryfikowano {email} w {vault}. Mosaic nie wysyła tej osobie nowych kluczy, a następna zmiana klucza ją zablokuje. Porównaj jej kod bezpieczeństwa lub ją usuń.",
  "sharing.finding.memberDropped":
    "{email} jest na liście członków {vault}, ale serwer już tej osoby nie wymienia. Mogła odejść lub serwer ją usunął. Sprawdź, czy tego oczekiwano, a jeśli nie, usuń tę osobę.",
  "sharing.finding.memberKeyMismatch":
    "Mosaic otrzymał klucz dla {email}, który nie pasuje do zweryfikowanego, więc ta osoba została usunięta z {vault}. Porównaj z nią kody bezpieczeństwa ponownie, zanim zaprosisz ją jeszcze raz.",
  "sharing.finding.vaultHeld":
    "{vault} czeka, aż Twoje pozostałe urządzenia potwierdzą, do kogo należy. Otwórz Mosaic na urządzeniu, które już używa tego sejfu.",
  "sharing.finding.accessLost":
    "Nie masz już dostępu do {vault}. Kopia na tym urządzeniu pozostaje czytelna, ale się nie aktualizuje. Poproś właściciela o ponowne zaproszenie.",
  "sharing.finding.rotationBlocked":
    "Nie udało się zmienić klucza {vault} po usunięciu kogoś, więc ta osoba nadal ma stary klucz. Zsynchronizuj ponownie. Jeśli to się utrzymuje, otwórz listę członków i sprawdź, kto ma dostęp.",
  "emergency.title": "Dostęp awaryjny",
  "emergency.body":
    "Wskaż osoby, które mogą poprosić o dostęp do Twoich sejfów, jeśli coś Ci się stanie. To nie jest odzyskiwanie konta: otrzymują dostęp tylko do odczytu do wybranych sejfów, dopiero po czasie oczekiwania, a w tym czasie możesz odmówić. Nie zastępuje Zestawu awaryjnego.",
  "emergency.folderUnsupported":
    "Dostęp awaryjny wymaga serwera Mosaic. Folder synchronizacji nie może wymusić czasu oczekiwania.",
  "emergency.granted.title": "Osoby, które mogą prosić o dostęp",
  "emergency.granted.empty": "Nikt nie może prosić o dostęp do Twoich sejfów.",
  "emergency.granted.add": "Dodaj osobę…",
  "emergency.granted.scopeAll": "Wszystkie Twoje sejfy",
  "emergency.granted.scopeSome":
    "{count, plural, one {# sejf} few {# sejfy} many {# sejfów} other {# sejfu}}",
  "emergency.granted.wait":
    "Czeka {days, plural, one {# dzień} few {# dni} many {# dni} other {# dnia}}",
  "emergency.granted.waitingUntil": "Poproszono o dostęp. Otworzy się {when}, chyba że odmówisz.",
  "emergency.granted.approved": "Żądanie zatwierdzono.",
  "emergency.granted.released": "Dostęp został udzielony. Ta osoba może czytać sejfy.",
  "emergency.granted.idle": "Brak żądania",
  "emergency.granted.remove": "Odbierz dostęp",
  "emergency.granted.removeOf": "Odbierz dostęp: {email}",
  "emergency.granted.remove.title": "Odebrać dostęp awaryjny: {email}?",
  "emergency.granted.remove.body":
    "Ta osoba nie może już prosić. Jeśli jej dostęp został już udzielony, Mosaic zmienia klucze otwartych przez nią sejfów; zachowuje to, co już pobrała.",
  "emergency.granted.removed": "Dostęp awaryjny odebrany.",
  "emergency.grant.title": "Zezwól na dostęp awaryjny",
  "emergency.grant.lookup": "Kto ma móc prosić?",
  "emergency.grant.optionsTitle": "Co może otworzyć {email}",
  "emergency.grant.scope": "Co może otworzyć",
  "emergency.grant.scope.all": "Wszystkie moje sejfy, także nowe",
  "emergency.grant.scope.selected": "Tylko wybrane sejfy",
  "emergency.grant.scope.pick": "Wybierz co najmniej jeden sejf.",
  "emergency.grant.wait": "Czas oczekiwania",
  "emergency.grant.wait.option":
    "{days, plural, one {# dzień} few {# dni} many {# dni} other {# dnia}}",
  "emergency.grant.waitHint":
    "Otwórz Mosaic co najmniej raz w tym czasie po żądaniu, aby je zobaczyć i móc odmówić. Gdy czas minie, osoba otrzymuje dostęp tylko do odczytu, bez ponownego pytania Cię o zgodę.",
  "emergency.grant.serverTrust":
    "Czas oczekiwania pilnuje serwer. Jeśli wskazana osoba i osoba zarządzająca serwerem będą współpracować, mogą go pominąć. Sam serwer nie może odczytać Twoich sejfów.",
  "emergency.grant.vaultKeysOnly":
    "Obejmuje to tylko klucze Twoich sejfów. Nie daje dostępu do konta, hasła głównego ani Klucza tajnego.",
  "emergency.grant.submit": "Zezwól na dostęp awaryjny",
  "emergency.grant.done": "{email} może teraz prosić o dostęp awaryjny.",
  "emergency.grant.needVerify": "Najpierw porównaj kod bezpieczeństwa {email}.",
  "emergency.received.title": "Osoby, które Cię wskazały",
  "emergency.received.empty": "Nikt nie wskazał Cię do dostępu awaryjnego.",
  "emergency.received.request": "Poproś o dostęp",
  "emergency.received.requestOf": "Poproś o dostęp: {email}",
  "emergency.received.request.title": "Poprosić {email} o dostęp?",
  "emergency.received.request.body":
    "{email} zobaczy Twoje żądanie przy następnym otwarciu Mosaic. Po {days, plural, one {# dniu} few {# dniach} many {# dniach} other {# dnia}} możesz otworzyć sejfy tej osoby, tylko do odczytu, chyba że odmówi.",
  "emergency.received.requested":
    "Żądanie wysłano. {email} zobaczy je przy następnym otwarciu Mosaic.",
  "emergency.received.requestHint":
    "{email} zobaczy Twoje żądanie w Mosaic. Po {days, plural, one {# dniu} few {# dniach} many {# dniach} other {# dnia}} możesz otworzyć sejfy tej osoby, tylko do odczytu, chyba że odmówi.",
  "emergency.received.countdown": "Sejfy tej osoby możesz otworzyć {when}.",
  "emergency.received.waitingLive": "Twoje żądanie czeka.",
  "emergency.received.claim": "Otwórz sejfy tej osoby",
  "emergency.received.claimOf": "Otwórz sejfy: {email}",
  "emergency.received.claimed":
    "{count, plural, one {Dodano # sejf, tylko do odczytu.} few {Dodano # sejfy, tylko do odczytu.} many {Dodano # sejfów, tylko do odczytu.} other {Dodano # sejfu, tylko do odczytu.}}",
  "emergency.received.cooldown": "Odmowa ze strony {email}. Możesz poprosić ponownie {when}.",
  "emergency.received.verifyFirst":
    "Porównaj kod bezpieczeństwa {email}, zanim otworzysz sejfy tej osoby. Mosaic używa tylko zweryfikowanego klucza.",
  "emergency.received.verify": "Porównaj kod…",
  "emergency.received.verifyOf": "Porównaj kod z {email}",
  "emergency.received.approved": "Twoje żądanie zostało zatwierdzone przez {email}.",
  "emergency.received.waitDays":
    "Czeka {days, plural, one {# dzień} few {# dni} many {# dni} other {# dnia}}",
  "emergency.banner.region": "Żądanie dostępu awaryjnego",
  "emergency.banner.body":
    "Prośba o dostęp awaryjny do Twoich sejfów od: {email}. Otworzy się {when}, chyba że odmówisz.",
  "emergency.banner.refuse": "Odmów",
  "emergency.banner.approve": "Zatwierdź teraz…",
  "emergency.banner.approve.title": "Zatwierdzić dostęp awaryjny teraz?",
  "emergency.banner.approve.body":
    "{email} od razu otrzymuje dostęp tylko do odczytu do Twoich sejfów. Wpisz hasło główne, aby potwierdzić.",
  "emergency.banner.approve.confirm": "Zatwierdź",
  "emergency.banner.refused": "Żądanie odrzucone.",
  "emergency.banner.approved": "Dostęp zatwierdzony.",
} satisfies Record<keyof typeof en, string>;
