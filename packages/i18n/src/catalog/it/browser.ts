/**
 * Italian UI strings: browser. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { browser as en } from "../en/browser.ts";

export const browser = {
  "browser.pair.title": "Un browser vuole collegarsi a Mosaic",
  "browser.pair.description":
    "Consenti solo se hai appena fatto clic su Collega nell'estensione di Mosaic e mostra questo codice.",
  "browser.pair.code": "Codice",
  "browser.pair.browser": "Browser",
  "browser.pair.label": "Nome per questo browser",
  "browser.pair.labelHint": "Vedrai questo nome nelle Impostazioni.",
  "browser.pair.deny": "Nega",
  "browser.pair.allow": "Consenti",
  "browser.expires":
    "Si chiude automaticamente tra {seconds, plural, one {# secondo} other {# secondi}}",
  "browser.escDenies": "per negare",
  "browser.hint.chrome": "Chrome",
  "browser.hint.edge": "Edge",
  "browser.hint.brave": "Brave",
  "browser.hint.opera": "Opera",
  "browser.hint.vivaldi": "Vivaldi",
  "browser.hint.chromium": "Chromium",
  "browser.hint.firefox": "Firefox",
  "browser.hint.other": "Browser",
  "browser.family.chromium": "Browser basato su Chromium",
  "browser.family.firefox": "Firefox",
  "browser.escCancels": "per annullare",
  "browser.save.titleNew": "Salvare questo accesso in Mosaic?",
  "browser.save.titleUpdate": "Aggiornare questo accesso in Mosaic?",
  "browser.save.description":
    "{browser} propone di salvare un accesso. Mosaic lo scrive solo se scegli Salva.",
  "browser.save.descriptionUpdate":
    "{browser} propone una nuova password per questo accesso. Mosaic la modifica solo se scegli Aggiorna password.",
  "browser.save.site": "Sito web",
  "browser.save.username": "Nome utente",
  "browser.save.noUsername": "Nessuno",
  "browser.save.password": "Password",
  "browser.save.passwordHidden": "Nascosta",
  "browser.save.target": "Salva come",
  "browser.save.updateOption": "Aggiorna “{title}”",
  "browser.save.newOption": "Un nuovo accesso",
  "browser.save.itemTitle": "Titolo",
  "browser.save.vault": "Cassaforte",
  "browser.save.allowHttp": "Consenti la compilazione su questa pagina non cifrata",
  "browser.save.allowHttpHint":
    "Il sito non usa HTTPS. Chiunque sia nella rete potrebbe leggere ciò che inserisci.",
  "browser.save.cancel": "Annulla",
  "browser.save.saveNew": "Salva",
  "browser.save.update": "Aggiorna password",
  "browser.passkey.secure": "Connessione sicura (https)",
  "browser.passkey.getTitle": "Accedi a {rpId} con una passkey",
  "browser.passkey.createTitle": "Salva una nuova passkey",
  "browser.passkey.excludedTitle": "Passkey già salvata",
  "browser.passkey.createBody": "Creare una passkey per {rpId} in Mosaic?",
  "browser.passkey.excludedBody": "Hai già una passkey per {rpId} in Mosaic.",
  "browser.passkey.uvNote":
    "Il sito ha richiesto la verifica dell'utente. Inserisci la password principale per confermare che sei tu.",
  "browser.passkey.password": "Password principale",
  "browser.passkey.account": "Account",
  "browser.passkey.noName": "Senza nome",
  "browser.passkey.vault": "Cassaforte",
  "browser.passkey.signIn": "Accedi con passkey",
  "browser.passkey.save": "Salva passkey",
  "browser.passkey.anotherDevice": "Usa un altro dispositivo o una chiave di sicurezza",
  "browser.passkey.cancel": "Annulla",
  "browser.passkey.ok": "Ho capito",
  "browser.passkey.escCloses": "per chiudere",
  "browser.passkey.passwordNeeded":
    "Inserisci la password principale per confermare che sei tu.",
  "browser.settings.title": "Browser",
  "browser.settings.body":
    "Estensioni del browser che compilano gli accessi da questa app. Approvi ogni collegamento qui.",
  "browser.settings.empty":
    "Nessun browser è ancora collegato. Installa l'estensione di Mosaic, poi scegli Collega al suo interno.",
  "browser.settings.connected": "Collegato ora",
  "browser.settings.lastUsed": "Ultimo uso {when}",
  "browser.settings.neverUsed": "Mai usato",
  "browser.settings.disconnect": "Scollega",
  "browser.settings.disconnectNamed": "Scollega {name}",
  "browser.settings.disconnectTitle": "Scollegare “{name}”?",
  "browser.settings.disconnectBody":
    "Questo browser non potrà compilare gli accessi finché non lo ricolleghi dall'estensione.",
  "browser.settings.endpointInUse":
    "Un altro programma, o un'altra finestra di Mosaic, usa già il collegamento per i browser. Chiudilo, poi riavvia Mosaic.",
  "browser.settings.endpointUnavailable":
    "Mosaic non è riuscito ad aprire il collegamento per i browser. Riavvia Mosaic. Se succede ancora, controlla che la tua cartella utente sia scrivibile.",
  "browser.settings.registryUnreadable":
    "I collegamenti dei browser salvati non possono essere letti. Mosaic non li modificherà finché il problema non sarà risolto.",
  "browser.connector.title": "Un collegamento del browser punta altrove",
  "browser.connector.body":
    "Il collegamento per {browser} avvia un programma diverso da Mosaic: {path}. Potrebbe essere una versione precedente di Mosaic o qualcos'altro che si è registrato da solo. Con Ripara torna a puntare a questo Mosaic.",
  "browser.connector.item": "{browser}: {path}",
  "browser.connector.repair": "Ripara",
  "browser.connector.repaired": "Collegamento del browser riparato.",
} satisfies Record<keyof typeof en, string>;
