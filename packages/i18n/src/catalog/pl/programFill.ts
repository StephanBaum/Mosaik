/**
 * Polish UI strings: programFill. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { programFill as en } from "../en/programFill.ts";

export const programFill = {
  "programFill.chip.label": "Wypełnij przez Mosaic",
  "programFill.chip.locked": "Odblokuj Mosaic, aby wypełnić",
  "programFill.list.title": "Logowania dla {program}",
  "programFill.list.none": "Brak logowania dla {program}",
  "programFill.list.signerChanged": "Podpisujący ten program się zmienił. Powiązać ponownie?",
  "programFill.list.fileChanged": "Ten program zmienił się od czasu powiązania. Powiązać ponownie?",
  "programFill.list.suggestions": "Propozycje",
  "programFill.list.search": "Szukaj logowań",
  "programFill.list.link": "Powiąż logowanie…",
  "programFill.list.block": "Nie proponuj dla tego programu",
  "programFill.link.packaged": "{name} · Aplikacja ze sklepu · Wydawca {publisher}",
  "programFill.link.packagedNoPublisher": "{name} · Aplikacja ze sklepu",
  "programFill.link.signed": "{exe} · Podpisany przez {signer}, sprawdzony przez Windows",
  "programFill.link.unsigned": "{exe} · Niepodpisany · {path}",
  "programFill.link.unsignedNote":
    "Mosaic wypełni tylko ten dokładny plik. Jeśli się zmieni, Mosaic zapyta ponownie.",
  "programFill.link.lookalike":
    "Uwaga: ta nazwa wygląda jak {similar}, który został wcześniej powiązany.",
  "programFill.link.hashChange": "Powiązany plik {old}…, ten plik {new}…",
  "programFill.link.login": "Logowanie: {title}",
  "programFill.link.confirm": "Powiąż i zaloguj",
  "programFill.link.confirmFill": "Powiąż i wypełnij",
  "programFill.link.confirmUnsigned": "Powiąż ten niepodpisany program",
  "programFill.link.confirmAgain": "Potwierdź ponownie, aby powiązać",
  "programFill.notice.elevated":
    "Mosaic nie może wypełniać programów uruchomionych jako administrator. Skopiuj zamiast tego hasło z Mosaic.",
  "programFill.notice.noField": "Mosaic nie widzi tu pola logowania.",
  "programFill.notice.aborted":
    "Mosaic przerwał wypełnianie, ponieważ okno się zmieniło. Sprawdź pole.",
  "programFill.notice.abortedCleared":
    "Mosaic przerwał wypełnianie, ponieważ okno się zmieniło, i usunął wpisane dane.",
  "programFill.notice.modifiersHeld":
    "Zwolnij klawisze Shift, Ctrl, Alt i Windows, a potem ponownie wybierz logowanie.",
  "programFill.notice.notResponding":
    "Ten program nie odpowiedział na czas. Nic więcej nie wypełniono.",
  "programFill.notice.close": "Zamknij",
  "programFill.settings.title": "Wypełnianie w programach",
  "programFill.settings.description":
    "Mosaic obserwuje, które pole ma fokus w innych programach na tym urządzeniu, aby proponować tam Twoje logowania. Nic nie jest zapisywane ani wysyłane.",
  "programFill.settings.enable": "Proponuj logowania w programach",
  "programFill.settings.submit": "Zaloguj po wypełnieniu",
  "programFill.settings.hotkey": "Skrót otwierający listę",
  "programFill.settings.hotkeyNone": "Brak skrótu",
  "programFill.settings.hotkeySave": "Zapisz skrót listy",
  "programFill.settings.blocked": "Programy, dla których Mosaic nie proponuje logowań",
  "programFill.settings.blockedNone": "Brak",
  "programFill.settings.unblock": "Proponuj ponownie",
  "programFill.settings.notWindows": "Na razie tylko w systemie Windows.",
  "programFill.settings.mosaicElevated":
    "Mosaic działa jako administrator, więc nie wypełnia programów. Uruchom go normalnie.",
  "programFill.settings.hostFailed":
    "Mosaic nie mógł rozpocząć obserwowania programów. Uruchom Mosaic ponownie, aby spróbować jeszcze raz.",
  "programFill.card.title": "Wypełniaj logowania także w programach",
  "programFill.card.turnOn": "Włącz",
  "programFill.card.notNow": "Nie teraz",
  "programFill.item.program": "Program",
  "programFill.item.storeApp": "Aplikacja ze sklepu",
  "programFill.item.signedBy": "Podpisany przez {signer}",
  "programFill.item.unsigned": "Niepodpisany: {path}",
  "programFill.item.submit": "Zaloguj po wypełnieniu",
  "programFill.item.unlink": "Odłącz",
} satisfies Record<keyof typeof en, string>;
