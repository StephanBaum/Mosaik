/** German UI strings: insights. Informal "du". Same keys as English (checked by the type). */
import type { insights as en } from "../en/insights.ts";

export const insights = {
  "insights.title": "Sicherheit",
  "insights.intro":
    "Die Prüfungen laufen auf diesem Gerät. Zuletzt geprüft am {checkedAt, date, medium} um {checkedAt, time, short}, {count, plural, one {# Eintrag} other {# Einträge}}.",
  "insights.checkAgain": "Erneut prüfen",
  "insights.summary": "Übersicht",
  "insights.fixFirst": "Das zuerst beheben",
  "insights.showAll": "Alle {count, plural, one {# Befund} other {# Befunde}} anzeigen",
  "insights.empty": "Keine Probleme gefunden. Mosaic prüft erneut, wenn du diese Seite öffnest.",
  "insights.kind.weak": "Schwach",
  "insights.kind.weak.description": "Leicht zu erraten",
  "insights.kind.reused": "Mehrfach verwendet",
  "insights.kind.reused.description": "{groups, plural, one {# Gruppe} other {# Gruppen}}",
  "insights.kind.old": "Alt (über 1 Jahr)",
  "insights.kind.old.description": "Seit über einem Jahr nicht geändert",
  "insights.kind.missingTotp": "2FA fehlt",
  "insights.kind.missingTotp.description":
    "Die Seite unterstützt es, aber du hast es noch nicht eingerichtet",
  "insights.kind.insecureUrl": "Unverschlüsseltes HTTP",
  "insights.kind.insecureUrl.description": "Sendet dein Passwort unverschlüsselt",
  "insights.finding.weak": "Leicht zu erraten. Ersetze es durch ein generiertes Passwort.",
  "insights.finding.reused":
    "Auch in {count, plural, one {# anderen Eintrag} other {# anderen Einträgen}} verwendet. Gib jeder Seite ein eigenes Passwort.",
  "insights.finding.old":
    "Seit über einem Jahr nicht geändert. Ändere es, wenn du das Konto noch nutzt.",
  "insights.finding.missingTotp":
    "{site} bietet Zwei-Faktor-Codes an. Schalte sie dort ein und speichere den Code in Mosaic.",
  "insights.finding.insecureUrl":
    "Nutzt unverschlüsseltes HTTP: {url}. Wechsle zu https://, wenn die Seite es unterstützt.",
  "insights.severity.high": "Hoch",
  "insights.severity.medium": "Mittel",
  "insights.severity.low": "Niedrig",
  "insights.action.changePassword": "Passwort ändern",
  "insights.action.setUpTotp": "2FA einrichten",
  "insights.action.reviewUrl": "Prüfen",
  "insights.attribution.twoFactor": "2FA-Daten: 2FA Directory (2fa.directory), MIT-Lizenz.",
  "insights.counts": "{count, plural, =0 {Keine Probleme} one {# Problem} other {# Probleme}}",
  "insights.documentation": "So schaltest du es ein: {url}",
  "insights.openItem": "{title} öffnen",
} satisfies Record<keyof typeof en, string>;
