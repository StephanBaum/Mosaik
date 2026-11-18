/**
 * Italian UI strings: settings. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { settings as en } from "../en/settings.ts";

export const settings = {
  "settings.title": "Impostazioni",
  "settings.security": "Sicurezza",
  "settings.autoLock": "Blocca dopo un periodo di inattività",
  "settings.autoLock.never": "Mai",
  "settings.autoLock.minutes": "{minutes, plural, one {# minuto} other {# minuti}}",
  "settings.lockOnSleep": "Blocca quando il computer va in stop",
  "settings.lockOnScreenLock": "Blocca quando lo schermo si blocca",
  "settings.screenLock.unsupported":
    "Mosaic non riesce a capire quando lo schermo si blocca su questo sistema. Si blocca comunque dopo l'inattività e quando il computer si riattiva.",
  "settings.lockTrigger.required":
    "Mantieni attivo almeno un blocco. Mosaic non riesce a rilevare uno schermo bloccato su questo sistema.",
  "settings.clipboard": "Cancella gli appunti dopo",
  "settings.clipboard.never": "Mai",
  "settings.clipboard.seconds": "{seconds, plural, one {# secondo} other {# secondi}}",
  "settings.appearance": "Aspetto",
  "settings.theme": "Tema",
  "settings.theme.system": "Segui il sistema",
  "settings.theme.light": "Chiaro",
  "settings.theme.dark": "Scuro",
  "settings.language": "Lingua",
  "settings.language.system": "Segui il sistema",
  "settings.language.machineNote":
    "Tradotto con l'aiuto dell'IA. Se qualcosa suona male, faccelo sapere.",
  "settings.import": "Importa",
  "settings.import.body":
    "Porta i tuoi accessi da un browser o da un altro gestore di password. Mosaic ti guida nell'esportazione e ti aiuta a eliminare il file dopo.",
  "settings.import.open": "Importa password",
  "settings.ssh": "Agente SSH",
  "settings.ssh.enable": "Usa le mie chiavi SSH da Mosaic",
  "settings.ssh.hint":
    "Disattivato per impostazione predefinita. L'agente funziona solo mentre Mosaic è sbloccato e ti chiede conferma a ogni utilizzo.",
  "settings.ssh.status":
    "{count, plural, =0 {Nessuna chiave offerta} one {# chiave offerta} other {# chiavi offerte}}",
  "settings.ssh.endpoint": "Imposta SSH_AUTH_SOCK su",
  "settings.ssh.skipped": "Chiavi non offerte",
  "settings.kit": "Kit di emergenza",
  "settings.kit.save": "Salva di nuovo il Kit di emergenza",
  "settings.saved": "Impostazioni salvate.",
  "reauth.title": "Conferma con la password principale",
  "reauth.body":
    "Questa modifica rende Mosaic meno sicuro, quindi serve la tua password principale.",
  "reauth.bodyKit":
    "La tua Chiave segreta sta per essere mostrata in un file, quindi Mosaic ha bisogno della tua password principale.",
  "reauth.password": "Password principale",
  "reauth.submit": "Conferma",
  "settings.intro":
    "Come Mosaic si blocca, cancella i segreti e protegge le tue chiavi su questo dispositivo.",
  "settings.section.autoLock": "Blocco automatico",
  "settings.section.autoLock.body": "Quando Mosaic si blocca da solo senza che tu lo chieda.",
  "settings.section.clipboard": "Appunti",
  "settings.section.clipboard.body": "I segreti copiati vengono cancellati dopo questo tempo.",
  "settings.appearance.body": "Il tema e la lingua di Mosaic.",
  "settings.ssh.body":
    "Lascia che i programmi accedano con le chiavi SSH della tua cassaforte. Approvi ogni utilizzo.",
  "settings.kit.body":
    "Salva una copia nuova del tuo Kit di emergenza. Serve la tua password principale.",
  "settings.quick": "Sblocco rapido",
  "settings.quick.body": "Modi più veloci per rientrare, oltre alla tua password principale.",
  "settings.quick.windowsHello": "Windows Hello",
  "settings.quick.touchId": "Touch ID",
  "settings.quick.biometric.body":
    "Sblocca con il volto, l'impronta digitale o il PIN. Dopo un riavvio serve comunque la tua password principale.",
  "settings.quick.biometric.bodyMac":
    "Sblocca con la tua impronta digitale. Dopo un riavvio serve comunque la tua password principale.",
  "settings.quick.expires":
    "Mosaic chiede di nuovo la tua password principale il {date, date, medium}.",
  "settings.quick.maxAge": "Chiedi la password principale almeno ogni",
  "settings.quick.maxAgeDays": "{days, plural, one {# giorno} other {# giorni}}",
  "settings.quick.biometricLinux": "Lo sblocco biometrico non è disponibile su Linux.",
  "settings.quick.biometricUnavailable": "Non configurato su questo dispositivo.",
  "settings.quick.keys": "Chiavi di sicurezza",
  "settings.quick.keysCount":
    "{count, plural, =0 {Nessuna registrata} one {# registrata} other {# registrate}}",
  "settings.quick.keysUnsupported":
    "Le chiavi di sicurezza non sono ancora supportate su questo sistema.",
  "settings.quick.keyRow": "Chiave di sicurezza · {tag} · aggiunta il {addedAt, date, medium}",
  "settings.quick.addKey": "Aggiungi chiave di sicurezza",
  "settings.quick.addKeyBody":
    "Inserisci la chiave che vuoi aggiungere e digita il suo PIN. Poi toccala quando lampeggia.",
  "settings.quick.manageKeys": "Gestisci",
  "settings.quick.removeKey": "Rimuovi",
  "settings.quick.removeKeyNamed": "Rimuovi la chiave di sicurezza {tag}",
  "settings.quick.removeKeyConfirm": "Rimuovere la chiave di sicurezza {tag}?",
  "settings.quick.keyRisk":
    "Chiunque abbia questa chiave e il suo PIN può aprire la tua cassaforte senza la tua password principale.",
  "reauth.bodyQuick":
    "Questo rende più facile entrare in Mosaic, quindi serve la tua password principale.",
  "reauth.bodyKey": "Per aggiungere una chiave a questa cassaforte serve la tua password principale.",
  "settings.passkeyExport": "Esporta passkey",
  "settings.passkeyExport.body":
    "Salva le passkey di una cassaforte in un file FIDO CXF, per spostarle in un'altra app.",
  "settings.passkeyExport.vault": "Cassaforte",
  "settings.passkeyExport.warning":
    "Il file contiene le tue passkey non cifrate. Chiunque abbia il file può accedere come te. Eliminalo dopo l'importazione.",
  "settings.passkeyExport.none": "Questa cassaforte non ha passkey da esportare.",
  "settings.passkeyExport.button": "Esporta passkey",
  "settings.passkeyExport.reauth":
    "Il file conterrà le tue passkey non cifrate, quindi per esportare serve la tua password principale.",
  "settings.passkeyExport.saved":
    "{count, plural, one {# passkey esportata.} other {# passkey esportate.}} Elimina il file dopo l'importazione.",
  "reauth.bodyMove":
    "Per spostare un elemento fuori da una cassaforte protetta da una chiave di sicurezza serve la tua password principale.",
  "toast.settingsReset":
    "Le tue impostazioni di sicurezza sono state modificate fuori da Mosaic e sono state riportate a valori sicuri.",
  "settings.shortcut.title": "Scorciatoia per la ricerca rapida",
  "settings.shortcut.body":
    "Premi questa combinazione in qualsiasi programma per aprire la ricerca di Mosaic.",
  "settings.shortcut.label": "Scorciatoia",
  "settings.shortcut.hint":
    "Fai clic sul campo, poi premi i tasti che vuoi: Ctrl, Alt o ⌘ (su un Mac), più un altro tasto.",
  "settings.shortcut.save": "Salva scorciatoia",
  "settings.shortcut.reset": "Usa quella predefinita",
  "settings.shortcut.saved": "Scorciatoia salvata: {shortcut}.",
  "settings.shortcut.invalid":
    "Questa non è una scorciatoia che Mosaic può usare. Premi Ctrl, Alt o ⌘ (su un Mac) insieme a un altro tasto.",
  "settings.shortcut.unsupported":
    "Il tuo sistema non permette alle app di impostare una scorciatoia globale, quindi qui non si può cambiare.",
} satisfies Record<keyof typeof en, string>;
