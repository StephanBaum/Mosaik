/**
 * Italian UI strings: rotation. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { rotation as en } from "../en/rotation.ts";

export const rotation = {
  "rotation.section.title": "Password e chiavi",
  "rotation.section.body":
    "Cambia la password principale, ottieni una nuova Chiave segreta o sposta tutto il tuo account su nuove chiavi. Ogni modifica ti mostra cosa succede prima di toccare qualcosa.",
  "rotation.action.password": "Cambia la password principale",
  "rotation.action.password.body":
    "Scegli una nuova password principale. La tua Chiave segreta e le tue casseforti restano come sono.",
  "rotation.action.password.button": "Cambia…",
  "rotation.action.secretKey": "Rinnova la Chiave segreta",
  "rotation.action.secretKey.body":
    "Ottieni una nuova Chiave segreta e un nuovo Kit di emergenza. Usalo se il kit è andato perso o qualcuno potrebbe averlo visto.",
  "rotation.action.secretKey.button": "Rinnova…",
  "rotation.action.profile": "Potenzia la protezione",
  "rotation.action.profile.body":
    "Rende più difficile indovinare la tua password principale usando più memoria a ogni tentativo. Lo sblocco richiede un po' più di tempo.",
  "rotation.action.profile.button": "Potenzia…",
  "rotation.action.rekey": "Rinnova le chiavi dell'account",
  "rotation.action.rekey.body":
    "Rinnova ogni chiave del tuo account: una nuova Chiave segreta, nuove chiavi delle casseforti e un nuovo codice di sicurezza. Usalo se qualcuno potrebbe aver copiato i dati della tua cassaforte insieme alla tua password e alla Chiave segreta.",
  "rotation.action.rekey.button": "Rinnova le chiavi…",
  "rotation.action.rekey.needsSync":
    "Il rinnovo delle chiavi aggiorna la copia sul tuo server o sulla tua cartella di sincronizzazione, quindi serve la sincronizzazione. Configurala prima.",
  "rotation.title.password": "Cambia la password principale",
  "rotation.title.secretKey": "Rinnova la Chiave segreta",
  "rotation.title.profile": "Potenzia la protezione",
  "rotation.title.rekey": "Rinnova le chiavi dell'account",
  "rotation.step": "Passaggio {current} di {total}",
  "rotation.back": "Indietro",
  "rotation.stop.title": "Interrompere questa modifica?",
  "rotation.stop.body":
    "Il Kit di emergenza che hai appena salvato non funzionerà. Conserva il tuo vecchio kit.",
  "rotation.stop.confirm": "Interrompi la modifica",
  "rotation.stop.keep": "Continua",
  "rotation.next": "Continua",
  "rotation.close": "Chiudi",
  "rotation.review.title.password": "Prima di cambiare la password",
  "rotation.review.title.secretKey": "Prima di rinnovare la Chiave segreta",
  "rotation.review.title.profile": "Prima di potenziare la protezione",
  "rotation.review.title.rekey": "Qualcosa è stato compromesso?",
  "rotation.review.changes": "Cosa cambia",
  "rotation.review.stays": "Cosa resta uguale",
  "rotation.review.password.changes":
    "La tua password principale. Lo sblocco rapido su questo dispositivo viene disattivato e potrai configurarlo di nuovo dopo.",
  "rotation.review.password.stays":
    "La tua Chiave segreta, le tue casseforti e i tuoi elementi, il tuo codice di sicurezza e i tuoi contatti di emergenza.",
  "rotation.review.password.devices":
    "I tuoi altri dispositivi vengono disconnessi dalla sincronizzazione. Su ciascuno digita una volta la nuova password principale e la sincronizzazione riprende.",
  "rotation.review.secretKey.changes":
    "La tua Chiave segreta e il tuo Kit di emergenza. Lo sblocco rapido su questo dispositivo viene disattivato e potrai configurarlo di nuovo dopo.",
  "rotation.review.secretKey.stays":
    "La tua password principale, le tue casseforti e i tuoi elementi, il tuo codice di sicurezza e i tuoi contatti di emergenza. Le chiavi di sicurezza che hai registrato continuano a funzionare.",
  "rotation.review.secretKey.devices":
    "I tuoi altri dispositivi vengono disconnessi dalla sincronizzazione. Su ciascuno digita una volta la password e la nuova Chiave segreta del nuovo kit.",
  "rotation.review.secretKey.limit":
    "Questo impedisce l'accesso con la vecchia Chiave segreta. Chi ha già copiato i dati della tua cassaforte con la vecchia password e la vecchia Chiave segreta può ancora leggerli. Per impedirlo, rinnova le chiavi dell'account.",
  "rotation.review.secretKey.compromised": "Qualcosa è stato compromesso: rinnova invece le chiavi",
  "rotation.review.profile.changes":
    "Quanta memoria richiede ogni tentativo di sblocco, per te e per chiunque cerchi di indovinare. Lo sblocco rapido su questo dispositivo viene disattivato e potrai configurarlo di nuovo dopo.",
  "rotation.review.profile.stays":
    "La tua password principale, la tua Chiave segreta, le tue casseforti e il tuo codice di sicurezza. Le chiavi di sicurezza che hai registrato continuano a funzionare.",
  "rotation.review.profile.devices":
    "I tuoi altri dispositivi vengono disconnessi dalla sincronizzazione. Su ciascuno digita una volta la password principale e la sincronizzazione riprende. I dispositivi più lenti impiegano un po' più di tempo a sbloccare.",
  "rotation.review.rekey.intro":
    "Il rinnovo delle chiavi serve nel caso in cui qualcuno possa avere i dati della tua cassaforte insieme alla tua password e alla Chiave segreta. Se hai solo perso un dispositivo, basta rinnovare la Chiave segreta o cambiare la password.",
  "rotation.review.rekey.yes": "Sì, rinnova le chiavi del mio account",
  "rotation.review.rekey.no": "No, rinnova solo la Chiave segreta",
  "rotation.review.rekey.limit":
    "Il rinnovo delle chiavi non annulla ciò che qualcuno ha già copiato. Le vecchie copie su server e nei backup restano cifrate con le vecchie chiavi, e le vecchie chiavi le aprono ancora.",
  "rotation.review.rekey.title": "Cosa fa il rinnovo delle chiavi",
  "rotation.review.rekey.vaults":
    "{count, plural, one {# cassaforte che usi solo tu riceve una nuova chiave.} other {# casseforti che usi solo tu ricevono nuove chiavi.}}",
  "rotation.review.rekey.keys":
    "Una nuova Chiave segreta, un nuovo Kit di emergenza, un nuovo codice di sicurezza e una nuova chiave dell'account.",
  "rotation.review.rekey.devices":
    "I tuoi altri dispositivi vengono disconnessi e non possono sincronizzare finché non digiti su ciascuno la nuova Chiave segreta e la nuova password principale.",
  "rotation.review.rekey.shared":
    "{count, plural, one {Condividi # cassaforte.} other {Condividi # casseforti.}} Ognuna riceve una nuova chiave.",
  "rotation.review.rekey.sharedRow":
    "{name}, {count, plural, one {# membro} other {# membri}}",
  "rotation.review.rekey.sharedMembers":
    "Chiedi a ogni persona con cui condividi di confrontare il tuo nuovo codice di sicurezza. Finché non lo fa, non riceve nuove chiavi da te e non può aprire la cassaforte rinnovata.",
  "rotation.review.rekey.memberVaults":
    "{count, plural, one {# cassaforte appartiene a un'altra persona.} other {# casseforti appartengono ad altre persone.}} Ai rispettivi proprietari viene chiesto di rinnovarle. Finché non lo fanno, le tue vecchie chiavi possono ancora leggerle.",
  "rotation.review.rekey.grants":
    "{count, plural, one {# persona ha accesso di emergenza alle tue casseforti.} other {# persone hanno accesso di emergenza alle tue casseforti.}} Il loro accesso viene rinnovato per le nuove chiavi. Ciascuna deve confrontare anche il tuo nuovo codice di sicurezza.",
  "rotation.review.rekey.quick":
    "Lo sblocco rapido su questo dispositivo viene disattivato. Potrai configurarlo di nuovo dopo.",
  "rotation.review.rekey.hardwareKeys":
    "{count, plural, one {# chiave di sicurezza che hai registrato per lo sblocco viene disattivata.} other {# chiavi di sicurezza che hai registrato per lo sblocco vengono disattivate.}} Potrai registrarle di nuovo dopo.",
  "rotation.review.rekey.hardwareVaults":
    "{count, plural, one {# cassaforte protetta dall'hardware non è interessata.} other {# casseforti protette dall'hardware non sono interessate.}}",
  "rotation.review.rekey.attachments":
    "{count, plural, one {# allegato non è su questo dispositivo.} other {# allegati non sono su questo dispositivo.}} Vengono scaricati prima, così possono essere protetti di nuovo. Quelli che non si riescono a raggiungere mantengono la vecchia chiave.",
  "rotation.review.rekey.folder":
    "La tua cartella di sincronizzazione riceve un nuovo file dell'account. Gli altri dispositivi vi trovano una nota che le chiavi sono cambiate.",
  "rotation.review.rekey.unavailable":
    "Il rinnovo delle chiavi non è disponibile: la sincronizzazione non è configurata.",
  "rotation.review.loading": "Controllo di cosa tocca il rinnovo delle chiavi…",
  "rotation.review.rekey.needsClean":
    "Il rinnovo delle chiavi richiede che la sincronizzazione sia aggiornata e senza avvisi. Sincronizza ora, controlla gli eventuali avvisi, poi riprova.",
  "rotation.credentials.title": "Conferma che sei tu",
  "rotation.credentials.password.title": "Le tue password principali",
  "rotation.credentials.current": "Password principale attuale",
  "rotation.credentials.new": "Nuova password principale",
  "rotation.credentials.confirm": "Digita di nuovo la nuova password principale",
  "rotation.credentials.mismatch": "Le due password sono diverse.",
  "rotation.credentials.same":
    "La nuova password principale deve essere diversa da quella attuale.",
  "rotation.credentials.hint":
    "Una frase d'accesso lunga è meglio di una breve e complicata. Non puoi recuperarla: Mosaic non la conosce.",
  "rotation.credentials.code": "Codice di 6 cifre dalla tua app di autenticazione",
  "rotation.credentials.profile": "Livello di protezione",
  "rotation.credentials.profile.standard": "Standard: 256 MB di memoria per tentativo",
  "rotation.credentials.profile.strong": "Forte: 512 MB di memoria per tentativo",
  "rotation.credentials.profile.max": "Massimo: 1 GB di memoria per tentativo",
  "rotation.credentials.profile.slow":
    "Con questo livello lo sblocco richiede a questo dispositivo più di due secondi. Gli altri dispositivi, più vecchi, saranno ancora più lenti.",
  "rotation.credentials.profile.force": "Potenzia comunque",
  "rotation.kit.title": "Salva il tuo nuovo Kit di emergenza",
  "rotation.kit.body":
    "La tua nuova Chiave segreta è in questo kit. Salvalo prima: Mosaic cambia il tuo account solo dopo che hai confermato di averlo, perché senza di esso non puoi accedere su un nuovo dispositivo.",
  "rotation.kit.save": "Salva il nuovo Kit di emergenza",
  "rotation.kit.saved": "Salvato. Conservalo in un posto sicuro, separato dai tuoi dispositivi.",
  "rotation.kit.cancelled": "Non è stato ancora salvato nulla.",
  "rotation.kit.verify":
    "Digita l'ultimo gruppo della tua nuova Chiave segreta dal kit salvato.",
  "rotation.kit.confirm": "Conferma il kit",
  "rotation.kit.mistyped":
    "Questa non è la fine della nuova Chiave segreta. Controlla il kit salvato.",
  "rotation.kit.verified": "Kit confermato.",
  "rotation.confirm.title.password": "Cambia la password principale",
  "rotation.confirm.title.secretKey": "Rinnova la Chiave segreta",
  "rotation.confirm.title.profile": "Potenzia la protezione",
  "rotation.confirm.title.rekey": "Rinnova le chiavi del tuo account",
  "rotation.confirm.body.rekey":
    "Questo disconnette i tuoi altri dispositivi e rinnova ogni chiave. Tieni Mosaic aperto finché non ha finito.",
  "rotation.confirm.body.secretKey":
    "Questo disconnette i tuoi altri dispositivi dalla sincronizzazione. Tieni Mosaic aperto finché non ha finito.",
  "rotation.confirm.verb.password": "Cambia password",
  "rotation.confirm.verb.secretKey": "Rinnova la Chiave segreta",
  "rotation.confirm.verb.profile": "Potenzia la protezione",
  "rotation.confirm.verb.rekey": "Rinnova le chiavi dell'account",
  "rotation.run.title": "Rinnovo delle tue chiavi…",
  "rotation.run.keepOpen":
    "Tieni Mosaic aperto. Con molti allegati può richiedere qualche minuto.",
  "rotation.progress.label": "Avanzamento",
  "rotation.progress.count": "{done} di {total}",
  "rotation.progress.done": "completato",
  "rotation.phase.prepared": "Preparazione",
  "rotation.phase.backendStarted": "Richiesta al tuo account di passare alle nuove chiavi",
  "rotation.phase.backendAccepted": "Il tuo account ha accettato le nuove chiavi",
  "rotation.phase.folderWritten": "La cartella di sincronizzazione ha il nuovo file dell'account",
  "rotation.phase.localCommitted": "Questo dispositivo ora usa le nuove chiavi",
  "rotation.phase.attachments": "Protezione degli allegati di nuovo",
  "rotation.phase.uploading": "Invio dei dati protetti",
  "rotation.phase.done": "Terminato",
  "rotation.done.title.password": "Password principale cambiata",
  "rotation.done.title.secretKey": "Chiave segreta rinnovata",
  "rotation.done.title.profile": "Protezione potenziata",
  "rotation.done.title.rekey": "Chiavi dell'account rinnovate",
  "rotation.done.body.password":
    "D'ora in poi usa la nuova password principale. I tuoi altri dispositivi la chiedono una volta.",
  "rotation.done.body.secretKey":
    "Conserva al sicuro il nuovo Kit di emergenza ed elimina quello vecchio. I tuoi altri dispositivi chiedono la nuova Chiave segreta una volta.",
  "rotation.done.body.profile":
    "Lo sblocco ora richiede più memoria. I tuoi altri dispositivi chiedono la password una volta.",
  "rotation.done.body.rekey":
    "Conserva al sicuro il nuovo Kit di emergenza ed elimina quello vecchio. I tuoi altri dispositivi chiedono una volta la nuova Chiave segreta e la nuova password principale.",
  "rotation.done.members":
    "Chiedi a tutti con cui condividi una cassaforte di confrontare il tuo nuovo codice di sicurezza. Finché non lo fanno, non ricevono nuove chiavi da te.",
  "rotation.done.quick": "Configura di nuovo lo sblocco rapido",
  "rotation.done.quick.body": "Lo sblocco rapido è stato disattivato.",
  "rotation.done.hardware": "Registra di nuovo le chiavi di sicurezza",
  "rotation.done.hardware.body":
    "{count, plural, one {# chiave di sicurezza è stata disattivata per lo sblocco.} other {# chiavi di sicurezza sono state disattivate per lo sblocco.}}",
  "rotation.done.unreachable.note":
    "Questi file non hanno potuto essere letti e non sono stati cifrati di nuovo. Mantengono la vecchia chiave del file finché non li rimuovi o li aggiungi di nuovo, quindi chi ha le tue vecchie chiavi e una copia di un file potrebbe ancora aprirlo.",
  "rotation.done.unreachable.file": "{item}: {file}",
  "rotation.done.unreachable.unknownItem": "Elemento sconosciuto",
  "rotation.done.unreachable.unknownFile": "file sconosciuto",
  "rotation.done.unreachable.more":
    "{count, plural, one {e # altro file} other {e altri # file}}",
  "rotation.done.unfinished":
    "L'ultima parte prosegue più tardi. Se Mosaic si chiude prima della fine, la prossima volta scegli Continua il rinnovo delle chiavi.",
  "rotation.resume.title": "Rinnovo delle chiavi non terminato",
  "rotation.resume.body.password":
    "Un cambio di chiavi è stato interrotto. Non si è perso nulla. Scegli Continua per digitare di nuovo le password, oppure scartalo.",
  "rotation.resume.body.secretKey":
    "Il rinnovo della tua Chiave segreta è stato interrotto. Non si è perso nulla. Continua per completarlo.",
  "rotation.resume.body.rekey":
    "Il rinnovo delle chiavi del tuo account è stato interrotto. Non si è perso nulla. Continua per completarlo.",
  "rotation.resume.body.profile":
    "Il potenziamento della protezione è stato interrotto. Non si è perso nulla. Continua per completarlo.",
  "rotation.resume.continue": "Continua il rinnovo delle chiavi",
  "rotation.resume.discard": "Scarta…",
  "rotation.resume.running": "Operazione in corso…",
  "rotation.resume.progress":
    "{done} di {total} completati. Tieni Mosaic aperto finché non ha finito.",
  "rotation.resume.dialog.title": "Continua il rinnovo delle chiavi",
  "rotation.resume.dialog.password": "Password principale che hai ora",
  "rotation.resume.dialog.group": "Ultimo gruppo della nuova Chiave segreta",
  "rotation.resume.dialog.body":
    "Le chiavi del tuo account potrebbero essere già cambiate. Per essere sicuri che sei tu, digita la password principale e l'ultimo gruppo della nuova Chiave segreta dal nuovo Kit di emergenza.",
  "rotation.resume.dialog.submit": "Continua",
  "rotation.resume.discard.title": "Scartare questo cambio di chiavi?",
  "rotation.resume.discard.body":
    "La nuova Chiave segreta e il nuovo Kit di emergenza di questo tentativo vengono eliminati. Il tuo account resta com'è.",
  "rotation.resume.discard.confirm": "Scarta il cambio di chiavi",
  "rotation.resume.abandon": "Rinuncia al resto…",
  "rotation.resume.stalled":
    "La continuazione non è terminata. Se continua a fallire, puoi fermarti qui.",
  "rotation.abandon.title": "Interrompere la nuova cifratura dei file?",
  "rotation.abandon.body":
    "Le tue nuove chiavi sono già in uso. Mosaic smette di cifrare di nuovo gli allegati ancora in attesa. Mantengono la vecchia chiave del file, quindi chi ha le tue vecchie chiavi e una copia di un file potrebbe ancora aprirlo.",
  "rotation.abandon.stays":
    "Tutto ciò che è già stato cifrato di nuovo resta com'è. Rimuovi o aggiungi di nuovo più tardi i file in attesa per proteggerli con la nuova chiave.",
  "rotation.abandon.confirm": "Interrompi la nuova cifratura",
  "rotation.abandon.keep": "Continua a provare",
  "rotation.marker.title": "Avviso di rinnovo delle chiavi non confermato",
  "rotation.marker.hint":
    "Se non hai rinnovato le chiavi su un altro dispositivo, questo avviso potrebbe non essere autentico. Puoi ignorarlo.",
  "rotation.marker.open": "Ignora questo avviso…",
  "rotation.marker.open.label": "Ignora questo avviso: {name}",
  "rotation.marker.confirm":
    "Mosaic ha trovato un avviso di rinnovo delle chiavi che non ha potuto confermare. Se non hai rinnovato le chiavi del tuo account su un altro dispositivo, puoi ignorarlo.",
  "rotation.marker.ignore": "Ignora l'avviso",
  "rotation.marker.keep": "Conserva l'avviso",
  "rotation.ignored.title": "Avvisi di rinnovo delle chiavi ignorati",
  "rotation.ignored.body":
    "Avvisi che hai scelto di ignorare. Ripristinane uno se hai davvero rinnovato le chiavi del tuo account su un altro dispositivo.",
  "rotation.ignored.row": "Avviso di rinnovo delle chiavi dalla tua cartella di sincronizzazione",
  "rotation.ignored.restore": "Ripristina",
  "rotation.ignored.restore.label": "Ripristina l'avviso {name}",
  "rotation.resume.done": "Rinnovo delle chiavi terminato.",
  "rotation.recovery.title": "Le chiavi del tuo account sono state rinnovate su un altro dispositivo",
  "rotation.recovery.body":
    "Un tuo dispositivo ha rinnovato le chiavi di questo account. Per continuare a sincronizzare, digita la password principale e la Chiave segreta del nuovo Kit di emergenza.",
  "rotation.recovery.password": "Password principale (quella nuova, se l'hai cambiata lì)",
  "rotation.recovery.secretKey": "Chiave segreta dal nuovo Kit di emergenza",
  "rotation.recovery.submit": "Usa le nuove chiavi",
  "rotation.recovery.warning":
    "Non hai rinnovato tu le chiavi? Allora potrebbe averlo fatto qualcuno con la tua vecchia password e la tua vecchia Chiave segreta. Non inserire nulla qui; controlla prima i tuoi altri dispositivi.",
  "rotation.recovery.notice":
    "La sincronizzazione è in pausa finché non lo fai su questo dispositivo.",
  "rotation.recovery.open": "Usa le nuove chiavi…",
  "rotation.recovery.keyHint":
    "Se le tue chiavi sono state rinnovate su un altro dispositivo, inserisci anche la nuova Chiave segreta dal suo Kit di emergenza.",
  "rotation.recovery.done": "Questo dispositivo ora usa le nuove chiavi.",
  "rotation.recovery.optional":
    "Chiave segreta dal Kit di emergenza più recente (solo se l'hai rinnovata)",
  "rotation.file.unknown":
    "Nella tua cartella di sincronizzazione è comparso un file dell'account sconosciuto. Non può essere verificato con le tue chiavi: potrebbe essere un residuo, oppure qualcuno ha modificato la tua cartella. I tuoi dati non sono interessati.",
  "rotation.file.dismiss": "Ignora",
  "rotation.file.conflict.title": "Due dispositivi hanno rinnovato le chiavi nello stesso momento",
  "rotation.file.conflict":
    "Un altro dispositivo ha rinnovato le chiavi del tuo account nello stesso momento di questo, quindi i due non sono più d'accordo. Rinnova di nuovo le chiavi da questo dispositivo per riallinearli: le chiavi di questo dispositivo prevalgono e l'altro chiede la nuova Chiave segreta.",
  "rotation.file.conflict.rekey": "Rinnova di nuovo le chiavi da questo dispositivo",
  "rotation.file.conflict.keep": "Nascondi l'avviso",
  "rotation.resume.body.vault":
    "La nuova cifratura degli allegati di una cassaforte è stata interrotta. Non si è perso nulla. Continua per completarla.",
} satisfies Record<keyof typeof en, string>;
