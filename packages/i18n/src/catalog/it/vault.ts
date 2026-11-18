/**
 * Italian UI strings: vault. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { vault as en } from "../en/vault.ts";

export const vault = {
  "vault.itemCount": "{count, plural, =0 {Nessun elemento} one {# elemento} other {# elementi}}",
  "vault.deleteConfirm":
    "Eliminare “{title}”? Puoi ripristinarlo dal Cestino per 30 giorni.",
  "vault.defaultName": "Personale",
  "vault.unnamed": "Cassaforte senza nome",
  "vault.vaults": "Casseforti",
  "vault.allItems": "Tutti gli elementi",
  "vault.items": "Elementi",
  "vault.search": "Cerca nella tua cassaforte",
  "vault.untitled": "Senza titolo",
  "vault.noResults":
    "Nessuna corrispondenza per “{query}”. Controlla l'ortografia, oppure cerca invece un sito web.",
  "vault.new": "Nuovo elemento",
  "vault.selectItem": "Seleziona un elemento per vederne i dettagli.",
  "vault.trashEmpty": "Il Cestino è vuoto. Gli elementi eliminati restano qui per 30 giorni.",
  "vault.favorite": "Preferito",
  "vault.unfavorite": "Rimuovi dai preferiti",
  "vault.moveTo": "Sposta nella cassaforte",
  "vault.allVaults": "Tutte le casseforti",
  "vault.category.favorites": "Preferiti",
  "vault.category.logins": "Accessi",
  "vault.category.passkeys": "Passkey",
  "vault.category.codes": "Codici monouso",
  "vault.category.cards": "Carte",
  "vault.category.notes": "Note sicure",
  "vault.category.ssh": "Chiavi SSH",
  "vault.flag.otp": "2FA",
  "vault.moveToNamed": "Sposta in {name}",
  "vault.emptyTitle": "La tua cassaforte è vuota",
  "vault.emptyBody":
    "Aggiungi un accesso, una passkey, una carta o una nota. Tutto viene cifrato su questo dispositivo prima di essere sincronizzato.",
  "vault.addFirst": "Aggiungi il tuo primo elemento",
  "vault.paletteTip.before": "Premi",
  "vault.paletteTip.after": "in qualsiasi momento per cercare o eseguire un comando.",
  "trash.retention": "Gli elementi eliminati vengono conservati per 30 giorni.",
  "trash.rowMeta":
    "Eliminato il {deletedAt, date, medium} · {days, plural, one {# giorno rimasto} other {# giorni rimasti}}",
  "trash.detailMeta": "{kind} · {vault} · Eliminato il {deletedAt, date, medium}",
  "trash.purgeConfirm.title": "Eliminare definitivamente \"{title}\"?",
  "trash.purgeConfirm.body": "Non puoi annullare.",
  "vault.hardware.placeholderName": "Codici (chiave di sicurezza)",
  "vault.hardware.title": "Questa cassaforte richiede la tua chiave di sicurezza",
  "vault.hardware.body":
    "I codici qui restano al sicuro anche se qualcuno scopre la tua password principale e la tua Chiave segreta, ma non puoi vederli senza la tua chiave di sicurezza.",
  "vault.hardware.open": "Usa la chiave di sicurezza",
  "vault.hardware.lockedNote": "Bloccata. Le altre casseforti restano sbloccate.",
  "vault.hardware.lockNow": "Blocca questa cassaforte",
  "vault.hardware.closesIn": "Si blocca tra {time}",
  "vault.hardware.backupTitle": "Registra una chiave di riserva",
  "vault.hardware.backupBody":
    "Hai 1 chiave registrata. Ne consigliamo due: se perdi l'unica chiave, non potrai più aprire questa cassaforte.",
  "vault.hardware.createOption": "Proteggi con una chiave di sicurezza (solo codici)",
  "vault.hardware.tradeoff":
    "I codici in questa cassaforte richiedono ogni volta la tua chiave di sicurezza, quindi una password principale rubata da sola non basta, ma perdendo tutte le chiavi registrate perdi questi codici.",
  "vault.hardware.removeNote":
    "Rimuovere una chiave non cambia la chiave di questa cassaforte. Se una chiave è stata rubata, sposta i codici in una nuova cassaforte.",
  "vault.hardware.moveNote":
    "Spostare in questa cassaforte un codice che già usi non protegge le copie precedenti: il vecchio elemento, la sua cronologia, i backup e le esportazioni. Se puoi, emetti un nuovo segreto presso il fornitore e aggiungi quello.",
  "vault.hardware.pinBody":
    "Inserisci il PIN della tua chiave, se ne ha uno, poi toccala quando lampeggia.",
  "vault.hardware.keys": "Chiavi",
  "vault.hardware.keysLabel": "Chiavi di questa cassaforte",
  "vault.hardware.eyebrow": "Codici protetti dall'hardware",
  "vault.hardware.selfLocked":
    "Questa cassaforte si è bloccata da sola. Usa la tua chiave di sicurezza per riaprirla.",
  "vault.hardware.closingSoon":
    "Questa cassaforte si blocca tra meno di 30 secondi. Poi usa la tua chiave di sicurezza per riaprirla.",
  "vault.hardware.keysTitle": "Chiavi di questa cassaforte",
  "vault.hardware.codesOnly": "Solo codici monouso",
  "vault.create": "Nuova cassaforte",
  "vault.create.name": "Nome della cassaforte",
  "vault.create.submit": "Crea cassaforte",
  "vault.menu": "Azioni della cassaforte",
  "vaultKey.menu": "Rinnova la chiave della cassaforte…",
  "vaultKey.title": "Rinnova la chiave di {vault}",
  "vaultKey.body":
    "Mosaic crea una nuova chiave per questa cassaforte e la trasmette a ogni dispositivo e persona che ha accesso. Le copie che hanno già restano leggibili per loro; la nuova chiave protegge ciò che cambia da ora in poi. Usalo dopo che qualcuno ha perso l'accesso o un dispositivo è andato perso.",
  "vaultKey.attachments": "Rinnova anche le chiavi degli allegati",
  "vaultKey.attachments.hint":
    "Dà una nuova chiave anche a ogni file di questa cassaforte. Con file numerosi o grandi richiede più tempo.",
  "vaultKey.reauth": "Per rinnovare la chiave di una cassaforte serve la tua password principale.",
  "vaultKey.submit": "Rinnova la chiave",
  "vaultKey.working": "Rinnovo della chiave…",
  "vaultKey.progress": "Rinnovo delle chiavi degli allegati: {done} di {total}.",
  "vaultKey.done": "La chiave della cassaforte è stata rinnovata.",
  "vaultKey.doneAttachments":
    "La chiave della cassaforte è stata rinnovata, e sono state rinnovate anche le chiavi degli allegati.",
  "vaultKey.unfinished":
    "La chiave della cassaforte è stata rinnovata. Il rinnovo delle chiavi degli allegati prosegue più tardi. Se Mosaic si chiude prima, la prossima volta scegli Continua il rinnovo delle chiavi.",
  "vaultKey.unreachable":
    "{count, plural, one {# allegato non è ancora su questo dispositivo e mantiene la vecchia chiave.} other {# allegati non sono ancora su questo dispositivo e mantengono la vecchia chiave.}}",
  "vaultKey.close": "Chiudi",
  "vault.meta.title": "I dettagli della cassaforte sono stati accorciati",
  "vault.meta.body":
    "Due dispositivi hanno modificato {fields} di “{vault}” nello stesso momento e insieme erano troppo lunghi per essere salvati. Mosaic ha conservato una versione, oppure ha lasciato il campo vuoto. Controllalo e digitalo di nuovo se serve.",
  "vault.edit.title": "Modifica cassaforte",
  "vault.meta.edit": "Modifica cassaforte",
  "vault.meta.field.name": "il nome",
  "vault.meta.field.icon": "l'icona",
  "vault.meta.field.description": "la descrizione",
  "vault.meta.dismiss": "Ignora",
} satisfies Record<keyof typeof en, string>;
