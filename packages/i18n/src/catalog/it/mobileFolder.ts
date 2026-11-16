/**
 * Italian UI strings: mobileFolder. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { mobileFolder as en } from "../en/mobileFolder.ts";

export const mobileFolder = {
  "mobile.sync.never": "Non ancora sincronizzato",
  "mobile.sync.last": "Sincronizzato {time}",
  "mobile.sync.localOnly":
    "Questa cassaforte è solo su questo telefono. La sincronizzazione non è configurata.",
  "mobile.sync.pending":
    "{count, plural, one {# modifica in attesa di caricamento} other {# modifiche in attesa di caricamento}}",
  "mobile.sync.now": "Sincronizza ora",
  "mobile.sync.working": "Sincronizzazione",
  "mobile.sync.warnings": "Avvisi di sincronizzazione",
  "mobile.sync.warning.freezeSuspected":
    "Le modifiche di un altro dispositivo non arrivano da un po'. Controlla che l'altro dispositivo sia acceso e online.",
  "mobile.sync.warning.noCommonView":
    "Questo telefono e un altro dispositivo non vedono gli stessi dati del server da due giorni. Il server potrebbe mostrare loro versioni diverse. Sincronizza l'altro dispositivo, poi sincronizza qui.",
  "mobile.sync.warning.rollbackDetected":
    "I tuoi dati di sincronizzazione sembrano più vecchi di quelli di questo telefono. Apri Mosaic su un altro dispositivo e controlla che le tue ultime modifiche ci siano. Poi scegli “Ho controllato”.",
  "mobile.sync.warning.identityDuplicated":
    "Un'altra copia di Mosaic sta scrivendo con l'identità di questo telefono, ad esempio dopo il ripristino di un backup. La sincronizzazione è in pausa. Assicurati che sia in uso una sola copia, poi scegli “Ho controllato”.",
  "mobile.sync.warning.serverInconsistent":
    "Il server ha mostrato a questo telefono dati diversi da quelli di un altro dispositivo. Apri Mosaic su un altro dispositivo e controlla che le tue ultime modifiche ci siano. Poi scegli “Ho controllato”.",
  "mobile.sync.warning.serverRollback":
    "La copia di una delle tue casseforti sul server è più vecchia di quella di questo telefono. Apri Mosaic su un altro dispositivo e controlla che le tue ultime modifiche ci siano. Poi scegli “Ho controllato”.",
  "mobile.sync.warning.sharingAttention":
    "Una cassaforte condivisa richiede attenzione: c'è un cambio di chiave in attesa, oppure un membro non ha potuto essere verificato. Apri Mosaic sul computer per vedere quale. Poi scegli “Ho controllato”.",
  "mobile.sync.warning.undecryptable":
    "{count, plural, one {# elemento ricevuto non ha potuto essere letto} other {# elementi ricevuti non hanno potuto essere letti}} su questo telefono. Aggiorna Mosaic su tutti i tuoi dispositivi, poi sincronizza di nuovo.",
  "mobile.sync.warning.accountRollback":
    "Il server mostra una chiave dell'account più vecchia di quella di questo telefono. Non è stato cambiato nulla. Apri Mosaic su un altro dispositivo e controlla che le tue ultime modifiche ci siano. Poi scegli “Ho controllato”.",
  "mobile.sync.acknowledge": "Ho controllato…",
  "mobile.sync.acknowledgeTitle": "Cancellare gli avvisi di sincronizzazione?",
  "mobile.sync.acknowledgeBody":
    "Procedi solo dopo aver controllato tu stesso il server. Questo cancella ogni avviso e la sincronizzazione prosegue. Se i tuoi dati sono stati davvero riportati a uno stato precedente o modificati, Mosaic non potrà più accorgersene. Fallo su ciascuno dei tuoi dispositivi.",
  "mobile.sync.acknowledgeConfirm": "Ho controllato",
  "mobile.folder.intro": "Questo account si sincronizza tramite la tua cartella {provider}.",
  "mobile.folder.signIn": "Accedi a {provider}",
  "mobile.folder.scope":
    "{provider} chiederà l'accesso a tutti i tuoi file. Mosaic apre solo la cartella che scegli e non tocca mai nient'altro.",
  "mobile.folder.waiting": "Completa l'accesso nel browser, poi torna qui.",
  "mobile.folder.serverLabel": "Indirizzo Nextcloud o WebDAV",
  "mobile.folder.serverHint": "https://cloud.example.com",
  "mobile.folder.serverConfirm":
    "Mosaic effettuerà l'accesso su {server}. Continua solo se è il tuo server.",
  "mobile.folder.locating": "Ricerca della tua cartella Mosaic…",
  "mobile.folder.browseTitle": "Scegli la tua cartella Mosaic",
  "mobile.folder.browseHint":
    "È la cartella che il tuo computer sincronizza, con un file chiamato mosaic-sync.json.",
  "mobile.folder.currentFolder": "Cartella attuale: {path}",
  "mobile.folder.noSubfolders": "Qui non ci sono cartelle.",
  "mobile.folder.useThis": "Usa questa cartella",
  "mobile.folder.up": "Livello superiore",
  "mobile.folder.chooseProvider": "Dove si sincronizza il tuo account?",
  "mobile.folder.noDrive":
    "Google Drive e iCloud Drive non funzionano ancora su Android. Usa Dropbox, OneDrive, Nextcloud o la sincronizzazione di Mosaic.",
  "mobile.folder.useFolder": "Sincronizza invece tramite una cartella cloud",
  "mobile.folder.providerDropbox": "Dropbox",
  "mobile.folder.providerOneDrive": "OneDrive",
  "mobile.folder.providerWebDav": "Nextcloud o WebDAV",
  "mobile.folder.appPasswordToggle": "Usa invece una password per app",
  "mobile.folder.davLabel": "Indirizzo della cartella WebDAV",
  "mobile.folder.loginLabel": "Nome utente",
  "mobile.folder.appPasswordLabel": "Password per app",
  "mobile.folder.appPasswordHint":
    "Crea una password per app nelle impostazioni di sicurezza del tuo server. Non usare la tua password principale.",
  "mobile.folder.appPasswordAction": "Accedi con la password per app",
  "mobile.folder.pickAccount": "A quale account si unisce questo telefono?",
  "mobile.folder.accountOption": "Account {id}",
  "mobile.folder.linked": "Accesso effettuato. Sincronizzazione ripresa.",
  "mobile.folder.settingsLine": "Si sincronizza tramite {provider}: {path}",
  "mobile.folder.relink": "Accedi di nuovo",
  "mobile.folder.msRevokeHint":
    "Per rimuovere del tutto l'accesso di Mosaic, rimuovilo nella pagina del tuo account Microsoft, sotto App.",
  "mobile.reset.revokeDropbox":
    "L'account è stato rimosso. Non siamo riusciti a terminare l'accesso di Mosaic su Dropbox: rimuovilo in Dropbox, in Impostazioni, App collegate.",
  "mobile.reset.revokeWebdav":
    "L'account è stato rimosso. Non siamo riusciti a eliminare la password per app sul tuo server: rimuovila lì, in Impostazioni, Sicurezza, Dispositivi e sessioni.",
  "mobile.reset.title": "Rimuovi questo account da questo telefono",
  "mobile.reset.body":
    "Questo elimina la copia della cassaforte, la Chiave segreta e l'accesso al cloud memorizzati su questo telefono. I tuoi dati nella cartella cloud o sul server restano. Potrai unirti di nuovo in seguito con la tua Chiave segreta e la password principale.",
  "mobile.reset.action": "Rimuovi account…",
  "mobile.reset.confirm": "Rimuovi account",
  "mobile.reset.phraseLabel": "Conferma",
  "mobile.reset.typePhrase": "Digita {phrase} per confermare",
} satisfies Record<keyof typeof en, string>;
