/**
 * Dutch UI strings: errors. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { errors as en } from "../en/errors.ts";

export const errors = {
  "error.notImplemented":
    "Dit is nog niet beschikbaar. Het komt in een latere versie van Mosaic.",
  "error.noAccount":
    "Er is nog geen account op dit apparaat. Maak er een aan, of log in met je Noodkit.",
  "error.accountExists": "Dit apparaat heeft al een Mosaic-account. Ontgrendel dat in plaats daarvan.",
  "error.locked": "Mosaic is vergrendeld. Ontgrendel het om door te gaan.",
  "error.alreadyUnlocked": "Mosaic is al ontgrendeld. Je kunt verdergaan waar je was.",
  "error.wrongCredentials":
    "Dat hoofdwachtwoord of die Geheime Sleutel werkte niet. Controleer Caps Lock en je toetsenbordindeling en probeer het opnieuw.",
  "error.secretKeyRequired":
    "Dit apparaat heeft je Geheime Sleutel niet. Typ hem over van je Noodkit.",
  "error.secretKeyMalformed":
    "Dat is geen Geheime Sleutel. De jouwe begint met “M1-” en staat op je Noodkit.",
  "error.secretKeyMistyped":
    "Die Geheime Sleutel bevat een typefout. Vergelijk hem teken voor teken met je Noodkit.",
  "error.emptyPassword": "Het hoofdwachtwoord is leeg. Voer het in om door te gaan.",
  "error.quickUnlockUnavailable":
    "Snel ontgrendelen is op dit apparaat niet ingesteld. Ontgrendel met je hoofdwachtwoord.",
  "error.storeInUse":
    "Mosaic is al geopend op deze computer. Sluit het andere venster en probeer het opnieuw.",
  "error.storeUnavailable":
    "Mosaic kon het kluisbestand op dit apparaat niet openen. Controleer of de schijf beschikbaar is en vrije ruimte heeft en start Mosaic opnieuw.",
  "error.vaultNotFound":
    "Die kluis bestaat niet meer. Hij is mogelijk op een ander apparaat verwijderd.",
  "error.vaultUnavailable":
    "Deze kluis kan in deze sessie niet worden geopend. Ontgrendel hem met zijn hardwaresleutel, of synchroniseer en probeer het opnieuw.",
  "error.itemNotFound":
    "Dat item bestaat niet meer. Het is mogelijk op een ander apparaat verplaatst of verwijderd.",
  "error.itemDeleted":
    "Dat item is definitief verwijderd en kan niet worden geopend. Kies een ander item.",
  "error.sameVault": "Het item staat al in die kluis. Kies een andere kluis.",
  "error.noOtp": "Dit item heeft geen eenmalige code. Bewerk het item om er een toe te voegen.",
  "error.fieldEmpty":
    "Dat veld is leeg, dus er is niets om te kopiëren of te tonen. Bewerk het item om het in te vullen.",
  "error.invalidInput":
    "Er klopt iets niet in dit formulier. Controleer de gemarkeerde velden en probeer het opnieuw.",
  "error.sshKeyInvalid":
    "Dit is geen persoonlijke OpenSSH-sleutel. Plak de hele sleutel, inclusief de BEGIN- en END-regels.",
  "error.reauthRequired": "Voor deze wijziging is je hoofdwachtwoord nodig. Voer het in om te bevestigen.",
  "error.attachmentTooLarge": "Dat bestand is groter dan 50 MiB. Kies een kleiner bestand.",
  "error.attachmentQuota":
    "Je bestanden zouden meer dan de beschikbare 1 GiB gebruiken. Verwijder een aantal bijlagen en probeer het opnieuw.",
  "error.attachmentNotFound": "Dit bestand maakt geen deel meer uit van het item. Laad het item opnieuw.",
  "error.attachmentUnavailable":
    "Dit bestand kan op dit apparaat niet worden geopend. Het is mogelijk nog niet gesynchroniseerd of beschadigd.",
  "error.attachmentFileUnreadable": "Dat bestand kan niet worden gelezen. Kies een ander bestand.",
  "error.attachmentSaveFailed": "Het bestand kon daar niet worden bewaard. Kies een andere map.",
  "error.attachmentPreviewUnsupported":
    "Er is geen voorvertoning voor dit bestand. Gebruik Bewaren als om het te openen.",
  "error.attachmentPreviewTooLarge":
    "Deze afbeelding is te groot voor een voorvertoning. Gebruik Bewaren als om hem te openen.",
  "error.clipboardUnavailable":
    "Mosaic kon het klembord niet gebruiken. Sluit apps die het mogelijk vasthouden en probeer het opnieuw.",
  "error.importFileUnreadable":
    "Mosaic kon dat bestand niet lezen. Controleer of het nog bestaat en niet in een andere app open staat en probeer het opnieuw.",
  "error.importDeleteNotAllowed":
    "Importeer het bestand eerst. Mosaic verwijdert een export pas nadat de items in je kluis staan.",
  "error.importDeleteFailed":
    "Het bestand kon niet worden verwijderd. Het staat mogelijk open in een andere app of is alleen-lezen. Verwijder het zelf en leeg daarna de prullenbak.",
  "error.importFormatInvalid":
    "Dat bestand heeft niet het formaat dat je koos. Kies het juiste formaat of exporteer het bestand opnieuw.",
  "error.importPreviewExpired":
    "Deze import is geannuleerd. Kies het bestand opnieuw om opnieuw te beginnen.",
  "error.sshRequestExpired":
    "Dat SSH-verzoek is al beëindigd. Voer de opdracht opnieuw uit als je het nog nodig hebt.",
  "error.sshAgentUnavailable":
    "De SSH-agent kon niet starten. Controleer of er geen andere SSH-agent actief is en zet hem opnieuw aan.",
  "error.unlockDelayed":
    "Te veel foute pogingen achter elkaar. Wacht een paar seconden en probeer het opnieuw.",
  "error.tooManyAttempts":
    "Te veel foute antwoorden. Mosaic is vergrendeld. Ontgrendel het met je hoofdwachtwoord om door te gaan.",
  "error.sshAllowTooSoon":
    "Dat ging te snel om het verzoek te lezen. Controleer de sleutel en het programma en kies opnieuw Toestaan.",
  "error.browserRequestExpired":
    "Dat browserverzoek is al beëindigd. Vraag het opnieuw aan vanuit de extensie als je het nog nodig hebt.",
  "error.browserAllowTooSoon":
    "Dat ging te snel om het verzoek te lezen. Controleer de details en kies opnieuw.",
  "error.browserPairingNotFound":
    "Die browser is niet meer verbonden. Open de instellingen opnieuw om de actuele lijst te zien.",
  "error.browserHostMissing":
    "Deze kopie van Mosaic heeft geen browserconnector geïnstalleerd, dus er is niets te herstellen. Installeer Mosaic met het installatieprogramma om een browser te verbinden.",
  "error.agentOriginInvalid":
    "Agents kunnen een sleutel alleen naar een site sturen die met https:// begint en alleen de site noemt, zoals https://api.example.com. Controleer het adres.",
  "error.agentBindingInvalid":
    "Mosaic kan deze instelling niet voor agents gebruiken. Controleer het veld, de sites en hoe de sleutel wordt verstuurd.",
  "error.agentValueUnusable":
    "Deze waarde is te kort of bevat een regeleinde, dus agents kunnen hem niet gebruiken. Een sleutel heeft minstens 12 tekens op één regel nodig.",
  "error.agentVaultShared":
    "Agents kunnen alleen items gebruiken in een kluis die van jou is en niet wordt gedeeld. Verplaats het item eerst naar een van je eigen kluizen.",
  "error.agentAccessOff":
    "Agenttoegang staat uit. Zet hem aan in de instellingen, onder Agents.",
  "error.agentRequestExpired":
    "Dat agentverzoek is al beëindigd. De agent kan het opnieuw vragen als hij het nog nodig heeft.",
  "error.agentAllowTooSoon":
    "Dat ging te snel om het verzoek te lezen. Controleer de details en kies opnieuw.",
  "error.agentPairingNotFound":
    "Die agent is niet meer verbonden. Open de instellingen opnieuw om de actuele lijst te zien.",
  "error.agentEndpointUnavailable":
    "Mosaic kon de verbinding die agents gebruiken niet openen. Sluit andere kopieën van Mosaic en probeer het opnieuw.",
  "error.kitNotVerified":
    "Bewaar eerst de nieuwe Noodkit en bevestig de laatste groep ervan. Mosaic wijzigt daarvoor niets.",
  "error.kitGroupMismatch":
    "Dat is niet de laatste groep van de nieuwe Geheime Sleutel. Controleer de nieuwe Noodkit.",
  "error.rotationInProgress":
    "Er staat nog een andere sleutelwijziging open. Rond hem af met Sleutelrotatie voortzetten, of verwerp hem eerst.",
  "error.rotationNone": "Er is geen sleutelwijziging om voort te zetten.",
  "error.rotationNotFound":
    "Die wachtwoordwijziging staat niet meer open. Ze is mogelijk op een ander apparaat of in de browser gesloten. Open het item opnieuw om te zien hoe het ervoor staat.",
  "error.rotationLimit":
    "Er staan al vier wachtwoordwijzigingen van dit item open. Rond er eerst een af of sluit er een in het item.",
  "error.rotationOpen":
    "Deze login heeft een openstaande wachtwoordwijziging. Zet hem terug uit de prullenmand en kies dan op het item Het werkte of Het werkte niet. Daarna kun je hem definitief verwijderen.",
  "error.rotationNotSupported":
    "Mosaic kan deze wijziging niet begeleiden. Daarvoor is een login met een https-adres nodig. Wijzig het wachtwoord zelf op de site.",
  "error.historyNotFound":
    "Dat eerdere wachtwoord staat niet meer in de geschiedenis. Open het item opnieuw om de actuele lijst te zien.",
  "error.originNotRelated":
    "Die pagina hoort niet bij de site van deze login, dus Mosaic heeft hem niet toegevoegd. Bewerk de adressen van het item als dat wel zo moet zijn.",
  "error.breachConsentRequired":
    "De datalekcontrole staat uit. Zet hem eerst aan en ga akkoord met wat er wordt verstuurd.",
  "error.breachRunning":
    "Er loopt al een datalekcontrole. Wacht tot die klaar is of stop hem eerst.",
  "error.rotationTooFar":
    "Deze wijziging heeft je account al bereikt en kan niet worden verworpen. Zet hem voort om hem af te ronden.",
  "error.rotationJumpTooLarge":
    "Dit apparaat loopt te ver achter op de andere apparaten om bij te komen. Stel het opnieuw in: verwijder het account van dit apparaat en sluit je opnieuw aan met je Noodkit.",
  "error.rotationNotApplied":
    "De wijziging heeft je account nooit bereikt, dus er is niets gewijzigd. Begin opnieuw of verwerp hem.",
  "error.secretKeyUnchanged":
    "Die Geheime Sleutel werkt niet meer. Gebruik de sleutel uit de Noodkit die je hebt bewaard op het apparaat dat de wijziging maakte.",
  "error.passwordUnchanged":
    "Het nieuwe hoofdwachtwoord moet anders zijn dan het huidige.",
  "error.profileDowngrade":
    "Dat niveau is niet sterker dan wat je account nu gebruikt. Kies een hoger niveau.",
  "error.profileTooSlow":
    "Ontgrendelen met dat niveau zou dit apparaat meer dan twee seconden kosten. Kies een lager niveau of upgrade toch.",
  "error.rekeyNeedsCleanSync":
    "Voor het vernieuwen van de sleutels moet de synchronisatie bij zijn en vrij van waarschuwingen. Synchroniseer nu, bekijk eventuele waarschuwingen en probeer het opnieuw.",
  "error.rekeyMemberKeyMismatch":
    "Je hebt de veiligheidscode van iedereen in een gedeelde kluis nog niet vergeleken, dus die kluis kan niet worden vernieuwd. Vergelijk hun codes in de deelinstellingen van de kluis en probeer het opnieuw.",
  "error.rekeyBusy":
    "Je kluizen bleven veranderen terwijl Mosaic bezig was. Wacht even en probeer het opnieuw.",
  "error.rekeyOutcomeUnknown":
    "Mosaic kon niet bevestigen of je account de nieuwe sleutels heeft geaccepteerd. Kies Sleutelrotatie voortzetten om het te achterhalen.",
  "error.rekeyNeedsServer":
    "Voor het vernieuwen van de sleutels is je synchronisatieserver of -map nodig, en die is niet bereikbaar. Controleer de verbinding en probeer het opnieuw.",
  "error.rekeyNoAccountFile":
    "Niets in je synchronisatiemap opent met dit wachtwoord en deze Geheime Sleutel. Controleer de nieuwe Noodkit.",
  "error.syncAccountRekeyed":
    "De sleutels van je account zijn op een ander apparaat vernieuwd. Gebruik de Geheime Sleutel uit de nieuwe Noodkit om door te gaan.",
  "error.syncEpochRollback":
    "Je synchronisatie-backend toont oudere accountsleutels dan dit apparaat heeft. Er is niets gewijzigd.",
  "error.syncRekeyRefused":
    "De sleutels die je synchronisatie-backend aanbiedt zijn niet de volgende na die van dit apparaat. Er is niets gewijzigd. Controleer je andere apparaten.",
  "error.pickerStale": "Deze lijst is verouderd. Klik opnieuw op het veld.",
  "error.pickerTooSoon": "Wacht even en bevestig dan opnieuw.",
  "error.programFillUnavailable": "Invullen in programma’s is op dit apparaat niet beschikbaar.",
  "error.busy": "Mosaic rondt de vorige stap nog af. Wacht even en probeer het opnieuw.",
  "error.internal":
    "Er ging onverwacht iets mis bij deze actie. Probeer het opnieuw en start Mosaic opnieuw als het blijft gebeuren.",
  "error.importTooLarge":
    "Dit bestand is te groot of te complex om te importeren. Exporteer minder items tegelijk en probeer het opnieuw.",
  "error.importUnsupported":
    "Mosaic kan deze versie van het formaat niet lezen. Exporteer opnieuw in een actueel formaat (voor KeePass: KDBX 4 met AES of ChaCha20) en probeer het opnieuw.",
  "error.importEncrypted":
    "Deze export is met een wachtwoord beveiligd. Exporteer opnieuw zonder versleuteling, importeer hem en verwijder daarna het bestand.",
  "error.sshAgentEndpointInUse":
    "Er draait al een andere SSH-agent (in Windows de service OpenSSH Authentication Agent). Stop die en zet de SSH-agent van Mosaic opnieuw aan.",
  "error.cancelled": "Geannuleerd. Er is niets gewijzigd.",
  "error.quickUnlockExpired":
    "Snel ontgrendelen is verlopen. Ontgrendel met je hoofdwachtwoord en zet het daarna opnieuw aan in de instellingen.",
  "error.quickUnlockInvalidated":
    "Snel ontgrendelen werkt op dit apparaat niet meer en is uitgezet. Ontgrendel met je hoofdwachtwoord.",
  "error.passwordRequired":
    "Voer deze keer je hoofdwachtwoord in. Daarna werkt snel ontgrendelen weer.",
  "error.hardwareKeyUnsupported":
    "Deze beveiligingssleutel of dit systeem kan niet met Mosaic worden gebruikt.",
  "error.hardwareKeyNotFound": "Geen beveiligingssleutel gevonden. Steek hem in en probeer het opnieuw.",
  "error.hardwareKeyTimeout": "De sleutel is niet op tijd aangeraakt. Probeer het opnieuw.",
  "error.hardwareKeyPinRequired": "Voer de pincode van je beveiligingssleutel in.",
  "error.hardwareKeyPinInvalid":
    "Verkeerde pincode. Na een aantal foute pogingen vergrendelt de sleutel zichzelf.",
  "error.hardwareKeyPinBlocked":
    "Deze sleutel is vergrendeld na te veel foute pincodes. Reset hem met het hulpprogramma van de maker; zijn registraties gaan dan verloren.",
  "error.hardwareKeyNoPin":
    "Stel eerst een pincode in op deze sleutel, met het hulpprogramma van de maker.",
  "error.hardwareKeyNotRecognized":
    "Deze sleutel is hier niet geregistreerd. Probeer een andere sleutel of je hoofdwachtwoord.",
  "error.hardwareKeyLimit":
    "Je hebt het maximale aantal toegestane sleutels (8) geregistreerd. Verwijder er eerst een.",
  "error.hardwareKeyLast":
    "Dit is de enige sleutel van de kluis. Registreer eerst een andere voordat je hem verwijdert.",
  "error.itemTypeNotAllowed": "Deze kluis bevat alleen eenmalige codes.",
  "error.otpScanUnsupported":
    "Het scannen van het scherm is op dit systeem niet beschikbaar. Plak in plaats daarvan een schermafbeelding.",
  "error.otpQrNotFound":
    "Geen QR-code gevonden. Zorg dat de hele code zichtbaar is en probeer het opnieuw.",
  "error.otpQrNotOtp":
    "Deze QR-code is geen installatiecode voor tweestapsverificatie. Zoek de code op de 2FA-instelpagina van de site.",
  "error.otpQrMigration":
    "Dit is een exportcode van Google Authenticator, die Mosaic nog niet kan lezen. Stel elk account opnieuw in op zijn site, of voer de installatiesleutel in.",
  "error.otpQrMultiple":
    "Er is meer dan één installatiecode zichtbaar. Toon alleen de code die je wilt en probeer het opnieuw.",
  "error.otpQrInvalid":
    "Deze installatiecode gebruikt instellingen die Mosaic niet ondersteunt. Voer in plaats daarvan de installatiesleutel in.",
  "error.otpUriInvalid": "Dit is geen geldige otpauth://-link. Kopieer de hele link opnieuw.",
  "error.otpKeyInvalid":
    "Deze sleutel is ongeldig. Hij gebruikt alleen de letters A tot Z en de cijfers 2 tot 7; controleer op typefouten.",
  "error.otpImageUnreadable": "Mosaic kan deze afbeelding niet lezen. Gebruik een PNG- of JPEG-bestand.",
  "error.otpImageTooLarge":
    "Deze afbeelding is te groot. Gebruik een schermafbeelding van alleen de QR-code.",
  "error.otpClipboardNoImage":
    "Er staat geen afbeelding op het klembord. Maak een schermafbeelding van de QR-code en plak hem hier.",
  "error.otpPendingExpired": "De gescande code is niet meer beschikbaar. Scan hem opnieuw.",
  "error.syncNotConfigured":
    "Synchronisatie is op dit apparaat nog niet ingesteld. Stel het eerst in bij de instellingen.",
  "error.syncAlreadyConfigured":
    "Synchronisatie is al ingesteld. Verbreek eerst de verbinding om een andere server of map te gebruiken.",
  "error.syncServerUnreachable":
    "Mosaic kon de server niet bereiken. Controleer het adres en je internetverbinding en probeer het opnieuw.",
  "error.syncServerInvalid":
    "Dit lijkt niet op een Mosaic-server. Controleer het adres; het hoort eruit te zien als https://sync.example.com.",
  "error.syncServerInsecure":
    "Mosaic maakt alleen via https verbinding met servers, zodat niemand onderweg je gegevens kan lezen of wijzigen. Gebruik een https://-adres.",
  "error.syncServerMismatch":
    "Dit is niet de server waarmee dit account synchroniseert. Controleer het adres.",
  "error.syncServerIncompatible":
    "Deze server draait een versie die niet bij je Mosaic past. Werk Mosaic bij of vraag de eigenaar van de server om te updaten.",
  "error.syncRegistrationClosed":
    "Deze server accepteert geen nieuwe synchronisatieaccounts. Log in op een bestaand account of vraag het de eigenaar van de server.",
  "error.syncInviteInvalid":
    "Deze uitnodigingscode is ongeldig of al gebruikt. Controleer hem of vraag de eigenaar van de server om een nieuwe.",
  "error.syncAccountExists":
    "Je hebt al een synchronisatieaccount op deze server. Kies in plaats daarvan Inloggen.",
  "error.syncAccountMismatch":
    "Het synchronisatieaccount op die server hoort bij een ander Mosaic-account. Controleer het adres en je e-mailadres.",
  "error.syncTwoFactorRequired":
    "Dit synchronisatieaccount heeft een code uit je authenticator-app nodig. Voer de 6-cijferige code in.",
  "error.syncTwoFactorInvalid":
    "Die code klopt niet of is verlopen. Voer de actuele 6-cijferige code uit je authenticator-app in.",
  "error.syncSignedOut":
    "De server heeft dit apparaat uitgelogd. Log opnieuw in om te blijven synchroniseren. Je wijzigingen zijn veilig op dit apparaat.",
  "error.syncRateLimited": "Te veel pogingen. Wacht een paar minuten en probeer het opnieuw.",
  "error.syncQuotaExceeded":
    "De server of map heeft geen ruimte meer voor je gegevens. Maak ruimte vrij of vraag de eigenaar van de server om meer.",
  "error.syncFolderUnavailable":
    "Mosaic kan deze map niet gebruiken. Controleer of hij bestaat, of je cloudservice klaar is met synchroniseren en of Mosaic erin mag schrijven.",
  "error.syncFolderNotEmpty":
    "Deze map bevat andere bestanden, dus Mosaic schrijft er niet in. Kies een lege map of maak een nieuwe.",
  "error.syncFolderNoAccount":
    "Deze map bevat geen synchronisatiegegevens voor dit account. Kies de map waarmee je andere apparaat synchroniseert.",
  "error.syncFolderDamaged":
    "Sommige synchronisatiebestanden in deze map zijn beschadigd. Laat je cloudservice klaar zijn met synchroniseren of zet de map terug uit een back-up en probeer het opnieuw.",
  "error.syncPasswordOutcomeUnknown":
    "We konden niet bevestigen of de server je hoofdwachtwoord heeft gewijzigd. Bewaar beide wachtwoorden totdat de synchronisatie bevestigt welk van toepassing is.",
  "error.syncWebauthnUnavailable":
    "Je server kan hiervoor nu geen beveiligingssleutel gebruiken, of dit account heeft er geen. Gebruik in plaats daarvan een code uit een authenticator-app.",
  "error.syncWebauthnLimit":
    "Dit account heeft al het maximale aantal beveiligingssleutels dat de server toestaat. Verwijder er een die je niet meer gebruikt en probeer het opnieuw.",
  "error.syncHandoffExpired": "De stap in de browser is verlopen. Begin opnieuw.",
  "error.syncHandoffFailed":
    "Te veel foute codes of mislukte controles in de browser. Begin de stap opnieuw.",
  "error.syncPasswordNeedsServer":
    "Voor het wijzigen van je hoofdwachtwoord is je synchronisatieserver nodig, en die is niet bereikbaar. Controleer je internetverbinding en probeer het opnieuw.",
  "error.syncDeviceIsCurrent":
    "Je kunt het apparaat dat je gebruikt niet verwijderen. Gebruik in plaats daarvan Verbinding verbreken.",
  "error.syncChoiceExpired": "De mapkeuze is verlopen. Kies de map opnieuw.",
  "error.syncFreshLoginExpired":
    "Je wachtwoordcontrole is verlopen. Voer je hoofdwachtwoord opnieuw in.",
  "error.syncPasswordRollback":
    "De server bood een ouder hoofdwachtwoord aan. Controleer de server.",
  "error.syncPasswordChanged":
    "Je hoofdwachtwoord is op de server of in de map gewijzigd. Voer hieronder het nieuwe in.",
  "error.sharingNotHosted":
    "Delen heeft een Mosaic-server nodig. Verbind er eerst een bij de synchronisatie-instellingen.",
  "error.setupCodeNeedsServer": "Een telefoon kan alleen deelnemen aan een account dat synchroniseert.",
  "error.sharingContactUnverified":
    "Vergelijk eerst de veiligheidscode van deze persoon. Mosaic deelt alleen met mensen die je hebt geverifieerd.",
  "error.sharingKeyChanged":
    "De sleutel van deze persoon is gewijzigd, of de code komt niet overeen. Zoek de persoon opnieuw op en vergelijk de nieuwe code voordat je verdergaat.",
  "error.sharingRoleForbidden":
    "Je rol in deze kluis staat dat niet toe. Vraag het de eigenaar of een beheerder.",
  "error.sharingInviteExists":
    "Er wacht al een uitnodiging voor deze persoon en kluis. Trek die eerst in als je een nieuwe wilt sturen.",
  "error.sharingInviteStale":
    "De uitnodiging is intussen veranderd, bijvoorbeeld na een sleutelwijziging. Open hem opnieuw.",
  "error.sharingInviteExpired":
    "Deze uitnodiging is verlopen of ingetrokken. Vraag de afzender je opnieuw uit te nodigen.",
  "error.sharingInviteUnreadable":
    "Deze uitnodiging kan niet worden geopend. Hij is mogelijk beschadigd of niet voor dit account bedoeld. Vraag de afzender je opnieuw uit te nodigen.",
  "error.sharingInviteNotInRoster":
    "De ledenlijst van de kluis komt niet overeen met deze uitnodiging, dus Mosaic heeft hem niet geopend. Vraag de afzender je opnieuw uit te nodigen.",
  "error.sharingOwnerCannotLeave":
    "De eigenaar kan een kluis niet verlaten. Verwijder in plaats daarvan de andere mensen.",
  "error.sharingAlreadyMember": "Deze persoon is al lid van de kluis.",
  "error.sharingReinviteBlocked":
    "Je hebt deze persoon onlangs verwijderd. Je kunt de persoon opnieuw uitnodigen zodra de kluissleutel is gewijzigd. Synchroniseer en probeer het opnieuw.",
  "error.sharingInviteLimit":
    "Er wachten te veel uitnodigingen. Wacht op antwoorden of trek er een paar in.",
  "error.sharingVaultKnown":
    "Dit apparaat heeft die kluis al, dus Mosaic accepteerde er geen sleutel voor. Vraag de afzender welke kluis hij heeft gedeeld.",
  "error.syncHasSharedVaults":
    "Je kunt niet van server wisselen zolang er gedeelde kluizen op dit apparaat staan. Verlaat eerst de gedeelde kluizen, of laat de eigenaars je verwijderen.",
  "error.emergencyWaiting":
    "Er wacht een verzoek, dus dit kan nu niet worden gewijzigd. Weiger het of wacht erop.",
  "error.emergencyCooldown":
    "Je laatste verzoek is geweigerd. Je kunt opnieuw vragen na de wachttijd van één dag.",
  "error.emergencyNotGranted":
    "Er is niets op te eisen: er staat geen verzoek open, of de persoon heeft je geen toegang gegeven.",
  "error.shortcutUnavailable":
    "Een ander programma gebruikt deze sneltoets al, of Mosaic kan hem niet registreren. Kies een andere combinatie.",
  "error.shortcutUnsupported":
    "Je systeem laat apps geen algemene sneltoets instellen. Open Mosaic in plaats daarvan vanuit het venster.",
} satisfies Record<keyof typeof en, string>;
