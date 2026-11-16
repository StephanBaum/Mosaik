/** German UI strings of the mobile apps: folder accounts and the sync line (docs/MOBILE.md §6). Informal "du". Same keys as English. */
import type { mobileFolder as en } from "../en/mobileFolder.ts";

export const mobileFolder = {
  // Sync status line.
  "mobile.sync.never": "Noch nicht synchronisiert",
  "mobile.sync.last": "Synchronisiert {time}",
  "mobile.sync.localOnly":
    "Dieser Tresor liegt nur auf diesem Handy. Die Synchronisierung ist nicht eingerichtet.",
  "mobile.sync.pending":
    "{count, plural, one {# Änderung wartet aufs Hochladen} other {# Änderungen warten aufs Hochladen}}",
  "mobile.sync.now": "Jetzt synchronisieren",
  "mobile.sync.working": "Synchronisiere",
  "mobile.sync.warnings": "Sync-Warnungen",
  "mobile.sync.warning.freezeSuspected":
    "Änderungen von einem anderen Gerät sind seit einer Weile nicht angekommen. Prüfe, ob das andere Gerät an und online ist.",
  "mobile.sync.warning.noCommonView":
    "Seit zwei Tagen stimmen die Serverdaten auf diesem Handy und auf einem anderen Gerät nicht überein. Der Server zeigt ihnen vielleicht unterschiedliche Versionen. Synchronisiere dein anderes Gerät und dann dieses.",
  "mobile.sync.warning.rollbackDetected":
    "Deine Sync-Daten wirken älter als dieses Handy. Öffne Mosaic auf einem anderen Gerät und prüfe, ob deine letzten Änderungen da sind. Wähle dann „Ich habe es geprüft“.",
  "mobile.sync.warning.identityDuplicated":
    "Eine andere Kopie von Mosaic schreibt unter der Identität dieses Handys, zum Beispiel nach dem Wiederherstellen einer Sicherung. Die Synchronisierung ist pausiert. Stelle sicher, dass nur eine Kopie benutzt wird, und wähle dann „Ich habe es geprüft“.",
  "mobile.sync.warning.serverInconsistent":
    "Der Server hat diesem Handy andere Daten gezeigt als einem anderen Gerät. Öffne Mosaic auf einem anderen Gerät und prüfe, ob deine letzten Änderungen da sind. Wähle dann „Ich habe es geprüft“.",
  "mobile.sync.warning.serverRollback":
    "Die Kopie eines deiner Tresore auf dem Server ist älter als die auf diesem Handy. Öffne Mosaic auf einem anderen Gerät und prüfe, ob deine letzten Änderungen da sind. Wähle dann „Ich habe es geprüft“.",
  "mobile.sync.warning.sharingAttention":
    "Bei einem geteilten Tresor ist etwas offen: Ein Schlüsselwechsel wartet, oder ein Mitglied konnte nicht geprüft werden. Öffne Mosaic auf deinem Computer, um zu sehen, was es ist. Wähle dann „Ich habe es geprüft“.",
  "mobile.sync.warning.undecryptable":
    "{count, plural, one {# empfangener Eintrag konnte} other {# empfangene Einträge konnten}} auf diesem Handy nicht gelesen werden. Aktualisiere Mosaic auf allen deinen Geräten und synchronisiere dann erneut.",
  "mobile.sync.warning.accountRollback":
    "Der Server zeigt einen älteren Kontoschlüssel als dieses Handy. Es wurde nichts geändert. Öffne Mosaic auf einem anderen Gerät und prüfe, ob deine letzten Änderungen da sind. Wähle dann „Ich habe es geprüft“.",
  "mobile.sync.acknowledge": "Ich habe es geprüft…",
  "mobile.sync.acknowledgeTitle": "Sync-Warnungen löschen?",
  "mobile.sync.acknowledgeBody":
    "Mach nur weiter, wenn du den Server selbst geprüft hast. Das löscht alle Warnungen, und die Synchronisierung läuft weiter. Wurden deine Daten wirklich zurückgesetzt oder verändert, kann Mosaic das danach nicht mehr erkennen. Mach das auf jedem deiner Geräte.",
  "mobile.sync.acknowledgeConfirm": "Ich habe es geprüft",
  // Android folder accounts (docs/MOBILE.md §6): sign-in to the provider, choosing the folder, settings, reset.
  "mobile.folder.intro": "Dieses Konto synchronisiert über deinen {provider}-Ordner.",
  "mobile.folder.signIn": "Bei {provider} anmelden",
  "mobile.folder.scope":
    "{provider} fragt nach Zugriff auf alle deine Dateien. Mosaic öffnet nur den Ordner, den du auswählst, und rührt sonst nichts an.",
  "mobile.folder.waiting": "Schließ die Anmeldung im Browser ab und komm dann hierher zurück.",
  "mobile.folder.serverLabel": "Nextcloud- oder WebDAV-Adresse",
  "mobile.folder.serverHint": "https://cloud.example.com",
  "mobile.folder.serverConfirm":
    "Mosaic meldet sich bei {server} an. Mach nur weiter, wenn das dein eigener Server ist.",
  "mobile.folder.locating": "Suche deinen Mosaic-Ordner …",
  "mobile.folder.browseTitle": "Wähle deinen Mosaic-Ordner",
  "mobile.folder.browseHint":
    "Es ist der Ordner, den dein Computer synchronisiert, mit einer Datei namens mosaic-sync.json.",
  "mobile.folder.currentFolder": "Aktueller Ordner: {path}",
  "mobile.folder.noSubfolders": "Hier gibt es keine Ordner.",
  "mobile.folder.useThis": "Diesen Ordner verwenden",
  "mobile.folder.up": "Eine Ebene höher",
  "mobile.folder.chooseProvider": "Wo synchronisiert dein Konto?",
  "mobile.folder.noDrive":
    "Google Drive und iCloud Drive funktionieren unter Android noch nicht. Nimm Dropbox, OneDrive, Nextcloud oder die Mosaic-Synchronisierung.",
  "mobile.folder.useFolder": "Stattdessen über einen Cloud-Ordner synchronisieren",
  "mobile.folder.providerDropbox": "Dropbox",
  "mobile.folder.providerOneDrive": "OneDrive",
  "mobile.folder.providerWebDav": "Nextcloud oder WebDAV",
  "mobile.folder.appPasswordToggle": "Stattdessen ein App-Passwort verwenden",
  "mobile.folder.davLabel": "WebDAV-Ordneradresse",
  "mobile.folder.loginLabel": "Benutzername",
  "mobile.folder.appPasswordLabel": "App-Passwort",
  "mobile.folder.appPasswordHint":
    "Erstelle ein App-Passwort in den Sicherheitseinstellungen deines Servers. Nimm nicht dein Hauptpasswort.",
  "mobile.folder.appPasswordAction": "Mit dem App-Passwort anmelden",
  "mobile.folder.pickAccount": "Welchem Konto soll dieses Handy beitreten?",
  "mobile.folder.accountOption": "Konto {id}",
  "mobile.folder.linked": "Angemeldet. Die Synchronisierung läuft wieder.",
  "mobile.folder.settingsLine": "Synchronisiert über {provider}: {path}",
  "mobile.folder.relink": "Neu anmelden",
  "mobile.folder.msRevokeHint":
    "Um Mosaics Zugriff ganz zu entfernen, entferne die App auf der Seite deines Microsoft-Kontos unter Apps.",
  "mobile.reset.revokeDropbox":
    "Das Konto wurde entfernt. Mosaics Zugriff bei Dropbox konnten wir nicht beenden: Entferne ihn in Dropbox unter Einstellungen, Verbundene Apps.",
  "mobile.reset.revokeWebdav":
    "Das Konto wurde entfernt. Das App-Passwort auf deinem Server konnten wir nicht löschen: Entferne es dort unter Einstellungen, Sicherheit, Geräte und Sitzungen.",
  "mobile.reset.title": "Dieses Konto von diesem Handy entfernen",
  "mobile.reset.body":
    "Das löscht die Tresor-Kopie, den Secret Key und die Cloud-Anmeldung auf diesem Handy. Deine Daten im Cloud-Ordner oder auf dem Server bleiben erhalten. Du kannst später mit deinem Secret Key und Master-Passwort wieder beitreten.",
  "mobile.reset.action": "Konto entfernen …",
  "mobile.reset.confirm": "Konto entfernen",
  "mobile.reset.phraseLabel": "Bestätigung",
  "mobile.reset.typePhrase": "Gib {phrase} ein, um zu bestätigen",
} satisfies Record<keyof typeof en, string>;
