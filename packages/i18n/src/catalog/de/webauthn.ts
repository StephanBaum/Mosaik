/**
 * Deutsche Texte der Sicherheitsschlüssel-Seite, die der Sync-Server unter `/webauthn/` ausliefert
 * (CRYPTO_SPEC §9.3). Nur einfache `{name}`-Platzhalter: Die Seite setzt sie selbst ein und
 * schreibt das Ergebnis als Text, nie als HTML. `pnpm --filter @mosaic/i18n export:webauthn`
 * schreibt beide Kataloge nach `crates/server/assets/webauthn/strings.json`.
 */
import type { webauthn as en } from "../en/webauthn.ts";

export const webauthn: Record<keyof typeof en, string> = {
  "webauthn.documentTitle": "Mosaic: Sicherheitsschlüssel",
  "webauthn.heading.code": "Prüfe zuerst den Code",
  "webauthn.lead.code":
    "Gib die 6 Ziffern ein, die die Mosaic-App auf deinem Computer anzeigt. Erst danach fragt diese Seite nach deinem Sicherheitsschlüssel.",
  "webauthn.code.label": "Code aus der Mosaic-App",
  "webauthn.code.submit": "Weiter",
  "webauthn.code.format": "Gib die 6 Ziffern ein.",
  "webauthn.code.invalid": "Dieser Code stimmt nicht. Prüfe die Ziffern in der Mosaic-App.",
  "webauthn.heading.login": "Mit deinem Sicherheitsschlüssel anmelden",
  "webauthn.heading.register": "Sicherheitsschlüssel hinzufügen",
  "webauthn.heading.confirm": "Mit deinem Sicherheitsschlüssel bestätigen",
  "webauthn.facts.device": "Gerät",
  "webauthn.facts.time": "Angefragt um",
  "webauthn.facts.account": "Konto",
  "webauthn.lead.ask":
    "Prüfe, ob Gerät und Zeit oben zu dir gehören. Nutze dann deinen Sicherheitsschlüssel, sobald dein Browser danach fragt.",
  "webauthn.status.waiting": "Wartet auf deinen Sicherheitsschlüssel.",
  "webauthn.status.sending": "Deine Antwort wird geprüft.",
  "webauthn.retry": "Noch einmal versuchen",
  "webauthn.cancelled":
    "Dein Sicherheitsschlüssel wurde nicht benutzt: Die Anfrage wurde abgebrochen oder ist abgelaufen.",
  "webauthn.alreadyRegistered": "Dieser Sicherheitsschlüssel gehört schon zu deinem Konto.",
  "webauthn.browserError": "Dein Browser konnte dafür keinen Sicherheitsschlüssel benutzen.",
  "webauthn.unsupported":
    "Dieser Browser kann keine Sicherheitsschlüssel benutzen. Öffne den Link in einem aktuellen Browser.",
  "webauthn.heading.done": "Fertig",
  "webauthn.done.login":
    "Du bist angemeldet. Gehe zurück zur Mosaic-App. Du kannst diesen Tab schließen.",
  "webauthn.done.register":
    "Dein Sicherheitsschlüssel ist hinzugefügt. Gehe zurück zur Mosaic-App. Du kannst diesen Tab schließen.",
  "webauthn.done.confirm":
    "Bestätigt. Gehe zurück zur Mosaic-App. Du kannst diesen Tab schließen.",
  "webauthn.heading.expired": "Diese Anfrage ist abgelaufen",
  "webauthn.expired.body":
    "Die Anfrage war nur wenige Minuten gültig. Starte noch einmal in der Mosaic-App auf deinem Computer.",
  "webauthn.heading.failed": "Das hat nicht geklappt",
  "webauthn.failed.body":
    "Dein Sicherheitsschlüssel wurde nicht akzeptiert, oder alle Versuche sind aufgebraucht. Starte noch einmal in der Mosaic-App auf deinem Computer.",
  "webauthn.heading.invalid": "Dieser Link ist nicht vollständig",
  "webauthn.invalid.body":
    "Öffne den Link aus der Mosaic-App auf deinem Computer. Tippe ihn nicht ab und nimm ihn nicht von woanders.",
  "webauthn.rateLimited":
    "Zu viele Versuche. Warte ein paar Minuten und starte dann noch einmal in der Mosaic-App.",
  "webauthn.unavailable":
    "Dieser Server bietet gerade keine Sicherheitsschlüssel an. Melde dich in der Mosaic-App auf andere Weise an.",
  "webauthn.network":
    "Der Server ist nicht erreichbar. Prüfe deine Verbindung und versuche es noch einmal.",
};
