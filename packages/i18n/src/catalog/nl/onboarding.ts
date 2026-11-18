/**
 * Dutch UI strings: onboarding. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { onboarding as en } from "../en/onboarding.ts";

export const onboarding = {
  "onboarding.secretKey.title": "Je Geheime Sleutel",
  "onboarding.secretKey.body":
    "Je Geheime Sleutel en hoofdwachtwoord ontgrendelen samen je kluis. Wij kunnen geen van beide herstellen.",
  "onboarding.secretKey.verify":
    "Typ de laatste groep van je Geheime Sleutel om te bevestigen dat je hem hebt bewaard.",
  "onboarding.secretKey.mistyped":
    "Dat komt niet overeen. Controleer de laatste groep in je Noodkit.",
  "onboarding.step": "Stap {current} van {total}",
  "onboarding.create.title": "Maak je account aan",
  "onboarding.create.body":
    "Je hoofdwachtwoord en een Geheime Sleutel die wij op dit apparaat genereren, ontgrendelen samen je kluis. Geen van beide wordt ooit naar de server gestuurd.",
  "onboarding.create.email": "E-mail",
  "onboarding.create.password": "Hoofdwachtwoord",
  "onboarding.create.passwordHint":
    "Vier niet bij elkaar horende woorden werken goed. Gebruik het alleen voor Mosaic.",
  "onboarding.create.confirm": "Hoofdwachtwoord bevestigen",
  "onboarding.create.mismatch":
    "De twee wachtwoorden komen niet overeen. Typ in beide velden hetzelfde hoofdwachtwoord.",
  "onboarding.create.submit": "Account aanmaken",
  "onboarding.create.working": "Je account wordt aangemaakt. Dit duurt een paar seconden.",
  "onboarding.kit.accountId": "Account-ID",
  "onboarding.kit.secretKey": "Geheime Sleutel",
  "onboarding.kit.lastGroup": "Laatste groep van je Geheime Sleutel",
  "onboarding.kit.confirm": "Bevestigen",
  "kit.title": "Bewaar je Noodkit",
  "kit.body":
    "De kit is een pdf met je Geheime Sleutel. Samen met je hoofdwachtwoord opent hij je kluis op een nieuw apparaat. Bewaar hem op een veilige plek, buiten deze computer.",
  "kit.secretKey": "Je Geheime Sleutel",
  "kit.save": "Noodkit bewaren (pdf)",
  "kit.saved": "Bewaard. Bewaar het bestand op een veilige plek.",
  "kit.cancelled": "Nog niet bewaard. Bewaar de kit om door te gaan.",
  "kit.recover.alreadySaved": "Ik heb hem al bewaard",
  "kit.recover.title": "Bewaar je Noodkit opnieuw",
  "kit.recover.body":
    "Je hebt je Noodkit nog niet bevestigd. Voer je hoofdwachtwoord in om hem opnieuw te bewaren en bevestig daarna de laatste groep van je Geheime Sleutel.",
  "onboarding.import.title": "Neem je wachtwoorden mee",
  "onboarding.import.body":
    "Uit Chrome, Edge, Firefox, Safari en andere browsers, of uit 1Password, Bitwarden, LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm, KeePass en Enpass. Het duurt een paar minuten. Je kunt het ook later doen bij de instellingen.",
  "onboarding.import.start": "Importeren uit een andere app",
  "onboarding.import.skip": "Nu overslaan",
  "onboarding.done.title": "Alles is klaar",
  "onboarding.done.body":
    "Je kluis is klaar. Vergrendel hem op elk moment met de knop Vergrendelen.",
  "onboarding.browser.title": "Installeer de browserextensie",
  "onboarding.browser.body":
    "Vul logins in je browser in. De extensie vult alleen in wanneer je erop klikt of de sneltoets gebruikt.",
  "onboarding.browser.steps": "Zo verbind je een browser",
  "onboarding.browser.step1": "Installeer de Mosaic-extensie in je browser.",
  "onboarding.browser.step2": "Open de extensie en kies Verbinden.",
  "onboarding.browser.step3":
    "Controleer of de code overeenkomt en kies Toestaan in het venster dat hier opent.",
  "onboarding.browser.notInStores":
    "De extensie staat nog niet in de browserwinkels. Je kunt hem later verbinden via Instellingen → Browsers.",
  "onboarding.browser.getFor": "Downloaden voor {browser}",
  "onboarding.browser.connectedList": "Verbonden browsers",
  "onboarding.browser.waiting": "Wachten op de extensie. Er is nog geen browser verbonden.",
  "onboarding.browser.connected": "{name} verbonden",
  "onboarding.browser.done": "Klaar",
  "onboarding.browser.skip": "Nu overslaan",
  "onboarding.progress": "Voortgang van de installatie",
  "onboarding.stepName.create": "Account aanmaken",
  "onboarding.stepName.kit": "Noodkit",
  "onboarding.stepName.verify": "Bevestig je kit",
  "onboarding.stepName.import": "Importeren (optioneel)",
  "onboarding.stepName.browser": "Browserextensie (optioneel)",
  "kit.preview": "Voorvertoning van de Noodkit",
  "kit.continueHint": "Bewaar de kit om door te gaan.",
  "kit.warning":
    "Zonder deze kit en je hoofdwachtwoord kan niemand je kluis herstellen, ook wij niet.",
  "onboarding.join.link": "Ik gebruik Mosaic al",
  "onboarding.join.title": "Dit apparaat aan je account toevoegen",
  "onboarding.join.body":
    "Log in met je hoofdwachtwoord en de Geheime Sleutel uit je Noodkit. Je kluizen worden daarna op dit apparaat gedownload en ontsleuteld.",
  "onboarding.join.tabs": "Waar staan je synchronisatiegegevens?",
  "onboarding.join.tab.server": "Mosaic-server",
  "onboarding.join.tab.folder": "Cloudmap",
  "onboarding.join.serverUrl": "Serveradres",
  "onboarding.join.email": "E-mail van het synchronisatieaccount",
  "onboarding.join.password": "Hoofdwachtwoord",
  "onboarding.join.secretKey": "Geheime Sleutel",
  "onboarding.join.secretKeyHint": "Uit je Noodkit. Hij begint met M1.",
  "onboarding.join.deviceName": "Naam van dit apparaat",
  "onboarding.join.chooseFolder": "Map kiezen…",
  "onboarding.join.folder": "Map",
  "onboarding.join.noAccounts":
    "Deze map bevat geen Mosaic-account. Kies de map waarmee je andere apparaat synchroniseert.",
  "onboarding.join.account": "Account",
  "onboarding.join.accountOption": "Account {id}",
  "onboarding.join.submit": "Dit apparaat toevoegen",
  "onboarding.join.working": "Inloggen en je kluizen downloaden…",
  "onboarding.join.twoFactor.title": "Voer je code in",
  "onboarding.join.twoFactor.body":
    "Je synchronisatieaccount gebruikt tweestapsverificatie. Voer de 6-cijferige code uit je authenticator-app in.",
  "onboarding.join.twoFactor.code": "6-cijferige code",
  "onboarding.join.twoFactor.submit": "Verifiëren",
  "onboarding.join.back": "In plaats daarvan een nieuw account aanmaken",
} satisfies Record<keyof typeof en, string>;
