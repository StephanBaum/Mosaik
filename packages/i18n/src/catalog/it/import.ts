/**
 * Italian UI strings: import. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { importMessages as en } from "../en/import.ts";

export const importMessages = {
  "import.format.enpass": "Enpass (.json)",
  "import.format.bitwarden": "Bitwarden (.json, non cifrato)",
  "import.format.keepass": "KeePass (.kdbx)",
  "import.format.onePassword": "1Password (.1pux)",
  "import.format.cxf": "FIDO Credential Exchange (.json): passkey e password",
  "import.format.dashlane": "Dashlane (.zip)",
  "import.format.keeper": "Keeper (.json)",
  "import.format.keeperCsv": "Keeper (.csv)",
  "import.format.protonPass": "Proton Pass (.zip o .json, non cifrato)",
  "import.format.csv": "File CSV (.csv)",
  "import.keepassPassword": "Password del database KeePass",
  "import.preview.ready":
    "{count, plural, one {# elemento è pronto da importare.} other {# elementi sono pronti da importare.}}",
  "import.preview.attachments": "{count, plural, one {# allegato} other {# allegati}}",
  "import.preview.duplicate":
    "Hai già un elemento simile. Non è selezionato, così non verrà importato due volte.",
  "import.preview.duplicateInFile":
    "Questo file contiene due volte lo stesso elemento. La copia non è selezionata.",
  "import.warning.unmappedFields":
    "Alcuni campi non hanno un campo corrispondente in Mosaic. Sono stati conservati come campi personalizzati.",
  "import.warning.attachmentsSkipped":
    "Alcuni allegati superano i 50 MiB, non rientrano nel tuo spazio di 1 GiB per gli allegati oppure mancano nell'esportazione. Aggiungili a mano dopo l'importazione.",
  "import.warning.entriesSkipped":
    "Alcune voci non sono state lette e verranno escluse. Controllale nel tuo vecchio gestore di password.",
  "import.warning.otpUnreadable":
    "Alcune impostazioni dei codici monouso non sono state riconosciute. Sono state conservate come campi nascosti: configura di nuovo il codice a partire da quelli.",
  "import.warning.otpWeak":
    "Alcuni codici monouso hanno un segreto più corto di 80 bit, quindi i codici sono più facili da indovinare. Sono stati importati comunque; configurali di nuovo con un nuovo segreto appena puoi.",
  "import.warning.passkeysNotImported":
    "Questo formato di esportazione non contiene le chiavi private delle passkey, quindi non possono essere importate. Il resto di questi elementi viene importato.",
  "import.warning.passkeyUnreadable":
    "Alcune passkey non sono state lette e sono state escluse.",
  "import.warning.passkeyUnsupported":
    "Alcune passkey usano un tipo di chiave che Mosaic non supporta (funziona solo ES256) e sono state escluse.",
  "import.warning.passkeysSkipped":
    "Alcune passkey sono state escluse perché sono troppe (al massimo 100 per elemento e 10.000 per file).",
  "import.warning.passkeysKeepOldApp":
    "Conserva la tua vecchia app finché non hai spostato le passkey.",
  "import.entry.unmappedFields":
    "Alcuni campi non hanno un campo corrispondente in Mosaic. Sono stati conservati come campi personalizzati.",
  "import.entry.otpUnreadable":
    "L'impostazione del codice monouso non è stata riconosciuta. È stata conservata come campo nascosto: configura di nuovo il codice a partire da essa.",
  "import.entry.otpWeak":
    "Il segreto del codice monouso è più corto di 80 bit, quindi il codice è più facile da indovinare. È stato importato comunque; configuralo di nuovo con un nuovo segreto appena puoi.",
  "import.entry.attachmentTooLarge":
    "Un allegato supera i 50 MiB ed è stato escluso. Aggiungilo a mano dopo l'importazione.",
  "import.entry.attachmentMissing":
    "Un allegato manca nell'esportazione. Aggiungilo a mano dopo l'importazione.",
  "import.entry.attachmentQuota":
    "Un allegato non rientra nel tuo spazio di 1 GiB per gli allegati ed è stato escluso.",
  "import.entry.passkeysNotImported":
    "Questo formato di esportazione non contiene la chiave privata della passkey di questo elemento, quindi non può essere importata. Conserva la tua vecchia app finché non l'hai spostata.",
  "import.entry.passkeyUnreadable":
    "Una passkey di questo elemento non è stata letta ed è stata esclusa. Conserva la tua vecchia app finché non l'hai spostata.",
  "import.entry.passkeyUnsupported":
    "Una passkey di questo elemento usa un tipo di chiave che Mosaic non supporta (funziona solo ES256) ed è stata esclusa. Conserva la tua vecchia app finché non l'hai spostata.",
  "import.entry.historyTruncated": "Sono state conservate solo le 32 vecchie password più recenti.",
  "import.entry.tagsDropped": "I tag vuoti o troppo lunghi sono stati esclusi.",
  "import.entry.archived": "Archiviato nella tua vecchia app. Riceve il tag \"archived\".",
  "import.entry.unknownCategory":
    "Tipo di elemento sconosciuto. Viene importato come nota sicura.",
  "import.entry.titleCleaned":
    "Il titolo conteneva caratteri invisibili o che invertono il testo. Sono stati rimossi.",
  "import.entry.passkeysLimit":
    "Questo elemento ha più passkey di quante ne accetti una sola importazione (100 per elemento, 10.000 per file). Le passkey in più sono state escluse. Conserva la tua vecchia app finché non le hai spostate.",
  "import.entry.fieldsDropped":
    "Questo elemento ha più campi di quelli che Mosaic conserva. I campi in più sono esclusi.",
  "import.entry.notesTruncated":
    "Le note di questo elemento sono più lunghe di quanto un elemento possa contenere. La parte che non entrava è stata esclusa.",
  "import.entry.idnHost":
    "Questo indirizzo usa caratteri speciali nel nome di dominio e può sembrare un altro sito. Controllalo prima di fidarti.",
  "import.entry.urlKeptAsText":
    "Questo indirizzo web non ha potuto essere usato come link. È stato conservato come testo.",
  "import.skip.malformed": "Questa voce è danneggiata e non può essere letta.",
  "import.skip.empty": "Questa voce è vuota.",
  "import.skip.tooLarge":
    "Questa voce supera 1 MiB, il massimo che un elemento può contenere.",
  "import.result":
    "{imported, plural, one {# elemento importato.} other {# elementi importati.}} {skipped, plural, =0 {} one {# escluso.} other {# esclusi.}}",
  "import.flow.title": "Importa",
  "import.flow.formatStep": "Da dove stai importando?",
  "import.flow.chooseFile": "Scegli il file di esportazione",
  "import.flow.fileChosen": "File: {name}",
  "import.flow.password": "Password del database KeePass",
  "import.flow.preview": "Anteprima",
  "import.flow.previewBusy": "Lettura del file. Può richiedere un momento.",
  "import.flow.selectAll": "Seleziona tutto",
  "import.flow.target": "Importa in",
  "import.flow.commit": "Importa {count, plural, one {# elemento} other {# elementi}}",
  "import.flow.entries": "Voci",
  "import.flow.skipped": "Esclusi",
  "import.flow.attachments": "{count, plural, one {# allegato} other {# allegati}}",
  "import.flow.cancel": "Annulla importazione",
  "import.flow.noneSelected": "Seleziona almeno un elemento da importare.",
  "import.flow.resultFailed":
    "{failed, plural, =0 {} one {# elemento non ha potuto essere salvato.} other {# elementi non hanno potuto essere salvati.}}",
  "import.flow.deleteReminder":
    "Elimina subito il file di esportazione. Contiene le tue password in chiaro.",
  "import.flow.dropHint":
    "Scegli il tuo file di esportazione. Tutto viene letto e cifrato su questo dispositivo.",
} satisfies Record<keyof typeof en, string>;
