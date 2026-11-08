/** German strings of the Switch guide (website v2 §5.7). */
import type { switchGuide as en } from "../../en/web/switch.ts";

export const switchGuide = {
  "web.switchPage.sub":
    "Exportiere aus deiner alten App, öffne die Datei in {name}, prüf die Vorschau. Deine Passwörter funktionieren weiter, du musst nichts zurücksetzen.",
  "web.switchPage.step1.title": "Exportieren.",
  "web.switchPage.step1.text": "Speichere deinen Tresor in deiner alten App als Datei, auf deinem eigenen Computer.",
  "web.switchPage.step2.title": "Importieren.",
  "web.switchPage.step2.text":
    "Öffne die Datei in {name}. Du siehst alles, bevor etwas gespeichert wird, Duplikate sind markiert.",
  "web.switchPage.step3.title": "Aufräumen.",
  "web.switchPage.step3.text": "Lösch die Exportdatei und leere den Papierkorb. Sie ist nicht verschlüsselt.",
  "web.switchPage.pick": "Wähl die App, von der du kommst.",
  "web.switchPage.src.1password": "1Password",
  "web.switchPage.src.1password.text":
    "Wähl in der 1Password-Desktop-App Datei, dann Exportieren, und dein Konto. Speichere als 1PUX-Datei. Wähl in {name} Importieren und öffne die Datei.",
  "web.switchPage.src.bitwarden": "Bitwarden",
  "web.switchPage.src.bitwarden.text":
    "Öffne in Bitwarden Tresor exportieren und wähl das Format .json. Wähl in {name} Importieren und öffne die Datei. Passkeys kommen mit.",
  "web.switchPage.src.lastpass": "LastPass",
  "web.switchPage.src.lastpass.text":
    "Öffne in LastPass Erweiterte Optionen, dann Exportieren, und speichere die CSV-Datei. Wähl in {name} Importieren, dann CSV, und öffne die Datei.",
  "web.switchPage.src.keepass": "KeePass und KeePassXC",
  "web.switchPage.src.keepass.text":
    "Kein Export nötig. Wähl in {name} Importieren und öffne deine .kdbx-Datenbank (KDBX 4). Du gibst einmal ihr Master-Passwort ein.",
  "web.switchPage.src.enpass": "Enpass",
  "web.switchPage.src.enpass.text":
    "Wähl in Enpass Datei, dann Exportieren, und speichere als .json. Wähl in {name} Importieren und öffne die Datei.",
  "web.switchPage.src.browsers": "Chrome und andere Browser",
  "web.switchPage.src.browsers.text":
    "Öffne in Chrome den Google Passwortmanager, dann Einstellungen, dann Passwörter exportieren, und speichere die CSV-Datei. Edge und Firefox haben dieselbe Option in ihren Passwort-Einstellungen. Wähl in {name} Importieren, dann CSV, und öffne die Datei.",
  "web.switchPage.src.other": "Alles andere",
  "web.switchPage.src.other.text":
    "Fast jede App kann eine CSV-Tabelle exportieren. {name} erkennt die üblichen Spaltennamen.",
  "web.switchPage.safety.title": "Die eine heikle Minute.",
  "web.switchPage.safety.text":
    "Eine Exportdatei enthält jedes Passwort im Klartext. Erstell sie auf deinem eigenen Computer, importiere sie sofort, lösch sie dann und leere den Papierkorb. Schick sie dir nicht per Mail und leg sie nicht in einen Cloud-Ordner.",
  "web.switchPage.passkeys.title": "Passkeys kommen mit.",
  "web.switchPage.passkeys.text":
    "Aus Apps, die das neue Austauschformat für Passkeys unterstützen, und aus Exporten von Bitwarden und KeePassXC. Andere Apps halten Passkeys fest; dafür legst du nach dem Umzug auf der Website einen neuen Passkey an.",
  "web.switchPage.checklist.title": "Deine Checkliste.",
  "web.switchPage.checklist.c1": "{name} installieren und Tresor anlegen.",
  "web.switchPage.checklist.c2": "Emergency Kit drucken oder speichern und das Master-Passwort von Hand eintragen.",
  "web.switchPage.checklist.c3": "Aus der alten App exportieren.",
  "web.switchPage.checklist.c4": "In {name} importieren und die Vorschau prüfen.",
  "web.switchPage.checklist.c5": "Exportdatei löschen und Papierkorb leeren.",
  "web.switchPage.checklist.c6": "Browser-Erweiterung hinzufügen und dich mit einem Klick auf einer Website anmelden.",
  "web.switchPage.checklist.c7": "Synchronisierung wählen: dein Cloud-Ordner oder gehostet.",
  "web.switchPage.checklist.c8": "Wenn alles läuft, das alte Konto löschen.",
  "web.switchPage.checklist.print": "Checkliste drucken",
  "web.switchPage.faq.title": "Fragen zum Umzug",
  "web.switchPage.faq.q1": "Muss ich meine Passwörter ändern?",
  "web.switchPage.faq.a1":
    "Nein. Sie funktionieren weiter. {name} zeigt dir, welche schwach oder mehrfach benutzt sind, damit du die wichtigen zuerst änderst.",
  "web.switchPage.faq.q2": "Kann ich zurück?",
  "web.switchPage.faq.a2": "Ja. Du kannst deinen ganzen Tresor jederzeit exportieren.",
  "web.switchPage.faq.q3": "Was ist mit meinen 2FA-Codes?",
  "web.switchPage.faq.a3":
    "Codes aus deinem alten Passwort-Manager kommen mit dem Login mit. Codes in einer separaten Authenticator-App bleiben dort, bis du sie in {name} scannst.",
} satisfies Record<keyof typeof en, string>;
