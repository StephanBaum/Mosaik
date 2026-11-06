/** German UI strings: import. Informal "du". Same keys as English (checked by the type). */
import type { importMessages as en } from "../en/import.ts";

export const importMessages = {
  "import.format.enpass": "Enpass (.json)",
  "import.format.bitwarden": "Bitwarden (.json, unverschlüsselt)",
  "import.format.keepass": "KeePass (.kdbx)",
  "import.format.onePassword": "1Password (.1pux)",
  "import.format.cxf": "FIDO Credential Exchange (.json): Passkeys und Passwörter",
  "import.format.csv": "CSV-Datei (.csv)",
  "import.keepassPassword": "Passwort der KeePass-Datenbank",
  "import.preview.ready":
    "{count, plural, one {# Eintrag ist bereit für den Import.} other {# Einträge sind bereit für den Import.}}",
  "import.preview.attachments": "{count, plural, one {# Anhang} other {# Anhänge}}",
  "import.preview.duplicate":
    "Du hast schon einen Eintrag wie diesen. Er ist abgewählt, damit er nicht doppelt importiert wird.",
  "import.preview.duplicateInFile":
    "Diese Datei enthält denselben Eintrag zweimal. Die Kopie ist abgewählt.",
  "import.warning.unmappedFields":
    "Manche Felder haben kein passendes Feld in Mosaic. Sie wurden als eigene Felder übernommen.",
  "import.warning.attachmentsSkipped":
    "Manche Anhänge sind größer als 50 MiB, passen nicht mehr in deinen Anhang-Speicher von 1 GiB oder fehlen im Export. Füge sie nach dem Import von Hand hinzu.",
  "import.warning.entriesSkipped":
    "Manche Einträge konnten nicht gelesen werden und werden ausgelassen. Prüfe sie in deinem alten Passwortmanager.",
  "import.warning.otpUnreadable":
    "Manche Einstellungen für Einmalcodes wurden nicht erkannt. Sie wurden als verborgene Felder übernommen; richte den Code damit neu ein.",
  "import.warning.passkeysNotImported":
    "Dieses Exportformat enthält die privaten Schlüssel der Passkeys nicht, deshalb lassen sie sich nicht importieren. Der Rest dieser Einträge wird importiert; behalte deine alte App, bis du die Passkeys umgezogen hast.",
  "import.warning.passkeyUnreadable":
    "Manche Passkeys konnten nicht gelesen werden und wurden ausgelassen. Behalte deine alte App, bis du sie umgezogen hast.",
  "import.warning.passkeyUnsupported":
    "Manche Passkeys nutzen einen Schlüsseltyp, den Mosaic nicht unterstützt (nur ES256 funktioniert), und wurden ausgelassen. Behalte deine alte App, bis du sie umgezogen hast.",
  "import.warning.passkeysSkipped":
    "Manche Passkeys wurden ausgelassen, weil es zu viele sind (höchstens 100 pro Eintrag und 10.000 pro Datei). Behalte deine alte App, bis du sie umgezogen hast.",
  "import.entry.unmappedFields":
    "Manche Felder haben kein passendes Feld in Mosaic. Sie wurden als eigene Felder übernommen.",
  "import.entry.otpUnreadable":
    "Die Einstellung für den Einmalcode wurde nicht erkannt. Sie wurde als verborgenes Feld übernommen; richte den Code damit neu ein.",
  "import.entry.attachmentTooLarge":
    "Ein Anhang ist größer als 50 MiB und wurde ausgelassen. Füge ihn nach dem Import von Hand hinzu.",
  "import.entry.attachmentMissing":
    "Ein Anhang fehlt im Export. Füge ihn nach dem Import von Hand hinzu.",
  "import.entry.attachmentQuota":
    "Ein Anhang passt nicht mehr in deinen Anhang-Speicher von 1 GiB und wurde ausgelassen.",
  "import.entry.passkeysNotImported":
    "Dieses Exportformat enthält den privaten Schlüssel des Passkeys dieses Eintrags nicht, deshalb lässt er sich nicht importieren. Behalte deine alte App, bis du ihn umgezogen hast.",
  "import.entry.passkeyUnreadable":
    "Ein Passkey dieses Eintrags konnte nicht gelesen werden und wurde ausgelassen. Behalte deine alte App, bis du ihn umgezogen hast.",
  "import.entry.passkeyUnsupported":
    "Ein Passkey dieses Eintrags nutzt einen Schlüsseltyp, den Mosaic nicht unterstützt (nur ES256 funktioniert), und wurde ausgelassen. Behalte deine alte App, bis du ihn umgezogen hast.",
  "import.entry.historyTruncated": "Nur die 32 neuesten alten Passwörter wurden übernommen.",
  "import.entry.tagsDropped": "Leere oder zu lange Tags wurden ausgelassen.",
  "import.entry.titleCleaned":
    "Der Titel enthielt unsichtbare oder textumkehrende Zeichen. Sie wurden entfernt.",
  "import.entry.archived":
    "In deiner alten App archiviert. Der Eintrag bekommt den Tag „archived“.",
  "import.entry.unknownCategory": "Unbekannter Eintragstyp. Er wird als sichere Notiz importiert.",
  "import.entry.passkeysLimit":
    "Dieser Eintrag hat mehr Passkeys, als ein Import übernimmt (100 pro Eintrag, 10.000 pro Datei). Die überzähligen Passkeys wurden ausgelassen. Behalte deine alte App, bis du sie umgezogen hast.",
  "import.entry.fieldsDropped":
    "Dieser Eintrag hat mehr Felder, als Mosaic speichert. Die übrigen Felder werden ausgelassen.",
  "import.skip.malformed": "Dieser Eintrag ist beschädigt und lässt sich nicht lesen.",
  "import.skip.empty": "Dieser Eintrag ist leer.",
  "import.skip.tooLarge": "Dieser Eintrag ist größer als 1 MiB, das Maximum für einen Eintrag.",
  "import.result":
    "{imported, plural, one {# Eintrag importiert.} other {# Einträge importiert.}} {skipped, plural, =0 {} one {# ausgelassen.} other {# ausgelassen.}}",
  "import.flow.title": "Import",
  "import.flow.formatStep": "Woher importierst du?",
  "import.flow.chooseFile": "Exportdatei wählen",
  "import.flow.fileChosen": "Datei: {name}",
  "import.flow.password": "Passwort der KeePass-Datenbank",
  "import.flow.preview": "Vorschau",
  "import.flow.previewBusy": "Die Datei wird gelesen. Das kann einen Moment dauern.",
  "import.flow.selectAll": "Alle auswählen",
  "import.flow.target": "Importieren in",
  "import.flow.commit": "{count, plural, one {# Eintrag} other {# Einträge}} importieren",
  "import.flow.entries": "Einträge",
  "import.flow.skipped": "Ausgelassen",
  "import.flow.attachments": "{count, plural, one {# Anhang} other {# Anhänge}}",
  "import.flow.cancel": "Import abbrechen",
  "import.flow.noneSelected": "Wähle mindestens einen Eintrag zum Importieren.",
  "import.flow.resultFailed":
    "{failed, plural, =0 {} one {# Eintrag konnte nicht gespeichert werden.} other {# Einträge konnten nicht gespeichert werden.}}",
  "import.flow.deleteReminder":
    "Lösche die Exportdatei jetzt. Sie enthält deine Passwörter im Klartext.",
  "import.flow.dropHint": "Wähle deine Exportdatei. Alles wird auf diesem Gerät gelesen und verschlüsselt.",
} satisfies Record<keyof typeof en, string>;
