/**
 * Italian UI strings: mobileUx. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { mobileUx as en } from "../en/mobileUx.ts";

export const mobileUx = {
  "mobile.sync.warningsBanner":
    "La sincronizzazione ha trovato un problema. Apri le Impostazioni per controllarlo.",
  "mobile.provider.errorTitle": "Mosaic non può continuare",
  "mobile.onboarding.details.keyWiped":
    "Mosaic ha rimosso la Chiave segreta scansionata quando hai lasciato l'app. Scansiona di nuovo il codice o digita la chiave.",
  "mobile.common.showField": "Mostra {field}",
  "mobile.common.hideField": "Nascondi {field}",
  "mobile.error.storeInUse":
    "Mosaic è occupato altrove su questo telefono. Attendi un momento, poi riprova.",
  "mobile.error.noAccount":
    "Mosaic non è ancora configurato su questo telefono. Apri Mosaic per configurarlo.",
  "mobile.error.wrongCredentials":
    "La password principale o la Chiave segreta non ha funzionato. Controlla eventuali errori di battitura e la lingua della tastiera, poi riprova.",
  "mobile.error.invalidInput":
    "Qualcosa di ciò che hai inserito non è valido. Controlla i dati e riprova.",
  "mobile.error.vaultUnavailable":
    "Questa cassaforte non può essere aperta in questo momento. Sincronizza e riprova, oppure sblocca di nuovo Mosaic.",
  "mobile.unlock.quickOptionTouchId": "La prossima volta sblocca con Touch ID",
  "mobile.detail.readCode": "Leggi il codice",
  "mobile.detail.labelValue": "{label}: {value}",
  "mobile.autofill.warningWithName": "Controlla prima l'indirizzo | {name}",
  "mobile.autofill.linkEntry": "Collega {login} e compila",
  "mobile.link.imported": "Importato per questa app",
  "mobile.link.confirmTitle": "Collegare {login} a {app}?",
  "mobile.link.confirmSubtitle": "ID app {appId} · Firma {fingerprint}",
  "mobile.link.confirmFallbackHint": "Dopo il collegamento, tocca di nuovo il campo per compilarlo.",
  "mobile.link.importedNotVerified":
    "Accesso importato. Il sito web non ha confermato questa app.",
} satisfies Record<keyof typeof en, string>;
