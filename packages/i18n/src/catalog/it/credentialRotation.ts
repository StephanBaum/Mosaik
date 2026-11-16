/**
 * Italian UI strings: credentialRotation. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { credentialRotation as en } from "../en/credentialRotation.ts";

export const credentialRotation = {
  "credentialRotation.tablist": "Viste di sicurezza",
  "credentialRotation.tab.overview": "Panoramica",
  "credentialRotation.tab.rotate": "Cambia ora",
  "credentialRotation.tab.dueHidden": ", {count} da cambiare",
  "credentialRotation.queue.title": "Cambia ora",
  "credentialRotation.queue.intro":
    "Le password da cambiare per prime. Mosaic ti guida, ma ogni password la cambi tu sul sito.",
  "credentialRotation.queue.empty": "Al momento non c'è nulla da cambiare.",
  "credentialRotation.queue.count":
    "{count, plural, one {# password da cambiare} other {# password da cambiare}}",
  "credentialRotation.queue.list": "Password da cambiare",
  "credentialRotation.queue.snoozedHeading": "Posticipate",
  "credentialRotation.queue.snoozedUntil": "Posticipata fino al {date, date, medium}",
  "credentialRotation.queue.reason.weak": "Debole",
  "credentialRotation.queue.reason.reused": "Riutilizzata",
  "credentialRotation.queue.reason.old": "Vecchia",
  "credentialRotation.queue.reason.breached": "In una violazione",
  "credentialRotation.queue.reason.unfinished": "Cambio non terminato",
  "credentialRotation.queue.change": "Cambia password",
  "credentialRotation.queue.continue": "Continua il cambio",
  "credentialRotation.queue.editItem": "Modifica elemento",
  "credentialRotation.queue.manualHint":
    "Mosaic non può guidarti per questa. Apri l'elemento e cambiala tu.",
  "credentialRotation.queue.actionFor": "{action}: {title}",
  "credentialRotation.queue.snooze": "Posticipa",
  "credentialRotation.queue.snoozeFor": "Posticipa {title}",
  "credentialRotation.queue.snoozeDays": "{days, plural, one {# giorno} other {# giorni}}",
  "credentialRotation.queue.snoozed":
    "Posticipata di {days, plural, one {# giorno} other {# giorni}}.",
  "credentialRotation.queue.dismiss": "Ignora",
  "credentialRotation.queue.dismissFor": "Ignora {title}",
  "credentialRotation.queue.dismissed": "Ignorata. Torna quando la password cambia.",
  "credentialRotation.queue.undo": "Annulla",
  "credentialRotation.panel.title": "Cambia la password di {title}",
  "credentialRotation.panel.step": "Passaggio {current} di {total}",
  "credentialRotation.panel.stays":
    "Il cambio per {title} resta aperto. La nuova password rimane salvata nell'elemento finché non scegli Ha funzionato o Non ha funzionato.",
  "credentialRotation.panel.cancel": "Annulla",
  "credentialRotation.steps.new.title": "Scegli una nuova password",
  "credentialRotation.steps.new.intro":
    "Mosaic salva la nuova password in questo elemento prima che tu la usi altrove, così non si può perdere. La password attuale resta al suo posto finché non confermi.",
  "credentialRotation.steps.new.label": "Nuova password",
  "credentialRotation.steps.new.use": "Usa questa password",
  "credentialRotation.steps.new.saving": "Salvataggio…",
  "credentialRotation.steps.site.title": "Cambiala sul sito",
  "credentialRotation.steps.site.pageLabel": "Pagina del cambio",
  "credentialRotation.steps.site.body":
    "Apri nel browser la pagina delle password del sito, fai clic su Mosaic nella barra degli strumenti e scegli Compila modulo di cambio. Mosaic non invia mai il modulo.",
  "credentialRotation.steps.site.copyLink": "Copia il link alla pagina del cambio",
  "credentialRotation.steps.site.linkCopied": "Link copiato.",
  "credentialRotation.steps.site.linkHint":
    "Il link non è un segreto. Se il sito non ha una pagina a quell'indirizzo, cerca a mano le sue impostazioni della password.",
  "credentialRotation.steps.site.noLink":
    "Questo accesso non ha un indirizzo https, quindi Mosaic non ha una pagina di cambio da suggerire. Cambia la password a mano sul sito.",
  "credentialRotation.steps.site.copyPassword": "Copia nuova password",
  "credentialRotation.steps.site.next": "Ho provato",
  "credentialRotation.steps.result.title": "Il sito ha accettato la nuova password?",
  "credentialRotation.steps.done.failed": "Password attuale mantenuta",
  "credentialRotation.steps.done.notSure": "Cambio lasciato aperto",
  "credentialRotation.steps.result.body":
    "Mosaic non può vedere il sito. Dì a Mosaic cosa è successo, così in questo elemento resta la password giusta. In entrambi i casi non viene eliminato nulla.",
  "credentialRotation.steps.result.working": "Salvataggio…",
  "credentialRotation.result.worked": "Ha funzionato",
  "credentialRotation.result.failed": "Non ha funzionato",
  "credentialRotation.result.notSure": "Non ancora sicuro",
  "credentialRotation.result.workedDone":
    "Fatto. La nuova password è ora la password di questo elemento. Quella vecchia è nella cronologia.",
  "credentialRotation.result.failedDone":
    "La password attuale resta. Quella nuova viene conservata nella cronologia come non confermata, nel caso il sito l'avesse applicata comunque.",
  "credentialRotation.result.notSureDone":
    "Il cambio resta aperto su questo elemento. Torna quando lo sai.",
  "credentialRotation.banner.title": "Cambio password iniziato il {date, date, medium}",
  "credentialRotation.banner.prepared":
    "La nuova password è conservata qui finché non confermi. Nessuna pagina l'ha ancora ricevuta.",
  "credentialRotation.banner.delivered":
    "La nuova password è conservata qui finché non confermi. L'estensione del browser l'ha inserita nel modulo del sito.",
  "credentialRotation.banner.reveal": "Mostra nuova password",
  "credentialRotation.banner.hide": "Nascondi nuova password",
  "credentialRotation.banner.copy": "Copia nuova password",
  "credentialRotation.banner.newPassword": "Nuova password",
  "credentialRotation.banner.copyLink": "Copia il link alla pagina del cambio",
  "credentialRotation.history.title": "Cronologia delle password",
  "credentialRotation.history.list": "Password precedenti",
  "credentialRotation.history.entry": "Sostituita il {date, date, medium}",
  "credentialRotation.history.unconfirmed": "Non confermata",
  "credentialRotation.history.unconfirmedEntry": "Provata il {date, date, medium}",
  "credentialRotation.history.unconfirmedHint":
    "Il sito potrebbe aver applicato questa. Ripristinala se l'accesso con la password attuale non riesce.",
  "credentialRotation.history.reveal": "Mostra la password del {date, date, medium}",
  "credentialRotation.history.hide": "Nascondi la password del {date, date, medium}",
  "credentialRotation.history.restore": "Ripristina",
  "credentialRotation.history.restoreFor": "Ripristina la password del {date, date, medium}",
  "credentialRotation.history.restored":
    "Ripristinata. La password precedente è nella cronologia.",
  "credentialRotation.policy.label": "Ricordami di cambiare questa password",
  "credentialRotation.policy.none": "Mai",
  "credentialRotation.policy.every": "Ogni {days} giorni",
  "credentialRotation.policy.help":
    "Un promemoria mette l'elemento nell'elenco Cambia ora. Mosaic non invia notifiche e non fa nessuna richiesta di rete per questo.",
  "credentialRotation.url.help":
    "Facoltativo. L'indirizzo https della pagina delle password del sito, sullo stesso sito di questo accesso. Mosaic lo apre solo quando scegli Apri la pagina del cambio nell'estensione del browser.",
  "credentialRotation.url.invalid":
    "Mosaic non userà questo indirizzo. Deve iniziare con https:// e trovarsi sullo stesso sito di questo accesso.",
  "credentialRotation.settings.title": "Promemoria delle password",
  "credentialRotation.settings.body":
    "Dopo lo sblocco Mosaic può dirti quante password conviene cambiare.",
  "credentialRotation.settings.reminders": "Mostra dopo lo sblocco quante password cambiare",
  "credentialRotation.settings.remindersHint":
    "Solo un messaggio nell'app. Nessuna notifica, nessuna richiesta di rete.",
  "credentialRotation.reminder.toast":
    "{count, plural, one {# password da cambiare} other {# password da cambiare}}",
  "credentialRotation.reminder.show": "Mostra",
  "credentialRotation.dialog.beginTitle": "Avviare un cambio di password?",
  "credentialRotation.dialog.beginBody":
    "{browser} vuole cambiare la password di {title} su {host}. Mosaic conserva la nuova password insieme a questo accesso e mantiene quella attuale finché non confermi che il sito l'ha accettata.",
  "credentialRotation.dialog.site": "Sito web",
  "credentialRotation.dialog.login": "Accesso",
  "credentialRotation.dialog.password": "Nuova password",
  "credentialRotation.dialog.hidden": "Nascosta",
  "credentialRotation.dialog.show": "Mostra",
  "credentialRotation.dialog.hide": "Nascondi",
  "credentialRotation.dialog.regenerate": "Generane un'altra",
  "credentialRotation.dialog.regenerated": "È stata generata una nuova password.",
  "credentialRotation.dialog.allow": "Usa questa password",
  "credentialRotation.dialog.cancel": "Annulla",
  "credentialRotation.dialog.decideLater": "Decidi più tardi",
  "credentialRotation.dialog.untitled": "questo accesso",
  "credentialRotation.dialog.attempt":
    "Iniziato il {at, date, medium}, {at, time, short} · {stage}{n, plural, =0 {} one { · # altro cambio aperto} other { · # altri cambi aperti}}",
  "credentialRotation.dialog.stagePrepared": "nessuna pagina ha ancora la nuova password",
  "credentialRotation.dialog.stageDelivered": "la nuova password è stata inserita in una pagina",
  "credentialRotation.dialog.resultTitle": "{host} ha accettato la nuova password?",
  "credentialRotation.dialog.resultBody":
    "{browser} dice che il cambio ha funzionato. Se confermi, la nuova password diventa la password di {title} e quella attuale passa nella sua cronologia.",
  "credentialRotation.dialog.confirm": "Ha funzionato",
  "credentialRotation.dialog.keepOld": "Mantieni la vecchia password",
  "credentialRotation.dialog.failedTitle": "Mantenere la vecchia password?",
  "credentialRotation.dialog.failedBody":
    "{browser} dice che il cambio non ha funzionato. Mosaic mantiene la password attuale di {title}. Il nuovo valore resta nella cronologia, nel caso il sito l'avesse cambiata comunque.",
  "credentialRotation.addOrigin.title": "Aggiungere {host} a {title}?",
  "credentialRotation.addOrigin.body":
    "Mosaic compilerà questo accesso anche su {host}. Aggiungilo solo se {host} fa davvero parte dello stesso sito.",
  "credentialRotation.addOrigin.idn":
    "Questo indirizzo usa caratteri speciali. Il tuo browser potrebbe mostrarlo come {unicode}. Controllalo con attenzione.",
  "credentialRotation.addOrigin.add": "Aggiungi indirizzo",
  "credentialRotation.addOrigin.cancel": "Non ora",
} satisfies Record<keyof typeof en, string>;
