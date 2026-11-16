/**
 * Dutch UI strings: insights. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { insights as en } from "../en/insights.ts";

export const insights = {
  "insights.title": "Beveiliging",
  "insights.intro":
    "Controles draaien op dit apparaat. Laatst gecontroleerd op {checkedAt, date, medium} om {checkedAt, time, short} over {count, plural, one {# item} other {# items}}.",
  "insights.checkAgain": "Opnieuw controleren",
  "insights.summary": "Samenvatting",
  "insights.fixFirst": "Los deze eerst op",
  "insights.showAll": "Alle {count, plural, one {# bevinding} other {# bevindingen}} tonen",
  "insights.empty": "Geen problemen gevonden. Mosaic controleert opnieuw zodra je deze pagina opent.",
  "insights.kind.weak": "Zwak",
  "insights.kind.weak.description": "Makkelijk te raden",
  "insights.kind.reused": "Hergebruikt",
  "insights.kind.reused.description": "{groups, plural, one {# groep} other {# groepen}}",
  "insights.kind.old": "Oud (ouder dan 1 jaar)",
  "insights.kind.old.description": "Al meer dan een jaar niet gewijzigd",
  "insights.kind.missingTotp": "2FA ontbreekt",
  "insights.kind.missingTotp.description": "De site ondersteunt het, maar je hebt het niet ingesteld",
  "insights.kind.insecureUrl": "Onbeveiligde HTTP",
  "insights.kind.insecureUrl.description": "Verstuurt je wachtwoord onversleuteld",
  "insights.kind.breached": "In een datalek",
  "insights.kind.breached.description": "Gevonden door de datalekcontrole waarvoor je kiest",
  "insights.finding.breached":
    "Dit wachtwoord komt voor in een bekend datalek. Wijzig het nu, overal waar je het gebruikte.",
  "insights.finding.weak": "Makkelijk te raden. Wijzig het in een gegenereerd wachtwoord.",
  "insights.finding.reused":
    "Wordt ook gebruikt door {count, plural, one {# ander item} other {# andere items}}. Geef elke site een eigen wachtwoord.",
  "insights.finding.old":
    "Al meer dan een jaar niet gewijzigd. Wijzig het als je dit account nog gebruikt.",
  "insights.finding.missingTotp":
    "{site} biedt codes voor tweestapsverificatie. Zet ze daar aan en bewaar de code in Mosaic.",
  "insights.finding.insecureUrl":
    "Gebruikt gewone HTTP: {url}. Schakel over op https:// als de site dat ondersteunt.",
  "insights.severity.high": "Hoog",
  "insights.severity.medium": "Gemiddeld",
  "insights.severity.low": "Laag",
  "insights.action.changePassword": "Wachtwoord wijzigen",
  "insights.action.setUpTotp": "2FA instellen",
  "insights.action.reviewUrl": "Bekijken",
  "insights.attribution.twoFactor": "2FA-gegevens: 2FA Directory (2fa.directory), MIT-licentie.",
  "insights.counts": "{count, plural, =0 {Geen problemen} one {# probleem} other {# problemen}}",
  "insights.documentation": "Zo zet je het aan: {url}",
  "insights.actionFor": "{action}: {title}",
} satisfies Record<keyof typeof en, string>;
