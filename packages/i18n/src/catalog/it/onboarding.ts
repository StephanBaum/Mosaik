/**
 * Italian UI strings: onboarding. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { onboarding as en } from "../en/onboarding.ts";

export const onboarding = {
  "onboarding.secretKey.title": "La tua Chiave segreta",
  "onboarding.secretKey.body":
    "La tua Chiave segreta e la password principale insieme sbloccano la tua cassaforte. Non possiamo recuperare nessuna delle due.",
  "onboarding.secretKey.verify":
    "Digita l'ultimo gruppo della tua Chiave segreta per confermare di averla salvata.",
  "onboarding.secretKey.mistyped":
    "Non corrisponde. Controlla l'ultimo gruppo nel tuo Kit di emergenza.",
  "onboarding.step": "Passaggio {current} di {total}",
  "onboarding.create.title": "Crea il tuo account",
  "onboarding.create.body":
    "La tua password principale e una Chiave segreta generata su questo dispositivo insieme sbloccano la tua cassaforte. Nessuna delle due viene mai inviata al server.",
  "onboarding.create.email": "Email",
  "onboarding.create.password": "Password principale",
  "onboarding.create.passwordHint":
    "Quattro parole senza legame tra loro funzionano bene. Usala solo per Mosaic.",
  "onboarding.create.confirm": "Conferma la password principale",
  "onboarding.create.mismatch":
    "Le due password non coincidono. Digita la stessa password principale in entrambi i campi.",
  "onboarding.create.submit": "Crea account",
  "onboarding.create.working": "Creazione del tuo account. Richiede qualche secondo.",
  "onboarding.kit.accountId": "ID account",
  "onboarding.kit.secretKey": "Chiave segreta",
  "onboarding.kit.lastGroup": "Ultimo gruppo della tua Chiave segreta",
  "onboarding.kit.confirm": "Conferma",
  "kit.title": "Salva il tuo Kit di emergenza",
  "kit.body":
    "Il kit è un PDF con la tua Chiave segreta. Insieme alla password principale apre la tua cassaforte su un nuovo dispositivo. Conservalo in un posto sicuro, lontano da questo computer.",
  "kit.secretKey": "La tua Chiave segreta",
  "kit.save": "Salva il Kit di emergenza (PDF)",
  "kit.saved": "Salvato. Conserva il file in un posto sicuro.",
  "kit.cancelled": "Non ancora salvato. Salva il kit per continuare.",
  "kit.recover.alreadySaved": "L'ho già salvato",
  "kit.recover.title": "Salva di nuovo il tuo Kit di emergenza",
  "kit.recover.body":
    "Non hai ancora confermato il tuo Kit di emergenza. Inserisci la password principale per salvarlo di nuovo, poi conferma l'ultimo gruppo della tua Chiave segreta.",
  "onboarding.import.title": "Porta con te le tue password",
  "onboarding.import.body":
    "Da Chrome, Edge, Firefox, Safari e altri browser, oppure da 1Password, Bitwarden, LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm, KeePass ed Enpass. Servono pochi minuti. Puoi farlo anche più tardi in Impostazioni.",
  "onboarding.import.start": "Importa da un'altra app",
  "onboarding.import.skip": "Salta per ora",
  "onboarding.done.title": "È tutto pronto",
  "onboarding.done.body":
    "La tua cassaforte è pronta. Bloccala quando vuoi con il pulsante Blocca.",
  "onboarding.browser.title": "Installa l'estensione del browser",
  "onboarding.browser.body":
    "Compila gli accessi nel browser. L'estensione compila solo quando fai clic su di essa o usi la sua scorciatoia da tastiera.",
  "onboarding.browser.steps": "Come collegare un browser",
  "onboarding.browser.step1": "Installa l'estensione di Mosaic nel tuo browser.",
  "onboarding.browser.step2": "Apri l'estensione e scegli Collega.",
  "onboarding.browser.step3":
    "Controlla che il codice coincida, poi scegli Consenti nella finestra che si apre qui.",
  "onboarding.browser.notInStores":
    "L'estensione non è ancora negli store dei browser. Puoi collegarla più tardi in Impostazioni → Browser.",
  "onboarding.browser.getFor": "Ottieni per {browser}",
  "onboarding.browser.connectedList": "Browser collegati",
  "onboarding.browser.waiting": "In attesa dell'estensione. Nessun browser è ancora collegato.",
  "onboarding.browser.connected": "{name} collegato",
  "onboarding.browser.done": "Fine",
  "onboarding.browser.skip": "Salta per ora",
  "onboarding.progress": "Avanzamento della configurazione",
  "onboarding.stepName.create": "Crea account",
  "onboarding.stepName.kit": "Kit di emergenza",
  "onboarding.stepName.verify": "Conferma il tuo kit",
  "onboarding.stepName.import": "Importa (facoltativo)",
  "onboarding.stepName.browser": "Estensione del browser (facoltativo)",
  "kit.preview": "Anteprima del Kit di emergenza",
  "kit.continueHint": "Salva il kit per continuare.",
  "kit.warning":
    "Senza questo kit e la tua password principale, nessuno può recuperare la tua cassaforte, nemmeno noi.",
  "onboarding.join.link": "Uso già Mosaic",
  "onboarding.join.title": "Aggiungi questo dispositivo al tuo account",
  "onboarding.join.body":
    "Accedi con la password principale e la Chiave segreta del tuo Kit di emergenza. Poi le tue casseforti vengono scaricate e decifrate su questo dispositivo.",
  "onboarding.join.tabs": "Dove sono i tuoi dati di sincronizzazione?",
  "onboarding.join.tab.server": "Server Mosaic",
  "onboarding.join.tab.folder": "Cartella cloud",
  "onboarding.join.serverUrl": "Indirizzo del server",
  "onboarding.join.email": "Email dell'account di sincronizzazione",
  "onboarding.join.password": "Password principale",
  "onboarding.join.secretKey": "Chiave segreta",
  "onboarding.join.secretKeyHint": "Dal tuo Kit di emergenza. Inizia con M1.",
  "onboarding.join.deviceName": "Nome di questo dispositivo",
  "onboarding.join.chooseFolder": "Scegli cartella…",
  "onboarding.join.folder": "Cartella",
  "onboarding.join.noAccounts":
    "Questa cartella non contiene nessun account Mosaic. Scegli la cartella con cui si sincronizza il tuo altro dispositivo.",
  "onboarding.join.account": "Account",
  "onboarding.join.accountOption": "Account {id}",
  "onboarding.join.submit": "Aggiungi questo dispositivo",
  "onboarding.join.working": "Accesso e download delle tue casseforti…",
  "onboarding.join.twoFactor.title": "Inserisci il tuo codice",
  "onboarding.join.twoFactor.body":
    "Il tuo account di sincronizzazione usa la verifica in due passaggi. Inserisci il codice di 6 cifre della tua app di autenticazione.",
  "onboarding.join.twoFactor.code": "Codice di 6 cifre",
  "onboarding.join.twoFactor.submit": "Verifica",
  "onboarding.join.back": "Crea invece un nuovo account",
} satisfies Record<keyof typeof en, string>;
