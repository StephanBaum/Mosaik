/** German UI strings: errors. Informal "du". Same keys as English (checked by the type). */
import type { errors as en } from "../en/errors.ts";

export const errors = {
  "error.notImplemented":
    "Das ist noch nicht verfügbar. Es kommt in einer späteren Version von Mosaic.",
  "error.noAccount":
    "Auf diesem Gerät gibt es noch kein Konto. Erstelle eins oder melde dich mit deinem Emergency Kit an.",
  "error.accountExists":
    "Auf diesem Gerät gibt es schon ein Mosaic-Konto. Entsperre es stattdessen.",
  "error.locked": "Mosaic ist gesperrt. Entsperre es, um weiterzumachen.",
  "error.alreadyUnlocked": "Mosaic ist bereits entsperrt. Du kannst direkt weitermachen.",
  "error.wrongCredentials":
    "Das Master-Passwort oder der Secret Key hat nicht funktioniert. Prüfe die Feststelltaste und dein Tastaturlayout und versuche es noch einmal.",
  "error.secretKeyRequired":
    "Dieses Gerät kennt deinen Secret Key nicht. Gib ihn aus deinem Emergency Kit ein.",
  "error.secretKeyMalformed":
    "Das ist kein Secret Key. Deiner beginnt mit „M1-“ und steht in deinem Emergency Kit.",
  "error.secretKeyMistyped":
    "Der Secret Key enthält einen Tippfehler. Vergleiche ihn Zeichen für Zeichen mit deinem Emergency Kit.",
  "error.emptyPassword": "Das Master-Passwort ist leer. Gib es ein, um weiterzumachen.",
  "error.quickUnlockUnavailable":
    "Schnelles Entsperren ist auf diesem Gerät nicht eingerichtet. Entsperre mit deinem Master-Passwort.",
  "error.storeInUse":
    "Mosaic ist auf diesem Computer schon geöffnet. Schließe das andere Fenster und versuche es dann noch einmal.",
  "error.storeUnavailable":
    "Mosaic konnte deine Tresordatei auf diesem Gerät nicht öffnen. Prüfe, ob das Laufwerk verfügbar ist und freien Speicher hat, und starte Mosaic dann neu.",
  "error.vaultNotFound":
    "Diesen Tresor gibt es nicht mehr. Vielleicht wurde er auf einem anderen Gerät gelöscht.",
  "error.vaultUnavailable":
    "Dieser Tresor lässt sich in dieser Sitzung nicht öffnen. Entsperre ihn mit seinem Hardware-Schlüssel oder synchronisiere und versuche es dann noch einmal.",
  "error.itemNotFound":
    "Diesen Eintrag gibt es nicht mehr. Vielleicht wurde er auf einem anderen Gerät verschoben oder gelöscht.",
  "error.itemDeleted":
    "Dieser Eintrag wurde endgültig gelöscht und lässt sich nicht mehr öffnen. Wähle einen anderen Eintrag.",
  "error.sameVault": "Der Eintrag liegt schon in diesem Tresor. Wähle einen anderen Tresor.",
  "error.noOtp":
    "Dieser Eintrag hat keinen Einmalcode. Bearbeite den Eintrag, um einen hinzuzufügen.",
  "error.fieldEmpty":
    "Dieses Feld ist leer, es gibt also nichts zu kopieren oder anzuzeigen. Bearbeite den Eintrag, um es auszufüllen.",
  "error.invalidInput":
    "Etwas in diesem Formular ist ungültig. Prüfe die markierten Felder und versuche es noch einmal.",
  "error.sshKeyInvalid":
    "Das ist kein privater OpenSSH-Schlüssel. Füge den ganzen Schlüssel ein, auch die BEGIN- und END-Zeilen.",
  "error.reauthRequired":
    "Diese Änderung braucht dein Master-Passwort. Gib es zur Bestätigung ein.",
  "error.attachmentTooLarge": "Diese Datei ist größer als 50 MiB. Wähle eine kleinere Datei.",
  "error.attachmentQuota":
    "Deine Dateien würden mehr als die verfügbaren 1 GiB belegen. Entferne einige Anhänge und versuche es dann noch einmal.",
  "error.attachmentNotFound":
    "Diese Datei gehört nicht mehr zum Eintrag. Lade den Eintrag neu.",
  "error.attachmentUnavailable":
    "Diese Datei lässt sich auf diesem Gerät nicht öffnen. Sie ist vielleicht noch nicht synchronisiert oder beschädigt.",
  "error.attachmentFileUnreadable":
    "Diese Datei lässt sich nicht lesen. Wähle eine andere Datei.",
  "error.attachmentSaveFailed":
    "Die Datei ließ sich dort nicht speichern. Wähle einen anderen Ordner.",
  "error.attachmentPreviewUnsupported":
    "Für diese Datei gibt es keine Vorschau. Öffne sie über Speichern unter.",
  "error.attachmentPreviewTooLarge":
    "Dieses Bild ist zu groß für die Vorschau. Öffne es über Speichern unter.",
  "error.clipboardUnavailable":
    "Mosaic konnte die Zwischenablage nicht verwenden. Schließe Apps, die sie blockieren könnten, und versuche es dann noch einmal.",
  "error.importFileUnreadable":
    "Mosaic konnte die Datei nicht lesen. Prüfe, ob sie noch existiert und nicht in einer anderen App geöffnet ist, und versuche es dann noch einmal.",
  "error.importFormatInvalid":
    "Die Datei hat nicht das gewählte Format. Wähle das passende Format oder exportiere die Datei noch einmal.",
  "error.importPreviewExpired":
    "Dieser Import wurde abgebrochen. Wähle die Datei noch einmal aus, um neu zu beginnen.",
  "error.sshRequestExpired":
    "Diese SSH-Anfrage ist schon beendet. Führe den Befehl noch einmal aus, wenn du ihn noch brauchst.",
  "error.sshAgentUnavailable":
    "Der SSH-Agent konnte nicht starten. Prüfe, dass kein anderer SSH-Agent läuft, und schalte ihn dann wieder ein.",
  "error.unlockDelayed":
    "Zu viele falsche Versuche hintereinander. Warte ein paar Sekunden und versuche es dann noch einmal.",
  "error.tooManyAttempts":
    "Zu viele falsche Antworten. Mosaic wurde gesperrt. Entsperre es mit deinem Master-Passwort, um fortzufahren.",
  "error.sshAllowTooSoon":
    "Das ging zu schnell, um die Anfrage zu lesen. Prüfe Schlüssel und Programm und wähle dann noch einmal Erlauben.",
  "error.browserRequestExpired":
    "Diese Browser-Anfrage ist bereits beendet. Frage in der Erweiterung noch einmal an, wenn du sie noch brauchst.",
  "error.browserAllowTooSoon":
    "Das ging zu schnell, um die Anfrage zu lesen. Prüfe die Angaben und wähle dann noch einmal.",
  "error.browserPairingNotFound":
    "Dieser Browser ist nicht mehr verbunden. Öffne die Einstellungen neu, um die aktuelle Liste zu sehen.",
  "error.busy":
    "Mosaic schließt noch den letzten Schritt ab. Warte kurz und versuche es dann noch einmal.",
  "error.internal":
    "Etwas Unerwartetes hat diese Aktion gestoppt. Versuche es noch einmal und starte Mosaic neu, wenn es wieder passiert.",
  "error.importTooLarge":
    "Diese Datei ist zu groß oder zu komplex für den Import. Exportiere weniger Einträge auf einmal und versuche es dann noch einmal.",
  "error.importUnsupported":
    "Mosaic kann diese Version des Formats nicht lesen. Exportiere noch einmal in einem aktuellen Format (bei KeePass: KDBX 4 mit AES oder ChaCha20) und versuche es dann erneut.",
  "error.importEncrypted":
    "Dieser Export ist mit einem Passwort geschützt. Exportiere noch einmal ohne Verschlüsselung, importiere die Datei und lösche sie danach.",
  "error.sshAgentEndpointInUse":
    "Ein anderer SSH-Agent läuft bereits (unter Windows der Dienst „OpenSSH Authentication Agent“). Beende ihn und schalte dann den SSH-Agent von Mosaic wieder ein.",
  "error.cancelled": "Abgebrochen. Es wurde nichts geändert.",
  "error.quickUnlockExpired":
    "Die Schnellentsperrung ist abgelaufen. Entsperre mit deinem Master-Passwort und schalte sie in den Einstellungen wieder ein.",
  "error.quickUnlockInvalidated":
    "Die Schnellentsperrung funktioniert auf diesem Gerät nicht mehr und wurde ausgeschaltet. Entsperre mit deinem Master-Passwort.",
  "error.passwordRequired":
    "Gib diesmal dein Master-Passwort ein. Danach funktioniert die Schnellentsperrung wieder.",
  "error.hardwareKeyUnsupported":
    "Dieser Sicherheitsschlüssel oder dieses System kann mit Mosaic nicht verwendet werden.",
  "error.hardwareKeyNotFound":
    "Kein Sicherheitsschlüssel gefunden. Stecke ihn ein und versuche es erneut.",
  "error.hardwareKeyTimeout": "Der Schlüssel wurde nicht rechtzeitig berührt. Versuche es erneut.",
  "error.hardwareKeyPinRequired": "Gib die PIN deines Sicherheitsschlüssels ein.",
  "error.hardwareKeyPinInvalid":
    "Falsche PIN. Nach mehreren Fehlversuchen sperrt sich der Schlüssel selbst.",
  "error.hardwareKeyPinBlocked":
    "Dieser Schlüssel ist nach zu vielen falschen PINs gesperrt. Setze ihn mit dem Werkzeug des Herstellers zurück; seine Registrierungen gehen dabei verloren.",
  "error.hardwareKeyNoPin":
    "Lege zuerst mit dem Werkzeug des Herstellers eine PIN für diesen Schlüssel fest.",
  "error.hardwareKeyNotRecognized":
    "Dieser Schlüssel ist hier nicht registriert. Versuche einen anderen Schlüssel oder dein Master-Passwort.",
  "error.hardwareKeyLimit":
    "Du hast die Höchstzahl von 8 Schlüsseln registriert. Entferne zuerst einen.",
  "error.hardwareKeyLast":
    "Das ist der einzige Schlüssel dieses Tresors. Registriere zuerst einen weiteren.",
  "error.itemTypeNotAllowed": "Dieser Tresor enthält nur Einmalcodes.",
  "error.otpScanUnsupported":
    "Bildschirm-Scan ist auf diesem System nicht verfügbar. Füge stattdessen einen Screenshot ein.",
  "error.otpQrNotFound":
    "Kein QR-Code gefunden. Achte darauf, dass der ganze Code sichtbar ist, und versuch es noch einmal.",
  "error.otpQrNotOtp":
    "Dieser QR-Code ist kein 2FA-Einrichtungscode. Such den Code auf der 2FA-Einrichtungsseite der Website.",
  "error.otpQrMigration":
    "Das ist ein Exportcode aus Google Authenticator, den Mosaic noch nicht lesen kann. Richte jedes Konto auf seiner Website neu ein oder gib den Einrichtungsschlüssel ein.",
  "error.otpQrMultiple":
    "Mehr als ein Einrichtungscode ist sichtbar. Zeig nur den gewünschten und versuch es noch einmal.",
  "error.otpQrInvalid":
    "Dieser Einrichtungscode nutzt Einstellungen, die Mosaic nicht unterstützt. Gib stattdessen den Einrichtungsschlüssel ein.",
  "error.otpUriInvalid":
    "Das ist kein gültiger otpauth://-Link. Kopier den ganzen Link noch einmal.",
  "error.otpKeyInvalid":
    "Dieser Schlüssel ist ungültig. Er besteht nur aus den Buchstaben A bis Z und den Ziffern 2 bis 7; prüf ihn auf Tippfehler.",
  "error.otpImageUnreadable":
    "Mosaic kann dieses Bild nicht lesen. Nimm eine PNG- oder JPEG-Datei.",
  "error.otpImageTooLarge": "Dieses Bild ist zu groß. Nimm einen Screenshot nur vom QR-Code.",
  "error.otpClipboardNoImage":
    "In der Zwischenablage ist kein Bild. Mach einen Screenshot vom QR-Code und füge ihn hier ein.",
  "error.otpPendingExpired":
    "Der gescannte Code ist nicht mehr verfügbar. Scann ihn noch einmal.",
  "error.syncNotConfigured":
    "Die Synchronisierung ist auf diesem Gerät noch nicht eingerichtet. Richte sie zuerst in den Einstellungen ein.",
  "error.syncAlreadyConfigured":
    "Die Synchronisierung ist schon eingerichtet. Trenne sie zuerst, um einen anderen Server oder Ordner zu nutzen.",
  "error.syncServerUnreachable":
    "Mosaic konnte den Server nicht erreichen. Prüfe die Adresse und deine Internetverbindung und versuche es dann noch einmal.",
  "error.syncServerInvalid":
    "Das sieht nicht nach einem Mosaic-Server aus. Prüfe die Adresse; sie sollte so aussehen: https://sync.example.com.",
  "error.syncServerInsecure":
    "Mosaic verbindet sich nur per https mit Servern, damit unterwegs niemand deine Daten lesen oder ändern kann. Nimm eine Adresse mit https://.",
  "error.syncServerMismatch":
    "Das ist nicht der Server, mit dem dieses Konto synchronisiert. Prüfe die Adresse.",
  "error.syncServerIncompatible":
    "Dieser Server läuft in einer Version, die nicht zu deinem Mosaic passt. Aktualisiere Mosaic oder bitte den Betreiber des Servers, ihn zu aktualisieren.",
  "error.syncRegistrationClosed":
    "Dieser Server nimmt keine neuen Sync-Konten an. Melde dich bei einem bestehenden an oder frag den Betreiber des Servers.",
  "error.syncInviteInvalid":
    "Dieser Einladungscode ist ungültig oder wurde schon benutzt. Prüfe ihn oder bitte den Betreiber des Servers um einen neuen.",
  "error.syncAccountExists":
    "Du hast auf diesem Server schon ein Sync-Konto. Wähle stattdessen „Anmelden“.",
  "error.syncAccountMismatch":
    "Das Sync-Konto auf diesem Server gehört zu einem anderen Mosaic-Konto. Prüfe die Adresse und deine E-Mail-Adresse.",
  "error.syncTwoFactorRequired":
    "Dieses Sync-Konto braucht einen Code aus deiner Authenticator-App. Gib den 6-stelligen Code ein.",
  "error.syncTwoFactorInvalid":
    "Dieser Code stimmt nicht oder ist abgelaufen. Gib den aktuellen 6-stelligen Code aus deiner Authenticator-App ein.",
  "error.syncSignedOut":
    "Der Server hat dieses Gerät abgemeldet. Melde dich erneut an, um weiter zu synchronisieren. Deine Änderungen sind auf diesem Gerät sicher.",
  "error.syncRateLimited":
    "Zu viele Versuche. Warte ein paar Minuten und versuche es dann noch einmal.",
  "error.syncQuotaExceeded":
    "Auf dem Server oder im Ordner ist kein Platz mehr für deine Daten. Schaffe Platz oder bitte den Betreiber des Servers um mehr.",
  "error.syncFolderUnavailable":
    "Mosaic kann diesen Ordner nicht verwenden. Prüfe, ob er existiert, ob dein Cloud-Dienst ihn fertig synchronisiert hat und ob Mosaic dort schreiben darf.",
  "error.syncFolderNotEmpty":
    "In diesem Ordner liegen andere Dateien, deshalb schreibt Mosaic dort nichts. Wähle einen leeren Ordner oder lege einen neuen an.",
  "error.syncFolderNoAccount":
    "In diesem Ordner liegen keine Sync-Daten für dieses Konto. Wähle den Ordner, mit dem dein anderes Gerät synchronisiert.",
  "error.syncFolderDamaged":
    "Einige Sync-Dateien in diesem Ordner sind beschädigt. Lass deinen Cloud-Dienst fertig synchronisieren oder stelle den Ordner aus einer Sicherung wieder her und versuche es dann noch einmal.",
  "error.syncPasswordOutcomeUnknown":
    "Wir konnten nicht bestätigen, ob der Server dein Master-Passwort geändert hat. Behalte beide Passwörter, bis der Sync bestätigt, welches gilt.",
  "error.syncPasswordNeedsServer":
    "Zum Ändern deines Master-Passworts wird dein Sync-Server gebraucht, und er ist nicht erreichbar. Prüfe deine Internetverbindung und versuche es noch einmal.",
  "error.syncDeviceIsCurrent":
    "Du kannst das Gerät, das du gerade benutzt, nicht entfernen. Nutze stattdessen Trennen.",
  "error.syncChoiceExpired":
    "Die Ordnerauswahl ist abgelaufen. Wähle den Ordner noch einmal.",
  "error.syncFreshLoginExpired":
    "Die Passwortprüfung ist abgelaufen. Gib dein Master-Passwort noch einmal ein.",
  "error.syncPasswordRollback":
    "Der Server hat ein älteres Master-Passwort angeboten. Prüfe den Server.",
  "error.syncPasswordChanged":
    "Dein Master-Passwort wurde auf dem Server oder im Ordner geändert. Gib unten das neue ein.",
} satisfies Record<keyof typeof en, string>;
