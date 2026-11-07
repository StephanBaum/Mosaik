/** German UI strings: onboarding. Informal "du". Same keys as English (checked by the type). */
import type { onboarding as en } from "../en/onboarding.ts";

export const onboarding = {
  "onboarding.secretKey.title": "Dein Secret Key",
  "onboarding.secretKey.body":
    "Dein Secret Key und dein Master-Passwort entsperren gemeinsam deinen Tresor. Wir können keins von beiden wiederherstellen.",
  "onboarding.secretKey.verify":
    "Gib die letzte Gruppe deines Secret Keys ein, um zu bestätigen, dass du ihn gesichert hast.",
  "onboarding.secretKey.mistyped":
    "Das stimmt nicht überein. Prüfe die letzte Gruppe in deinem Emergency Kit.",
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
  "kit.title": "Sichere dein Emergency Kit",
  "kit.body":
    "Das Kit ist ein PDF mit deinem Secret Key. Zusammen mit deinem Master-Passwort öffnet es deinen Tresor auf einem neuen Gerät. Bewahre es sicher auf, getrennt von diesem Computer.",
  "kit.secretKey": "Dein Secret Key",
  "kit.save": "Emergency Kit speichern (PDF)",
  "kit.saved": "Gespeichert. Bewahre die Datei sicher auf.",
  "kit.cancelled": "Noch nicht gespeichert. Speichere das Kit, um fortzufahren.",
  "kit.recover.alreadySaved": "Ich habe es schon gesichert",
  "kit.recover.title": "Speichere dein Emergency Kit noch einmal",
  "kit.recover.body":
    "Du hast dein Emergency Kit noch nicht bestätigt. Gib dein Master-Passwort ein, um es noch einmal zu speichern, und bestätige dann die letzte Gruppe deines Secret Keys.",
  "onboarding.import.title": "Bring deine Passwörter mit",
  "onboarding.import.body":
    "Importiere jetzt aus einem anderen Passwortmanager oder später im Tresor.",
  "onboarding.import.start": "Aus einer anderen App importieren",
  "onboarding.import.skip": "Jetzt überspringen",
  "onboarding.done.title": "Alles bereit",
  "onboarding.done.body":
    "Dein Tresor ist bereit. Sperre ihn jederzeit mit der Schaltfläche „Sperren“.",
  "onboarding.browser.title": "Installiere die Browser-Erweiterung",
  "onboarding.browser.body":
    "Fülle Logins direkt im Browser aus. Die Erweiterung füllt nur, wenn du sie anklickst oder ihr Tastenkürzel benutzt.",
  "onboarding.browser.steps": "So verbindest du einen Browser",
  "onboarding.browser.step1": "Installiere die Mosaic-Erweiterung in deinem Browser.",
  "onboarding.browser.step2": "Öffne die Erweiterung und wähle „Verbinden“.",
  "onboarding.browser.step3":
    "Prüfe, ob der Code übereinstimmt, und wähle dann „Erlauben“ in dem Fenster, das sich hier öffnet.",
  "onboarding.browser.notInStores":
    "Die Erweiterung ist noch nicht in den Browser-Stores. Du kannst sie später unter Einstellungen → Browser verbinden.",
  "onboarding.browser.getFor": "Für {browser} holen",
  "onboarding.browser.connectedList": "Verbundene Browser",
  "onboarding.browser.waiting": "Warte auf die Erweiterung. Noch ist kein Browser verbunden.",
  "onboarding.browser.connected": "{name} verbunden",
  "onboarding.browser.done": "Fertig",
  "onboarding.browser.skip": "Jetzt überspringen",
  "onboarding.progress": "Einrichtungsfortschritt",
  "onboarding.stepName.create": "Konto erstellen",
  "onboarding.stepName.kit": "Emergency Kit",
  "onboarding.stepName.verify": "Kit bestätigen",
  "onboarding.stepName.import": "Import (optional)",
  "onboarding.stepName.browser": "Browser-Erweiterung (optional)",
  "kit.preview": "Vorschau des Emergency Kit",
  "kit.continueHint": "Speichere das Kit, um fortzufahren.",
  "kit.warning": "Ohne dieses Kit und dein Master-Passwort kann niemand deinen Tresor wiederherstellen, auch wir nicht.",
  "onboarding.join.link":
    "Ich nutze Mosaic schon",
  "onboarding.join.title":
    "Dieses Gerät zu deinem Konto hinzufügen",
  "onboarding.join.body":
    "Melde dich mit deinem Master-Passwort und dem Secret Key aus deinem Emergency Kit an. Danach werden deine Tresore auf dieses Gerät geladen und hier entschlüsselt.",
  "onboarding.join.tabs":
    "Wo liegen deine Sync-Daten?",
  "onboarding.join.tab.server":
    "Mosaic-Server",
  "onboarding.join.tab.folder":
    "Cloud-Ordner",
  "onboarding.join.serverUrl":
    "Serveradresse",
  "onboarding.join.email":
    "E-Mail-Adresse des Sync-Kontos",
  "onboarding.join.password":
    "Master-Passwort",
  "onboarding.join.secretKey":
    "Secret Key",
  "onboarding.join.secretKeyHint":
    "Aus deinem Emergency Kit. Er beginnt mit M1.",
  "onboarding.join.deviceName":
    "Name dieses Geräts",
  "onboarding.join.chooseFolder":
    "Ordner wählen…",
  "onboarding.join.folder":
    "Ordner",
  "onboarding.join.noAccounts":
    "In diesem Ordner liegt kein Mosaic-Konto. Wähle den Ordner, mit dem dein anderes Gerät synchronisiert.",
  "onboarding.join.account":
    "Konto",
  "onboarding.join.accountOption":
    "Konto {id}",
  "onboarding.join.submit":
    "Dieses Gerät hinzufügen",
  "onboarding.join.working":
    "Du wirst angemeldet und deine Tresore werden geladen…",
  "onboarding.join.twoFactor.title":
    "Code eingeben",
  "onboarding.join.twoFactor.body":
    "Dein Sync-Konto nutzt die Bestätigung in zwei Schritten. Gib den 6-stelligen Code aus deiner Authenticator-App ein.",
  "onboarding.join.twoFactor.code":
    "6-stelliger Code",
  "onboarding.join.twoFactor.submit":
    "Bestätigen",
  "onboarding.join.back":
    "Stattdessen ein neues Konto erstellen",
} satisfies Record<keyof typeof en, string>;
