/**
 * Dutch UI strings: breach. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { breach as en } from "../en/breach.ts";

export const breach = {
  "breach.card.title": "Datalekcontrole",
  "breach.card.off":
    "Uit. Mosaic neemt nooit contact op met Have I Been Pwned, tenzij je dit aanzet.",
  "breach.card.about":
    "Controleert je wachtwoorden op bekende datalekken. Per wachtwoord verlaat alleen een stukje van 5 tekens van een versleutelde code dit apparaat.",
  "breach.mode.label": "Controleren op bekende datalekken",
  "breach.mode.off": "Uit",
  "breach.mode.manual": "Alleen als ik dat kies",
  "breach.mode.weekly": "Wekelijks, na het ontgrendelen",
  "breach.card.checkNow": "Nu controleren",
  "breach.card.stop": "Controle stoppen",
  "breach.card.turnOff": "Uitzetten",
  "breach.progress.starting": "Je wachtwoorden worden gecontroleerd…",
  "breach.progress.step": "Controleren… {done} van {total}",
  "breach.result.never": "Nog niet gecontroleerd.",
  "breach.result.last":
    "Laatst gecontroleerd op {date, date, medium}. {found, plural, =0 {Er is geen wachtwoord in een datalek gevonden.} one {# wachtwoord is in een datalek gevonden.} other {# wachtwoorden zijn in een datalek gevonden.}}",
  "breach.result.finished":
    "{checked, plural, one {# wachtwoord} other {# wachtwoorden}} gecontroleerd. {found, plural, =0 {Er is niets in een datalek gevonden.} one {# is in een datalek gevonden.} other {# zijn in een datalek gevonden.}}",
  "breach.result.upToDate": "Alles is onlangs gecontroleerd. Er is niets nieuws om op te zoeken.",
  "breach.error.offline":
    "Have I Been Pwned is niet bereikbaar. De laatste resultaten worden getoond. Probeer het opnieuw als je online bent.",
  "breach.error.rateLimited":
    "Have I Been Pwned vroeg Mosaic om te vertragen. Probeer het over een paar minuten opnieuw.",
  "breach.error.protocol":
    "Have I Been Pwned stuurde een antwoord dat Mosaic niet vertrouwt, dus sommige wachtwoorden zijn niet gecontroleerd. Probeer het later opnieuw.",
  "breach.error.cancelled":
    "De controle is gestopt. Resultaten die al klaar waren, blijven bewaard.",
  "breach.consent.title": "Je wachtwoorden controleren op bekende datalekken?",
  "breach.consent.body":
    "Mosaic maakt van elk wachtwoord een versleutelde code die niet meer kan worden teruggedraaid (een hash) en stuurt alleen de eerste 5 tekens van die code naar Have I Been Pwned (api.pwnedpasswords.com). De dienst ziet je IP-adres en die stukjes van 5 tekens. Hij ziet nooit een wachtwoord, je e-mailadres of voor welke site een wachtwoord is. De resultaten blijven op dit apparaat.",
  "breach.consent.detail":
    "Veel andere wachtwoorden delen elk stukje van 5 tekens, dus het antwoord verraadt niet welk wachtwoord van jou is. Het blijft wel een verzoek aan een andere dienst, daarom staat dit uit totdat je akkoord gaat. Je kunt het op elk moment uitzetten.",
  "breach.consent.agree": "Aanzetten en nu controleren",
  "breach.consent.decline": "Niet nu",
} satisfies Record<keyof typeof en, string>;
