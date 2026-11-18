/**
 * Italian UI strings: errors. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { errors as en } from "../en/errors.ts";

export const errors = {
  "error.notImplemented":
    "Questa funzione non è ancora disponibile. Arriverà in una prossima versione di Mosaic.",
  "error.noAccount":
    "Su questo dispositivo non c'è ancora un account. Creane uno, oppure accedi con il tuo Kit di emergenza.",
  "error.accountExists": "Questo dispositivo ha già un account Mosaic. Sbloccalo.",
  "error.locked": "Mosaic è bloccato. Sbloccalo per continuare.",
  "error.alreadyUnlocked": "Mosaic è già sbloccato. Puoi continuare da dove eri rimasto.",
  "error.wrongCredentials":
    "La password principale o la Chiave segreta non ha funzionato. Controlla il Blocco maiuscole e il layout della tastiera, poi riprova.",
  "error.secretKeyRequired":
    "Questo dispositivo non ha la tua Chiave segreta. Digitala dal tuo Kit di emergenza.",
  "error.secretKeyMalformed":
    "Questa non è una Chiave segreta. La tua inizia con “M1-” e si trova nel tuo Kit di emergenza.",
  "error.secretKeyMistyped":
    "Questa Chiave segreta contiene un errore di battitura. Confrontala con il tuo Kit di emergenza, carattere per carattere.",
  "error.emptyPassword": "La password principale è vuota. Inseriscila per continuare.",
  "error.quickUnlockUnavailable":
    "Lo sblocco rapido non è configurato su questo dispositivo. Sblocca con la password principale.",
  "error.storeInUse":
    "Mosaic è già aperto su questo computer. Chiudi l'altra finestra, poi riprova.",
  "error.storeUnavailable":
    "Mosaic non è riuscito ad aprire il file della tua cassaforte su questo dispositivo. Controlla che il disco sia disponibile e abbia spazio libero, poi riavvia Mosaic.",
  "error.vaultNotFound":
    "Questa cassaforte non esiste più. Potrebbe essere stata eliminata su un altro dispositivo.",
  "error.vaultUnavailable":
    "Questa cassaforte non può essere aperta in questa sessione. Sbloccala con la sua chiave hardware, oppure sincronizza e riprova.",
  "error.itemNotFound":
    "Questo elemento non esiste più. Potrebbe essere stato spostato o eliminato su un altro dispositivo.",
  "error.itemDeleted":
    "Questo elemento è stato eliminato definitivamente, quindi non può essere aperto. Scegli un altro elemento.",
  "error.sameVault": "L'elemento è già in quella cassaforte. Scegli una cassaforte diversa.",
  "error.noOtp": "Questo elemento non ha un codice monouso. Modificalo per aggiungerne uno.",
  "error.fieldEmpty":
    "Quel campo è vuoto, quindi non c'è nulla da copiare o mostrare. Modifica l'elemento per compilarlo.",
  "error.invalidInput":
    "Qualcosa in questo modulo non è valido. Controlla i campi evidenziati e riprova.",
  "error.sshKeyInvalid":
    "Questa non è una chiave privata OpenSSH. Incolla la chiave intera, comprese le righe BEGIN e END.",
  "error.reauthRequired":
    "Questa modifica richiede la tua password principale. Inseriscila per confermare.",
  "error.attachmentTooLarge": "Quel file supera i 50 MiB. Scegli un file più piccolo.",
  "error.attachmentQuota":
    "I tuoi file userebbero più del 1 GiB disponibile. Rimuovi alcuni allegati, poi riprova.",
  "error.attachmentNotFound":
    "Questo file non fa più parte dell'elemento. Ricarica l'elemento.",
  "error.attachmentUnavailable":
    "Questo file non può essere aperto su questo dispositivo. Potrebbe non essere ancora sincronizzato o essere danneggiato.",
  "error.attachmentFileUnreadable": "Quel file non può essere letto. Scegli un altro file.",
  "error.attachmentSaveFailed": "Non è stato possibile salvare il file lì. Scegli un'altra cartella.",
  "error.attachmentPreviewUnsupported":
    "Per questo file non c'è un'anteprima. Usa Salva con nome per aprirlo.",
  "error.attachmentPreviewTooLarge":
    "Questa immagine è troppo grande per l'anteprima. Usa Salva con nome per aprirla.",
  "error.clipboardUnavailable":
    "Mosaic non è riuscito a usare gli appunti. Chiudi le app che potrebbero tenerli occupati, poi riprova.",
  "error.importFileUnreadable":
    "Mosaic non è riuscito a leggere quel file. Controlla che esista ancora e non sia aperto in un'altra app, poi riprova.",
  "error.importDeleteNotAllowed":
    "Importa prima il file. Mosaic elimina un'esportazione solo dopo che i suoi elementi sono nella tua cassaforte.",
  "error.importDeleteFailed":
    "Non è stato possibile eliminare il file. Potrebbe essere aperto in un'altra app o di sola lettura. Eliminalo tu, poi svuota il cestino.",
  "error.importFormatInvalid":
    "Quel file non è nel formato che hai scelto. Scegli il formato corrispondente, oppure esporta di nuovo il file.",
  "error.importPreviewExpired":
    "Questa importazione è stata annullata. Scegli di nuovo il file per ricominciare.",
  "error.sshRequestExpired":
    "Quella richiesta SSH è già terminata. Esegui di nuovo il comando se ti serve ancora.",
  "error.sshAgentUnavailable":
    "L'agente SSH non è riuscito ad avviarsi. Controlla che non sia in esecuzione un altro agente SSH, poi riattivalo.",
  "error.unlockDelayed":
    "Troppi tentativi sbagliati di fila. Attendi qualche secondo, poi riprova.",
  "error.tooManyAttempts":
    "Troppe risposte sbagliate. Mosaic si è bloccato. Sbloccalo con la password principale per continuare.",
  "error.sshAllowTooSoon":
    "Troppo veloce per leggere la richiesta. Controlla la chiave e il programma, poi scegli di nuovo Consenti.",
  "error.browserRequestExpired":
    "Quella richiesta del browser è già terminata. Chiedi di nuovo dall'estensione se ti serve ancora.",
  "error.browserAllowTooSoon":
    "Troppo veloce per leggere la richiesta. Controlla i dettagli, poi scegli di nuovo.",
  "error.browserPairingNotFound":
    "Quel browser non è più collegato. Riapri le Impostazioni per vedere l'elenco aggiornato.",
  "error.browserHostMissing":
    "Questa copia di Mosaic non ha nessun connettore del browser installato, quindi non c'è nulla da riparare. Installa Mosaic con il suo programma di installazione per collegare un browser.",
  "error.agentOriginInvalid":
    "Gli agenti possono inviare una chiave solo a un sito che inizia con https:// e indica solo il sito, come https://api.example.com. Controlla l'indirizzo.",
  "error.agentBindingInvalid":
    "Mosaic non può usare questa impostazione per gli agenti. Controlla il campo, i siti e come viene inviata la chiave.",
  "error.agentValueUnusable":
    "Questo valore è troppo corto o contiene un'interruzione di riga, quindi gli agenti non possono usarlo. Una chiave richiede almeno 12 caratteri su una sola riga.",
  "error.agentVaultShared":
    "Gli agenti possono usare solo elementi di una cassaforte che è tua e non è condivisa. Sposta prima l'elemento in una delle tue casseforti.",
  "error.agentAccessOff":
    "L'accesso degli agenti è disattivato. Attivalo in Impostazioni, sotto Agenti.",
  "error.agentRequestExpired":
    "Quella richiesta dell'agente è già terminata. L'agente può chiedere di nuovo se gli serve ancora.",
  "error.agentAllowTooSoon":
    "Troppo veloce per leggere la richiesta. Controlla i dettagli, poi scegli di nuovo.",
  "error.agentPairingNotFound":
    "Quell'agente non è più collegato. Riapri le Impostazioni per vedere l'elenco aggiornato.",
  "error.agentEndpointUnavailable":
    "Mosaic non è riuscito ad aprire la connessione usata dagli agenti. Chiudi le altre copie di Mosaic e riprova.",
  "error.kitNotVerified":
    "Salva prima il nuovo Kit di emergenza e conferma il suo ultimo gruppo. Prima di allora Mosaic non cambia nulla.",
  "error.kitGroupMismatch":
    "Questo non è l'ultimo gruppo della nuova Chiave segreta. Controlla il nuovo Kit di emergenza.",
  "error.rotationInProgress":
    "Un altro rinnovo delle chiavi è ancora aperto. Terminalo con Continua il rinnovo delle chiavi, oppure scartalo prima.",
  "error.rotationNone": "Non c'è nessun rinnovo delle chiavi da continuare.",
  "error.rotationNotFound":
    "Quel cambio di password non è più aperto. Potrebbe essere stato chiuso su un altro dispositivo o nel browser. Riapri l'elemento per vedere a che punto è.",
  "error.rotationLimit":
    "Sono già aperti quattro cambi di password di questo elemento. Completane o chiudine uno nell'elemento prima.",
  "error.rotationOpen":
    "Questo accesso ha un cambio di password aperto. Ripristinalo dal cestino, poi scegli Ha funzionato o Non ha funzionato sull'elemento. Dopo potrai eliminarlo definitivamente.",
  "error.rotationNotSupported":
    "Mosaic non può guidarti in questo cambio. Serve un accesso con un indirizzo https. Cambia tu la password sul sito.",
  "error.historyNotFound":
    "Quella password precedente non è più nella cronologia. Riapri l'elemento per vedere l'elenco aggiornato.",
  "error.originNotRelated":
    "Quella pagina non appartiene al sito di questo accesso, quindi Mosaic non l'ha aggiunta. Modifica gli indirizzi dell'elemento se dovrebbe esserci.",
  "error.breachConsentRequired":
    "Il controllo delle violazioni è disattivato. Attivalo prima e accetta ciò che invia.",
  "error.breachRunning":
    "Un controllo delle violazioni è già in corso. Attendi che finisca, oppure interrompilo prima.",
  "error.rotationTooFar":
    "Questa modifica ha già raggiunto il tuo account, quindi non può essere scartata. Continuala per completarla.",
  "error.rotationJumpTooLarge":
    "Questo dispositivo è troppo indietro rispetto agli altri per aggiornarsi. Configuralo di nuovo: rimuovi l'account da questo dispositivo e rientra con il tuo Kit di emergenza.",
  "error.rotationNotApplied":
    "La modifica non ha mai raggiunto il tuo account, quindi non è stato cambiato nulla. Ricomincia o scartala.",
  "error.secretKeyUnchanged":
    "Questa Chiave segreta non funziona più. Usa quella del Kit di emergenza che hai salvato sul dispositivo che ha fatto la modifica.",
  "error.passwordUnchanged":
    "La nuova password principale deve essere diversa da quella attuale.",
  "error.profileDowngrade":
    "Quel livello non è più forte di quello che usa ora il tuo account. Scegline uno più alto.",
  "error.profileTooSlow":
    "Con quel livello lo sblocco richiederebbe a questo dispositivo più di due secondi. Scegli un livello più basso, oppure passa comunque al livello superiore.",
  "error.rekeyNeedsCleanSync":
    "Il rinnovo delle chiavi richiede che la sincronizzazione sia aggiornata e senza avvisi. Sincronizza ora, controlla gli eventuali avvisi, poi riprova.",
  "error.rekeyMemberKeyMismatch":
    "Non hai ancora confrontato il codice di sicurezza di tutti i membri di una cassaforte condivisa, quindi quella cassaforte non può essere rinnovata. Confronta i loro codici nelle impostazioni di condivisione della cassaforte, poi riprova.",
  "error.rekeyBusy":
    "Le tue casseforti hanno continuato a cambiare mentre Mosaic lavorava. Attendi un momento e riprova.",
  "error.rekeyOutcomeUnknown":
    "Mosaic non è riuscito a confermare se il tuo account ha accettato le nuove chiavi. Scegli Continua il rinnovo delle chiavi per scoprirlo.",
  "error.rekeyNeedsServer":
    "Il rinnovo delle chiavi richiede il tuo server o la tua cartella di sincronizzazione, che non sono raggiungibili. Controlla la connessione e riprova.",
  "error.rekeyNoAccountFile":
    "Nella tua cartella di sincronizzazione non c'è nulla che si apra con questa password e questa Chiave segreta. Controlla il nuovo Kit di emergenza.",
  "error.syncAccountRekeyed":
    "Le chiavi del tuo account sono state rinnovate su un altro dispositivo. Per continuare usa la Chiave segreta del nuovo Kit di emergenza.",
  "error.syncEpochRollback":
    "Il tuo servizio di sincronizzazione mostra chiavi dell'account più vecchie di quelle di questo dispositivo. Non è stato cambiato nulla.",
  "error.syncRekeyRefused":
    "Le chiavi offerte dal tuo servizio di sincronizzazione non sono le successive a quelle di questo dispositivo. Non è stato cambiato nulla. Controlla gli altri dispositivi.",
  "error.pickerStale": "Questo elenco non è aggiornato. Fai di nuovo clic sul campo.",
  "error.pickerTooSoon": "Attendi un momento, poi conferma di nuovo.",
  "error.programFillUnavailable":
    "La compilazione automatica nei programmi non è disponibile su questo dispositivo.",
  "error.busy":
    "Mosaic sta ancora completando l'ultimo passaggio. Attendi un momento, poi riprova.",
  "error.internal":
    "Qualcosa di imprevisto ha interrotto questa azione. Riprova e, se continua, riavvia Mosaic.",
  "error.importTooLarge":
    "Questo file è troppo grande o troppo complesso da importare. Esporta meno elementi alla volta, poi riprova.",
  "error.importUnsupported":
    "Mosaic non può leggere questa versione del formato. Esporta di nuovo in un formato attuale (per KeePass: KDBX 4 con AES o ChaCha20), poi riprova.",
  "error.importEncrypted":
    "Questa esportazione è protetta da password. Esporta di nuovo senza cifratura, importala, poi elimina il file.",
  "error.sshAgentEndpointInUse":
    "Un altro agente SSH è già in esecuzione (su Windows, il servizio OpenSSH Authentication Agent). Fermalo, poi riattiva l'agente SSH di Mosaic.",
  "error.cancelled": "Annullato. Non è cambiato nulla.",
  "error.quickUnlockExpired":
    "Lo sblocco rapido è scaduto. Sblocca con la password principale, poi riattivalo nelle Impostazioni.",
  "error.quickUnlockInvalidated":
    "Lo sblocco rapido ha smesso di funzionare su questo dispositivo ed è stato disattivato. Sblocca con la password principale.",
  "error.passwordRequired":
    "Questa volta inserisci la password principale. Dopo, lo sblocco rapido funziona di nuovo.",
  "error.hardwareKeyUnsupported":
    "Questa chiave di sicurezza o questo sistema non può essere usato con Mosaic.",
  "error.hardwareKeyNotFound": "Nessuna chiave di sicurezza trovata. Inseriscila e riprova.",
  "error.hardwareKeyTimeout": "La chiave non è stata toccata in tempo. Riprova.",
  "error.hardwareKeyPinRequired": "Inserisci il PIN della tua chiave di sicurezza.",
  "error.hardwareKeyPinInvalid":
    "PIN errato. Dopo diversi tentativi sbagliati la chiave si blocca da sola.",
  "error.hardwareKeyPinBlocked":
    "Questa chiave è bloccata dopo troppi PIN errati. Reimpostala con lo strumento del produttore; le sue registrazioni andranno perse.",
  "error.hardwareKeyNoPin": "Imposta prima un PIN su questa chiave, con lo strumento del produttore.",
  "error.hardwareKeyNotRecognized":
    "Questa chiave non è registrata qui. Prova un'altra chiave o la password principale.",
  "error.hardwareKeyLimit":
    "Hai registrato il numero massimo di chiavi consentito (8). Rimuovine prima una.",
  "error.hardwareKeyLast":
    "Questa è l'unica chiave della cassaforte. Registrane un'altra prima di rimuoverla.",
  "error.itemTypeNotAllowed": "Questa cassaforte contiene solo codici monouso.",
  "error.otpScanUnsupported":
    "La scansione dello schermo non è disponibile su questo sistema. Incolla invece uno screenshot.",
  "error.otpQrNotFound":
    "Nessun codice QR trovato. Assicurati che il codice sia visibile per intero, poi riprova.",
  "error.otpQrNotOtp":
    "Questo codice QR non è un codice di configurazione della verifica in due passaggi. Cerca il codice nella pagina di configurazione 2FA del sito.",
  "error.otpQrMigration":
    "Questo è un codice di esportazione di Google Authenticator, che Mosaic non sa ancora leggere. Configura di nuovo ogni account sul suo sito, oppure inserisci la sua chiave di configurazione.",
  "error.otpQrMultiple":
    "È visibile più di un codice di configurazione. Mostra solo quello che vuoi, poi riprova.",
  "error.otpQrInvalid":
    "Questo codice di configurazione usa impostazioni che Mosaic non supporta. Inserisci invece la chiave di configurazione.",
  "error.otpUriInvalid": "Questo non è un link otpauth:// valido. Copia di nuovo il link intero.",
  "error.otpKeyInvalid":
    "Questa chiave non è valida. Usa solo le lettere da A a Z e le cifre da 2 a 7; controlla eventuali errori di battitura.",
  "error.otpImageUnreadable": "Mosaic non riesce a leggere questa immagine. Usa un file PNG o JPEG.",
  "error.otpImageTooLarge":
    "Questa immagine è troppo grande. Usa uno screenshot del solo codice QR.",
  "error.otpClipboardNoImage":
    "Gli appunti non contengono immagini. Fai uno screenshot del codice QR, poi incollalo qui.",
  "error.otpPendingExpired": "Il codice scansionato non è più disponibile. Scansionalo di nuovo.",
  "error.syncNotConfigured":
    "La sincronizzazione non è ancora configurata su questo dispositivo. Configurala prima nelle Impostazioni.",
  "error.syncAlreadyConfigured":
    "La sincronizzazione è già configurata. Scollegala prima per usare un altro server o un'altra cartella.",
  "error.syncServerUnreachable":
    "Mosaic non è riuscito a raggiungere il server. Controlla l'indirizzo e la connessione a internet, poi riprova.",
  "error.syncServerInvalid":
    "Questo non sembra un server Mosaic. Controlla l'indirizzo; dovrebbe essere simile a https://sync.example.com.",
  "error.syncServerInsecure":
    "Mosaic si collega ai server solo tramite https, così nessuno lungo il percorso può leggere o modificare i tuoi dati. Usa un indirizzo https://.",
  "error.syncServerMismatch":
    "Questo non è il server con cui si sincronizza questo account. Controlla l'indirizzo.",
  "error.syncServerIncompatible":
    "Questo server usa una versione che non corrisponde al tuo Mosaic. Aggiorna Mosaic o chiedi al gestore del server di aggiornarlo.",
  "error.syncRegistrationClosed":
    "Questo server non accetta nuovi account di sincronizzazione. Accedi a uno esistente o chiedi al gestore del server.",
  "error.syncInviteInvalid":
    "Questo codice d'invito non è valido o è già stato usato. Controllalo oppure chiedi al gestore del server uno nuovo.",
  "error.syncAccountExists":
    "Hai già un account di sincronizzazione su questo server. Scegli invece Accedi.",
  "error.syncAccountMismatch":
    "L'account di sincronizzazione su quel server appartiene a un altro account Mosaic. Controlla l'indirizzo e la tua email.",
  "error.syncTwoFactorRequired":
    "Questo account di sincronizzazione richiede un codice dalla tua app di autenticazione. Inserisci il codice di 6 cifre.",
  "error.syncTwoFactorInvalid":
    "Quel codice non è corretto o è scaduto. Inserisci il codice di 6 cifre attuale dalla tua app di autenticazione.",
  "error.syncSignedOut":
    "Il server ha disconnesso questo dispositivo. Accedi di nuovo per continuare a sincronizzare. Le tue modifiche sono al sicuro su questo dispositivo.",
  "error.syncRateLimited": "Troppi tentativi. Attendi qualche minuto, poi riprova.",
  "error.syncQuotaExceeded":
    "Il server o la cartella non ha più spazio per i tuoi dati. Libera spazio o chiedi al gestore del server di fornirne di più.",
  "error.syncFolderUnavailable":
    "Mosaic non può usare questa cartella. Controlla che esista, che il tuo servizio cloud abbia finito di sincronizzarla e che Mosaic possa scriverci.",
  "error.syncFolderNotEmpty":
    "Questa cartella contiene altri file, quindi Mosaic non ci scriverà. Scegli una cartella vuota o creane una nuova.",
  "error.syncFolderNoAccount":
    "Questa cartella non contiene dati di sincronizzazione per questo account. Scegli la cartella con cui si sincronizza il tuo altro dispositivo.",
  "error.syncFolderDamaged":
    "Alcuni file di sincronizzazione in questa cartella sono danneggiati. Lascia che il tuo servizio cloud finisca di sincronizzare oppure ripristina la cartella da un backup, poi riprova.",
  "error.syncPasswordOutcomeUnknown":
    "Non siamo riusciti a confermare se il server ha cambiato la tua password principale. Conserva entrambe le password finché la sincronizzazione non conferma quale è valida.",
  "error.syncWebauthnUnavailable":
    "Il tuo server non può usare una chiave di sicurezza per questa operazione in questo momento, oppure questo account non ne ha. Usa invece un codice dell'app di autenticazione.",
  "error.syncWebauthnLimit":
    "Questo account ha già il numero massimo di chiavi di sicurezza consentito dal server. Rimuovine una che non usi più, poi riprova.",
  "error.syncHandoffExpired": "Il passaggio nel browser è scaduto. Ricomincia.",
  "error.syncHandoffFailed":
    "Troppi codici errati o controlli non riusciti nel browser. Ricomincia il passaggio.",
  "error.syncPasswordNeedsServer":
    "Per cambiare la password principale serve il tuo server di sincronizzazione, che non è raggiungibile. Controlla la connessione a internet e riprova.",
  "error.syncDeviceIsCurrent":
    "Non puoi rimuovere il dispositivo che stai usando. Usa invece Scollega.",
  "error.syncChoiceExpired": "La scelta della cartella è scaduta. Scegli di nuovo la cartella.",
  "error.syncFreshLoginExpired":
    "La verifica della password è scaduta. Inserisci di nuovo la password principale.",
  "error.syncPasswordRollback":
    "Il server ha offerto una password principale più vecchia. Controlla il server.",
  "error.syncPasswordChanged":
    "La tua password principale è stata cambiata sul server o nella cartella. Inserisci quella nuova qui sotto.",
  "error.sharingNotHosted":
    "La condivisione richiede un server Mosaic. Collegane prima uno nelle impostazioni di Sincronizzazione.",
  "error.setupCodeNeedsServer": "Un telefono può unirsi solo a un account che si sincronizza.",
  "error.sharingContactUnverified":
    "Confronta prima il codice di sicurezza di questa persona. Mosaic condivide solo con persone che hai verificato.",
  "error.sharingKeyChanged":
    "La chiave di questa persona è cambiata, oppure il codice non corrisponde. Cercala di nuovo e confronta il nuovo codice prima di proseguire.",
  "error.sharingRoleForbidden":
    "Il tuo ruolo in questa cassaforte non lo permette. Chiedi al proprietario o a un gestore.",
  "error.sharingInviteExists":
    "Un invito per questa persona e questa cassaforte è già in attesa. Ritiralo prima se vuoi inviarne uno nuovo.",
  "error.sharingInviteStale":
    "L'invito è cambiato nel frattempo, ad esempio dopo un cambio di chiave. Aprilo di nuovo.",
  "error.sharingInviteExpired":
    "Questo invito è scaduto o è stato ritirato. Chiedi a chi l'ha inviato di invitarti di nuovo.",
  "error.sharingInviteUnreadable":
    "Questo invito non può essere aperto. Potrebbe essere danneggiato o non destinato a questo account. Chiedi a chi l'ha inviato di invitarti di nuovo.",
  "error.sharingInviteNotInRoster":
    "L'elenco dei membri della cassaforte non corrisponde a questo invito, quindi Mosaic non l'ha aperto. Chiedi a chi l'ha inviato di invitarti di nuovo.",
  "error.sharingOwnerCannotLeave":
    "Il proprietario non può lasciare una cassaforte. Rimuovi invece le altre persone.",
  "error.sharingAlreadyMember": "Questa persona è già membro della cassaforte.",
  "error.sharingReinviteBlocked":
    "Hai rimosso questa persona di recente. Potrai invitarla di nuovo dopo che la chiave della cassaforte sarà cambiata. Sincronizza, poi riprova.",
  "error.sharingInviteLimit":
    "Ci sono troppi inviti in attesa. Aspetta le risposte o ritirane alcuni.",
  "error.sharingVaultKnown":
    "Questo dispositivo ha già quella cassaforte, quindi Mosaic non ha accettato una chiave per essa. Chiedi a chi l'ha inviata quale cassaforte ha condiviso.",
  "error.syncHasSharedVaults":
    "Non puoi cambiare server finché su questo dispositivo ci sono casseforti condivise. Lascia prima le casseforti condivise, oppure fatti rimuovere dai loro proprietari.",
  "error.emergencyWaiting":
    "Una richiesta è in attesa, quindi ora questo non si può modificare. Rifiutala o aspettala.",
  "error.emergencyCooldown":
    "La tua ultima richiesta è stata rifiutata. Potrai chiedere di nuovo dopo il periodo di attesa di un giorno.",
  "error.emergencyNotGranted":
    "Non c'è nulla da richiedere: nessuna richiesta è aperta, oppure la persona non ti ha concesso l'accesso.",
  "error.shortcutUnavailable":
    "Un altro programma usa già questa scorciatoia, oppure Mosaic non può registrarla. Scegli una combinazione diversa.",
  "error.shortcutUnsupported":
    "Il tuo sistema non permette alle app di impostare una scorciatoia globale. Apri invece Mosaic dalla sua finestra.",
} satisfies Record<keyof typeof en, string>;
