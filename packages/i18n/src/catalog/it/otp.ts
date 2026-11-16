/**
 * Italian UI strings: otp. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { otp as en } from "../en/otp.ts";

export const otp = {
  "otp.section.label": "Codice monouso (2FA)",
  "otp.section.empty":
    "Aggiungi il codice dalla pagina di configurazione della verifica in due passaggi del sito.",
  "otp.section.required": "Aggiungi un codice monouso per salvare questo elemento.",
  "otp.source.screen": "Scansiona lo schermo",
  "otp.source.clipboard": "Incolla screenshot",
  "otp.source.file": "Scegli immagine…",
  "otp.source.manual": "Non riesci a scansionare? Inserisci la chiave",
  "otp.source.screenHint":
    "Mosaic si fa da parte per un momento, guarda lo schermo una volta e non conserva altro che il codice.",
  "otp.source.clipboardHint":
    "{platform, select, mac {Premi Cmd+Ctrl+Maiusc+4, seleziona il codice QR, poi incollalo qui.} windows {Premi Win+Maiusc+S, seleziona il codice QR, poi incollalo qui.} other {Copia uno screenshot del codice QR, poi incollalo qui.}}",
  "otp.dialog.title": "Configura un codice monouso",
  "otp.working.screen": "Ricerca di un codice QR sullo schermo…",
  "otp.working.image": "Lettura del codice QR…",
  "otp.manual.title": "Inserisci la chiave di configurazione",
  "otp.manual.input": "Chiave di configurazione o link otpauth://",
  "otp.manual.inputHint":
    "Il sito la mostra accanto al codice QR, spesso sotto “Non riesci a scansionare?” o “Inserisci manualmente”. Gli spazi non contano.",
  "otp.manual.issuer": "Servizio",
  "otp.manual.account": "Account",
  "otp.manual.advanced": "Avanzate",
  "otp.manual.algorithm": "Algoritmo",
  "otp.manual.digits": "Cifre",
  "otp.manual.period": "Nuovo codice ogni",
  "otp.manual.periodValue": "{seconds} secondi",
  "otp.manual.advancedHint":
    "Cambiale solo se lo dice il sito. La maggior parte dei siti usa SHA-1, 6 cifre e 30 secondi.",
  "otp.manual.continue": "Continua",
  "otp.confirm.title": "È il codice giusto?",
  "otp.confirm.issuer": "Servizio",
  "otp.confirm.account": "Account",
  "otp.confirm.details":
    "{scheme, select, hotp {Basato su contatore, {digits} cifre} steam {Steam Guard, 5 caratteri} other {{digits} cifre, nuovo codice ogni {period} secondi}}",
  "otp.confirm.algorithmValue":
    "{algorithm, select, sha256 {SHA-256} sha512 {SHA-512} other {SHA-1}}",
  "otp.confirm.codeLabel": "Codice attuale",
  "otp.confirm.codeHint":
    "Se il sito chiede un codice per completare la configurazione, inserisci questo.",
  "otp.confirm.hotpNote":
    "I codici basati su contatore compaiono dopo il salvataggio, così il contatore resta allineato con il sito.",
  "otp.confirm.weak":
    "Questo segreto è insolitamente corto. Funziona, ma la protezione a due fattori di questo sito è più debole del solito.",
  "otp.confirm.use": "Usa questo codice",
  "otp.confirm.back": "Scansiona di nuovo",
  "otp.confirm.copy": "Copia codice",
  "otp.summary.noIssuer": "Codice monouso",
  "otp.summary.pending": "Salvato insieme a questo elemento.",
  "otp.summary.removed": "Il codice monouso verrà rimosso quando salvi.",
  "otp.action.replace": "Sostituisci",
  "otp.action.remove": "Rimuovi il codice monouso",
  "otp.action.keep": "Conservalo",
  "otp.action.cancelSetup": "Non aggiungere",
  "otp.storage.label": "Dove conservare il codice",
  "otp.storage.item": "Con questo elemento",
  "otp.storage.itemTradeoff":
    "Comodo, ma chiunque apra questa cassaforte ottiene insieme la tua password e il tuo secondo fattore.",
  "otp.storage.hardware": "In {vault} (richiede la tua chiave di sicurezza)",
  "otp.storage.hardwareClosed": "Apri prima {vault} con la tua chiave di sicurezza.",
  "otp.toast.clipboardCleared": "Screenshot letto. Mosaic ha cancellato gli appunti.",
  "otp.toast.savedInVault": "Codice salvato in {vault}.",
  "otp.clipboardCopies":
    "{platform, select, windows {Se la cronologia degli appunti o lo Strumento di cattura hanno conservato una copia dello screenshot, eliminala lì.} mac {Se hai salvato lo screenshot anche come file, eliminalo.} other {Se un gestore degli appunti ha conservato una copia dello screenshot, eliminala lì.}}",
} satisfies Record<keyof typeof en, string>;
