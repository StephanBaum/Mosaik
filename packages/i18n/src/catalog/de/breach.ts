/** German UI strings: the opt-in breach check (M7b). Informal "du". Same keys as English (checked by the type). */
import type { breach as en } from "../en/breach.ts";

export const breach = {
  "breach.card.title": "Leck-Prüfung",
  "breach.card.off": "Aus. Mosaic kontaktiert Have I Been Pwned nie, solange du das nicht einschaltest.",
  "breach.card.about":
    "Prüft deine Passwörter gegen bekannte Datenlecks. Pro Passwort verlässt nur ein 5-stelliges Bruchstück eines unumkehrbaren Codes dieses Gerät.",
  "breach.mode.label": "Gegen bekannte Datenlecks prüfen",
  "breach.mode.off": "Aus",
  "breach.mode.manual": "Nur wenn ich es auslöse",
  "breach.mode.weekly": "Wöchentlich, nach dem Entsperren",
  "breach.card.checkNow": "Jetzt prüfen",
  "breach.card.stop":
    "Prüfung stoppen",
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
    "Mosaic wandelt jedes Passwort in einen Code um, der sich nicht zurückrechnen lässt (einen Hash), und schickt nur dessen erste 5 Zeichen an Have I Been Pwned (api.pwnedpasswords.com). Der Dienst erfährt deine IP-Adresse und diese 5-stelligen Bruchstücke. Er sieht nie ein Passwort, deine E-Mail-Adresse oder zu welcher Website ein Passwort gehört. Die Ergebnisse bleiben auf diesem Gerät.",
  "breach.consent.detail":
    "Viele andere Passwörter teilen sich jedes dieser Bruchstücke, deshalb verrät die Antwort deines nicht. Es bleibt eine Anfrage an einen anderen Dienst, deshalb bleibt die Prüfung aus, bis du zustimmst. Du kannst sie jederzeit ausschalten.",
  "breach.consent.agree":
    "Einschalten und jetzt prüfen",
  "breach.consent.decline": "Nicht jetzt",
} as const satisfies Record<keyof typeof en, string>;
