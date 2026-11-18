/**
 * Dutch UI strings: mobile. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { mobile as en } from "../en/mobile.ts";

export const mobile = {
  "mobile.app.name": "Mosaic",
  "mobile.nav.vault": "Kluis",
  "mobile.nav.search": "Zoeken",
  "mobile.nav.settings": "Instellingen",
  "mobile.common.cancel": "Annuleren",
  "mobile.common.done": "Klaar",
  "mobile.common.close": "Sluiten",
  "mobile.common.continue": "Doorgaan",
  "mobile.unlock.title": "Mosaic ontgrendelen",
  "mobile.unlock.passwordLabel": "Hoofdwachtwoord",
  "mobile.unlock.action": "Ontgrendelen",
  "mobile.unlock.quick": "Snel ontgrendelen gebruiken",
  "mobile.unlock.failed":
    "{attempts, plural, one {Dat werkte # keer niet. Mosaic kan je hoofdwachtwoord niet resetten. Als je het op een ander apparaat hebt gewijzigd, gebruik dan het nieuwe.} other {Dat werkte # keer niet. Mosaic kan je hoofdwachtwoord niet resetten. Als je het op een ander apparaat hebt gewijzigd, gebruik dan het nieuwe.}}",
  "mobile.lock.now": "Nu vergrendelen",
  "mobile.vault.title": "Je kluis",
  "mobile.vault.count": "{count, plural, one {# item} other {# items}}",
  "mobile.vault.emptyTitle": "Nog geen items",
  "mobile.vault.emptyBody":
    "Trek omlaag om te synchroniseren met je andere apparaten, of voeg je eerste login toe.",
  "mobile.search.placeholder": "Zoeken in je kluis",
  "mobile.copy.done":
    "Gekopieerd. Het wordt over {seconds, plural, one {# seconde} other {# seconden}} van het klembord gewist.",
  "mobile.setup.scanTitle": "Scan je installatiecode",
  "mobile.setup.scanHint":
    "Open Mosaic op je computer, ga naar Instellingen > Synchronisatie en kies “Een telefoon instellen”. Houd de camera boven de code.",
  "mobile.autofill.title": "Invullen vanuit Mosaic",
  "mobile.autofill.target": "Invullen in {app}?",
  "mobile.settings.title": "Instellingen",
  "mobile.settings.about": "Over Mosaic",
  "mobile.settings.version": "Versie {version}",
  "mobile.common.back": "Terug",
  "mobile.common.notNow": "Niet nu",
  "mobile.onboarding.step": "Stap {current} van {total}",
  "mobile.onboarding.welcome.title": "Mosaic instellen op deze telefoon",
  "mobile.onboarding.welcome.body":
    "Log in op je Mosaic-account om je wachtwoorden hierheen te halen. Alles blijft versleuteld op deze telefoon.",
  "mobile.onboarding.welcome.start": "Aan de slag",
  "mobile.onboarding.details.title": "Je account",
  "mobile.onboarding.details.server": "Serveradres",
  "mobile.onboarding.details.serverHint": "https://sync.example.com",
  "mobile.onboarding.details.email": "E-mail",
  "mobile.onboarding.details.secretKey": "Geheime Sleutel",
  "mobile.onboarding.details.secretKeyHelp":
    "Uit je Noodkit: letters en cijfers in groepen, beginnend met M1.",
  "mobile.onboarding.details.deviceName": "Naam van deze telefoon",
  "mobile.onboarding.details.deviceNameDefault": "Android-telefoon",
  "mobile.onboarding.details.checking": "De server wordt gecontroleerd",
  "mobile.onboarding.password.title": "Je hoofdwachtwoord",
  "mobile.onboarding.password.body":
    "Mosaic verstuurt het nergens naartoe. Het ontgrendelt je kluis op deze telefoon.",
  "mobile.onboarding.password.action": "Inloggen",
  "mobile.onboarding.password.working": "Inloggen. Dit kan een paar seconden duren.",
  "mobile.onboarding.second.title": "Nog één stap",
  "mobile.onboarding.second.totpLabel": "6-cijferige code",
  "mobile.onboarding.second.totpAction": "Verifiëren",
  "mobile.onboarding.second.keyAction": "Gebruik je beveiligingssleutel",
  "mobile.onboarding.second.keyWaiting":
    "Wachten op je beveiligingssleutel. Rond het af in de browser en kom dan terug. De code daar moet {code} zijn.",
  "mobile.onboarding.quick.title": "Sneller ontgrendelen",
  "mobile.onboarding.quick.body":
    "Gebruik je vingerafdruk om Mosaic de volgende keer te ontgrendelen. Na een herstart van de telefoon heb je je hoofdwachtwoord nog één keer nodig.",
  "mobile.onboarding.quick.enable": "Snel ontgrendelen aanzetten",
  "mobile.onboarding.quick.unavailable":
    "Voeg een vingerafdruk toe in de instellingen van je telefoon om snel ontgrendelen te gebruiken.",
  "mobile.onboarding.autofill.title": "Logins automatisch invullen",
  "mobile.onboarding.autofill.body":
    "Laat Mosaic je logins aanbieden in andere apps en in de browser. Je kunt dit later wijzigen in de systeeminstellingen.",
  "mobile.onboarding.autofill.open": "Instellingen voor automatisch invullen openen",
  "mobile.onboarding.done.title": "Alles is klaar",
  "mobile.onboarding.done.body": "Je kluis staat op deze telefoon en is ontgrendeld.",
  "mobile.onboarding.done.action": "Je kluis openen",
  "mobile.unlock.account": "{email} ontgrendelen",
  "mobile.unlock.secretKey": "Geheime Sleutel",
  "mobile.unlock.secretKeyHelp":
    "Deze telefoon heeft je Geheime Sleutel niet opgeslagen. Typ hem over van je Noodkit.",
  "mobile.unlock.enrollNext": "De volgende keer ontgrendelen met mijn vingerafdruk",
  "mobile.unlock.working": "Ontgrendelen. Dit kan een paar seconden duren.",
  "mobile.unlock.biometricTitle": "Mosaic ontgrendelen",
  "mobile.unlock.biometricSubtitle": "Bevestig dat jij het bent",
  "mobile.unlock.usePassword": "Hoofdwachtwoord gebruiken",
  "mobile.unlock.busyTitle": "Mosaic is bezig",
  "mobile.unlock.retry": "Opnieuw proberen",
  "mobile.home.unlockedAs": "Ontgrendeld als {email}",
  "error.notFound":
    "Dat item bestaat niet meer. Het is mogelijk op een ander apparaat verwijderd.",
  "error.syncFailed":
    "Mosaic kon niet synchroniseren. Controleer je verbinding en probeer het opnieuw.",
  "error.passwordInApp":
    "Ontgrendelen met het hoofdwachtwoord kan hier niet. Open de Mosaic-app en ontgrendel daar.",
  "error.folderProviderUnsupported":
    "Deze telefoon kan de cloudmap van dit account niet bereiken. Log opnieuw in bij de aanbieder van de map, of gebruik een desktop.",
  "error.folderProviderSignedOut":
    "Je cloudservice heeft de aanmelding beëindigd of geweigerd. Log opnieuw in om te blijven synchroniseren.",
  "error.folderNotFound":
    "Mosaic kan die map niet vinden, of het is geen Mosaic-map. Kies de map waarmee je computer synchroniseert.",
  "error.folderStorageFull":
    "Je cloudopslag is vol. Maak daar ruimte vrij en synchroniseer opnieuw.",
  "error.folderLinkExpired": "Het inloggen duurde te lang. Begin opnieuw.",
  "error.folderLinkInvalid": "Het inloggen werkte niet of is geannuleerd. Begin opnieuw.",
  "error.folderServerTlsTooOld":
    "Deze server is te oud voor een beveiligde verbinding. Mosaic heeft TLS 1.3 nodig. Vraag degene die de server beheert om hem bij te werken.",
  "mobile.error.syncPasswordRollback":
    "De map biedt een ouder hoofdwachtwoord aan dan deze telefoon kent. Controleer de map en je cloudservice.",
  "error.setupCodeInvalid":
    "Die installatiecode is ongeldig of verlopen. Maak op je computer een nieuwe aan en scan hem opnieuw.",
  "error.notEnoughMemory":
    "Dit apparaat heeft niet genoeg vrij geheugen om je kluis te ontgrendelen. Sluit andere apps en probeer het opnieuw.",
  "error.noMatch":
    "Mosaic heeft geen login voor deze app of website. Open Mosaic om er een toe te voegen.",
  "error.insecureOrigin":
    "Deze pagina gebruikt geen beveiligde verbinding, dus Mosaic vult hem niet in. Open de https-versie van de pagina.",
  "error.appNotLinked":
    "Deze app is niet gekoppeld aan een bewaarde website. Koppel hem eerst in Mosaic en probeer het opnieuw.",
  "error.foreignFrame":
    "Dit veld hoort bij een andere site die in de pagina is ingebed, dus Mosaic vult het niet in.",
  "error.assetLinksUnavailable":
    "Mosaic kon niet controleren of deze app bij de website hoort. Controleer je verbinding en probeer het opnieuw.",
  "error.appNotVerified":
    "Deze app is niet geverifieerd voor die website, dus Mosaic gebruikt zijn passkeys niet. Gebruik in plaats daarvan de website in een browser.",
  "error.rpIdInvalid":
    "De websitenaam in dit verzoek is ongeldig. Annuleer en begin opnieuw.",
  "error.passkeyNotFound": "Mosaic heeft geen passkey voor deze website. Log op een andere manier in.",
  "error.passkeyExcluded":
    "Je hebt al een passkey voor dit account in Mosaic. Log in plaats daarvan daarmee in.",
  "error.passkeyUnsupportedAlgorithm":
    "Deze website heeft een passkeytype nodig dat Mosaic nog niet kan maken. Kies een andere manier om in te loggen.",
  "error.passkeySessionExpired":
    "Dit passkeyverzoek is verlopen. Begin opnieuw vanuit de app.",
  "error.qrNotFound":
    "Mosaic vond geen QR-code in die afbeelding. Probeer het opnieuw met een scherpere foto.",
  "error.storageUnavailable":
    "Mosaic kon de beveiligde opslag van dit apparaat niet bereiken. Herstart je telefoon en probeer het opnieuw.",
  "mobile.common.tryAgain": "Opnieuw proberen",
  "mobile.onboarding.preAudit":
    "Software vóór een audit: nog geen beveiligingsbeoordeling door een buitenstaander.",
  "mobile.onboarding.start": "Aan de slag",
  "mobile.onboarding.server": "Serveradres",
  "mobile.onboarding.serverPlaceholder": "https://",
  "mobile.onboarding.email": "E-mail",
  "mobile.onboarding.secretKey": "Geheime Sleutel",
  "mobile.onboarding.secretKeyHint":
    "Je vindt hem in je Noodkit of in de instellingen op je computer. Hoofdletters en streepjes maken niet uit.",
  "mobile.onboarding.deviceName": "Naam van deze telefoon",
  "mobile.onboarding.passwordHint":
    "Mosaic controleert het op deze telefoon. Het wordt nooit naar de server gestuurd.",
  "mobile.onboarding.signIn": "Inloggen",
  "mobile.onboarding.working": "Inloggen. Dit kan een paar seconden duren.",
  "mobile.onboarding.secondFactorTitle": "Nog één stap",
  "mobile.onboarding.totpLabel": "Code uit je authenticator-app",
  "mobile.onboarding.verify": "Verifiëren",
  "mobile.onboarding.securityKey": "Gebruik je beveiligingssleutel",
  "mobile.onboarding.securityKeyWaiting":
    "Rond het inloggen af in je browser. Controleer of de code {code} wordt getoond en kom dan hier terug.",
  "mobile.onboarding.autofillSteps":
    "Open de app Instellingen, tik op Wachtwoorden, dan op Wachtwoordopties en zet Mosaic aan.",
  "mobile.onboarding.openSettings": "Instellingen openen",
  "mobile.onboarding.notNow": "Niet nu",
  "mobile.unlock.secretKeyNeeded":
    "Deze telefoon heeft je Geheime Sleutel niet. Voer hem eenmalig in om te ontgrendelen.",
  "mobile.unlock.quickOptionFaceId": "De volgende keer ontgrendelen met Face ID",
  "mobile.unlock.biometricReason": "Je Mosaic-kluis ontgrendelen",
  "mobile.unlock.faceIdReason":
    "Mosaic gebruikt Face ID om je kluis snel te ontgrendelen. Je hoofdwachtwoord werkt altijd ook.",
  "mobile.unlock.quickInvalidated":
    "Je Face ID-instellingen zijn gewijzigd. Ontgrendel met je hoofdwachtwoord om snel ontgrendelen weer aan te zetten.",
  "mobile.unlock.storeBusy": "Mosaic is bezig. Probeer het zo meteen opnieuw.",
  "mobile.unlock.rebootNote": "Ontgrendel na een herstart eenmalig met je hoofdwachtwoord.",
  "mobile.capture.hidden": "Verborgen terwijl je scherm wordt opgenomen",
  "mobile.deviceLock.required.title": "Stel eerst een schermvergrendeling in",
  "mobile.deviceLock.required.body":
    "Mosaic bewaart je sleutels achter de schermvergrendeling van je telefoon. Stel in de systeeminstellingen een pincode, patroon of wachtwoord in en kom dan terug.",
  "mobile.deviceLock.required.action": "Beveiligingsinstellingen openen",
  "mobile.common.show": "Tonen",
  "mobile.common.hide": "Verbergen",
  "mobile.common.copy": "Kopiëren",
  "mobile.common.save": "Bewaren",
  "mobile.common.remove": "Verwijderen",
  "mobile.common.edit": "Bewerken",
  "mobile.common.working": "Bezig",
  "mobile.vault.add": "Item toevoegen",
  "mobile.vault.addLogin": "Login",
  "mobile.vault.addNote": "Beveiligde notitie",
  "mobile.vault.untitled": "Zonder titel",
  "mobile.vault.favorite": "Favoriet",
  "mobile.search.hint": "Zoek op titel, gebruikersnaam, website of label.",
  "mobile.search.none": "Geen items komen overeen met {query}.",
  "mobile.detail.favoriteAdd": "Aan favorieten toevoegen",
  "mobile.detail.favoriteRemove": "Uit favorieten verwijderen",
  "mobile.detail.delete": "Naar prullenmand",
  "mobile.detail.deleteTitle": "“{title}” naar de prullenmand verplaatsen?",
  "mobile.detail.deleteBody": "De wijziging wordt met je andere apparaten gesynchroniseerd.",
  "mobile.detail.otpLeft":
    "{seconds, plural, one {# seconde over} other {# seconden over}}",
  "mobile.detail.passkey": "Passkey voor {site}",
  "mobile.detail.attachments":
    "{count, plural, one {# bestand is bijgevoegd. Open het op je computer.} other {# bestanden zijn bijgevoegd. Open ze op je computer.}}",
  "mobile.detail.linkedApp": "App: {name}",
  "mobile.detail.needsLinking": "Moet worden gekoppeld voordat het kan invullen",
  "mobile.detail.linkApp": "Een app koppelen",
  "mobile.detail.linkSharedHint": "Apps kunnen niet worden gekoppeld aan inloggegevens in een gedeelde kluis.",
  "mobile.editor.newLogin": "Nieuwe login",
  "mobile.editor.editLogin": "Login bewerken",
  "mobile.editor.newNote": "Nieuwe notitie",
  "mobile.editor.editNote": "Notitie bewerken",
  "mobile.editor.title": "Titel",
  "mobile.editor.username": "Gebruikersnaam of e-mail",
  "mobile.editor.password": "Wachtwoord",
  "mobile.editor.passwordKeep": "Laat leeg om het huidige wachtwoord te behouden.",
  "mobile.editor.websites": "Websites",
  "mobile.editor.websiteHint": "https://example.com",
  "mobile.editor.websiteAdd": "Een website toevoegen",
  "mobile.editor.websiteRemove": "Website {number} verwijderen",
  "mobile.editor.notes": "Notities",
  "mobile.editor.favorite": "Als favoriet markeren",
  "mobile.editor.saving": "Bewaren",
  "mobile.editor.otpTitle": "Eenmalige code",
  "mobile.editor.otpHave": "Deze login heeft een eenmalige code.",
  "mobile.editor.otpInput": "Installatiesleutel of otpauth-link",
  "mobile.editor.otpAdd": "De code toevoegen",
  "mobile.editor.otpReady": "Klaar: {name}. Hij wordt met de login bewaard.",
  "mobile.editor.otpRemove": "De code verwijderen",
  "mobile.editor.otpRemoved": "De code wordt verwijderd wanneer je bewaart.",
  "mobile.editor.generate": "Een wachtwoord genereren",
  "mobile.generator.title": "Wachtwoordgenerator",
  "mobile.generator.password": "Tekens",
  "mobile.generator.passphrase": "Woorden",
  "mobile.generator.length": "Lengte: {length}",
  "mobile.generator.lowercase": "Kleine letters",
  "mobile.generator.uppercase": "Hoofdletters",
  "mobile.generator.digits": "Cijfers",
  "mobile.generator.symbols": "Symbolen",
  "mobile.generator.ambiguous": "Op elkaar lijkende tekens overslaan",
  "mobile.generator.wordCount": "Woorden: {count}",
  "mobile.generator.separator": "Scheidingsteken",
  "mobile.generator.capitalize": "Woorden met hoofdletter",
  "mobile.generator.appendDigit": "Een cijfer toevoegen",
  "mobile.generator.run": "Genereren",
  "mobile.generator.use": "Dit wachtwoord gebruiken",
  "mobile.generator.strength": "Ongeveer {bits} bits aan sterkte",
  "mobile.link.title": "Een app koppelen",
  "mobile.link.choose": "Kies de app die je aan {login} wilt koppelen.",
  "mobile.link.filter": "Een app zoeken",
  "mobile.link.app": "App",
  "mobile.link.package": "App-ID",
  "mobile.link.installer": "Geïnstalleerd via",
  "mobile.link.installerUnknown": "Onbekende bron",
  "mobile.link.certificate": "Ontwikkelaarshandtekening (begin)",
  "mobile.link.verified": "Geverifieerd door {host}",
  "mobile.link.notVerified": "Niet geverifieerd",
  "mobile.link.unavailable": "Kon niet worden gecontroleerd",
  "mobile.link.checking": "De website wordt gecontroleerd",
  "mobile.link.warning":
    "Koppel alleen apps die je vertrouwt. Mosaic kan niet bevestigen dat deze app bij {host} hoort.",
  "mobile.link.warningNoHost":
    "Koppel alleen apps die je vertrouwt. Deze login heeft geen website die Mosaic kan controleren.",
  "mobile.link.confirm": "{login} voortaan in deze app invullen",
  "mobile.link.done": "Gekoppeld. Mosaic vult {login} in deze app in.",
  "mobile.link.noApps": "Geen andere apps gevonden.",
  "mobile.settings.security": "Beveiliging",
  "mobile.settings.autoLock": "Vergrendelen wanneer je de app verlaat",
  "mobile.settings.autoLock.now": "Meteen",
  "mobile.settings.autoLock.s30": "Na 30 seconden",
  "mobile.settings.autoLock.s60": "Na 1 minuut",
  "mobile.settings.autoLock.s300": "Na 5 minuten",
  "mobile.settings.idle": "Vergrendelen wanneer je het scherm niet meer aanraakt",
  "mobile.settings.idleOption": "{minutes, plural, one {Na # minuut} other {Na # minuten}}",
  "mobile.settings.clipboard": "Gekopieerde waarden wissen",
  "mobile.settings.clipboardOption":
    "{seconds, plural, one {Na # seconde} other {Na # seconden}}",
  "mobile.settings.quick": "Snel ontgrendelen",
  "mobile.settings.quickOn": "Snel ontgrendelen staat aan.",
  "mobile.settings.quickOff": "Snel ontgrendelen staat uit.",
  "mobile.settings.quickEnable": "Aanzetten",
  "mobile.settings.quickDisable": "Uitzetten",
  "mobile.settings.quickPassword": "Typ je hoofdwachtwoord om snel ontgrendelen aan te zetten.",
  "mobile.settings.sync": "Synchronisatie",
  "mobile.about.preAudit":
    "Mosaic is software vóór een audit. Er is nog geen onafhankelijke beveiligingsaudit uitgevoerd.",
  "mobile.about.privacy":
    "Geen analyses, geen tracking en geen crashrapporten verlaten deze telefoon.",
  "mobile.about.licences": "Open-sourcelicenties",
  "mobile.licences.title": "Open-sourcelicenties",
  "mobile.licences.intro": "Mosaic voor Android is gebouwd met deze open-sourceonderdelen.",
  "mobile.licences.rust":
    "De Rust-kern gebruikt open-sourcecrates. Hun licenties staan in de broncoderepository van het project.",
  "mobile.autofill.unlockEntry": "Mosaic ontgrendelen",
  "mobile.autofill.noLogin": "Geen login voor {app}",
  "mobile.autofill.noLoginHint": "Tik om een login te koppelen",
  "mobile.autofill.cannotCheck": "Kon deze app niet controleren",
  "mobile.autofill.warning": "Controleer eerst het adres",
  "mobile.autofill.noUsername": "Geen gebruikersnaam",
  "mobile.autofill.otpEntry": "Eenmalige code",
  "mobile.link.pickLogin": "Kies de login die Mosaic in {app} moet invullen.",
  "mobile.credential.unlockEntry": "Mosaic ontgrendelen",
  "mobile.credential.passwordEntry": "Wachtwoord",
  "mobile.credential.passkeyEntry": "Passkey",
  "mobile.credential.createEntry": "Passkey bewaren in Mosaic",
  "mobile.credential.createTitle": "Een passkey bewaren?",
  "mobile.credential.createBody": "Mosaic bewaart een passkey voor {site} ({user}).",
  "mobile.credential.createVault": "Bewaren in",
  "mobile.credential.vaultDefault": "Persoonlijke kluis",
  "mobile.credential.createConfirm": "Passkey bewaren",
  "mobile.credential.verifyTitle": "Bevestig dat jij het bent",
  "mobile.credential.verifySignIn": "Inloggen bij {site}",
  "mobile.credential.verifyCreate": "Een passkey bewaren voor {site}",
  "mobile.credential.verifyFailed":
    "Mosaic kon niet bevestigen dat jij het bent. Probeer het opnieuw.",
  "mobile.credential.unsupported":
    "Mosaic kan dit inlogverzoek niet beantwoorden. Annuleer en kies een andere manier om in te loggen.",
  "mobile.settings.fill": "Invullen in andere apps",
  "mobile.settings.autofillOn": "Mosaic is je service voor automatisch invullen.",
  "mobile.settings.autofillOff": "Mosaic is nog niet je service voor automatisch invullen.",
  "mobile.settings.autofillEnable": "Mosaic gebruiken voor automatisch invullen",
  "mobile.settings.credentialHint":
    "Android kan je passkeys en wachtwoorden uit Mosaic ook aanbieden wanneer een app je vraagt in te loggen.",
  "mobile.settings.credentialEnable": "Mosaic gebruiken voor passkeys en wachtwoorden",
  "mobile.scan.setupAction": "Installatiecode scannen",
  "mobile.scan.otpAction": "Een QR-code scannen",
  "mobile.scan.otpTitle": "De eenmalige code scannen",
  "mobile.scan.otpHint":
    "Houd de camera boven de QR-code van de website. Mosaic leest hem alleen op deze telefoon.",
  "mobile.scan.preview": "Camerabeeld",
  "mobile.scan.looking": "Zoeken naar een QR-code",
  "mobile.scan.notSetupCode": "Geen Mosaic-installatiecode",
  "mobile.scan.notSetupCodeHelp":
    "Houd de camera boven de code die Mosaic op je computer toont, bij Instellingen onder Synchronisatie.",
  "mobile.scan.rationale":
    "Mosaic gebruikt de camera alleen om deze QR-code te lezen. Er wordt niets bewaard of verstuurd.",
  "mobile.scan.allow": "De camera toestaan",
  "mobile.scan.denied":
    "Mosaic kan de camera niet gebruiken. Sta het toe in de instellingen van deze telefoon om te scannen, of voer de gegevens handmatig in.",
  "mobile.scan.openSettings": "Instellingen openen",
  "mobile.scan.unavailable":
    "Mosaic kon de camera niet starten. Voer de gegevens handmatig in.",
  "mobile.onboarding.details.scanned":
    "Server, e-mail en Geheime Sleutel zijn ingevuld vanuit de installatiecode. Controleer ze en ga dan door.",
  "mobile.onboarding.details.scanOr": "Of voer de gegevens handmatig in",
  "mobile.onboarding.details.keyFromCode": "Geheime Sleutel uit de installatiecode",
  "mobile.settings.passkeyIdentities": "Mosaic gebruiken voor passkeys",
  "mobile.settings.passkeyIdentitiesHint":
    "Mosaic vertelt het systeem de website en gebruikersnaam van elke passkey, zodat het die kan aanbieden. De sleutels verlaten Mosaic nooit.",
  "mobile.settings.passwordIdentities": "Logins boven het toetsenbord voorstellen",
  "mobile.settings.passwordIdentitiesHint":
    "Mosaic vertelt het systeem de website en gebruikersnaam van elke login. Het systeem bewaart deze lijst op de telefoon, buiten de versleuteling van Mosaic.",
  "mobile.settings.identitiesPassword": "Typ je hoofdwachtwoord om dit aan te zetten.",
  "mobile.licences.introIos": "Mosaic voor iPhone is gebouwd met deze open-sourceonderdelen.",
  "mobile.ext.openApp": "Open Mosaic en ontgrendel het eenmalig met je hoofdwachtwoord.",
  "mobile.ext.noAccount":
    "Mosaic heeft nog geen account op deze iPhone. Open Mosaic om het in te stellen.",
  "mobile.ext.loading": "Je kluis wordt geopend",
  "mobile.ext.thisSite": "deze site",
  "mobile.ext.matches": "Logins voor {site}",
  "mobile.ext.noMatches": "Geen login voor {site}",
  "mobile.ext.noMatchesHint":
    "Mosaic vult alleen logins in die voor deze site zijn bewaard. Open Mosaic om deze site aan een login toe te voegen.",
  "mobile.ext.searchAll": "Alle logins doorzoeken",
  "mobile.ext.hideAll": "Andere logins verbergen",
  "mobile.ext.notSavedFor": "Niet bewaard voor {site}",
  "mobile.ext.fillHint": "Vult deze login in",
  "mobile.ext.warnPunycode":
    "Dit adres is met speciale tekens geschreven: {unicode}. Controleer of dit de site is die je bedoelt.",
  "mobile.ext.warnLookalike":
    "Dit adres lijkt op {site}, die je in Mosaic hebt bewaard. Controleer of dit de site is die je bedoelt.",
  "mobile.ext.otpTitle": "Kies een eenmalige code",
  "mobile.ext.passkeyTitle": "Kies een passkey",
  "mobile.ext.configTitle": "Automatisch invullen met Mosaic",
  "mobile.ext.configBody":
    "Mosaic biedt je logins, passkeys en eenmalige codes aan wanneer je op een veld of een inlogknop tikt. Wat het aanbiedt wijzig je bij Instellingen in de Mosaic-app.",
  "mobile.ext.confirmFillTitle": "De login voor {site} invullen?",
  "mobile.ext.confirmFillBody":
    "iOS vertelt Mosaic niet op welke pagina je zit, dus Mosaic kan dat niet controleren. Vul alleen in als dit de site is die je bedoelt. Mosaic vraagt daarna om Face ID of Touch ID.",
  "mobile.ext.confirmFillButton": "Deze login invullen",
  "mobile.settings.passwordIdentitiesLimit":
    "Wanneer je op een suggestie tikt, vertelt iOS Mosaic niet op welke pagina je zit. Daarom toont Mosaic de bewaarde site en vult het pas in nadat je dat bevestigt en ontgrendelt met Face ID of Touch ID.",
} satisfies Record<keyof typeof en, string>;
