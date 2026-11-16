/**
 * French UI strings: errors. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { errors as en } from "../en/errors.ts";

export const errors = {
  "error.notImplemented":
    "Cette fonction n’est pas encore disponible. Elle arrivera dans une prochaine version de Mosaic.",
  "error.noAccount":
    "Il n’y a pas encore de compte sur cet appareil. Créez-en un, ou connectez-vous avec votre kit d’urgence.",
  "error.accountExists":
    "Cet appareil a déjà un compte Mosaic. Déverrouillez-le plutôt.",
  "error.locked": "Mosaic est verrouillé. Déverrouillez-le pour continuer.",
  "error.alreadyUnlocked":
    "Mosaic est déjà déverrouillé. Vous pouvez reprendre là où vous en étiez.",
  "error.wrongCredentials":
    "Ce mot de passe principal ou cette clé secrète n’a pas fonctionné. Vérifiez la touche Verr. maj et la disposition de votre clavier, puis réessayez.",
  "error.secretKeyRequired":
    "Cet appareil n’a pas votre clé secrète. Saisissez-la depuis votre kit d’urgence.",
  "error.secretKeyMalformed":
    "Ce n’est pas une clé secrète. La vôtre commence par « M1- » et figure sur votre kit d’urgence.",
  "error.secretKeyMistyped":
    "Cette clé secrète contient une faute de frappe. Comparez-la avec votre kit d’urgence, caractère par caractère.",
  "error.emptyPassword": "Le mot de passe principal est vide. Saisissez-le pour continuer.",
  "error.quickUnlockUnavailable":
    "Le déverrouillage rapide n’est pas configuré sur cet appareil. Déverrouillez avec votre mot de passe principal.",
  "error.storeInUse":
    "Mosaic est déjà ouvert sur cet ordinateur. Fermez l’autre fenêtre, puis réessayez.",
  "error.storeUnavailable":
    "Mosaic n’a pas pu ouvrir le fichier de votre coffre-fort sur cet appareil. Vérifiez que le disque est disponible et qu’il reste de l’espace libre, puis redémarrez Mosaic.",
  "error.vaultNotFound":
    "Ce coffre-fort n’existe plus. Il a peut-être été supprimé sur un autre appareil.",
  "error.vaultUnavailable":
    "Ce coffre-fort ne peut pas être ouvert dans cette session. Déverrouillez-le avec sa clé matérielle, ou synchronisez et réessayez.",
  "error.itemNotFound":
    "Cet élément n’existe plus. Il a peut-être été déplacé ou supprimé sur un autre appareil.",
  "error.itemDeleted":
    "Cet élément a été supprimé définitivement et ne peut donc pas être ouvert. Choisissez un autre élément.",
  "error.sameVault": "L’élément est déjà dans ce coffre-fort. Choisissez un autre coffre-fort.",
  "error.noOtp":
    "Cet élément n’a pas de code à usage unique. Modifiez l’élément pour en ajouter un.",
  "error.fieldEmpty":
    "Ce champ est vide, il n’y a donc rien à copier ni à afficher. Modifiez l’élément pour le remplir.",
  "error.invalidInput":
    "Un élément de ce formulaire n’est pas valide. Vérifiez les champs en surbrillance et réessayez.",
  "error.sshKeyInvalid":
    "Ce n’est pas une clé privée OpenSSH. Collez la clé entière, y compris les lignes BEGIN et END.",
  "error.reauthRequired":
    "Cette modification nécessite votre mot de passe principal. Saisissez-le pour confirmer.",
  "error.attachmentTooLarge": "Ce fichier dépasse 50 Mio. Choisissez un fichier plus petit.",
  "error.attachmentQuota":
    "Vos fichiers dépasseraient le 1 Gio disponible. Supprimez des pièces jointes, puis réessayez.",
  "error.attachmentNotFound":
    "Ce fichier ne fait plus partie de l’élément. Rechargez l’élément.",
  "error.attachmentUnavailable":
    "Ce fichier ne peut pas être ouvert sur cet appareil. Il n’est peut-être pas encore synchronisé ou il est endommagé.",
  "error.attachmentFileUnreadable": "Ce fichier est illisible. Choisissez un autre fichier.",
  "error.attachmentSaveFailed":
    "Le fichier n’a pas pu être enregistré à cet endroit. Choisissez un autre dossier.",
  "error.attachmentPreviewUnsupported":
    "Il n’y a pas d’aperçu pour ce fichier. Utilisez Enregistrer sous pour l’ouvrir.",
  "error.attachmentPreviewTooLarge":
    "Cette image est trop grande pour l’aperçu. Utilisez Enregistrer sous pour l’ouvrir.",
  "error.clipboardUnavailable":
    "Mosaic n’a pas pu utiliser le presse-papiers. Fermez les applications qui pourraient le retenir, puis réessayez.",
  "error.importFileUnreadable":
    "Mosaic n’a pas pu lire ce fichier. Vérifiez qu’il existe toujours et qu’il n’est pas ouvert dans une autre application, puis réessayez.",
  "error.importDeleteNotAllowed":
    "Importez d’abord le fichier. Mosaic ne supprime un export qu’une fois ses éléments dans votre coffre-fort.",
  "error.importDeleteFailed":
    "Le fichier n’a pas pu être supprimé. Il est peut-être ouvert dans une autre application ou en lecture seule. Supprimez-le vous-même, puis videz la corbeille.",
  "error.importFormatInvalid":
    "Ce fichier n’est pas au format que vous avez choisi. Choisissez le format correspondant, ou exportez de nouveau le fichier.",
  "error.importPreviewExpired":
    "Cet import a été annulé. Choisissez de nouveau le fichier pour recommencer.",
  "error.sshRequestExpired":
    "Cette demande SSH est déjà terminée. Exécutez de nouveau la commande si vous en avez encore besoin.",
  "error.sshAgentUnavailable":
    "L’agent SSH n’a pas pu démarrer. Vérifiez qu’aucun autre agent SSH n’est en cours d’exécution, puis réactivez-le.",
  "error.unlockDelayed":
    "Trop de tentatives erronées de suite. Patientez quelques secondes, puis réessayez.",
  "error.tooManyAttempts":
    "Trop de réponses erronées. Mosaic s’est verrouillé. Déverrouillez-le avec votre mot de passe principal pour continuer.",
  "error.sshAllowTooSoon":
    "C’était trop rapide pour lire la demande. Vérifiez la clé et le programme, puis choisissez de nouveau Autoriser.",
  "error.browserRequestExpired":
    "Cette demande du navigateur est déjà terminée. Redemandez depuis l’extension si vous en avez encore besoin.",
  "error.browserAllowTooSoon":
    "C’était trop rapide pour lire la demande. Vérifiez les détails, puis choisissez de nouveau.",
  "error.browserPairingNotFound":
    "Ce navigateur n’est plus connecté. Rouvrez les réglages pour voir la liste actuelle.",
  "error.browserHostMissing":
    "Cette copie de Mosaic n’a pas de connecteur de navigateur installé, il n’y a donc rien à réparer. Installez Mosaic avec son programme d’installation pour connecter un navigateur.",
  "error.agentOriginInvalid":
    "Les agents ne peuvent envoyer une clé qu’à un site qui commence par https:// et ne désigne que le site, comme https://api.example.com. Vérifiez l’adresse.",
  "error.agentBindingInvalid":
    "Mosaic ne peut pas utiliser ce réglage pour les agents. Vérifiez le champ, les sites et la façon dont la clé est envoyée.",
  "error.agentValueUnusable":
    "Cette valeur est trop courte ou contient un saut de ligne, les agents ne peuvent donc pas l’utiliser. Une clé doit faire au moins 12 caractères sur une seule ligne.",
  "error.agentVaultShared":
    "Les agents ne peuvent utiliser que des éléments d’un coffre-fort qui vous appartient et qui n’est pas partagé. Déplacez d’abord l’élément vers l’un de vos propres coffres-forts.",
  "error.agentAccessOff":
    "L’accès des agents est désactivé. Activez-le dans les réglages, sous Agents.",
  "error.agentRequestExpired":
    "Cette demande d’agent est déjà terminée. L’agent peut redemander s’il en a encore besoin.",
  "error.agentAllowTooSoon":
    "C’était trop rapide pour lire la demande. Vérifiez les détails, puis choisissez de nouveau.",
  "error.agentPairingNotFound":
    "Cet agent n’est plus connecté. Rouvrez les réglages pour voir la liste actuelle.",
  "error.agentEndpointUnavailable":
    "Mosaic n’a pas pu ouvrir la connexion utilisée par les agents. Fermez les autres copies de Mosaic et réessayez.",
  "error.kitNotVerified":
    "Enregistrez d’abord le nouveau kit d’urgence et confirmez son dernier groupe. Mosaic ne modifie rien avant.",
  "error.kitGroupMismatch":
    "Ce n’est pas le dernier groupe de la nouvelle clé secrète. Vérifiez le nouveau kit d’urgence.",
  "error.rotationInProgress":
    "Un autre changement de clés est encore ouvert. Terminez-le avec Poursuivre le renouvellement des clés, ou abandonnez-le d’abord.",
  "error.rotationNone": "Il n’y a aucun changement de clés à poursuivre.",
  "error.rotationNotFound":
    "Ce changement de mot de passe n’est plus ouvert. Il a peut-être été fermé sur un autre appareil ou dans le navigateur. Rouvrez l’élément pour voir où il en est.",
  "error.rotationLimit":
    "Quatre changements de mot de passe de cet élément sont déjà ouverts. Terminez-en ou fermez-en un dans l’élément d’abord.",
  "error.rotationOpen":
    "Cet identifiant a un changement de mot de passe ouvert. Restaurez-le depuis la corbeille, puis choisissez Ça a marché ou Ça n’a pas marché sur l’élément. Vous pourrez ensuite le supprimer définitivement.",
  "error.rotationNotSupported":
    "Mosaic ne peut pas guider ce changement. Il faut un identifiant avec une adresse https. Changez vous-même le mot de passe sur le site.",
  "error.historyNotFound":
    "Cet ancien mot de passe n’est plus dans l’historique. Rouvrez l’élément pour voir la liste actuelle.",
  "error.originNotRelated":
    "Cette page n’appartient pas au site de cet identifiant, Mosaic ne l’a donc pas ajoutée. Modifiez les adresses de l’élément si elle devrait y figurer.",
  "error.breachConsentRequired":
    "La vérification de fuites est désactivée. Activez-la d’abord et acceptez ce qu’elle envoie.",
  "error.breachRunning":
    "Une vérification de fuites est déjà en cours. Attendez qu’elle se termine, ou arrêtez-la d’abord.",
  "error.rotationTooFar":
    "Ce changement a déjà atteint votre compte, il ne peut donc pas être abandonné. Poursuivez-le pour le terminer.",
  "error.rotationJumpTooLarge":
    "Cet appareil a trop de retard sur les autres appareils pour les rattraper. Configurez-le de nouveau : retirez le compte de cet appareil et rejoignez-le à nouveau avec votre kit d’urgence.",
  "error.rotationNotApplied":
    "Le changement n’a jamais atteint votre compte, rien n’a donc été modifié. Recommencez ou abandonnez-le.",
  "error.secretKeyUnchanged":
    "Cette clé secrète ne fonctionne plus. Utilisez celle du kit d’urgence que vous avez enregistré sur l’appareil qui a fait le changement.",
  "error.passwordUnchanged":
    "Le nouveau mot de passe principal doit être différent de l’actuel.",
  "error.profileDowngrade":
    "Ce niveau n’est pas plus fort que celui que votre compte utilise actuellement. Choisissez-en un plus élevé.",
  "error.profileTooSlow":
    "Avec ce niveau, le déverrouillage prendrait plus de deux secondes sur cet appareil. Choisissez un niveau plus bas, ou renforcez quand même.",
  "error.rekeyNeedsCleanSync":
    "La régénération des clés exige une synchronisation à jour et sans alerte. Synchronisez maintenant, examinez les éventuelles alertes, puis réessayez.",
  "error.rekeyMemberKeyMismatch":
    "Vous n’avez pas encore comparé le code de sécurité de toutes les personnes d’un coffre-fort partagé, ce coffre-fort ne peut donc pas être renouvelé. Comparez leurs codes dans les réglages de partage du coffre-fort, puis réessayez.",
  "error.rekeyBusy":
    "Vos coffres-forts ont continué à changer pendant que Mosaic travaillait. Patientez un instant et réessayez.",
  "error.rekeyOutcomeUnknown":
    "Mosaic n’a pas pu confirmer si votre compte a accepté les nouvelles clés. Choisissez Poursuivre le renouvellement des clés pour le savoir.",
  "error.rekeyNeedsServer":
    "La régénération des clés nécessite votre serveur ou dossier de synchronisation, qui est injoignable. Vérifiez la connexion et réessayez.",
  "error.rekeyNoAccountFile":
    "Rien dans votre dossier de synchronisation ne s’ouvre avec ce mot de passe et cette clé secrète. Vérifiez le nouveau kit d’urgence.",
  "error.syncAccountRekeyed":
    "Les clés de votre compte ont été renouvelées sur un autre appareil. Utilisez la clé secrète du nouveau kit d’urgence pour continuer.",
  "error.syncEpochRollback":
    "Votre système de synchronisation montre des clés de compte plus anciennes que celles de cet appareil. Rien n’a été modifié.",
  "error.syncRekeyRefused":
    "Les clés proposées par votre système de synchronisation ne sont pas les suivantes après celles de cet appareil. Rien n’a été modifié. Vérifiez vos autres appareils.",
  "error.pickerStale": "Cette liste est périmée. Cliquez de nouveau sur le champ.",
  "error.pickerTooSoon": "Patientez un instant, puis confirmez de nouveau.",
  "error.programFillUnavailable":
    "Le remplissage dans les programmes n’est pas disponible sur cet appareil.",
  "error.busy":
    "Mosaic termine encore la dernière étape. Patientez un instant, puis réessayez.",
  "error.internal":
    "Un problème inattendu a interrompu cette action. Réessayez, et redémarrez Mosaic si cela se répète.",
  "error.importTooLarge":
    "Ce fichier est trop volumineux ou trop complexe à importer. Exportez moins d’éléments à la fois, puis réessayez.",
  "error.importUnsupported":
    "Mosaic ne peut pas lire cette version du format. Exportez de nouveau dans un format actuel (pour KeePass : KDBX 4 avec AES ou ChaCha20), puis réessayez.",
  "error.importEncrypted":
    "Cet export est protégé par un mot de passe. Exportez de nouveau sans chiffrement, importez-le, puis supprimez le fichier.",
  "error.sshAgentEndpointInUse":
    "Un autre agent SSH est déjà en cours d’exécution (sous Windows, le service Agent d’authentification OpenSSH). Arrêtez-le, puis réactivez l’agent SSH de Mosaic.",
  "error.cancelled": "Annulé. Rien n’a changé.",
  "error.quickUnlockExpired":
    "Le déverrouillage rapide a expiré. Déverrouillez avec votre mot de passe principal, puis réactivez-le dans les réglages.",
  "error.quickUnlockInvalidated":
    "Le déverrouillage rapide a cessé de fonctionner sur cet appareil et a été désactivé. Déverrouillez avec votre mot de passe principal.",
  "error.passwordRequired":
    "Saisissez votre mot de passe principal cette fois. Le déverrouillage rapide refonctionnera ensuite.",
  "error.hardwareKeyUnsupported":
    "Cette clé de sécurité ou ce système ne peut pas être utilisé avec Mosaic.",
  "error.hardwareKeyNotFound": "Aucune clé de sécurité trouvée. Insérez-la et réessayez.",
  "error.hardwareKeyTimeout": "La clé n’a pas été touchée à temps. Réessayez.",
  "error.hardwareKeyPinRequired": "Saisissez le code PIN de votre clé de sécurité.",
  "error.hardwareKeyPinInvalid":
    "Code PIN incorrect. Après plusieurs essais erronés, la clé se verrouille d’elle-même.",
  "error.hardwareKeyPinBlocked":
    "Cette clé est verrouillée après trop de codes PIN erronés. Réinitialisez-la avec l’outil de son fabricant ; ses enregistrements sont alors perdus.",
  "error.hardwareKeyNoPin":
    "Définissez d’abord un code PIN sur cette clé, avec l’outil de son fabricant.",
  "error.hardwareKeyNotRecognized":
    "Cette clé n’est pas enregistrée ici. Essayez une autre clé ou votre mot de passe principal.",
  "error.hardwareKeyLimit":
    "Vous avez enregistré le nombre maximal de clés autorisé (8). Retirez-en une d’abord.",
  "error.hardwareKeyLast":
    "C’est la seule clé de ce coffre-fort. Enregistrez-en une autre avant de la retirer.",
  "error.itemTypeNotAllowed": "Ce coffre-fort ne contient que des codes à usage unique.",
  "error.otpScanUnsupported":
    "L’analyse de l’écran n’est pas disponible sur ce système. Collez plutôt une capture d’écran.",
  "error.otpQrNotFound":
    "Aucun code QR trouvé. Assurez-vous que tout le code est visible, puis réessayez.",
  "error.otpQrNotOtp":
    "Ce code QR n’est pas un code de configuration à deux facteurs. Cherchez le code sur la page de configuration 2FA du site.",
  "error.otpQrMigration":
    "Il s’agit d’un code d’export de Google Authenticator, que Mosaic ne sait pas encore lire. Configurez de nouveau chaque compte sur son site, ou saisissez sa clé de configuration.",
  "error.otpQrMultiple":
    "Plusieurs codes de configuration sont visibles. N’affichez que celui que vous voulez, puis réessayez.",
  "error.otpQrInvalid":
    "Ce code de configuration utilise des réglages que Mosaic ne prend pas en charge. Saisissez plutôt la clé de configuration.",
  "error.otpUriInvalid":
    "Ce n’est pas un lien otpauth:// valide. Copiez de nouveau le lien entier.",
  "error.otpKeyInvalid":
    "Cette clé n’est pas valide. Elle n’utilise que les lettres A à Z et les chiffres 2 à 7 ; vérifiez qu’il n’y a pas de faute de frappe.",
  "error.otpImageUnreadable": "Mosaic ne peut pas lire cette image. Utilisez un fichier PNG ou JPEG.",
  "error.otpImageTooLarge":
    "Cette image est trop grande. Utilisez une capture d’écran du seul code QR.",
  "error.otpClipboardNoImage":
    "Le presse-papiers ne contient pas d’image. Faites une capture d’écran du code QR, puis collez-la ici.",
  "error.otpPendingExpired": "Le code scanné n’est plus disponible. Scannez-le de nouveau.",
  "error.syncNotConfigured":
    "La synchronisation n’est pas encore configurée sur cet appareil. Configurez-la d’abord dans les réglages.",
  "error.syncAlreadyConfigured":
    "La synchronisation est déjà configurée. Déconnectez-la d’abord pour utiliser un autre serveur ou dossier.",
  "error.syncServerUnreachable":
    "Mosaic n’a pas pu joindre le serveur. Vérifiez l’adresse et votre connexion Internet, puis réessayez.",
  "error.syncServerInvalid":
    "Cela ne ressemble pas à un serveur Mosaic. Vérifiez l’adresse ; elle doit ressembler à https://sync.example.com.",
  "error.syncServerInsecure":
    "Mosaic ne se connecte aux serveurs qu’en https, pour que personne en chemin ne puisse lire ou modifier vos données. Utilisez une adresse https://.",
  "error.syncServerMismatch":
    "Ce n’est pas le serveur avec lequel ce compte se synchronise. Vérifiez l’adresse.",
  "error.syncServerIncompatible":
    "Ce serveur utilise une version qui ne correspond pas à votre Mosaic. Mettez Mosaic à jour ou demandez au propriétaire du serveur de le mettre à jour.",
  "error.syncRegistrationClosed":
    "Ce serveur n’accepte pas de nouveaux comptes de synchronisation. Connectez-vous à un compte existant ou demandez au propriétaire du serveur.",
  "error.syncInviteInvalid":
    "Ce code d’invitation n’est pas valide ou a déjà été utilisé. Vérifiez-le ou demandez-en un nouveau au propriétaire du serveur.",
  "error.syncAccountExists":
    "Vous avez déjà un compte de synchronisation sur ce serveur. Choisissez plutôt Se connecter.",
  "error.syncAccountMismatch":
    "Le compte de synchronisation de ce serveur appartient à un autre compte Mosaic. Vérifiez l’adresse et votre e-mail.",
  "error.syncTwoFactorRequired":
    "Ce compte de synchronisation nécessite un code de votre application d’authentification. Saisissez le code à 6 chiffres.",
  "error.syncTwoFactorInvalid":
    "Ce code n’est pas correct ou a expiré. Saisissez le code à 6 chiffres actuel de votre application d’authentification.",
  "error.syncSignedOut":
    "Le serveur a déconnecté cet appareil. Reconnectez-vous pour continuer à synchroniser. Vos modifications sont en sécurité sur cet appareil.",
  "error.syncRateLimited":
    "Trop de tentatives. Patientez quelques minutes, puis réessayez.",
  "error.syncQuotaExceeded":
    "Le serveur ou le dossier n’a plus de place pour vos données. Libérez de l’espace ou demandez-en davantage au propriétaire du serveur.",
  "error.syncFolderUnavailable":
    "Mosaic ne peut pas utiliser ce dossier. Vérifiez qu’il existe, que votre service cloud a terminé de le synchroniser et que Mosaic peut y écrire.",
  "error.syncFolderNotEmpty":
    "Ce dossier contient d’autres fichiers, Mosaic n’y écrira donc pas. Choisissez un dossier vide ou créez-en un nouveau.",
  "error.syncFolderNoAccount":
    "Ce dossier ne contient aucune donnée de synchronisation pour ce compte. Choisissez le dossier avec lequel votre autre appareil se synchronise.",
  "error.syncFolderDamaged":
    "Certains fichiers de synchronisation de ce dossier sont endommagés. Laissez votre service cloud terminer la synchronisation ou restaurez le dossier depuis une sauvegarde, puis réessayez.",
  "error.syncPasswordOutcomeUnknown":
    "Nous n’avons pas pu confirmer si le serveur a changé votre mot de passe principal. Gardez les deux mots de passe jusqu’à ce que la synchronisation confirme lequel s’applique.",
  "error.syncWebauthnUnavailable":
    "Votre serveur ne peut pas utiliser de clé de sécurité pour cela pour le moment, ou ce compte n’en a aucune. Utilisez plutôt un code d’application d’authentification.",
  "error.syncWebauthnLimit":
    "Ce compte a déjà le nombre maximal de clés de sécurité autorisé par le serveur. Retirez-en une que vous n’utilisez plus, puis réessayez.",
  "error.syncHandoffExpired": "Le délai de l’étape dans le navigateur est dépassé. Recommencez.",
  "error.syncHandoffFailed":
    "Trop de codes erronés ou de vérifications échouées dans le navigateur. Recommencez l’étape.",
  "error.syncPasswordNeedsServer":
    "Changer votre mot de passe principal nécessite votre serveur de synchronisation, qui est injoignable. Vérifiez votre connexion Internet et réessayez.",
  "error.syncDeviceIsCurrent":
    "Vous ne pouvez pas retirer l’appareil que vous utilisez. Utilisez plutôt Déconnecter.",
  "error.syncChoiceExpired": "Le choix du dossier a expiré. Choisissez de nouveau le dossier.",
  "error.syncFreshLoginExpired":
    "La vérification de votre mot de passe a expiré. Saisissez de nouveau votre mot de passe principal.",
  "error.syncPasswordRollback":
    "Le serveur a proposé un mot de passe principal plus ancien. Vérifiez le serveur.",
  "error.syncPasswordChanged":
    "Votre mot de passe principal a été changé sur le serveur ou dans le dossier. Saisissez le nouveau ci-dessous.",
  "error.sharingNotHosted":
    "Le partage nécessite un serveur Mosaic. Connectez-en un d’abord dans les réglages de synchronisation.",
  "error.setupCodeNeedsServer":
    "Un téléphone ne peut rejoindre qu’un compte qui se synchronise.",
  "error.sharingContactUnverified":
    "Comparez d’abord le code de sécurité de cette personne. Mosaic ne partage qu’avec des personnes que vous avez vérifiées.",
  "error.sharingKeyChanged":
    "La clé de cette personne a changé, ou le code ne correspond pas. Recherchez-la de nouveau et comparez le nouveau code avant de continuer.",
  "error.sharingRoleForbidden":
    "Votre rôle dans ce coffre-fort ne le permet pas. Demandez au propriétaire ou à un gestionnaire.",
  "error.sharingInviteExists":
    "Une invitation pour cette personne et ce coffre-fort est déjà en attente. Retirez-la d’abord si vous voulez en envoyer une nouvelle.",
  "error.sharingInviteStale":
    "L’invitation a changé entre-temps, par exemple après un changement de clé. Ouvrez-la de nouveau.",
  "error.sharingInviteExpired":
    "Cette invitation a expiré ou a été retirée. Demandez à l’expéditeur de vous inviter de nouveau.",
  "error.sharingInviteUnreadable":
    "Cette invitation ne peut pas être ouverte. Elle est peut-être endommagée ou n’est pas destinée à ce compte. Demandez à l’expéditeur de vous inviter de nouveau.",
  "error.sharingInviteNotInRoster":
    "La liste des membres du coffre-fort ne correspond pas à cette invitation, Mosaic ne l’a donc pas ouverte. Demandez à l’expéditeur de vous inviter de nouveau.",
  "error.sharingOwnerCannotLeave":
    "Le propriétaire ne peut pas quitter un coffre-fort. Retirez plutôt les autres personnes.",
  "error.sharingAlreadyMember": "Cette personne est déjà membre du coffre-fort.",
  "error.sharingReinviteBlocked":
    "Vous avez retiré cette personne récemment. Vous pourrez l’inviter de nouveau une fois la clé du coffre-fort changée. Synchronisez, puis réessayez.",
  "error.sharingInviteLimit":
    "Trop d’invitations sont en attente. Attendez les réponses ou retirez-en quelques-unes.",
  "error.sharingVaultKnown":
    "Cet appareil a déjà ce coffre-fort, Mosaic n’a donc pas accepté de clé pour lui. Demandez à l’expéditeur quel coffre-fort il a partagé.",
  "error.syncHasSharedVaults":
    "Vous ne pouvez pas changer de serveur tant que des coffres-forts partagés sont sur cet appareil. Quittez d’abord les coffres-forts partagés, ou faites-vous retirer par leurs propriétaires.",
  "error.emergencyWaiting":
    "Une demande est en attente, cela ne peut donc pas changer maintenant. Refusez-la ou attendez-la.",
  "error.emergencyCooldown":
    "Votre dernière demande a été refusée. Vous pouvez redemander après le délai d’attente d’un jour.",
  "error.emergencyNotGranted":
    "Il n’y a rien à réclamer : aucune demande n’est ouverte, ou la personne ne vous a pas donné accès.",
  "error.shortcutUnavailable":
    "Un autre programme utilise déjà ce raccourci, ou Mosaic ne peut pas l’enregistrer. Choisissez une autre combinaison.",
  "error.shortcutUnsupported":
    "Votre système ne permet pas aux applications de définir un raccourci global. Ouvrez plutôt Mosaic depuis sa fenêtre.",
} satisfies Record<keyof typeof en, string>;
