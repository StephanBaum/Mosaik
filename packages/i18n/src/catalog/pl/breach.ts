/**
 * Polish UI strings: breach. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { breach as en } from "../en/breach.ts";

export const breach = {
  "breach.card.title": "Sprawdzanie wycieków",
  "breach.card.off":
    "Wyłączone. Mosaic nigdy nie łączy się z Have I Been Pwned, dopóki tego nie włączysz.",
  "breach.card.about":
    "Sprawdza Twoje hasła pod kątem znanych wycieków danych. Z tego urządzenia wychodzi tylko 5-znakowy fragment zaszyfrowanego kodu każdego hasła.",
  "breach.mode.label": "Sprawdzaj pod kątem znanych wycieków danych",
  "breach.mode.off": "Wyłączone",
  "breach.mode.manual": "Tylko gdy sam zdecyduję",
  "breach.mode.weekly": "Co tydzień, po odblokowaniu",
  "breach.card.checkNow": "Sprawdź teraz",
  "breach.card.stop": "Zatrzymaj sprawdzanie",
  "breach.card.turnOff": "Wyłącz",
  "breach.progress.starting": "Sprawdzanie haseł…",
  "breach.progress.step": "Sprawdzanie… {done} z {total}",
  "breach.result.never": "Jeszcze nie sprawdzano.",
  "breach.result.last":
    "Ostatnio sprawdzono {date, date, medium}. {found, plural, =0 {Nie znaleziono żadnego hasła w wycieku.} one {Znaleziono # hasło w wycieku.} few {Znaleziono # hasła w wycieku.} many {Znaleziono # haseł w wycieku.} other {Znaleziono # hasła w wycieku.}}",
  "breach.result.finished":
    "Sprawdzono {checked, plural, one {# hasło} few {# hasła} many {# haseł} other {# hasła}}. {found, plural, =0 {Nic nie znaleziono w wycieku.} one {# znaleziono w wycieku.} few {# znaleziono w wycieku.} many {# znaleziono w wycieku.} other {# znaleziono w wycieku.}}",
  "breach.result.upToDate": "Wszystko sprawdzono niedawno. Nie ma nic nowego do wyszukania.",
  "breach.error.offline":
    "Nie można połączyć się z Have I Been Pwned. Widać ostatnie wyniki. Spróbuj ponownie, gdy będziesz online.",
  "breach.error.rateLimited":
    "Have I Been Pwned poprosił Mosaic o zwolnienie tempa. Spróbuj ponownie za kilka minut.",
  "breach.error.protocol":
    "Have I Been Pwned wysłał odpowiedź, której Mosaic nie ufa, więc część haseł nie została sprawdzona. Spróbuj ponownie później.",
  "breach.error.cancelled": "Sprawdzanie zatrzymano. Zakończone wyniki zostały zachowane.",
  "breach.consent.title": "Sprawdzać hasła pod kątem znanych wycieków danych?",
  "breach.consent.body":
    "Mosaic zamienia każde hasło w zaszyfrowany kod, którego nie da się odwrócić (skrót), i wysyła do Have I Been Pwned (api.pwnedpasswords.com) tylko pierwsze 5 znaków tego kodu. Usługa dowiaduje się Twojego adresu IP i tych 5-znakowych fragmentów. Nigdy nie widzi hasła, Twojego adresu e-mail ani witryny, do której hasło należy. Wyniki zostają na tym urządzeniu.",
  "breach.consent.detail":
    "Każdy 5-znakowy fragment pasuje do wielu innych haseł, więc odpowiedź nie ujawnia Twojego. To nadal żądanie do innej usługi, dlatego funkcja jest wyłączona, dopóki się nie zgodzisz. Możesz ją wyłączyć w dowolnym momencie.",
  "breach.consent.agree": "Włącz i sprawdź teraz",
  "breach.consent.decline": "Nie teraz",
} satisfies Record<keyof typeof en, string>;
