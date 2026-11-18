/**
 * German strings of the report and security challenge pages. Informal "du". Same keys as
 * English (checked by the type).
 */
import type { help as en } from "../../en/web/help.ts";

export const help = {
  "web.report.meta.title": "Problem melden · {name}",
  "web.report.meta.description": "Du hast einen Fehler oder eine Sicherheitslücke in {name} gefunden? Hier meldest du sie, und das lässt du dabei weg.",
  "web.report.eyebrow": "Problem melden",
  "web.report.title": "Etwas gefunden? Sag es uns.",
  "web.report.sub": "Jede Meldung wird gelesen. Sicherheitsmeldungen gehen über einen privaten Kanal und bekommen innerhalb von sieben Tagen eine Antwort.",
  "web.report.bug.title": "Etwas funktioniert nicht",
  "web.report.bug.text":
    "Eröffne ein öffentliches Issue auf GitHub. Schreib, was du gemacht hast, was du erwartet hast und was passiert ist, dazu dein System und deine {name}-Version.",
  "web.report.bug.link": "Issue auf GitHub eröffnen",
  "web.report.vuln.title": "Du hast eine Sicherheitslücke gefunden",
  "web.report.vuln.text":
    "Bitte poste sie nicht öffentlich. Melde sie privat über GitHub. Wir bestätigen innerhalb von 7 Tagen und bewerten innerhalb von 14. Wir veröffentlichen, sobald sie behoben ist, spätestens nach 90 Tagen.",
  "web.report.vuln.link": "Privat über GitHub melden",
  "web.report.vuln.policy": "Sicherheitsrichtlinie lesen",
  "web.report.never.title": "Schick uns nie Geheimnisse",
  "web.report.never.text":
    "Keine Meldung braucht deine Passwörter, dein Master-Passwort, deinen Secret Key, dein Emergency Kit oder deine Tresordateien. Wenn ein Screenshot etwas davon zeigt, deck es vorher ab.",
  "web.report.challenge": "Du willst tiefer graben? Mach bei der Security Challenge mit.",
  "web.challenge.meta.title": "Security Challenge · {name}",
  "web.challenge.meta.description": "Versuch, {name} zu knacken. Sieben Ziele, ein öffentlicher Testserver, faire Regeln und Safe Harbor.",
  "web.challenge.eyebrow": "Security Challenge",
  "web.challenge.title": "Knack es. Wir meinen das ernst.",
  "web.challenge.sub":
    "Wir verstecken uns nicht hinter einem Siegel. Wir veröffentlichen Ziele, geben dir, was ein Angreifer hätte, und danken allen, die eine Lücke finden.",
  "web.challenge.rules": "Regeln lesen",
  "web.challenge.targetsLink": "Ziele ansehen",
  "web.challenge.open": "Der Testserver ist offen. Seine Adresse steht bei den Zielen.",
  "web.challenge.soon": "Der Testserver öffnet mit der ersten Saison. Regeln und Ziele sind schon öffentlich.",
  "web.challenge.how.title": "So läuft es",
  "web.challenge.how1": "Immer offen, in Saisons. Jede Saison startet mit frischen Daten und neuen Flags.",
  "web.challenge.how2": "Der Testserver ist getrennt und enthält keine echten Daten. Er wird jede Woche gelöscht.",
  "web.challenge.how3": "Finde eine Flag und melde, wie du sie bekommen hast. Eine Flag ohne Schritte zählt nicht.",
  "web.challenge.how4": "Safe Harbor: Forschung im Rahmen der Regeln ist willkommen. Was wir zusagen, steht genau in den Regeln.",
  "web.challenge.targets.title": "Sieben Ziele",
  "web.challenge.c1": "Feindlicher Server: Du bekommst sein komplettes Datenverzeichnis.",
  "web.challenge.c2": "Gestohlenes Gerät: ein gesperrter lokaler Speicher, ohne Schlüsselspeicher.",
  "web.challenge.c3": "Exportdatei: geschützt mit einer Passphrase aus sechs Wörtern.",
  "web.challenge.c4": "Zweiter Faktor: Passwort und Secret Key sind öffentlich, der zweite Faktor nicht.",
  "web.challenge.c5": "Fremde Konten: Lies oder ändere die Daten von jemand anderem.",
  "web.challenge.c6": "Browser-Erweiterung: Bring sie dazu, die falsche Website auszufüllen, oder ohne Klick.",
  "web.challenge.c7": "Desktop-App: Lies Tresordaten als anderer Nutzer oder aus einer Sandbox.",
  "web.challenge.scope":
    "Nicht dabei: Denial of Service, Social Engineering, physischer Zugriff, Dienste Dritter und diese Website. Fehler auf dieser Website kannst du trotzdem melden.",
  "web.challenge.reward":
    "Standardmäßig danken wir dir in der Hall of Fame, wenn du genannt werden willst. Hat eine Saison Preise, stehen sie in den Regeln.",
  "web.challenge.hallOfFame": "Hall of Fame",
  "web.challenge.report": "Fund melden",
  "web.security.challenge.title": "Glaub uns nicht einfach.",
  "web.security.challenge.text":
    "Versuch, {name} in der offenen Security Challenge zu knacken. Sieben Ziele, ein öffentlicher Testserver und ein Dankeschön für jede Lücke, die du findest.",
  "web.security.challenge.link": "Zur Challenge",
  "web.footer.report": "Problem melden",
  "web.footer.challenge": "Security Challenge",
} as const satisfies Record<keyof typeof en, string>;
