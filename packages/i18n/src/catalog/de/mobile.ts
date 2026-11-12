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
  // iOS (T10)
  "mobile.common.back": "Zurück",
  "mobile.common.tryAgain": "Noch einmal versuchen",
  "mobile.onboarding.welcomeTitle": "Mosaic auf diesem Handy einrichten",
  "mobile.onboarding.welcomeBody":
    "Melde dich mit dem Konto an, das du schon am Computer nutzt. Dein Master-Passwort und dein Secret Key verlassen deine Geräte nie.",
  "mobile.onboarding.preAudit": "Pre-Audit: Mosaic wurde noch nicht von außen geprüft.",
  "mobile.onboarding.start": "Los geht's",
  "mobile.onboarding.detailsTitle": "Dein Konto",
  "mobile.onboarding.server": "Serveradresse",
  "mobile.onboarding.serverPlaceholder": "https://",
  "mobile.onboarding.email": "E-Mail",
  "mobile.onboarding.secretKey": "Secret Key",
  "mobile.onboarding.secretKeyHint":
    "Du findest ihn in deinem Emergency Kit oder in den Einstellungen auf deinem Computer. Groß- und Kleinschreibung und Bindestriche spielen keine Rolle.",
  "mobile.onboarding.deviceName": "Name dieses Handys",
  "mobile.onboarding.passwordTitle": "Dein Master-Passwort",
  "mobile.onboarding.passwordHint":
    "Mosaic prüft es auf diesem Handy. Es wird nie an den Server gesendet.",
  "mobile.onboarding.signIn": "Anmelden",
  "mobile.onboarding.working": "Anmeldung läuft. Das kann ein paar Sekunden dauern.",
  "mobile.onboarding.secondFactorTitle": "Noch ein Schritt",
  "mobile.onboarding.totpLabel": "Code aus deiner Authenticator-App",
  "mobile.onboarding.verify": "Bestätigen",
  "mobile.onboarding.securityKey": "Sicherheitsschlüssel verwenden",
  "mobile.onboarding.securityKeyWaiting":
    "Schließe die Anmeldung in deinem Browser ab. Prüfe, dass dort der Code {code} steht, und komm dann hierher zurück.",
  "mobile.onboarding.autofillTitle": "Passwörter und Passkeys ausfüllen",
  "mobile.onboarding.autofillBody":
    "Schalte Mosaic in den AutoFill-Einstellungen ein, damit es Passwörter, Passkeys und Codes in Apps und in Safari ausfüllen kann.",
  "mobile.onboarding.autofillSteps":
    "Öffne die Einstellungen, dann Allgemein, dann AutoFill & Passwörter, und schalte Mosaic ein.",
  "mobile.onboarding.openSettings": "Einstellungen öffnen",
  "mobile.onboarding.notNow": "Nicht jetzt",
  "mobile.onboarding.doneTitle": "Alles bereit",
  "mobile.onboarding.doneBody": "Dein Tresor ist auf diesem Handy und entsperrt.",
  "mobile.unlock.account": "Konto: {email}",
  "mobile.unlock.secretKeyNeeded":
    "Dieses Handy kennt deinen Secret Key nicht. Gib ihn einmal ein, um zu entsperren.",
  "mobile.unlock.quickOptionFaceId": "Nächstes Mal mit Face ID entsperren",
  "mobile.unlock.biometricReason": "Deinen Mosaic-Tresor entsperren",
  "mobile.unlock.faceIdReason":
    "Mosaic nutzt Face ID, um deinen Tresor schnell zu entsperren. Dein Master-Passwort funktioniert immer auch.",
  "mobile.unlock.usePassword": "Master-Passwort verwenden",
  "mobile.unlock.working": "Entsperren läuft. Das kann ein paar Sekunden dauern.",
  "mobile.unlock.quickInvalidated":
    "Deine Face-ID-Einstellungen haben sich geändert. Entsperre mit deinem Master-Passwort, um das schnelle Entsperren wieder einzuschalten.",
  "mobile.unlock.storeBusy": "Mosaic ist beschäftigt. Versuche es gleich noch einmal.",
  "mobile.unlock.rebootNote": "Entsperre nach einem Neustart einmal mit deinem Master-Passwort.",
  "mobile.deviceLock.required.title": "Richte zuerst einen Gerätecode ein",
  "mobile.deviceLock.required.body":
    "Mosaic speichert deinen Secret Key nur auf Handys, die mit einem Code geschützt sind. Richte in den Geräteeinstellungen einen Code ein und öffne Mosaic dann noch einmal.",
  "mobile.capture.hidden": "Ausgeblendet, solange dein Bildschirm aufgezeichnet wird",
} satisfies Record<keyof typeof en, string>;
