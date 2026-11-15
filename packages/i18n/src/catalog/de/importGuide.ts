/**
 * German UI strings: the guided import. Informal "du". Same keys as English (checked by the type).
 * Menu paths are unverified against the current product (launch gate).
 */
import type { importGuide as en } from "../en/importGuide.ts";

export const importGuide = {
  "import.grid.title": "Wo liegen deine Passwörter gerade?",
  "import.grid.browsers": "Browser",
  "import.grid.managers": "Passwortmanager",
  "import.grid.other": "Etwas anderes",
  "import.source.chrome": "Chrome",
  "import.source.edge": "Edge",
  "import.source.brave": "Brave",
  "import.source.opera": "Opera",
  "import.source.vivaldi": "Vivaldi",
  "import.source.arc": "Arc",
  "import.source.firefox": "Firefox",
  "import.source.safari": "Safari",
  "import.source.applePasswords": "Apple Passwörter",
  "import.source.onePassword": "1Password",
  "import.source.bitwarden": "Bitwarden",
  "import.source.keepass": "KeePass",
  "import.source.keepassxcCsv": "KeePassXC (CSV)",
  "import.source.enpass": "Enpass",
  "import.source.lastpass": "LastPass",
  "import.source.dashlane": "Dashlane",
  "import.source.keeper": "Keeper",
  "import.source.nordpass": "NordPass",
  "import.source.protonPass": "Proton Pass",
  "import.source.roboform": "RoboForm",
  "import.source.otherFile": "Andere CSV- oder Datei",
  "import.guide.chrome.1":
    "Öffne diese Adresse in Chrome: Kopiere sie und füge sie in die Adressleiste ein.",
  "import.guide.chrome.2":
    "Klicke neben \"Passwörter exportieren\" auf \"Datei herunterladen\" und bestätige mit dem Passwort deines Computers.",
  "import.guide.chrome.3": "Chrome speichert \"Chrome Passwords.csv\" im Ordner Downloads.",
  "import.guide.edge.1": "Öffne diese Adresse in Edge: Kopiere sie und füge sie in die Adressleiste ein.",
  "import.guide.edge.2":
    "Klicke neben \"Gespeicherte Kennwörter\" auf \"⋯\", dann auf \"Kennwörter exportieren\" und bestätige.",
  "import.guide.edge.3": "Edge speichert die CSV-Datei im Ordner Downloads.",
  "import.guide.brave.1": "Öffne diese Adresse in Brave: Kopiere sie und füge sie in die Adressleiste ein.",
  "import.guide.brave.2":
    "Klicke neben \"Passwörter exportieren\" auf \"Datei herunterladen\" und bestätige mit dem Passwort deines Computers.",
  "import.guide.brave.3": "Brave speichert die CSV-Datei im Ordner Downloads.",
  "import.guide.opera.1": "Öffne diese Adresse in Opera: Kopiere sie und füge sie in die Adressleiste ein.",
  "import.guide.opera.2":
    "Klicke neben \"Passwörter exportieren\" auf \"Datei herunterladen\" und bestätige mit dem Passwort deines Computers.",
  "import.guide.opera.3": "Opera speichert die CSV-Datei im Ordner Downloads.",
  "import.guide.vivaldi.1":
    "Öffne diese Adresse in Vivaldi: Kopiere sie und füge sie in die Adressleiste ein.",
  "import.guide.vivaldi.2":
    "Klicke neben \"Passwörter exportieren\" auf \"Datei herunterladen\" und bestätige mit dem Passwort deines Computers.",
  "import.guide.vivaldi.3": "Vivaldi speichert die CSV-Datei im Ordner Downloads.",
  "import.guide.arc.1": "Öffne diese Adresse in Arc: Kopiere sie und füge sie in die Adressleiste ein.",
  "import.guide.arc.2":
    "Klicke neben \"Passwörter exportieren\" auf \"Datei herunterladen\" und bestätige mit dem Passwort deines Computers.",
  "import.guide.arc.3": "Arc speichert die CSV-Datei im Ordner Downloads.",
  "import.guide.firefox.1":
    "Öffne diese Adresse in Firefox: Kopiere sie und füge sie in die Adressleiste ein.",
  "import.guide.firefox.2":
    "Klicke oben rechts auf \"⋯\", dann auf \"Passwörter exportieren…\" und bestätige.",
  "import.guide.firefox.3": "Speichere \"logins.csv\" im Ordner Downloads.",
  "import.guide.safari.1":
    "Safari → Einstellungen → Passwörter (macOS 14 und älter; ab macOS 15 nimmst du Apple Passwörter).",
  "import.guide.safari.2": "Klicke unter der Liste auf \"⋯\", dann auf \"Alle Passwörter exportieren…\".",
  "import.guide.safari.3": "Speichere die CSV-Datei im Ordner Downloads.",
  "import.guide.applePasswords.1": "Öffne die App \"Passwörter\".",
  "import.guide.applePasswords.2": "Ablage → \"Alle Passwörter in Datei exportieren…\".",
  "import.guide.applePasswords.3": "Speichere die CSV-Datei im Ordner Downloads.",
  "import.guide.onePassword.1": "Öffne die 1Password-App und entsperre sie.",
  "import.guide.onePassword.2": "Ablage → Exportieren → dein Konto, dann gib dein Passwort ein.",
  "import.guide.onePassword.3": "Wähle \"1PUX\" und klicke auf \"Daten exportieren\".",
  "import.guide.bitwarden.1": "Im Web-Tresor oder in der App: Tools → Tresor exportieren.",
  "import.guide.bitwarden.2": "Wähle das Dateiformat \".json\" (nicht \"verschlüsselt\").",
  "import.guide.bitwarden.3": "Bestätige mit deinem Master-Passwort.",
  "import.guide.keepass.1": "Nimm deine Datenbankdatei (.kdbx) so, wie sie ist.",
  "import.guide.keepass.2": "Das Passwort der Datenbank gibst du im nächsten Schritt ein. Es bleibt auf diesem Gerät.",
  "import.guide.keepassxcCsv.1": "KeePassXC: Datenbank → Exportieren → CSV-Datei…",
  "import.guide.keepassxcCsv.2":
    "Speichere sie im Ordner Downloads. Die .kdbx-Datei enthält mehr, nimm also nach Möglichkeit \"KeePass\".",
  "import.guide.enpass.1": "Enpass: Menü → Datei → Exportieren.",
  "import.guide.enpass.2": "Wähle den Tresor, das Format \".json\" und gib dein Master-Passwort ein.",
  "import.guide.enpass.3": "Speichere die Datei im Ordner Downloads.",
  "import.guide.lastpass.1":
    "Melde dich auf lastpass.com an und öffne Erweiterte Optionen → Exportieren.",
  "import.guide.lastpass.2": "Gib dein Master-Passwort ein. Die CSV-Datei wird heruntergeladen.",
  "import.guide.lastpass.3":
    "Zeigt der Browser stattdessen Text an, speichere die Seite als \"lastpass.csv\".",
  "import.guide.dashlane.1":
    "Öffne Dashlane im Browser → Mein Konto → Einstellungen → Daten exportieren.",
  "import.guide.dashlane.2": "Wähle \"CSV\". Dashlane lädt eine ZIP-Datei herunter.",
  "import.guide.dashlane.3": "Lass die ZIP-Datei so, wie sie ist.",
  "import.guide.keeper.1": "Keeper Web Vault → Einstellungen → Exportieren.",
  "import.guide.keeper.2": "Wähle \"JSON\" (es behält eigene Felder und 2FA-Codes) und exportiere.",
  "import.guide.nordpass.1": "NordPass → Einstellungen → Einträge exportieren.",
  "import.guide.nordpass.2":
    "Bestätige mit deinem Master-Passwort und speichere die CSV-Datei im Ordner Downloads.",
  "import.guide.protonPass.1": "Proton Pass → Einstellungen → Exportieren.",
  "import.guide.protonPass.2":
    "Wähle \"ZIP\" und lass die Verschlüsselung aus. Mosaic kann den PGP-verschlüsselten Export nicht lesen.",
  "import.guide.protonPass.3": "Entsperre und speichere die Datei im Ordner Downloads.",
  "import.guide.roboform.1": "RoboForm → Einstellungen → Konto & Daten → Exportieren.",
  "import.guide.roboform.2": "Wähle \"CSV\" und \"Logins\" und speichere die Datei im Ordner Downloads.",
  "import.guide.otherFile.1":
    "Exportiere eine CSV-Datei mit Kopfzeile oder eines der oben genannten Formate.",
  "import.guide.otherFile.2": "Wähle die Datei. Mosaic ordnet die Spalten zu.",
  "import.guide.copyUrl": "Adresse kopieren",
  "import.guide.urlCopied": "Adresse kopiert.",
  "import.guide.choose": "Exportdatei auswählen",
  "import.guide.dropHint": "…oder zieh eine Datei hierher. Sie wird nur auf diesem Gerät gelesen.",
  "import.guide.back": "Zurück",
  "import.detect.detected": "Erkannt: {source}",
  "import.detect.mismatch":
    "Diese Datei sieht aus wie ein Export von {source}. Mosaic liest sie entsprechend.",
  "import.detected.chromium": "Chrome, Edge oder ein anderer Chromium-Browser",
  "import.detected.firefox": "Firefox",
  "import.detected.safari": "Safari oder Apple Passwörter",
  "import.detected.bitwardenCsv": "Bitwarden (CSV)",
  "import.detected.bitwarden": "Bitwarden",
  "import.detected.enpass": "Enpass",
  "import.detected.keepass": "KeePass",
  "import.detected.keepassxcCsv": "KeePassXC (CSV)",
  "import.detected.onePassword": "1Password",
  "import.detected.cxf": "eine FIDO-Credential-Exchange-Datei",
  "import.detected.lastpass": "LastPass",
  "import.detected.dashlane": "Dashlane",
  "import.detected.keeper": "Keeper",
  "import.detected.nordpass": "NordPass",
  "import.detected.protonPass": "Proton Pass",
  "import.detected.roboform": "RoboForm",
  "import.detected.csv": "eine CSV-Datei",
  "import.lost.title": "Kommt nicht mit",
  "import.lost.passkeys":
    "Passkeys: {source} legt sie nicht in den Export. Behalte {source}, bis du neue Passkeys eingerichtet hast.",
  "import.lost.attachments": "Anhänge: {source} legt sie nicht in den Export. Füge sie von Hand hinzu.",
  "import.lost.history": "Alte Passwörter (Verlauf): Dieser Export enthält sie nicht.",
  "import.lost.roboformExtras":
    "Identitäten, sichere Notizen und Lesezeichen: Exportiere sie getrennt. Sie gehören nicht zur Logins-Datei.",
  "import.lost.entriesSkipped":
    "{count, plural, one {# Eintrag konnte nicht gelesen werden und wird ausgelassen.} other {# Einträge konnten nicht gelesen werden und werden ausgelassen.}}",
  "import.lost.fieldsDropped":
    "Manche Einträge haben mehr Felder, als Mosaic behält. Die überzähligen Felder werden ausgelassen.",
  "import.lost.notesTruncated":
    "Manche Notizen sind länger, als ein Eintrag fassen kann. Der Teil, der nicht passt, wird ausgelassen.",
  "import.lost.attachmentsSkipped":
    "Manche Anhänge fehlen, sind zu groß oder passen nicht in deinen Anhang-Speicher. Füge sie von Hand hinzu.",
  "import.cleanup.title": "Aufräumen",
  "import.cleanup.delete": "{name} jetzt löschen",
  "import.cleanup.deleted": "{name} wurde überschrieben und gelöscht.",
  "import.cleanup.deleteFailed": "{name} konnte nicht gelöscht werden. Lösche die Datei selbst.",
  "import.cleanup.keepFile":
    "Manche Einträge wurden nicht importiert. Behalte {name}, bis du geprüft hast, was fehlt, und lösche sie dann selbst.",
  "import.cleanup.ssdNote":
    "Auf SSDs und in synchronisierten Ordnern können ältere Kopien überleben: im Versionsverlauf, in Backups oder im Papierkorb. Prüfe auch diese.",
  "import.cleanup.synced":
    "Diese Datei liegt in deinem {service}-Ordner. {service} kann frühere Versionen aufbewahren; lösche sie auch dort.",
  "import.synced.oneDrive": "OneDrive",
  "import.synced.dropbox": "Dropbox",
  "import.synced.googleDrive": "Google Drive",
  "import.synced.icloud": "iCloud",
  "import.synced.box": "Box",
  "import.checklist.title": "Bevor du deine alte App verlässt",
  "import.checklist.browserSaving":
    "Schalte in {source} das Speichern von Passwörtern und die automatische Anmeldung aus, damit es nicht neben Mosaic zum Speichern anbietet.",
  "import.checklist.firefoxSaving":
    "Öffne about:preferences#privacy und entferne den Haken bei \"Fragen, ob Zugangsdaten gespeichert werden sollen\", damit Firefox nicht neben Mosaic zum Speichern anbietet.",
  "import.checklist.safariSaving":
    "Safari → Einstellungen → AutoFill: Entferne den Haken bei \"Benutzernamen und Passwörter\", damit Safari nicht neben Mosaic ausfüllt.",
  "import.checklist.oldExtension":
    "Schalte die Browser-Erweiterung von {source} aus, damit nur eine App ausfüllt.",
  "import.checklist.deleteOld":
    "Lösche deine Daten in {source}, sobald du in Mosaic alles geprüft hast.",
} as const satisfies Record<keyof typeof en, string>;
