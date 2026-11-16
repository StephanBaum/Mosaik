/**
 * Dutch UI strings: import. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { importMessages as en } from "../en/import.ts";

export const importMessages = {
  "import.format.enpass": "Enpass (.json)",
  "import.format.bitwarden": "Bitwarden (.json, niet versleuteld)",
  "import.format.keepass": "KeePass (.kdbx)",
  "import.format.onePassword": "1Password (.1pux)",
  "import.format.cxf": "FIDO Credential Exchange (.json): passkeys en wachtwoorden",
  "import.format.dashlane": "Dashlane (.zip)",
  "import.format.keeper": "Keeper (.json)",
  "import.format.keeperCsv": "Keeper (.csv)",
  "import.format.protonPass": "Proton Pass (.zip of .json, niet versleuteld)",
  "import.format.csv": "CSV-bestand (.csv)",
  "import.keepassPassword": "Wachtwoord van de KeePass-database",
  "import.preview.ready":
    "{count, plural, one {# item is klaar om te importeren.} other {# items zijn klaar om te importeren.}}",
  "import.preview.attachments": "{count, plural, one {# bijlage} other {# bijlagen}}",
  "import.preview.duplicate":
    "Je hebt al een soortgelijk item. Het is uitgevinkt, zodat het niet dubbel wordt geïmporteerd.",
  "import.preview.duplicateInFile":
    "Dit bestand bevat hetzelfde item twee keer. De kopie is uitgevinkt.",
  "import.warning.unmappedFields":
    "Sommige velden hebben geen overeenkomstig veld in Mosaic. Ze zijn als eigen velden bewaard.",
  "import.warning.attachmentsSkipped":
    "Sommige bijlagen zijn groter dan 50 MiB, passen niet in je 1 GiB aan bijlageopslag of ontbreken in de export. Voeg ze na de import handmatig toe.",
  "import.warning.entriesSkipped":
    "Sommige vermeldingen konden niet worden gelezen en worden weggelaten. Controleer ze in je oude wachtwoordbeheerder.",
  "import.warning.otpUnreadable":
    "Sommige instellingen voor eenmalige codes zijn niet herkend. Ze zijn als verborgen velden bewaard, dus stel de code er opnieuw mee in.",
  "import.warning.otpWeak":
    "Sommige eenmalige codes hebben een geheim van minder dan 80 bits, waardoor de codes makkelijker te raden zijn. Ze zijn toch geïmporteerd; stel ze opnieuw in met een nieuw geheim wanneer je kunt.",
  "import.warning.passkeysNotImported":
    "Dit exportformaat bevat de persoonlijke sleutels van passkeys niet, dus ze kunnen niet worden geïmporteerd. De rest van die items wordt wel geïmporteerd.",
  "import.warning.passkeyUnreadable":
    "Sommige passkeys konden niet worden gelezen en zijn weggelaten.",
  "import.warning.passkeyUnsupported":
    "Sommige passkeys gebruiken een sleuteltype dat Mosaic niet ondersteunt (alleen ES256 werkt) en zijn weggelaten.",
  "import.warning.passkeysSkipped":
    "Sommige passkeys zijn weggelaten omdat het er te veel zijn (maximaal 100 per item en 10.000 per bestand).",
  "import.warning.passkeysKeepOldApp": "Houd je oude app totdat je de passkeys hebt overgezet.",
  "import.entry.unmappedFields":
    "Sommige velden hebben geen overeenkomstig veld in Mosaic. Ze zijn als eigen velden bewaard.",
  "import.entry.otpUnreadable":
    "De instelling voor de eenmalige code is niet herkend. Ze is als verborgen veld bewaard, dus stel de code er opnieuw mee in.",
  "import.entry.otpWeak":
    "Het geheim van de eenmalige code is korter dan 80 bits, waardoor de code makkelijker te raden is. Hij is toch geïmporteerd; stel hem opnieuw in met een nieuw geheim wanneer je kunt.",
  "import.entry.attachmentTooLarge":
    "Een bijlage is groter dan 50 MiB en is weggelaten. Voeg hem na de import handmatig toe.",
  "import.entry.attachmentMissing":
    "Een bijlage ontbreekt in de export. Voeg hem na de import handmatig toe.",
  "import.entry.attachmentQuota":
    "Een bijlage past niet in je 1 GiB aan bijlageopslag en is weggelaten.",
  "import.entry.passkeysNotImported":
    "Dit exportformaat bevat de persoonlijke sleutel van de passkey van dit item niet, dus hij kan niet worden geïmporteerd. Houd je oude app totdat je hem hebt overgezet.",
  "import.entry.passkeyUnreadable":
    "Een passkey van dit item kon niet worden gelezen en is weggelaten. Houd je oude app totdat je hem hebt overgezet.",
  "import.entry.passkeyUnsupported":
    "Een passkey van dit item gebruikt een sleuteltype dat Mosaic niet ondersteunt (alleen ES256 werkt) en is weggelaten. Houd je oude app totdat je hem hebt overgezet.",
  "import.entry.historyTruncated": "Alleen de 32 meest recente oude wachtwoorden zijn bewaard.",
  "import.entry.tagsDropped": "Lege of te lange labels zijn weggelaten.",
  "import.entry.archived": "Gearchiveerd in je oude app. Het krijgt het label \"archived\".",
  "import.entry.unknownCategory": "Onbekend itemtype. Het wordt als beveiligde notitie geïmporteerd.",
  "import.entry.titleCleaned":
    "De titel bevatte onzichtbare tekens of tekens die de tekstrichting omkeren. Ze zijn verwijderd.",
  "import.entry.passkeysLimit":
    "Dit item heeft meer passkeys dan één import aankan (100 per item, 10.000 per bestand). De extra passkeys zijn weggelaten. Houd je oude app totdat je ze hebt overgezet.",
  "import.entry.fieldsDropped":
    "Dit item heeft meer velden dan Mosaic bewaart. De extra velden zijn weggelaten.",
  "import.entry.notesTruncated":
    "De notities van dit item zijn langer dan één item kan bevatten. Het deel dat niet paste, is weggelaten.",
  "import.entry.idnHost":
    "Dit adres gebruikt speciale tekens in de domeinnaam en kan op een andere site lijken. Controleer het voordat je erop vertrouwt.",
  "import.entry.urlKeptAsText":
    "Dit webadres kon niet als link worden gebruikt. Het is als tekst bewaard.",
  "import.skip.malformed": "Deze vermelding is beschadigd en kan niet worden gelezen.",
  "import.skip.empty": "Deze vermelding is leeg.",
  "import.skip.tooLarge":
    "Deze vermelding is groter dan 1 MiB, het maximum dat één item kan bevatten.",
  "import.result":
    "{imported, plural, one {# item geïmporteerd.} other {# items geïmporteerd.}} {skipped, plural, =0 {} one {# weggelaten.} other {# weggelaten.}}",
  "import.flow.title": "Importeren",
  "import.flow.formatStep": "Waaruit importeer je?",
  "import.flow.chooseFile": "Kies het exportbestand",
  "import.flow.fileChosen": "Bestand: {name}",
  "import.flow.password": "Wachtwoord van de KeePass-database",
  "import.flow.preview": "Voorvertoning",
  "import.flow.previewBusy": "Het bestand wordt gelezen. Dit kan even duren.",
  "import.flow.selectAll": "Alles selecteren",
  "import.flow.target": "Importeren in",
  "import.flow.commit": "{count, plural, one {# item} other {# items}} importeren",
  "import.flow.entries": "Vermeldingen",
  "import.flow.skipped": "Weggelaten",
  "import.flow.attachments": "{count, plural, one {# bijlage} other {# bijlagen}}",
  "import.flow.cancel": "Import annuleren",
  "import.flow.noneSelected": "Selecteer minstens één item om te importeren.",
  "import.flow.resultFailed":
    "{failed, plural, =0 {} one {# item kon niet worden bewaard.} other {# items konden niet worden bewaard.}}",
  "import.flow.deleteReminder":
    "Verwijder het exportbestand nu. Het bevat je wachtwoorden als leesbare tekst.",
  "import.flow.dropHint":
    "Kies je exportbestand. Alles wordt op dit apparaat gelezen en versleuteld.",
} satisfies Record<keyof typeof en, string>;
