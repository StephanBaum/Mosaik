/**
 * French UI strings: sync. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { sync as en } from "../en/sync.ts";

export const sync = {
  "sync.status.signedOut": "Déconnecté",
  "sync.platform.windows": "Windows",
  "sync.platform.macos": "macOS",
  "sync.platform.linux": "Linux",
  "settings.sync.title": "Synchronisation",
  "settings.sync.body":
    "Gardez vos coffres-forts à jour sur tous vos appareils. Tout est chiffré sur cet appareil avant de le quitter.",
  "settings.sync.notSetUp":
    "La synchronisation est désactivée. Vos coffres-forts ne sont stockés que sur cet appareil.",
  "settings.sync.deviceName": "Nom de cet appareil",
  "settings.sync.deviceName.default": "Mosaic sur {platform}",
  "settings.sync.password": "Mot de passe principal",
  "settings.sync.code": "Code à 6 chiffres",
  "settings.sync.server.title": "Serveur Mosaic",
  "settings.sync.server.body":
    "Synchronisez via un serveur Mosaic que vous ou une personne de confiance exploitez.",
  "settings.sync.server.url": "Adresse du serveur",
  "settings.sync.server.urlHint": "Par exemple https://sync.example.com",
  "settings.sync.server.check": "Vérifier le serveur",
  "settings.sync.server.checking": "Vérification du serveur…",
  "settings.sync.server.registration.open":
    "Ce serveur permet à n’importe qui de créer un compte de synchronisation.",
  "settings.sync.server.registration.invite":
    "Ce serveur exige un code d’invitation pour créer un compte de synchronisation.",
  "settings.sync.server.registration.closed":
    "Ce serveur n’accepte pas de nouveaux comptes de synchronisation. Vous pouvez toujours vous connecter à un compte existant.",
  "settings.sync.server.choice.label": "Compte de synchronisation",
  "settings.sync.server.choice.signIn": "J’ai un compte de synchronisation",
  "settings.sync.server.choice.create": "En créer un",
  "settings.sync.server.invite": "Code d’invitation",
  "settings.sync.server.create": "Créer un compte de synchronisation",
  "settings.sync.server.signIn": "Se connecter",
  "settings.sync.server.signInAgain": "Se reconnecter",
  "settings.sync.server.working": "Connexion…",
  "settings.sync.server.sees":
    "Le serveur ne voit que des données chiffrées, l’e-mail de votre compte et le moment où vous synchronisez. Il ne voit jamais votre mot de passe principal ni votre clé secrète.",
  "settings.sync.twoFactor.title": "Saisissez votre code",
  "settings.sync.twoFactor.body":
    "Ce compte de synchronisation utilise la vérification en deux étapes. Saisissez le code à 6 chiffres de votre application d’authentification.",
  "settings.sync.twoFactor.submit": "Vérifier",
  "settings.sync.folder.title": "Votre propre dossier cloud",
  "settings.sync.folder.body":
    "Synchronisez via un dossier que votre service cloud (Dropbox, OneDrive, iCloud Drive et d’autres) maintient à jour.",
  "settings.sync.folder.choose": "Choisir un dossier…",
  "settings.sync.folder.chooseOther": "Choisir un autre dossier…",
  "settings.sync.folder.path": "Dossier",
  "settings.sync.folder.contents.empty": "Le dossier est vide. Mosaic peut l’utiliser.",
  "settings.sync.folder.contents.mosaic":
    "Ce dossier contient déjà des données de synchronisation Mosaic.",
  "settings.sync.folder.contents.other":
    "Ce dossier contient d’autres fichiers. Mosaic n’y écrit rien. Choisissez un dossier vide ou créez-en un nouveau.",
  "settings.sync.folder.connect": "Connecter le dossier",
  "settings.sync.folder.working": "Connexion…",
  "settings.sync.folder.sees":
    "Votre fournisseur cloud et toute personne ayant accès au dossier ne voient que des données chiffrées, l’identifiant du compte et le moment où les fichiers changent. Ils ne voient jamais votre mot de passe principal ni votre clé secrète.",
  "settings.sync.status.label": "État",
  "settings.sync.lastSynced": "Dernière synchronisation {when}",
  "settings.sync.neverSynced": "Pas encore synchronisé",
  "settings.sync.offline":
    "Vous êtes hors ligne. Vos modifications sont enregistrées sur cet appareil et seront synchronisées dès que vous serez de nouveau en ligne.",
  "settings.sync.now": "Synchroniser maintenant",
  "settings.sync.backend.server": "Serveur",
  "settings.sync.backend.account": "Compte de synchronisation",
  "settings.sync.backend.folder": "Dossier",
  "settings.sync.backend.device": "Cet appareil",
  "settings.sync.signedOut.title": "Cet appareil est déconnecté",
  "settings.sync.signedOut.body":
    "Le serveur a mis fin à la session de cet appareil. Reconnectez-vous pour continuer à synchroniser. Vos modifications sont en sécurité sur cet appareil.",
  "settings.sync.recover.title": "Votre mot de passe principal a été changé",
  "settings.sync.recover.body":
    "Le serveur ou le dossier contient un mot de passe principal plus récent que celui de cet appareil. Saisissez le nouveau mot de passe principal pour mettre cet appareil à jour. Vos coffres-forts et votre clé secrète restent les mêmes.",
  "settings.sync.recover.password": "Nouveau mot de passe principal",
  "settings.sync.recover.submit": "Mettre à jour cet appareil",
  "settings.sync.disconnect": "Déconnecter…",
  "settings.sync.disconnect.title": "Déconnecter la synchronisation ?",
  "settings.sync.disconnect.body":
    "Cet appareil arrête de synchroniser et se déconnecte du serveur. Vos coffres-forts restent sur cet appareil, et vos autres appareils gardent leurs propres copies.",
  "settings.sync.disconnect.confirm": "Déconnecter",
  "settings.sync.devices.title": "Appareils",
  "settings.sync.devices.thisDevice": "Cet appareil",
  "settings.sync.devices.unnamed": "Appareil sans nom",
  "settings.sync.devices.lastSeen": "Vu pour la dernière fois {when}",
  "settings.sync.devices.neverSeen": "Pas encore vu",
  "settings.sync.devices.rename": "Renommer",
  "settings.sync.devices.renameNamed": "Renommer {name}",
  "settings.sync.devices.renameLabel": "Nouveau nom de cet appareil",
  "settings.sync.devices.remove": "Retirer",
  "settings.sync.devices.removeNamed": "Retirer {name}",
  "settings.sync.devices.removeTitle": "Retirer {name} ?",
  "settings.sync.devices.removeBody":
    "Retirer un appareil le déconnecte de la synchronisation. Cela ne supprime pas la copie de vos coffres-forts qu’il possède déjà. Pour exclure définitivement un appareil, changez votre mot de passe principal et votre clé secrète.",
  "settings.sync.devices.removeConfirm": "Retirer l’appareil",
  "settings.sync.devices.folderNote":
    "Pour exclure un appareil, changez votre mot de passe principal et votre clé secrète.",
  "settings.sync.phone.title": "Configurer un téléphone",
  "settings.sync.phone.body":
    "Affiche un code que votre téléphone scanne, pour que vous n’ayez pas à saisir votre clé secrète. Le téléphone a toujours besoin de votre mot de passe principal.",
  "settings.sync.phone.button": "Configurer un téléphone",
  "settings.sync.phone.reauth":
    "Le code contient votre clé secrète, il faut donc votre mot de passe principal.",
  "settings.sync.phone.dialogTitle": "Scannez ce code avec votre téléphone",
  "settings.sync.phone.steps":
    "Sur votre téléphone, ouvrez Mosaic et choisissez de le configurer, puis scannez ce code.",
  "settings.sync.phone.hint":
    "Le code se masque après 60 secondes ou quand vous quittez cette fenêtre. Ne partagez pas de photo du code.",
  "settings.sync.phone.canvas": "Code de configuration pour votre téléphone",
  "settings.sync.phone.hidden.time": "Les 60 secondes sont écoulées, le code est donc masqué.",
  "settings.sync.phone.hidden.away": "Vous avez quitté cette fenêtre, le code est donc masqué.",
  "settings.sync.phone.again": "Afficher de nouveau",
  "settings.sync.phone.folderHint":
    "Votre téléphone se connecte à votre stockage cloud et ouvre ce même dossier.",
  "settings.sync.totp.title": "Application d’authentification",
  "settings.sync.totp.body":
    "Demande un code d’une application d’authentification quand quelqu’un se connecte à votre compte de synchronisation. Cela protège le compte sur le serveur, pas vos coffres-forts : ceux-ci restent chiffrés avec votre mot de passe principal et votre clé secrète.",
  "settings.sync.totp.on": "Les codes d’une application d’authentification sont activés.",
  "settings.sync.totp.off": "Les codes d’une application d’authentification sont désactivés.",
  "settings.sync.totp.enable": "Activer…",
  "settings.sync.totp.disable": "Désactiver…",
  "settings.sync.totp.beginTitle": "Activer les codes d’une application d’authentification",
  "settings.sync.totp.beginBody": "Saisissez votre mot de passe principal pour commencer.",
  "settings.sync.totp.begin": "Continuer",
  "settings.sync.totp.setupBody":
    "Ajoutez cette clé à votre application d’authentification, puis saisissez le code à 6 chiffres qu’elle affiche.",
  "settings.sync.totp.secret": "Clé de configuration",
  "settings.sync.totp.confirm": "Activer",
  "settings.sync.totp.disableTitle": "Désactiver les codes d’une application d’authentification",
  "settings.sync.totp.disableBody": "Saisissez votre mot de passe principal et un code actuel.",
  "settings.sync.totp.disableConfirm": "Désactiver",
  "settings.sync.totp.enabledToast": "Les codes d’une application d’authentification sont activés.",
  "settings.sync.totp.disabledToast":
    "Les codes d’une application d’authentification sont désactivés.",
  "settings.sync.key.title": "Clés de sécurité",
  "settings.sync.key.body":
    "Une clé de sécurité peut servir de seconde étape quand vous vous connectez à votre compte de synchronisation. Elle protège le compte sur le serveur, pas vos coffres-forts. Ajouter ou utiliser une clé ouvre votre navigateur.",
  "settings.sync.key.none": "Aucune clé de sécurité ajoutée.",
  "settings.sync.key.added": "Ajoutée {when}",
  "settings.sync.key.lastUsed": "Dernière utilisation {when}",
  "settings.sync.key.neverUsed": "Pas encore utilisée",
  "settings.sync.key.add": "Ajouter une clé de sécurité…",
  "settings.sync.key.addTitle": "Ajouter une clé de sécurité",
  "settings.sync.key.addBody":
    "Donnez un nom à la clé et saisissez votre mot de passe principal. Mosaic ouvre ensuite votre navigateur, où vous utilisez la clé.",
  "settings.sync.key.label": "Nom de la clé",
  "settings.sync.key.labelHint": "Par exemple « Clé du bureau ». 64 caractères au maximum.",
  "settings.sync.key.addConfirm": "Continuer",
  "settings.sync.key.addedToast": "Clé de sécurité ajoutée.",
  "settings.sync.key.noBackup":
    "Une clé de sécurité serait votre seule seconde étape. Si vous la perdez, vous risquez de ne plus pouvoir vous connecter à votre compte de synchronisation. Nous recommandons une solution de secours : une seconde clé ou un code d’une application d’authentification.",
  "settings.sync.key.recommend":
    "Nous recommandons deux méthodes, par exemple deux clés, ou une clé et un code d’une application d’authentification.",
  "settings.sync.key.remove": "Retirer",
  "settings.sync.key.removeLabel": "Retirer {name}",
  "settings.sync.key.removeTitle": "Retirer {name} ?",
  "settings.sync.key.removeBody":
    "Vous confirmez une fois avec une clé de sécurité dans votre navigateur. Ensuite, cette clé ne fonctionne plus pour se connecter.",
  "settings.sync.key.removeLast":
    "C’est votre dernière seconde étape. La vérification en deux étapes se désactive, et votre compte de synchronisation n’est alors protégé que par votre mot de passe principal et votre clé secrète.",
  "settings.sync.key.removeCode": "Code de l’application d’authentification",
  "settings.sync.key.removeCodeHint":
    "Saisissez un code actuel pour retirer la clé immédiatement. Laissez vide pour confirmer plutôt avec une clé de sécurité dans votre navigateur.",
  "settings.sync.key.removeConfirm": "Retirer la clé",
  "settings.sync.key.removedToast": "Clé de sécurité retirée.",
  "settings.sync.key.confirmTitle": "Confirmer avec une clé de sécurité",
  "settings.sync.key.confirmBody":
    "Retirer une clé nécessite une seconde étape. Saisissez votre mot de passe principal, puis confirmez avec une clé dans votre navigateur.",
  "settings.sync.key.step.methods": "Vérification du mode de connexion de ce compte…",
  "settings.sync.key.step.useKey": "Utiliser la clé de sécurité (ouvre votre navigateur)",
  "settings.sync.key.step.keyOnlyTitle": "Utilisez votre clé de sécurité",
  "settings.sync.key.step.keyOnlyBody":
    "Ce compte est protégé par une clé de sécurité. Mosaic ouvre une page dans votre navigateur, où vous l’utilisez.",
  "settings.sync.key.step.or": "Ou utilisez un code",
  "settings.sync.key.wait.login": "Connectez-vous avec votre clé de sécurité",
  "settings.sync.key.wait.register": "Ajoutez votre clé de sécurité",
  "settings.sync.key.wait.confirm": "Confirmez avec votre clé de sécurité",
  "settings.sync.key.wait.lead":
    "Votre navigateur a ouvert une page Mosaic de votre serveur de synchronisation. Saisissez ce code à cet endroit, puis utilisez votre clé de sécurité.",
  "settings.sync.key.wait.codeLabel": "Code à saisir dans votre navigateur",
  "settings.sync.key.wait.codeSpoken": "Code à saisir dans votre navigateur : {digits}",
  "settings.sync.key.wait.expires": "Cette étape expire à {time}.",
  "settings.sync.key.wait.waiting": "En attente de votre clé de sécurité…",
  "settings.sync.key.wait.done": "Terminé.",
  "settings.sync.key.wait.expired": "Le délai de cette étape est dépassé. Recommencez.",
  "settings.sync.key.wait.failed":
    "L’étape a échoué : le code a été erroné trop souvent, ou la clé de sécurité n’a pas été acceptée. Rien n’a été modifié. Recommencez.",
  "settings.sync.key.wait.reopen": "Rouvrir la page du navigateur",
  "settings.sync.key.wait.retry": "Recommencer",
  "sync.warning.region": "Alertes de synchronisation",
  "sync.warning.unknownDevice": "un autre appareil",
  "sync.warning.unknownVault": "l’un de vos coffres-forts",
  "sync.warning.freezeSuspected":
    "Les modifications de {device} ne sont pas arrivées depuis plus d’un jour. Vérifiez que l’autre appareil est allumé et en ligne.",
  "sync.warning.missingFiles":
    "Des fichiers de {device} sont absents du dossier. Laissez votre service cloud terminer la synchronisation, puis synchronisez de nouveau.",
  "sync.warning.noCommonView":
    "Cet appareil et {device} n’ont pas vu les mêmes données du serveur depuis deux jours. Le serveur leur montre peut-être des versions différentes.",
  "sync.warning.rollbackDetected":
    "Vos données de synchronisation semblent plus anciennes que cet appareil. Vérifiez le serveur ou le dossier, puis choisissez « J’ai vérifié ».",
  "sync.warning.identityDuplicated":
    "Une autre copie de Mosaic écrit sous l’identité de cet appareil, par exemple après la restauration d’une sauvegarde. La synchronisation est suspendue. Assurez-vous qu’une seule copie est utilisée, puis choisissez « J’ai vérifié ».",
  "sync.warning.serverInconsistent":
    "Le serveur a montré à cet appareil des données différentes de celles d’un autre appareil. Vérifiez le serveur, puis choisissez « J’ai vérifié ».",
  "sync.warning.serverRollback":
    "La copie de {vault} sur le serveur est plus ancienne que celle de cet appareil. Vérifiez le serveur, puis choisissez « J’ai vérifié ».",
  "sync.warning.undecryptable":
    "{count, plural, one {# élément reçu n’a pas pu être lu} other {# éléments reçus n’ont pas pu être lus}} sur cet appareil. Mettez Mosaic à jour sur tous vos appareils, puis synchronisez de nouveau.",
  "sync.warning.hardwareVaultsLocal":
    "Les coffres-forts protégés par matériel restent sur cet appareil. {count, plural, one {# coffre-fort n’est pas synchronisé} other {# coffres-forts ne sont pas synchronisés}}.",
  "sync.warning.attachmentsNotRepaired":
    "{count, plural, one {# fichier} other {# fichiers}} sur le serveur ou dans le dossier n’ont pas pu être réparés depuis cet appareil. Ouvrez l’élément sur un appareil qui a encore le fichier et ajoutez-le de nouveau.",
  "sync.warning.acknowledge": "J’ai vérifié…",
  "sync.acknowledge.title": "Effacer les alertes de synchronisation qui demandent une vérification ?",
  "sync.acknowledge.body":
    "Ne continuez qu’après avoir vérifié vous-même le serveur ou le dossier. Cela efface toutes les alertes qui demandent une vérification, et la synchronisation continue. Si vos données ont réellement été restaurées à un état antérieur ou modifiées, Mosaic ne pourra plus le détecter ensuite.",
  "sync.acknowledge.eachDevice": "Faites-le sur chacun de vos appareils.",
  "toast.syncAttention":
    "La synchronisation a fusionné ou réparé certains éléments venant d’un autre appareil. Examinez ce qui a changé.",
  "sync.revoked.rekey":
    "Les clés de ce compte ont été renouvelées sur un autre appareil : connectez-vous avec la nouvelle clé secrète de votre kit d’urgence le plus récent.",
  "sync.revoked.passwordChange":
    "Votre mot de passe principal a été changé sur un autre appareil : connectez-vous avec le nouveau mot de passe.",
  "sync.revoked.deviceRemoved":
    "Cet appareil a été retiré de votre compte depuis un autre appareil. Reconnectez-vous pour l’ajouter de nouveau.",
  "sync.revoked.operator":
    "L’opérateur du serveur a mis fin à cette session. Reconnectez-vous pour continuer à synchroniser.",
} satisfies Record<keyof typeof en, string>;
