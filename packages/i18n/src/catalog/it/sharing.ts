/**
 * Italian UI strings: sharing. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { sharing as en } from "../en/sharing.ts";

export const sharing = {
  "sharing.title": "Condivisione",
  "sharing.body":
    "Condividi le casseforti con persone che conosci. Tutto resta cifrato: solo le persone che inviti possono leggere una cassaforte condivisa, e Mosaic condivide solo con persone di cui hai confrontato il codice di sicurezza.",
  "sharing.notHosted.none":
    "La condivisione richiede un server Mosaic. Attiva prima la sincronizzazione con un server Mosaic nelle impostazioni di Sincronizzazione.",
  "sharing.notHosted.folder":
    "La condivisione non è disponibile con una cartella di sincronizzazione. Una cartella non può sapere chi può leggere una cassaforte, quindi Mosaic tiene private le casseforti in una cartella. Per condividere, scollega la cartella e collega un server Mosaic nelle impostazioni di Sincronizzazione.",
  "sharing.notHosted":
    "La condivisione richiede un server Mosaic. Collegane prima uno nelle impostazioni di Sincronizzazione.",
  "sharing.syncSettings": "Apri le impostazioni di sincronizzazione",
  "sharing.ownCode.title": "Il tuo codice di sicurezza",
  "sharing.ownCode.body":
    "Leggi questo codice alla persona con cui condividi, di persona o al telefono. Se coincide con quello che vede per te, nessuno si è messo in mezzo.",
  "sharing.ownCode.pending":
    "Il tuo codice di sicurezza compare dopo la prima sincronizzazione con il server.",
  "sharing.contacts.title": "Persone che hai cercato",
  "sharing.contacts.empty": "Non hai ancora cercato nessuno.",
  "sharing.contacts.verified": "Verificato",
  "sharing.contacts.notVerified": "Non verificato",
  "sharing.contacts.changed": "Serve un nuovo controllo",
  "sharing.contacts.verify": "Confronta il codice…",
  "sharing.contacts.verifyOf": "Confronta il codice con {email}",
  "sharing.contacts.review": "Controlla la modifica…",
  "sharing.contacts.reviewOf": "Controlla la modifica per {email}",
  "sharing.share.open": "Condividi…",
  "sharing.share.title": "Condividi {vault}",
  "sharing.share.dialog": "Condivisione: {vault}",
  "sharing.share.roleStep": "Scegli un ruolo per {email}",
  "sharing.lookup.email": "Il suo indirizzo email",
  "sharing.lookup.find": "Cerca",
  "sharing.lookup.searching": "Ricerca in corso…",
  "sharing.lookup.notFoundHint":
    "Mosaic non può dirti se un indirizzo ha un account. Se il codice qui sopra non coincide con quello che vede la persona, non condividere.",
  "sharing.code.title": "Confronta i codici di sicurezza",
  "sharing.code.compare":
    "Confronta questo codice con {email}, di persona o al telefono. Non usare email o chat.",
  "sharing.code.compareNew":
    "Confronta il nuovo codice con {email}, di persona o al telefono. Non usare email o chat.",
  "sharing.code.shown": "Codice che Mosaic mostra per {email}",
  "sharing.code.typed": "Codice che ti ha letto (facoltativo)",
  "sharing.code.typedMatch": "È il codice qui sopra.",
  "sharing.code.attest":
    "Ho confrontato questo codice con {email} al telefono o di persona ed è lo stesso.",
  "sharing.code.matches": "I codici coincidono",
  "sharing.code.mismatch":
    "I codici sono diversi. Non condividere. Qualcuno potrebbe essersi messo tra te e {email}.",
  "sharing.code.mismatchHelp":
    "Controlla l'indirizzo e chiedi a {email} di leggere di nuovo il codice. Se è ancora diverso, annulla e avvisalo in un altro modo.",
  "sharing.code.verified": "{email} è verificato.",
  "sharing.code.status.new": "Nuovo contatto",
  "sharing.code.status.pinned": "Non ancora verificato",
  "sharing.code.status.verified": "Verificato",
  "sharing.code.status.changed": "Chiave cambiata",
  "sharing.code.status.conflicting": "I dispositivi non concordano",
  "sharing.code.conflicting":
    "I tuoi dispositivi hanno visto chiavi diverse per {email}. Confronta di nuovo il codice con la persona prima di condividere.",
  "sharing.verify.title": "Verifica {email}",
  "sharing.role.label": "Ruolo",
  "sharing.role.owner": "Proprietario",
  "sharing.role.manage": "Gestore",
  "sharing.role.write": "Editor",
  "sharing.role.read": "Lettore",
  "sharing.role.desc.manage": "Può invitare e rimuovere persone e cambiare i ruoli.",
  "sharing.role.desc.write": "Può aggiungere e modificare elementi.",
  "sharing.role.desc.read": "Può solo leggere.",
  "sharing.role.honesty":
    "Chiunque possa leggere un elemento può copiarlo. Un lettore non può modificare la cassaforte, ma niente gli impedisce di conservare ciò che vede, e rimuovere qualcuno in seguito non annulla ciò che ha già scaricato.",
  "sharing.invite.send": "Invia invito",
  "sharing.invite.sent":
    "Invito inviato a {email}. Deve aprire Mosaic e accettarlo.",
  "sharing.invite.sentTitle": "Invito inviato",
  "sharing.invite.pending": "In attesa di {email}",
  "sharing.invite.expires": "Scade {when}",
  "sharing.invite.cancel": "Ritira invito",
  "sharing.invite.cancelOf": "Ritira l'invito per {email}",
  "sharing.invite.cancelled": "Invito ritirato.",
  "sharing.inbox.title": "Inviti",
  "sharing.inbox.nav": "Inviti",
  "sharing.inbox.navCount": "Inviti, {count} in attesa",
  "sharing.inbox.intro":
    "Persone che ti hanno invitato a una cassaforte condivisa. Mosaic controlla ogni invito prima di mostrartelo.",
  "sharing.inbox.empty":
    "Nessun invito al momento. Quando qualcuno ti invita a una cassaforte condivisa, compare qui.",
  "sharing.inbox.from": "Da {email}",
  "sharing.inbox.review": "Controlla",
  "sharing.inbox.reviewOf": "Controlla l'invito di {email}",
  "sharing.inbox.refresh": "Aggiorna",
  "sharing.inbox.preview.title": "Unirti a {vault}?",
  "sharing.inbox.preview.body": "{email} ti invita come {role}.",
  "sharing.inbox.preview.unnamed": "una cassaforte condivisa",
  "sharing.inbox.preview.owner": "La cassaforte appartiene a {email}.",
  "sharing.inbox.verifySender": "Prima di unirti, confronta il codice di sicurezza di {email}.",
  "sharing.inbox.verifyOwner":
    "{sender} non è il proprietario. Prima di unirti, confronta anche il codice di sicurezza del proprietario: è il proprietario che può cambiare la chiave della cassaforte.",
  "sharing.inbox.ownerCode": "Codice del proprietario, {email}",
  "sharing.inbox.accept": "Unisciti alla cassaforte",
  "sharing.inbox.readyTitle": "Tutto controllato. Unirti a questa cassaforte?",
  "sharing.inbox.decline": "Rifiuta",
  "sharing.inbox.block": "Rifiuta e blocca il mittente",
  "sharing.inbox.block.title": "Rifiutare e bloccare questo mittente?",
  "sharing.inbox.block.body":
    "Mosaic rifiuta l'invito e d'ora in poi ignora ogni invito da {email}. Non puoi annullarlo in Mosaic.",
  "sharing.inbox.unreadable":
    "Questo invito non può essere aperto. Potrebbe essere danneggiato o non destinato a te. Chiedi a {email} di inviarlo di nuovo.",
  "sharing.inbox.joined":
    "Ti sei unito alla cassaforte. Tra un momento comparirà nel tuo elenco di casseforti.",
  "sharing.inbox.declined": "Invito rifiutato.",
  "sharing.inbox.outgoing": "Inviti che hai inviato",
  "sharing.inbox.outgoing.empty": "Nessuno dei tuoi inviti è in attesa di risposta.",
  "sharing.inbox.unknownVault": "una cassaforte",
  "sharing.members.title": "Persone con accesso",
  "sharing.members.you": "Tu",
  "sharing.members.unverified": "Non verificato",
  "sharing.members.unconfirmed": "Non nell'elenco dei membri",
  "sharing.members.unconfirmed.help":
    "Il server elenca questa persona, ma l'elenco dei membri della cassaforte no. Il prossimo cambio di chiave la esclude. Se non la conosci, rimuovila.",
  "sharing.members.dropped": "Non più sul server",
  "sharing.members.dropped.help":
    "Questa persona è nell'elenco dei membri della cassaforte, ma il server non la elenca più. Potrebbe essere uscita. Controlla che sia previsto.",
  "sharing.members.invited": "Invitato",
  "sharing.members.removing": "In rimozione",
  "sharing.members.verified": "Verificato",
  "sharing.members.verify": "Verifica…",
  "sharing.members.verifyOf": "Verifica {email}",
  "sharing.members.remove": "Rimuovi",
  "sharing.members.removeOf": "Rimuovi {email}",
  "sharing.members.remove.title": "Rimuovere {email}?",
  "sharing.members.remove.confirm":
    "{email} conserva ciò che ha già scaricato. Mosaic cambia ora la chiave della cassaforte e la persona non può leggere nulla di nuovo.",
  "sharing.members.remove.confirmManager":
    "{email} conserva ciò che ha già scaricato. Il server smette ora di inviarle aggiornamenti. Il dispositivo del proprietario della cassaforte cambia la chiave della cassaforte alla prossima sincronizzazione e, fino ad allora, solo il server tiene fuori {email}.",
  "sharing.members.cancel": "Annulla",
  "sharing.members.removed": "Rimosso.",
  "sharing.members.leave": "Lascia la cassaforte",
  "sharing.members.leave.title": "Lasciare questa cassaforte?",
  "sharing.members.leave.confirm":
    "La copia su questo dispositivo resta leggibile, ma smette di aggiornarsi. Per unirti di nuovo serve un nuovo invito.",
  "sharing.members.left": "Hai lasciato la cassaforte.",
  "sharing.members.roleOf": "Ruolo di {email}",
  "sharing.members.changeRole": "Cambia ruolo",
  "sharing.members.changeRoleOf": "Cambia il ruolo di {email}",
  "sharing.members.invite": "Invita qualcuno…",
  "sharing.members.empty": "Per ora solo tu puoi aprire questa cassaforte.",
  "sharing.members.unknownEmail": "Indirizzo sconosciuto",
  "sharing.members.managerNote":
    "Puoi invitare e rimuovere persone. Solo il dispositivo del proprietario cambia la chiave della cassaforte, quindi una rimozione è completa quando il dispositivo del proprietario si è sincronizzato.",
  "sharing.members.readerNote":
    "Puoi leggere questa cassaforte ma non cambiare chi ha accesso.",
  "sharing.rotation.running":
    "Una rimozione è in attesa che il dispositivo del proprietario cambi la chiave della cassaforte. Finché non si sincronizza, solo il server tiene fuori la persona rimossa.",
  "sharing.rotation.unprotected":
    "{count, plural, one {# file mantiene la vecchia protezione perché questo dispositivo non lo ha. Una persona che hai rimosso può ancora aprirlo se ha conservato la vecchia chiave. Aggiungilo di nuovo da un dispositivo che lo ha.} other {# file mantengono la vecchia protezione perché questo dispositivo non li ha. Una persona che hai rimosso può ancora aprirli se ha conservato la vecchia chiave. Aggiungili di nuovo da un dispositivo che li ha.}}",
  "sharing.keyChanged.title": "Il codice di sicurezza di {email} è cambiato",
  "sharing.keyChanged.body":
    "Il server ora mostra una chiave diversa per {email}. Succede quando la persona configura di nuovo Mosaic, ed è anche l'aspetto che avrebbe un attacco al server. Con la nuova chiave non viene condiviso nulla finché non hai confrontato i codici con la persona, di persona o al telefono.",
  "sharing.keyChanged.old": "Codice che hai confrontato prima",
  "sharing.keyChanged.new": "Nuovo codice",
  "sharing.keyChanged.accept": "Il nuovo codice coincide",
  "sharing.finding.region": "Avvisi di condivisione",
  "sharing.finding.summary":
    "{count, plural, one {# avviso di condivisione} other {# avvisi di condivisione}}",
  "sharing.finding.review": "Controlla",
  "sharing.finding.collapse": "Comprimi",
  "sharing.finding.dismiss": "Ignora",
  "sharing.finding.unknownEmail": "una persona sconosciuta",
  "sharing.finding.verify": "Verifica…",
  "sharing.finding.remove": "Rimuovi",
  "sharing.finding.removeOf": "Rimuovi {email} da {vault}",
  "sharing.finding.identityMismatch":
    "Il server mostra per il tuo account una chiave di condivisione diversa da quella creata da questo dispositivo, quindi la condivisione è in pausa. Non condividere nulla e controlla il tuo server e gli altri dispositivi. Poi scegli “Ho controllato…”.",
  "sharing.finding.rosterInvalid":
    "L'elenco dei membri di {vault} è danneggiato, oppure qualcuno lo ha modificato senza autorizzazione. Non aggiungere persone finché non hai controllato chi ha accesso. Poi scegli “Ho controllato…”.",
  "sharing.finding.shareUntrusted":
    "Una nuova chiave per {vault} è stata rifiutata: l'ha inviata {sender}, ma non può cambiare la chiave di questa cassaforte. La cassaforte resta sulla vecchia chiave. Chiedi al proprietario se l'ha cambiata. Poi scegli “Ho controllato…”.",
  "sharing.finding.memberUnverified":
    "Non hai verificato {email} in {vault}. Mosaic non le invia nuove chiavi e il prossimo cambio di chiave la esclude. Confronta il suo codice di sicurezza, oppure rimuovila.",
  "sharing.finding.memberDropped":
    "{email} è nell'elenco dei membri di {vault}, ma il server non la elenca più. Potrebbe essere uscita, oppure l'ha rimossa il server. Controlla che sia previsto e rimuovila se non lo è.",
  "sharing.finding.memberKeyMismatch":
    "Mosaic ha ricevuto per {email} una chiave che non coincide con quella che hai verificato, quindi è stata rimossa da {vault}. Confronta di nuovo i codici di sicurezza con la persona prima di invitarla di nuovo.",
  "sharing.finding.vaultHeld":
    "{vault} è in attesa che i tuoi altri dispositivi confermino chi ne è il proprietario. Apri Mosaic su un dispositivo che usa già questa cassaforte.",
  "sharing.finding.accessLost":
    "Non hai più accesso a {vault}. La copia su questo dispositivo resta leggibile ma non si aggiorna. Chiedi al proprietario di invitarti di nuovo.",
  "sharing.finding.rotationBlocked":
    "La chiave di {vault} non ha potuto essere cambiata dopo che hai rimosso qualcuno, quindi la persona ha ancora la vecchia chiave. Sincronizza di nuovo. Se persiste, apri l'elenco dei membri e controlla chi ha accesso.",
  "emergency.title": "Accesso di emergenza",
  "emergency.body":
    "Indica le persone che possono chiedere l'accesso alle tue casseforti se ti succede qualcosa. Non è un recupero dell'account: ottengono accesso di sola lettura alle casseforti che scegli, solo dopo un tempo di attesa, e in quel periodo puoi rifiutare. Non sostituisce il tuo Kit di emergenza.",
  "emergency.folderUnsupported":
    "L'accesso di emergenza richiede il server Mosaic. Una cartella di sincronizzazione non può far rispettare un tempo di attesa.",
  "emergency.granted.title": "Persone che possono richiedere l'accesso",
  "emergency.granted.empty": "Nessuno può chiedere l'accesso alle tue casseforti.",
  "emergency.granted.add": "Aggiungi una persona…",
  "emergency.granted.scopeAll": "Tutte le tue casseforti",
  "emergency.granted.scopeSome": "{count, plural, one {# cassaforte} other {# casseforti}}",
  "emergency.granted.wait": "Attesa di {days, plural, one {# giorno} other {# giorni}}",
  "emergency.granted.waitingUntil":
    "Ha chiesto l'accesso. Si apre {when}, a meno che tu non rifiuti.",
  "emergency.granted.approved": "Hai approvato la richiesta.",
  "emergency.granted.released": "L'accesso è stato concesso. Può leggere le casseforti.",
  "emergency.granted.idle": "Nessuna richiesta",
  "emergency.granted.remove": "Rimuovi accesso",
  "emergency.granted.removeOf": "Rimuovi l'accesso di {email}",
  "emergency.granted.remove.title": "Rimuovere l'accesso di emergenza di {email}?",
  "emergency.granted.remove.body":
    "Non potrà più chiedere. Se il suo accesso era già stato concesso, Mosaic cambia le chiavi delle casseforti che ha aperto; conserva ciò che ha già scaricato.",
  "emergency.granted.removed": "Accesso di emergenza rimosso.",
  "emergency.grant.title": "Consenti l'accesso di emergenza",
  "emergency.grant.lookup": "Chi dovrebbe poter chiedere?",
  "emergency.grant.optionsTitle": "Cosa può aprire {email}",
  "emergency.grant.scope": "Cosa può aprire",
  "emergency.grant.scope.all": "Tutte le mie casseforti, comprese quelle nuove",
  "emergency.grant.scope.selected": "Solo le casseforti che scelgo",
  "emergency.grant.scope.pick": "Scegli almeno una cassaforte.",
  "emergency.grant.wait": "Tempo di attesa",
  "emergency.grant.wait.option": "{days, plural, one {# giorno} other {# giorni}}",
  "emergency.grant.waitHint":
    "Dopo una richiesta, apri Mosaic almeno una volta entro questo tempo, così puoi vederla e rifiutare. Allo scadere del tempo, la persona ottiene accesso di sola lettura senza che ti venga chiesto di nuovo.",
  "emergency.grant.serverTrust":
    "Il tempo di attesa è gestito dal server. Se la persona che indichi e chi gestisce il server si accordano, potrebbero saltarlo. Il server da solo non può leggere le tue casseforti.",
  "emergency.grant.vaultKeysOnly":
    "Questo riguarda solo le chiavi delle tue casseforti. Non dà accesso al tuo account, alla tua password principale o alla tua Chiave segreta.",
  "emergency.grant.submit": "Consenti l'accesso di emergenza",
  "emergency.grant.done": "{email} ora può chiedere l'accesso di emergenza.",
  "emergency.grant.needVerify": "Confronta prima il codice di sicurezza di {email}.",
  "emergency.received.title": "Persone che ti hanno indicato",
  "emergency.received.empty": "Nessuno ti ha indicato per l'accesso di emergenza.",
  "emergency.received.request": "Richiedi l'accesso",
  "emergency.received.requestOf": "Richiedi l'accesso a {email}",
  "emergency.received.request.title": "Chiedere l'accesso a {email}?",
  "emergency.received.request.body":
    "{email} vede la tua richiesta alla prossima apertura di Mosaic. Dopo {days, plural, one {# giorno} other {# giorni}} puoi aprire le sue casseforti, in sola lettura, a meno che non rifiuti.",
  "emergency.received.requested":
    "Richiesta inviata. {email} la vede alla prossima apertura di Mosaic.",
  "emergency.received.requestHint":
    "{email} vede la tua richiesta in Mosaic. Dopo {days, plural, one {# giorno} other {# giorni}} puoi aprire le sue casseforti, in sola lettura, a meno che non rifiuti.",
  "emergency.received.countdown": "Potrai aprire le sue casseforti {when}.",
  "emergency.received.waitingLive": "La tua richiesta è in attesa.",
  "emergency.received.claim": "Apri le sue casseforti",
  "emergency.received.claimOf": "Apri le sue casseforti: {email}",
  "emergency.received.claimed":
    "{count, plural, one {# cassaforte è stata aggiunta} other {# casseforti sono state aggiunte}}, in sola lettura.",
  "emergency.received.cooldown": "{email} ha rifiutato. Potrai chiedere di nuovo {when}.",
  "emergency.received.verifyFirst":
    "Confronta il codice di sicurezza di {email} prima di aprire le sue casseforti. Mosaic usa solo una chiave che hai verificato.",
  "emergency.received.verify": "Confronta il codice…",
  "emergency.received.verifyOf": "Confronta il codice con {email}",
  "emergency.received.approved": "{email} ha approvato la tua richiesta.",
  "emergency.received.waitDays": "Attesa di {days, plural, one {# giorno} other {# giorni}}",
  "emergency.banner.region": "Richiesta di accesso di emergenza",
  "emergency.banner.body":
    "{email} ha chiesto l'accesso di emergenza alle tue casseforti. Si apre {when}, a meno che tu non rifiuti.",
  "emergency.banner.refuse": "Rifiuta",
  "emergency.banner.approve": "Approva ora…",
  "emergency.banner.approve.title": "Approvare subito l'accesso di emergenza?",
  "emergency.banner.approve.body":
    "{email} ottiene subito accesso di sola lettura alle tue casseforti. Inserisci la password principale per confermare.",
  "emergency.banner.approve.confirm": "Approva",
  "emergency.banner.refused": "Richiesta rifiutata.",
  "emergency.banner.approved": "Accesso approvato.",
} satisfies Record<keyof typeof en, string>;
