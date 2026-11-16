/**
 * Dutch UI strings: vault. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { vault as en } from "../en/vault.ts";

export const vault = {
  "vault.itemCount": "{count, plural, =0 {Geen items} one {# item} other {# items}}",
  "vault.deleteConfirm":
    "“{title}” verwijderen? Je kunt het 30 dagen lang terugzetten uit de prullenmand.",
  "vault.defaultName": "Persoonlijk",
  "vault.unnamed": "Kluis zonder naam",
  "vault.vaults": "Kluizen",
  "vault.allItems": "Alle items",
  "vault.items": "Items",
  "vault.search": "Zoeken in je kluis",
  "vault.untitled": "Zonder titel",
  "vault.noResults":
    "Niets komt overeen met “{query}”. Controleer de spelling, of zoek in plaats daarvan op een website.",
  "vault.new": "Nieuw item",
  "vault.selectItem": "Selecteer een item om de details te zien.",
  "vault.trashEmpty":
    "De prullenmand is leeg. Verwijderde items blijven hier 30 dagen staan.",
  "vault.favorite": "Favoriet",
  "vault.unfavorite": "Uit favorieten verwijderen",
  "vault.moveTo": "Naar kluis verplaatsen",
  "vault.allVaults": "Alle kluizen",
  "vault.category.favorites": "Favorieten",
  "vault.category.logins": "Logins",
  "vault.category.passkeys": "Passkeys",
  "vault.category.codes": "Eenmalige codes",
  "vault.category.cards": "Kaarten",
  "vault.category.notes": "Beveiligde notities",
  "vault.category.ssh": "SSH-sleutels",
  "vault.flag.otp": "2FA",
  "vault.moveToNamed": "Naar {name} verplaatsen",
  "vault.emptyTitle": "Je kluis is leeg",
  "vault.emptyBody":
    "Voeg een login, een passkey, een kaart of een notitie toe. Alles wordt op dit apparaat versleuteld voordat het wordt gesynchroniseerd.",
  "vault.addFirst": "Je eerste item toevoegen",
  "vault.paletteTip.before": "Druk op",
  "vault.paletteTip.after": "om op elk moment te zoeken of een opdracht uit te voeren.",
  "trash.retention": "Verwijderde items worden 30 dagen bewaard.",
  "trash.rowMeta":
    "Verwijderd op {deletedAt, date, medium} · nog {days, plural, one {# dag} other {# dagen}}",
  "trash.detailMeta": "{kind} · {vault} · Verwijderd op {deletedAt, date, medium}",
  "trash.purgeConfirm.title": "\"{title}\" definitief verwijderen?",
  "trash.purgeConfirm.body": "Je kunt dit niet ongedaan maken.",
  "vault.hardware.placeholderName": "Codes (beveiligingssleutel)",
  "vault.hardware.title": "Deze kluis heeft je beveiligingssleutel nodig",
  "vault.hardware.body":
    "Codes hier blijven veilig, ook als iemand je hoofdwachtwoord en Geheime Sleutel te weten komt, maar je kunt ze niet zien zonder je beveiligingssleutel.",
  "vault.hardware.open": "Beveiligingssleutel gebruiken",
  "vault.hardware.lockedNote": "Vergrendeld. Je andere kluizen blijven ontgrendeld.",
  "vault.hardware.lockNow": "Deze kluis vergrendelen",
  "vault.hardware.closesIn": "Vergrendelt over {time}",
  "vault.hardware.backupTitle": "Een reservesleutel registreren",
  "vault.hardware.backupBody":
    "Je hebt 1 sleutel geregistreerd. We raden er twee aan: als je je enige sleutel kwijtraakt, kun je deze kluis niet meer openen.",
  "vault.hardware.createOption": "Beschermen met een beveiligingssleutel (alleen codes)",
  "vault.hardware.tradeoff":
    "Voor codes in deze kluis is elke keer je beveiligingssleutel nodig, dus een gestolen hoofdwachtwoord alleen is niet genoeg, maar als je elke geregistreerde sleutel kwijtraakt, raak je deze codes kwijt.",
  "vault.hardware.removeNote":
    "Het verwijderen van een sleutel verandert de sleutel van deze kluis niet. Als een sleutel is gestolen, verplaats de codes dan naar een nieuwe kluis.",
  "vault.hardware.moveNote":
    "Het verplaatsen van een code die je al gebruikt naar deze kluis beschermt eerdere kopieën niet: het oude item, de geschiedenis ervan, back-ups en exports. Vraag als je kunt een nieuw geheim aan bij de aanbieder en voeg dat in plaats daarvan toe.",
  "vault.hardware.pinBody":
    "Voer de pincode van je sleutel in als hij die heeft en raak de sleutel dan aan wanneer hij knippert.",
  "vault.hardware.keys": "Sleutels",
  "vault.hardware.keysLabel": "Sleutels voor deze kluis",
  "vault.hardware.eyebrow": "Met hardware beveiligde codes",
  "vault.hardware.selfLocked":
    "Deze kluis heeft zichzelf vergrendeld. Gebruik je beveiligingssleutel om hem weer te openen.",
  "vault.hardware.closingSoon":
    "Deze kluis vergrendelt over minder dan 30 seconden. Gebruik daarna je beveiligingssleutel om hem weer te openen.",
  "vault.hardware.keysTitle": "Sleutels voor deze kluis",
  "vault.hardware.codesOnly": "Alleen eenmalige codes",
  "vault.create": "Nieuwe kluis",
  "vault.create.name": "Kluisnaam",
  "vault.create.submit": "Kluis aanmaken",
  "vault.menu": "Kluisacties",
  "vaultKey.menu": "Kluissleutel vernieuwen…",
  "vaultKey.title": "De sleutel van {vault} vernieuwen",
  "vaultKey.body":
    "Mosaic maakt een nieuwe sleutel voor deze kluis en geeft hem door aan elk apparaat en elke persoon met toegang. Kopieën die ze al hebben, blijven voor hen leesbaar; de nieuwe sleutel beschermt wat vanaf nu verandert. Gebruik dit nadat iemand toegang heeft verloren of een apparaat kwijt is geraakt.",
  "vaultKey.attachments": "Ook de sleutels van bijlagen vernieuwen",
  "vaultKey.attachments.hint":
    "Geeft ook elk bestand in deze kluis een nieuwe sleutel. Dit duurt langer bij veel of grote bestanden.",
  "vaultKey.reauth": "Voor het vernieuwen van een kluissleutel is je hoofdwachtwoord nodig.",
  "vaultKey.submit": "Sleutel vernieuwen",
  "vaultKey.working": "De sleutel wordt vernieuwd…",
  "vaultKey.progress": "Bijlagesleutels vernieuwen: {done} van {total}.",
  "vaultKey.done": "De kluissleutel is vernieuwd.",
  "vaultKey.doneAttachments":
    "De kluissleutel is vernieuwd en de bijlagesleutels zijn ook vernieuwd.",
  "vaultKey.unfinished":
    "De kluissleutel is vernieuwd. Het vernieuwen van de bijlagesleutels gaat later verder. Als Mosaic eerst sluit, kies dan de volgende keer Sleutelrotatie voortzetten.",
  "vaultKey.unreachable":
    "{count, plural, one {# bijlage staat nog niet op dit apparaat en houdt zijn oude sleutel.} other {# bijlagen staan nog niet op dit apparaat en houden hun oude sleutel.}}",
  "vaultKey.close": "Sluiten",
  "vault.meta.title": "Kluisgegevens zijn ingekort",
  "vault.meta.body":
    "Twee apparaten hebben tegelijk de {fields} van “{vault}” gewijzigd en samen waren ze te lang om op te slaan. Mosaic heeft één versie behouden of het veld leeggelaten. Controleer het en typ het zo nodig opnieuw.",
  "vault.edit.title": "Kluis bewerken",
  "vault.meta.edit": "Kluis bewerken",
  "vault.meta.field.name": "naam",
  "vault.meta.field.icon": "pictogram",
  "vault.meta.field.description": "beschrijving",
  "vault.meta.dismiss": "Sluiten",
} satisfies Record<keyof typeof en, string>;
