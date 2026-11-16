/**
 * Dutch UI strings: otp. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { otp as en } from "../en/otp.ts";

export const otp = {
  "otp.section.label": "Eenmalige code (2FA)",
  "otp.section.empty": "Voeg de code toe van de installatiepagina voor tweestapsverificatie van de site.",
  "otp.section.required": "Voeg een eenmalige code toe om dit item te bewaren.",
  "otp.source.screen": "Scherm scannen",
  "otp.source.clipboard": "Schermafbeelding plakken",
  "otp.source.file": "Afbeelding kiezen…",
  "otp.source.manual": "Lukt scannen niet? Voer de sleutel in",
  "otp.source.screenHint":
    "Mosaic gaat even opzij, kijkt één keer naar je scherm en bewaart niets behalve de code.",
  "otp.source.clipboardHint":
    "{platform, select, mac {Druk op Cmd+Ctrl+Shift+4, selecteer de QR-code en plak hem hier.} windows {Druk op Win+Shift+S, selecteer de QR-code en plak hem hier.} other {Kopieer een schermafbeelding van de QR-code en plak hem hier.}}",
  "otp.dialog.title": "Een eenmalige code instellen",
  "otp.working.screen": "Er wordt gezocht naar een QR-code op je scherm…",
  "otp.working.image": "De QR-code wordt gelezen…",
  "otp.manual.title": "Voer de installatiesleutel in",
  "otp.manual.input": "Installatiesleutel of otpauth://-link",
  "otp.manual.inputHint":
    "De site toont hem naast de QR-code, vaak onder \"Kan niet scannen?\" of \"Handmatig invoeren\". Spaties maken niet uit.",
  "otp.manual.issuer": "Dienst",
  "otp.manual.account": "Account",
  "otp.manual.advanced": "Geavanceerd",
  "otp.manual.algorithm": "Algoritme",
  "otp.manual.digits": "Cijfers",
  "otp.manual.period": "Nieuwe code elke",
  "otp.manual.periodValue": "{seconds} seconden",
  "otp.manual.advancedHint":
    "Wijzig dit alleen als de site dat zegt. De meeste sites gebruiken SHA-1, 6 cijfers en 30 seconden.",
  "otp.manual.continue": "Doorgaan",
  "otp.confirm.title": "Is dit de juiste code?",
  "otp.confirm.issuer": "Dienst",
  "otp.confirm.account": "Account",
  "otp.confirm.details":
    "{scheme, select, hotp {Op teller gebaseerd, {digits} cijfers} steam {Steam Guard, 5 tekens} other {{digits} cijfers, nieuwe code elke {period} seconden}}",
  "otp.confirm.algorithmValue":
    "{algorithm, select, sha256 {SHA-256} sha512 {SHA-512} other {SHA-1}}",
  "otp.confirm.codeLabel": "Huidige code",
  "otp.confirm.codeHint":
    "Als de site om een code vraagt om de installatie af te ronden, voer dan deze in.",
  "otp.confirm.hotpNote":
    "Op een teller gebaseerde codes verschijnen nadat je bewaart, zodat de teller gelijk blijft lopen met de site.",
  "otp.confirm.weak":
    "Dit geheim is ongebruikelijk kort. Het werkt, maar de bescherming met tweestapsverificatie van deze site is zwakker dan gebruikelijk.",
  "otp.confirm.use": "Deze code gebruiken",
  "otp.confirm.back": "Opnieuw scannen",
  "otp.confirm.copy": "Code kopiëren",
  "otp.summary.noIssuer": "Eenmalige code",
  "otp.summary.pending": "Samen met dit item bewaard.",
  "otp.summary.removed": "De eenmalige code wordt verwijderd wanneer je bewaart.",
  "otp.action.replace": "Vervangen",
  "otp.action.remove": "Eenmalige code verwijderen",
  "otp.action.keep": "Behouden",
  "otp.action.cancelSetup": "Niet toevoegen",
  "otp.storage.label": "Waar de code te bewaren",
  "otp.storage.item": "Bij dit item",
  "otp.storage.itemTradeoff":
    "Handig, maar iedereen die deze kluis opent, krijgt je wachtwoord en je tweede factor samen.",
  "otp.storage.hardware": "In {vault} (vereist je beveiligingssleutel)",
  "otp.storage.hardwareClosed": "Open eerst {vault} met je beveiligingssleutel.",
  "otp.toast.clipboardCleared": "Schermafbeelding gelezen. Mosaic heeft het klembord gewist.",
  "otp.toast.savedInVault": "Code bewaard in {vault}.",
  "otp.clipboardCopies":
    "{platform, select, windows {Als de klembordgeschiedenis of Knipprogramma een kopie van de schermafbeelding heeft bewaard, verwijder die daar.} mac {Als je de schermafbeelding ook als bestand hebt bewaard, verwijder die dan.} other {Als een klembordbeheerder een kopie van de schermafbeelding heeft bewaard, verwijder die daar.}}",
} satisfies Record<keyof typeof en, string>;
