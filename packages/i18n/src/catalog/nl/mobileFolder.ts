/**
 * Dutch UI strings: mobileFolder. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { mobileFolder as en } from "../en/mobileFolder.ts";

export const mobileFolder = {
  "mobile.sync.never": "Nog niet gesynchroniseerd",
  "mobile.sync.last": "Gesynchroniseerd {time}",
  "mobile.sync.localOnly":
    "Deze kluis staat alleen op deze telefoon. Synchronisatie is niet ingesteld.",
  "mobile.sync.pending":
    "{count, plural, one {# wijziging wacht om te worden geüpload} other {# wijzigingen wachten om te worden geüpload}}",
  "mobile.sync.now": "Nu synchroniseren",
  "mobile.sync.working": "Synchroniseren",
  "mobile.sync.warnings": "Synchronisatiewaarschuwingen",
  "mobile.sync.warning.freezeSuspected":
    "Wijzigingen van een ander apparaat zijn al een tijd niet aangekomen. Controleer of het andere apparaat aan staat en online is.",
  "mobile.sync.warning.noCommonView":
    "Deze telefoon en een ander apparaat hebben twee dagen lang niet dezelfde servergegevens gezien. De server toont ze mogelijk verschillende versies. Synchroniseer je andere apparaat en daarna deze telefoon.",
  "mobile.sync.warning.rollbackDetected":
    "Je synchronisatiegegevens lijken ouder dan deze telefoon. Open Mosaic op een ander apparaat en controleer of je laatste wijzigingen er staan. Kies daarna “Ik heb het gecontroleerd”.",
  "mobile.sync.warning.identityDuplicated":
    "Een andere kopie van Mosaic schrijft onder de identiteit van deze telefoon, bijvoorbeeld nadat een back-up is teruggezet. Synchronisatie is gepauzeerd. Zorg dat er maar één kopie in gebruik is en kies daarna “Ik heb het gecontroleerd”.",
  "mobile.sync.warning.serverInconsistent":
    "De server toonde deze telefoon andere gegevens dan een ander apparaat. Open Mosaic op een ander apparaat en controleer of je laatste wijzigingen er staan. Kies daarna “Ik heb het gecontroleerd”.",
  "mobile.sync.warning.serverRollback":
    "De kopie die de server van een van je kluizen heeft, is ouder dan die van deze telefoon. Open Mosaic op een ander apparaat en controleer of je laatste wijzigingen er staan. Kies daarna “Ik heb het gecontroleerd”.",
  "mobile.sync.warning.sharingAttention":
    "Een gedeelde kluis heeft aandacht nodig: er wacht een sleutelwijziging, of een lid kon niet worden gecontroleerd. Open Mosaic op je computer om te zien welke. Kies daarna “Ik heb het gecontroleerd”.",
  "mobile.sync.warning.undecryptable":
    "{count, plural, one {# ontvangen item kon} other {# ontvangen items konden}} op deze telefoon niet worden gelezen. Werk Mosaic bij op al je apparaten en synchroniseer opnieuw.",
  "mobile.sync.warning.accountRollback":
    "De server toont een oudere accountsleutel dan deze telefoon heeft. Er is niets gewijzigd. Open Mosaic op een ander apparaat en controleer of je laatste wijzigingen er staan. Kies daarna “Ik heb het gecontroleerd”.",
  "mobile.sync.acknowledge": "Ik heb het gecontroleerd…",
  "mobile.sync.acknowledgeTitle": "De synchronisatiewaarschuwingen wissen?",
  "mobile.sync.acknowledgeBody":
    "Ga alleen verder nadat je de server zelf hebt gecontroleerd. Dit wist elke waarschuwing en de synchronisatie gaat door. Als je gegevens echt zijn teruggedraaid of gewijzigd, kan Mosaic dat achteraf niet meer zien. Doe dit op elk van je apparaten.",
  "mobile.sync.acknowledgeConfirm": "Ik heb het gecontroleerd",
  "mobile.folder.intro": "Dit account synchroniseert via je {provider}-map.",
  "mobile.folder.signIn": "Inloggen bij {provider}",
  "mobile.folder.scope":
    "{provider} vraagt toegang tot al je bestanden. Mosaic opent alleen de map die je kiest en raakt niets anders aan.",
  "mobile.folder.waiting": "Rond het inloggen af in je browser en kom dan hier terug.",
  "mobile.folder.serverLabel": "Nextcloud- of WebDAV-adres",
  "mobile.folder.serverHint": "https://cloud.example.com",
  "mobile.folder.serverConfirm":
    "Mosaic logt in bij {server}. Ga alleen door als dit je eigen server is.",
  "mobile.folder.locating": "Je Mosaic-map wordt gezocht…",
  "mobile.folder.browseTitle": "Kies je Mosaic-map",
  "mobile.folder.browseHint":
    "Het is de map waarmee je computer synchroniseert, met een bestand met de naam mosaic-sync.json.",
  "mobile.folder.currentFolder": "Huidige map: {path}",
  "mobile.folder.noSubfolders": "Er zijn hier geen mappen.",
  "mobile.folder.useThis": "Deze map gebruiken",
  "mobile.folder.up": "Een niveau omhoog",
  "mobile.folder.chooseProvider": "Waarmee synchroniseert je account?",
  "mobile.folder.noDrive":
    "Google Drive en iCloud Drive werken nog niet op Android. Gebruik Dropbox, OneDrive, Nextcloud of Mosaic-synchronisatie.",
  "mobile.folder.useFolder": "In plaats daarvan synchroniseren via een cloudmap",
  "mobile.folder.providerDropbox": "Dropbox",
  "mobile.folder.providerOneDrive": "OneDrive",
  "mobile.folder.providerWebDav": "Nextcloud of WebDAV",
  "mobile.folder.appPasswordToggle": "In plaats daarvan een app-wachtwoord gebruiken",
  "mobile.folder.davLabel": "Adres van de WebDAV-map",
  "mobile.folder.loginLabel": "Gebruikersnaam",
  "mobile.folder.appPasswordLabel": "App-wachtwoord",
  "mobile.folder.appPasswordHint":
    "Maak een app-wachtwoord aan in de beveiligingsinstellingen van je server. Gebruik niet je hoofdwachtwoord van de server.",
  "mobile.folder.appPasswordAction": "Inloggen met het app-wachtwoord",
  "mobile.folder.pickAccount": "Bij welk account sluit deze telefoon aan?",
  "mobile.folder.accountOption": "Account {id}",
  "mobile.folder.linked": "Ingelogd. Er wordt weer gesynchroniseerd.",
  "mobile.folder.settingsLine": "Synchroniseert via {provider}: {path}",
  "mobile.folder.relink": "Opnieuw inloggen",
  "mobile.folder.msRevokeHint":
    "Om de toegang van Mosaic helemaal te verwijderen, verwijder je Mosaic op de pagina van je Microsoft-account onder Apps.",
  "mobile.reset.revokeDropbox":
    "Het account is verwijderd. We konden de toegang van Mosaic bij Dropbox niet beëindigen: verwijder die in Dropbox onder Instellingen, Verbonden apps.",
  "mobile.reset.revokeWebdav":
    "Het account is verwijderd. We konden het app-wachtwoord op je server niet verwijderen: verwijder het daar onder Instellingen, Beveiliging, Apparaten en sessies.",
  "mobile.reset.title": "Dit account van deze telefoon verwijderen",
  "mobile.reset.body":
    "Dit verwijdert de kluiskopie, de Geheime Sleutel en de cloudaanmelding die op deze telefoon staan. Je gegevens in de cloudmap of op de server blijven bestaan. Je kunt later weer aansluiten met je Geheime Sleutel en hoofdwachtwoord.",
  "mobile.reset.action": "Account verwijderen…",
  "mobile.reset.confirm": "Account verwijderen",
  "mobile.reset.phraseLabel": "Bevestiging",
  "mobile.reset.typePhrase": "Typ {phrase} om te bevestigen",
} satisfies Record<keyof typeof en, string>;
