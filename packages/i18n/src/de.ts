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

  "app.loading": "Mosaic wird geöffnet …",
  "app.skipToContent": "Zum Inhalt springen",

  "onboarding.step": "Schritt {current} von {total}",
  "onboarding.create.title": "Erstelle dein Konto",
  "onboarding.create.body":
    "Dein Master-Passwort und ein Secret Key, den wir auf diesem Gerät erzeugen, entsperren gemeinsam deinen Tresor. Keins von beiden wird jemals an den Server gesendet.",
  "onboarding.create.email": "E-Mail",
  "onboarding.create.password": "Master-Passwort",
  "onboarding.create.passwordHint":
    "Vier Wörter ohne Zusammenhang funktionieren gut. Nutze es nur für Mosaic.",
  "onboarding.create.confirm": "Master-Passwort bestätigen",
  "onboarding.create.mismatch":
    "Die beiden Passwörter stimmen nicht überein. Gib in beiden Feldern dasselbe Master-Passwort ein.",
  "onboarding.create.submit": "Konto erstellen",
  "onboarding.create.working": "Dein Konto wird erstellt. Das dauert ein paar Sekunden.",
  "onboarding.kit.accountId": "Konto-ID",
  "onboarding.kit.secretKey": "Secret Key",
  "onboarding.kit.lastGroup": "Letzte Gruppe deines Secret Keys",
  "onboarding.kit.confirm": "Bestätigen",

  "lock.secretKey": "Secret Key",
  "lock.secretKeyHint":
    "Dieses Gerät kennt deinen Secret Key noch nicht. Gib ihn aus deinem Emergency Kit ein, dann merkt Mosaic ihn sich auf diesem Gerät.",
  "lock.submit": "Entsperren",
  "lock.working": "Wird entsperrt …",

  "vault.defaultName": "Persönlich",
  "vault.unnamed": "Unbenannter Tresor",
  "vault.vaults": "Tresore",
  "vault.allItems": "Alle Einträge",
  "vault.items": "Einträge",
  "vault.search": "Tresor durchsuchen",
  "vault.untitled": "Ohne Titel",
  "vault.empty":
    "Dein Tresor ist leer. Lege deinen ersten Login an oder importiere aus einem anderen Passwortmanager.",
  "vault.noResults":
    "Nichts passt zu „{query}“. Prüfe die Schreibweise oder suche stattdessen nach einer Website.",

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
  "error.reauthRequired":
    "Diese Änderung braucht dein Master-Passwort. Gib es zur Bestätigung ein.",
  "error.attachmentTooLarge": "Diese Datei ist größer als 50 MiB. Wähle eine kleinere Datei.",
  "error.attachmentQuota":
    "Deine Dateien würden mehr als die verfügbaren 1 GiB belegen. Entferne einige Anhänge und versuche es dann noch einmal.",
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
  "error.busy":
    "Mosaic schließt noch den letzten Schritt ab. Warte kurz und versuche es dann noch einmal.",
  "error.internal":
    "Etwas Unerwartetes hat diese Aktion gestoppt. Versuche es noch einmal und starte Mosaic neu, wenn es wieder passiert.",
  "shell.emergencyKit.title": "Mosaic Emergency Kit",
  "shell.emergencyKit.intro":
    "Bewahre dieses Blatt sicher auf, zum Beispiel in einer abschließbaren Schublade. Dein Secret Key und dein Master-Passwort entsperren gemeinsam deinen Tresor auf einem neuen Gerät. Niemand, auch nicht Mosaic, kann eins von beiden für dich wiederherstellen.",
  "shell.emergencyKit.email": "E-Mail des Kontos",
  "shell.emergencyKit.accountId": "Konto-ID",
  "shell.emergencyKit.signInUrl": "Anmeldeadresse",
  "shell.emergencyKit.signInUrlLocal": "Nur dieses Gerät (noch kein Sync-Server eingerichtet)",
  "shell.emergencyKit.secretKey": "Secret Key",
  "shell.emergencyKit.masterPassword": "Master-Passwort",
  "shell.emergencyKit.masterPasswordHint":
    "Schreib es nur dann von Hand auf die Linie, wenn dieses Blatt sicher aufbewahrt bleibt. Sonst behalte es im Kopf.",
  "shell.emergencyKit.footer":
    "Mosaic ist Pre-Audit-Software: Sie wurde noch nicht von einem externen Sicherheitsprüfer begutachtet.",
  "shell.shortcut.conflict":
    "Eine andere App nutzt {shortcut} schon. Schließ diese App oder ändere ihr Tastenkürzel und starte Mosaic dann neu, um die Schnellsuche zu nutzen.",
};
