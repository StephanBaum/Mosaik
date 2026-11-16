/**
 * Dutch UI strings: rotation. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { rotation as en } from "../en/rotation.ts";

export const rotation = {
  "rotation.section.title": "Wachtwoord en sleutels",
  "rotation.section.body":
    "Wijzig je hoofdwachtwoord, vraag een nieuwe Geheime Sleutel aan of verplaats je hele account naar nieuwe sleutels. Elke wijziging laat je eerst zien wat er gebeurt voordat er iets wordt aangeraakt.",
  "rotation.action.password": "Hoofdwachtwoord wijzigen",
  "rotation.action.password.body":
    "Kies een nieuw hoofdwachtwoord. Je Geheime Sleutel en je kluizen blijven zoals ze zijn.",
  "rotation.action.password.button": "Wijzigen…",
  "rotation.action.secretKey": "Geheime Sleutel vernieuwen",
  "rotation.action.secretKey.body":
    "Vraag een nieuwe Geheime Sleutel en een nieuwe Noodkit aan. Gebruik dit als je kit kwijt is of iemand hem mogelijk heeft gezien.",
  "rotation.action.secretKey.button": "Vernieuwen…",
  "rotation.action.profile": "Bescherming verbeteren",
  "rotation.action.profile.body":
    "Maak het raden van je hoofdwachtwoord moeilijker door bij elke poging meer geheugen te gebruiken. Ontgrendelen duurt iets langer.",
  "rotation.action.profile.button": "Verbeteren…",
  "rotation.action.rekey": "Account opnieuw coderen",
  "rotation.action.rekey.body":
    "Vernieuw elke sleutel van je account: een nieuwe Geheime Sleutel, nieuwe kluissleutels en een nieuwe veiligheidscode. Gebruik dit als iemand je kluisgegevens mogelijk samen met je wachtwoord en Geheime Sleutel heeft gekopieerd.",
  "rotation.action.rekey.button": "Opnieuw coderen…",
  "rotation.action.rekey.needsSync":
    "Bij het opnieuw coderen wordt de kopie op je synchronisatieserver of -map vernieuwd, dus er is synchronisatie voor nodig. Stel eerst synchronisatie in.",
  "rotation.title.password": "Hoofdwachtwoord wijzigen",
  "rotation.title.secretKey": "Geheime Sleutel vernieuwen",
  "rotation.title.profile": "Bescherming verbeteren",
  "rotation.title.rekey": "Account opnieuw coderen",
  "rotation.step": "Stap {current} van {total}",
  "rotation.back": "Terug",
  "rotation.stop.title": "Deze wijziging stoppen?",
  "rotation.stop.body":
    "De Noodkit die je zojuist hebt bewaard, werkt dan niet. Houd je oude kit.",
  "rotation.stop.confirm": "Wijziging stoppen",
  "rotation.stop.keep": "Doorgaan",
  "rotation.next": "Doorgaan",
  "rotation.close": "Sluiten",
  "rotation.review.title.password": "Voordat je je wachtwoord wijzigt",
  "rotation.review.title.secretKey": "Voordat je je Geheime Sleutel vernieuwt",
  "rotation.review.title.profile": "Voordat je de bescherming verbetert",
  "rotation.review.title.rekey": "Is er iets gecompromitteerd?",
  "rotation.review.changes": "Wat verandert",
  "rotation.review.stays": "Wat gelijk blijft",
  "rotation.review.password.changes":
    "Je hoofdwachtwoord. Snel ontgrendelen op dit apparaat wordt uitgezet en je kunt het daarna weer instellen.",
  "rotation.review.password.stays":
    "Je Geheime Sleutel, je kluizen en items, je veiligheidscode en je noodcontacten.",
  "rotation.review.password.devices":
    "Je andere apparaten worden uit de synchronisatie uitgelogd. Typ op elk ervan eenmalig het nieuwe hoofdwachtwoord en de synchronisatie gaat door.",
  "rotation.review.secretKey.changes":
    "Je Geheime Sleutel en je Noodkit. Snel ontgrendelen op dit apparaat wordt uitgezet en je kunt het daarna weer instellen.",
  "rotation.review.secretKey.stays":
    "Je hoofdwachtwoord, je kluizen en items, je veiligheidscode en je noodcontacten. Beveiligingssleutels die je hebt geregistreerd blijven werken.",
  "rotation.review.secretKey.devices":
    "Je andere apparaten worden uit de synchronisatie uitgelogd. Typ op elk ervan eenmalig je wachtwoord en de nieuwe Geheime Sleutel uit de nieuwe kit.",
  "rotation.review.secretKey.limit":
    "Dit stopt het inloggen met de oude Geheime Sleutel. Wie je kluisgegevens al met je oude wachtwoord en Geheime Sleutel heeft gekopieerd, kan ze nog steeds lezen. Codeer het account opnieuw om dat te stoppen.",
  "rotation.review.secretKey.compromised":
    "Er is iets gecompromitteerd: codeer in plaats daarvan opnieuw",
  "rotation.review.profile.changes":
    "Hoeveel geheugen elke ontgrendelpoging nodig heeft, voor jou en voor iedereen die probeert te raden. Snel ontgrendelen op dit apparaat wordt uitgezet en je kunt het daarna weer instellen.",
  "rotation.review.profile.stays":
    "Je hoofdwachtwoord, je Geheime Sleutel, je kluizen en je veiligheidscode. Beveiligingssleutels die je hebt geregistreerd blijven werken.",
  "rotation.review.profile.devices":
    "Je andere apparaten worden uit de synchronisatie uitgelogd. Typ op elk ervan eenmalig je hoofdwachtwoord en de synchronisatie gaat door. Tragere apparaten hebben iets meer tijd nodig om te ontgrendelen.",
  "rotation.review.rekey.intro":
    "Opnieuw coderen is bedoeld voor het geval dat iemand je kluisgegevens en je wachtwoord en Geheime Sleutel mogelijk heeft. Als je alleen een apparaat kwijt bent, volstaat het om de Geheime Sleutel te vernieuwen of je wachtwoord te wijzigen.",
  "rotation.review.rekey.yes": "Ja, mijn account opnieuw coderen",
  "rotation.review.rekey.no": "Nee, vernieuw in plaats daarvan alleen de Geheime Sleutel",
  "rotation.review.rekey.limit":
    "Opnieuw coderen maakt niet ongedaan wat iemand al heeft gekopieerd. Oude kopieën op servers en in back-ups blijven met de oude sleutels versleuteld, en de oude sleutels openen ze nog steeds.",
  "rotation.review.rekey.title": "Wat opnieuw coderen doet",
  "rotation.review.rekey.vaults":
    "{count, plural, one {# kluis die alleen jij gebruikt krijgt een nieuwe sleutel.} other {# kluizen die alleen jij gebruikt krijgen nieuwe sleutels.}}",
  "rotation.review.rekey.keys":
    "Een nieuwe Geheime Sleutel, een nieuwe Noodkit, een nieuwe veiligheidscode en een nieuwe accountsleutel.",
  "rotation.review.rekey.devices":
    "Je andere apparaten worden uitgelogd en kunnen niet synchroniseren totdat je op elk ervan de nieuwe Geheime Sleutel en je nieuwe hoofdwachtwoord typt.",
  "rotation.review.rekey.shared":
    "{count, plural, one {Je deelt # kluis.} other {Je deelt # kluizen.}} Elke kluis krijgt een nieuwe sleutel.",
  "rotation.review.rekey.sharedRow": "{name}, {count, plural, one {# lid} other {# leden}}",
  "rotation.review.rekey.sharedMembers":
    "Vraag iedereen met wie je deelt om je nieuwe veiligheidscode te vergelijken. Totdat ze dat doen, krijgen ze geen nieuwe sleutels van jou en kunnen ze de vernieuwde kluis niet openen.",
  "rotation.review.rekey.memberVaults":
    "{count, plural, one {# kluis is van iemand anders.} other {# kluizen zijn van anderen.}} Hun eigenaars worden gevraagd ze te vernieuwen. Totdat ze dat doen, kunnen je oude sleutels ze nog lezen.",
  "rotation.review.rekey.grants":
    "{count, plural, one {# persoon heeft noodtoegang tot je kluizen.} other {# personen hebben noodtoegang tot je kluizen.}} Hun toegang wordt vernieuwd voor de nieuwe sleutels. Elk van hen moet ook je nieuwe veiligheidscode vergelijken.",
  "rotation.review.rekey.quick":
    "Snel ontgrendelen op dit apparaat wordt uitgezet. Je kunt het daarna weer instellen.",
  "rotation.review.rekey.hardwareKeys":
    "{count, plural, one {# beveiligingssleutel die je voor het ontgrendelen hebt geregistreerd wordt uitgezet.} other {# beveiligingssleutels die je voor het ontgrendelen hebt geregistreerd worden uitgezet.}} Je kunt ze daarna opnieuw registreren.",
  "rotation.review.rekey.hardwareVaults":
    "{count, plural, one {# met hardware beveiligde kluis wordt niet beïnvloed.} other {# met hardware beveiligde kluizen worden niet beïnvloed.}}",
  "rotation.review.rekey.attachments":
    "{count, plural, one {# bijlage staat niet op dit apparaat.} other {# bijlagen staan niet op dit apparaat.}} Ze worden eerst gedownload zodat ze opnieuw kunnen worden beschermd. Bijlagen die niet bereikbaar zijn, houden de oude sleutel.",
  "rotation.review.rekey.folder":
    "Je synchronisatiemap krijgt een nieuw accountbestand. Andere apparaten vinden daar een melding dat de sleutels zijn gewijzigd.",
  "rotation.review.rekey.unavailable":
    "Opnieuw coderen is niet beschikbaar: synchronisatie is niet ingesteld.",
  "rotation.review.loading": "Er wordt gecontroleerd wat opnieuw coderen raakt…",
  "rotation.review.rekey.needsClean":
    "Voor opnieuw coderen moet de synchronisatie bij zijn en vrij van waarschuwingen. Synchroniseer nu, bekijk eventuele waarschuwingen en probeer het opnieuw.",
  "rotation.credentials.title": "Bevestig dat jij het bent",
  "rotation.credentials.password.title": "Je hoofdwachtwoorden",
  "rotation.credentials.current": "Huidig hoofdwachtwoord",
  "rotation.credentials.new": "Nieuw hoofdwachtwoord",
  "rotation.credentials.confirm": "Typ het nieuwe hoofdwachtwoord nogmaals",
  "rotation.credentials.mismatch": "De twee wachtwoorden zijn verschillend.",
  "rotation.credentials.same":
    "Het nieuwe hoofdwachtwoord moet anders zijn dan het huidige.",
  "rotation.credentials.hint":
    "Een lange wachtzin is beter dan een korte ingewikkelde. Je kunt het niet herstellen: Mosaic kent het niet.",
  "rotation.credentials.code": "6-cijferige code uit je authenticator-app",
  "rotation.credentials.profile": "Beschermingsniveau",
  "rotation.credentials.profile.standard": "Standaard: 256 MB geheugen per poging",
  "rotation.credentials.profile.strong": "Sterk: 512 MB geheugen per poging",
  "rotation.credentials.profile.max": "Maximaal: 1 GB geheugen per poging",
  "rotation.credentials.profile.slow":
    "Ontgrendelen met dit niveau duurt op dit apparaat meer dan twee seconden. Andere, oudere apparaten zijn nog trager.",
  "rotation.credentials.profile.force": "Toch verbeteren",
  "rotation.kit.title": "Bewaar je nieuwe Noodkit",
  "rotation.kit.body":
    "Je nieuwe Geheime Sleutel staat in deze kit. Bewaar hem eerst: Mosaic wijzigt je account pas nadat je bevestigt dat je hem hebt, want zonder hem kun je op een nieuw apparaat niet inloggen.",
  "rotation.kit.save": "Nieuwe Noodkit bewaren",
  "rotation.kit.saved": "Bewaard. Bewaar hem op een veilige plek, los van je apparaten.",
  "rotation.kit.cancelled": "Er is nog niets bewaard.",
  "rotation.kit.verify":
    "Typ de laatste groep van je nieuwe Geheime Sleutel uit de bewaarde kit.",
  "rotation.kit.confirm": "Kit bevestigen",
  "rotation.kit.mistyped":
    "Dat is niet het einde van de nieuwe Geheime Sleutel. Controleer de bewaarde kit.",
  "rotation.kit.verified": "Kit bevestigd.",
  "rotation.confirm.title.password": "Je hoofdwachtwoord wijzigen",
  "rotation.confirm.title.secretKey": "Je Geheime Sleutel vernieuwen",
  "rotation.confirm.title.profile": "De bescherming verbeteren",
  "rotation.confirm.title.rekey": "Je account opnieuw coderen",
  "rotation.confirm.body.rekey":
    "Dit logt je andere apparaten uit en vernieuwt elke sleutel. Houd Mosaic open totdat het klaar is.",
  "rotation.confirm.body.secretKey":
    "Dit logt je andere apparaten uit de synchronisatie uit. Houd Mosaic open totdat het klaar is.",
  "rotation.confirm.verb.password": "Wachtwoord wijzigen",
  "rotation.confirm.verb.secretKey": "Geheime Sleutel vernieuwen",
  "rotation.confirm.verb.profile": "Bescherming verbeteren",
  "rotation.confirm.verb.rekey": "Account opnieuw coderen",
  "rotation.run.title": "Je sleutels worden vernieuwd…",
  "rotation.run.keepOpen":
    "Houd Mosaic open. Met veel bijlagen kan dit een paar minuten duren.",
  "rotation.progress.label": "Voortgang",
  "rotation.progress.count": "{done} van {total}",
  "rotation.progress.done": "klaar",
  "rotation.phase.prepared": "Voorbereiden",
  "rotation.phase.backendStarted": "Je account wordt gevraagd over te stappen op de nieuwe sleutels",
  "rotation.phase.backendAccepted": "Je account heeft de nieuwe sleutels geaccepteerd",
  "rotation.phase.folderWritten": "De synchronisatiemap heeft het nieuwe accountbestand",
  "rotation.phase.localCommitted": "Dit apparaat gebruikt nu de nieuwe sleutels",
  "rotation.phase.attachments": "Bijlagen worden opnieuw beschermd",
  "rotation.phase.uploading": "De beschermde gegevens worden verstuurd",
  "rotation.phase.done": "Klaar",
  "rotation.done.title.password": "Hoofdwachtwoord gewijzigd",
  "rotation.done.title.secretKey": "Geheime Sleutel vernieuwd",
  "rotation.done.title.profile": "Bescherming verbeterd",
  "rotation.done.title.rekey": "Account opnieuw gecodeerd",
  "rotation.done.body.password":
    "Gebruik voortaan het nieuwe hoofdwachtwoord. Je andere apparaten vragen er eenmalig om.",
  "rotation.done.body.secretKey":
    "Bewaar de nieuwe Noodkit veilig en verwijder de oude. Je andere apparaten vragen eenmalig om de nieuwe Geheime Sleutel.",
  "rotation.done.body.profile":
    "Ontgrendelen heeft nu meer geheugen nodig. Je andere apparaten vragen eenmalig om je wachtwoord.",
  "rotation.done.body.rekey":
    "Bewaar de nieuwe Noodkit veilig en verwijder de oude. Je andere apparaten vragen eenmalig om de nieuwe Geheime Sleutel en je nieuwe hoofdwachtwoord.",
  "rotation.done.members":
    "Vraag iedereen met wie je een kluis deelt om je nieuwe veiligheidscode te vergelijken. Totdat ze dat doen, krijgen ze geen nieuwe sleutels van jou.",
  "rotation.done.quick": "Snel ontgrendelen opnieuw instellen",
  "rotation.done.quick.body": "Snel ontgrendelen is uitgezet.",
  "rotation.done.hardware": "Beveiligingssleutels opnieuw registreren",
  "rotation.done.hardware.body":
    "{count, plural, one {# beveiligingssleutel is voor ontgrendelen uitgezet.} other {# beveiligingssleutels zijn voor ontgrendelen uitgezet.}}",
  "rotation.done.unreachable.note":
    "Deze bestanden konden niet worden gelezen en zijn niet opnieuw versleuteld. Ze houden hun oude bestandssleutel totdat je ze verwijdert of opnieuw toevoegt, dus wie je oude sleutels en een kopie van een bestand heeft, kan het nog steeds openen.",
  "rotation.done.unreachable.file": "{item}: {file}",
  "rotation.done.unreachable.unknownItem": "Onbekend item",
  "rotation.done.unreachable.unknownFile": "onbekend bestand",
  "rotation.done.unreachable.more":
    "{count, plural, one {en # bestand meer} other {en # bestanden meer}}",
  "rotation.done.unfinished":
    "Het laatste deel gaat later verder. Als Mosaic sluit voordat het klaar is, kies dan de volgende keer Sleutelrotatie voortzetten.",
  "rotation.resume.title": "Sleutelwijziging niet afgerond",
  "rotation.resume.body.password":
    "Een sleutelwijziging is afgebroken. Er gaat niets verloren. Kies Doorgaan om je wachtwoorden opnieuw te typen, of verwerp hem.",
  "rotation.resume.body.secretKey":
    "Het vernieuwen van je Geheime Sleutel is afgebroken. Er gaat niets verloren. Ga door om het af te ronden.",
  "rotation.resume.body.rekey":
    "Het opnieuw coderen van je account is afgebroken. Er gaat niets verloren. Ga door om het af te ronden.",
  "rotation.resume.body.profile":
    "Het verbeteren van de bescherming is afgebroken. Er gaat niets verloren. Ga door om het af te ronden.",
  "rotation.resume.continue": "Sleutelrotatie voortzetten",
  "rotation.resume.discard": "Verwerpen…",
  "rotation.resume.running": "Doorgaan…",
  "rotation.resume.progress": "{done} van {total} klaar. Houd Mosaic open totdat het klaar is.",
  "rotation.resume.dialog.title": "Sleutelrotatie voortzetten",
  "rotation.resume.dialog.password": "Hoofdwachtwoord dat je nu hebt",
  "rotation.resume.dialog.group": "Laatste groep van de nieuwe Geheime Sleutel",
  "rotation.resume.dialog.body":
    "De sleutels van je account zijn mogelijk al gewijzigd. Om zeker te zijn dat jij het was, typ je je hoofdwachtwoord en de laatste groep van de nieuwe Geheime Sleutel uit de nieuwe Noodkit.",
  "rotation.resume.dialog.submit": "Doorgaan",
  "rotation.resume.discard.title": "Deze sleutelwijziging verwerpen?",
  "rotation.resume.discard.body":
    "De nieuwe Geheime Sleutel en de nieuwe Noodkit van deze poging worden verwijderd. Je account blijft zoals het is.",
  "rotation.resume.discard.confirm": "Sleutelwijziging verwerpen",
  "rotation.resume.abandon": "De rest opgeven…",
  "rotation.resume.stalled":
    "Doorgaan is niet afgerond. Als het blijft mislukken, kun je hier stoppen.",
  "rotation.abandon.title": "Stoppen met het opnieuw versleutelen van je bestanden?",
  "rotation.abandon.body":
    "Je nieuwe sleutels zijn al in gebruik. Mosaic stopt met het opnieuw versleutelen van de bijlagen die nog wachten. Ze houden hun oude bestandssleutel, dus wie je oude sleutels en een kopie van een bestand heeft, kan het nog steeds openen.",
  "rotation.abandon.stays":
    "Alles wat al opnieuw is versleuteld, blijft zoals het is. Verwijder de wachtende bestanden later of voeg ze opnieuw toe om ze met de nieuwe sleutel te beschermen.",
  "rotation.abandon.confirm": "Stoppen met opnieuw versleutelen",
  "rotation.abandon.keep": "Blijven proberen",
  "rotation.marker.title": "Onbevestigde melding over opnieuw coderen",
  "rotation.marker.hint":
    "Als je je sleutels niet op een ander apparaat hebt vernieuwd, is deze melding mogelijk niet echt. Je kunt hem negeren.",
  "rotation.marker.open": "Deze melding negeren…",
  "rotation.marker.open.label": "Deze melding negeren: {name}",
  "rotation.marker.confirm":
    "Mosaic vond een melding over opnieuw coderen die het niet kon bevestigen. Als je je account niet op een ander apparaat opnieuw hebt gecodeerd, kun je hem negeren.",
  "rotation.marker.ignore": "Melding negeren",
  "rotation.marker.keep": "Melding behouden",
  "rotation.ignored.title": "Genegeerde meldingen over opnieuw coderen",
  "rotation.ignored.body":
    "Meldingen die je hebt genegeerd. Zet er een terug als je je account wel op een ander apparaat opnieuw hebt gecodeerd.",
  "rotation.ignored.row": "Melding over opnieuw coderen uit je synchronisatiemap",
  "rotation.ignored.restore": "Terugzetten",
  "rotation.ignored.restore.label": "De melding {name} terugzetten",
  "rotation.resume.done": "Sleutelwijziging afgerond.",
  "rotation.recovery.title": "De sleutels van je account zijn op een ander apparaat vernieuwd",
  "rotation.recovery.body":
    "Een apparaat van jou heeft de sleutels van dit account vernieuwd. Om te blijven synchroniseren, typ je je hoofdwachtwoord en de Geheime Sleutel uit de nieuwe Noodkit.",
  "rotation.recovery.password": "Hoofdwachtwoord (het nieuwe, als je het daar hebt gewijzigd)",
  "rotation.recovery.secretKey": "Geheime Sleutel uit de nieuwe Noodkit",
  "rotation.recovery.submit": "De nieuwe sleutels gebruiken",
  "rotation.recovery.warning":
    "Heb je je sleutels niet zelf vernieuwd? Dan heeft iemand met je oude wachtwoord en Geheime Sleutel dat mogelijk gedaan. Voer hier niets in; controleer eerst je andere apparaten.",
  "rotation.recovery.notice":
    "Synchronisatie is gepauzeerd totdat je dit op dit apparaat doet.",
  "rotation.recovery.open": "De nieuwe sleutels gebruiken…",
  "rotation.recovery.keyHint":
    "Als je sleutels op een ander apparaat zijn vernieuwd, voer dan ook de nieuwe Geheime Sleutel uit de bijbehorende Noodkit in.",
  "rotation.recovery.done": "Dit apparaat gebruikt nu de nieuwe sleutels.",
  "rotation.recovery.optional":
    "Geheime Sleutel uit de nieuwste Noodkit (alleen als je hem hebt vernieuwd)",
  "rotation.file.unknown":
    "Er is een onbekend accountbestand in je synchronisatiemap verschenen. Het kan niet met je sleutels worden gecontroleerd: het kan een overblijfsel zijn, of iemand heeft je map gewijzigd. Je gegevens worden niet beïnvloed.",
  "rotation.file.dismiss": "Sluiten",
  "rotation.file.conflict.title": "Twee apparaten hebben tegelijk de sleutels vernieuwd",
  "rotation.file.conflict":
    "Een ander apparaat heeft de sleutels van je account op hetzelfde moment als dit apparaat vernieuwd, dus de twee komen niet meer overeen. Codeer opnieuw vanaf dit apparaat om ze weer gelijk te trekken: de sleutels van dit apparaat winnen dan en het andere vraagt om de nieuwe Geheime Sleutel.",
  "rotation.file.conflict.rekey": "Opnieuw coderen vanaf dit apparaat",
  "rotation.file.conflict.keep": "Melding verbergen",
  "rotation.resume.body.vault":
    "Het opnieuw versleutelen van de bijlagen van een kluis is afgebroken. Er gaat niets verloren. Ga door om het af te ronden.",
} satisfies Record<keyof typeof en, string>;
