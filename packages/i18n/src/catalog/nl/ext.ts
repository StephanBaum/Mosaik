/**
 * Dutch UI strings: ext. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { ext as en } from "../en/ext.ts";

export const ext = {
  "ext.manifest.name": "Mosaic",
  "ext.manifest.description":
    "Vul logins in uit je Mosaic-kluis. Je hoofdwachtwoord blijft in de desktop-app.",
  "ext.command.fill": "Login invullen met Mosaic",
  "ext.command.generate": "Een wachtwoord genereren met Mosaic",
  "ext.save.badgeTitle": "Login bewaren?",
  "ext.loading": "Laden…",
  "ext.site.secure": "Huidige site, beveiligde verbinding (https): {host}",
  "ext.site.insecure": "Huidige site, niet versleuteld (http): {host}",
  "ext.site.none": "Dit tabblad heeft geen website",
  "ext.nav.label": "Weergaven",
  "ext.nav.logins": "Logins en zoeken",
  "ext.nav.generator": "Wachtwoordgenerator",
  "ext.footer.open": "Mosaic openen",
  "ext.footer.more": "Meer opties",
  "ext.footer.connected": "Verbonden met Mosaic",
  "ext.footer.locked": "Mosaic is vergrendeld",
  "ext.footer.offline": "Niet verbonden",
  "ext.more.saveOffers": "Aanbieden om logins te bewaren",
  "ext.more.saveOffersHint":
    "Heeft toegang tot alle websites nodig, zodat de extensie een login kan opmerken die je verstuurt. Er wordt niets bewaard totdat je in de Mosaic-app bevestigt.",
  "ext.passkeys.toggle": "Mosaic gebruiken voor passkeys",
  "ext.passkeys.hint":
    "Heeft toegang tot alle websites nodig om passkeyverzoeken te beantwoorden. Je bevestigt elk verzoek in de Mosaic-app.",
  "ext.passkeys.permissionDenied":
    "Mosaic heeft toegang tot websites nodig om passkeyverzoeken te beantwoorden, en die heb je niet toegestaan.",
  "ext.more.disconnect": "Verbinding met Mosaic verbreken",
  "ext.more.disconnectTitle": "Deze browser loskoppelen?",
  "ext.more.disconnectBody":
    "Deze browser kan geen logins invullen totdat je hem opnieuw verbindt. Je keurt de nieuwe verbinding goed in de Mosaic-app.",
  "ext.action.cancel": "Annuleren",
  "ext.neverSave.menu": "Sites waarvoor nooit wordt aangeboden te bewaren…",
  "ext.neverSave.title": "Nooit aanbieden om te bewaren",
  "ext.neverSave.intro":
    "Mosaic biedt op deze sites niet aan om logins te bewaren. Verwijder een site om daar weer gevraagd te worden.",
  "ext.neverSave.empty":
    "Nog geen sites. Je kunt er een toevoegen met “Nooit voor deze site” wanneer Mosaic aanbiedt een login te bewaren.",
  "ext.neverSave.remove": "Verwijderen",
  "ext.neverSave.removeLabel": "{origin} uit de lijst verwijderen",
  "ext.neverSave.removed":
    "{origin} verwijderd. Mosaic kan daar weer aanbieden om logins te bewaren.",
  "ext.neverSave.close": "Sluiten",
  "ext.connect.checking": "Verbinden met Mosaic…",
  "ext.connect.hostMissing.title": "Mosaic is niet ingesteld voor deze browser",
  "ext.connect.hostMissing.body":
    "Installeer de Mosaic-app op deze computer en start de browser daarna opnieuw.",
  "ext.connect.hostMissing.dev": "Ontwikkelaars: voer {command} uit en start de browser opnieuw.",
  "ext.connect.hostForbidden.title": "Deze browser mag niet verbinden",
  "ext.connect.hostForbidden.body":
    "Mosaic praat alleen met de officiële Mosaic-extensie. Installeer hem uit de winkel van je browser.",
  "ext.connect.desktopNotRunning.title": "Open Mosaic op deze computer",
  "ext.connect.desktopNotRunning.body":
    "De extensie werkt via de Mosaic-app. Start de app en controleer het daarna opnieuw.",
  "ext.connect.check": "Opnieuw controleren",
  "ext.connect.notPaired.title": "Verbinden met Mosaic",
  "ext.connect.notPaired.body":
    "Verbind deze browser met de Mosaic-app op deze computer. Je keurt het goed in de app.",
  "ext.connect.start": "Verbinden",
  "ext.connect.pairing.title": "Bevestig in Mosaic",
  "ext.connect.pairing.body":
    "Controleer of Mosaic dezelfde code toont en klik daar op Toestaan.",
  "ext.connect.pairing.code": "Code",
  "ext.connect.pairing.waiting": "Wachten op je antwoord in de Mosaic-app…",
  "ext.connect.protocolMismatch.title": "Mosaic bijwerken",
  "ext.connect.protocolMismatch.body":
    "De extensie en de Mosaic-app hebben verschillende versies. Werk beide bij naar de nieuwste versie.",
  "ext.connect.error.title": "Mosaic is niet bereikbaar",
  "ext.connect.error.body": "Controleer of de Mosaic-app draait en probeer het opnieuw.",
  "ext.locked.title": "Mosaic is vergrendeld",
  "ext.locked.body": "Ontgrendel het in de Mosaic-app om in te vullen op {host}.",
  "ext.locked.bodyNoSite": "Ontgrendel het in de Mosaic-app om het hier te gebruiken.",
  "ext.locked.open": "Mosaic openen",
  "ext.locked.note": "Je hoofdwachtwoord typ je alleen in de Mosaic-app.",
  "ext.logins.forSite": "Voor {host}",
  "ext.logins.count": "{count, plural, one {# login} other {# logins}}",
  "ext.logins.sameSite": "dezelfde site",
  "ext.logins.none": "Geen bewaarde login voor {host}.",
  "ext.logins.noneHint":
    "Mosaic vergelijkt het volledige adres, dus sites die erop lijken krijgen nooit een match.",
  "ext.logins.unsupported":
    "Mosaic kan op deze pagina niet invullen. Zoek in je kluis om een login te kopiëren.",
  "ext.logins.shortcut": "{shortcut} vult de eerste match in.",
  "ext.logins.frameNote":
    "Mosaic vult alleen in op {host}, nooit in ingebedde frames van andere sites.",
  "ext.item.untitled": "Zonder titel",
  "ext.item.noUsername": "Geen gebruikersnaam",
  "ext.item.twoFactor": "2FA",
  "ext.item.fill": "Invullen",
  "ext.item.fillLabel": "{name} invullen",
  "ext.item.copyMenu": "Kopieeropties voor {name}",
  "ext.item.copyUsername": "Gebruikersnaam kopiëren",
  "ext.item.copyPassword": "Wachtwoord kopiëren",
  "ext.item.copyCode": "Code kopiëren",
  "ext.item.blocked":
    "Geblokkeerd: deze pagina is niet versleuteld en deze login staat dat niet toe.",
  "ext.search.label": "Andere items",
  "ext.search.placeholder": "Zoeken in je kluis",
  "ext.search.note":
    "Resultaten blijven in dit venster. De pagina ontvangt alleen de ene login die je invult.",
  "ext.search.none": "Niets gevonden.",
  "ext.search.copy": "Kopiëren",
  "ext.search.copyOnly": "Alleen kopiëren: niet bewaard voor deze site",
  "ext.shortcut.copyUser": "{platform, select, mac {⌘C} other {Ctrl+C}}",
  "ext.shortcut.copyPassword": "{platform, select, mac {⇧⌘C} other {Ctrl+Shift+C}}",
  "ext.shortcut.copyCode": "{platform, select, mac {⌥⌘C} other {Ctrl+Alt+C}}",
  "ext.search.results": "{count, plural, one {# resultaat} other {# resultaten}}",
  "ext.frame.cross":
    "Het actieve veld staat in een frame van een andere site. Mosaic vult daar niet in.",
  "ext.frameConfirm.title": "Invullen in een ingebed frame?",
  "ext.frameConfirm.body":
    "Het veld staat in een frame op {host}, niet in de pagina zelf. Vul hier alleen in als je dit frame vertrouwt.",
  "ext.frameConfirm.confirm": "In het frame invullen",
  "ext.warning.idn.title": "Controleer het adres",
  "ext.warning.idn.body":
    "Dit adres gebruikt speciale tekens en kan op een andere site lijken. Controleer de adresbalk voordat je inlogt.",
  "ext.warning.lookalike.body":
    "Deze site is niet {saved}. Hij lijkt erop, wat een veelgebruikte phishingtruc is. Sluit dit tabblad, of typ {saved} zelf in.",
  "ext.warning.youAreOn": "Je bent op",
  "ext.warning.writtenAs": "Geschreven als",
  "ext.warning.savedFor": "Bewaard voor",
  "ext.warning.insecure.title": "Deze pagina is niet versleuteld",
  "ext.warning.insecure.body":
    "De pagina gebruikt http. Anderen op het netwerk kunnen je wachtwoord lezen. Mosaic vult hier alleen in voor logins waarvoor je dat hebt toegestaan.",
  "ext.otp.title": "Je eenmalige code invullen",
  "ext.otp.next": "Wachtwoord ingevuld voor {name}. Volgende: de code voor tweestapsverificatie.",
  "ext.otp.for": "Code voor {name}",
  "ext.otp.fill": "Code invullen",
  "ext.otp.copy": "Code kopiëren",
  "ext.otp.note":
    "De code gaat rechtstreeks van Mosaic naar het veld. Dit venster toont hem nooit.",
  "ext.save.titleNew": "Deze login voor {host} bewaren?",
  "ext.save.titleUpdate": "De bewaarde login voor {host} bijwerken?",
  "ext.save.generated": "Een wachtwoord dat Mosaic voor {host} heeft gegenereerd.",
  "ext.save.updates": "Werkt “{title}” bij.",
  "ext.save.username": "Gebruikersnaam",
  "ext.save.noUsername": "Geen",
  "ext.save.password": "Wachtwoord",
  "ext.save.passwordHidden": "Verborgen. Je ziet het in de Mosaic-app.",
  "ext.save.insecure": "Deze pagina is niet versleuteld (http).",
  "ext.save.action": "Bewaren in Mosaic…",
  "ext.save.notNow": "Niet nu",
  "ext.save.never": "Nooit voor deze site",
  "ext.save.waiting": "Bevestig in de Mosaic-app. Er wordt niets bewaard totdat je dat doet.",
  "ext.save.saved": "Bewaard in Mosaic.",
  "ext.save.updated": "Bijgewerkt in Mosaic.",
  "ext.save.cancelled": "Niet bewaard.",
  "ext.save.dismissed": "Verworpen. Het wachtwoord wordt vergeten.",
  "ext.save.neverDone": "Mosaic biedt niet meer aan om te bewaren op {host}.",
  "ext.save.permissionAsk":
    "Mosaic vraagt je browser om toegang tot websites. Sta het toe om dit aan te zetten.",
  "ext.generator.title": "Een wachtwoord genereren voor {host}",
  "ext.generator.titleGeneric": "Een wachtwoord genereren",
  "ext.generator.value": "Gegenereerd wachtwoord",
  "ext.generator.regenerate": "Een ander wachtwoord genereren",
  "ext.generator.type": "Wachtwoordtype",
  "ext.generator.random": "Willekeurig",
  "ext.generator.passphrase": "Wachtzin",
  "ext.generator.include": "Opnemen",
  "ext.generator.rules.page": "Volgt de wachtwoordregels van deze pagina",
  "ext.generator.rules.siteQuirks": "Volgt de bekende regels voor deze site",
  "ext.generator.fill": "Invullen",
  "ext.generator.fillHint":
    "Om in te vullen, klik je op de pagina in een veld voor een nieuw wachtwoord en open je dit venster opnieuw.",
  "ext.generator.copy": "Kopiëren",
  "ext.generator.expired":
    "Dat wachtwoord werd niet meer vastgehouden, dus er is een nieuw gegenereerd.",
  "ext.generator.filled": "Ingevuld. Mosaic biedt aan het te bewaren.",
  "ext.copied":
    "Gekopieerd. Wordt gewist over {seconds, plural, one {# seconde} other {# seconden}}.",
  "ext.copiedNoClear": "Gekopieerd.",
  "ext.filled": "Ingevuld.",
  "ext.filledCode": "Code ingevuld.",
  "ext.error.invalidRequest":
    "Mosaic begreep dat verzoek niet. Werk de extensie en de Mosaic-app bij.",
  "ext.error.notPaired": "Deze browser is niet met Mosaic verbonden. Verbind hem opnieuw.",
  "ext.error.protocolVersion":
    "De extensie en de Mosaic-app hebben verschillende versies. Werk beide bij.",
  "ext.error.busy": "Mosaic is bezig met een ander verzoek. Probeer het zo meteen opnieuw.",
  "ext.error.locked": "Mosaic is vergrendeld. Ontgrendel het in de Mosaic-app.",
  "ext.error.rateLimited":
    "Dat waren te veel verzoeken in korte tijd. Wacht even en probeer het opnieuw.",
  "ext.error.originMismatch":
    "Deze login is niet voor deze website bewaard, dus Mosaic vult hem hier niet in.",
  "ext.error.insecureOrigin":
    "Deze pagina is niet versleuteld en de login staat invullen daarop niet toe. Er is niets ingevuld.",
  "ext.error.crossOriginFrame":
    "Dat veld staat in een frame van een andere site. Mosaic vult daar niet in.",
  "ext.error.itemNotFound":
    "Dat item bestaat niet meer. Het is mogelijk verplaatst of verwijderd.",
  "ext.error.vaultUnavailable": "Die kluis is gesloten. Open hem eerst in de Mosaic-app.",
  "ext.error.noOtp": "Dit item heeft geen eenmalige code.",
  "ext.error.otpNotFillable":
    "Deze code telt bij elk gebruik op (HOTP), dus hij kan hier niet worden ingevuld of gekopieerd. Gebruik hem in de Mosaic-app.",
  "ext.error.fieldEmpty": "Dat veld is leeg, dus er is niets in te vullen of te kopiëren.",
  "ext.error.clipboardUnavailable":
    "Mosaic kon het klembord niet gebruiken. Probeer het opnieuw, of kopieer het in de Mosaic-app.",
  "ext.error.cancelled": "Geannuleerd in de Mosaic-app. Probeer het opnieuw als je klaar bent.",
  "ext.error.passkeyNotFound": "Mosaic heeft geen passkey voor deze site.",
  "ext.error.passkeyExcluded": "Je hebt al een passkey voor dit account in Mosaic.",
  "ext.error.rpIdMismatch":
    "Deze site kan geen passkey voor dat domein gebruiken, dus Mosaic heeft niet geantwoord.",
  "ext.error.passkeyUnsupported":
    "Mosaic kan met de opties die deze site vroeg geen passkey maken.",
  "ext.error.pairingDenied":
    "De verbinding is in de Mosaic-app geweigerd. Kies Verbinden om het opnieuw te proberen en kies dan Toestaan.",
  "ext.error.pairingExpired":
    "Het verbindingsverzoek is verlopen. Kies Verbinden om opnieuw te beginnen.",
  "ext.error.pairingConflict":
    "Er stond een ander verbindingsverzoek open. Kies Verbinden om het opnieuw te proberen.",
  "ext.error.pairingBlocked":
    "Er zijn te veel verbindingen geweigerd. Wacht tien minuten en probeer het opnieuw.",
  "ext.error.pairingLimit":
    "Deze computer heeft te veel verbonden browsers. Verbreek er een in de Mosaic-instellingen.",
  "ext.error.internal":
    "Mosaic liep tegen een onverwacht probleem aan. Probeer het opnieuw en start de Mosaic-app opnieuw als het zich herhaalt.",
  "ext.error.desktopNotRunning":
    "Mosaic draait niet op deze computer. Open het en probeer het opnieuw.",
  "ext.error.desktopNotAuthentic":
    "De app die antwoordde is niet de Mosaic-app waarmee deze browser is verbonden. Er is niets verstuurd.",
  "ext.error.timeout":
    "Mosaic antwoordde niet op tijd. Als er een dialoogvenster open staat in de Mosaic-app, beantwoord het daar.",
  "ext.error.frameTooLarge": "Dat waren te veel gegevens om naar Mosaic te sturen.",
  "ext.error.hostMissing":
    "De Mosaic-hulpmodule is niet geïnstalleerd voor deze browser. Installeer de Mosaic-app en start de browser opnieuw.",
  "ext.error.hostForbidden":
    "Deze browser of extensie mag niet met Mosaic praten.",
  "ext.error.hostExited": "De verbinding met Mosaic is gesloten. Probeer het opnieuw.",
  "ext.error.unsupportedPage": "Mosaic kan op dit soort pagina niet invullen.",
  "ext.error.noFields":
    "Geen loginvelden gevonden. Klik in een veld en probeer het opnieuw.",
  "ext.error.noMatch": "Mosaic heeft geen bewaarde login voor deze site.",
  "ext.error.frameNeedsConfirmation":
    "Het veld staat in een frame. Open het Mosaic-venster en bevestig het frame om in te vullen.",
  "ext.error.frameOpaque": "Het veld staat in een frame in een sandbox. Mosaic vult daar niet in.",
  "ext.error.originChanged":
    "De pagina veranderde terwijl Mosaic bezig was met invullen. Er is niets ingevuld. Probeer het opnieuw.",
  "ext.error.pageHidden": "De pagina is niet zichtbaar, dus Mosaic heeft niet ingevuld.",
  "ext.error.permissionDenied":
    "Mosaic heeft toegang tot websites nodig om logins op te merken, en die heb je niet toegestaan.",
  "ext.error.generatedExpired":
    "Mosaic heeft dat wachtwoord niet meer. Genereer een nieuw wachtwoord.",
  "ext.rotation.card.title": "Wachtwoordwijziging",
  "ext.rotation.card.for": "Voor {name}",
  "ext.rotation.card.started": "Gestart {when}",
  "ext.rotation.card.delivered": "Het nieuwe wachtwoord is al eenmaal ingevuld.",
  "ext.rotation.card.note":
    "Het nieuwe wachtwoord wordt alleen in de Mosaic-app getoond. Mosaic verstuurt het formulier nooit voor je.",
  "ext.rotation.card.list": "Lopende wachtwoordwijzigingen",
  "ext.rotation.change": "Wachtwoord wijzigen…",
  "ext.rotation.changeLabel": "Wachtwoord wijzigen… voor {name}",
  "ext.rotation.openPage": "Wijzigingspagina openen",
  "ext.rotation.fillChange": "Wijzigingsformulier invullen",
  "ext.rotation.fillPending": "Nieuw wachtwoord invullen om in te loggen",
  "ext.rotation.worked": "Het werkte",
  "ext.rotation.failed": "Het werkte niet",
  "ext.rotation.fresh": "Opnieuw beginnen met een nieuw wachtwoord",
  "ext.rotation.waiting": "Wachten op je in de Mosaic-app…",
  "ext.rotation.started": "Gestart. Kies Dit wachtwoord gebruiken in de Mosaic-app.",
  "ext.rotation.startCancelled": "Er is niets gestart.",
  "ext.rotation.filled":
    "Wijzigingsformulier ingevuld. Controleer het en verstuur het zelf.",
  "ext.rotation.opened": "De wijzigingspagina is geopend.",
  "ext.rotation.confirmed": "Klaar. Het nieuwe wachtwoord is nu in Mosaic bewaard.",
  "ext.rotation.keptOld": "Mosaic heeft het oude wachtwoord behouden.",
  "ext.rotation.resultCancelled": "Er is niets gewijzigd. De wijziging staat nog open.",
  "ext.rotation.offer.title": "Een verwant adres",
  "ext.rotation.offer.body":
    "Deze pagina staat op {host}, dat niet is bewaard voor {title}. Toevoegen zodat Mosaic daar kan invullen?",
  "ext.rotation.offer.add": "{host} toevoegen",
  "ext.rotation.offer.added": "Toegevoegd. De wijziging wordt nu hier getoond.",
  "ext.rotation.offer.cancelled": "Niet toegevoegd.",
  "ext.error.rotationNotFound":
    "Die wachtwoordwijziging staat niet meer open. Begin een nieuwe vanuit de login.",
  "ext.error.originNotRelated":
    "Dat adres staat niet op dezelfde site als deze login, dus Mosaic heeft het niet toegevoegd.",
  "ext.error.rotationLimit":
    "Deze login heeft al vier onafgeronde wijzigingen. Rond er eerst een af of annuleer er een in de Mosaic-app.",
  "ext.error.changeFormNotFound":
    "Mosaic vond hier geen formulier voor het wijzigen van een wachtwoord. Open de wachtwoordpagina van je account en probeer het opnieuw.",
  "ext.error.changeFormAmbiguous":
    "Mosaic kon niet bepalen welk veld welk is. Kopieer het nieuwe wachtwoord uit Mosaic en plak het zelf.",
} satisfies Record<keyof typeof en, string>;
