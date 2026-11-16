/**
 * French UI strings: settings. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { settings as en } from "../en/settings.ts";

export const settings = {
  "settings.title": "Réglages",
  "settings.security": "Sécurité",
  "settings.autoLock": "Verrouiller après inactivité",
  "settings.autoLock.never": "Jamais",
  "settings.autoLock.minutes": "{minutes, plural, one {# minute} other {# minutes}}",
  "settings.lockOnSleep": "Verrouiller quand l’ordinateur passe en veille",
  "settings.lockOnScreenLock": "Verrouiller quand l’écran se verrouille",
  "settings.screenLock.unsupported":
    "Mosaic ne peut pas savoir quand l’écran se verrouille sur ce système. Il se verrouille toujours après une période d’inactivité et au réveil de l’ordinateur.",
  "settings.lockTrigger.required":
    "Gardez au moins un verrouillage activé. Mosaic ne peut pas détecter un écran verrouillé sur ce système.",
  "settings.clipboard": "Effacer le presse-papiers après",
  "settings.clipboard.never": "Jamais",
  "settings.clipboard.seconds": "{seconds, plural, one {# seconde} other {# secondes}}",
  "settings.appearance": "Apparence",
  "settings.theme": "Thème",
  "settings.theme.system": "Suivre le système",
  "settings.theme.light": "Clair",
  "settings.theme.dark": "Sombre",
  "settings.language": "Langue",
  "settings.language.system": "Suivre le système",
  "settings.language.machineNote":
    "Traduit avec l’aide de l’IA. Si quelque chose sonne faux, dites-le-nous.",
  "settings.import": "Importer",
  "settings.import.body":
    "Récupérez vos identifiants depuis un navigateur ou un autre gestionnaire de mots de passe. Mosaic vous guide pendant l’export et vous aide à supprimer ensuite le fichier.",
  "settings.import.open": "Importer des mots de passe",
  "settings.ssh": "Agent SSH",
  "settings.ssh.enable": "Utiliser mes clés SSH depuis Mosaic",
  "settings.ssh.hint":
    "Désactivé par défaut. L’agent ne fonctionne que lorsque Mosaic est déverrouillé et vous demande avant chaque utilisation.",
  "settings.ssh.status":
    "{count, plural, =0 {Aucune clé proposée} one {# clé proposée} other {# clés proposées}}",
  "settings.ssh.endpoint": "Définir SSH_AUTH_SOCK sur",
  "settings.ssh.skipped": "Clés non proposées",
  "settings.kit": "Kit d’urgence",
  "settings.kit.save": "Enregistrer de nouveau le kit d’urgence",
  "settings.saved": "Réglages enregistrés.",
  "reauth.title": "Confirmer avec votre mot de passe principal",
  "reauth.body":
    "Cette modification rend Mosaic moins sûr, elle nécessite donc votre mot de passe principal.",
  "reauth.bodyKit":
    "Votre clé secrète va être affichée dans un fichier, Mosaic a donc besoin de votre mot de passe principal.",
  "reauth.password": "Mot de passe principal",
  "reauth.submit": "Confirmer",
  "settings.intro":
    "Comment Mosaic se verrouille, efface les secrets et protège vos clés sur cet appareil.",
  "settings.section.autoLock": "Verrouillage automatique",
  "settings.section.autoLock.body": "Quand Mosaic se verrouille de lui-même sans que vous le demandiez.",
  "settings.section.clipboard": "Presse-papiers",
  "settings.section.clipboard.body": "Les secrets copiés sont effacés après ce délai.",
  "settings.appearance.body": "Le thème et la langue de Mosaic.",
  "settings.ssh.body":
    "Permet aux programmes de se connecter avec les clés SSH de votre coffre-fort. Vous approuvez chaque utilisation.",
  "settings.kit.body":
    "Enregistrez une nouvelle copie de votre kit d’urgence. Cela nécessite votre mot de passe principal.",
  "settings.quick": "Déverrouillage rapide",
  "settings.quick.body": "Des moyens plus rapides de revenir, en plus de votre mot de passe principal.",
  "settings.quick.windowsHello": "Windows Hello",
  "settings.quick.touchId": "Touch ID",
  "settings.quick.biometric.body":
    "Déverrouillez avec votre visage, votre empreinte ou votre code PIN. Votre mot de passe principal reste nécessaire après un redémarrage.",
  "settings.quick.biometric.bodyMac":
    "Déverrouillez avec votre empreinte. Votre mot de passe principal reste nécessaire après un redémarrage.",
  "settings.quick.expires":
    "Mosaic redemandera votre mot de passe principal le {date, date, medium}.",
  "settings.quick.maxAge": "Demander le mot de passe principal au moins tous les",
  "settings.quick.maxAgeDays": "{days, plural, one {# jour} other {# jours}}",
  "settings.quick.biometricLinux": "Le déverrouillage biométrique n’est pas disponible sous Linux.",
  "settings.quick.biometricUnavailable": "Non configuré sur cet appareil.",
  "settings.quick.keys": "Clés de sécurité",
  "settings.quick.keysCount":
    "{count, plural, =0 {Aucune enregistrée} one {# enregistrée} other {# enregistrées}}",
  "settings.quick.keysUnsupported":
    "Les clés de sécurité ne sont pas encore prises en charge sur ce système.",
  "settings.quick.keyRow": "Clé de sécurité · {tag} · ajoutée le {addedAt, date, medium}",
  "settings.quick.addKey": "Ajouter une clé de sécurité",
  "settings.quick.addKeyBody":
    "Insérez la clé que vous voulez ajouter et saisissez son code PIN. Touchez-la ensuite quand elle clignote.",
  "settings.quick.manageKeys": "Gérer",
  "settings.quick.removeKey": "Retirer",
  "settings.quick.removeKeyNamed": "Retirer la clé de sécurité {tag}",
  "settings.quick.removeKeyConfirm": "Retirer la clé de sécurité {tag} ?",
  "settings.quick.keyRisk":
    "Toute personne qui possède cette clé et son code PIN peut ouvrir votre coffre-fort sans votre mot de passe principal.",
  "reauth.bodyQuick":
    "Cela facilite l’accès à Mosaic, il faut donc votre mot de passe principal.",
  "reauth.bodyKey": "Ajouter une clé à ce coffre-fort nécessite votre mot de passe principal.",
  "settings.passkeyExport": "Exporter les clés d’accès",
  "settings.passkeyExport.body":
    "Enregistrez les clés d’accès d’un coffre-fort dans un fichier FIDO CXF, pour les déplacer vers une autre application.",
  "settings.passkeyExport.vault": "Coffre-fort",
  "settings.passkeyExport.warning":
    "Le fichier contient vos clés d’accès non chiffrées. Toute personne qui possède le fichier peut se connecter à votre place. Supprimez-le après l’import.",
  "settings.passkeyExport.none": "Ce coffre-fort n’a aucune clé d’accès à exporter.",
  "settings.passkeyExport.button": "Exporter les clés d’accès",
  "settings.passkeyExport.reauth":
    "Le fichier contiendra vos clés d’accès non chiffrées, l’export nécessite donc votre mot de passe principal.",
  "settings.passkeyExport.saved":
    "{count, plural, one {# clé d’accès exportée.} other {# clés d’accès exportées.}} Supprimez le fichier après l’import.",
  "reauth.bodyMove":
    "Déplacer un élément hors d’un coffre-fort protégé par une clé de sécurité nécessite votre mot de passe principal.",
  "toast.settingsReset":
    "Vos réglages de sécurité ont été modifiés en dehors de Mosaic et ont été réinitialisés aux valeurs sûres par défaut.",
  "settings.shortcut.title": "Raccourci de recherche rapide",
  "settings.shortcut.body":
    "Appuyez sur cette combinaison dans n’importe quel programme pour ouvrir la recherche de Mosaic.",
  "settings.shortcut.label": "Raccourci",
  "settings.shortcut.hint":
    "Cliquez dans le champ, puis appuyez sur les touches voulues : Ctrl, Alt ou ⌘ (sur un Mac), plus une autre touche.",
  "settings.shortcut.save": "Enregistrer le raccourci",
  "settings.shortcut.reset": "Utiliser celui par défaut",
  "settings.shortcut.saved": "Raccourci enregistré : {shortcut}.",
  "settings.shortcut.invalid":
    "Mosaic ne peut pas utiliser ce raccourci. Appuyez sur Ctrl, Alt ou ⌘ (sur un Mac) avec une autre touche.",
  "settings.shortcut.unsupported":
    "Votre système ne permet pas aux applications de définir un raccourci global, il ne peut donc pas être modifié ici.",
} satisfies Record<keyof typeof en, string>;
