/**
 * Italian UI strings: app. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { app as en } from "../en/app.ts";

export const app = {
  "app.name": "Mosaic",
  "app.preAudit": "Software in fase pre-audit: non ancora verificato da un revisore di sicurezza esterno.",
  "app.previewTag": "Anteprima",
  "app.previewBuild":
    "Versione di anteprima, non una release. Non firmata, pre-audit, solo per provare.",
  "action.cancel": "Annulla",
  "action.continue": "Continua",
  "action.retry": "Riprova",
  "action.copy": "Copia",
  "action.reveal": "Mostra",
  "action.hide": "Nascondi",
  "action.lock": "Blocca",
  "action.details": "Dettagli",
  "sync.offline":
    "Impossibile raggiungere il tuo server di sincronizzazione. Le modifiche sono salvate su questo dispositivo e verranno sincronizzate quando tornerai online.",
  "app.loading": "Apertura di Mosaic…",
  "app.skipToContent": "Vai al contenuto",
  "action.save": "Salva",
  "action.close": "Chiudi",
  "action.showPassword": "Mostra password",
  "action.hidePassword": "Nascondi password",
  "action.back": "Indietro",
  "action.confirm": "Conferma",
  "action.edit": "Modifica",
  "action.delete": "Sposta nel Cestino",
  "action.restore": "Ripristina",
  "action.purge": "Elimina definitivamente",
  "action.generate": "Genera",
  "action.useThis": "Usa questa",
  "action.done": "Fine",
  "nav.main": "Navigazione principale",
  "nav.vault": "Cassaforte",
  "nav.security": "Sicurezza",
  "nav.generator": "Generatore",
  "nav.settings": "Impostazioni",
  "nav.trash": "Cestino",
  "nav.breadcrumb": "Percorso",
  "nav.securityCount":
    "{count, plural, one {# segnalazione da controllare} other {# segnalazioni da controllare}}",
  "sync.status.local": "Salvato su questo dispositivo",
  "sync.status.upToDate": "Sincronizzato",
  "sync.status.attention": "Richiede attenzione",
  "sync.status.syncing": "Sincronizzazione in corso…",
  "sync.status.offline": "Offline",
  "sync.status.error": "Sincronizzazione non riuscita",
  "action.moreActions": "Altre azioni",
  "action.regenerate": "Genera di nuovo",
} satisfies Record<keyof typeof en, string>;
