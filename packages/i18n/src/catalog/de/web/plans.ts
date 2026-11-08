/** German strings new on Pricing, Teams, Download and FAQ (website v2 §5.8 to §5.11). */
import type { plans as en } from "../../en/web/plans.ts";

export const plans = {
  "web.pricingPage.teamsLink": "Für ein Team? So funktionieren geteilte Tresore.",
  "web.pricingPage.faqTitle": "Fragen zum Bezahlen",

  "web.teamsPage.storage.title": "Wo eure Team-Tresore liegen.",
  "web.teamsPage.storage.company": "Eure Firmen-Cloud: ein Dropbox-Business-Teamordner oder Nextcloud. Nichts Neues zu genehmigen.",
  "web.teamsPage.storage.hosted": "Unser gehosteter Server: keine Einrichtung, dasselbe Schloss.",
  "web.teamsPage.storage.own": "Euer eigener Server: derselbe Sync-Server, auf eurer Maschine.",
  "web.teamsPage.setup.title": "An einem Nachmittag eingerichtet.",
  "web.teamsPage.setup.s1": "{name} für alle installieren, die es brauchen.",
  "web.teamsPage.setup.s2": "Einen geteilten Tresor anlegen und wählen, wo er liegt.",
  "web.teamsPage.setup.s3": "Leute einladen und einmal die Sicherheitscodes vergleichen.",
  "web.teamsPage.setup.s4": "Wenn jemand geht, entfernen. Der Tresor bekommt ein neues Schloss.",
  "web.teamsPage.price":
    "Die Team-Preise folgen bald. Geteilte Tresore in eurem eigenen Teamordner funktionieren auch mit der kostenlosen App.",
  "web.teamsPage.contact.title": "Ihr plant die Einführung?",
  "web.teamsPage.contact.text":
    "Schreib uns, wie viele ihr seid und wo eure Dateien liegen. Wir beantworten jede Nachricht selbst.",
  "web.teamsPage.contact.button": "Schreib uns",
  "web.teamsPage.contact.capture": "Die Team-Anleitung per Mail",

  "web.downloadPage.eyebrow": "Download",
  "web.downloadPage.title": "Hol dir {name}. Kostenlos auf jedem Gerät.",
  "web.downloadPage.sub":
    "Eine App für deinen Computer, dein Handy und deinen Browser. Dein Tresor bleibt zwischen allen synchron.",
  "web.downloadPage.cta": "Für {os} herunterladen",
  "web.downloadPage.ctaAny": "Alle Downloads ansehen",
  "web.downloadPage.other": "Anderes System? Alle Downloads weiter unten.",
  "web.downloadPage.computer": "Computer",
  "web.downloadPage.phone": "Handy",
  "web.downloadPage.browser": "Browser",
  "web.downloadPage.cli": "Kommandozeile",
  "web.downloadPage.get": "Herunterladen",
  "web.downloadPage.soon": "Bald verfügbar",
  "web.downloadPage.detected": "Dein System",
  "web.downloadPage.signed.title": "Jeder Download ist signiert.",
  "web.downloadPage.signed.text":
    "Prüf vor der Installation, dass eine Datei wirklich von uns kommt. Das dauert eine Minute.",
  "web.downloadPage.signed.link": "So prüfst du einen Download",
  "web.downloadPage.next.title": "Installiert? So geht es weiter.",
  "web.downloadPage.next.link": "Deine ersten fünf Minuten",

  "web.next.title": "Deine ersten fünf Minuten.",
  "web.next.s1":
    "Installieren und Tresor anlegen. Wähl ein Master-Passwort, das du dir merken kannst: ein paar zufällige Wörter funktionieren gut.",
  "web.next.s2":
    "Emergency Kit sichern. Druck es aus oder speichere es und trag dein Master-Passwort von Hand ein. Bewahr es sicher auf. Es ist der einzige Weg zurück.",
  "web.next.s3": "Passwörter mitnehmen. Folge der Umzugsanleitung für deine alte App.",
  "web.next.s3link": "Zur Umzugsanleitung",
  "web.next.s4":
    "Browser-Erweiterung hinzufügen. Dann melde dich mit einem Klick auf einer Website an und sieh zu, wie sie ausgefüllt wird.",
  "web.next.s5":
    "Synchronisierung wählen. Dein eigener Cloud-Ordner ist kostenlos, die gehostete Synchronisierung braucht keine Einrichtung.",
  "web.next.s5link": "Zu den Tarifen",

  "web.verify.title": "So prüfst du einen Download.",
  "web.verify.text": "Zu jeder Datei gibt es eine SHA-256-Prüfsumme, direkt daneben auf der Download-Seite.",
  "web.verify.windows": "Windows (PowerShell)",
  "web.verify.unix": "macOS und Linux",
  "web.verify.file": "Datei",
  "web.verify.match":
    "Das Ergebnis muss mit unserer Prüfsumme übereinstimmen. Wenn nicht, installier die Datei nicht.",
  "web.verify.back": "Zurück zu den Downloads",

  "web.faqPage.safety": "Sicherheit",
  "web.faqPage.account": "Dein Konto",
  "web.faqPage.using": "Im Alltag",
} satisfies Record<keyof typeof en, string>;
