/**
 * Dutch UI strings: settings. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { settings as en } from "../en/settings.ts";

export const settings = {
  "settings.title": "Instellingen",
  "settings.security": "Beveiliging",
  "settings.autoLock": "Vergrendelen na inactiviteit",
  "settings.autoLock.never": "Nooit",
  "settings.autoLock.minutes": "{minutes, plural, one {# minuut} other {# minuten}}",
  "settings.lockOnSleep": "Vergrendelen wanneer de computer slaapt",
  "settings.lockOnScreenLock": "Vergrendelen wanneer het scherm vergrendelt",
  "settings.screenLock.unsupported":
    "Mosaic kan op dit systeem niet zien wanneer het scherm vergrendelt. Het vergrendelt nog steeds na inactiviteit en wanneer de computer wakker wordt.",
  "settings.lockTrigger.required":
    "Laat minstens één vergrendeling aan staan. Mosaic kan op dit systeem geen vergrendeld scherm detecteren.",
  "settings.clipboard": "Klembord wissen na",
  "settings.clipboard.never": "Nooit",
  "settings.clipboard.seconds": "{seconds, plural, one {# seconde} other {# seconden}}",
  "settings.appearance": "Weergave",
  "settings.theme": "Thema",
  "settings.theme.system": "Het systeem volgen",
  "settings.theme.light": "Licht",
  "settings.theme.dark": "Donker",
  "settings.language": "Taal",
  "settings.language.system": "Het systeem volgen",
  "settings.language.machineNote":
    "Vertaald met hulp van AI. Klinkt iets verkeerd, laat het ons weten.",
  "settings.import": "Importeren",
  "settings.import.body":
    "Neem je logins mee uit een browser of een andere wachtwoordbeheerder. Mosaic begeleidt je bij de export en helpt je het bestand daarna te verwijderen.",
  "settings.import.open": "Wachtwoorden importeren",
  "settings.ssh": "SSH-agent",
  "settings.ssh.enable": "Mijn SSH-sleutels uit Mosaic gebruiken",
  "settings.ssh.hint":
    "Standaard uit. De agent draait alleen terwijl Mosaic is ontgrendeld en vraagt je voor elk gebruik om toestemming.",
  "settings.ssh.status":
    "{count, plural, =0 {Geen sleutels beschikbaar} one {# sleutel beschikbaar} other {# sleutels beschikbaar}}",
  "settings.ssh.endpoint": "Stel SSH_AUTH_SOCK in op",
  "settings.ssh.skipped": "Sleutels die niet worden aangeboden",
  "settings.kit": "Noodkit",
  "settings.kit.save": "De Noodkit opnieuw bewaren",
  "settings.saved": "Instellingen bewaard.",
  "reauth.title": "Bevestig met je hoofdwachtwoord",
  "reauth.body":
    "Deze wijziging maakt Mosaic minder veilig, dus je hoofdwachtwoord is nodig.",
  "reauth.bodyKit":
    "Je Geheime Sleutel wordt zo in een bestand getoond, dus Mosaic heeft je hoofdwachtwoord nodig.",
  "reauth.password": "Hoofdwachtwoord",
  "reauth.submit": "Bevestigen",
  "settings.intro":
    "Hoe Mosaic op dit apparaat vergrendelt, geheimen wist en je sleutels beschermt.",
  "settings.section.autoLock": "Automatisch vergrendelen",
  "settings.section.autoLock.body": "Wanneer Mosaic zichzelf vergrendelt zonder dat je het vraagt.",
  "settings.section.clipboard": "Klembord",
  "settings.section.clipboard.body": "Gekopieerde geheimen worden na deze tijd gewist.",
  "settings.appearance.body": "Het thema en de taal van Mosaic.",
  "settings.ssh.body":
    "Laat programma’s inloggen met SSH-sleutels uit je kluis. Je keurt elk gebruik goed.",
  "settings.kit.body":
    "Bewaar een nieuwe kopie van je Noodkit. Hiervoor is je hoofdwachtwoord nodig.",
  "settings.quick": "Snel ontgrendelen",
  "settings.quick.body": "Snellere manieren om weer binnen te komen, naast je hoofdwachtwoord.",
  "settings.quick.windowsHello": "Windows Hello",
  "settings.quick.touchId": "Touch ID",
  "settings.quick.biometric.body":
    "Ontgrendel met je gezicht, vingerafdruk of pincode. Je hoofdwachtwoord is na een herstart nog steeds nodig.",
  "settings.quick.biometric.bodyMac":
    "Ontgrendel met je vingerafdruk. Je hoofdwachtwoord is na een herstart nog steeds nodig.",
  "settings.quick.expires":
    "Mosaic vraagt op {date, date, medium} opnieuw om je hoofdwachtwoord.",
  "settings.quick.maxAge": "Vraag minstens elke zoveel tijd om het hoofdwachtwoord:",
  "settings.quick.maxAgeDays": "{days, plural, one {# dag} other {# dagen}}",
  "settings.quick.biometricLinux": "Biometrisch ontgrendelen is niet beschikbaar op Linux.",
  "settings.quick.biometricUnavailable": "Niet ingesteld op dit apparaat.",
  "settings.quick.keys": "Beveiligingssleutels",
  "settings.quick.keysCount":
    "{count, plural, =0 {Geen geregistreerd} one {# geregistreerd} other {# geregistreerd}}",
  "settings.quick.keysUnsupported":
    "Beveiligingssleutels worden op dit systeem nog niet ondersteund.",
  "settings.quick.keyRow": "Beveiligingssleutel · {tag} · toegevoegd op {addedAt, date, medium}",
  "settings.quick.addKey": "Beveiligingssleutel toevoegen",
  "settings.quick.addKeyBody":
    "Steek de sleutel in die je wilt toevoegen en voer de pincode ervan in. Raak hem daarna aan wanneer hij knippert.",
  "settings.quick.manageKeys": "Beheren",
  "settings.quick.removeKey": "Verwijderen",
  "settings.quick.removeKeyNamed": "Beveiligingssleutel {tag} verwijderen",
  "settings.quick.removeKeyConfirm": "Beveiligingssleutel {tag} verwijderen?",
  "settings.quick.keyRisk":
    "Iedereen met deze sleutel en de pincode ervan kan je kluis openen zonder je hoofdwachtwoord.",
  "reauth.bodyQuick":
    "Hierdoor kom je makkelijker in Mosaic, dus je hoofdwachtwoord is nodig.",
  "reauth.bodyKey": "Voor het toevoegen van een sleutel aan deze kluis is je hoofdwachtwoord nodig.",
  "settings.passkeyExport": "Passkeys exporteren",
  "settings.passkeyExport.body":
    "Bewaar de passkeys van een kluis in een FIDO CXF-bestand om ze naar een andere app te verplaatsen.",
  "settings.passkeyExport.vault": "Kluis",
  "settings.passkeyExport.warning":
    "Het bestand bevat je passkeys onversleuteld. Iedereen met het bestand kan als jou inloggen. Verwijder het na het importeren.",
  "settings.passkeyExport.none": "Deze kluis heeft geen passkeys om te exporteren.",
  "settings.passkeyExport.button": "Passkeys exporteren",
  "settings.passkeyExport.reauth":
    "Het bestand bevat je passkeys onversleuteld, dus voor exporteren is je hoofdwachtwoord nodig.",
  "settings.passkeyExport.saved":
    "{count, plural, one {# passkey geëxporteerd.} other {# passkeys geëxporteerd.}} Verwijder het bestand na het importeren.",
  "reauth.bodyMove":
    "Voor het verplaatsen van een item uit een kluis die met een beveiligingssleutel is beschermd, is je hoofdwachtwoord nodig.",
  "toast.settingsReset":
    "Je beveiligingsinstellingen zijn buiten Mosaic om gewijzigd en teruggezet naar veilige standaardwaarden.",
  "settings.shortcut.title": "Sneltoets voor snel zoeken",
  "settings.shortcut.body":
    "Druk in elk programma op deze combinatie om de zoekfunctie van Mosaic te openen.",
  "settings.shortcut.label": "Sneltoets",
  "settings.shortcut.hint":
    "Klik op het veld en druk op de toetsen die je wilt: Ctrl, Alt of ⌘ (op een Mac), plus nog één toets.",
  "settings.shortcut.save": "Sneltoets bewaren",
  "settings.shortcut.reset": "Standaard gebruiken",
  "settings.shortcut.saved": "Sneltoets bewaard: {shortcut}.",
  "settings.shortcut.invalid":
    "Dat is geen sneltoets die Mosaic kan gebruiken. Druk op Ctrl, Alt of ⌘ (op een Mac) samen met nog één toets.",
  "settings.shortcut.unsupported":
    "Je systeem laat apps geen algemene sneltoets instellen, dus dit kan hier niet worden gewijzigd.",
} satisfies Record<keyof typeof en, string>;
