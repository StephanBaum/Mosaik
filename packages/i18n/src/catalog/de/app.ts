/** German UI strings: app. Informal "du". Same keys as English (checked by the type). */
import type { app as en } from "../en/app.ts";

export const app = {
  "app.name": "Mosaic",
  "app.preAudit":
    "Software vor dem Audit: noch nicht von einem externen Sicherheitsprüfer begutachtet.",
  "action.cancel": "Abbrechen",
  "action.continue": "Weiter",
  "action.retry": "Erneut versuchen",
  "action.copy": "Kopieren",
  "action.reveal": "Anzeigen",
  "action.hide": "Verbergen",
  "action.lock": "Sperren",
  "action.details": "Details",
  "sync.offline":
    "Dein Sync-Server ist nicht erreichbar. Deine Änderungen sind auf diesem Gerät gespeichert und werden synchronisiert, sobald du wieder online bist.",
  "app.loading": "Mosaic wird geöffnet …",
  "app.skipToContent": "Zum Inhalt springen",
  "action.save": "Speichern",
  "action.close": "Schließen",
  "action.back": "Zurück",
  "action.confirm": "Bestätigen",
  "action.edit": "Bearbeiten",
  "action.delete": "In den Papierkorb",
  "action.restore": "Wiederherstellen",
  "action.purge": "Endgültig löschen",
  "action.generate": "Erzeugen",
  "action.useThis": "Übernehmen",
  "action.done": "Fertig",
  "nav.main": "Hauptnavigation",
  "nav.vault": "Tresor",
  "nav.security": "Sicherheit",
  "nav.generator": "Generator",
  "nav.settings": "Einstellungen",
  "nav.trash": "Papierkorb",
} satisfies Record<keyof typeof en, string>;
