/**
 * Dutch UI strings: sharing. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { sharing as en } from "../en/sharing.ts";

export const sharing = {
  "sharing.title": "Delen",
  "sharing.body":
    "Deel kluizen met mensen die je kent. Alles blijft versleuteld: alleen de mensen die je uitnodigt kunnen een gedeelde kluis lezen, en Mosaic deelt alleen met mensen van wie je de veiligheidscode hebt vergeleken.",
  "sharing.notHosted.none":
    "Delen heeft een Mosaic-server nodig. Zet eerst synchronisatie met een Mosaic-server aan bij de synchronisatie-instellingen.",
  "sharing.notHosted.folder":
    "Delen is niet beschikbaar met een synchronisatiemap. Een map kan niet bepalen wie een kluis mag lezen, dus Mosaic houdt kluizen in een map privé voor jou. Om te delen verbreek je de verbinding met de map en verbind je een Mosaic-server bij de synchronisatie-instellingen.",
  "sharing.notHosted":
    "Delen heeft een Mosaic-server nodig. Verbind er eerst een bij de synchronisatie-instellingen.",
  "sharing.syncSettings": "Synchronisatie-instellingen openen",
  "sharing.ownCode.title": "Je veiligheidscode",
  "sharing.ownCode.body":
    "Lees deze code voor aan de persoon met wie je deelt, persoonlijk of via de telefoon. Als hij overeenkomt met wat die persoon voor jou ziet, zit er niemand tussen jullie in.",
  "sharing.ownCode.pending":
    "Je veiligheidscode verschijnt na de eerste synchronisatie met de server.",
  "sharing.contacts.title": "Mensen die je hebt opgezocht",
  "sharing.contacts.empty": "Je hebt nog niemand opgezocht.",
  "sharing.contacts.verified": "Geverifieerd",
  "sharing.contacts.notVerified": "Niet geverifieerd",
  "sharing.contacts.changed": "Moet opnieuw worden gecontroleerd",
  "sharing.contacts.verify": "Code vergelijken…",
  "sharing.contacts.verifyOf": "Code vergelijken met {email}",
  "sharing.contacts.review": "Wijziging bekijken…",
  "sharing.contacts.reviewOf": "Wijziging bekijken voor {email}",
  "sharing.share.open": "Delen…",
  "sharing.share.title": "{vault} delen",
  "sharing.share.dialog": "Delen: {vault}",
  "sharing.share.roleStep": "Kies een rol voor {email}",
  "sharing.lookup.email": "Het e-mailadres van de persoon",
  "sharing.lookup.find": "Opzoeken",
  "sharing.lookup.searching": "Opzoeken…",
  "sharing.lookup.notFoundHint":
    "Mosaic kan je niet vertellen of een adres een account heeft. Als de code hierboven niet overeenkomt met wat de persoon ziet, deel dan niet.",
  "sharing.code.title": "Veiligheidscodes vergelijken",
  "sharing.code.compare":
    "Vergelijk deze code met {email}, persoonlijk of via de telefoon. Gebruik geen e-mail of chat.",
  "sharing.code.compareNew":
    "Vergelijk de nieuwe code met {email}, persoonlijk of via de telefoon. Gebruik geen e-mail of chat.",
  "sharing.code.shown": "Code die Mosaic toont voor {email}",
  "sharing.code.typed": "Code die ze aan jou voorlezen (optioneel)",
  "sharing.code.typedMatch": "Dit is de code hierboven.",
  "sharing.code.attest":
    "Ik heb deze code per telefoon of persoonlijk met {email} vergeleken en hij is hetzelfde.",
  "sharing.code.matches": "Codes komen overeen",
  "sharing.code.mismatch":
    "De codes verschillen. Deel niet. Mogelijk zit er iemand tussen jou en {email}.",
  "sharing.code.mismatchHelp":
    "Controleer het adres en vraag {email} de code opnieuw voor te lezen. Verschilt hij nog steeds, annuleer dan en vertel het de persoon op een andere manier.",
  "sharing.code.verified": "{email} is geverifieerd.",
  "sharing.code.status.new": "Nieuw contact",
  "sharing.code.status.pinned": "Nog niet geverifieerd",
  "sharing.code.status.verified": "Geverifieerd",
  "sharing.code.status.changed": "Sleutel gewijzigd",
  "sharing.code.status.conflicting": "Apparaten zijn het oneens",
  "sharing.code.conflicting":
    "Je apparaten zagen verschillende sleutels voor {email}. Vergelijk de code opnieuw met de persoon voordat je deelt.",
  "sharing.verify.title": "{email} verifiëren",
  "sharing.role.label": "Rol",
  "sharing.role.owner": "Eigenaar",
  "sharing.role.manage": "Beheerder",
  "sharing.role.write": "Bewerker",
  "sharing.role.read": "Lezer",
  "sharing.role.desc.manage": "Kan mensen uitnodigen en verwijderen en rollen wijzigen.",
  "sharing.role.desc.write": "Kan items toevoegen en wijzigen.",
  "sharing.role.desc.read": "Kan alleen lezen.",
  "sharing.role.honesty":
    "Iedereen die een item kan lezen, kan het kopiëren. Een lezer kan de kluis niet wijzigen, maar niets weerhoudt de lezer ervan te bewaren wat hij ziet, en iemand later verwijderen maakt niet ongedaan wat al is gedownload.",
  "sharing.invite.send": "Uitnodiging versturen",
  "sharing.invite.sent":
    "Uitnodiging verstuurd naar {email}. De persoon moet Mosaic openen en hem accepteren.",
  "sharing.invite.sentTitle": "Uitnodiging verstuurd",
  "sharing.invite.pending": "Wachten op {email}",
  "sharing.invite.expires": "Verloopt {when}",
  "sharing.invite.cancel": "Uitnodiging intrekken",
  "sharing.invite.cancelOf": "Uitnodiging voor {email} intrekken",
  "sharing.invite.cancelled": "Uitnodiging ingetrokken.",
  "sharing.inbox.title": "Uitnodigingen",
  "sharing.inbox.nav": "Uitnodigingen",
  "sharing.inbox.navCount": "Uitnodigingen, {count} wachtend",
  "sharing.inbox.intro":
    "Mensen die je hebben uitgenodigd voor een gedeelde kluis. Mosaic controleert elke uitnodiging voordat hij die aan je toont.",
  "sharing.inbox.empty":
    "Op dit moment geen uitnodigingen. Wanneer iemand je uitnodigt voor een gedeelde kluis, verschijnt die hier.",
  "sharing.inbox.from": "Van {email}",
  "sharing.inbox.review": "Bekijken",
  "sharing.inbox.reviewOf": "Uitnodiging van {email} bekijken",
  "sharing.inbox.refresh": "Vernieuwen",
  "sharing.inbox.preview.title": "Deelnemen aan {vault}?",
  "sharing.inbox.preview.body": "{email} nodigt je uit als {role}.",
  "sharing.inbox.preview.unnamed": "een gedeelde kluis",
  "sharing.inbox.preview.owner": "De kluis is van {email}.",
  "sharing.inbox.verifySender":
    "Vergelijk voordat je deelneemt de veiligheidscode van {email}.",
  "sharing.inbox.verifyOwner":
    "{sender} is niet de eigenaar. Vergelijk voordat je deelneemt ook de veiligheidscode van de eigenaar: de eigenaar kan de sleutel van de kluis wijzigen.",
  "sharing.inbox.ownerCode": "Code voor de eigenaar, {email}",
  "sharing.inbox.accept": "Deelnemen aan kluis",
  "sharing.inbox.readyTitle": "Alles gecontroleerd. Deelnemen aan deze kluis?",
  "sharing.inbox.decline": "Weigeren",
  "sharing.inbox.block": "Weigeren en afzender blokkeren",
  "sharing.inbox.block.title": "Deze afzender weigeren en blokkeren?",
  "sharing.inbox.block.body":
    "Mosaic weigert de uitnodiging en negeert voortaan elke uitnodiging van {email}. Je kunt dit in Mosaic niet ongedaan maken.",
  "sharing.inbox.unreadable":
    "Deze uitnodiging kan niet worden geopend. Hij is mogelijk beschadigd of niet voor jou bedoeld. Vraag {email} hem opnieuw te sturen.",
  "sharing.inbox.joined":
    "Je bent lid geworden van de kluis. Hij verschijnt zo meteen in je kluizenlijst.",
  "sharing.inbox.declined": "Uitnodiging geweigerd.",
  "sharing.inbox.outgoing": "Uitnodigingen die je hebt verstuurd",
  "sharing.inbox.outgoing.empty": "Geen van je uitnodigingen wacht op antwoord.",
  "sharing.inbox.unknownVault": "een kluis",
  "sharing.members.title": "Mensen met toegang",
  "sharing.members.you": "Jij",
  "sharing.members.unverified": "Niet geverifieerd",
  "sharing.members.unconfirmed": "Niet op de ledenlijst",
  "sharing.members.unconfirmed.help":
    "De server vermeldt deze persoon, maar de eigen ledenlijst van de kluis niet. De volgende sleutelwijziging sluit de persoon buiten. Als je de persoon niet kent, verwijder haar of hem dan.",
  "sharing.members.dropped": "Niet meer op de server",
  "sharing.members.dropped.help":
    "Deze persoon staat op de ledenlijst van de kluis, maar de server vermeldt haar of hem niet meer. De persoon is mogelijk weggegaan. Controleer of dit verwacht is.",
  "sharing.members.invited": "Uitgenodigd",
  "sharing.members.removing": "Wordt verwijderd",
  "sharing.members.verified": "Geverifieerd",
  "sharing.members.verify": "Verifiëren…",
  "sharing.members.verifyOf": "{email} verifiëren",
  "sharing.members.remove": "Verwijderen",
  "sharing.members.removeOf": "{email} verwijderen",
  "sharing.members.remove.title": "{email} verwijderen?",
  "sharing.members.remove.confirm":
    "{email} houdt alles wat al is gedownload. Mosaic wijzigt nu de kluissleutel en de persoon kan niets nieuws lezen.",
  "sharing.members.remove.confirmManager":
    "{email} houdt alles wat al is gedownload. De server stuurt de persoon nu geen updates meer. Het apparaat van de eigenaar van de kluis wijzigt de kluissleutel de volgende keer dat het synchroniseert, en tot die tijd houdt alleen de server {email} buiten.",
  "sharing.members.cancel": "Annuleren",
  "sharing.members.removed": "Verwijderd.",
  "sharing.members.leave": "Kluis verlaten",
  "sharing.members.leave.title": "Deze kluis verlaten?",
  "sharing.members.leave.confirm":
    "De kopie op dit apparaat blijft leesbaar, maar wordt niet meer bijgewerkt. Om opnieuw deel te nemen heb je een nieuwe uitnodiging nodig.",
  "sharing.members.left": "Je hebt de kluis verlaten.",
  "sharing.members.roleOf": "Rol van {email}",
  "sharing.members.changeRole": "Rol wijzigen",
  "sharing.members.changeRoleOf": "Rol van {email} wijzigen",
  "sharing.members.invite": "Iemand uitnodigen…",
  "sharing.members.empty": "Tot nu toe kan alleen jij deze kluis openen.",
  "sharing.members.unknownEmail": "Onbekend adres",
  "sharing.members.managerNote":
    "Je kunt mensen uitnodigen en verwijderen. Alleen het apparaat van de eigenaar wijzigt de kluissleutel, dus een verwijdering is voltooid zodra het apparaat van de eigenaar heeft gesynchroniseerd.",
  "sharing.members.readerNote":
    "Je kunt deze kluis lezen, maar niet wijzigen wie toegang heeft.",
  "sharing.rotation.running":
    "Een verwijdering wacht tot het apparaat van de eigenaar de kluissleutel wijzigt. Totdat het synchroniseert, houdt alleen de server de verwijderde persoon buiten.",
  "sharing.rotation.unprotected":
    "{count, plural, one {# bestand behoudt} other {# bestanden behouden}} de oude bescherming omdat dit apparaat {count, plural, one {het} other {ze}} niet heeft. Een persoon die je hebt verwijderd kan {count, plural, one {het} other {ze}} nog openen als die de oude sleutel heeft bewaard. Voeg {count, plural, one {het} other {ze}} opnieuw toe vanaf een apparaat dat {count, plural, one {het} other {ze}} wel heeft.",
  "sharing.keyChanged.title": "De veiligheidscode van {email} is gewijzigd",
  "sharing.keyChanged.body":
    "De server toont nu een andere sleutel voor {email}. Dat gebeurt wanneer de persoon Mosaic opnieuw heeft ingesteld, en zo zou ook een aanval op de server eruitzien. Er wordt niets gedeeld met de nieuwe sleutel totdat je de codes met de persoon hebt vergeleken, persoonlijk of via de telefoon.",
  "sharing.keyChanged.old": "Code die je eerder hebt vergeleken",
  "sharing.keyChanged.new": "Nieuwe code",
  "sharing.keyChanged.accept": "Nieuwe code komt overeen",
  "sharing.finding.region": "Deelwaarschuwingen",
  "sharing.finding.summary":
    "{count, plural, one {# deelwaarschuwing} other {# deelwaarschuwingen}}",
  "sharing.finding.review": "Bekijken",
  "sharing.finding.collapse": "Samenvouwen",
  "sharing.finding.dismiss": "Sluiten",
  "sharing.finding.unknownEmail": "een onbekende persoon",
  "sharing.finding.verify": "Verifiëren…",
  "sharing.finding.remove": "Verwijderen",
  "sharing.finding.removeOf": "{email} uit {vault} verwijderen",
  "sharing.finding.identityMismatch":
    "De server toont een andere deelsleutel voor je account dan dit apparaat heeft gemaakt, dus delen is gepauzeerd. Deel niets en controleer je server en je andere apparaten. Kies daarna “Ik heb het gecontroleerd…”.",
  "sharing.finding.rosterInvalid":
    "De ledenlijst van {vault} is beschadigd, of iemand heeft hem zonder toestemming gewijzigd. Voeg geen mensen toe totdat je hebt gecontroleerd wie toegang heeft. Kies daarna “Ik heb het gecontroleerd…”.",
  "sharing.finding.shareUntrusted":
    "Een nieuwe sleutel voor {vault} is geweigerd: {sender} stuurde hem, maar mag de sleutel van deze kluis niet wijzigen. De kluis blijft op zijn oude sleutel. Vraag de eigenaar of die hem heeft gewijzigd. Kies daarna “Ik heb het gecontroleerd…”.",
  "sharing.finding.memberUnverified":
    "Je hebt {email} in {vault} niet geverifieerd. Mosaic stuurt de persoon geen nieuwe sleutels en de volgende sleutelwijziging sluit de persoon buiten. Vergelijk de veiligheidscode of verwijder de persoon.",
  "sharing.finding.memberDropped":
    "{email} staat op de ledenlijst van {vault}, maar de server vermeldt de persoon niet meer. De persoon is mogelijk weggegaan, of de server heeft haar of hem verwijderd. Controleer of dit verwacht is en verwijder de persoon als dat niet zo is.",
  "sharing.finding.memberKeyMismatch":
    "Mosaic kreeg een sleutel voor {email} die niet overeenkomt met de sleutel die je hebt geverifieerd, dus de persoon is uit {vault} verwijderd. Vergelijk opnieuw veiligheidscodes met de persoon voordat je opnieuw uitnodigt.",
  "sharing.finding.vaultHeld":
    "{vault} wacht totdat je andere apparaten bevestigen wie de eigenaar is. Open Mosaic op een apparaat dat deze kluis al gebruikt.",
  "sharing.finding.accessLost":
    "Je hebt geen toegang meer tot {vault}. De kopie op dit apparaat blijft leesbaar maar wordt niet bijgewerkt. Vraag de eigenaar je opnieuw uit te nodigen.",
  "sharing.finding.rotationBlocked":
    "De sleutel van {vault} kon niet worden gewijzigd nadat je iemand had verwijderd, dus die persoon heeft nog de oude sleutel. Synchroniseer opnieuw. Blijft dit zo, open dan de ledenlijst en controleer wie toegang heeft.",
  "emergency.title": "Noodtoegang",
  "emergency.body":
    "Noem mensen die toegang tot je kluizen kunnen vragen als je iets overkomt. Dit is geen accountherstel: ze krijgen alleen-lezentoegang tot de kluizen die je kiest, pas na een wachttijd, en je kunt in die tijd weigeren. Het vervangt je Noodkit niet.",
  "emergency.folderUnsupported":
    "Noodtoegang heeft de Mosaic-server nodig. Een synchronisatiemap kan een wachttijd niet afdwingen.",
  "emergency.granted.title": "Mensen die toegang kunnen aanvragen",
  "emergency.granted.empty": "Niemand kan toegang tot je kluizen aanvragen.",
  "emergency.granted.add": "Een persoon toevoegen…",
  "emergency.granted.scopeAll": "Al je kluizen",
  "emergency.granted.scopeSome": "{count, plural, one {# kluis} other {# kluizen}}",
  "emergency.granted.wait": "Wacht {days, plural, one {# dag} other {# dagen}}",
  "emergency.granted.waitingUntil":
    "Heeft toegang gevraagd. Die gaat open {when}, tenzij je weigert.",
  "emergency.granted.approved": "Je hebt het verzoek goedgekeurd.",
  "emergency.granted.released": "De toegang is vrijgegeven. De persoon kan de kluizen lezen.",
  "emergency.granted.idle": "Geen verzoek",
  "emergency.granted.remove": "Toegang verwijderen",
  "emergency.granted.removeOf": "Toegang voor {email} verwijderen",
  "emergency.granted.remove.title": "Noodtoegang voor {email} verwijderen?",
  "emergency.granted.remove.body":
    "De persoon kan niets meer aanvragen. Als de toegang al was vrijgegeven, wijzigt Mosaic de sleutels van de kluizen die de persoon heeft geopend; de persoon houdt wat al is gedownload.",
  "emergency.granted.removed": "Noodtoegang verwijderd.",
  "emergency.grant.title": "Noodtoegang toestaan",
  "emergency.grant.lookup": "Wie moet kunnen aanvragen?",
  "emergency.grant.optionsTitle": "Wat {email} kan openen",
  "emergency.grant.scope": "Wat de persoon kan openen",
  "emergency.grant.scope.all": "Al mijn kluizen, ook nieuwe",
  "emergency.grant.scope.selected": "Alleen de kluizen die ik kies",
  "emergency.grant.scope.pick": "Kies minstens één kluis.",
  "emergency.grant.wait": "Wachttijd",
  "emergency.grant.wait.option": "{days, plural, one {# dag} other {# dagen}}",
  "emergency.grant.waitHint":
    "Open Mosaic na een verzoek minstens één keer binnen deze tijd, zodat je het kunt zien en weigeren. Als de tijd om is, krijgt de persoon alleen-lezentoegang zonder dat jou nog iets wordt gevraagd.",
  "emergency.grant.serverTrust":
    "De server houdt de wachttijd bij. Als de persoon die je noemt en wie de server beheert samenwerken, kunnen ze hem overslaan. De server alleen kan je kluizen niet lezen.",
  "emergency.grant.vaultKeysOnly":
    "Dit geldt alleen voor de sleutels van je kluizen. Het geeft geen toegang tot je account, je hoofdwachtwoord of je Geheime Sleutel.",
  "emergency.grant.submit": "Noodtoegang toestaan",
  "emergency.grant.done": "{email} kan nu noodtoegang aanvragen.",
  "emergency.grant.needVerify": "Vergelijk eerst de veiligheidscode van {email}.",
  "emergency.received.title": "Mensen die jou hebben genoemd",
  "emergency.received.empty": "Niemand heeft jou voor noodtoegang genoemd.",
  "emergency.received.request": "Toegang aanvragen",
  "emergency.received.requestOf": "Toegang aanvragen bij {email}",
  "emergency.received.request.title": "{email} om toegang vragen?",
  "emergency.received.request.body":
    "{email} ziet je verzoek de volgende keer dat de persoon Mosaic opent. Na {days, plural, one {# dag} other {# dagen}} kun je de kluizen van de persoon openen, alleen-lezen, tenzij de persoon weigert.",
  "emergency.received.requested":
    "Verzoek verstuurd. {email} ziet het de volgende keer dat de persoon Mosaic opent.",
  "emergency.received.requestHint":
    "{email} ziet je verzoek in Mosaic. Na {days, plural, one {# dag} other {# dagen}} kun je de kluizen van de persoon openen, alleen-lezen, tenzij de persoon weigert.",
  "emergency.received.countdown": "Je kunt hun kluizen openen {when}.",
  "emergency.received.waitingLive": "Je verzoek wacht.",
  "emergency.received.claim": "Hun kluizen openen",
  "emergency.received.claimOf": "Hun kluizen openen: {email}",
  "emergency.received.claimed":
    "{count, plural, one {# kluis is} other {# kluizen zijn}} toegevoegd, alleen-lezen.",
  "emergency.received.cooldown": "{email} heeft geweigerd. Je kunt opnieuw vragen {when}.",
  "emergency.received.verifyFirst":
    "Vergelijk de veiligheidscode van {email} voordat je hun kluizen opent. Mosaic gebruikt alleen een sleutel die je hebt geverifieerd.",
  "emergency.received.verify": "Code vergelijken…",
  "emergency.received.verifyOf": "Code vergelijken met {email}",
  "emergency.received.approved": "{email} heeft je verzoek goedgekeurd.",
  "emergency.received.waitDays": "Wacht {days, plural, one {# dag} other {# dagen}}",
  "emergency.banner.region": "Verzoek om noodtoegang",
  "emergency.banner.body":
    "{email} heeft noodtoegang tot je kluizen gevraagd. Die gaat open {when}, tenzij je weigert.",
  "emergency.banner.refuse": "Weigeren",
  "emergency.banner.approve": "Nu goedkeuren…",
  "emergency.banner.approve.title": "Noodtoegang nu goedkeuren?",
  "emergency.banner.approve.body":
    "{email} krijgt meteen alleen-lezentoegang tot je kluizen. Voer je hoofdwachtwoord in om te bevestigen.",
  "emergency.banner.approve.confirm": "Goedkeuren",
  "emergency.banner.refused": "Verzoek geweigerd.",
  "emergency.banner.approved": "Toegang goedgekeurd.",
} satisfies Record<keyof typeof en, string>;
