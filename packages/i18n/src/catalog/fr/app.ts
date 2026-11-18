/**
 * French UI strings: app. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { app as en } from "../en/app.ts";

export const app = {
  "app.name": "Mosaic",
  "app.preAudit":
    "Logiciel avant audit : pas encore examiné par un auditeur de sécurité externe.",
  "app.previewTag": "Préversion",
  "app.previewBuild":
    "Version d’aperçu, pas une version finale. Non signée, avant audit, pour essai uniquement.",
  "action.cancel": "Annuler",
  "action.continue": "Continuer",
  "action.retry": "Réessayer",
  "action.copy": "Copier",
  "action.reveal": "Afficher",
  "action.hide": "Masquer",
  "action.lock": "Verrouiller",
  "action.details": "Détails",
  "sync.offline":
    "Impossible de joindre votre serveur de synchronisation. Vos modifications sont enregistrées sur cet appareil et seront synchronisées dès que vous serez de nouveau en ligne.",
  "app.loading": "Ouverture de Mosaic…",
  "app.skipToContent": "Aller au contenu",
  "action.save": "Enregistrer",
  "action.close": "Fermer",
  "action.showPassword": "Afficher le mot de passe",
  "action.hidePassword": "Masquer le mot de passe",
  "action.back": "Retour",
  "action.confirm": "Confirmer",
  "action.edit": "Modifier",
  "action.delete": "Mettre à la corbeille",
  "action.restore": "Restaurer",
  "action.purge": "Supprimer définitivement",
  "action.generate": "Générer",
  "action.useThis": "Utiliser",
  "action.done": "Terminé",
  "nav.main": "Navigation principale",
  "nav.vault": "Coffre-fort",
  "nav.security": "Sécurité",
  "nav.generator": "Générateur",
  "nav.settings": "Réglages",
  "nav.trash": "Corbeille",
  "nav.breadcrumb": "Fil d’Ariane",
  "nav.securityCount": "{count, plural, one {# point à examiner} other {# points à examiner}}",
  "sync.status.local": "Stocké sur cet appareil",
  "sync.status.upToDate": "Synchronisé",
  "sync.status.attention": "Action requise",
  "sync.status.syncing": "Synchronisation…",
  "sync.status.offline": "Hors ligne",
  "sync.status.error": "Échec de la synchronisation",
  "action.moreActions": "Plus d’actions",
  "action.regenerate": "Régénérer",
} satisfies Record<keyof typeof en, string>;
