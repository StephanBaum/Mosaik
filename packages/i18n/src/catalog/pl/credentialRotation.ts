/**
 * Polish UI strings: credentialRotation. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { credentialRotation as en } from "../en/credentialRotation.ts";

export const credentialRotation = {
  "credentialRotation.tablist": "Widoki bezpieczeństwa",
  "credentialRotation.tab.overview": "Przegląd",
  "credentialRotation.tab.rotate": "Zmień teraz",
  "credentialRotation.tab.dueHidden": ", do zmiany: {count}",
  "credentialRotation.queue.title": "Zmień teraz",
  "credentialRotation.queue.intro":
    "Hasła, które warto zmienić w pierwszej kolejności. Mosaic Cię poprowadzi, ale każde hasło zmieniasz samodzielnie w witrynie.",
  "credentialRotation.queue.empty": "Teraz nie ma nic do zmiany.",
  "credentialRotation.queue.count":
    "{count, plural, one {# hasło do zmiany} few {# hasła do zmiany} many {# haseł do zmiany} other {# hasła do zmiany}}",
  "credentialRotation.queue.list": "Hasła do zmiany",
  "credentialRotation.queue.snoozedHeading": "Odłożone",
  "credentialRotation.queue.snoozedUntil": "Odłożone do {date, date, medium}",
  "credentialRotation.queue.reason.weak": "Słabe",
  "credentialRotation.queue.reason.reused": "Powtórzone",
  "credentialRotation.queue.reason.old": "Stare",
  "credentialRotation.queue.reason.breached": "W wycieku",
  "credentialRotation.queue.reason.unfinished": "Zmiana niedokończona",
  "credentialRotation.queue.change": "Zmień hasło",
  "credentialRotation.queue.continue": "Kontynuuj zmianę",
  "credentialRotation.queue.editItem": "Edytuj element",
  "credentialRotation.queue.manualHint":
    "Mosaic nie może poprowadzić tej zmiany. Otwórz element i zmień hasło samodzielnie.",
  "credentialRotation.queue.actionFor": "{action}: {title}",
  "credentialRotation.queue.snooze": "Odłóż",
  "credentialRotation.queue.snoozeFor": "Odłóż {title}",
  "credentialRotation.queue.snoozeDays":
    "{days, plural, one {# dzień} few {# dni} many {# dni} other {# dnia}}",
  "credentialRotation.queue.snoozed":
    "Odłożono na {days, plural, one {# dzień} few {# dni} many {# dni} other {# dnia}}.",
  "credentialRotation.queue.dismiss": "Odrzuć",
  "credentialRotation.queue.dismissFor": "Odrzuć {title}",
  "credentialRotation.queue.dismissed": "Odrzucono. Wróci, gdy hasło się zmieni.",
  "credentialRotation.queue.undo": "Cofnij",
  "credentialRotation.panel.title": "Zmień hasło do: {title}",
  "credentialRotation.panel.step": "Krok {current} z {total}",
  "credentialRotation.panel.stays":
    "Zmiana dla {title} pozostaje otwarta. Nowe hasło jest przechowywane w elemencie, dopóki nie wybierzesz Udało się lub Nie udało się.",
  "credentialRotation.panel.cancel": "Anuluj",
  "credentialRotation.steps.new.title": "Wybierz nowe hasło",
  "credentialRotation.steps.new.intro":
    "Mosaic zapisuje nowe hasło w tym elemencie, zanim użyjesz go gdziekolwiek, więc nie może zaginąć. Obecne hasło pozostaje na miejscu do czasu Twojego potwierdzenia.",
  "credentialRotation.steps.new.label": "Nowe hasło",
  "credentialRotation.steps.new.use": "Użyj tego hasła",
  "credentialRotation.steps.new.saving": "Zapisywanie…",
  "credentialRotation.steps.site.title": "Zmień je w witrynie",
  "credentialRotation.steps.site.pageLabel": "Strona zmiany",
  "credentialRotation.steps.site.body":
    "Otwórz w przeglądarce stronę zmiany hasła w witrynie, kliknij Mosaic na pasku narzędzi i wybierz Wypełnij formularz zmiany. Mosaic nigdy nie wysyła formularza.",
  "credentialRotation.steps.site.copyLink": "Kopiuj link do strony zmiany",
  "credentialRotation.steps.site.linkCopied": "Skopiowano link.",
  "credentialRotation.steps.site.linkHint":
    "Link nie jest sekretem. Jeśli witryna nie ma strony pod tym adresem, znajdź ustawienia hasła ręcznie.",
  "credentialRotation.steps.site.noLink":
    "To logowanie nie ma adresu https, więc Mosaic nie ma strony zmiany do zaproponowania. Zmień hasło w witrynie ręcznie.",
  "credentialRotation.steps.site.copyPassword": "Kopiuj nowe hasło",
  "credentialRotation.steps.site.next": "Wypróbowano",
  "credentialRotation.steps.result.title": "Czy witryna przyjęła nowe hasło?",
  "credentialRotation.steps.done.failed": "Zachowano obecne hasło",
  "credentialRotation.steps.done.notSure": "Zmiana pozostaje otwarta",
  "credentialRotation.steps.result.body":
    "Mosaic nie widzi witryny. Powiedz mu, co się stało, aby w tym elemencie zostało właściwe hasło. W obu przypadkach nic nie jest usuwane.",
  "credentialRotation.steps.result.working": "Zapisywanie…",
  "credentialRotation.result.worked": "Udało się",
  "credentialRotation.result.failed": "Nie udało się",
  "credentialRotation.result.notSure": "Jeszcze nie wiem",
  "credentialRotation.result.workedDone":
    "Gotowe. Nowe hasło jest teraz hasłem tego elementu. Stare znajduje się w historii.",
  "credentialRotation.result.failedDone":
    "Obecne hasło zostaje. Nowe jest zachowane w historii jako niepotwierdzone, na wypadek gdyby witryna jednak je zastosowała.",
  "credentialRotation.result.notSureDone":
    "Zmiana pozostaje otwarta w tym elemencie. Wróć, gdy będziesz wiedzieć.",
  "credentialRotation.banner.title": "Zmianę hasła rozpoczęto {date, date, medium}",
  "credentialRotation.banner.prepared":
    "Nowe hasło jest tu przechowywane do czasu Twojego potwierdzenia. Żadna strona jeszcze go nie otrzymała.",
  "credentialRotation.banner.delivered":
    "Nowe hasło jest tu przechowywane do czasu Twojego potwierdzenia. Rozszerzenie przeglądarki wpisało je w formularz witryny.",
  "credentialRotation.banner.reveal": "Pokaż nowe hasło",
  "credentialRotation.banner.hide": "Ukryj nowe hasło",
  "credentialRotation.banner.copy": "Kopiuj nowe hasło",
  "credentialRotation.banner.newPassword": "Nowe hasło",
  "credentialRotation.banner.copyLink": "Kopiuj link do strony zmiany",
  "credentialRotation.history.title": "Historia haseł",
  "credentialRotation.history.list": "Wcześniejsze hasła",
  "credentialRotation.history.entry": "Zastąpiono {date, date, medium}",
  "credentialRotation.history.unconfirmed": "Niepotwierdzone",
  "credentialRotation.history.unconfirmedEntry": "Wypróbowano {date, date, medium}",
  "credentialRotation.history.unconfirmedHint":
    "Witryna mogła zastosować to hasło. Przywróć je, jeśli logowanie obecnym hasłem się nie powiedzie.",
  "credentialRotation.history.reveal": "Pokaż hasło z {date, date, medium}",
  "credentialRotation.history.hide": "Ukryj hasło z {date, date, medium}",
  "credentialRotation.history.restore": "Przywróć",
  "credentialRotation.history.restoreFor": "Przywróć hasło z {date, date, medium}",
  "credentialRotation.history.restored": "Przywrócono. Poprzednie hasło jest w historii.",
  "credentialRotation.policy.label": "Przypominaj o zmianie tego hasła",
  "credentialRotation.policy.none": "Nigdy",
  "credentialRotation.policy.every": "Co {days} dni",
  "credentialRotation.policy.help":
    "Przypomnienie dodaje element do listy Zmień teraz. Mosaic nie wysyła z tego powodu powiadomień ani żadnych żądań.",
  "credentialRotation.url.help":
    "Opcjonalnie. Adres https strony zmiany hasła w witrynie, w tej samej witrynie co to logowanie. Mosaic otwiera go tylko po wybraniu Otwórz stronę zmiany w rozszerzeniu przeglądarki.",
  "credentialRotation.url.invalid":
    "Mosaic nie użyje tego adresu. Musi zaczynać się od https:// i należeć do tej samej witryny co to logowanie.",
  "credentialRotation.settings.title": "Przypomnienia o hasłach",
  "credentialRotation.settings.body":
    "Mosaic może po odblokowaniu pokazać, ile haseł warto zmienić.",
  "credentialRotation.settings.reminders": "Pokaż po odblokowaniu, ile haseł do zmiany",
  "credentialRotation.settings.remindersHint":
    "Tylko komunikat w aplikacji. Bez powiadomień i bez żądań sieciowych.",
  "credentialRotation.reminder.toast":
    "{count, plural, one {# hasło do zmiany} few {# hasła do zmiany} many {# haseł do zmiany} other {# hasła do zmiany}}",
  "credentialRotation.reminder.show": "Pokaż",
  "credentialRotation.dialog.beginTitle": "Rozpocząć zmianę hasła?",
  "credentialRotation.dialog.beginBody":
    "{browser} chce zmienić hasło do {title} w witrynie {host}. Mosaic przechowuje nowe hasło przy tym logowaniu i zachowuje obecne, dopóki nie potwierdzisz, że witryna je przyjęła.",
  "credentialRotation.dialog.site": "Witryna",
  "credentialRotation.dialog.login": "Logowanie",
  "credentialRotation.dialog.password": "Nowe hasło",
  "credentialRotation.dialog.hidden": "Ukryte",
  "credentialRotation.dialog.show": "Pokaż",
  "credentialRotation.dialog.hide": "Ukryj",
  "credentialRotation.dialog.regenerate": "Generuj inne",
  "credentialRotation.dialog.regenerated": "Wygenerowano nowe hasło.",
  "credentialRotation.dialog.allow": "Użyj tego hasła",
  "credentialRotation.dialog.cancel": "Anuluj",
  "credentialRotation.dialog.decideLater": "Zdecyduj później",
  "credentialRotation.dialog.untitled": "to logowanie",
  "credentialRotation.dialog.attempt":
    "Rozpoczęto {at, date, medium}, {at, time, short} · {stage}{n, plural, =0 {} one { · # inna zmiana otwarta} few { · # inne zmiany otwarte} many { · # innych zmian otwartych} other { · # innej zmiany otwartej}}",
  "credentialRotation.dialog.stagePrepared": "żadna strona nie ma jeszcze nowego hasła",
  "credentialRotation.dialog.stageDelivered": "nowe hasło wpisano na stronie",
  "credentialRotation.dialog.resultTitle": "Czy {host} przyjął nowe hasło?",
  "credentialRotation.dialog.resultBody":
    "{browser} podaje, że zmiana się udała. Jeśli potwierdzisz, nowe hasło stanie się hasłem do {title}, a obecne trafi do jego historii.",
  "credentialRotation.dialog.confirm": "Udało się",
  "credentialRotation.dialog.keepOld": "Zachowaj stare hasło",
  "credentialRotation.dialog.failedTitle": "Zachować stare hasło?",
  "credentialRotation.dialog.failedBody":
    "{browser} podaje, że zmiana się nie udała. Mosaic zachowuje obecne hasło do {title}. Nowa wartość pozostaje w historii, na wypadek gdyby witryna jednak je zmieniła.",
  "credentialRotation.addOrigin.title": "Dodać {host} do {title}?",
  "credentialRotation.addOrigin.body":
    "Mosaic będzie wypełniać to logowanie także w {host}. Dodaj tylko wtedy, gdy {host} naprawdę jest częścią tej samej witryny.",
  "credentialRotation.addOrigin.idn":
    "Ten adres zawiera znaki specjalne. Przeglądarka może pokazać go jako {unicode}. Sprawdź go uważnie.",
  "credentialRotation.addOrigin.add": "Dodaj adres",
  "credentialRotation.addOrigin.cancel": "Nie teraz",
} satisfies Record<keyof typeof en, string>;
