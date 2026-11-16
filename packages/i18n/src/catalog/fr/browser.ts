/**
 * French UI strings: browser. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { browser as en } from "../en/browser.ts";

export const browser = {
  "browser.pair.title": "Un navigateur veut se connecter à Mosaic",
  "browser.pair.description":
    "N’autorisez que si vous venez de cliquer sur Connecter dans l’extension Mosaic et qu’elle affiche ce code.",
  "browser.pair.code": "Code",
  "browser.pair.browser": "Navigateur",
  "browser.pair.label": "Nom de ce navigateur",
  "browser.pair.labelHint": "Vous verrez ce nom dans les réglages.",
  "browser.pair.deny": "Refuser",
  "browser.pair.allow": "Autoriser",
  "browser.expires":
    "Fermeture automatique dans {seconds, plural, one {# seconde} other {# secondes}}",
  "browser.escDenies": "pour refuser",
  "browser.hint.chrome": "Chrome",
  "browser.hint.edge": "Edge",
  "browser.hint.brave": "Brave",
  "browser.hint.opera": "Opera",
  "browser.hint.vivaldi": "Vivaldi",
  "browser.hint.chromium": "Chromium",
  "browser.hint.firefox": "Firefox",
  "browser.hint.other": "Navigateur",
  "browser.family.chromium": "Navigateur basé sur Chromium",
  "browser.family.firefox": "Firefox",
  "browser.escCancels": "pour annuler",
  "browser.save.titleNew": "Enregistrer cet identifiant dans Mosaic ?",
  "browser.save.titleUpdate": "Mettre à jour cet identifiant dans Mosaic ?",
  "browser.save.description":
    "{browser} propose d’enregistrer un identifiant. Mosaic ne l’écrit que si vous choisissez Enregistrer.",
  "browser.save.descriptionUpdate":
    "{browser} propose un nouveau mot de passe pour cet identifiant. Mosaic ne le change que si vous choisissez Mettre à jour le mot de passe.",
  "browser.save.site": "Site web",
  "browser.save.username": "Nom d’utilisateur",
  "browser.save.noUsername": "Aucun",
  "browser.save.password": "Mot de passe",
  "browser.save.passwordHidden": "Masqué",
  "browser.save.target": "Enregistrer comme",
  "browser.save.updateOption": "Mettre à jour « {title} »",
  "browser.save.newOption": "Un nouvel identifiant",
  "browser.save.itemTitle": "Titre",
  "browser.save.vault": "Coffre-fort",
  "browser.save.allowHttp": "Autoriser le remplissage sur cette page non chiffrée",
  "browser.save.allowHttpHint":
    "Le site n’utilise pas HTTPS. Toute personne sur le réseau pourrait lire ce que vous saisissez.",
  "browser.save.cancel": "Annuler",
  "browser.save.saveNew": "Enregistrer",
  "browser.save.update": "Mettre à jour le mot de passe",
  "browser.passkey.secure": "Connexion sécurisée (https)",
  "browser.passkey.getTitle": "Se connecter à {rpId} avec une clé d’accès",
  "browser.passkey.createTitle": "Enregistrer une nouvelle clé d’accès",
  "browser.passkey.excludedTitle": "Clé d’accès déjà enregistrée",
  "browser.passkey.createBody": "Créer une clé d’accès pour {rpId} dans Mosaic ?",
  "browser.passkey.excludedBody": "Vous avez déjà une clé d’accès pour {rpId} dans Mosaic.",
  "browser.passkey.uvNote":
    "Le site a demandé une vérification de l’utilisateur. Saisissez votre mot de passe principal pour confirmer que c’est bien vous.",
  "browser.passkey.password": "Mot de passe principal",
  "browser.passkey.account": "Compte",
  "browser.passkey.noName": "Sans nom",
  "browser.passkey.vault": "Coffre-fort",
  "browser.passkey.signIn": "Se connecter avec la clé d’accès",
  "browser.passkey.save": "Enregistrer la clé d’accès",
  "browser.passkey.anotherDevice": "Utiliser un autre appareil ou une clé de sécurité",
  "browser.passkey.cancel": "Annuler",
  "browser.passkey.ok": "Compris",
  "browser.passkey.escCloses": "pour fermer",
  "browser.passkey.passwordNeeded":
    "Saisissez votre mot de passe principal pour confirmer que c’est bien vous.",
  "browser.settings.title": "Navigateurs",
  "browser.settings.body":
    "Extensions de navigateur qui remplissent les identifiants depuis cette application. Vous approuvez chaque connexion ici.",
  "browser.settings.empty":
    "Aucun navigateur n’est connecté pour l’instant. Installez l’extension Mosaic, puis choisissez Connecter dans l’extension.",
  "browser.settings.connected": "Connecté actuellement",
  "browser.settings.lastUsed": "Dernière utilisation {when}",
  "browser.settings.neverUsed": "Pas encore utilisé",
  "browser.settings.disconnect": "Déconnecter",
  "browser.settings.disconnectNamed": "Déconnecter {name}",
  "browser.settings.disconnectTitle": "Déconnecter « {name} » ?",
  "browser.settings.disconnectBody":
    "Ce navigateur ne pourra plus remplir d’identifiants tant que vous ne le reconnecterez pas depuis l’extension.",
  "browser.settings.endpointInUse":
    "Un autre programme, ou une autre fenêtre Mosaic, utilise déjà la connexion pour les navigateurs. Fermez-le, puis redémarrez Mosaic.",
  "browser.settings.endpointUnavailable":
    "Mosaic n’a pas pu ouvrir la connexion pour les navigateurs. Redémarrez Mosaic. Si le problème persiste, vérifiez que votre dossier utilisateur est accessible en écriture.",
  "browser.settings.registryUnreadable":
    "Les connexions de navigateur enregistrées sont illisibles. Mosaic ne les modifiera pas tant que le problème n’est pas résolu.",
  "browser.connector.title": "Une connexion de navigateur pointe ailleurs",
  "browser.connector.body":
    "La connexion pour {browser} démarre un autre programme que Mosaic : {path}. Il peut s’agir d’une ancienne version de Mosaic, ou d’autre chose qui s’est enregistré lui-même. La réparation la fait de nouveau pointer vers ce Mosaic.",
  "browser.connector.item": "{browser} : {path}",
  "browser.connector.repair": "Réparer",
  "browser.connector.repaired": "Connexion du navigateur réparée.",
} satisfies Record<keyof typeof en, string>;
