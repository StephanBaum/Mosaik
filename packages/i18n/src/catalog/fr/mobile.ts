/**
 * French UI strings: mobile. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { mobile as en } from "../en/mobile.ts";

export const mobile = {
  "mobile.app.name": "Mosaic",
  "mobile.nav.vault": "Coffre-fort",
  "mobile.nav.search": "Recherche",
  "mobile.nav.settings": "Réglages",
  "mobile.common.cancel": "Annuler",
  "mobile.common.done": "Terminé",
  "mobile.common.close": "Fermer",
  "mobile.common.continue": "Continuer",
  "mobile.unlock.title": "Déverrouiller Mosaic",
  "mobile.unlock.passwordLabel": "Mot de passe principal",
  "mobile.unlock.action": "Déverrouiller",
  "mobile.unlock.quick": "Utiliser le déverrouillage rapide",
  "mobile.unlock.failed":
    "{attempts, plural, one {Cela n’a pas fonctionné # fois. Mosaic ne peut pas réinitialiser votre mot de passe principal. Si vous l’avez changé sur un autre appareil, utilisez le nouveau.} other {Cela n’a pas fonctionné # fois. Mosaic ne peut pas réinitialiser votre mot de passe principal. Si vous l’avez changé sur un autre appareil, utilisez le nouveau.}}",
  "mobile.lock.now": "Verrouiller maintenant",
  "mobile.vault.title": "Votre coffre-fort",
  "mobile.vault.count": "{count, plural, one {# élément} other {# éléments}}",
  "mobile.vault.emptyTitle": "Aucun élément pour l’instant",
  "mobile.vault.emptyBody":
    "Tirez vers le bas pour synchroniser avec vos autres appareils, ou ajoutez votre premier identifiant.",
  "mobile.search.placeholder": "Rechercher dans votre coffre-fort",
  "mobile.copy.done":
    "Copié. Il sera effacé du presse-papiers dans {seconds, plural, one {# seconde} other {# secondes}}.",
  "mobile.setup.scanTitle": "Scanner votre code de configuration",
  "mobile.setup.scanHint":
    "Sur votre ordinateur, ouvrez Mosaic, allez dans Réglages > Synchronisation et choisissez « Configurer un téléphone ». Placez la caméra devant le code.",
  "mobile.autofill.title": "Remplir depuis Mosaic",
  "mobile.autofill.target": "Remplir dans {app} ?",
  "mobile.settings.title": "Réglages",
  "mobile.settings.about": "À propos de Mosaic",
  "mobile.settings.version": "Version {version}",
  "mobile.common.back": "Retour",
  "mobile.common.notNow": "Pas maintenant",
  "mobile.onboarding.step": "Étape {current} sur {total}",
  "mobile.onboarding.welcome.title": "Configurer Mosaic sur ce téléphone",
  "mobile.onboarding.welcome.body":
    "Connectez-vous à votre compte Mosaic pour y retrouver vos mots de passe. Tout reste chiffré sur ce téléphone.",
  "mobile.onboarding.welcome.start": "Commencer",
  "mobile.onboarding.details.title": "Votre compte",
  "mobile.onboarding.details.server": "Adresse du serveur",
  "mobile.onboarding.details.serverHint": "https://sync.example.com",
  "mobile.onboarding.details.email": "E-mail",
  "mobile.onboarding.details.secretKey": "Clé secrète",
  "mobile.onboarding.details.secretKeyHelp":
    "Depuis votre kit d’urgence : des lettres et des chiffres en groupes, commençant par M1.",
  "mobile.onboarding.details.deviceName": "Nom de ce téléphone",
  "mobile.onboarding.details.deviceNameDefault": "Téléphone Android",
  "mobile.onboarding.details.checking": "Vérification du serveur",
  "mobile.onboarding.password.title": "Votre mot de passe principal",
  "mobile.onboarding.password.body":
    "Mosaic ne l’envoie jamais nulle part. Il déverrouille votre coffre-fort sur ce téléphone.",
  "mobile.onboarding.password.action": "Se connecter",
  "mobile.onboarding.password.working": "Connexion en cours. Cela peut prendre quelques secondes.",
  "mobile.onboarding.second.title": "Encore une étape",
  "mobile.onboarding.second.totpLabel": "Code à 6 chiffres",
  "mobile.onboarding.second.totpAction": "Vérifier",
  "mobile.onboarding.second.keyAction": "Utiliser votre clé de sécurité",
  "mobile.onboarding.second.keyWaiting":
    "En attente de votre clé de sécurité. Terminez dans le navigateur, puis revenez ici. Le code affiché là-bas doit être {code}.",
  "mobile.onboarding.quick.title": "Déverrouiller plus vite",
  "mobile.onboarding.quick.body":
    "Utilisez votre empreinte pour déverrouiller Mosaic la prochaine fois. Après un redémarrage du téléphone, votre mot de passe principal reste nécessaire une fois.",
  "mobile.onboarding.quick.enable": "Activer le déverrouillage rapide",
  "mobile.onboarding.quick.unavailable":
    "Ajoutez une empreinte dans les réglages de votre téléphone pour utiliser le déverrouillage rapide.",
  "mobile.onboarding.autofill.title": "Remplir automatiquement les identifiants",
  "mobile.onboarding.autofill.body":
    "Laissez Mosaic proposer vos identifiants dans d’autres applications et dans le navigateur. Vous pourrez le modifier plus tard dans les réglages du système.",
  "mobile.onboarding.autofill.open": "Ouvrir les réglages de saisie automatique",
  "mobile.onboarding.done.title": "Tout est prêt",
  "mobile.onboarding.done.body": "Votre coffre-fort est sur ce téléphone et déverrouillé.",
  "mobile.onboarding.done.action": "Ouvrir votre coffre-fort",
  "mobile.unlock.account": "Déverrouillage de {email}",
  "mobile.unlock.secretKey": "Clé secrète",
  "mobile.unlock.secretKeyHelp":
    "Ce téléphone n’a pas votre clé secrète en mémoire. Saisissez-la depuis votre kit d’urgence.",
  "mobile.unlock.enrollNext": "Déverrouiller avec mon empreinte la prochaine fois",
  "mobile.unlock.working": "Déverrouillage en cours. Cela peut prendre quelques secondes.",
  "mobile.unlock.biometricTitle": "Déverrouiller Mosaic",
  "mobile.unlock.biometricSubtitle": "Confirmez que c’est bien vous",
  "mobile.unlock.usePassword": "Utiliser le mot de passe principal",
  "mobile.unlock.busyTitle": "Mosaic est occupé",
  "mobile.unlock.retry": "Réessayer",
  "mobile.home.unlockedAs": "Déverrouillé en tant que {email}",
  "error.notFound":
    "Cet élément n’existe plus. Il a peut-être été supprimé sur un autre appareil.",
  "error.syncFailed":
    "Mosaic n’a pas pu synchroniser. Vérifiez votre connexion, puis réessayez.",
  "error.passwordInApp":
    "Le déverrouillage avec le mot de passe principal n’est pas possible ici. Ouvrez l’application Mosaic et déverrouillez-la là-bas.",
  "error.folderProviderUnsupported":
    "Ce téléphone ne peut pas accéder au dossier cloud de ce compte. Reconnectez-vous au fournisseur du dossier, ou utilisez un ordinateur.",
  "error.folderProviderSignedOut":
    "Votre service cloud a mis fin à la connexion ou l’a refusée. Reconnectez-vous pour continuer à synchroniser.",
  "error.folderNotFound":
    "Mosaic ne trouve pas ce dossier, ou ce n’est pas un dossier Mosaic. Choisissez le dossier avec lequel votre ordinateur se synchronise.",
  "error.folderStorageFull":
    "Votre stockage cloud est plein. Libérez de l’espace là-bas, puis synchronisez de nouveau.",
  "error.folderLinkExpired": "La connexion a pris trop de temps. Recommencez.",
  "error.folderLinkInvalid": "La connexion n’a pas fonctionné ou a été annulée. Recommencez.",
  "error.folderServerTlsTooOld":
    "Ce serveur est trop ancien pour une connexion sécurisée. Mosaic exige TLS 1.3. Demandez à la personne qui exploite le serveur de le mettre à jour.",
  "mobile.error.syncPasswordRollback":
    "Le dossier propose un mot de passe principal plus ancien que celui que connaît ce téléphone. Vérifiez le dossier et votre service cloud.",
  "error.setupCodeInvalid":
    "Ce code de configuration n’est pas valide ou a expiré. Créez-en un nouveau sur votre ordinateur et scannez-le de nouveau.",
  "error.notEnoughMemory":
    "Cet appareil n’a pas assez de mémoire libre pour déverrouiller votre coffre-fort. Fermez d’autres applications et réessayez.",
  "error.noMatch":
    "Mosaic n’a aucun identifiant pour cette application ou ce site web. Ouvrez Mosaic pour en ajouter un.",
  "error.insecureOrigin":
    "Cette page n’utilise pas de connexion sécurisée, Mosaic ne la remplira donc pas. Ouvrez la version https de la page.",
  "error.appNotLinked":
    "Cette application n’est pas associée à un site web enregistré. Associez-la d’abord dans Mosaic, puis réessayez.",
  "error.foreignFrame":
    "Ce champ appartient à un autre site intégré à la page, Mosaic ne le remplira donc pas.",
  "error.assetLinksUnavailable":
    "Mosaic n’a pas pu vérifier que cette application appartient au site web. Vérifiez votre connexion et réessayez.",
  "error.appNotVerified":
    "Cette application n’est pas vérifiée pour ce site web, Mosaic n’utilisera donc pas ses clés d’accès. Utilisez plutôt le site web dans un navigateur.",
  "error.rpIdInvalid":
    "Le nom de site web de cette demande n’est pas valide. Annulez et recommencez.",
  "error.passkeyNotFound":
    "Mosaic n’a pas de clé d’accès pour ce site web. Connectez-vous autrement.",
  "error.passkeyExcluded":
    "Vous avez déjà une clé d’accès pour ce compte dans Mosaic. Connectez-vous plutôt avec elle.",
  "error.passkeyUnsupportedAlgorithm":
    "Ce site web nécessite un type de clé d’accès que Mosaic ne sait pas encore créer. Choisissez un autre moyen de vous connecter.",
  "error.passkeySessionExpired":
    "Cette demande de clé d’accès a expiré. Recommencez depuis l’application.",
  "error.qrNotFound":
    "Mosaic n’a trouvé aucun code QR dans cette image. Réessayez avec une image plus nette.",
  "error.storageUnavailable":
    "Mosaic n’a pas pu accéder au stockage sécurisé de cet appareil. Redémarrez votre téléphone et réessayez.",
  "mobile.common.tryAgain": "Réessayer",
  "mobile.onboarding.preAudit":
    "Logiciel avant audit : pas encore d’examen de sécurité externe.",
  "mobile.onboarding.start": "Commencer",
  "mobile.onboarding.server": "Adresse du serveur",
  "mobile.onboarding.serverPlaceholder": "https://",
  "mobile.onboarding.email": "E-mail",
  "mobile.onboarding.secretKey": "Clé secrète",
  "mobile.onboarding.secretKeyHint":
    "Vous la trouvez dans votre kit d’urgence ou dans les réglages sur votre ordinateur. Les majuscules et les tirets n’ont pas d’importance.",
  "mobile.onboarding.deviceName": "Nom de ce téléphone",
  "mobile.onboarding.passwordHint":
    "Mosaic le vérifie sur ce téléphone. Il n’est jamais envoyé au serveur.",
  "mobile.onboarding.signIn": "Se connecter",
  "mobile.onboarding.working": "Connexion en cours. Cela peut prendre quelques secondes.",
  "mobile.onboarding.secondFactorTitle": "Encore une étape",
  "mobile.onboarding.totpLabel": "Code de votre application d’authentification",
  "mobile.onboarding.verify": "Vérifier",
  "mobile.onboarding.securityKey": "Utiliser votre clé de sécurité",
  "mobile.onboarding.securityKeyWaiting":
    "Terminez la connexion dans votre navigateur. Vérifiez qu’il affiche le code {code}, puis revenez ici.",
  "mobile.onboarding.autofillSteps":
    "Ouvrez l’app Réglages, touchez Mots de passe, puis Options des mots de passe, et activez Mosaic.",
  "mobile.onboarding.openSettings": "Ouvrir les réglages",
  "mobile.onboarding.notNow": "Pas maintenant",
  "mobile.unlock.secretKeyNeeded":
    "Ce téléphone n’a pas votre clé secrète. Saisissez-la une fois pour déverrouiller.",
  "mobile.unlock.quickOptionFaceId": "Déverrouiller avec Face ID la prochaine fois",
  "mobile.unlock.biometricReason": "Déverrouiller votre coffre-fort Mosaic",
  "mobile.unlock.faceIdReason":
    "Mosaic utilise Face ID pour déverrouiller rapidement votre coffre-fort. Votre mot de passe principal fonctionne toujours aussi.",
  "mobile.unlock.quickInvalidated":
    "Vos réglages Face ID ont changé. Déverrouillez avec votre mot de passe principal pour réactiver le déverrouillage rapide.",
  "mobile.unlock.storeBusy": "Mosaic est occupé. Réessayez dans un instant.",
  "mobile.unlock.rebootNote":
    "Déverrouillez une fois avec votre mot de passe principal après un redémarrage.",
  "mobile.capture.hidden": "Masqué pendant l’enregistrement de votre écran",
  "mobile.deviceLock.required.title": "Définissez d’abord un verrouillage d’écran",
  "mobile.deviceLock.required.body":
    "Mosaic garde vos clés derrière le verrouillage d’écran de votre téléphone. Définissez un code PIN, un schéma ou un mot de passe dans les réglages du système, puis revenez.",
  "mobile.deviceLock.required.action": "Ouvrir les réglages de sécurité",
  "mobile.common.show": "Afficher",
  "mobile.common.hide": "Masquer",
  "mobile.common.copy": "Copier",
  "mobile.common.save": "Enregistrer",
  "mobile.common.remove": "Retirer",
  "mobile.common.edit": "Modifier",
  "mobile.common.working": "En cours",
  "mobile.vault.add": "Ajouter un élément",
  "mobile.vault.addLogin": "Identifiant",
  "mobile.vault.addNote": "Note sécurisée",
  "mobile.vault.untitled": "Sans titre",
  "mobile.vault.favorite": "Favori",
  "mobile.search.hint": "Recherchez par titre, nom d’utilisateur, site web ou étiquette.",
  "mobile.search.none": "Aucun élément ne correspond à {query}.",
  "mobile.detail.favoriteAdd": "Ajouter aux favoris",
  "mobile.detail.favoriteRemove": "Retirer des favoris",
  "mobile.detail.delete": "Mettre à la corbeille",
  "mobile.detail.deleteTitle": "Mettre « {title} » à la corbeille ?",
  "mobile.detail.deleteBody": "Le changement est synchronisé avec vos autres appareils.",
  "mobile.detail.otpLeft": "{seconds, plural, one {# seconde restante} other {# secondes restantes}}",
  "mobile.detail.passkey": "Clé d’accès pour {site}",
  "mobile.detail.attachments":
    "{count, plural, one {# fichier est joint. Ouvrez-le sur votre ordinateur.} other {# fichiers sont joints. Ouvrez-les sur votre ordinateur.}}",
  "mobile.detail.linkedApp": "Application : {name}",
  "mobile.detail.needsLinking": "Doit être associé avant de pouvoir remplir",
  "mobile.detail.linkApp": "Associer une application",
  "mobile.editor.newLogin": "Nouvel identifiant",
  "mobile.editor.editLogin": "Modifier l’identifiant",
  "mobile.editor.newNote": "Nouvelle note",
  "mobile.editor.editNote": "Modifier la note",
  "mobile.editor.title": "Titre",
  "mobile.editor.username": "Nom d’utilisateur ou e-mail",
  "mobile.editor.password": "Mot de passe",
  "mobile.editor.passwordKeep": "Laissez vide pour conserver le mot de passe actuel.",
  "mobile.editor.websites": "Sites web",
  "mobile.editor.websiteHint": "https://example.com",
  "mobile.editor.websiteAdd": "Ajouter un site web",
  "mobile.editor.websiteRemove": "Retirer le site web {number}",
  "mobile.editor.notes": "Notes",
  "mobile.editor.favorite": "Marquer comme favori",
  "mobile.editor.saving": "Enregistrement",
  "mobile.editor.otpTitle": "Code à usage unique",
  "mobile.editor.otpHave": "Cet identifiant a un code à usage unique.",
  "mobile.editor.otpInput": "Clé de configuration ou lien otpauth",
  "mobile.editor.otpAdd": "Ajouter le code",
  "mobile.editor.otpReady": "Prêt : {name}. Il est enregistré avec l’identifiant.",
  "mobile.editor.otpRemove": "Retirer le code",
  "mobile.editor.otpRemoved": "Le code sera retiré à l’enregistrement.",
  "mobile.editor.generate": "Générer un mot de passe",
  "mobile.generator.title": "Générateur de mots de passe",
  "mobile.generator.password": "Caractères",
  "mobile.generator.passphrase": "Mots",
  "mobile.generator.length": "Longueur : {length}",
  "mobile.generator.lowercase": "Lettres minuscules",
  "mobile.generator.uppercase": "Lettres majuscules",
  "mobile.generator.digits": "Chiffres",
  "mobile.generator.symbols": "Symboles",
  "mobile.generator.ambiguous": "Exclure les caractères qui se ressemblent",
  "mobile.generator.wordCount": "Mots : {count}",
  "mobile.generator.separator": "Séparateur",
  "mobile.generator.capitalize": "Mettre les mots en majuscule",
  "mobile.generator.appendDigit": "Ajouter un chiffre",
  "mobile.generator.run": "Générer",
  "mobile.generator.use": "Utiliser ce mot de passe",
  "mobile.generator.strength": "Environ {bits} bits de robustesse",
  "mobile.link.title": "Associer une application",
  "mobile.link.choose": "Choisissez l’application à associer à {login}.",
  "mobile.link.filter": "Trouver une application",
  "mobile.link.app": "Application",
  "mobile.link.package": "ID de l’application",
  "mobile.link.installer": "Installée depuis",
  "mobile.link.installerUnknown": "Source inconnue",
  "mobile.link.certificate": "Signature du développeur (début)",
  "mobile.link.verified": "Vérifiée par {host}",
  "mobile.link.notVerified": "Non vérifiée",
  "mobile.link.unavailable": "Vérification impossible",
  "mobile.link.checking": "Vérification du site web",
  "mobile.link.warning":
    "N’associez que des applications de confiance. Mosaic ne peut pas confirmer que cette application appartient à {host}.",
  "mobile.link.warningNoHost":
    "N’associez que des applications de confiance. Cet identifiant n’a aucun site web que Mosaic puisse vérifier.",
  "mobile.link.confirm": "Remplir {login} dans cette application à partir de maintenant",
  "mobile.link.done": "Associé. Mosaic remplit {login} dans cette application.",
  "mobile.link.noApps": "Aucune autre application trouvée.",
  "mobile.settings.security": "Sécurité",
  "mobile.settings.autoLock": "Verrouiller quand vous quittez l’application",
  "mobile.settings.autoLock.now": "Immédiatement",
  "mobile.settings.autoLock.s30": "Après 30 secondes",
  "mobile.settings.autoLock.s60": "Après 1 minute",
  "mobile.settings.autoLock.s300": "Après 5 minutes",
  "mobile.settings.idle": "Verrouiller quand vous ne touchez plus l’écran",
  "mobile.settings.idleOption": "{minutes, plural, one {Après # minute} other {Après # minutes}}",
  "mobile.settings.clipboard": "Effacer les valeurs copiées",
  "mobile.settings.clipboardOption":
    "{seconds, plural, one {Après # seconde} other {Après # secondes}}",
  "mobile.settings.quick": "Déverrouillage rapide",
  "mobile.settings.quickOn": "Le déverrouillage rapide est activé.",
  "mobile.settings.quickOff": "Le déverrouillage rapide est désactivé.",
  "mobile.settings.quickEnable": "Activer",
  "mobile.settings.quickDisable": "Désactiver",
  "mobile.settings.quickPassword":
    "Saisissez votre mot de passe principal pour activer le déverrouillage rapide.",
  "mobile.settings.sync": "Synchronisation",
  "mobile.about.preAudit":
    "Mosaic est un logiciel avant audit. Aucun audit de sécurité indépendant n’a encore eu lieu.",
  "mobile.about.privacy":
    "Aucune analyse d’usage, aucun suivi et aucun rapport de plantage ne quitte ce téléphone.",
  "mobile.about.licences": "Licences open source",
  "mobile.licences.title": "Licences open source",
  "mobile.licences.intro": "Mosaic pour Android est construit avec ces composants open source.",
  "mobile.licences.rust":
    "Le noyau Rust utilise des crates open source. Leurs licences sont listées dans le dépôt source du projet.",
  "mobile.autofill.unlockEntry": "Déverrouiller Mosaic",
  "mobile.autofill.noLogin": "Aucun identifiant pour {app}",
  "mobile.autofill.noLoginHint": "Touchez pour associer un identifiant",
  "mobile.autofill.cannotCheck": "Vérification de cette application impossible",
  "mobile.autofill.warning": "Vérifiez d’abord l’adresse",
  "mobile.autofill.noUsername": "Aucun nom d’utilisateur",
  "mobile.autofill.otpEntry": "Code à usage unique",
  "mobile.link.pickLogin": "Choisissez l’identifiant que Mosaic doit remplir dans {app}.",
  "mobile.credential.unlockEntry": "Déverrouiller Mosaic",
  "mobile.credential.passwordEntry": "Mot de passe",
  "mobile.credential.passkeyEntry": "Clé d’accès",
  "mobile.credential.createEntry": "Enregistrer la clé d’accès dans Mosaic",
  "mobile.credential.createTitle": "Enregistrer une clé d’accès ?",
  "mobile.credential.createBody": "Mosaic va enregistrer une clé d’accès pour {site} ({user}).",
  "mobile.credential.createVault": "Enregistrer dans",
  "mobile.credential.vaultDefault": "Coffre-fort personnel",
  "mobile.credential.createConfirm": "Enregistrer la clé d’accès",
  "mobile.credential.verifyTitle": "Confirmez que c’est bien vous",
  "mobile.credential.verifySignIn": "Se connecter à {site}",
  "mobile.credential.verifyCreate": "Enregistrer une clé d’accès pour {site}",
  "mobile.credential.verifyFailed":
    "Mosaic n’a pas pu confirmer que c’est bien vous. Réessayez.",
  "mobile.credential.unsupported":
    "Mosaic ne peut pas répondre à cette demande de connexion. Annulez et choisissez un autre moyen de vous connecter.",
  "mobile.settings.fill": "Remplir dans d’autres applications",
  "mobile.settings.autofillOn": "Mosaic est votre service de saisie automatique.",
  "mobile.settings.autofillOff": "Mosaic n’est pas encore votre service de saisie automatique.",
  "mobile.settings.autofillEnable": "Utiliser Mosaic pour la saisie automatique",
  "mobile.settings.credentialHint":
    "Android peut aussi proposer vos clés d’accès et mots de passe depuis Mosaic quand une application vous demande de vous connecter.",
  "mobile.settings.credentialEnable": "Utiliser Mosaic pour les clés d’accès et les mots de passe",
  "mobile.scan.setupAction": "Scanner le code de configuration",
  "mobile.scan.otpAction": "Scanner un code QR",
  "mobile.scan.otpTitle": "Scanner le code à usage unique",
  "mobile.scan.otpHint":
    "Placez la caméra devant le code QR du site web. Mosaic le lit uniquement sur ce téléphone.",
  "mobile.scan.preview": "Vue de la caméra",
  "mobile.scan.looking": "Recherche d’un code QR",
  "mobile.scan.notSetupCode": "Ce n’est pas un code de configuration Mosaic",
  "mobile.scan.notSetupCodeHelp":
    "Placez la caméra devant le code que Mosaic affiche sur votre ordinateur, dans Réglages sous Synchronisation.",
  "mobile.scan.rationale":
    "Mosaic n’utilise la caméra que pour lire ce code QR. Rien n’est enregistré ni envoyé.",
  "mobile.scan.allow": "Autoriser la caméra",
  "mobile.scan.denied":
    "Mosaic ne peut pas utiliser la caméra. Autorisez-la dans les réglages de ce téléphone pour scanner, ou saisissez les informations à la main.",
  "mobile.scan.openSettings": "Ouvrir les réglages",
  "mobile.scan.unavailable":
    "Mosaic n’a pas pu démarrer la caméra. Saisissez les informations à la main.",
  "mobile.onboarding.details.scanned":
    "Le serveur, l’e-mail et la clé secrète sont renseignés à partir du code de configuration. Vérifiez-les, puis continuez.",
  "mobile.onboarding.details.scanOr": "Ou saisissez les informations à la main",
  "mobile.onboarding.details.keyFromCode": "Clé secrète issue du code de configuration",
  "mobile.settings.passkeyIdentities": "Utiliser Mosaic pour les clés d’accès",
  "mobile.settings.passkeyIdentitiesHint":
    "Mosaic indique au système le site web et le nom d’utilisateur de chaque clé d’accès pour qu’il puisse la proposer. Les clés ne quittent jamais Mosaic.",
  "mobile.settings.passwordIdentities": "Proposer des identifiants au-dessus du clavier",
  "mobile.settings.passwordIdentitiesHint":
    "Mosaic indique au système le site web et le nom d’utilisateur de chaque identifiant. Le système conserve cette liste sur le téléphone, en dehors du chiffrement de Mosaic.",
  "mobile.settings.identitiesPassword":
    "Saisissez votre mot de passe principal pour l’activer.",
  "mobile.licences.introIos": "Mosaic pour iPhone est construit avec ces composants open source.",
  "mobile.ext.openApp":
    "Ouvrez Mosaic et déverrouillez-le une fois avec votre mot de passe principal.",
  "mobile.ext.noAccount":
    "Mosaic n’a pas encore de compte sur cet iPhone. Ouvrez Mosaic pour le configurer.",
  "mobile.ext.loading": "Ouverture de votre coffre-fort",
  "mobile.ext.thisSite": "ce site",
  "mobile.ext.matches": "Identifiants pour {site}",
  "mobile.ext.noMatches": "Aucun identifiant pour {site}",
  "mobile.ext.noMatchesHint":
    "Mosaic ne remplit que les identifiants enregistrés pour ce site. Ouvrez Mosaic pour ajouter ce site à un identifiant.",
  "mobile.ext.searchAll": "Rechercher dans tous les identifiants",
  "mobile.ext.hideAll": "Masquer les autres identifiants",
  "mobile.ext.notSavedFor": "Non enregistré pour {site}",
  "mobile.ext.fillHint": "Remplit cet identifiant",
  "mobile.ext.warnPunycode":
    "Cette adresse est écrite avec des caractères spéciaux : {unicode}. Vérifiez que c’est bien le site que vous voulez.",
  "mobile.ext.warnLookalike":
    "Cette adresse ressemble à {site}, que vous avez enregistré dans Mosaic. Vérifiez que c’est bien le site que vous voulez.",
  "mobile.ext.otpTitle": "Choisir un code à usage unique",
  "mobile.ext.passkeyTitle": "Choisir une clé d’accès",
  "mobile.ext.configTitle": "Saisie automatique Mosaic",
  "mobile.ext.configBody":
    "Mosaic propose vos identifiants, clés d’accès et codes à usage unique quand vous touchez un champ ou un bouton de connexion. Vous modifiez ce qu’il propose dans les réglages de l’application Mosaic.",
  "mobile.ext.confirmFillTitle": "Remplir l’identifiant pour {site} ?",
  "mobile.ext.confirmFillBody":
    "iOS ne dit pas à Mosaic sur quelle page vous êtes, Mosaic ne peut donc pas la vérifier. Ne remplissez que si c’est bien le site que vous voulez. Mosaic demande ensuite Face ID ou Touch ID.",
  "mobile.ext.confirmFillButton": "Remplir cet identifiant",
  "mobile.settings.passwordIdentitiesLimit":
    "Quand vous touchez une suggestion, iOS ne dit pas à Mosaic sur quelle page vous êtes. Mosaic affiche donc le site enregistré et ne remplit qu’après votre confirmation et un déverrouillage avec Face ID ou Touch ID.",
} satisfies Record<keyof typeof en, string>;
