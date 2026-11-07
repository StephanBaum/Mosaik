/** German UI strings: sync settings and warnings (M6). Informal "du". Same keys as English (checked by the type). */
import type { sync as en } from "../en/sync.ts";

export const sync = {
  "sync.status.signedOut": "Abgemeldet",
  "sync.platform.windows": "Windows",
  "sync.platform.macos": "macOS",
  "sync.platform.linux": "Linux",

  "settings.sync.title": "Synchronisierung",
  "settings.sync.body":
    "Halte deine Tresore auf allen deinen Geräten auf dem neuesten Stand. Alles wird auf diesem Gerät verschlüsselt, bevor es das Gerät verlässt.",
  "settings.sync.notSetUp":
    "Die Synchronisierung ist aus. Deine Tresore liegen nur auf diesem Gerät.",
  "settings.sync.deviceName": "Name dieses Geräts",
  "settings.sync.deviceName.default": "Mosaic unter {platform}",
  "settings.sync.password": "Master-Passwort",
  "settings.sync.code": "6-stelliger Code",

  "settings.sync.server.title": "Mosaic-Server",
  "settings.sync.server.body":
    "Synchronisiere über einen Mosaic-Server, den du selbst betreibst oder dessen Betreiber du vertraust.",
  "settings.sync.server.url": "Serveradresse",
  "settings.sync.server.urlHint": "Zum Beispiel https://sync.example.com",
  "settings.sync.server.check": "Server prüfen",
  "settings.sync.server.checking": "Der Server wird geprüft…",
  "settings.sync.server.registration.open":
    "Auf diesem Server kann jeder ein Sync-Konto erstellen.",
  "settings.sync.server.registration.invite":
    "Für ein Sync-Konto auf diesem Server brauchst du einen Einladungscode.",
  "settings.sync.server.registration.closed":
    "Dieser Server nimmt keine neuen Sync-Konten an. Du kannst dich aber bei einem bestehenden anmelden.",
  "settings.sync.server.choice.label": "Sync-Konto",
  "settings.sync.server.choice.signIn": "Ich habe ein Sync-Konto",
  "settings.sync.server.choice.create": "Eins erstellen",
  "settings.sync.server.invite": "Einladungscode",
  "settings.sync.server.create": "Sync-Konto erstellen",
  "settings.sync.server.signIn": "Anmelden",
  "settings.sync.server.signInAgain": "Erneut anmelden",
  "settings.sync.server.working": "Verbindung wird hergestellt…",
  "settings.sync.server.sees":
    "Der Server sieht nur verschlüsselte Daten, die E-Mail-Adresse deines Kontos und wann du synchronisierst. Dein Master-Passwort und dein Secret Key bekommt er nie zu sehen.",

  "settings.sync.twoFactor.title": "Code eingeben",
  "settings.sync.twoFactor.body":
    "Dieses Sync-Konto nutzt die Bestätigung in zwei Schritten. Gib den 6-stelligen Code aus deiner Authenticator-App ein.",
  "settings.sync.twoFactor.submit": "Bestätigen",

  "settings.sync.folder.title": "Dein eigener Cloud-Ordner",
  "settings.sync.folder.body":
    "Synchronisiere über einen Ordner, den dein Cloud-Dienst (Dropbox, OneDrive, iCloud Drive und andere) aktuell hält.",
  "settings.sync.folder.choose": "Ordner wählen…",
  "settings.sync.folder.chooseOther": "Anderen Ordner wählen…",
  "settings.sync.folder.path": "Ordner",
  "settings.sync.folder.contents.empty": "Der Ordner ist leer. Mosaic kann ihn verwenden.",
  "settings.sync.folder.contents.mosaic": "In diesem Ordner liegen schon Mosaic-Sync-Daten.",
  "settings.sync.folder.contents.other":
    "In diesem Ordner liegen andere Dateien. Mosaic schreibt dort nichts. Wähle einen leeren Ordner oder lege einen neuen an.",
  "settings.sync.folder.connect": "Ordner verbinden",
  "settings.sync.folder.working": "Verbindung wird hergestellt…",
  "settings.sync.folder.sees":
    "Dein Cloud-Anbieter und alle, die Zugriff auf den Ordner haben, sehen nur verschlüsselte Daten, die Konto-ID und wann sich Dateien ändern. Dein Master-Passwort und dein Secret Key bekommen sie nie zu sehen.",

  "settings.sync.status.label": "Status",
  "settings.sync.lastSynced": "Zuletzt synchronisiert {when}",
  "settings.sync.neverSynced": "Noch nicht synchronisiert",
  "settings.sync.offline":
    "Du bist offline. Deine Änderungen sind auf diesem Gerät gespeichert und werden synchronisiert, sobald du wieder online bist.",
  "settings.sync.now": "Jetzt synchronisieren",
  "settings.sync.backend.server": "Server",
  "settings.sync.backend.account": "Sync-Konto",
  "settings.sync.backend.folder": "Ordner",
  "settings.sync.backend.device": "Dieses Gerät",
  "settings.sync.signedOut.title": "Dieses Gerät ist abgemeldet",
  "settings.sync.signedOut.body":
    "Der Server hat die Sitzung dieses Geräts beendet. Melde dich erneut an, um weiter zu synchronisieren. Deine Änderungen sind auf diesem Gerät sicher.",

  "settings.sync.recover.title": "Dein Master-Passwort wurde geändert",
  "settings.sync.recover.body":
    "Auf dem Server oder im Ordner liegt ein neueres Master-Passwort als auf diesem Gerät. Gib das neue Master-Passwort ein, um dieses Gerät auf den neuesten Stand zu bringen. Deine Tresore und dein Secret Key bleiben gleich.",
  "settings.sync.recover.password": "Neues Master-Passwort",
  "settings.sync.recover.submit": "Dieses Gerät aktualisieren",

  "settings.sync.disconnect": "Trennen…",
  "settings.sync.disconnect.title": "Synchronisierung trennen?",
  "settings.sync.disconnect.body":
    "Dieses Gerät hört auf zu synchronisieren und meldet sich beim Server ab. Deine Tresore bleiben auf diesem Gerät, und deine anderen Geräte behalten ihre eigenen Kopien.",
  "settings.sync.disconnect.confirm": "Trennen",

  "settings.sync.devices.title": "Geräte",
  "settings.sync.devices.thisDevice": "Dieses Gerät",
  "settings.sync.devices.unnamed": "Gerät ohne Namen",
  "settings.sync.devices.lastSeen": "Zuletzt gesehen {when}",
  "settings.sync.devices.neverSeen": "Noch nicht gesehen",
  "settings.sync.devices.rename": "Umbenennen",
  "settings.sync.devices.renameNamed": "{name} umbenennen",
  "settings.sync.devices.renameLabel": "Neuer Name für dieses Gerät",
  "settings.sync.devices.remove": "Entfernen",
  "settings.sync.devices.removeNamed": "{name} entfernen",
  "settings.sync.devices.removeTitle": "{name} entfernen?",
  "settings.sync.devices.removeBody":
    "Beim Entfernen wird das Gerät von der Synchronisierung abgemeldet. Die Kopie deiner Tresore, die es schon hat, nimmt ihm das nicht weg. Um ein Gerät dauerhaft auszusperren, ändere dein Master-Passwort und deinen Secret Key.",
  "settings.sync.devices.removeConfirm": "Gerät entfernen",
  "settings.sync.devices.folderNote":
    "Um ein Gerät auszusperren, ändere dein Master-Passwort und deinen Secret Key.",

  "settings.sync.totp.title": "Bestätigung in zwei Schritten",
  "settings.sync.totp.body":
    "Verlangt einen Code aus einer Authenticator-App, wenn sich jemand bei deinem Sync-Konto anmeldet. Das schützt das Konto auf dem Server, nicht deine Tresore: Die bleiben mit deinem Master-Passwort und Secret Key verschlüsselt.",
  "settings.sync.totp.on": "Die Bestätigung in zwei Schritten ist an.",
  "settings.sync.totp.off": "Die Bestätigung in zwei Schritten ist aus.",
  "settings.sync.totp.enable": "Einschalten…",
  "settings.sync.totp.disable": "Ausschalten…",
  "settings.sync.totp.beginTitle": "Bestätigung in zwei Schritten einschalten",
  "settings.sync.totp.beginBody": "Gib zum Starten dein Master-Passwort ein.",
  "settings.sync.totp.begin": "Weiter",
  "settings.sync.totp.setupBody":
    "Füge diesen Schlüssel deiner Authenticator-App hinzu und gib dann den 6-stelligen Code ein, den sie anzeigt.",
  "settings.sync.totp.secret": "Einrichtungsschlüssel",
  "settings.sync.totp.confirm": "Einschalten",
  "settings.sync.totp.disableTitle": "Bestätigung in zwei Schritten ausschalten",
  "settings.sync.totp.disableBody": "Gib dein Master-Passwort und einen aktuellen Code ein.",
  "settings.sync.totp.disableConfirm": "Ausschalten",
  "settings.sync.totp.enabledToast": "Die Bestätigung in zwei Schritten ist an.",
  "settings.sync.totp.disabledToast": "Die Bestätigung in zwei Schritten ist aus.",

  "sync.warning.region": "Sync-Warnungen",
  "sync.warning.unknownDevice": "einem anderen Gerät",
  "sync.warning.unknownVault": "einem deiner Tresore",
  "sync.warning.freezeSuspected":
    "Änderungen von {device} sind seit über einem Tag nicht angekommen. Prüfe, ob das andere Gerät an und online ist.",
  "sync.warning.missingFiles":
    "Dateien von {device} fehlen im Ordner. Lass deinen Cloud-Dienst fertig synchronisieren und synchronisiere dann erneut.",
  "sync.warning.noCommonView":
    "Seit zwei Tagen stimmen die Serverdaten auf diesem Gerät und auf {device} nicht überein. Der Server zeigt ihnen vielleicht unterschiedliche Versionen.",
  "sync.warning.rollbackDetected":
    "Deine Sync-Daten wirken älter als dieses Gerät. Prüfe den Server oder Ordner und wähle dann „Ich habe es geprüft“.",
  "sync.warning.identityDuplicated":
    "Eine andere Kopie von Mosaic schreibt unter der Identität dieses Geräts, zum Beispiel nach dem Wiederherstellen einer Sicherung. Die Synchronisierung ist pausiert. Stelle sicher, dass nur eine Kopie benutzt wird, und wähle dann „Ich habe es geprüft“.",
  "sync.warning.serverInconsistent":
    "Der Server hat diesem Gerät andere Daten gezeigt als einem anderen Gerät. Prüfe den Server und wähle dann „Ich habe es geprüft“.",
  "sync.warning.serverRollback":
    "Die Kopie von {vault} auf dem Server ist älter als die auf diesem Gerät. Prüfe den Server und wähle dann „Ich habe es geprüft“.",
  "sync.warning.undecryptable":
    "{count, plural, one {# empfangener Eintrag konnte} other {# empfangene Einträge konnten}} auf diesem Gerät nicht gelesen werden. Aktualisiere Mosaic auf allen deinen Geräten und synchronisiere dann erneut.",
  "sync.warning.hardwareVaultsLocal":
    "Hardwaregeschützte Tresore bleiben auf diesem Gerät. {count, plural, one {# Tresor wird} other {# Tresore werden}} nicht synchronisiert.",
  "sync.warning.attachmentsNotRepaired":
    "{count, plural, one {# Datei} other {# Dateien}} auf dem Server oder im Ordner {count, plural, one {konnte} other {konnten}} von diesem Gerät aus nicht repariert werden. Öffne den Eintrag auf einem Gerät, das die Datei noch hat, und füge sie erneut hinzu.",
  "sync.warning.acknowledge": "Ich habe es geprüft…",
  "sync.acknowledge.title": "Sync-Warnungen zum Prüfen ausblenden?",
  "sync.acknowledge.body":
    "Mach erst weiter, wenn du den Server oder Ordner selbst geprüft hast. Damit verschwinden alle Warnungen, die eine Prüfung brauchen, und die Synchronisierung läuft weiter. Wurden deine Daten tatsächlich zurückgesetzt oder verändert, kann Mosaic das danach nicht mehr erkennen.",
  "sync.acknowledge.eachDevice": "Mach das auf jedem deiner Geräte.",

  "toast.syncAttention":
    "Die Synchronisierung hat einige Einträge von einem anderen Gerät zusammengeführt oder repariert. Sieh nach, was sich geändert hat.",
} satisfies Record<keyof typeof en, string>;
