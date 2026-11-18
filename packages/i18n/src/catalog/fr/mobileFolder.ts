/**
 * French UI strings: mobileFolder. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { mobileFolder as en } from "../en/mobileFolder.ts";

export const mobileFolder = {
  "mobile.sync.never": "Pas encore synchronisé",
  "mobile.sync.last": "Synchronisé {time}",
  "mobile.sync.localOnly":
    "Ce coffre-fort n’existe que sur ce téléphone. La synchronisation n’est pas configurée.",
  "mobile.sync.pending":
    "{count, plural, one {# modification en attente d’envoi} other {# modifications en attente d’envoi}}",
  "mobile.sync.now": "Synchroniser maintenant",
  "mobile.sync.working": "Synchronisation",
  "mobile.sync.warnings": "Alertes de synchronisation",
  "mobile.sync.warning.freezeSuspected":
    "Les modifications d’un autre appareil ne sont pas arrivées depuis un moment. Vérifiez que l’autre appareil est allumé et en ligne.",
  "mobile.sync.warning.noCommonView":
    "Ce téléphone et un autre appareil n’ont pas vu les mêmes données du serveur depuis deux jours. Le serveur leur montre peut-être des versions différentes. Synchronisez votre autre appareil, puis synchronisez ici.",
  "mobile.sync.warning.rollbackDetected":
    "Vos données de synchronisation semblent plus anciennes que ce téléphone. Ouvrez Mosaic sur un autre appareil et vérifiez que vos dernières modifications s’y trouvent. Choisissez ensuite « J’ai vérifié ».",
  "mobile.sync.warning.identityDuplicated":
    "Une autre copie de Mosaic écrit sous l’identité de ce téléphone, par exemple après la restauration d’une sauvegarde. La synchronisation est suspendue. Assurez-vous qu’une seule copie est utilisée, puis choisissez « J’ai vérifié ».",
  "mobile.sync.warning.serverInconsistent":
    "Le serveur a montré à ce téléphone des données différentes de celles d’un autre appareil. Ouvrez Mosaic sur un autre appareil et vérifiez que vos dernières modifications s’y trouvent. Choisissez ensuite « J’ai vérifié ».",
  "mobile.sync.warning.serverRollback":
    "La copie du serveur d’un de vos coffres-forts est plus ancienne que celle de ce téléphone. Ouvrez Mosaic sur un autre appareil et vérifiez que vos dernières modifications s’y trouvent. Choisissez ensuite « J’ai vérifié ».",
  "mobile.sync.warning.sharingAttention":
    "Un coffre-fort partagé demande votre attention : un changement de clé est en attente, ou un membre n’a pas pu être vérifié. Ouvrez Mosaic sur votre ordinateur pour voir lequel. Choisissez ensuite « J’ai vérifié ».",
  "mobile.sync.warning.undecryptable":
    "{count, plural, one {# élément reçu n’a pas pu être lu} other {# éléments reçus n’ont pas pu être lus}} sur ce téléphone. Mettez Mosaic à jour sur tous vos appareils, puis synchronisez de nouveau.",
  "mobile.sync.warning.accountRollback":
    "Le serveur montre une clé de compte plus ancienne que celle de ce téléphone. Rien n’a été modifié. Ouvrez Mosaic sur un autre appareil et vérifiez que vos dernières modifications s’y trouvent. Choisissez ensuite « J’ai vérifié ».",
  "mobile.sync.acknowledge": "J’ai vérifié…",
  "mobile.sync.acknowledgeTitle": "Effacer les alertes de synchronisation ?",
  "mobile.sync.acknowledgeBody":
    "Ne continuez qu’après avoir vérifié vous-même le serveur. Cela efface toutes les alertes et la synchronisation reprend. Si vos données ont réellement été restaurées à un état antérieur ou modifiées, Mosaic ne pourra plus le détecter ensuite. Faites-le sur chacun de vos appareils.",
  "mobile.sync.acknowledgeConfirm": "J’ai vérifié",
  "mobile.folder.intro": "Ce compte se synchronise via votre dossier {provider}.",
  "mobile.folder.signIn": "Se connecter à {provider}",
  "mobile.folder.scope":
    "{provider} demandera l’accès à tous vos fichiers. Mosaic n’ouvre que le dossier que vous choisissez et ne touche à rien d’autre.",
  "mobile.folder.waiting":
    "Terminez la connexion dans votre navigateur, puis revenez ici.",
  "mobile.folder.serverLabel": "Adresse Nextcloud ou WebDAV",
  "mobile.folder.serverHint": "https://cloud.example.com",
  "mobile.folder.serverConfirm":
    "Mosaic se connectera à {server}. Ne continuez que s’il s’agit de votre propre serveur.",
  "mobile.folder.locating": "Recherche de votre dossier Mosaic…",
  "mobile.folder.browseTitle": "Choisir votre dossier Mosaic",
  "mobile.folder.browseHint":
    "C’est le dossier que votre ordinateur synchronise, avec un fichier nommé mosaic-sync.json.",
  "mobile.folder.currentFolder": "Dossier actuel : {path}",
  "mobile.folder.noSubfolders": "Il n’y a aucun dossier ici.",
  "mobile.folder.useThis": "Utiliser ce dossier",
  "mobile.folder.up": "Dossier parent",
  "mobile.folder.chooseProvider": "Où votre compte se synchronise-t-il ?",
  "mobile.folder.noDrive":
    "Google Drive et iCloud Drive ne fonctionnent pas encore sur Android. Utilisez Dropbox, OneDrive, Nextcloud ou la synchronisation Mosaic.",
  "mobile.folder.useFolder": "Synchroniser plutôt via un dossier cloud",
  "mobile.folder.providerDropbox": "Dropbox",
  "mobile.folder.providerOneDrive": "OneDrive",
  "mobile.folder.providerWebDav": "Nextcloud ou WebDAV",
  "mobile.folder.appPasswordToggle": "Utiliser plutôt un mot de passe d’application",
  "mobile.folder.davLabel": "Adresse du dossier WebDAV",
  "mobile.folder.loginLabel": "Nom d’utilisateur",
  "mobile.folder.appPasswordLabel": "Mot de passe d’application",
  "mobile.folder.appPasswordHint":
    "Créez un mot de passe d’application dans les réglages de sécurité de votre serveur. N’utilisez pas votre mot de passe principal.",
  "mobile.folder.appPasswordAction": "Se connecter avec le mot de passe d’application",
  "mobile.folder.pickAccount": "Quel compte ce téléphone rejoint-il ?",
  "mobile.folder.accountOption": "Compte {id}",
  "mobile.folder.linked": "Connecté. Synchronisation en cours.",
  "mobile.folder.settingsLine": "Se synchronise via {provider} : {path}",
  "mobile.folder.relink": "Se reconnecter",
  "mobile.folder.msRevokeHint":
    "Pour supprimer complètement l’accès de Mosaic, retirez-le sur la page de votre compte Microsoft, sous Applications.",
  "mobile.reset.revokeDropbox":
    "Le compte a été supprimé. Nous n’avons pas pu mettre fin à l’accès de Mosaic chez Dropbox : retirez-le dans Dropbox, sous Paramètres, Applications connectées.",
  "mobile.reset.revokeWebdav":
    "Le compte a été supprimé. Nous n’avons pas pu supprimer le mot de passe d’application sur votre serveur : retirez-le là-bas, sous Paramètres, Sécurité, Appareils et sessions.",
  "mobile.reset.title": "Retirer ce compte de ce téléphone",
  "mobile.reset.body":
    "Cela supprime la copie du coffre-fort, la clé secrète et la connexion au cloud stockées sur ce téléphone. Vos données dans le dossier cloud ou sur le serveur restent en place. Vous pourrez rejoindre le compte plus tard avec votre clé secrète et votre mot de passe principal.",
  "mobile.reset.action": "Retirer le compte…",
  "mobile.reset.confirm": "Retirer le compte",
  "mobile.reset.phraseLabel": "Confirmation",
  "mobile.reset.typePhrase": "Saisissez {phrase} pour confirmer",
} satisfies Record<keyof typeof en, string>;
