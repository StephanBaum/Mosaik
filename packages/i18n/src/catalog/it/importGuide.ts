/**
 * Italian UI strings: importGuide. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { importGuide as en } from "../en/importGuide.ts";

export const importGuide = {
  "import.grid.title": "Dove sono ora le tue password?",
  "import.grid.browsers": "Browser",
  "import.grid.managers": "Gestori di password",
  "import.grid.other": "Altro",
  "import.source.chrome": "Chrome",
  "import.source.edge": "Edge",
  "import.source.brave": "Brave",
  "import.source.opera": "Opera",
  "import.source.vivaldi": "Vivaldi",
  "import.source.arc": "Arc",
  "import.source.firefox": "Firefox",
  "import.source.safari": "Safari",
  "import.source.applePasswords": "Apple Passwords",
  "import.source.onePassword": "1Password",
  "import.source.bitwarden": "Bitwarden",
  "import.source.keepass": "KeePass",
  "import.source.keepassxcCsv": "KeePassXC (CSV)",
  "import.source.enpass": "Enpass",
  "import.source.lastpass": "LastPass",
  "import.source.dashlane": "Dashlane",
  "import.source.keeper": "Keeper",
  "import.source.nordpass": "NordPass",
  "import.source.protonPass": "Proton Pass",
  "import.source.roboform": "RoboForm",
  "import.source.otherFile": "Altro CSV o file",
  "import.guide.chrome.1":
    "Apri questo indirizzo in Chrome: copialo e incollalo nella barra degli indirizzi.",
  "import.guide.chrome.2":
    "Accanto a “Esporta password”, fai clic su “Scarica file” e conferma con la password del tuo computer.",
  "import.guide.chrome.3": "Chrome salva “Chrome Passwords.csv” in Download.",
  "import.guide.edge.1":
    "Apri questo indirizzo in Edge: copialo e incollalo nella barra degli indirizzi.",
  "import.guide.edge.2":
    "Fai clic su “⋯” accanto a “Password salvate”, poi su “Esporta password”, e conferma.",
  "import.guide.edge.3": "Edge salva il file CSV in Download.",
  "import.guide.brave.1":
    "Apri questo indirizzo in Brave: copialo e incollalo nella barra degli indirizzi.",
  "import.guide.brave.2":
    "Accanto a “Esporta password”, fai clic su “Scarica file” e conferma con la password del tuo computer.",
  "import.guide.brave.3": "Brave salva il file CSV in Download.",
  "import.guide.opera.1":
    "Apri questo indirizzo in Opera: copialo e incollalo nella barra degli indirizzi.",
  "import.guide.opera.2":
    "Accanto a “Esporta password”, fai clic su “Scarica file” e conferma con la password del tuo computer.",
  "import.guide.opera.3": "Opera salva il file CSV in Download.",
  "import.guide.vivaldi.1":
    "Apri questo indirizzo in Vivaldi: copialo e incollalo nella barra degli indirizzi.",
  "import.guide.vivaldi.2":
    "Accanto a “Esporta password”, fai clic su “Scarica file” e conferma con la password del tuo computer.",
  "import.guide.vivaldi.3": "Vivaldi salva il file CSV in Download.",
  "import.guide.arc.1":
    "Apri questo indirizzo in Arc: copialo e incollalo nella barra degli indirizzi.",
  "import.guide.arc.2":
    "Accanto a “Esporta password”, fai clic su “Scarica file” e conferma con la password del tuo computer.",
  "import.guide.arc.3": "Arc salva il file CSV in Download.",
  "import.guide.firefox.1":
    "Apri questo indirizzo in Firefox: copialo e incollalo nella barra degli indirizzi.",
  "import.guide.firefox.2":
    "Fai clic su “⋯” in alto a destra, poi su “Esporta password…”, e conferma.",
  "import.guide.firefox.3": "Salva “logins.csv” in Download.",
  "import.guide.safari.1":
    "Safari → Impostazioni → Password (macOS 14 e precedenti; da macOS 15 in poi usa Apple Passwords).",
  "import.guide.safari.2":
    "Fai clic su “⋯” sotto l'elenco, poi su “Esporta tutte le password…”.",
  "import.guide.safari.3": "Salva il file CSV in Download.",
  "import.guide.applePasswords.1": "Apri l'app Password.",
  "import.guide.applePasswords.2": "File → “Esporta tutte le password in un file…”.",
  "import.guide.applePasswords.3": "Salva il file CSV in Download.",
  "import.guide.onePassword.1": "Apri l'app 1Password e sbloccala.",
  "import.guide.onePassword.2":
    "File → Esporta → il tuo account, poi inserisci la tua password.",
  "import.guide.onePassword.3": "Scegli “1PUX” e fai clic su “Esporta dati”.",
  "import.guide.bitwarden.1":
    "Nella cassaforte web o nell'app: Strumenti → Esporta cassaforte.",
  "import.guide.bitwarden.2": "Scegli il formato di file “.json” (non “cifrato”).",
  "import.guide.bitwarden.3": "Conferma con la tua password principale.",
  "import.guide.keepass.1": "Usa il tuo file di database (.kdbx) così com'è.",
  "import.guide.keepass.2":
    "Inserirai la sua password nel passaggio successivo. Resta su questo dispositivo.",
  "import.guide.keepassxcCsv.1": "KeePassXC: Database → Esporta → File CSV…",
  "import.guide.keepassxcCsv.2":
    "Salvalo in Download. Il file .kdbx conserva di più, quindi se puoi preferisci “KeePass”.",
  "import.guide.enpass.1": "Enpass: Menu → File → Esporta.",
  "import.guide.enpass.2":
    "Scegli la cassaforte, scegli il formato “.json” e inserisci la tua password principale.",
  "import.guide.enpass.3": "Salvalo in Download.",
  "import.guide.lastpass.1":
    "Accedi su lastpass.com e apri Opzioni avanzate → Esporta.",
  "import.guide.lastpass.2": "Inserisci la tua password principale. Il file CSV viene scaricato.",
  "import.guide.lastpass.3":
    "Se il browser mostra invece del testo, salva la pagina come “lastpass.csv”.",
  "import.guide.dashlane.1":
    "Apri Dashlane nel browser → Il mio account → Impostazioni → Esporta dati.",
  "import.guide.dashlane.2": "Scegli “CSV”. Dashlane scarica un file ZIP.",
  "import.guide.dashlane.3": "Lascia il file ZIP così com'è.",
  "import.guide.keeper.1": "Keeper Web Vault → Impostazioni → Esporta.",
  "import.guide.keeper.2":
    "Scegli “JSON” (conserva i campi personalizzati e i codici 2FA) ed esporta.",
  "import.guide.nordpass.1": "NordPass → Impostazioni → Esporta elementi.",
  "import.guide.nordpass.2":
    "Conferma con la tua password principale e salva il file CSV in Download.",
  "import.guide.protonPass.1": "Proton Pass → Impostazioni → Esporta.",
  "import.guide.protonPass.2":
    "Scegli “ZIP” e lascia la cifratura disattivata. Mosaic non può leggere l'esportazione cifrata con PGP.",
  "import.guide.protonPass.3": "Sblocca e salvalo in Download.",
  "import.guide.roboform.1": "RoboForm → Impostazioni → Account e dati → Esporta.",
  "import.guide.roboform.2": "Scegli “CSV” e “Accessi”, poi salvalo in Download.",
  "import.guide.otherFile.1":
    "Esporta un file CSV con una riga di intestazione, oppure uno dei formati qui sopra.",
  "import.guide.otherFile.2": "Scegli il file. Mosaic associa le colonne.",
  "import.guide.copyUrl": "Copia indirizzo",
  "import.guide.urlCopied": "Indirizzo copiato.",
  "import.guide.choose": "Scegli il file di esportazione",
  "import.guide.dropHint": "…oppure trascina qui un file. Viene letto solo su questo dispositivo.",
  "import.guide.back": "Indietro",
  "import.detect.detected": "Riconosciuto: {source}",
  "import.detect.mismatch":
    "Questo file sembra un'esportazione di {source}. Mosaic lo leggerà come tale.",
  "import.detected.chromium": "Chrome, Edge o un altro browser Chromium",
  "import.detected.firefox": "Firefox",
  "import.detected.safari": "Safari o Apple Passwords",
  "import.detected.bitwardenCsv": "Bitwarden (CSV)",
  "import.detected.bitwarden": "Bitwarden",
  "import.detected.enpass": "Enpass",
  "import.detected.keepass": "KeePass",
  "import.detected.keepassxcCsv": "KeePassXC (CSV)",
  "import.detected.onePassword": "1Password",
  "import.detected.cxf": "un file FIDO Credential Exchange",
  "import.detected.lastpass": "LastPass",
  "import.detected.dashlane": "Dashlane",
  "import.detected.keeper": "Keeper",
  "import.detected.nordpass": "NordPass",
  "import.detected.protonPass": "Proton Pass",
  "import.detected.roboform": "RoboForm",
  "import.detected.csv": "un file CSV",
  "import.lost.title": "Non verrà trasferito",
  "import.lost.passkeys":
    "Passkey: {source} non le include nella sua esportazione. Conserva {source} finché non hai configurato nuove passkey.",
  "import.lost.attachments":
    "Allegati: {source} non li include nella sua esportazione. Aggiungili a mano.",
  "import.lost.history":
    "Vecchie password (cronologia): questa esportazione non le contiene.",
  "import.lost.roboformExtras":
    "Identità, note sicure e segnalibri: esportali separatamente. Non fanno parte del file degli accessi.",
  "import.lost.entriesSkipped":
    "{count, plural, one {# voce non è stata letta ed è esclusa.} other {# voci non sono state lette e sono escluse.}}",
  "import.lost.fieldsDropped":
    "Alcuni elementi hanno più campi di quelli che Mosaic conserva. I campi in più sono esclusi.",
  "import.lost.notesTruncated":
    "Alcune note sono più lunghe di quanto un elemento possa contenere. La parte che non entrava è esclusa.",
  "import.lost.attachmentsSkipped":
    "Alcuni allegati mancano, sono troppo grandi o non rientrano nel tuo spazio per gli allegati. Aggiungili a mano.",
  "import.cleanup.title": "Pulizia",
  "import.cleanup.delete": "Elimina {name} ora",
  "import.cleanup.deleted": "{name} è stato sovrascritto ed eliminato.",
  "import.cleanup.deleteFailed": "{name} non ha potuto essere eliminato. Eliminalo tu.",
  "import.cleanup.keepFile":
    "Alcuni elementi non sono stati importati, o non sono arrivati completi. Conserva {name} finché non hai controllato cosa manca, poi eliminalo tu.",
  "import.cleanup.ssdNote":
    "Su SSD e nelle cartelle sincronizzate possono sopravvivere copie più vecchie: nella cronologia delle versioni, nei backup o nel cestino. Controlla anche quelli.",
  "import.cleanup.synced":
    "Questo file si trova nella tua cartella {service}. {service} potrebbe conservare versioni precedenti; eliminale anche lì.",
  "import.synced.oneDrive": "OneDrive",
  "import.synced.dropbox": "Dropbox",
  "import.synced.googleDrive": "Google Drive",
  "import.synced.icloud": "iCloud",
  "import.synced.box": "Box",
  "import.checklist.title": "Prima di lasciare la tua vecchia app",
  "import.checklist.browserSaving":
    "Disattiva in {source} il salvataggio delle password e l'accesso automatico, così non propone di salvare insieme a Mosaic.",
  "import.checklist.firefoxSaving":
    "Apri about:preferences#privacy e deseleziona “Richiedi di salvare le password”, così Firefox non propone di salvare insieme a Mosaic.",
  "import.checklist.safariSaving":
    "Safari → Impostazioni → Compilazione automatica: deseleziona “Nomi utente e password”, così Safari non compila insieme a Mosaic.",
  "import.checklist.oldExtension":
    "Disattiva l'estensione del browser di {source}, così compila una sola app.",
  "import.checklist.deleteOld":
    "Elimina i tuoi dati in {source} dopo aver controllato tutto in Mosaic.",
} satisfies Record<keyof typeof en, string>;
