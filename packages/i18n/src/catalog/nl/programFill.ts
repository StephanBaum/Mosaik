/**
 * Dutch UI strings: programFill. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { programFill as en } from "../en/programFill.ts";

export const programFill = {
  "programFill.chip.label": "Invullen met Mosaic",
  "programFill.chip.locked": "Ontgrendel Mosaic om in te vullen",
  "programFill.list.title": "Logins voor {program}",
  "programFill.list.none": "Nog geen login voor {program}",
  "programFill.list.signerChanged":
    "De ondertekenaar van dit programma is gewijzigd. Opnieuw koppelen?",
  "programFill.list.fileChanged":
    "Dit programma is gewijzigd sinds je het koppelde. Opnieuw koppelen?",
  "programFill.list.suggestions": "Suggesties",
  "programFill.list.search": "Logins zoeken",
  "programFill.list.link": "Een login koppelen…",
  "programFill.list.block": "Niet aanbieden voor dit programma",
  "programFill.link.packaged": "{name} · Store-app · Uitgever {publisher}",
  "programFill.link.packagedNoPublisher": "{name} · Store-app",
  "programFill.link.signed": "{exe} · Ondertekend door {signer}, gecontroleerd door Windows",
  "programFill.link.unsigned": "{exe} · Niet ondertekend · {path}",
  "programFill.link.unsignedNote":
    "Mosaic vult alleen dit exacte bestand in. Als het verandert, vraagt Mosaic het opnieuw.",
  "programFill.link.lookalike":
    "Let op: deze naam lijkt op {similar}, dat je eerder hebt gekoppeld.",
  "programFill.link.hashChange": "Gekoppeld bestand {old}…, dit bestand {new}…",
  "programFill.link.login": "Login: {title}",
  "programFill.link.confirm": "Koppelen en inloggen",
  "programFill.link.confirmFill": "Koppelen en invullen",
  "programFill.link.confirmUnsigned": "Dit niet-ondertekende programma koppelen",
  "programFill.link.confirmAgain": "Bevestig opnieuw om te koppelen",
  "programFill.notice.elevated":
    "Mosaic kan geen programma’s invullen die als beheerder draaien. Kopieer in plaats daarvan het wachtwoord uit Mosaic.",
  "programFill.notice.noField": "Mosaic ziet hier geen loginveld.",
  "programFill.notice.aborted":
    "Mosaic stopte met invullen omdat het venster veranderde. Controleer het veld.",
  "programFill.notice.abortedCleared":
    "Mosaic stopte met invullen omdat het venster veranderde en heeft verwijderd wat het had getypt.",
  "programFill.notice.modifiersHeld":
    "Laat Shift, Ctrl, Alt en de Windows-toets los en kies de login opnieuw.",
  "programFill.notice.notResponding":
    "Dit programma antwoordde niet op tijd. Er is niets meer ingevuld.",
  "programFill.notice.close": "Sluiten",
  "programFill.settings.title": "Invullen in programma’s",
  "programFill.settings.description":
    "Mosaic let op welk veld de focus heeft in andere programma’s op dit apparaat om je logins daar aan te bieden. Er wordt niets opgeslagen of verstuurd.",
  "programFill.settings.enable": "Logins aanbieden in programma’s",
  "programFill.settings.submit": "Inloggen na het invullen",
  "programFill.settings.hotkey": "Sneltoets om de lijst te openen",
  "programFill.settings.hotkeyNone": "Geen sneltoets",
  "programFill.settings.hotkeySave": "Sneltoets van de lijst bewaren",
  "programFill.settings.blocked": "Programma’s waarvoor Mosaic geen logins aanbiedt",
  "programFill.settings.blockedNone": "Geen",
  "programFill.settings.unblock": "Weer aanbieden",
  "programFill.settings.notWindows": "Voorlopig alleen op Windows.",
  "programFill.settings.mosaicElevated":
    "Mosaic draait als beheerder en vult daarom geen programma’s in. Start het normaal.",
  "programFill.settings.hostFailed":
    "Mosaic kon niet beginnen met het volgen van programma’s. Start Mosaic opnieuw om het nog eens te proberen.",
  "programFill.card.title": "Vul ook logins in programma’s in",
  "programFill.card.turnOn": "Aanzetten",
  "programFill.card.notNow": "Niet nu",
  "programFill.item.program": "Programma",
  "programFill.item.storeApp": "Store-app",
  "programFill.item.signedBy": "Ondertekend door {signer}",
  "programFill.item.unsigned": "Niet ondertekend: {path}",
  "programFill.item.submit": "Inloggen na het invullen",
  "programFill.item.unlink": "Loskoppelen",
} satisfies Record<keyof typeof en, string>;
