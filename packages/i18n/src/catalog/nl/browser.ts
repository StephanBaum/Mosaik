/**
 * Dutch UI strings: browser. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { browser as en } from "../en/browser.ts";

export const browser = {
  "browser.pair.title": "Een browser wil verbinden met Mosaic",
  "browser.pair.description":
    "Sta dit alleen toe als je zojuist in de Mosaic-extensie op Verbinden hebt geklikt en die deze code toont.",
  "browser.pair.code": "Code",
  "browser.pair.browser": "Browser",
  "browser.pair.label": "Naam voor deze browser",
  "browser.pair.labelHint": "Je ziet deze naam in de instellingen.",
  "browser.pair.deny": "Weigeren",
  "browser.pair.allow": "Toestaan",
  "browser.expires": "Sluit automatisch over {seconds, plural, one {# seconde} other {# seconden}}",
  "browser.escDenies": "om te weigeren",
  "browser.hint.chrome": "Chrome",
  "browser.hint.edge": "Edge",
  "browser.hint.brave": "Brave",
  "browser.hint.opera": "Opera",
  "browser.hint.vivaldi": "Vivaldi",
  "browser.hint.chromium": "Chromium",
  "browser.hint.firefox": "Firefox",
  "browser.hint.other": "Browser",
  "browser.family.chromium": "Op Chromium gebaseerde browser",
  "browser.family.firefox": "Firefox",
  "browser.escCancels": "om te annuleren",
  "browser.save.titleNew": "Deze login in Mosaic bewaren?",
  "browser.save.titleUpdate": "Deze login in Mosaic bijwerken?",
  "browser.save.description":
    "{browser} stelt voor een login te bewaren. Mosaic schrijft die alleen weg als je Bewaren kiest.",
  "browser.save.descriptionUpdate":
    "{browser} stelt een nieuw wachtwoord voor deze login voor. Mosaic wijzigt het alleen als je Wachtwoord bijwerken kiest.",
  "browser.save.site": "Website",
  "browser.save.username": "Gebruikersnaam",
  "browser.save.noUsername": "Geen",
  "browser.save.password": "Wachtwoord",
  "browser.save.passwordHidden": "Verborgen",
  "browser.save.target": "Bewaren als",
  "browser.save.updateOption": "“{title}” bijwerken",
  "browser.save.newOption": "Een nieuwe login",
  "browser.save.itemTitle": "Titel",
  "browser.save.vault": "Kluis",
  "browser.save.allowHttp": "Invullen op deze onbeveiligde pagina toestaan",
  "browser.save.allowHttpHint":
    "De site gebruikt geen HTTPS. Iedereen op het netwerk kan lezen wat je invult.",
  "browser.save.cancel": "Annuleren",
  "browser.save.saveNew": "Bewaren",
  "browser.save.update": "Wachtwoord bijwerken",
  "browser.passkey.secure": "Beveiligde verbinding (https)",
  "browser.passkey.getTitle": "Met een passkey inloggen bij {rpId}",
  "browser.passkey.createTitle": "Een nieuwe passkey bewaren",
  "browser.passkey.excludedTitle": "Passkey al bewaard",
  "browser.passkey.createBody": "Een passkey voor {rpId} aanmaken in Mosaic?",
  "browser.passkey.excludedBody": "Je hebt al een passkey voor {rpId} in Mosaic.",
  "browser.passkey.uvNote":
    "De site vroeg om gebruikersverificatie. Voer je hoofdwachtwoord in om te bevestigen dat jij het bent.",
  "browser.passkey.password": "Hoofdwachtwoord",
  "browser.passkey.account": "Account",
  "browser.passkey.noName": "Geen naam",
  "browser.passkey.vault": "Kluis",
  "browser.passkey.signIn": "Inloggen met passkey",
  "browser.passkey.save": "Passkey bewaren",
  "browser.passkey.anotherDevice": "Een ander apparaat of beveiligingssleutel gebruiken",
  "browser.passkey.cancel": "Annuleren",
  "browser.passkey.ok": "Begrepen",
  "browser.passkey.escCloses": "om te sluiten",
  "browser.passkey.passwordNeeded":
    "Voer je hoofdwachtwoord in om te bevestigen dat jij het bent.",
  "browser.settings.title": "Browsers",
  "browser.settings.body":
    "Browserextensies die logins uit deze app invullen. Je keurt elke verbinding hier goed.",
  "browser.settings.empty":
    "Er is nog geen browser verbonden. Installeer de Mosaic-extensie en kies daarin Verbinden.",
  "browser.settings.connected": "Nu verbonden",
  "browser.settings.lastUsed": "Laatst gebruikt {when}",
  "browser.settings.neverUsed": "Nog niet gebruikt",
  "browser.settings.disconnect": "Verbinding verbreken",
  "browser.settings.disconnectNamed": "Verbinding met {name} verbreken",
  "browser.settings.disconnectTitle": "Verbinding met “{name}” verbreken?",
  "browser.settings.disconnectBody":
    "Deze browser kan geen logins invullen totdat je hem opnieuw verbindt vanuit de extensie.",
  "browser.settings.endpointInUse":
    "Een ander programma of een ander Mosaic-venster gebruikt de verbinding voor browsers al. Sluit dat af en start Mosaic opnieuw.",
  "browser.settings.endpointUnavailable":
    "Mosaic kon de verbinding voor browsers niet openen. Start Mosaic opnieuw. Gebeurt dit steeds weer, controleer dan of je gebruikersmap beschrijfbaar is.",
  "browser.settings.registryUnreadable":
    "Bewaarde browserverbindingen kunnen niet worden gelezen. Mosaic wijzigt ze niet totdat dit is opgelost.",
  "browser.connector.title": "Een browserverbinding wijst ergens anders naartoe",
  "browser.connector.body":
    "De verbinding voor {browser} start een ander programma in plaats van Mosaic: {path}. Het kan een oudere Mosaic zijn, of iets anders dat zichzelf heeft geregistreerd. Met Herstellen wijst hij weer naar deze Mosaic.",
  "browser.connector.item": "{browser}: {path}",
  "browser.connector.repair": "Herstellen",
  "browser.connector.repaired": "Browserverbinding hersteld.",
} satisfies Record<keyof typeof en, string>;
