/**
 * Dutch UI strings: sync. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { sync as en } from "../en/sync.ts";

export const sync = {
  "sync.status.signedOut": "Uitgelogd",
  "sync.platform.windows": "Windows",
  "sync.platform.macos": "macOS",
  "sync.platform.linux": "Linux",
  "settings.sync.title": "Synchronisatie",
  "settings.sync.body":
    "Houd je kluizen up-to-date op al je apparaten. Alles wordt op dit apparaat versleuteld voordat het het apparaat verlaat.",
  "settings.sync.notSetUp":
    "Synchronisatie staat uit. Je kluizen staan alleen op dit apparaat.",
  "settings.sync.deviceName": "Naam van dit apparaat",
  "settings.sync.deviceName.default": "Mosaic op {platform}",
  "settings.sync.password": "Hoofdwachtwoord",
  "settings.sync.code": "6-cijferige code",
  "settings.sync.server.title": "Mosaic-server",
  "settings.sync.server.body":
    "Synchroniseer via een Mosaic-server die jij of iemand die je vertrouwt beheert.",
  "settings.sync.server.url": "Serveradres",
  "settings.sync.server.urlHint": "Bijvoorbeeld https://sync.example.com",
  "settings.sync.server.check": "Server controleren",
  "settings.sync.server.checking": "De server wordt gecontroleerd…",
  "settings.sync.server.registration.open":
    "Deze server laat iedereen een synchronisatieaccount aanmaken.",
  "settings.sync.server.registration.invite":
    "Deze server heeft een uitnodigingscode nodig om een synchronisatieaccount aan te maken.",
  "settings.sync.server.registration.closed":
    "Deze server accepteert geen nieuwe synchronisatieaccounts. Je kunt nog wel inloggen op een bestaand account.",
  "settings.sync.server.choice.label": "Synchronisatieaccount",
  "settings.sync.server.choice.signIn": "Ik heb een synchronisatieaccount",
  "settings.sync.server.choice.create": "Een account aanmaken",
  "settings.sync.server.invite": "Uitnodigingscode",
  "settings.sync.server.create": "Synchronisatieaccount aanmaken",
  "settings.sync.server.signIn": "Inloggen",
  "settings.sync.server.signInAgain": "Opnieuw inloggen",
  "settings.sync.server.working": "Verbinden…",
  "settings.sync.server.sees":
    "De server ziet alleen versleutelde gegevens, het e-mailadres van je account en wanneer je synchroniseert. Hij ziet nooit je hoofdwachtwoord of je Geheime Sleutel.",
  "settings.sync.twoFactor.title": "Voer je code in",
  "settings.sync.twoFactor.body":
    "Dit synchronisatieaccount gebruikt tweestapsverificatie. Voer de 6-cijferige code uit je authenticator-app in.",
  "settings.sync.twoFactor.submit": "Verifiëren",
  "settings.sync.folder.title": "Je eigen cloudmap",
  "settings.sync.folder.body":
    "Synchroniseer via een map die je cloudservice (Dropbox, OneDrive, iCloud Drive en andere) up-to-date houdt.",
  "settings.sync.folder.choose": "Map kiezen…",
  "settings.sync.folder.chooseOther": "Een andere map kiezen…",
  "settings.sync.folder.path": "Map",
  "settings.sync.folder.contents.empty": "De map is leeg. Mosaic kan hem gebruiken.",
  "settings.sync.folder.contents.mosaic": "Deze map bevat al Mosaic-synchronisatiegegevens.",
  "settings.sync.folder.contents.other":
    "Deze map bevat andere bestanden. Mosaic schrijft daar niets. Kies een lege map of maak een nieuwe.",
  "settings.sync.folder.connect": "Map verbinden",
  "settings.sync.folder.working": "Verbinden…",
  "settings.sync.folder.sees":
    "Je cloudaanbieder en iedereen met toegang tot de map ziet alleen versleutelde gegevens, de account-ID en wanneer bestanden veranderen. Ze zien nooit je hoofdwachtwoord of je Geheime Sleutel.",
  "settings.sync.status.label": "Status",
  "settings.sync.lastSynced": "Laatst gesynchroniseerd {when}",
  "settings.sync.neverSynced": "Nog niet gesynchroniseerd",
  "settings.sync.offline":
    "Je bent offline. Je wijzigingen zijn op dit apparaat opgeslagen en worden gesynchroniseerd zodra je weer online bent.",
  "settings.sync.now": "Nu synchroniseren",
  "settings.sync.backend.server": "Server",
  "settings.sync.backend.account": "Synchronisatieaccount",
  "settings.sync.backend.folder": "Map",
  "settings.sync.backend.device": "Dit apparaat",
  "settings.sync.signedOut.title": "Dit apparaat is uitgelogd",
  "settings.sync.signedOut.body":
    "De server heeft de sessie van dit apparaat beëindigd. Log opnieuw in om te blijven synchroniseren. Je wijzigingen zijn veilig op dit apparaat.",
  "settings.sync.recover.title": "Je hoofdwachtwoord is gewijzigd",
  "settings.sync.recover.body":
    "De server of de map heeft een nieuwer hoofdwachtwoord dan dit apparaat. Voer het nieuwe hoofdwachtwoord in om dit apparaat bij te werken. Je kluizen en je Geheime Sleutel blijven hetzelfde.",
  "settings.sync.recover.password": "Nieuw hoofdwachtwoord",
  "settings.sync.recover.submit": "Dit apparaat bijwerken",
  "settings.sync.disconnect": "Verbinding verbreken…",
  "settings.sync.disconnect.title": "Synchronisatie loskoppelen?",
  "settings.sync.disconnect.body":
    "Dit apparaat stopt met synchroniseren en logt uit bij de server. Je kluizen blijven op dit apparaat staan en je andere apparaten houden hun eigen kopieën.",
  "settings.sync.disconnect.confirm": "Verbinding verbreken",
  "settings.sync.devices.title": "Apparaten",
  "settings.sync.devices.thisDevice": "Dit apparaat",
  "settings.sync.devices.unnamed": "Apparaat zonder naam",
  "settings.sync.devices.lastSeen": "Laatst gezien {when}",
  "settings.sync.devices.neverSeen": "Nog niet gezien",
  "settings.sync.devices.rename": "Hernoemen",
  "settings.sync.devices.renameNamed": "{name} hernoemen",
  "settings.sync.devices.renameLabel": "Nieuwe naam voor dit apparaat",
  "settings.sync.devices.remove": "Verwijderen",
  "settings.sync.devices.removeNamed": "{name} verwijderen",
  "settings.sync.devices.removeTitle": "{name} verwijderen?",
  "settings.sync.devices.removeBody":
    "Een apparaat verwijderen logt het uit de synchronisatie uit. Het neemt de kopie van je kluizen die het al heeft niet weg. Om een apparaat definitief buiten te sluiten, wijzig je je hoofdwachtwoord en Geheime Sleutel.",
  "settings.sync.devices.removeConfirm": "Apparaat verwijderen",
  "settings.sync.devices.folderNote":
    "Om een apparaat buiten te sluiten, wijzig je je hoofdwachtwoord en Geheime Sleutel.",
  "settings.sync.phone.title": "Een telefoon instellen",
  "settings.sync.phone.body":
    "Toon een code die je telefoon scant, zodat je je Geheime Sleutel niet hoeft te typen. De telefoon heeft nog steeds je hoofdwachtwoord nodig.",
  "settings.sync.phone.button": "Een telefoon instellen",
  "settings.sync.phone.reauth":
    "De code bevat je Geheime Sleutel, dus je hoofdwachtwoord is nodig.",
  "settings.sync.phone.dialogTitle": "Scan deze code met je telefoon",
  "settings.sync.phone.steps":
    "Open Mosaic op je telefoon, kies om het in te stellen en scan dan deze code.",
  "settings.sync.phone.hint":
    "De code wordt na 60 seconden verborgen, of wanneer je dit venster verlaat. Deel er geen foto van.",
  "settings.sync.phone.canvas": "Installatiecode voor je telefoon",
  "settings.sync.phone.hidden.time": "De 60 seconden zijn voorbij, dus de code is verborgen.",
  "settings.sync.phone.hidden.away": "Je hebt dit venster verlaten, dus de code is verborgen.",
  "settings.sync.phone.again": "Opnieuw tonen",
  "settings.sync.phone.folderHint":
    "Je telefoon logt in bij je cloudopslag en opent deze zelfde map.",
  "settings.sync.totp.title": "Authenticator-app",
  "settings.sync.totp.body":
    "Vraagt om een code uit een authenticator-app wanneer iemand inlogt op je synchronisatieaccount. Het beschermt het serveraccount, niet je kluizen: die blijven versleuteld met je hoofdwachtwoord en Geheime Sleutel.",
  "settings.sync.totp.on": "Codes uit een authenticator-app staan aan.",
  "settings.sync.totp.off": "Codes uit een authenticator-app staan uit.",
  "settings.sync.totp.enable": "Aanzetten…",
  "settings.sync.totp.disable": "Uitzetten…",
  "settings.sync.totp.beginTitle": "Codes uit een authenticator-app aanzetten",
  "settings.sync.totp.beginBody": "Voer je hoofdwachtwoord in om te beginnen.",
  "settings.sync.totp.begin": "Doorgaan",
  "settings.sync.totp.setupBody":
    "Voeg deze sleutel toe aan je authenticator-app en voer dan de 6-cijferige code in die de app toont.",
  "settings.sync.totp.secret": "Installatiesleutel",
  "settings.sync.totp.confirm": "Aanzetten",
  "settings.sync.totp.disableTitle": "Codes uit een authenticator-app uitzetten",
  "settings.sync.totp.disableBody": "Voer je hoofdwachtwoord en een huidige code in.",
  "settings.sync.totp.disableConfirm": "Uitzetten",
  "settings.sync.totp.enabledToast": "Codes uit een authenticator-app staan aan.",
  "settings.sync.totp.disabledToast": "Codes uit een authenticator-app staan uit.",
  "settings.sync.key.title": "Beveiligingssleutels",
  "settings.sync.key.body":
    "Een beveiligingssleutel kan een tweede stap zijn wanneer je inlogt op je synchronisatieaccount. Hij beschermt het account op de server, niet je kluizen. Bij het toevoegen of gebruiken van een sleutel opent je browser.",
  "settings.sync.key.none": "Geen beveiligingssleutels toegevoegd.",
  "settings.sync.key.added": "Toegevoegd {when}",
  "settings.sync.key.lastUsed": "Laatst gebruikt {when}",
  "settings.sync.key.neverUsed": "Nog niet gebruikt",
  "settings.sync.key.add": "Een beveiligingssleutel toevoegen…",
  "settings.sync.key.addTitle": "Een beveiligingssleutel toevoegen",
  "settings.sync.key.addBody":
    "Geef de sleutel een naam en voer je hoofdwachtwoord in. Mosaic opent dan je browser, waar je de sleutel gebruikt.",
  "settings.sync.key.label": "Naam van de sleutel",
  "settings.sync.key.labelHint": "Bijvoorbeeld “Bureausleutel”. Maximaal 64 tekens.",
  "settings.sync.key.addConfirm": "Doorgaan",
  "settings.sync.key.addedToast": "Beveiligingssleutel toegevoegd.",
  "settings.sync.key.noBackup":
    "Een beveiligingssleutel zou je enige tweede stap zijn. Raak je hem kwijt, dan kun je mogelijk niet meer inloggen op je synchronisatieaccount. We raden een reserve aan: een tweede sleutel of een code uit een authenticator-app.",
  "settings.sync.key.recommend":
    "We raden twee methoden aan, bijvoorbeeld twee sleutels, of een sleutel en een code uit een authenticator-app.",
  "settings.sync.key.remove": "Verwijderen",
  "settings.sync.key.removeLabel": "{name} verwijderen",
  "settings.sync.key.removeTitle": "{name} verwijderen?",
  "settings.sync.key.removeBody":
    "Je bevestigt eenmalig met een beveiligingssleutel in je browser. Daarna werkt deze sleutel niet meer om in te loggen.",
  "settings.sync.key.removeLast":
    "Dit is je laatste tweede stap. Tweestapsverificatie gaat dan uit en je synchronisatieaccount wordt daarna alleen beschermd door je hoofdwachtwoord en Geheime Sleutel.",
  "settings.sync.key.removeCode": "Code uit een authenticator-app",
  "settings.sync.key.removeCodeHint":
    "Voer een huidige code in om de sleutel meteen te verwijderen. Laat leeg om in plaats daarvan te bevestigen met een beveiligingssleutel in je browser.",
  "settings.sync.key.removeConfirm": "Sleutel verwijderen",
  "settings.sync.key.removedToast": "Beveiligingssleutel verwijderd.",
  "settings.sync.key.confirmTitle": "Bevestig met een beveiligingssleutel",
  "settings.sync.key.confirmBody":
    "Voor het verwijderen van een sleutel is een tweede stap nodig. Voer je hoofdwachtwoord in en bevestig dan met een sleutel in je browser.",
  "settings.sync.key.step.methods": "Er wordt gecontroleerd hoe dit account inlogt…",
  "settings.sync.key.step.useKey": "Beveiligingssleutel gebruiken (opent je browser)",
  "settings.sync.key.step.keyOnlyTitle": "Gebruik je beveiligingssleutel",
  "settings.sync.key.step.keyOnlyBody":
    "Dit account is beschermd met een beveiligingssleutel. Mosaic opent een pagina in je browser, waar je hem gebruikt.",
  "settings.sync.key.step.or": "Of gebruik een code",
  "settings.sync.key.wait.login": "Log in met je beveiligingssleutel",
  "settings.sync.key.wait.register": "Voeg je beveiligingssleutel toe",
  "settings.sync.key.wait.confirm": "Bevestig met je beveiligingssleutel",
  "settings.sync.key.wait.lead":
    "Je browser heeft een Mosaic-pagina van je synchronisatieserver geopend. Typ daar deze code en gebruik dan je beveiligingssleutel.",
  "settings.sync.key.wait.codeLabel": "Code om in je browser te typen",
  "settings.sync.key.wait.codeSpoken": "Code om in je browser te typen: {digits}",
  "settings.sync.key.wait.expires": "Deze stap verloopt om {time}.",
  "settings.sync.key.wait.waiting": "Wachten op je beveiligingssleutel…",
  "settings.sync.key.wait.done": "Klaar.",
  "settings.sync.key.wait.expired": "Deze stap is verlopen. Begin opnieuw.",
  "settings.sync.key.wait.failed":
    "De stap is mislukt: de code was te vaak fout, of de beveiligingssleutel is niet geaccepteerd. Er is niets gewijzigd. Begin opnieuw.",
  "settings.sync.key.wait.reopen": "De browserpagina opnieuw openen",
  "settings.sync.key.wait.retry": "Opnieuw beginnen",
  "sync.warning.region": "Synchronisatiewaarschuwingen",
  "sync.warning.unknownDevice": "een ander apparaat",
  "sync.warning.unknownVault": "een van je kluizen",
  "sync.warning.freezeSuspected":
    "Wijzigingen van {device} zijn al meer dan een dag niet aangekomen. Controleer of het andere apparaat aan staat en online is.",
  "sync.warning.missingFiles":
    "Bestanden van {device} ontbreken in de map. Laat je cloudservice klaar zijn met synchroniseren en synchroniseer dan opnieuw.",
  "sync.warning.noCommonView":
    "Dit apparaat en {device} hebben twee dagen lang niet dezelfde servergegevens gezien. De server toont ze mogelijk verschillende versies.",
  "sync.warning.rollbackDetected":
    "Je synchronisatiegegevens lijken ouder dan dit apparaat. Controleer de server of map en kies dan “Ik heb het gecontroleerd”.",
  "sync.warning.identityDuplicated":
    "Een andere kopie van Mosaic schrijft onder de identiteit van dit apparaat, bijvoorbeeld nadat een back-up is teruggezet. Synchronisatie is gepauzeerd. Zorg dat er maar één kopie in gebruik is en kies dan “Ik heb het gecontroleerd”.",
  "sync.warning.serverInconsistent":
    "De server toonde dit apparaat andere gegevens dan een ander apparaat. Controleer de server en kies dan “Ik heb het gecontroleerd”.",
  "sync.warning.serverRollback":
    "De kopie die de server van {vault} heeft, is ouder dan die van dit apparaat. Controleer de server en kies dan “Ik heb het gecontroleerd”.",
  "sync.warning.undecryptable":
    "{count, plural, one {# ontvangen item kon} other {# ontvangen items konden}} op dit apparaat niet worden gelezen. Werk Mosaic bij op al je apparaten en synchroniseer opnieuw.",
  "sync.warning.hardwareVaultsLocal":
    "Met hardware beveiligde kluizen blijven op dit apparaat. {count, plural, one {# kluis wordt} other {# kluizen worden}} niet gesynchroniseerd.",
  "sync.warning.attachmentsNotRepaired":
    "{count, plural, one {# bestand} other {# bestanden}} op de server of map kon vanaf dit apparaat niet worden hersteld. Open het item op een apparaat dat het bestand nog heeft en voeg het opnieuw toe.",
  "sync.warning.acknowledge": "Ik heb het gecontroleerd…",
  "sync.acknowledge.title": "De synchronisatiewaarschuwingen die een controle vragen wissen?",
  "sync.acknowledge.body":
    "Ga alleen verder nadat je de server of map zelf hebt gecontroleerd. Dit wist elke waarschuwing die een controle vraagt en de synchronisatie gaat door. Als je gegevens echt zijn teruggedraaid of gewijzigd, kan Mosaic dat achteraf niet meer zien.",
  "sync.acknowledge.eachDevice": "Doe dit op elk van je apparaten.",
  "toast.syncAttention":
    "Synchronisatie heeft enkele items van een ander apparaat samengevoegd of hersteld. Bekijk wat er is gewijzigd.",
  "sync.revoked.rekey":
    "De sleutels van dit account zijn op een ander apparaat vernieuwd: log in met de nieuwe Geheime Sleutel uit je nieuwste Noodkit.",
  "sync.revoked.passwordChange":
    "Je hoofdwachtwoord is op een ander apparaat gewijzigd: log in met het nieuwe wachtwoord.",
  "sync.revoked.deviceRemoved":
    "Dit apparaat is op een ander apparaat uit je account verwijderd. Log opnieuw in om het weer toe te voegen.",
  "sync.revoked.operator":
    "De beheerder van de server heeft deze sessie beëindigd. Log opnieuw in om te blijven synchroniseren.",
} satisfies Record<keyof typeof en, string>;
