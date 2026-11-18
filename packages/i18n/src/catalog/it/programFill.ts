/**
 * Italian UI strings: programFill. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { programFill as en } from "../en/programFill.ts";

export const programFill = {
  "programFill.chip.label": "Compila con Mosaic",
  "programFill.chip.locked": "Sblocca Mosaic per compilare",
  "programFill.list.title": "Accessi per {program}",
  "programFill.list.none": "Ancora nessun accesso per {program}",
  "programFill.list.signerChanged": "Il firmatario di questo programma è cambiato. Collegare di nuovo?",
  "programFill.list.fileChanged":
    "Questo programma è cambiato da quando lo hai collegato. Collegare di nuovo?",
  "programFill.list.suggestions": "Suggerimenti",
  "programFill.list.search": "Cerca accessi",
  "programFill.list.link": "Collega un accesso…",
  "programFill.list.block": "Non proporre per questo programma",
  "programFill.link.packaged": "{name} · App dello Store · Editore {publisher}",
  "programFill.link.packagedNoPublisher": "{name} · App dello Store",
  "programFill.link.signed": "{exe} · Firmato da {signer}, verificato da Windows",
  "programFill.link.unsigned": "{exe} · Non firmato · {path}",
  "programFill.link.unsignedNote":
    "Mosaic compilerà solo questo file esatto. Se cambia, Mosaic chiede di nuovo.",
  "programFill.link.lookalike":
    "Attenzione: questo nome somiglia a {similar}, che hai collegato in precedenza.",
  "programFill.link.hashChange": "File collegato {old}…, questo file {new}…",
  "programFill.link.login": "Accesso: {title}",
  "programFill.link.confirm": "Collega e accedi",
  "programFill.link.confirmFill": "Collega e compila",
  "programFill.link.confirmUnsigned": "Collega questo programma non firmato",
  "programFill.link.confirmAgain": "Conferma di nuovo per collegare",
  "programFill.notice.elevated":
    "Mosaic non può compilare i programmi eseguiti come amministratore. Copia invece la password da Mosaic.",
  "programFill.notice.noField": "Mosaic non vede un campo di accesso qui.",
  "programFill.notice.aborted":
    "Mosaic ha interrotto la compilazione perché la finestra è cambiata. Controlla il campo.",
  "programFill.notice.abortedCleared":
    "Mosaic ha interrotto la compilazione perché la finestra è cambiata e ha rimosso ciò che aveva digitato.",
  "programFill.notice.modifiersHeld":
    "Rilascia Maiusc, Ctrl, Alt e il tasto Windows, poi scegli di nuovo l'accesso.",
  "programFill.notice.notResponding":
    "Questo programma non ha risposto in tempo. Non è stato compilato altro.",
  "programFill.notice.close": "Chiudi",
  "programFill.settings.title": "Compilazione nei programmi",
  "programFill.settings.description":
    "Mosaic osserva quale campo ha il focus negli altri programmi su questo dispositivo per proporre lì i tuoi accessi. Non viene memorizzato né inviato nulla.",
  "programFill.settings.enable": "Proponi gli accessi nei programmi",
  "programFill.settings.submit": "Accedi dopo la compilazione",
  "programFill.settings.hotkey": "Scorciatoia per aprire l'elenco",
  "programFill.settings.hotkeyNone": "Nessuna scorciatoia",
  "programFill.settings.hotkeySave": "Salva la scorciatoia dell'elenco",
  "programFill.settings.blocked": "Programmi per cui Mosaic non propone accessi",
  "programFill.settings.blockedNone": "Nessuno",
  "programFill.settings.unblock": "Proponi di nuovo",
  "programFill.settings.notWindows": "Per ora solo su Windows.",
  "programFill.settings.mosaicElevated":
    "Mosaic è eseguito come amministratore, quindi non compila i programmi. Avvialo normalmente.",
  "programFill.settings.hostFailed":
    "Mosaic non è riuscito ad avviare l'osservazione dei programmi. Riavvia Mosaic per riprovare.",
  "programFill.card.title": "Compila gli accessi anche nei programmi",
  "programFill.card.turnOn": "Attiva",
  "programFill.card.notNow": "Non ora",
  "programFill.item.program": "Programma",
  "programFill.item.storeApp": "App dello Store",
  "programFill.item.signedBy": "Firmato da {signer}",
  "programFill.item.unsigned": "Non firmato: {path}",
  "programFill.item.submit": "Accedi dopo la compilazione",
  "programFill.item.unlink": "Scollega",
} satisfies Record<keyof typeof en, string>;
