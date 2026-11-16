/**
 * Italian UI strings: mobile. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { mobile as en } from "../en/mobile.ts";

export const mobile = {
  "mobile.app.name": "Mosaic",
  "mobile.nav.vault": "Cassaforte",
  "mobile.nav.search": "Cerca",
  "mobile.nav.settings": "Impostazioni",
  "mobile.common.cancel": "Annulla",
  "mobile.common.done": "Fine",
  "mobile.common.close": "Chiudi",
  "mobile.common.continue": "Continua",
  "mobile.unlock.title": "Sblocca Mosaic",
  "mobile.unlock.passwordLabel": "Password principale",
  "mobile.unlock.action": "Sblocca",
  "mobile.unlock.quick": "Usa lo sblocco rapido",
  "mobile.unlock.failed":
    "{attempts, plural, one {Non ha funzionato # volta. Mosaic non può reimpostare la tua password principale. Se l'hai cambiata su un altro dispositivo, usa quella nuova.} other {Non ha funzionato # volte. Mosaic non può reimpostare la tua password principale. Se l'hai cambiata su un altro dispositivo, usa quella nuova.}}",
  "mobile.lock.now": "Blocca ora",
  "mobile.vault.title": "La tua cassaforte",
  "mobile.vault.count": "{count, plural, one {# elemento} other {# elementi}}",
  "mobile.vault.emptyTitle": "Ancora nessun elemento",
  "mobile.vault.emptyBody":
    "Scorri verso il basso per sincronizzare con gli altri dispositivi, oppure aggiungi il tuo primo accesso.",
  "mobile.search.placeholder": "Cerca nella tua cassaforte",
  "mobile.copy.done":
    "Copiato. Viene cancellato dagli appunti tra {seconds, plural, one {# secondo} other {# secondi}}.",
  "mobile.setup.scanTitle": "Scansiona il tuo codice di configurazione",
  "mobile.setup.scanHint":
    "Sul computer apri Mosaic, vai su Impostazioni > Sincronizzazione e scegli “Configura un telefono”. Inquadra il codice con la fotocamera.",
  "mobile.autofill.title": "Compila da Mosaic",
  "mobile.autofill.target": "Compilare in {app}?",
  "mobile.settings.title": "Impostazioni",
  "mobile.settings.about": "Informazioni su Mosaic",
  "mobile.settings.version": "Versione {version}",
  "mobile.common.back": "Indietro",
  "mobile.common.notNow": "Non ora",
  "mobile.onboarding.step": "Passaggio {current} di {total}",
  "mobile.onboarding.welcome.title": "Configura Mosaic su questo telefono",
  "mobile.onboarding.welcome.body":
    "Accedi al tuo account Mosaic per portare qui le tue password. Tutto resta cifrato su questo telefono.",
  "mobile.onboarding.welcome.start": "Inizia",
  "mobile.onboarding.details.title": "Il tuo account",
  "mobile.onboarding.details.server": "Indirizzo del server",
  "mobile.onboarding.details.serverHint": "https://sync.example.com",
  "mobile.onboarding.details.email": "Email",
  "mobile.onboarding.details.secretKey": "Chiave segreta",
  "mobile.onboarding.details.secretKeyHelp":
    "Dal tuo Kit di emergenza: lettere e numeri in gruppi, che iniziano con M1.",
  "mobile.onboarding.details.deviceName": "Nome di questo telefono",
  "mobile.onboarding.details.deviceNameDefault": "Telefono Android",
  "mobile.onboarding.details.checking": "Controllo del server",
  "mobile.onboarding.password.title": "La tua password principale",
  "mobile.onboarding.password.body":
    "Mosaic non la invia mai da nessuna parte. Sblocca la tua cassaforte su questo telefono.",
  "mobile.onboarding.password.action": "Accedi",
  "mobile.onboarding.password.working": "Accesso in corso. Può richiedere qualche secondo.",
  "mobile.onboarding.second.title": "Ancora un passaggio",
  "mobile.onboarding.second.totpLabel": "Codice di 6 cifre",
  "mobile.onboarding.second.totpAction": "Verifica",
  "mobile.onboarding.second.keyAction": "Usa la tua chiave di sicurezza",
  "mobile.onboarding.second.keyWaiting":
    "In attesa della tua chiave di sicurezza. Completa l'operazione nel browser, poi torna qui. Il codice mostrato lì dovrebbe essere {code}.",
  "mobile.onboarding.quick.title": "Sblocca più in fretta",
  "mobile.onboarding.quick.body":
    "Usa la tua impronta digitale per sbloccare Mosaic la prossima volta. Dopo un riavvio del telefono ti serve comunque la password principale una volta.",
  "mobile.onboarding.quick.enable": "Attiva lo sblocco rapido",
  "mobile.onboarding.quick.unavailable":
    "Aggiungi un'impronta digitale nelle impostazioni del telefono per usare lo sblocco rapido.",
  "mobile.onboarding.autofill.title": "Compila gli accessi automaticamente",
  "mobile.onboarding.autofill.body":
    "Lascia che Mosaic proponga i tuoi accessi in altre app e nel browser. Puoi cambiarlo in seguito nelle impostazioni di sistema.",
  "mobile.onboarding.autofill.open": "Apri le impostazioni di compilazione automatica",
  "mobile.onboarding.done.title": "È tutto pronto",
  "mobile.onboarding.done.body": "La tua cassaforte è su questo telefono ed è sbloccata.",
  "mobile.onboarding.done.action": "Apri la tua cassaforte",
  "mobile.unlock.account": "Sblocco di {email}",
  "mobile.unlock.secretKey": "Chiave segreta",
  "mobile.unlock.secretKeyHelp":
    "Questo telefono non ha la tua Chiave segreta salvata. Digitala dal tuo Kit di emergenza.",
  "mobile.unlock.enrollNext": "La prossima volta sblocca con la mia impronta digitale",
  "mobile.unlock.working": "Sblocco in corso. Può richiedere qualche secondo.",
  "mobile.unlock.biometricTitle": "Sblocca Mosaic",
  "mobile.unlock.biometricSubtitle": "Conferma che sei tu",
  "mobile.unlock.usePassword": "Usa la password principale",
  "mobile.unlock.busyTitle": "Mosaic è occupato",
  "mobile.unlock.retry": "Riprova",
  "mobile.home.unlockedAs": "Sbloccato come {email}",
  "error.notFound":
    "Questo elemento non esiste più. Potrebbe essere stato eliminato su un altro dispositivo.",
  "error.syncFailed":
    "Mosaic non è riuscito a sincronizzare. Controlla la connessione, poi riprova.",
  "error.passwordInApp":
    "Qui non è possibile sbloccare con la password principale. Apri l'app Mosaic e sbloccala lì.",
  "error.folderProviderUnsupported":
    "Questo telefono non può raggiungere la cartella cloud di questo account. Accedi di nuovo al fornitore della cartella, oppure usa un computer.",
  "error.folderProviderSignedOut":
    "Il tuo servizio cloud ha terminato o rifiutato l'accesso. Accedi di nuovo per continuare a sincronizzare.",
  "error.folderNotFound":
    "Mosaic non trova quella cartella, oppure non è una cartella Mosaic. Scegli la cartella con cui si sincronizza il tuo computer.",
  "error.folderStorageFull":
    "Il tuo spazio cloud è pieno. Libera spazio lì, poi sincronizza di nuovo.",
  "error.folderLinkExpired": "L'accesso ha impiegato troppo tempo. Ricomincia.",
  "error.folderLinkInvalid": "L'accesso non ha funzionato o è stato annullato. Ricomincia.",
  "error.folderServerTlsTooOld":
    "Questo server è troppo vecchio per una connessione sicura. Mosaic richiede TLS 1.3. Chiedi a chi gestisce il server di aggiornarlo.",
  "mobile.error.syncPasswordRollback":
    "La cartella offre una password principale più vecchia di quella che questo telefono conosce. Controlla la cartella e il tuo servizio cloud.",
  "error.setupCodeInvalid":
    "Quel codice di configurazione non è valido o è scaduto. Creane uno nuovo sul computer e scansionalo di nuovo.",
  "error.notEnoughMemory":
    "Questo dispositivo non ha abbastanza memoria libera per sbloccare la tua cassaforte. Chiudi le altre app e riprova.",
  "error.noMatch":
    "Mosaic non ha un accesso per questa app o questo sito web. Apri Mosaic per aggiungerne uno.",
  "error.insecureOrigin":
    "Questa pagina non usa una connessione sicura, quindi Mosaic non la compila. Apri la versione https della pagina.",
  "error.appNotLinked":
    "Questa app non è collegata a un sito web salvato. Collegala prima in Mosaic, poi riprova.",
  "error.foreignFrame":
    "Questo campo appartiene a un altro sito incorporato nella pagina, quindi Mosaic non lo compila.",
  "error.assetLinksUnavailable":
    "Mosaic non è riuscito a verificare che questa app appartenga al sito web. Controlla la connessione e riprova.",
  "error.appNotVerified":
    "Questa app non è verificata per quel sito web, quindi Mosaic non usa le sue passkey. Usa invece il sito web in un browser.",
  "error.rpIdInvalid":
    "Il nome del sito web in questa richiesta non è valido. Annulla e ricomincia.",
  "error.passkeyNotFound":
    "Mosaic non ha una passkey per questo sito web. Accedi in un altro modo.",
  "error.passkeyExcluded":
    "Hai già una passkey per questo account in Mosaic. Accedi invece con quella.",
  "error.passkeyUnsupportedAlgorithm":
    "Questo sito web richiede un tipo di passkey che Mosaic non sa ancora creare. Scegli un altro modo per accedere.",
  "error.passkeySessionExpired":
    "Questa richiesta di passkey è scaduta. Ricomincia dall'app.",
  "error.qrNotFound":
    "Mosaic non ha trovato nessun codice QR in quell'immagine. Riprova con una foto più nitida.",
  "error.storageUnavailable":
    "Mosaic non è riuscito a raggiungere l'archivio sicuro di questo dispositivo. Riavvia il telefono e riprova.",
  "mobile.common.tryAgain": "Riprova",
  "mobile.onboarding.preAudit":
    "Software in fase pre-audit: ancora nessuna revisione di sicurezza esterna.",
  "mobile.onboarding.start": "Inizia",
  "mobile.onboarding.server": "Indirizzo del server",
  "mobile.onboarding.serverPlaceholder": "https://",
  "mobile.onboarding.email": "Email",
  "mobile.onboarding.secretKey": "Chiave segreta",
  "mobile.onboarding.secretKeyHint":
    "La trovi nel tuo Kit di emergenza o nelle Impostazioni sul computer. Maiuscole e trattini non contano.",
  "mobile.onboarding.deviceName": "Nome di questo telefono",
  "mobile.onboarding.passwordHint":
    "Mosaic la controlla su questo telefono. Non viene mai inviata al server.",
  "mobile.onboarding.signIn": "Accedi",
  "mobile.onboarding.working": "Accesso in corso. Può richiedere qualche secondo.",
  "mobile.onboarding.secondFactorTitle": "Ancora un passaggio",
  "mobile.onboarding.totpLabel": "Codice della tua app di autenticazione",
  "mobile.onboarding.verify": "Verifica",
  "mobile.onboarding.securityKey": "Usa la tua chiave di sicurezza",
  "mobile.onboarding.securityKeyWaiting":
    "Completa l'accesso nel browser. Controlla che mostri il codice {code}, poi torna qui.",
  "mobile.onboarding.autofillSteps":
    "Apri l'app Impostazioni, tocca Password, poi Opzioni password, e attiva Mosaic.",
  "mobile.onboarding.openSettings": "Apri le impostazioni",
  "mobile.onboarding.notNow": "Non ora",
  "mobile.unlock.secretKeyNeeded":
    "Questo telefono non ha la tua Chiave segreta. Inseriscila una volta per sbloccare.",
  "mobile.unlock.quickOptionFaceId": "La prossima volta sblocca con Face ID",
  "mobile.unlock.biometricReason": "Sblocca la tua cassaforte Mosaic",
  "mobile.unlock.faceIdReason":
    "Mosaic usa Face ID per sbloccare rapidamente la tua cassaforte. Funziona sempre anche la password principale.",
  "mobile.unlock.quickInvalidated":
    "Le impostazioni di Face ID sono cambiate. Sblocca con la password principale per riattivare lo sblocco rapido.",
  "mobile.unlock.storeBusy": "Mosaic è occupato. Riprova tra un momento.",
  "mobile.unlock.rebootNote": "Dopo un riavvio, sblocca una volta con la password principale.",
  "mobile.capture.hidden": "Nascosto mentre lo schermo viene registrato",
  "mobile.deviceLock.required.title": "Imposta prima un blocco schermo",
  "mobile.deviceLock.required.body":
    "Mosaic tiene le tue chiavi dietro il blocco schermo del telefono. Imposta un PIN, una sequenza o una password nelle impostazioni di sistema, poi torna qui.",
  "mobile.deviceLock.required.action": "Apri le impostazioni di sicurezza",
  "mobile.common.show": "Mostra",
  "mobile.common.hide": "Nascondi",
  "mobile.common.copy": "Copia",
  "mobile.common.save": "Salva",
  "mobile.common.remove": "Rimuovi",
  "mobile.common.edit": "Modifica",
  "mobile.common.working": "Operazione in corso",
  "mobile.vault.add": "Aggiungi elemento",
  "mobile.vault.addLogin": "Accesso",
  "mobile.vault.addNote": "Nota sicura",
  "mobile.vault.untitled": "Senza titolo",
  "mobile.vault.favorite": "Preferito",
  "mobile.search.hint": "Cerca per titolo, nome utente, sito web o tag.",
  "mobile.search.none": "Nessun elemento corrisponde a {query}.",
  "mobile.detail.favoriteAdd": "Aggiungi ai preferiti",
  "mobile.detail.favoriteRemove": "Rimuovi dai preferiti",
  "mobile.detail.delete": "Sposta nel cestino",
  "mobile.detail.deleteTitle": "Spostare “{title}” nel cestino?",
  "mobile.detail.deleteBody": "La modifica viene sincronizzata con gli altri tuoi dispositivi.",
  "mobile.detail.otpLeft": "{seconds, plural, one {# secondo rimasto} other {# secondi rimasti}}",
  "mobile.detail.passkey": "Passkey per {site}",
  "mobile.detail.attachments":
    "{count, plural, one {# file è allegato. Aprilo sul computer.} other {# file sono allegati. Aprili sul computer.}}",
  "mobile.detail.linkedApp": "App: {name}",
  "mobile.detail.needsLinking": "Va collegata prima di poter compilare",
  "mobile.detail.linkApp": "Collega un'app",
  "mobile.editor.newLogin": "Nuovo accesso",
  "mobile.editor.editLogin": "Modifica accesso",
  "mobile.editor.newNote": "Nuova nota",
  "mobile.editor.editNote": "Modifica nota",
  "mobile.editor.title": "Titolo",
  "mobile.editor.username": "Nome utente o email",
  "mobile.editor.password": "Password",
  "mobile.editor.passwordKeep": "Lascia vuoto per conservare la password attuale.",
  "mobile.editor.websites": "Siti web",
  "mobile.editor.websiteHint": "https://example.com",
  "mobile.editor.websiteAdd": "Aggiungi un sito web",
  "mobile.editor.websiteRemove": "Rimuovi il sito web {number}",
  "mobile.editor.notes": "Note",
  "mobile.editor.favorite": "Segna come preferito",
  "mobile.editor.saving": "Salvataggio",
  "mobile.editor.otpTitle": "Codice monouso",
  "mobile.editor.otpHave": "Questo accesso ha un codice monouso.",
  "mobile.editor.otpInput": "Chiave di configurazione o link otpauth",
  "mobile.editor.otpAdd": "Aggiungi il codice",
  "mobile.editor.otpReady": "Pronto: {name}. Viene salvato con l'accesso.",
  "mobile.editor.otpRemove": "Rimuovi il codice",
  "mobile.editor.otpRemoved": "Il codice viene rimosso quando salvi.",
  "mobile.editor.generate": "Genera una password",
  "mobile.generator.title": "Generatore di password",
  "mobile.generator.password": "Caratteri",
  "mobile.generator.passphrase": "Parole",
  "mobile.generator.length": "Lunghezza: {length}",
  "mobile.generator.lowercase": "Lettere minuscole",
  "mobile.generator.uppercase": "Lettere maiuscole",
  "mobile.generator.digits": "Cifre",
  "mobile.generator.symbols": "Simboli",
  "mobile.generator.ambiguous": "Salta i caratteri simili tra loro",
  "mobile.generator.wordCount": "Parole: {count}",
  "mobile.generator.separator": "Separatore",
  "mobile.generator.capitalize": "Parole con l'iniziale maiuscola",
  "mobile.generator.appendDigit": "Aggiungi una cifra",
  "mobile.generator.run": "Genera",
  "mobile.generator.use": "Usa questa password",
  "mobile.generator.strength": "Circa {bits} bit di robustezza",
  "mobile.link.title": "Collega un'app",
  "mobile.link.choose": "Scegli l'app da collegare a {login}.",
  "mobile.link.filter": "Trova un'app",
  "mobile.link.app": "App",
  "mobile.link.package": "ID dell'app",
  "mobile.link.installer": "Installata da",
  "mobile.link.installerUnknown": "Origine sconosciuta",
  "mobile.link.certificate": "Firma dello sviluppatore (inizio)",
  "mobile.link.verified": "Verificata da {host}",
  "mobile.link.notVerified": "Non verificata",
  "mobile.link.unavailable": "Impossibile verificare",
  "mobile.link.checking": "Controllo del sito web",
  "mobile.link.warning":
    "Collega solo app di cui ti fidi. Mosaic non può confermare che questa app appartenga a {host}.",
  "mobile.link.warningNoHost":
    "Collega solo app di cui ti fidi. Questo accesso non ha un sito web che Mosaic possa controllare.",
  "mobile.link.confirm": "D'ora in poi compila {login} in questa app",
  "mobile.link.done": "Collegata. Mosaic compila {login} in questa app.",
  "mobile.link.noApps": "Nessun'altra app trovata.",
  "mobile.settings.security": "Sicurezza",
  "mobile.settings.autoLock": "Blocca quando esci dall'app",
  "mobile.settings.autoLock.now": "Subito",
  "mobile.settings.autoLock.s30": "Dopo 30 secondi",
  "mobile.settings.autoLock.s60": "Dopo 1 minuto",
  "mobile.settings.autoLock.s300": "Dopo 5 minuti",
  "mobile.settings.idle": "Blocca quando smetti di toccare lo schermo",
  "mobile.settings.idleOption": "{minutes, plural, one {Dopo # minuto} other {Dopo # minuti}}",
  "mobile.settings.clipboard": "Cancella i valori copiati",
  "mobile.settings.clipboardOption":
    "{seconds, plural, one {Dopo # secondo} other {Dopo # secondi}}",
  "mobile.settings.quick": "Sblocco rapido",
  "mobile.settings.quickOn": "Lo sblocco rapido è attivo.",
  "mobile.settings.quickOff": "Lo sblocco rapido è disattivato.",
  "mobile.settings.quickEnable": "Attiva",
  "mobile.settings.quickDisable": "Disattiva",
  "mobile.settings.quickPassword": "Digita la tua password principale per attivare lo sblocco rapido.",
  "mobile.settings.sync": "Sincronizzazione",
  "mobile.about.preAudit":
    "Mosaic è un software in fase pre-audit. Non è ancora stato fatto nessun audit di sicurezza indipendente.",
  "mobile.about.privacy":
    "Da questo telefono non esce nessuna analisi d'uso, nessun tracciamento e nessun rapporto sugli arresti anomali.",
  "mobile.about.licences": "Licenze open source",
  "mobile.licences.title": "Licenze open source",
  "mobile.licences.intro": "Mosaic per Android è costruito con queste componenti open source.",
  "mobile.licences.rust":
    "Il nucleo Rust usa crate open source. Le loro licenze sono elencate nel repository dei sorgenti del progetto.",
  "mobile.autofill.unlockEntry": "Sblocca Mosaic",
  "mobile.autofill.noLogin": "Nessun accesso per {app}",
  "mobile.autofill.noLoginHint": "Tocca per collegare un accesso",
  "mobile.autofill.cannotCheck": "Impossibile verificare questa app",
  "mobile.autofill.warning": "Controlla prima l'indirizzo",
  "mobile.autofill.noUsername": "Nessun nome utente",
  "mobile.autofill.otpEntry": "Codice monouso",
  "mobile.link.pickLogin": "Scegli l'accesso che Mosaic deve compilare in {app}.",
  "mobile.credential.unlockEntry": "Sblocca Mosaic",
  "mobile.credential.passwordEntry": "Password",
  "mobile.credential.passkeyEntry": "Passkey",
  "mobile.credential.createEntry": "Salva la passkey in Mosaic",
  "mobile.credential.createTitle": "Salvare una passkey?",
  "mobile.credential.createBody": "Mosaic salverà una passkey per {site} ({user}).",
  "mobile.credential.createVault": "Salva in",
  "mobile.credential.vaultDefault": "Cassaforte personale",
  "mobile.credential.createConfirm": "Salva passkey",
  "mobile.credential.verifyTitle": "Conferma che sei tu",
  "mobile.credential.verifySignIn": "Accedi a {site}",
  "mobile.credential.verifyCreate": "Salva una passkey per {site}",
  "mobile.credential.verifyFailed": "Mosaic non è riuscito a confermare che sei tu. Riprova.",
  "mobile.credential.unsupported":
    "Mosaic non può rispondere a questa richiesta di accesso. Annulla e scegli un altro modo per accedere.",
  "mobile.settings.fill": "Compila in altre app",
  "mobile.settings.autofillOn": "Mosaic è il tuo servizio di compilazione automatica.",
  "mobile.settings.autofillOff": "Mosaic non è ancora il tuo servizio di compilazione automatica.",
  "mobile.settings.autofillEnable": "Usa Mosaic per la compilazione automatica",
  "mobile.settings.credentialHint":
    "Android può anche proporre le tue passkey e password da Mosaic quando un'app ti chiede di accedere.",
  "mobile.settings.credentialEnable": "Usa Mosaic per passkey e password",
  "mobile.scan.setupAction": "Scansiona il codice di configurazione",
  "mobile.scan.otpAction": "Scansiona un codice QR",
  "mobile.scan.otpTitle": "Scansiona il codice monouso",
  "mobile.scan.otpHint":
    "Inquadra con la fotocamera il codice QR del sito web. Mosaic lo legge solo su questo telefono.",
  "mobile.scan.preview": "Vista della fotocamera",
  "mobile.scan.looking": "Ricerca di un codice QR",
  "mobile.scan.notSetupCode": "Non è un codice di configurazione di Mosaic",
  "mobile.scan.notSetupCodeHelp":
    "Inquadra con la fotocamera il codice che Mosaic mostra sul computer, in Impostazioni sotto Sincronizzazione.",
  "mobile.scan.rationale":
    "Mosaic usa la fotocamera solo per leggere questo codice QR. Non viene salvato né inviato nulla.",
  "mobile.scan.allow": "Consenti la fotocamera",
  "mobile.scan.denied":
    "Mosaic non può usare la fotocamera. Consentila nelle impostazioni di questo telefono per scansionare, oppure inserisci i dati a mano.",
  "mobile.scan.openSettings": "Apri le impostazioni",
  "mobile.scan.unavailable":
    "Mosaic non è riuscito ad avviare la fotocamera. Inserisci i dati a mano.",
  "mobile.onboarding.details.scanned":
    "Server, email e Chiave segreta sono compilati dal codice di configurazione. Controllali, poi continua.",
  "mobile.onboarding.details.scanOr": "Oppure inserisci i dati a mano",
  "mobile.onboarding.details.keyFromCode": "Chiave segreta dal codice di configurazione",
  "mobile.settings.passkeyIdentities": "Usa Mosaic per le passkey",
  "mobile.settings.passkeyIdentitiesHint":
    "Mosaic comunica al sistema il sito web e il nome utente di ogni passkey, così può proporla. Le chiavi non escono mai da Mosaic.",
  "mobile.settings.passwordIdentities": "Suggerisci gli accessi sopra la tastiera",
  "mobile.settings.passwordIdentitiesHint":
    "Mosaic comunica al sistema il sito web e il nome utente di ogni accesso. Il sistema conserva questo elenco sul telefono, fuori dalla cifratura di Mosaic.",
  "mobile.settings.identitiesPassword": "Digita la tua password principale per attivarlo.",
  "mobile.licences.introIos": "Mosaic per iPhone è costruito con queste componenti open source.",
  "mobile.ext.openApp": "Apri Mosaic e sbloccalo una volta con la tua password principale.",
  "mobile.ext.noAccount":
    "Mosaic non ha ancora un account su questo iPhone. Apri Mosaic per configurarlo.",
  "mobile.ext.loading": "Apertura della tua cassaforte",
  "mobile.ext.thisSite": "questo sito",
  "mobile.ext.matches": "Accessi per {site}",
  "mobile.ext.noMatches": "Nessun accesso per {site}",
  "mobile.ext.noMatchesHint":
    "Mosaic compila solo gli accessi salvati per questo sito. Apri Mosaic per aggiungere questo sito a un accesso.",
  "mobile.ext.searchAll": "Cerca in tutti gli accessi",
  "mobile.ext.hideAll": "Nascondi gli altri accessi",
  "mobile.ext.notSavedFor": "Non salvato per {site}",
  "mobile.ext.fillHint": "Compila questo accesso",
  "mobile.ext.warnPunycode":
    "Questo indirizzo è scritto con caratteri speciali: {unicode}. Controlla che sia il sito che intendi.",
  "mobile.ext.warnLookalike":
    "Questo indirizzo somiglia a {site}, che hai salvato in Mosaic. Controlla che sia il sito che intendi.",
  "mobile.ext.otpTitle": "Scegli un codice monouso",
  "mobile.ext.passkeyTitle": "Scegli una passkey",
  "mobile.ext.configTitle": "Compilazione automatica di Mosaic",
  "mobile.ext.configBody":
    "Mosaic propone i tuoi accessi, le passkey e i codici monouso quando tocchi un campo o un pulsante di accesso. Cambi ciò che propone in Impostazioni nell'app Mosaic.",
  "mobile.ext.confirmFillTitle": "Compilare l'accesso per {site}?",
  "mobile.ext.confirmFillBody":
    "iOS non dice a Mosaic su quale pagina ti trovi, quindi Mosaic non può verificarlo. Compila solo se questo è il sito che intendi. Poi Mosaic chiede Face ID o Touch ID.",
  "mobile.ext.confirmFillButton": "Compila questo accesso",
  "mobile.settings.passwordIdentitiesLimit":
    "Quando tocchi un suggerimento, iOS non dice a Mosaic su quale pagina ti trovi. Per questo Mosaic mostra il sito salvato e compila solo dopo che lo confermi e sblocchi con Face ID o Touch ID.",
} satisfies Record<keyof typeof en, string>;
