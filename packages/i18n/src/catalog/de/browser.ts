/** German UI strings: browser extension (pairing, saving, settings). Informal "du". Same keys as English (checked by the type). */
import type { browser as en } from "../en/browser.ts";

export const browser = {
  "browser.pair.title": "Ein Browser möchte sich mit Mosaic verbinden",
  "browser.pair.description":
    "Erlaube das nur, wenn du gerade in der Mosaic-Erweiterung auf „Verbinden“ geklickt hast und sie diesen Code zeigt.",
  "browser.pair.code": "Code",
  "browser.pair.browser": "Browser",
  "browser.pair.label": "Name für diesen Browser",
  "browser.pair.labelHint": "Diesen Namen siehst du in den Einstellungen.",
  "browser.pair.deny": "Ablehnen",
  "browser.pair.allow": "Erlauben",
  "browser.expires":
    "Schließt sich automatisch in {seconds, plural, one {# Sekunde} other {# Sekunden}}",
  "browser.escDenies": "zum Ablehnen",
  "browser.hint.chrome": "Chrome",
  "browser.hint.edge": "Edge",
  "browser.hint.brave": "Brave",
  "browser.hint.opera": "Opera",
  "browser.hint.vivaldi": "Vivaldi",
  "browser.hint.chromium": "Chromium",
  "browser.hint.firefox": "Firefox",
  "browser.hint.other": "Browser",
  "browser.family.chromium": "Browser auf Chromium-Basis",
  "browser.family.firefox": "Firefox",
  "browser.escCancels": "zum Abbrechen",
  "browser.save.titleNew": "Dieses Login in Mosaic speichern?",
  "browser.save.titleUpdate": "Dieses Login in Mosaic aktualisieren?",
  "browser.save.description":
    "{browser} bietet an, ein Login zu speichern. Mosaic schreibt es nur, wenn du „Speichern“ wählst.",
  "browser.save.descriptionUpdate":
    "{browser} bietet ein neues Passwort für dieses Login an. Mosaic ändert es nur, wenn du „Passwort aktualisieren“ wählst.",
  "browser.save.site": "Website",
  "browser.save.username": "Benutzername",
  "browser.save.noUsername": "Keiner",
  "browser.save.password": "Passwort",
  "browser.save.passwordHidden": "Verborgen",
  "browser.save.target": "Speichern als",
  "browser.save.updateOption": "„{title}“ aktualisieren",
  "browser.save.newOption": "Neues Login",
  "browser.save.itemTitle": "Titel",
  "browser.save.vault": "Tresor",
  "browser.save.allowHttp": "Ausfüllen auf dieser unverschlüsselten Seite erlauben",
  "browser.save.allowHttpHint":
    "Die Seite nutzt kein HTTPS. Jeder im Netzwerk könnte mitlesen, was du eingibst.",
  "browser.save.cancel": "Abbrechen",
  "browser.save.saveNew": "Speichern",
  "browser.save.update": "Passwort aktualisieren",
  "browser.passkey.secure": "Sichere Verbindung (https)",
  "browser.passkey.getTitle": "Mit einem Passkey bei {rpId} anmelden",
  "browser.passkey.createTitle": "Neuen Passkey speichern",
  "browser.passkey.excludedTitle": "Passkey bereits gespeichert",
  "browser.passkey.createBody": "Passkey für {rpId} in Mosaic erstellen?",
  "browser.passkey.excludedBody": "Du hast bereits einen Passkey für {rpId} in Mosaic.",
  "browser.passkey.uvNote":
    "Die Website verlangt eine Bestätigung. Gib dein Master-Passwort ein.",
  "browser.passkey.password": "Master-Passwort",
  "browser.passkey.account": "Konto",
  "browser.passkey.noName": "Kein Name",
  "browser.passkey.vault": "Tresor",
  "browser.passkey.signIn": "Mit Passkey anmelden",
  "browser.passkey.save": "Passkey speichern",
  "browser.passkey.anotherDevice": "Anderes Gerät oder Sicherheitsschlüssel verwenden",
  "browser.passkey.cancel": "Abbrechen",
  "browser.passkey.ok": "Verstanden",
  "browser.passkey.escCloses": "zum Schließen",
  "browser.passkey.passwordNeeded":
    "Gib dein Master-Passwort ein, um zu bestätigen, dass du es bist.",
  "browser.settings.title": "Browser",
  "browser.settings.body":
    "Browser-Erweiterungen, die Logins aus dieser App ausfüllen. Jede Verbindung bestätigst du hier.",
  "browser.settings.empty":
    "Noch ist kein Browser verbunden. Installiere die Mosaic-Erweiterung und wähle dort „Verbinden“.",
  "browser.settings.connected": "Jetzt verbunden",
  "browser.settings.lastUsed": "Zuletzt genutzt {when}",
  "browser.settings.neverUsed": "Noch nicht genutzt",
  "browser.settings.disconnect": "Trennen",
  "browser.settings.disconnectNamed": "{name} trennen",
  "browser.settings.disconnectTitle": "„{name}“ trennen?",
  "browser.settings.disconnectBody":
    "Dieser Browser kann keine Logins mehr ausfüllen, bis du ihn in der Erweiterung neu verbindest.",
  "browser.settings.endpointInUse":
    "Ein anderes Programm oder ein anderes Mosaic-Fenster nutzt die Verbindung für Browser bereits. Schließe es und starte Mosaic neu.",
  "browser.settings.endpointUnavailable":
    "Mosaic konnte die Verbindung für Browser nicht öffnen. Starte Mosaic neu. Wenn das bleibt, prüfe, ob dein Benutzerordner beschreibbar ist.",
  "browser.settings.registryUnreadable":
    "Gespeicherte Browser-Verbindungen lassen sich nicht lesen. Mosaic ändert sie nicht, bis das behoben ist.",
} as const satisfies Record<keyof typeof en, string>;
