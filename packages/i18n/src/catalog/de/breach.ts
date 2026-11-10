/** German UI strings: the opt-in breach check (M7b). Informal "du". Same keys as English (checked by the type). */
import type { breach as en } from "../en/breach.ts";

export const breach = {
  "breach.card.title": "Leck-Prüfung",
  "breach.card.off": "Aus. Mosaic kontaktiert Have I Been Pwned nie, solange du das nicht einschaltest.",
  "breach.card.about":
    "Prüft deine Passwörter gegen bekannte Datenlecks. Nur die ersten 5 Zeichen des SHA-1-Hashes jedes Passworts verlassen dieses Gerät.",
  "breach.mode.label": "Gegen bekannte Datenlecks prüfen",
  "breach.mode.off": "Aus",
  "breach.mode.manual": "Nur wenn ich es auslöse",
  "breach.mode.weekly": "Wöchentlich, nach dem Entsperren",
  "breach.card.checkNow": "Jetzt prüfen",
  "breach.card.stop": "Stopp",
  "breach.card.turnOff": "Ausschalten",

  "breach.progress.starting": "Deine Passwörter werden geprüft …",
  "breach.progress.step": "Prüfung läuft … {done} von {total}",

  "breach.result.never": "Noch nicht geprüft.",
  "breach.result.last":
    "Zuletzt geprüft am {date, date, medium}. {found, plural, =0 {Kein Passwort wurde in einem Datenleck gefunden.} one {# Passwort wurde in einem Datenleck gefunden.} other {# Passwörter wurden in einem Datenleck gefunden.}}",
  "breach.result.finished":
    "{checked, plural, one {# Passwort} other {# Passwörter}} geprüft. {found, plural, =0 {Keines wurde in einem Datenleck gefunden.} one {# wurde in einem Datenleck gefunden.} other {# wurden in einem Datenleck gefunden.}}",
  "breach.result.upToDate": "Alles wurde vor Kurzem geprüft. Es gibt nichts Neues nachzuschlagen.",

  "breach.error.offline":
    "Have I Been Pwned war nicht erreichbar. Die letzten Ergebnisse werden angezeigt. Versuche es erneut, wenn du online bist.",
  "breach.error.rateLimited":
    "Have I Been Pwned hat Mosaic gebeten, langsamer zu machen. Versuche es in ein paar Minuten erneut.",
  "breach.error.protocol":
    "Have I Been Pwned hat eine Antwort geschickt, der Mosaic nicht traut. Einige Passwörter wurden deshalb nicht geprüft. Versuche es später erneut.",
  "breach.error.cancelled": "Die Prüfung wurde gestoppt. Fertige Ergebnisse bleiben erhalten.",

  "breach.consent.title": "Deine Passwörter gegen bekannte Datenlecks prüfen?",
  "breach.consent.body":
    "Mosaic sendet die ersten 5 Zeichen des SHA-1-Hashes jedes Passworts an Have I Been Pwned (api.pwnedpasswords.com). Der Dienst sieht deine IP-Adresse und diese Präfixe, nie ein Passwort. Die Ergebnisse bleiben auf diesem Gerät.",
  "breach.consent.detail":
    "Mosaic schickt die Präfixe einzeln in zufälliger Reihenfolge und fordert aufgefüllte Antworten an. Es bleibt eine Anfrage an einen Dritten, deshalb bleibt die Prüfung aus, bis du zustimmst. Du kannst sie jederzeit wieder ausschalten.",
  "breach.consent.agree": "Jetzt prüfen",
  "breach.consent.decline": "Nicht jetzt",
} as const satisfies Record<keyof typeof en, string>;
