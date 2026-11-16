/**
 * French UI strings: insights. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { insights as en } from "../en/insights.ts";

export const insights = {
  "insights.title": "Sécurité",
  "insights.intro":
    "Les vérifications s’effectuent sur cet appareil. Dernière vérification le {checkedAt, date, medium} à {checkedAt, time, short} sur {count, plural, one {# élément} other {# éléments}}.",
  "insights.checkAgain": "Vérifier à nouveau",
  "insights.summary": "Résumé",
  "insights.fixFirst": "À corriger en premier",
  "insights.showAll":
    "Afficher {count, plural, one {le # point} other {les # points}}",
  "insights.empty":
    "Aucun problème détecté. Mosaic vérifie de nouveau chaque fois que vous ouvrez cette page.",
  "insights.kind.weak": "Faibles",
  "insights.kind.weak.description": "Faciles à deviner",
  "insights.kind.reused": "Réutilisés",
  "insights.kind.reused.description": "{groups, plural, one {# groupe} other {# groupes}}",
  "insights.kind.old": "Anciens (plus d’un an)",
  "insights.kind.old.description": "Inchangés depuis plus d’un an",
  "insights.kind.missingTotp": "2FA manquante",
  "insights.kind.missingTotp.description": "Le site la propose, mais vous ne l’avez pas configurée",
  "insights.kind.insecureUrl": "HTTP non sécurisé",
  "insights.kind.insecureUrl.description": "Envoie votre mot de passe sans chiffrement",
  "insights.kind.breached": "Dans une fuite",
  "insights.kind.breached.description": "Trouvé par la vérification de fuites, activée sur option",
  "insights.finding.breached":
    "Ce mot de passe apparaît dans une fuite de données connue. Changez-le dès maintenant, partout où vous l’avez utilisé.",
  "insights.finding.weak": "Facile à deviner. Remplacez-le par un mot de passe généré.",
  "insights.finding.reused":
    "Également utilisé par {count, plural, one {# autre élément} other {# autres éléments}}. Donnez à chaque site son propre mot de passe.",
  "insights.finding.old":
    "Inchangé depuis plus d’un an. Changez-le si vous utilisez toujours ce compte.",
  "insights.finding.missingTotp":
    "{site} propose des codes à deux facteurs. Activez-les sur le site et enregistrez le code dans Mosaic.",
  "insights.finding.insecureUrl":
    "Utilise du HTTP simple : {url}. Passez à https:// si le site le permet.",
  "insights.severity.high": "Élevée",
  "insights.severity.medium": "Moyenne",
  "insights.severity.low": "Faible",
  "insights.action.changePassword": "Changer le mot de passe",
  "insights.action.setUpTotp": "Configurer la 2FA",
  "insights.action.reviewUrl": "Examiner",
  "insights.attribution.twoFactor":
    "Données 2FA : 2FA Directory (2fa.directory), licence MIT.",
  "insights.counts":
    "{count, plural, =0 {Aucun problème} one {# problème} other {# problèmes}}",
  "insights.documentation": "Comment l’activer : {url}",
  "insights.actionFor": "{action} : {title}",
} satisfies Record<keyof typeof en, string>;
