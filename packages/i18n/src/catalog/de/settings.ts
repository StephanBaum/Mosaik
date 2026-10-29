/** German UI strings: settings. Informal "du". Same keys as English (checked by the type). */
import type { settings as en } from "../en/settings.ts";

export const settings = {
  "settings.title": "Einstellungen",
  "settings.security": "Sicherheit",
  "settings.autoLock": "Nach Inaktivität sperren",
  "settings.autoLock.never": "Nie",
  "settings.autoLock.minutes": "{minutes, plural, one {# Minute} other {# Minuten}}",
  "settings.lockOnSleep": "Beim Ruhezustand sperren",
  "settings.lockOnScreenLock": "Beim Sperren des Bildschirms sperren",
  "settings.screenLock.unsupported":
    "Mosaic erkennt auf diesem System nicht, wann der Bildschirm gesperrt wird. Es sperrt trotzdem nach Inaktivität und beim Aufwachen des Computers.",
  "settings.lockTrigger.required":
    "Lass mindestens eine Sperre an. Mosaic erkennt auf diesem System nicht, wann der Bildschirm gesperrt wird.",
  "settings.clipboard": "Zwischenablage leeren nach",
  "settings.clipboard.never": "Nie",
  "settings.clipboard.seconds": "{seconds, plural, one {# Sekunde} other {# Sekunden}}",
  "settings.appearance": "Darstellung",
  "settings.theme": "Design",
  "settings.theme.system": "Wie das System",
  "settings.theme.light": "Hell",
  "settings.theme.dark": "Dunkel",
  "settings.language": "Sprache",
  "settings.language.system": "Wie das System",
  "settings.language.en": "Englisch",
  "settings.language.de": "Deutsch",
  "settings.ssh": "SSH-Agent",
  "settings.ssh.enable": "Meine SSH-Schlüssel aus Mosaic verwenden",
  "settings.ssh.hint":
    "Standardmäßig aus. Der Agent läuft nur, solange Mosaic entsperrt ist, und fragt vor jeder Nutzung.",
  "settings.ssh.status":
    "{count, plural, =0 {Keine Schlüssel angeboten} one {# Schlüssel angeboten} other {# Schlüssel angeboten}}",
  "settings.ssh.endpoint": "Setze SSH_AUTH_SOCK auf",
  "settings.ssh.skipped": "Schlüssel, die nicht angeboten werden",
  "settings.kit": "Emergency Kit",
  "settings.kit.save": "Emergency Kit noch einmal speichern",
  "settings.saved": "Einstellungen gespeichert.",
  "reauth.title": "Mit deinem Master-Passwort bestätigen",
  "reauth.body":
    "Diese Änderung macht Mosaic weniger sicher, daher braucht sie dein Master-Passwort.",
  "reauth.bodyKit":
    "Dein Secret Key wird gleich in eine Datei geschrieben, daher braucht Mosaic dein Master-Passwort.",
  "reauth.password": "Master-Passwort",
  "reauth.submit": "Bestätigen",
} satisfies Record<keyof typeof en, string>;
