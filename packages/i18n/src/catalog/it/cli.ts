/**
 * Italian CLI messages. Placeholders are plain `{name}` only (ADR-0075).
 * `pnpm --filter @mosaic/i18n export:cli` writes them to `crates/cli/i18n/it.json`.
 */
import type { cli as en } from "../en/cli.ts";

export const cli = {
  "cli.err.storeInUse":
    "L'app Mosaic ha aperto questa cassaforte. Chiudila, oppure dai alla CLI un dispositivo tutto suo con `mosaic setup own-store`.",
  "cli.err.noStore":
    "Nessuna cassaforte trovata in {path}. Configura prima Mosaic nell'app, oppure usa --store.",
  "cli.err.noAccount": "Questo archivio non ha ancora un account.",
  "cli.err.wrongCredentials": "Password principale o Chiave segreta errata.",
  "cli.err.secretKeyInvalid":
    "Questa Chiave segreta non è valida. Controllala con il tuo Kit di emergenza.",
  "cli.err.inputRequired": "Serve un input, ma è impostato --no-input.",
  "cli.err.promptTimeout": "Nessun input entro {seconds} secondi.",
  "cli.err.notFound": "Non è stato trovato nulla per {ref}.",
  "cli.err.ambiguousRef":
    "Più di un elemento corrisponde a {ref}. Usa un riferimento più preciso o l'ID dell'elemento.",
  "cli.err.invalidRef": "{ref} non è un riferimento valido.",
  "cli.err.noSuchField": "Questo elemento non ha il campo {field}.",
  "cli.err.noOtp": "Questo elemento non ha una password monouso.",
  "cli.err.revealRequired":
    "{field} è segreto. Aggiungi --reveal per stamparlo, oppure --copy per metterlo negli appunti.",
  "cli.err.vaultUnavailable":
    "Quella cassaforte è protetta dall'hardware oppure non è disponibile qui.",
  "cli.err.rotationInProgress": "Completa prima il rinnovo delle chiavi nell'app Mosaic.",
  "cli.err.secretInArgv":
    "{flag} metterebbe un segreto nella riga di comando. Usa invece la richiesta interattiva, --password-stdin o --password-fd.",
  "cli.err.unknownCommand": "Comando sconosciuto {name}. Esegui `mosaic help`.",
  "cli.err.unknownOption": "Opzione sconosciuta {name}.",
  "cli.err.missingValue": "L'opzione {name} richiede un valore.",
  "cli.err.missingArgument": "Manca l'argomento {name}.",
  "cli.err.unexpectedArgument": "Argomento imprevisto {value}.",
  "cli.err.invalidValue": "Valore non valido per {name}.",
  "cli.err.nonUtf8": "Gli argomenti devono essere UTF-8 validi.",
  "cli.err.fdUnsupported": "--password-fd non è disponibile su questo sistema.",
  "cli.err.sessionExpired": "La sessione di sblocco è terminata. Inserisci la password principale.",
  "cli.err.sessionNotCached":
    "Non è stato possibile salvare una sessione di sblocco (l'archivio chiavi del sistema o la cartella di runtime non sono disponibili). Ogni comando chiederà la tua password principale.",
  "cli.err.twoFactorMethodUnsupported":
    "Questo account accede con una chiave di sicurezza, che la riga di comando non può usare. Aggiungi un codice di un'app di autenticazione nell'app desktop, poi riprova.",
  "cli.err.envSecretIgnored":
    "{name} è impostata ma viene ignorata. Mosaic non legge mai segreti dall'ambiente.",
  "cli.err.exportUnreadable": "Impossibile leggere il file: frase d'accesso errata o file danneggiato.",
  "cli.err.importFailed": "Non è stato possibile importare il file.",
  "cli.err.importTooLarge": "Il file è troppo grande per essere importato.",
  "cli.err.syncNotConfigured": "La sincronizzazione non è configurata per questo archivio.",
  "cli.err.syncHalted": "La sincronizzazione è ferma. Esegui `mosaic sync --status`.",
  "cli.err.syncFailed": "Sincronizzazione non riuscita: {reason}.",
  "cli.err.syncSessionRevoked":
    "Le chiavi di questo account sono state rinnovate su un altro dispositivo, quindi questa sessione è terminata. Accedi di nuovo con la nuova Chiave segreta del tuo Kit di emergenza più recente.",
  "cli.err.syncSessionPasswordChanged":
    "La tua password principale è stata cambiata su un altro dispositivo, quindi questa sessione è terminata. Accedi di nuovo con la nuova password.",
  "cli.err.syncSessionDeviceRemoved":
    "Questo dispositivo è stato rimosso dal tuo account da un altro dispositivo, quindi questa sessione è terminata. Accedi di nuovo per aggiungerlo di nuovo.",
  "cli.err.syncSessionOperator":
    "Il gestore del server ha terminato questa sessione. Accedi di nuovo per continuare a sincronizzare.",
  "cli.err.fileExists": "{path} esiste già. Mosaic non sovrascrive i file.",
  "cli.err.fileRead": "Impossibile leggere {path}.",
  "cli.err.fileWrite": "Impossibile scrivere {path}.",
  "cli.err.passphraseMismatch": "Le due frasi d'accesso sono diverse.",
  "cli.err.passphraseWeak": "Usa una frase d'accesso di almeno {min} caratteri.",
  "cli.err.badEnvName": "{name} non è un nome valido per una variabile d'ambiente.",
  "cli.err.spawnFailed": "Impossibile avviare {command}.",
  "cli.err.noVaultChosen": "Esistono più casseforti. Scegline una con --vault.",
  "cli.err.notSupported": "{what} non è disponibile in questa versione della CLI.",
  "cli.err.storeExists": "{path} esiste già.",
  "cli.err.internal": "Qualcosa è andato storto.",
  "cli.prompt.password": "Password principale: ",
  "cli.prompt.secretKey": "Chiave segreta (dal tuo Kit di emergenza): ",
  "cli.prompt.filePassword": "Password del file: ",
  "cli.prompt.exportPassphrase": "Frase d'accesso per il file di esportazione: ",
  "cli.prompt.exportPassphraseAgain": "Ripeti la frase d'accesso: ",
  "cli.prompt.importPassphrase": "Frase d'accesso del file di esportazione: ",
  "cli.prompt.confirmImport": "Importare {count} elementi in {vault}? [y/N] ",
  "cli.prompt.newPassword": "Nuova password principale (quella che hai impostato altrove): ",
  "cli.prompt.twoFactor": "Codice di verifica in due passaggi: ",
  "cli.out.recovered": "Recuperato. Questo dispositivo ora usa la tua nuova password principale.",
  "cli.prompt.acknowledge": "Digita la parola {word} per continuare: ",
  "cli.warn.revealToTerminal":
    "I valori segreti vengono stampati nel terminale e restano nella sua cronologia.",
  "cli.out.unlocked": "Sbloccato per {minutes} minuti. Esegui `mosaic lock` quando hai finito.",
  "cli.out.locked": "Bloccato. Sessione rimossa.",
  "cli.out.concealed": "••••••••",
  "cli.out.copied": "Copiato. Si cancella tra {seconds} secondi.",
  "cli.out.noItems": "Nessun elemento.",
  "cli.out.ambiguousCandidate": "{vault} / {title} ({id})",
  "cli.out.attachment": "{name} ({size} byte, {mime})",
  "cli.label.id": "ID",
  "cli.label.vault": "Cassaforte",
  "cli.label.type": "Tipo",
  "cli.label.title": "Titolo",
  "cli.label.otp": "Codice monouso",
  "cli.label.attachments": "Allegati",
  "cli.status.store": "Archivio",
  "cli.status.profile": "Profilo",
  "cli.status.account": "Account",
  "cli.status.session": "Sessione",
  "cli.status.sessionNone": "bloccato (nessuna sessione)",
  "cli.status.sessionUntil": "sbloccato fino a {time} (Unix ms)",
  "cli.status.sync": "Sincronizzazione",
  "cli.status.syncOff": "non configurata",
  "cli.status.syncOn": "{backend}, ferma: {halted}, segnalazioni: {findings}",
  "cli.import.preview": "{count} elementi da importare in {vault}: {summary}",
  "cli.import.duplicates": "{count} sembrano duplicati e vengono saltati.",
  "cli.import.warnings": "{count} voci non sono state lette e sono escluse.",
  "cli.import.done": "Importati {imported} elementi ({skipped} saltati, {failed} non riusciti).",
  "cli.import.dryRun": "Prova: non è stato importato nulla.",
  "cli.import.cancelled": "Annullato. Non è stato importato nulla.",
  "cli.export.done": "Esportati {items} elementi da {vaults} casseforti in {path}.",
  "cli.export.attachmentWarn": "{count} elementi hanno allegati. Sono inclusi.",
  "cli.export.attachmentsLeftOut":
    "{count} file allegati non sono nell'esportazione. Usa --include-attachments.",
  "cli.export.unavailableVaults": "{count} casseforti protette dall'hardware sono state escluse.",
  "cli.sync.done": "Sincronizzazione completata: {pulled} ricevuti, {pushed} inviati.",
  "cli.sync.finding": "Segnalazione: {kind}.",
  "cli.sync.clean": "La sincronizzazione è a posto. Nessuna segnalazione.",
  "cli.sync.acknowledged": "Segnalazioni cancellate.",
  "cli.sync.acknowledgeWord": "acknowledge",
  "cli.setup.done": "Questo dispositivo è configurato come archivio proprio. Esegui `mosaic sync`.",
  "cli.ssh.agentStatus": "L'agente SSH funziona nell'app Mosaic. La CLI non ne avvia uno.",
  "cli.ssh.noKeys": "Nessuna chiave SSH.",
  "cli.err.agentContext":
    "Sembra una sessione di un agente IA. Stampare qui un segreto lo metterebbe nel contesto dell'agente. Usa invece `mosaic agent mcp`, oppure aggiungi --i-know se lo esegui tu stesso.",
  "cli.err.agentAlreadyPaired":
    "Esiste già un collegamento chiamato {name}. Aggiungi --replace per collegarlo di nuovo.",
  "cli.err.desktopNotRunning": "L'app Mosaic non è in esecuzione.",
  "cli.err.agentAccessOff":
    "L'accesso degli agenti è disattivato. Attivalo nell'app Mosaic, in Impostazioni.",
  "cli.err.agentLocked": "La cassaforte è bloccata. Sblocca prima l'app Mosaic.",
  "cli.err.pairingDenied": "Il collegamento è stato negato nell'app Mosaic.",
  "cli.err.pairingExpired": "La richiesta di collegamento è scaduta.",
  "cli.err.pairingLimit":
    "L'app Mosaic ha raggiunto il numero massimo di collegamenti consentiti. Rimuovine prima uno.",
  "cli.err.pairingNameTaken":
    "Esiste già un collegamento con questo nome: rimuovilo prima o scegli un altro nome.",
  "cli.err.pairingBlocked":
    "Il collegamento è bloccato per qualche minuto dopo ripetuti rifiuti.",
  "cli.err.notPaired": "Questo client non è collegato. Esegui `mosaic agent pair --name {name}`.",
  "cli.err.agentRefused": "L'app Mosaic ha rifiutato la richiesta ({reason}).",
  "cli.err.agentIo": "La connessione con l'app Mosaic non è riuscita.",
  "cli.agent.pairCode": "Confronta questo codice con quello mostrato da Mosaic: {code}",
  "cli.agent.paired": "Collegato come {name}.",
  "cli.agent.status":
    "Desktop: {desktop}. Cassaforte: {vault}. Accesso degli agenti: {access}. Collegati: {paired}.",
} satisfies Record<keyof typeof en, string>;
