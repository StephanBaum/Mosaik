/** English UI strings: import. ICU MessageFormat. */
export const importMessages = {
  "import.format.enpass": "Enpass (.json)",
  "import.format.bitwarden": "Bitwarden (.json, unencrypted)",
  "import.format.keepass": "KeePass (.kdbx)",
  "import.format.onePassword": "1Password (.1pux)",
  "import.format.cxf": "FIDO Credential Exchange (.json): passkeys and passwords",
  "import.format.csv": "CSV file (.csv)",
  "import.keepassPassword": "Password of the KeePass database",
  "import.preview.ready":
    "{count, plural, one {# item is ready to import.} other {# items are ready to import.}}",
  "import.preview.attachments": "{count, plural, one {# attachment} other {# attachments}}",
  "import.preview.duplicate":
    "You already have an item like this. It's unticked, so it won't be imported twice.",
  "import.preview.duplicateInFile": "This file has the same item twice. The copy is unticked.",
  "import.warning.unmappedFields":
    "Some fields have no matching field in Mosaic. They were kept as custom fields.",
  "import.warning.attachmentsSkipped":
    "Some attachments are larger than 50 MiB, don't fit your 1 GiB of attachment storage, or are missing from the export. Add them by hand after the import.",
  "import.warning.entriesSkipped":
    "Some entries couldn't be read and will be left out. Check them in your old password manager.",
  "import.warning.otpUnreadable":
    "Some one-time code settings weren't recognized. They were kept as hidden fields, so set up the code again from them.",
  "import.warning.passkeysNotImported":
    "This export format doesn't contain the private keys of passkeys, so they can't be imported. The rest of those items is imported; keep your old app until you've moved the passkeys.",
  "import.warning.passkeyUnreadable":
    "Some passkeys couldn't be read and were left out. Keep your old app until you've moved them.",
  "import.warning.passkeyUnsupported":
    "Some passkeys use a key type Mosaic doesn't support (only ES256 works) and were left out. Keep your old app until you've moved them.",
  "import.entry.unmappedFields":
    "Some fields have no matching field in Mosaic. They were kept as custom fields.",
  "import.entry.otpUnreadable":
    "The one-time code setting wasn't recognized. It was kept as a hidden field, so set up the code again from it.",
  "import.entry.attachmentTooLarge":
    "An attachment is larger than 50 MiB and was left out. Add it by hand after the import.",
  "import.entry.attachmentMissing":
    "An attachment is missing from the export. Add it by hand after the import.",
  "import.entry.attachmentQuota":
    "An attachment doesn't fit your 1 GiB of attachment storage and was left out.",
  "import.entry.passkeysNotImported":
    "This export format doesn't contain the private key of this item's passkey, so it can't be imported. Keep your old app until you've moved it.",
  "import.entry.passkeyUnreadable":
    "A passkey of this item couldn't be read and was left out. Keep your old app until you've moved it.",
  "import.entry.passkeyUnsupported":
    "A passkey of this item uses a key type Mosaic doesn't support (only ES256 works) and was left out. Keep your old app until you've moved it.",
  "import.entry.historyTruncated": "Only the 32 most recent old passwords were kept.",
  "import.entry.tagsDropped": "Empty or overlong tags were left out.",
  "import.entry.archived": "Archived in your old app. It gets the tag \"archived\".",
  "import.entry.unknownCategory": "Unknown item type. It's imported as a secure note.",
  "import.entry.titleCleaned":
    "The title had invisible or text-reversing characters. They were removed.",
  "import.entry.fieldsDropped":
    "This item has more fields than Mosaic keeps. The extra fields are left out.",
  "import.skip.malformed": "This entry is damaged and can't be read.",
  "import.skip.empty": "This entry is empty.",
  "import.skip.tooLarge": "This entry is larger than 1 MiB, the most one item can hold.",
  "import.result":
    "{imported, plural, one {# item imported.} other {# items imported.}} {skipped, plural, =0 {} one {# left out.} other {# left out.}}",
  "import.flow.title": "Import",
  "import.flow.formatStep": "Where are you importing from?",
  "import.flow.chooseFile": "Choose the export file",
  "import.flow.fileChosen": "File: {name}",
  "import.flow.password": "Password of the KeePass database",
  "import.flow.preview": "Preview",
  "import.flow.previewBusy": "Reading the file. This can take a moment.",
  "import.flow.selectAll": "Select all",
  "import.flow.target": "Import into",
  "import.flow.commit": "Import {count, plural, one {# item} other {# items}}",
  "import.flow.entries": "Entries",
  "import.flow.skipped": "Left out",
  "import.flow.attachments": "{count, plural, one {# attachment} other {# attachments}}",
  "import.flow.cancel": "Cancel import",
  "import.flow.noneSelected": "Select at least one item to import.",
  "import.flow.resultFailed":
    "{failed, plural, =0 {} one {# item could not be saved.} other {# items could not be saved.}}",
  "import.flow.deleteReminder":
    "Delete the export file now. It holds your passwords in plain text.",
  "import.flow.dropHint": "Choose your export file. Everything is read and encrypted on this device.",
} as const satisfies Record<string, string>;
