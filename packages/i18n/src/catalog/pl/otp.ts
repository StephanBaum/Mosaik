/**
 * Polish UI strings: otp. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { otp as en } from "../en/otp.ts";

export const otp = {
  "otp.section.label": "Kod jednorazowy (2FA)",
  "otp.section.empty": "Dodaj kod ze strony konfiguracji uwierzytelniania dwuskładnikowego witryny.",
  "otp.section.required": "Dodaj kod jednorazowy, aby zapisać ten element.",
  "otp.source.screen": "Skanuj ekran",
  "otp.source.clipboard": "Wklej zrzut ekranu",
  "otp.source.file": "Wybierz obraz…",
  "otp.source.manual": "Nie można zeskanować? Wpisz klucz",
  "otp.source.screenHint":
    "Mosaic na chwilę ustępuje, raz patrzy na Twój ekran i zachowuje tylko kod.",
  "otp.source.clipboardHint":
    "{platform, select, mac {Naciśnij Cmd+Ctrl+Shift+4, zaznacz kod QR, a potem wklej go tutaj.} windows {Naciśnij Win+Shift+S, zaznacz kod QR, a potem wklej go tutaj.} other {Skopiuj zrzut ekranu kodu QR, a potem wklej go tutaj.}}",
  "otp.dialog.title": "Skonfiguruj kod jednorazowy",
  "otp.working.screen": "Szukanie kodu QR na ekranie…",
  "otp.working.image": "Odczytywanie kodu QR…",
  "otp.manual.title": "Wpisz klucz konfiguracji",
  "otp.manual.input": "Klucz konfiguracji lub link otpauth://",
  "otp.manual.inputHint":
    "Witryna pokazuje go obok kodu QR, często pod „Nie można zeskanować?” lub „Wpisz ręcznie”. Spacje nie mają znaczenia.",
  "otp.manual.issuer": "Usługa",
  "otp.manual.account": "Konto",
  "otp.manual.advanced": "Zaawansowane",
  "otp.manual.algorithm": "Algorytm",
  "otp.manual.digits": "Cyfry",
  "otp.manual.period": "Nowy kod co",
  "otp.manual.periodValue": "{seconds} s",
  "otp.manual.advancedHint":
    "Zmieniaj to tylko wtedy, gdy witryna tak mówi. Większość witryn używa SHA-1, 6 cyfr i 30 sekund.",
  "otp.manual.continue": "Dalej",
  "otp.confirm.title": "Czy to właściwy kod?",
  "otp.confirm.issuer": "Usługa",
  "otp.confirm.account": "Konto",
  "otp.confirm.details":
    "{scheme, select, hotp {Oparty na liczniku, cyfry: {digits}} steam {Steam Guard, 5 znaków} other {Cyfry: {digits}, nowy kod co {period} s}}",
  "otp.confirm.algorithmValue":
    "{algorithm, select, sha256 {SHA-256} sha512 {SHA-512} other {SHA-1}}",
  "otp.confirm.codeLabel": "Bieżący kod",
  "otp.confirm.codeHint": "Jeśli witryna prosi o kod, aby zakończyć konfigurację, wpisz ten.",
  "otp.confirm.hotpNote":
    "Kody oparte na liczniku pojawiają się po zapisaniu, aby licznik był zgodny z witryną.",
  "otp.confirm.weak":
    "Ten sekret jest nietypowo krótki. Działa, ale ochrona dwuskładnikowa tej witryny jest słabsza niż zwykle.",
  "otp.confirm.use": "Użyj tego kodu",
  "otp.confirm.back": "Skanuj ponownie",
  "otp.confirm.copy": "Kopiuj kod",
  "otp.summary.noIssuer": "Kod jednorazowy",
  "otp.summary.pending": "Zapisany razem z tym elementem.",
  "otp.summary.removed": "Kod jednorazowy zostanie usunięty po zapisaniu.",
  "otp.action.replace": "Zamień",
  "otp.action.remove": "Usuń kod jednorazowy",
  "otp.action.keep": "Zachowaj",
  "otp.action.cancelSetup": "Nie dodawaj",
  "otp.storage.label": "Gdzie przechowywać kod",
  "otp.storage.item": "Razem z tym elementem",
  "otp.storage.itemTradeoff":
    "Wygodne, ale każdy, kto otworzy ten sejf, otrzyma jednocześnie hasło i drugi składnik.",
  "otp.storage.hardware": "W {vault} (wymaga klucza bezpieczeństwa)",
  "otp.storage.hardwareClosed": "Najpierw otwórz {vault} kluczem bezpieczeństwa.",
  "otp.toast.clipboardCleared": "Odczytano zrzut ekranu. Mosaic wyczyścił schowek.",
  "otp.toast.savedInVault": "Kod zapisano w {vault}.",
  "otp.clipboardCopies":
    "{platform, select, windows {Jeśli historia schowka lub Narzędzie wycinanie zachowały kopię zrzutu ekranu, usuń ją tam.} mac {Jeśli zrzut ekranu zapisano także jako plik, usuń go.} other {Jeśli menedżer schowka zachował kopię zrzutu ekranu, usuń ją tam.}}",
} satisfies Record<keyof typeof en, string>;
