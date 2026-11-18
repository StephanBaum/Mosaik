/**
 * French UI strings: onboarding. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { onboarding as en } from "../en/onboarding.ts";

export const onboarding = {
  "onboarding.secretKey.title": "Votre clé secrète",
  "onboarding.secretKey.body":
    "Votre clé secrète et votre mot de passe principal, ensemble, déverrouillent votre coffre-fort. Nous ne pouvons récupérer ni l’une ni l’autre.",
  "onboarding.secretKey.verify":
    "Saisissez le dernier groupe de votre clé secrète pour confirmer que vous l’avez enregistrée.",
  "onboarding.secretKey.mistyped":
    "Cela ne correspond pas. Vérifiez le dernier groupe dans votre kit d’urgence.",
  "onboarding.step": "Étape {current} sur {total}",
  "onboarding.create.title": "Créer votre compte",
  "onboarding.create.body":
    "Votre mot de passe principal et une clé secrète générée sur cet appareil déverrouillent ensemble votre coffre-fort. Ni l’un ni l’autre n’est jamais envoyé au serveur.",
  "onboarding.create.email": "E-mail",
  "onboarding.create.password": "Mot de passe principal",
  "onboarding.create.passwordHint":
    "Quatre mots sans rapport entre eux fonctionnent bien. Utilisez-le uniquement pour Mosaic.",
  "onboarding.create.confirm": "Confirmer le mot de passe principal",
  "onboarding.create.mismatch":
    "Les deux mots de passe ne correspondent pas. Saisissez le même mot de passe principal dans les deux champs.",
  "onboarding.create.submit": "Créer le compte",
  "onboarding.create.working": "Création de votre compte. Cela prend quelques secondes.",
  "onboarding.kit.accountId": "Identifiant du compte",
  "onboarding.kit.secretKey": "Clé secrète",
  "onboarding.kit.lastGroup": "Dernier groupe de votre clé secrète",
  "onboarding.kit.confirm": "Confirmer",
  "kit.title": "Enregistrer votre kit d’urgence",
  "kit.body":
    "Le kit est un PDF contenant votre clé secrète. Avec votre mot de passe principal, il ouvre votre coffre-fort sur un nouvel appareil. Conservez-le en lieu sûr, loin de cet ordinateur.",
  "kit.secretKey": "Votre clé secrète",
  "kit.save": "Enregistrer le kit d’urgence (PDF)",
  "kit.saved": "Enregistré. Conservez le fichier en lieu sûr.",
  "kit.cancelled": "Pas encore enregistré. Enregistrez le kit pour continuer.",
  "kit.recover.alreadySaved": "Je l’ai déjà enregistré",
  "kit.recover.title": "Enregistrer de nouveau votre kit d’urgence",
  "kit.recover.body":
    "Vous n’avez pas encore confirmé votre kit d’urgence. Saisissez votre mot de passe principal pour l’enregistrer de nouveau, puis confirmez le dernier groupe de votre clé secrète.",
  "onboarding.import.title": "Emportez vos mots de passe",
  "onboarding.import.body":
    "Depuis Chrome, Edge, Firefox, Safari et d’autres navigateurs, ou depuis 1Password, Bitwarden, LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm, KeePass et Enpass. Cela prend quelques minutes. Vous pouvez aussi le faire plus tard dans les réglages.",
  "onboarding.import.start": "Importer depuis une autre application",
  "onboarding.import.skip": "Passer pour l’instant",
  "onboarding.done.title": "Tout est prêt",
  "onboarding.done.body":
    "Votre coffre-fort est prêt. Verrouillez-le à tout moment avec le bouton Verrouiller.",
  "onboarding.browser.title": "Installer l’extension de navigateur",
  "onboarding.browser.body":
    "Remplissez vos identifiants dans votre navigateur. L’extension ne remplit que lorsque vous cliquez dessus ou utilisez son raccourci clavier.",
  "onboarding.browser.steps": "Comment connecter un navigateur",
  "onboarding.browser.step1": "Installez l’extension Mosaic dans votre navigateur.",
  "onboarding.browser.step2": "Ouvrez l’extension et choisissez Connecter.",
  "onboarding.browser.step3":
    "Vérifiez que le code correspond, puis choisissez Autoriser dans la fenêtre qui s’ouvre ici.",
  "onboarding.browser.notInStores":
    "L’extension n’est pas encore dans les boutiques des navigateurs. Vous pourrez la connecter plus tard dans Réglages → Navigateurs.",
  "onboarding.browser.getFor": "Obtenir pour {browser}",
  "onboarding.browser.connectedList": "Navigateurs connectés",
  "onboarding.browser.waiting":
    "En attente de l’extension. Aucun navigateur n’est connecté pour l’instant.",
  "onboarding.browser.connected": "{name} connecté",
  "onboarding.browser.done": "Terminé",
  "onboarding.browser.skip": "Passer pour l’instant",
  "onboarding.progress": "Progression de la configuration",
  "onboarding.stepName.create": "Créer le compte",
  "onboarding.stepName.kit": "Kit d’urgence",
  "onboarding.stepName.verify": "Confirmer votre kit",
  "onboarding.stepName.import": "Importer (facultatif)",
  "onboarding.stepName.browser": "Extension de navigateur (facultatif)",
  "kit.preview": "Aperçu du kit d’urgence",
  "kit.continueHint": "Enregistrez le kit pour continuer.",
  "kit.warning":
    "Sans ce kit et votre mot de passe principal, personne ne peut récupérer votre coffre-fort, pas même nous.",
  "onboarding.join.link": "J’utilise déjà Mosaic",
  "onboarding.join.title": "Ajouter cet appareil à votre compte",
  "onboarding.join.body":
    "Connectez-vous avec votre mot de passe principal et la clé secrète de votre kit d’urgence. Vos coffres-forts sont ensuite téléchargés et déchiffrés sur cet appareil.",
  "onboarding.join.tabs": "Où se trouvent vos données de synchronisation ?",
  "onboarding.join.tab.server": "Serveur Mosaic",
  "onboarding.join.tab.folder": "Dossier cloud",
  "onboarding.join.serverUrl": "Adresse du serveur",
  "onboarding.join.email": "E-mail du compte de synchronisation",
  "onboarding.join.password": "Mot de passe principal",
  "onboarding.join.secretKey": "Clé secrète",
  "onboarding.join.secretKeyHint": "Depuis votre kit d’urgence. Elle commence par M1.",
  "onboarding.join.deviceName": "Nom de cet appareil",
  "onboarding.join.chooseFolder": "Choisir un dossier…",
  "onboarding.join.folder": "Dossier",
  "onboarding.join.noAccounts":
    "Ce dossier ne contient aucun compte Mosaic. Choisissez le dossier avec lequel votre autre appareil se synchronise.",
  "onboarding.join.account": "Compte",
  "onboarding.join.accountOption": "Compte {id}",
  "onboarding.join.submit": "Ajouter cet appareil",
  "onboarding.join.working": "Connexion et téléchargement de vos coffres-forts…",
  "onboarding.join.twoFactor.title": "Saisissez votre code",
  "onboarding.join.twoFactor.body":
    "Votre compte de synchronisation utilise la vérification en deux étapes. Saisissez le code à 6 chiffres de votre application d’authentification.",
  "onboarding.join.twoFactor.code": "Code à 6 chiffres",
  "onboarding.join.twoFactor.submit": "Vérifier",
  "onboarding.join.back": "Créer plutôt un nouveau compte",
} satisfies Record<keyof typeof en, string>;
