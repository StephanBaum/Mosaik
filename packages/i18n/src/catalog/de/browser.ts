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
  "browser.save.titleNew": "Diesen Zugang in Mosaic speichern?",
  "browser.save.titleUpdate": "Diesen Zugang in Mosaic aktualisieren?",
  "browser.save.description":
    "{browser} bietet an, einen Zugang zu speichern. Mosaic schreibt ihn nur, wenn du „Speichern“ wählst.",
  "browser.save.site": "Website",
  "browser.save.username": "Benutzername",
  "browser.save.noUsername": "Keiner",
  "browser.save.password": "Passwort",
  "browser.save.passwordHidden": "Verborgen",
  "browser.save.target": "Speichern als",
  "browser.save.updateOption": "„{title}“ aktualisieren",
  "browser.save.newOption": "Neuer Zugang",
  "browser.save.itemTitle": "Titel",
  "browser.save.vault": "Tresor",
  "browser.save.allowHttp": "Ausfüllen auf dieser unverschlüsselten Seite erlauben",
  "browser.save.allowHttpHint":
    "Die Seite nutzt kein HTTPS. Jeder im Netzwerk könnte mitlesen, was du eingibst.",
  "browser.save.cancel": "Abbrechen",
  "browser.save.saveNew": "Speichern",
  "browser.save.update": "Passwort aktualisieren",
  "browser.settings.title": "Browser",
  "browser.settings.body":
    "Browser-Erweiterungen, die Zugangsdaten aus dieser App ausfüllen. Jede Verbindung bestätigst du hier.",
  "browser.settings.empty":
    "Noch ist kein Browser verbunden. Installiere die Mosaic-Erweiterung und wähle dort „Verbinden“.",
  "browser.settings.connected": "Jetzt verbunden",
  "browser.settings.lastUsed": "Zuletzt genutzt {when}",
  "browser.settings.neverUsed": "Noch nicht genutzt",
  "browser.settings.disconnect": "Trennen",
  "browser.settings.disconnectTitle": "„{name}“ trennen?",
  "browser.settings.disconnectBody":
    "Dieser Browser kann keine Zugangsdaten mehr ausfüllen, bis du ihn in der Erweiterung neu verbindest.",
  "browser.settings.endpointInUse":
    "Ein anderes Programm oder ein anderes Mosaic-Fenster nutzt die Verbindung für Browser bereits. Schließe es und starte Mosaic neu.",
  "browser.settings.endpointUnavailable":
    "Mosaic konnte die Verbindung für Browser nicht öffnen. Starte Mosaic neu. Wenn das bleibt, prüfe, ob dein Benutzerordner beschreibbar ist.",
  "browser.settings.registryUnreadable":
    "Gespeicherte Browser-Verbindungen lassen sich nicht lesen. Mosaic ändert sie nicht, bis das behoben ist.",
} as const satisfies Record<keyof typeof en, string>;
