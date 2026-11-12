/** German UI strings: the Android and iOS apps (M10). Informal "du". Same keys as English (checked by the type). */
import type { mobile as en } from "../en/mobile.ts";

export const mobile = {
  "mobile.app.name": "Mosaic",
  "mobile.nav.vault": "Tresor",
  "mobile.nav.search": "Suche",
  "mobile.nav.settings": "Einstellungen",
  "mobile.common.cancel": "Abbrechen",
  "mobile.common.done": "Fertig",
  "mobile.common.close": "Schließen",
  "mobile.common.continue": "Weiter",
  "mobile.unlock.title": "Mosaic entsperren",
  "mobile.unlock.passwordLabel": "Master-Passwort",
  "mobile.unlock.action": "Entsperren",
  "mobile.unlock.quick": "Schnell entsperren",
  "mobile.unlock.failed":
    "{attempts, plural, one {Das hat nicht geklappt. # Versuch bisher.} other {Das hat nicht geklappt. # Versuche bisher.}}",
  "mobile.lock.now": "Jetzt sperren",
  "mobile.vault.title": "Dein Tresor",
  "mobile.vault.count": "{count, plural, one {# Eintrag} other {# Einträge}}",
  "mobile.vault.emptyTitle": "Noch keine Einträge",
  "mobile.vault.emptyBody":
    "Füge deinen ersten Login hinzu oder synchronisiere mit deinem Desktop, um deine Einträge hierher zu holen.",
  "mobile.search.placeholder": "Tresor durchsuchen",
  "mobile.copy.done":
    "Kopiert. Die Zwischenablage wird in {seconds, plural, one {# Sekunde} other {# Sekunden}} geleert.",
  "mobile.setup.scanTitle": "Setup-Code scannen",
  "mobile.setup.scanHint":
    "Halte die Kamera auf den Code auf deinem Computer. Mosaic liest ihn nur auf diesem Gerät.",
  "mobile.autofill.title": "Mit Mosaic ausfüllen",
  "mobile.autofill.target": "{app} ausfüllen?",
  "mobile.settings.title": "Einstellungen",
  "mobile.settings.about": "Über Mosaic",
  "mobile.settings.version": "Version {version}",
  "mobile.common.back": "Zurück",
  "mobile.common.notNow": "Jetzt nicht",
  "mobile.onboarding.step": "Schritt {current} von {total}",
  "mobile.onboarding.welcome.title": "Mosaic auf diesem Handy einrichten",
  "mobile.onboarding.welcome.body":
    "Melde dich bei deinem Mosaic-Konto an, um deine Passwörter hierher zu holen. Alles bleibt auf diesem Handy verschlüsselt.",
  "mobile.onboarding.welcome.start": "Los geht's",
  "mobile.onboarding.details.title": "Dein Konto",
  "mobile.onboarding.details.server": "Serveradresse",
  "mobile.onboarding.details.serverHint": "https://sync.example.com",
  "mobile.onboarding.details.email": "E-Mail",
  "mobile.onboarding.details.secretKey": "Secret Key",
  "mobile.onboarding.details.secretKeyHelp":
    "Aus deinem Emergency Kit: Buchstaben und Ziffern in Gruppen, beginnend mit M1.",
  "mobile.onboarding.details.deviceName": "Name dieses Handys",
  "mobile.onboarding.details.deviceNameDefault": "Android-Handy",
  "mobile.onboarding.details.checking": "Server wird geprüft",
  "mobile.onboarding.password.title": "Dein Master-Passwort",
  "mobile.onboarding.password.body":
    "Mosaic sendet es nirgendwohin. Es entsperrt deinen Tresor auf diesem Handy.",
  "mobile.onboarding.password.action": "Anmelden",
  "mobile.onboarding.password.working": "Anmeldung läuft. Das kann ein paar Sekunden dauern.",
  "mobile.onboarding.second.title": "Noch ein Schritt",
  "mobile.onboarding.second.totpLabel": "6-stelliger Code",
  "mobile.onboarding.second.totpAction": "Bestätigen",
  "mobile.onboarding.second.keyAction": "Sicherheitsschlüssel verwenden",
  "mobile.onboarding.second.keyWaiting":
    "Warte auf deinen Sicherheitsschlüssel. Schließe den Vorgang im Browser ab und komm dann zurück. Der Code dort sollte {code} lauten.",
  "mobile.onboarding.quick.title": "Schneller entsperren",
  "mobile.onboarding.quick.body":
    "Entsperre Mosaic beim nächsten Mal mit deinem Fingerabdruck. Nach einem Neustart des Handys brauchst du einmal dein Master-Passwort.",
  "mobile.onboarding.quick.enable": "Schnelles Entsperren einschalten",
  "mobile.onboarding.quick.unavailable":
    "Auf diesem Handy ist keine starke Biometrie eingerichtet, deshalb ist schnelles Entsperren nicht verfügbar.",
  "mobile.onboarding.autofill.title": "Logins automatisch ausfüllen",
  "mobile.onboarding.autofill.body":
    "Lass Mosaic deine Logins in anderen Apps und im Browser anbieten. Das kannst du später in den Systemeinstellungen ändern.",
  "mobile.onboarding.autofill.open": "Autofill-Einstellungen öffnen",
  "mobile.onboarding.done.title": "Alles bereit",
  "mobile.onboarding.done.body": "Dein Tresor ist auf diesem Handy und entsperrt.",
  "mobile.onboarding.done.action": "Tresor öffnen",
  "mobile.unlock.account": "{email} wird entsperrt",
  "mobile.unlock.secretKey": "Secret Key",
  "mobile.unlock.secretKeyHelp":
    "Auf diesem Handy ist dein Secret Key nicht gespeichert. Tippe ihn aus deinem Emergency Kit ein.",
  "mobile.unlock.enrollNext": "Nächstes Mal mit Fingerabdruck entsperren",
  "mobile.unlock.working": "Entsperren läuft. Das kann ein paar Sekunden dauern.",
  "mobile.unlock.biometricTitle": "Mosaic entsperren",
  "mobile.unlock.biometricSubtitle": "Bestätige, dass du es bist",
  "mobile.unlock.usePassword": "Master-Passwort verwenden",
  "mobile.unlock.busyTitle": "Mosaic ist beschäftigt",
  "mobile.unlock.retry": "Noch einmal versuchen",
  "mobile.home.unlockedAs": "Entsperrt als {email}",
  "error.notFound":
    "Diesen Eintrag gibt es nicht mehr. Vielleicht wurde er auf einem anderen Gerät gelöscht.",
  "error.syncFailed":
    "Mosaic konnte nicht synchronisieren. Prüfe deine Verbindung und versuche es noch einmal.",
  "error.passwordInApp":
    "Hier ist Entsperren mit dem Master-Passwort nicht möglich. Öffne die Mosaic-App und entsperre sie dort.",
  "error.folderAccountUnsupported":
    "Dieses Konto synchronisiert über einen Cloud-Ordner, den Mosaic auf Smartphones noch nicht unterstützt. Nutze einen Desktop oder ein Konto auf einem Sync-Server.",
  "error.setupCodeInvalid":
    "Dieser Setup-Code ist ungültig oder abgelaufen. Erstelle auf deinem Computer einen neuen und scanne ihn noch einmal.",
  "error.notEnoughMemory":
    "Auf diesem Gerät ist nicht genug freier Speicher, um deinen Tresor zu entsperren. Schließe andere Apps und versuche es noch einmal.",
  "error.noMatch":
    "Mosaic hat keinen Login für diese App oder Website. Öffne Mosaic, um einen hinzuzufügen.",
  "error.insecureOrigin":
    "Diese Seite nutzt keine sichere Verbindung, deshalb füllt Mosaic sie nicht aus. Öffne die https-Version der Seite.",
  "error.appNotLinked":
    "Diese App ist mit keiner gespeicherten Website verknüpft. Verknüpfe sie zuerst in Mosaic und versuche es dann noch einmal.",
  "error.foreignFrame":
    "Dieses Feld gehört zu einer anderen, in die Seite eingebetteten Website. Mosaic füllt es deshalb nicht aus.",
  "error.assetLinksUnavailable":
    "Mosaic konnte nicht prüfen, ob diese App zur Website gehört. Prüfe deine Verbindung und versuche es noch einmal.",
  "error.appNotVerified":
    "Diese App ist für die Website nicht bestätigt, deshalb nutzt Mosaic ihre Passkeys nicht. Öffne die Website stattdessen im Browser.",
  "error.rpIdInvalid":
    "Der Website-Name in dieser Anfrage ist nicht gültig. Brich ab und starte neu.",
  "error.passkeyNotFound":
    "Mosaic hat keinen Passkey für diese Website. Melde dich auf anderem Weg an.",
  "error.passkeyExcluded":
    "Du hast für dieses Konto schon einen Passkey in Mosaic. Melde dich stattdessen damit an.",
  "error.passkeyUnsupportedAlgorithm":
    "Diese Website braucht einen Passkey-Typ, den Mosaic noch nicht erstellen kann. Wähle einen anderen Weg zur Anmeldung.",
  "error.passkeySessionExpired":
    "Diese Passkey-Anfrage ist abgelaufen. Starte sie noch einmal in der App.",
  "error.qrNotFound":
    "Mosaic hat in diesem Bild keinen QR-Code gefunden. Versuche es mit einem schärferen Bild noch einmal.",
  "error.storageUnavailable":
    "Mosaic konnte den sicheren Speicher dieses Geräts nicht erreichen. Starte dein Telefon neu und versuche es noch einmal.",
} satisfies Record<keyof typeof en, string>;
