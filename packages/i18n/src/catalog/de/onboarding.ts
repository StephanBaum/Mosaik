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
  "onboarding.done.open": "Meinen Tresor öffnen",
} satisfies Record<keyof typeof en, string>;
