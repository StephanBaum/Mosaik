/**
 * Italian UI strings: ext. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { ext as en } from "../en/ext.ts";

export const ext = {
  "ext.manifest.name": "Mosaic",
  "ext.manifest.description":
    "Compila gli accessi dalla tua cassaforte Mosaic. La password principale resta nell'app desktop.",
  "ext.command.fill": "Compila l'accesso con Mosaic",
  "ext.command.generate": "Genera una password con Mosaic",
  "ext.save.badgeTitle": "Salvare l'accesso?",
  "ext.loading": "Caricamento…",
  "ext.site.secure": "Sito attuale, connessione sicura (https): {host}",
  "ext.site.insecure": "Sito attuale, non cifrato (http): {host}",
  "ext.site.none": "Questa scheda non ha un sito web",
  "ext.nav.label": "Viste",
  "ext.nav.logins": "Accessi e ricerca",
  "ext.nav.generator": "Generatore di password",
  "ext.footer.open": "Apri Mosaic",
  "ext.footer.more": "Altre opzioni",
  "ext.footer.connected": "Collegato a Mosaic",
  "ext.footer.locked": "Mosaic è bloccato",
  "ext.footer.offline": "Non collegato",
  "ext.more.saveOffers": "Proponi di salvare gli accessi",
  "ext.more.saveOffersHint":
    "Richiede l'accesso a tutti i siti web, così l'estensione può accorgersi di un accesso che invii. Non viene salvato nulla finché non confermi nell'app Mosaic.",
  "ext.passkeys.toggle": "Usa Mosaic per le passkey",
  "ext.passkeys.hint":
    "Richiede l'accesso a tutti i siti web per rispondere alle richieste di passkey. Confermi ognuna nell'app Mosaic.",
  "ext.passkeys.permissionDenied":
    "Mosaic ha bisogno dell'accesso ai siti web per rispondere alle richieste di passkey e non l'hai consentito.",
  "ext.more.disconnect": "Scollega da Mosaic",
  "ext.more.disconnectTitle": "Scollegare questo browser?",
  "ext.more.disconnectBody":
    "Questo browser non potrà compilare gli accessi finché non lo ricolleghi. Approvi il nuovo collegamento nell'app Mosaic.",
  "ext.action.cancel": "Annulla",
  "ext.neverSave.menu": "Siti per cui non proporre il salvataggio…",
  "ext.neverSave.title": "Salvataggio mai proposto",
  "ext.neverSave.intro":
    "Mosaic non propone di salvare gli accessi su questi siti. Rimuovi un sito per ricevere di nuovo la richiesta.",
  "ext.neverSave.empty":
    "Ancora nessun sito. Puoi aggiungerne uno con “Mai per questo sito” quando Mosaic propone di salvare un accesso.",
  "ext.neverSave.remove": "Rimuovi",
  "ext.neverSave.removeLabel": "Rimuovi {origin} dall'elenco",
  "ext.neverSave.removed":
    "{origin} rimosso. Mosaic può di nuovo proporre di salvare gli accessi lì.",
  "ext.neverSave.close": "Chiudi",
  "ext.connect.checking": "Collegamento a Mosaic…",
  "ext.connect.hostMissing.title": "Mosaic non è configurato per questo browser",
  "ext.connect.hostMissing.body":
    "Installa l'app Mosaic su questo computer, poi riavvia il browser.",
  "ext.connect.hostMissing.dev": "Sviluppatori: esegui {command}, poi riavvia il browser.",
  "ext.connect.hostForbidden.title": "Questo browser non può collegarsi",
  "ext.connect.hostForbidden.body":
    "Mosaic comunica solo con l'estensione ufficiale di Mosaic. Installala dallo store del tuo browser.",
  "ext.connect.desktopNotRunning.title": "Apri Mosaic su questo computer",
  "ext.connect.desktopNotRunning.body":
    "L'estensione funziona tramite l'app Mosaic. Avvia l'app, poi controlla di nuovo.",
  "ext.connect.check": "Controlla di nuovo",
  "ext.connect.notPaired.title": "Collegati a Mosaic",
  "ext.connect.notPaired.body":
    "Collega questo browser all'app Mosaic su questo computer. Lo approvi nell'app.",
  "ext.connect.start": "Collega",
  "ext.connect.pairing.title": "Conferma in Mosaic",
  "ext.connect.pairing.body": "Controlla che Mosaic mostri lo stesso codice, poi fai clic su Consenti lì.",
  "ext.connect.pairing.code": "Codice",
  "ext.connect.pairing.waiting": "In attesa della tua risposta nell'app Mosaic…",
  "ext.connect.protocolMismatch.title": "Aggiorna Mosaic",
  "ext.connect.protocolMismatch.body":
    "L'estensione e l'app Mosaic hanno versioni diverse. Aggiorna entrambe all'ultima versione.",
  "ext.connect.error.title": "Impossibile raggiungere Mosaic",
  "ext.connect.error.body": "Controlla che l'app Mosaic sia in esecuzione, poi riprova.",
  "ext.locked.title": "Mosaic è bloccato",
  "ext.locked.body": "Sbloccalo nell'app Mosaic per compilare su {host}.",
  "ext.locked.bodyNoSite": "Sbloccalo nell'app Mosaic per usarlo qui.",
  "ext.locked.open": "Apri Mosaic",
  "ext.locked.note": "La tua password principale si digita sempre e solo nell'app Mosaic.",
  "ext.logins.forSite": "Per {host}",
  "ext.logins.count": "{count, plural, one {# accesso} other {# accessi}}",
  "ext.logins.sameSite": "stesso sito",
  "ext.logins.none": "Nessun accesso salvato per {host}.",
  "ext.logins.noneHint":
    "Mosaic confronta l'indirizzo completo, quindi i siti che sembrano simili non trovano mai corrispondenza.",
  "ext.logins.unsupported":
    "Mosaic non può compilare in questa pagina. Cerca nella tua cassaforte per copiare un accesso.",
  "ext.logins.shortcut": "{shortcut} compila la prima corrispondenza.",
  "ext.logins.frameNote":
    "Mosaic compila solo su {host}, mai dentro frame incorporati di altri siti.",
  "ext.item.untitled": "Senza titolo",
  "ext.item.noUsername": "Nessun nome utente",
  "ext.item.twoFactor": "2FA",
  "ext.item.fill": "Compila",
  "ext.item.fillLabel": "Compila {name}",
  "ext.item.copyMenu": "Opzioni di copia per {name}",
  "ext.item.copyUsername": "Copia nome utente",
  "ext.item.copyPassword": "Copia password",
  "ext.item.copyCode": "Copia codice",
  "ext.item.blocked": "Bloccato: questa pagina non è cifrata e questo accesso non lo consente.",
  "ext.search.label": "Altri elementi",
  "ext.search.placeholder": "Cerca nella tua cassaforte",
  "ext.search.note":
    "I risultati restano in questo popup. La pagina riceve sempre e solo l'accesso che compili.",
  "ext.search.none": "Nessun risultato.",
  "ext.search.copy": "Copia",
  "ext.search.copyOnly": "Solo copia: non salvato per questo sito",
  "ext.shortcut.copyUser": "{platform, select, mac {⌘C} other {Ctrl+C}}",
  "ext.shortcut.copyPassword": "{platform, select, mac {⇧⌘C} other {Ctrl+Maiusc+C}}",
  "ext.shortcut.copyCode": "{platform, select, mac {⌥⌘C} other {Ctrl+Alt+C}}",
  "ext.search.results": "{count, plural, one {# risultato} other {# risultati}}",
  "ext.frame.cross":
    "Il campo in primo piano è in un frame di un altro sito. Mosaic non compila lì.",
  "ext.frameConfirm.title": "Compilare dentro un frame incorporato?",
  "ext.frameConfirm.body":
    "Il campo è in un frame su {host}, non nella pagina stessa. Compila qui solo se ti fidi di questo frame.",
  "ext.frameConfirm.confirm": "Compila nel frame",
  "ext.warning.idn.title": "Controlla l'indirizzo",
  "ext.warning.idn.body":
    "Questo indirizzo usa caratteri speciali e può sembrare un altro sito. Controlla la barra degli indirizzi prima di accedere.",
  "ext.warning.lookalike.body":
    "Questo sito non è {saved}. Gli somiglia, un trucco di phishing comune. Chiudi questa scheda, oppure digita tu {saved}.",
  "ext.warning.youAreOn": "Ti trovi su",
  "ext.warning.writtenAs": "Scritto come",
  "ext.warning.savedFor": "Salvato per",
  "ext.warning.insecure.title": "Questa pagina non è cifrata",
  "ext.warning.insecure.body":
    "La pagina usa http. Altri nella rete potrebbero leggere la tua password. Mosaic compila qui solo gli accessi per cui l'hai consentito.",
  "ext.otp.title": "Compila il tuo codice monouso",
  "ext.otp.next": "Password compilata per {name}. Ora: il codice della verifica in due passaggi.",
  "ext.otp.for": "Codice per {name}",
  "ext.otp.fill": "Compila codice",
  "ext.otp.copy": "Copia codice",
  "ext.otp.note":
    "Il codice va da Mosaic direttamente nel campo. Questo popup non lo mostra mai.",
  "ext.save.titleNew": "Salvare questo accesso per {host}?",
  "ext.save.titleUpdate": "Aggiornare l'accesso salvato per {host}?",
  "ext.save.generated": "Una password generata da Mosaic per {host}.",
  "ext.save.updates": "Aggiorna “{title}”.",
  "ext.save.username": "Nome utente",
  "ext.save.noUsername": "Nessuno",
  "ext.save.password": "Password",
  "ext.save.passwordHidden": "Nascosta. La vedi nell'app Mosaic.",
  "ext.save.insecure": "Questa pagina non è cifrata (http).",
  "ext.save.action": "Salva in Mosaic…",
  "ext.save.notNow": "Non ora",
  "ext.save.never": "Mai per questo sito",
  "ext.save.waiting": "Conferma nell'app Mosaic. Non viene salvato nulla finché non lo fai.",
  "ext.save.saved": "Salvato in Mosaic.",
  "ext.save.updated": "Aggiornato in Mosaic.",
  "ext.save.cancelled": "Non salvato.",
  "ext.save.dismissed": "Scartata. La password viene dimenticata.",
  "ext.save.neverDone": "Mosaic non proporrà più di salvare su {host}.",
  "ext.save.permissionAsk":
    "Mosaic chiede al tuo browser l'accesso ai siti web. Consentilo per attivare questa funzione.",
  "ext.generator.title": "Genera una password per {host}",
  "ext.generator.titleGeneric": "Genera una password",
  "ext.generator.value": "Password generata",
  "ext.generator.regenerate": "Genera un'altra password",
  "ext.generator.type": "Tipo di password",
  "ext.generator.random": "Casuale",
  "ext.generator.passphrase": "Frase d'accesso",
  "ext.generator.include": "Includi",
  "ext.generator.rules.page": "Segue le regole di password di questa pagina",
  "ext.generator.rules.siteQuirks": "Segue le regole note per questo sito",
  "ext.generator.fill": "Compila",
  "ext.generator.fillHint":
    "Per compilare, fai clic in un campo di nuova password nella pagina, poi riapri questo popup.",
  "ext.generator.copy": "Copia",
  "ext.generator.expired":
    "Quella password non era più conservata, quindi ne è stata generata una nuova.",
  "ext.generator.filled": "Compilata. Mosaic proporrà di salvarla.",
  "ext.copied": "Copiato. Si cancella tra {seconds, plural, one {# secondo} other {# secondi}}.",
  "ext.copiedNoClear": "Copiato.",
  "ext.filled": "Compilato.",
  "ext.filledCode": "Codice compilato.",
  "ext.error.invalidRequest":
    "Mosaic non ha capito quella richiesta. Aggiorna l'estensione e l'app Mosaic.",
  "ext.error.notPaired": "Questo browser non è collegato a Mosaic. Collegalo di nuovo.",
  "ext.error.protocolVersion":
    "L'estensione e l'app Mosaic hanno versioni diverse. Aggiorna entrambe.",
  "ext.error.busy": "Mosaic è occupato con un'altra richiesta. Riprova tra un momento.",
  "ext.error.locked": "Mosaic è bloccato. Sbloccalo nell'app Mosaic.",
  "ext.error.rateLimited":
    "Troppe richieste in poco tempo. Attendi un momento, poi riprova.",
  "ext.error.originMismatch":
    "Questo accesso non è salvato per questo sito web, quindi Mosaic non lo compila qui.",
  "ext.error.insecureOrigin":
    "Questa pagina non è cifrata e l'accesso non consente la compilazione al suo interno. Non è stato compilato nulla.",
  "ext.error.crossOriginFrame":
    "Quel campo è in un frame di un altro sito. Mosaic non compila lì.",
  "ext.error.itemNotFound":
    "Quell'elemento non esiste più. Potrebbe essere stato spostato o eliminato.",
  "ext.error.vaultUnavailable": "Quella cassaforte è chiusa. Aprila prima nell'app Mosaic.",
  "ext.error.noOtp": "Questo elemento non ha un codice monouso.",
  "ext.error.otpNotFillable":
    "Questo codice aumenta a ogni uso (HOTP), quindi da qui non si può compilare né copiare. Usalo nell'app Mosaic.",
  "ext.error.fieldEmpty": "Quel campo è vuoto, quindi non c'è nulla da compilare o copiare.",
  "ext.error.clipboardUnavailable":
    "Mosaic non è riuscito a usare gli appunti. Riprova, oppure copialo nell'app Mosaic.",
  "ext.error.cancelled": "Annullato nell'app Mosaic. Riprova quando sei pronto.",
  "ext.error.passkeyNotFound": "Mosaic non ha una passkey per questo sito.",
  "ext.error.passkeyExcluded": "Hai già una passkey per questo account in Mosaic.",
  "ext.error.rpIdMismatch":
    "Questo sito non può usare una passkey per quel dominio, quindi Mosaic non ha risposto.",
  "ext.error.passkeyUnsupported":
    "Mosaic non può creare una passkey con le opzioni richieste da questo sito.",
  "ext.error.pairingDenied":
    "Il collegamento è stato negato nell'app Mosaic. Scegli Collega per riprovare, poi scegli Consenti.",
  "ext.error.pairingExpired":
    "La richiesta di collegamento è scaduta. Scegli Collega per ricominciare.",
  "ext.error.pairingConflict":
    "Un'altra richiesta di collegamento era aperta. Scegli Collega per riprovare.",
  "ext.error.pairingBlocked":
    "Troppi collegamenti sono stati negati. Attendi dieci minuti, poi riprova.",
  "ext.error.pairingLimit":
    "Questo computer ha troppi browser collegati. Scollegane uno nelle impostazioni di Mosaic.",
  "ext.error.internal":
    "Mosaic ha riscontrato un problema imprevisto. Riprova e, se si ripete, riavvia l'app Mosaic.",
  "ext.error.desktopNotRunning":
    "Mosaic non è in esecuzione su questo computer. Aprilo, poi riprova.",
  "ext.error.desktopNotAuthentic":
    "L'app che ha risposto non è l'app Mosaic a cui questo browser è collegato. Non è stato inviato nulla.",
  "ext.error.timeout":
    "Mosaic non ha risposto in tempo. Se nell'app Mosaic è aperta una finestra di dialogo, rispondi lì.",
  "ext.error.frameTooLarge": "Erano troppi dati da inviare a Mosaic.",
  "ext.error.hostMissing":
    "L'assistente di Mosaic non è installato per questo browser. Installa l'app Mosaic, poi riavvia il browser.",
  "ext.error.hostForbidden":
    "Questo browser o questa estensione non può comunicare con Mosaic.",
  "ext.error.hostExited": "La connessione con Mosaic si è chiusa. Riprova.",
  "ext.error.unsupportedPage": "Mosaic non può compilare in questo tipo di pagina.",
  "ext.error.noFields":
    "Qui non sono stati trovati campi di accesso. Fai clic in un campo, poi riprova.",
  "ext.error.noMatch": "Mosaic non ha un accesso salvato per questo sito.",
  "ext.error.frameNeedsConfirmation":
    "Il campo è dentro un frame. Apri il popup di Mosaic e conferma il frame per compilarlo.",
  "ext.error.frameOpaque": "Il campo è in un frame isolato (sandbox). Mosaic non compila lì.",
  "ext.error.originChanged":
    "La pagina è cambiata mentre Mosaic compilava. Non è stato compilato nulla. Riprova.",
  "ext.error.pageHidden": "La pagina non è visibile, quindi Mosaic non ha compilato.",
  "ext.error.permissionDenied":
    "Mosaic ha bisogno dell'accesso ai siti web per accorgersi degli accessi e non l'hai consentito.",
  "ext.error.generatedExpired": "Mosaic non conserva più quella password. Generane una nuova.",
  "ext.rotation.card.title": "Cambio password",
  "ext.rotation.card.for": "Per {name}",
  "ext.rotation.card.started": "Iniziato {when}",
  "ext.rotation.card.delivered": "La nuova password è già stata compilata una volta.",
  "ext.rotation.card.note":
    "La nuova password viene mostrata solo nell'app Mosaic. Mosaic non invia mai il modulo al posto tuo.",
  "ext.rotation.card.list": "Cambi di password in corso",
  "ext.rotation.change": "Cambia password…",
  "ext.rotation.changeLabel": "Cambia password… per {name}",
  "ext.rotation.openPage": "Apri la pagina del cambio",
  "ext.rotation.fillChange": "Compila modulo di cambio",
  "ext.rotation.fillPending": "Compila la nuova password per accedere",
  "ext.rotation.worked": "Ha funzionato",
  "ext.rotation.failed": "Non ha funzionato",
  "ext.rotation.fresh": "Ricomincia con una nuova password",
  "ext.rotation.waiting": "In attesa di te nell'app Mosaic…",
  "ext.rotation.started": "Avviato. Scegli Usa questa password nell'app Mosaic.",
  "ext.rotation.startCancelled": "Non è stato avviato nulla.",
  "ext.rotation.filled": "Modulo di cambio compilato. Controllalo, poi invialo tu.",
  "ext.rotation.opened": "Pagina del cambio aperta.",
  "ext.rotation.confirmed": "Fatto. La nuova password è ora salvata in Mosaic.",
  "ext.rotation.keptOld": "Mosaic ha mantenuto la vecchia password.",
  "ext.rotation.resultCancelled": "Non è cambiato nulla. Il cambio è ancora aperto.",
  "ext.rotation.offer.title": "Un indirizzo collegato",
  "ext.rotation.offer.body":
    "Questa pagina è su {host}, che non è salvato per {title}. Aggiungerlo perché Mosaic possa compilare lì?",
  "ext.rotation.offer.add": "Aggiungi {host}",
  "ext.rotation.offer.added": "Aggiunto. Il cambio ora viene mostrato qui.",
  "ext.rotation.offer.cancelled": "Non aggiunto.",
  "ext.error.rotationNotFound":
    "Quel cambio di password non è più aperto. Avviane uno nuovo dall'accesso.",
  "ext.error.originNotRelated":
    "Quell'indirizzo non è sullo stesso sito di questo accesso, quindi Mosaic non l'ha aggiunto.",
  "ext.error.rotationLimit":
    "Questo accesso ha già quattro cambi non terminati. Completane o annullane uno prima nell'app Mosaic.",
  "ext.error.changeFormNotFound":
    "Mosaic non ha trovato qui un modulo di cambio password. Apri la pagina della password del tuo account, poi riprova.",
  "ext.error.changeFormAmbiguous":
    "Mosaic non è riuscito a capire quale campo sia quale. Copia la nuova password da Mosaic e incollala tu.",
} satisfies Record<keyof typeof en, string>;
