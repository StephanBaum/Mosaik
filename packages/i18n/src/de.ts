import type { MessageKey } from "./en.ts";

/** German UI strings. Informal "du", see docs/UX_GUIDELINES.md §5. */
export const de: Readonly<Record<MessageKey, string>> = {
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

  "lock.unlockTitle": "Mosaic entsperren",
  "lock.masterPassword": "Master-Passwort",
  "lock.wrongPassword":
    "Dieses Master-Passwort hat nicht funktioniert. Prüfe die Feststelltaste und dein Tastaturlayout und versuche es erneut.",
  "lock.locked": "Mosaic ist gesperrt.",

  "onboarding.secretKey.title": "Dein Secret Key",
  "onboarding.secretKey.body":
    "Dein Secret Key und dein Master-Passwort entsperren gemeinsam deinen Tresor. Wir können keins von beiden wiederherstellen. Sichere jetzt dein Emergency Kit.",
  "onboarding.secretKey.verify":
    "Gib die letzte Gruppe deines Secret Keys ein, um zu bestätigen, dass du ihn gesichert hast.",
  "onboarding.secretKey.mistyped":
    "Das stimmt nicht überein. Prüfe die letzte Gruppe in deinem Emergency Kit.",

  "clipboard.copied":
    "Kopiert. Wird in {seconds, plural, one {# Sekunde} other {# Sekunden}} geleert.",
  "clipboard.cleared": "Zwischenablage geleert.",

  "sync.offline":
    "Dein Sync-Server ist nicht erreichbar. Deine Änderungen sind auf diesem Gerät gespeichert und werden synchronisiert, sobald du wieder online bist.",

  "vault.itemCount":
    "{count, plural, =0 {Keine Einträge} one {# Eintrag} other {# Einträge}}",
  "vault.deleteConfirm":
    "„{title}“ löschen? Du kannst den Eintrag 30 Tage lang aus dem Papierkorb wiederherstellen.",
};
