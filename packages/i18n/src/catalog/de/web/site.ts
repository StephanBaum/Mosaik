/**
 * German UI strings for the website v2 frame. Informal "du". Same keys as English (checked by
 * the type). Copy: docs/superpowers/specs/2026-11-08-website-v2.md §1 and §5, verbatim.
 */
import type { site as en } from "../../en/web/site.ts";

export const site = {
  // Navigation
  "web.nav.security": "Sicherheit",
  "web.nav.compare": "Vergleich",
  "web.crumbs.label": "Pfad",

  // Footer
  "web.footer.product": "Produkt",
  "web.footer.start": "Loslegen",
  "web.footer.tools": "Kostenlose Hilfen",
  "web.footer.compare": "Vergleich",
  "web.footer.faq": "Fragen",
  "web.footer.download": "Download",
  "web.footer.switch": "Umzugsanleitung",
  "web.footer.verify": "Download prüfen",
  "web.footer.selfCheck": "Wie angreifbar bist du?",
  "web.footer.switchKit": "Umzugspaket",
  "web.footer.checklist": "Umzugs-Checkliste",
  "web.footer.overview": "Übersicht",
  "web.footer.vs1password": "vs 1Password",
  "web.footer.vsBitwarden": "vs Bitwarden",
  "web.footer.imprint": "Impressum",
  "web.footer.privacy": "Datenschutz",

  // Seitentitel und Beschreibungen
  "web.home.meta.title": "{name}: Jeder Login. Ein Klick. Nur deiner.",
  "web.home.meta.description":
    "{name} ist ein Passwort-Manager, der deine Logins auf deinen Geräten hält, lesbar nur für dich. Keine Werbung. Kein Tracking.",
  "web.features.meta.title": "Funktionen · {name}",
  "web.features.meta.description":
    "Autofill, das sich nicht austricksen lässt, Passkeys auf jedem Gerät, 2FA-Codes, die sich selbst eintragen, und mehr. Alles, was {name} kann.",
  "web.security.meta.title": "Sicherheit · {name}",
  "web.security.meta.description":
    "Wie {name} jeden Eintrag auf deinem Gerät verschließt, dich wählen lässt, wo er liegt, und prüft, ob der Server ehrlich bleibt.",
  "web.compare.meta.title": "{name} im Vergleich mit 1Password, Bitwarden und weiteren",
  "web.compare.meta.description":
    "Ein fairer Vergleich von {name}, 1Password, Bitwarden, Proton Pass, KeePassXC und Enpass. Jede Zelle verlinkt ihre Quelle.",
  "web.vs1password.meta.title": "{name} vs 1Password",
  "web.vs1password.meta.description":
    "Beide verschließen deinen Tresor mit zwei Geheimnissen. Der Unterschied ist, wem du vertrauen musst. Ein Vergleich mit Quellen.",
  "web.vsBitwarden.meta.title": "{name} vs Bitwarden",
  "web.vsBitwarden.meta.description":
    "Beide sind günstig zum Einstieg. Bei Voreinstellungen und Vertrauen entscheiden sie sich unterschiedlich. Ein Vergleich mit Quellen.",
  "web.pricingPage.meta.title": "Preise · {name}",
  "web.pricingPage.meta.description":
    "Die App ist kostenlos, jede Funktion auf jedem Gerät. Du zahlst nur, wenn wir die Synchronisierung betreiben sollen.",
  "web.teamsPage.meta.title": "{name} für Teams und Familien",
  "web.teamsPage.meta.description":
    "Geteilte Tresore für kleine Teams und Familien, in eurer eigenen Firmen-Cloud oder auf unserem gehosteten Server.",
  "web.downloadPage.meta.title": "{name} herunterladen",
  "web.downloadPage.meta.description":
    "Hol dir {name} kostenlos für Windows, macOS, Linux, iPhone, Android und deinen Browser. Jeder Download ist signiert.",
  "web.next.meta.title": "Deine ersten fünf Minuten · {name}",
  "web.next.meta.description": "{name} in fünf Schritten einrichten: Tresor, Emergency Kit, Import, Erweiterung und Synchronisierung.",
  "web.verify.meta.title": "So prüfst du einen Download · {name}",
  "web.verify.meta.description": "Prüf mit der SHA-256-Prüfsumme, dass ein {name}-Download wirklich von uns kommt.",
  "web.switchPage.meta.title": "In Minuten zu {name} umziehen",
  "web.switchPage.meta.description":
    "Umzug von 1Password, Bitwarden, LastPass, KeePass, Enpass oder deinem Browser. Exportieren, importieren, aufräumen.",
  "web.check.meta.title": "Wie angreifbar bist du? Ein Selbstcheck in zwei Minuten · {name}",
  "web.check.meta.description":
    "Sieben Fragen zum Ankreuzen, wie du deine Logins heute aufbewahrst. Kein Passwort wird abgefragt, nichts gespeichert, nichts gesendet.",
  "web.faqPage.meta.title": "Fragen und Antworten · {name}",
  "web.faqPage.meta.description": "Was wir sehen können, was passiert, wenn du dein Master-Passwort vergisst, und mehr.",
  "web.thanks.meta.title": "Schau in dein Postfach · {name}",
  "web.thanks.meta.description": "Bestätige deine Adresse, um das Umzugspaket zu bekommen.",
  "web.confirmed.meta.title": "Du bist dabei · {name}",
  "web.confirmed.meta.description": "Deine Adresse ist bestätigt.",
  "web.imprint.meta.title": "Impressum · {name}",
  "web.imprint.meta.description": "Wer für diese Website verantwortlich ist.",
  "web.privacy.meta.title": "Datenschutz · {name}",
  "web.privacy.meta.description": "Wie diese Website mit deinen Daten umgeht.",
  "web.notFound.meta.title": "Seite nicht gefunden · {name}",
  "web.notFound.meta.description": "Diese Seite ist in Würfel zerfallen.",

  // Capture-Karte (Umzugspaket)
  "web.capture.title": "Bald umziehen? Hol dir das Umzugspaket.",
  "web.capture.text":
    "Das komplette Paket als PDF zum Ausdrucken, dazu fünf kurze Mails, die dich Schritt für Schritt durch den Umzug führen.",
  "web.capture.label": "E-Mail-Adresse",
  "web.capture.button": "Paket schicken",
  "web.capture.privacy":
    "Du bestätigst zuerst per Mail. Kein Tracking in unseren Mails, Abmelden mit einem Klick, und wir geben deine Adresse nie weiter.",
  "web.capture.error": "Das sieht nicht nach einer E-Mail-Adresse aus.",
  "web.capture.soon.title": "Die Anmeldung per Mail startet bald.",
  "web.capture.soon.text": "Die komplette Umzugsanleitung findest du schon hier, ganz ohne Adresse.",
  "web.capture.soon.link": "Zur Umzugsanleitung",

  // Danke und bestätigt
  "web.thanks.title": "Schau in dein Postfach.",
  "web.thanks.text":
    "Wir haben dir einen Link zur Bestätigung geschickt. Er gilt 7 Tage. Nichts da? Schau im Spam-Ordner nach oder schick das Formular noch einmal, falls du dich vertippt hast.",
  "web.thanks.link": "In der Zwischenzeit: {name} herunterladen",
  "web.confirmed.title": "Du bist dabei.",
  "web.confirmed.text": "Die erste Mail ist unterwegs.",

  // Rechtliches
  "web.imprint.title": "Impressum",
  "web.privacy.title": "Datenschutz",
  "web.legal.responsible": "Verantwortlich für diese Website",
  "web.legal.address": "Anschrift",
  "web.legal.contact": "Kontakt",

  // 404
  "web.notFound.title": "Diese Seite ist in Würfel zerfallen.",
  "web.notFound.body": "Die Adresse ist vielleicht alt oder vertippt.",
  "web.notFound.home": "Startseite",
  "web.notFound.check": "Selbstcheck",
  "web.notFound.download": "Download",

  // Produktbilder und Mockups (Alternativtexte)
  "web.shot.unlock": "Der Entsperrbildschirm von {name}, der nach dem Master-Passwort fragt.",
  "web.shot.item": "Ein Login in {name} mit Benutzername, Passwort und laufendem Einmalcode.",
  "web.shot.passkeys": "Ein in {name} gespeicherter Passkey.",
  "web.shot.totp": "Ein Einmalcode in {name}, mit dem Ring, der zeigt, wie lange er gilt.",
  "web.shot.list": "Die Liste der Logins in der Desktop-App von {name}.",
  "web.shot.insights": "Die Sicherheitsübersicht von {name}: schwache, mehrfach genutzte und geleakte Passwörter.",
  "web.shot.import": "Die Import-Vorschau von {name}, mit markierten Duplikaten, bevor etwas gespeichert wird.",
  "web.teamsPage.mock.label": "Beispiel: ein geteilter Tresor mit drei Mitgliedern und dem Stand ihrer Sicherheitscodes.",
  "web.teamsPage.mock.vault": "Familie",
  "web.teamsPage.mock.count": "3 Mitglieder",
  "web.teamsPage.mock.verified": "Bestätigt",
  "web.teamsPage.mock.pending": "Code vergleichen",
  "web.security.stolen.mock": "Beispiel: Zeilen versiegelter Daten, unlesbar ohne deine zwei Schlüssel.",
} satisfies Record<keyof typeof en, string>;
