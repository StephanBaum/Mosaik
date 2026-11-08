/** German strings of the self-check (website v2 §2.1, §5.5). */
import type { selfCheck as en } from "../../en/web/check.ts";

export const selfCheck = {
  "web.check.eyebrow": "Kostenloser Selbstcheck",
  "web.check.title": "Wie angreifbar bist du?",
  "web.check.sub":
    "Sieben Fragen, etwa zwei Minuten. Du kreuzt nur an; niemand fragt nach einem Passwort. Deine Antworten bleiben auf dieser Seite und sind weg, sobald du sie schließt.",
  "web.check.formLabel": "Fragen des Selbstchecks",
  "web.check.q1": "Wie viele Online-Konten hast du ungefähr?",
  "web.check.q1.a": "Weniger als 20",
  "web.check.q1.b": "20 bis 100",
  "web.check.q1.c": "Mehr als 100",
  "web.check.q1.d": "Keine Ahnung",
  "web.check.q2": "Nutzt du dasselbe Passwort auf mehr als einer Website?",
  "web.check.q2.a": "Nie",
  "web.check.q2.b": "Nur für ein paar unwichtige Websites",
  "web.check.q2.c": "Ja, auch für einige wichtige",
  "web.check.q2.d": "Meist dieselben paar Passwörter überall",
  "web.check.q3": "Wo bewahrst du deine Passwörter heute auf?",
  "web.check.q3.a": "In einem Passwort-Manager",
  "web.check.q3.b": "Im Browser gespeichert",
  "web.check.q3.c": "In Notizen, einer Datei oder auf Papier",
  "web.check.q3.d": "Nur im Kopf",
  "web.check.q4": "Nutzen dein E-Mail- und dein Bankkonto die Anmeldung in zwei Schritten (2FA)?",
  "web.check.q4.a": "Ja, alle",
  "web.check.q4.b": "Einige davon",
  "web.check.q4.c": "Nein",
  "web.check.q4.d": "Weiß ich nicht",
  "web.check.q5": "Wenn eine Website, die du nutzt, morgen ein Datenleck meldet, was tust du?",
  "web.check.q5.a": "Das eine Passwort ändern, fertig",
  "web.check.q5.b": "Es an mehreren Stellen ändern, weil ich es mehrfach nutze",
  "web.check.q5.c": "Ich wüsste nicht, wo ich anfangen soll",
  "web.check.q6": "Wie denkst du dir ein neues Passwort aus?",
  "web.check.q6.a": "Ein Passwort-Manager erzeugt es",
  "web.check.q6.b": "Ich denke mir jedes Mal etwas Langes und Neues aus",
  "web.check.q6.c": "Ich ändere ein bekanntes ein wenig ab",
  "web.check.q6.d": "Ich nutze jedes Mal dasselbe Muster",
  "web.check.q7": "Wann hast du zuletzt Konten geschlossen, die du nicht mehr nutzt?",
  "web.check.q7.a": "Im letzten Jahr",
  "web.check.q7.b": "Länger her",
  "web.check.q7.c": "Nie",
  "web.check.submit": "Mein Ergebnis zeigen",
  "web.check.incomplete": "Beantworte alle sieben Fragen, um dein Ergebnis zu sehen.",
  "web.check.noscript": "Das Ergebnis braucht JavaScript. Gesendet wird so oder so nichts.",
  "web.check.restart": "Neu beginnen",
  "web.check.result.eyebrow": "Dein Ergebnis",
  "web.check.low.title": "Du bist gut geschützt.",
  "web.check.low.text":
    "Das Wichtigste machst du schon. Ab hier spart dir ein Passwort-Manager vor allem Mühe und hält es auf all deinen Geräten so.",
  "web.check.mid.title": "Ein paar Türen stehen offen.",
  "web.check.mid.text":
    "Nichts Dramatisches, aber einige Gewohnheiten machen ein Datenleck teurer als nötig. Die Schritte unten schließen sie, die meisten an einem Nachmittag.",
  "web.check.high.title": "Ein Leck könnte viel erreichen.",
  "web.check.high.text":
    "Im Moment könnte ein einziges geleaktes Passwort mehrere deiner Konten öffnen. Das ist häufig, und es lässt sich beheben. Fang heute mit dem ersten Schritt unten an.",
  "web.check.next": "Deine nächsten Schritte",
  "web.check.tip.twofa":
    "Schalte zuerst für dein E-Mail-Konto die Anmeldung in zwei Schritten ein. Wer deine E-Mail kontrolliert, kann fast alles andere zurücksetzen.",
  "web.check.tip.reuse":
    "Fang mit den Konten an, die andere öffnen: zuerst deine E-Mail, dann Bank und Shopping. Gib jedem ein eigenes Passwort.",
  "web.check.tip.store":
    "Eine Notiz oder Datei kann jeder lesen, der an das Gerät oder das Konto kommt, mit dem sie synchronisiert wird. Zieh die Passwörter in einen verschlüsselten Tresor um und lösch die Datei.",
  "web.check.tip.leak":
    "Schreib dir heute deine fünf wichtigsten Konten auf. Wenn eins betroffen ist, weißt du, wo du nachsehen und was du ändern musst.",
  "web.check.tip.create":
    "Muster wie Jahreszeit plus Jahreszahl probieren Angreifer als Erstes. Lang und zufällig gewinnt, und merken musst du es dir nicht.",
  "web.check.tip.browser":
    "Im Browser gespeicherte Passwörter sind besser als Wiederverwendung. Ein Passwort-Manager bringt Passkeys, 2FA-Codes und Ausfüllen nur auf der exakten Website dazu.",
  "web.check.tip.many":
    "Bei so vielen Konten merkt sich niemand für jedes ein eigenes Passwort. Das ist eine Aufgabe für einen Passwort-Manager, nicht für dein Gedächtnis.",
  "web.check.tip.review":
    "Alte Konten halten noch deine Daten. Schließ die, die du nicht mehr nutzt, zuerst die mit einem mehrfach genutzten Passwort.",
  "web.check.tip.keep": "Bewahr dein Emergency Kit sicher auf, damit du immer in deinen Tresor zurückkommst.",
  "web.check.private": "Nichts wurde gespeichert oder gesendet. Lad die Seite neu, und deine Antworten sind weg.",
  "web.check.kit": "Hol dir das Umzugspaket",
} satisfies Record<keyof typeof en, string>;
