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
  "settings.intro": "Wie Mosaic sperrt, Geheimnisse löscht und deine Schlüssel auf diesem Gerät schützt.",
  "settings.section.autoLock": "Automatisch sperren",
  "settings.section.autoLock.body": "Wann sich Mosaic selbst sperrt, ohne dass du es verlangst.",
  "settings.section.clipboard": "Zwischenablage",
  "settings.section.clipboard.body": "Kopierte Geheimnisse werden nach dieser Zeit gelöscht.",
  "settings.appearance.body": "Das Erscheinungsbild und die Sprache von Mosaic.",
  "settings.ssh.body": "Lass Programme sich mit SSH-Schlüsseln aus deinem Tresor anmelden. Du bestätigst jede Nutzung.",
  "settings.kit.body": "Speichere eine neue Kopie deines Emergency Kit. Dafür brauchst du dein Master-Passwort.",
  "settings.quick": "Schnellentsperrung",
  "settings.quick.body": "Schnellere Wege zurück, zusätzlich zu deinem Master-Passwort.",
  "settings.quick.windowsHello": "Windows Hello",
  "settings.quick.touchId": "Touch ID",
  "settings.quick.biometric.body":
    "Entsperre mit Gesicht, Fingerabdruck oder PIN. Nach einem Neustart brauchst du weiterhin dein Master-Passwort.",
  "settings.quick.biometric.bodyMac":
    "Entsperre mit deinem Fingerabdruck. Nach einem Neustart brauchst du weiterhin dein Master-Passwort.",
  "settings.quick.expires": "Mosaic fragt am {date, date, medium} wieder nach deinem Master-Passwort.",
  "settings.quick.maxAge": "Mindestens so oft nach dem Master-Passwort fragen",
  "settings.quick.maxAgeDays": "{days, plural, one {# Tag} other {# Tage}}",
  "settings.quick.biometricLinux": "Biometrisches Entsperren ist unter Linux nicht verfügbar.",
  "settings.quick.biometricUnavailable": "Auf diesem Gerät nicht eingerichtet.",
  "settings.quick.keys": "Sicherheitsschlüssel",
  "settings.quick.keysCount":
    "{count, plural, =0 {Keiner registriert} one {# registriert} other {# registriert}}",
  "settings.quick.keysUnsupported":
    "Sicherheitsschlüssel werden auf diesem System noch nicht unterstützt.",
  "settings.quick.keyRow": "Sicherheitsschlüssel · {tag} · hinzugefügt am {addedAt, date, medium}",
  "settings.quick.addKey": "Sicherheitsschlüssel hinzufügen",
  "settings.quick.addKeyBody":
    "Stecke den Schlüssel ein, den du hinzufügen möchtest, und gib seine PIN ein. Berühre ihn dann, wenn er blinkt.",
  "settings.quick.manageKeys": "Verwalten",
  "settings.quick.removeKey": "Entfernen",
  "settings.quick.removeKeyNamed": "Sicherheitsschlüssel {tag} entfernen",
  "settings.quick.removeKeyConfirm": "Sicherheitsschlüssel {tag} entfernen?",
  "settings.quick.keyRisk":
    "Wer diesen Schlüssel und seine PIN hat, kann deinen Tresor ohne dein Master-Passwort öffnen.",
  "reauth.bodyQuick":
    "Das macht den Zugang zu Mosaic einfacher, deshalb braucht es dein Master-Passwort.",
  "reauth.bodyKey": "Um einen Schlüssel zu diesem Tresor hinzuzufügen, brauchst du dein Master-Passwort.",
  "settings.passkeyExport": "Passkeys exportieren",
  "settings.passkeyExport.body":
    "Speichere die Passkeys eines Tresors in einer FIDO-CXF-Datei, um sie in eine andere App zu übertragen.",
  "settings.passkeyExport.vault": "Tresor",
  "settings.passkeyExport.warning":
    "Die Datei enthält deine Passkeys unverschlüsselt. Wer die Datei hat, kann sich als du anmelden. Lösche sie nach dem Import.",
  "settings.passkeyExport.none": "In diesem Tresor gibt es keine Passkeys zum Exportieren.",
  "settings.passkeyExport.button": "Passkeys exportieren",
  "settings.passkeyExport.reauth":
    "Die Datei enthält deine Passkeys unverschlüsselt, deshalb braucht der Export dein Master-Passwort.",
  "settings.passkeyExport.saved":
    "{count, plural, one {# Passkey exportiert.} other {# Passkeys exportiert.}} Lösche die Datei nach dem Import.",
  "reauth.bodyMove":
    "Um einen Eintrag aus einem Tresor mit Sicherheitsschlüssel zu verschieben, brauchst du dein Master-Passwort.",
  "toast.settingsReset":
    "Deine Sicherheitseinstellungen wurden außerhalb von Mosaic geändert und auf sichere Standardwerte zurückgesetzt.",
} satisfies Record<keyof typeof en, string>;
