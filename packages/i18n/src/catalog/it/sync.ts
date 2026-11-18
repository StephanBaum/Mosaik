/**
 * Italian UI strings: sync. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { sync as en } from "../en/sync.ts";

export const sync = {
  "sync.status.signedOut": "Disconnesso",
  "sync.platform.windows": "Windows",
  "sync.platform.macos": "macOS",
  "sync.platform.linux": "Linux",
  "settings.sync.title": "Sincronizzazione",
  "settings.sync.body":
    "Mantieni aggiornate le tue casseforti su tutti i tuoi dispositivi. Tutto viene cifrato su questo dispositivo prima di uscire.",
  "settings.sync.notSetUp":
    "La sincronizzazione è disattivata. Le tue casseforti sono salvate solo su questo dispositivo.",
  "settings.sync.deviceName": "Nome di questo dispositivo",
  "settings.sync.deviceName.default": "Mosaic su {platform}",
  "settings.sync.password": "Password principale",
  "settings.sync.code": "Codice di 6 cifre",
  "settings.sync.server.title": "Server Mosaic",
  "settings.sync.server.body":
    "Sincronizza tramite un server Mosaic gestito da te o da qualcuno di cui ti fidi.",
  "settings.sync.server.url": "Indirizzo del server",
  "settings.sync.server.urlHint": "Ad esempio https://sync.example.com",
  "settings.sync.server.check": "Controlla il server",
  "settings.sync.server.checking": "Controllo del server…",
  "settings.sync.server.registration.open":
    "Questo server permette a chiunque di creare un account di sincronizzazione.",
  "settings.sync.server.registration.invite":
    "Questo server richiede un codice d'invito per creare un account di sincronizzazione.",
  "settings.sync.server.registration.closed":
    "Questo server non accetta nuovi account di sincronizzazione. Puoi comunque accedere a uno esistente.",
  "settings.sync.server.choice.label": "Account di sincronizzazione",
  "settings.sync.server.choice.signIn": "Ho un account di sincronizzazione",
  "settings.sync.server.choice.create": "Creane uno",
  "settings.sync.server.invite": "Codice d'invito",
  "settings.sync.server.create": "Crea account di sincronizzazione",
  "settings.sync.server.signIn": "Accedi",
  "settings.sync.server.signInAgain": "Accedi di nuovo",
  "settings.sync.server.working": "Connessione…",
  "settings.sync.server.sees":
    "Il server vede solo dati cifrati, l'email del tuo account e quando sincronizzi. Non vede mai la tua password principale né la tua Chiave segreta.",
  "settings.sync.twoFactor.title": "Inserisci il tuo codice",
  "settings.sync.twoFactor.body":
    "Questo account di sincronizzazione usa la verifica in due passaggi. Inserisci il codice di 6 cifre della tua app di autenticazione.",
  "settings.sync.twoFactor.submit": "Verifica",
  "settings.sync.folder.title": "La tua cartella cloud",
  "settings.sync.folder.body":
    "Sincronizza tramite una cartella che il tuo servizio cloud (Dropbox, OneDrive, iCloud Drive e altri) tiene aggiornata.",
  "settings.sync.folder.choose": "Scegli cartella…",
  "settings.sync.folder.chooseOther": "Scegli un'altra cartella…",
  "settings.sync.folder.path": "Cartella",
  "settings.sync.folder.contents.empty": "La cartella è vuota. Mosaic può usarla.",
  "settings.sync.folder.contents.mosaic":
    "Questa cartella contiene già dati di sincronizzazione di Mosaic.",
  "settings.sync.folder.contents.other":
    "Questa cartella contiene altri file. Mosaic non scrive nulla lì. Scegli una cartella vuota o creane una nuova.",
  "settings.sync.folder.connect": "Collega cartella",
  "settings.sync.folder.working": "Connessione…",
  "settings.sync.folder.sees":
    "Il tuo fornitore cloud e chiunque abbia accesso alla cartella vedono solo dati cifrati, l'ID dell'account e quando i file cambiano. Non vedono mai la tua password principale né la tua Chiave segreta.",
  "settings.sync.status.label": "Stato",
  "settings.sync.lastSynced": "Ultima sincronizzazione {when}",
  "settings.sync.neverSynced": "Non ancora sincronizzato",
  "settings.sync.offline":
    "Sei offline. Le tue modifiche sono salvate su questo dispositivo e verranno sincronizzate quando tornerai online.",
  "settings.sync.now": "Sincronizza ora",
  "settings.sync.backend.server": "Server",
  "settings.sync.backend.account": "Account di sincronizzazione",
  "settings.sync.backend.folder": "Cartella",
  "settings.sync.backend.device": "Questo dispositivo",
  "settings.sync.signedOut.title": "Questo dispositivo è disconnesso",
  "settings.sync.signedOut.body":
    "Il server ha terminato la sessione di questo dispositivo. Accedi di nuovo per continuare a sincronizzare. Le tue modifiche sono al sicuro su questo dispositivo.",
  "settings.sync.recover.title": "La tua password principale è stata cambiata",
  "settings.sync.recover.body":
    "Il server o la cartella contiene una password principale più recente di quella di questo dispositivo. Inserisci la nuova password principale per aggiornare questo dispositivo. Le tue casseforti e la tua Chiave segreta restano le stesse.",
  "settings.sync.recover.password": "Nuova password principale",
  "settings.sync.recover.submit": "Aggiorna questo dispositivo",
  "settings.sync.disconnect": "Scollega…",
  "settings.sync.disconnect.title": "Scollegare la sincronizzazione?",
  "settings.sync.disconnect.body":
    "Questo dispositivo smette di sincronizzare e si disconnette dal server. Le tue casseforti restano su questo dispositivo e gli altri tuoi dispositivi mantengono le proprie copie.",
  "settings.sync.disconnect.confirm": "Scollega",
  "settings.sync.devices.title": "Dispositivi",
  "settings.sync.devices.thisDevice": "Questo dispositivo",
  "settings.sync.devices.unnamed": "Dispositivo senza nome",
  "settings.sync.devices.lastSeen": "Visto l'ultima volta {when}",
  "settings.sync.devices.neverSeen": "Non ancora visto",
  "settings.sync.devices.rename": "Rinomina",
  "settings.sync.devices.renameNamed": "Rinomina {name}",
  "settings.sync.devices.renameLabel": "Nuovo nome per questo dispositivo",
  "settings.sync.devices.remove": "Rimuovi",
  "settings.sync.devices.removeNamed": "Rimuovi {name}",
  "settings.sync.devices.removeTitle": "Rimuovere {name}?",
  "settings.sync.devices.removeBody":
    "Rimuovere un dispositivo lo disconnette dalla sincronizzazione. Non elimina la copia delle tue casseforti che ha già. Per escludere un dispositivo in modo definitivo, cambia la password principale e la Chiave segreta.",
  "settings.sync.devices.removeConfirm": "Rimuovi dispositivo",
  "settings.sync.devices.folderNote":
    "Per escludere un dispositivo, cambia la password principale e la Chiave segreta.",
  "settings.sync.phone.title": "Configura un telefono",
  "settings.sync.phone.body":
    "Mostra un codice che il telefono scansiona, così non devi digitare la tua Chiave segreta. Il telefono ha comunque bisogno della tua password principale.",
  "settings.sync.phone.button": "Configura un telefono",
  "settings.sync.phone.reauth":
    "Il codice contiene la tua Chiave segreta, quindi serve la tua password principale.",
  "settings.sync.phone.dialogTitle": "Scansiona questo codice con il telefono",
  "settings.sync.phone.steps":
    "Sul telefono apri Mosaic e scegli di configurarlo, poi scansiona questo codice.",
  "settings.sync.phone.hint":
    "Il codice si nasconde dopo 60 secondi o quando lasci questa finestra. Non condividerne una foto.",
  "settings.sync.phone.canvas": "Codice di configurazione per il tuo telefono",
  "settings.sync.phone.hidden.time": "I 60 secondi sono finiti, quindi il codice è nascosto.",
  "settings.sync.phone.hidden.away": "Hai lasciato questa finestra, quindi il codice è nascosto.",
  "settings.sync.phone.again": "Mostra di nuovo",
  "settings.sync.phone.folderHint":
    "Il tuo telefono accede al tuo spazio cloud e apre questa stessa cartella.",
  "settings.sync.totp.title": "App di autenticazione",
  "settings.sync.totp.body":
    "Chiede un codice da un'app di autenticazione quando qualcuno accede al tuo account di sincronizzazione. Protegge l'account sul server, non le tue casseforti: queste restano cifrate con la tua password principale e la tua Chiave segreta.",
  "settings.sync.totp.on": "I codici di un'app di autenticazione sono attivi.",
  "settings.sync.totp.off": "I codici di un'app di autenticazione sono disattivati.",
  "settings.sync.totp.enable": "Attiva…",
  "settings.sync.totp.disable": "Disattiva…",
  "settings.sync.totp.beginTitle": "Attiva i codici dell'app di autenticazione",
  "settings.sync.totp.beginBody": "Inserisci la tua password principale per iniziare.",
  "settings.sync.totp.begin": "Continua",
  "settings.sync.totp.setupBody":
    "Aggiungi questa chiave alla tua app di autenticazione, poi inserisci il codice di 6 cifre che mostra.",
  "settings.sync.totp.secret": "Chiave di configurazione",
  "settings.sync.totp.confirm": "Attiva",
  "settings.sync.totp.disableTitle": "Disattiva i codici dell'app di autenticazione",
  "settings.sync.totp.disableBody": "Inserisci la tua password principale e un codice attuale.",
  "settings.sync.totp.disableConfirm": "Disattiva",
  "settings.sync.totp.enabledToast": "I codici di un'app di autenticazione sono attivi.",
  "settings.sync.totp.disabledToast": "I codici di un'app di autenticazione sono disattivati.",
  "settings.sync.key.title": "Chiavi di sicurezza",
  "settings.sync.key.body":
    "Una chiave di sicurezza può essere un secondo passaggio quando accedi al tuo account di sincronizzazione. Protegge l'account sul server, non le tue casseforti. Aggiungere o usare una chiave apre il browser.",
  "settings.sync.key.none": "Nessuna chiave di sicurezza aggiunta.",
  "settings.sync.key.added": "Aggiunta {when}",
  "settings.sync.key.lastUsed": "Ultimo uso {when}",
  "settings.sync.key.neverUsed": "Mai usata",
  "settings.sync.key.add": "Aggiungi una chiave di sicurezza…",
  "settings.sync.key.addTitle": "Aggiungi una chiave di sicurezza",
  "settings.sync.key.addBody":
    "Dai un nome alla chiave e inserisci la tua password principale. Poi Mosaic apre il browser, dove usi la chiave.",
  "settings.sync.key.label": "Nome della chiave",
  "settings.sync.key.labelHint": "Ad esempio “Chiave della scrivania”. Fino a 64 caratteri.",
  "settings.sync.key.addConfirm": "Continua",
  "settings.sync.key.addedToast": "Chiave di sicurezza aggiunta.",
  "settings.sync.key.noBackup":
    "Una chiave di sicurezza sarebbe il tuo unico secondo passaggio. Se la perdi, potresti non riuscire più ad accedere al tuo account di sincronizzazione. Consigliamo una riserva: una seconda chiave o un codice di un'app di autenticazione.",
  "settings.sync.key.recommend":
    "Consigliamo due metodi, ad esempio due chiavi, oppure una chiave e un codice di un'app di autenticazione.",
  "settings.sync.key.remove": "Rimuovi",
  "settings.sync.key.removeLabel": "Rimuovi {name}",
  "settings.sync.key.removeTitle": "Rimuovere {name}?",
  "settings.sync.key.removeBody":
    "Confermi una volta con una chiave di sicurezza nel browser. Dopo, questa chiave non funziona più per accedere.",
  "settings.sync.key.removeLast":
    "Questo è il tuo ultimo secondo passaggio. La verifica in due passaggi si disattiva e il tuo account di sincronizzazione è protetto solo dalla password principale e dalla Chiave segreta.",
  "settings.sync.key.removeCode": "Codice dell'app di autenticazione",
  "settings.sync.key.removeCodeHint":
    "Inserisci un codice attuale per rimuovere subito la chiave. Lascia vuoto per confermare invece con una chiave di sicurezza nel browser.",
  "settings.sync.key.removeConfirm": "Rimuovi chiave",
  "settings.sync.key.removedToast": "Chiave di sicurezza rimossa.",
  "settings.sync.key.confirmTitle": "Conferma con una chiave di sicurezza",
  "settings.sync.key.confirmBody":
    "Per rimuovere una chiave serve un secondo passaggio. Inserisci la tua password principale, poi conferma con una chiave nel browser.",
  "settings.sync.key.step.methods": "Controllo di come accede questo account…",
  "settings.sync.key.step.useKey": "Usa la chiave di sicurezza (apre il browser)",
  "settings.sync.key.step.keyOnlyTitle": "Usa la tua chiave di sicurezza",
  "settings.sync.key.step.keyOnlyBody":
    "Questo account è protetto da una chiave di sicurezza. Mosaic apre una pagina nel browser, dove la usi.",
  "settings.sync.key.step.or": "Oppure usa un codice",
  "settings.sync.key.wait.login": "Accedi con la tua chiave di sicurezza",
  "settings.sync.key.wait.register": "Aggiungi la tua chiave di sicurezza",
  "settings.sync.key.wait.confirm": "Conferma con la tua chiave di sicurezza",
  "settings.sync.key.wait.lead":
    "Il browser ha aperto una pagina di Mosaic dal tuo server di sincronizzazione. Digita lì questo codice, poi usa la tua chiave di sicurezza.",
  "settings.sync.key.wait.codeLabel": "Codice da digitare nel browser",
  "settings.sync.key.wait.codeSpoken": "Codice da digitare nel browser: {digits}",
  "settings.sync.key.wait.expires": "Questo passaggio scade alle {time}.",
  "settings.sync.key.wait.waiting": "In attesa della tua chiave di sicurezza…",
  "settings.sync.key.wait.done": "Fatto.",
  "settings.sync.key.wait.expired": "Questo passaggio è scaduto. Ricomincia.",
  "settings.sync.key.wait.failed":
    "Il passaggio non è riuscito: il codice era sbagliato troppe volte, oppure la chiave di sicurezza non è stata accettata. Non è stato cambiato nulla. Ricomincia.",
  "settings.sync.key.wait.reopen": "Riapri la pagina del browser",
  "settings.sync.key.wait.retry": "Ricomincia",
  "sync.warning.region": "Avvisi di sincronizzazione",
  "sync.warning.unknownDevice": "un altro dispositivo",
  "sync.warning.unknownVault": "una delle tue casseforti",
  "sync.warning.freezeSuspected":
    "Le modifiche di {device} non arrivano da più di un giorno. Controlla che l'altro dispositivo sia acceso e online.",
  "sync.warning.missingFiles":
    "Nella cartella mancano file di {device}. Lascia che il tuo servizio cloud finisca di sincronizzare, poi sincronizza di nuovo.",
  "sync.warning.noCommonView":
    "Questo dispositivo e {device} non vedono gli stessi dati del server da due giorni. Il server potrebbe mostrare loro versioni diverse.",
  "sync.warning.rollbackDetected":
    "I tuoi dati di sincronizzazione sembrano più vecchi di quelli di questo dispositivo. Controlla il server o la cartella, poi scegli “Ho controllato”.",
  "sync.warning.identityDuplicated":
    "Un'altra copia di Mosaic sta scrivendo con l'identità di questo dispositivo, ad esempio dopo il ripristino di un backup. La sincronizzazione è in pausa. Assicurati che sia in uso una sola copia, poi scegli “Ho controllato”.",
  "sync.warning.serverInconsistent":
    "Il server ha mostrato a questo dispositivo dati diversi da quelli di un altro dispositivo. Controlla il server, poi scegli “Ho controllato”.",
  "sync.warning.serverRollback":
    "La copia di {vault} sul server è più vecchia di quella di questo dispositivo. Controlla il server, poi scegli “Ho controllato”.",
  "sync.warning.undecryptable":
    "{count, plural, one {# elemento ricevuto non ha potuto essere letto} other {# elementi ricevuti non hanno potuto essere letti}} su questo dispositivo. Aggiorna Mosaic su tutti i tuoi dispositivi, poi sincronizza di nuovo.",
  "sync.warning.hardwareVaultsLocal":
    "Le casseforti protette dall'hardware restano su questo dispositivo. {count, plural, one {# cassaforte non è sincronizzata} other {# casseforti non sono sincronizzate}}.",
  "sync.warning.attachmentsNotRepaired":
    "{count, plural, one {# file} other {# file}} sul server o nella cartella non ha potuto essere riparato da questo dispositivo. Apri l'elemento su un dispositivo che ha ancora il file e aggiungilo di nuovo.",
  "sync.warning.acknowledge": "Ho controllato…",
  "sync.acknowledge.title": "Cancellare gli avvisi di sincronizzazione che richiedono un controllo?",
  "sync.acknowledge.body":
    "Procedi solo dopo aver controllato tu stesso il server o la cartella. Questo cancella ogni avviso che richiede un controllo e la sincronizzazione prosegue. Se i tuoi dati sono stati davvero riportati a uno stato precedente o modificati, Mosaic non potrà più accorgersene.",
  "sync.acknowledge.eachDevice": "Fallo su ciascuno dei tuoi dispositivi.",
  "toast.syncAttention":
    "La sincronizzazione ha unito o riparato alcuni elementi di un altro dispositivo. Dai un'occhiata a cosa è cambiato.",
  "sync.revoked.rekey":
    "Le chiavi di questo account sono state rinnovate su un altro dispositivo: accedi con la nuova Chiave segreta del tuo Kit di emergenza più recente.",
  "sync.revoked.passwordChange":
    "La tua password principale è stata cambiata su un altro dispositivo: accedi con la nuova password.",
  "sync.revoked.deviceRemoved":
    "Questo dispositivo è stato rimosso dal tuo account da un altro dispositivo. Accedi di nuovo per aggiungerlo di nuovo.",
  "sync.revoked.operator":
    "Il gestore del server ha terminato questa sessione. Accedi di nuovo per continuare a sincronizzare.",
} satisfies Record<keyof typeof en, string>;
