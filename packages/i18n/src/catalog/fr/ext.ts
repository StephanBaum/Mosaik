/**
 * French UI strings: ext. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { ext as en } from "../en/ext.ts";

export const ext = {
  "ext.manifest.name": "Mosaic",
  "ext.manifest.description":
    "Remplissez vos identifiants depuis votre coffre-fort Mosaic. Votre mot de passe principal reste dans l’application de bureau.",
  "ext.command.fill": "Remplir l’identifiant avec Mosaic",
  "ext.command.generate": "Générer un mot de passe avec Mosaic",
  "ext.save.badgeTitle": "Enregistrer l’identifiant ?",
  "ext.loading": "Chargement…",
  "ext.site.secure": "Site actuel, connexion sécurisée (https) : {host}",
  "ext.site.insecure": "Site actuel, non chiffré (http) : {host}",
  "ext.site.none": "Cet onglet n’a pas de site web",
  "ext.nav.label": "Vues",
  "ext.nav.logins": "Identifiants et recherche",
  "ext.nav.generator": "Générateur de mots de passe",
  "ext.footer.open": "Ouvrir Mosaic",
  "ext.footer.more": "Plus d’options",
  "ext.footer.connected": "Connecté à Mosaic",
  "ext.footer.locked": "Mosaic est verrouillé",
  "ext.footer.offline": "Non connecté",
  "ext.more.saveOffers": "Proposer d’enregistrer les identifiants",
  "ext.more.saveOffersHint":
    "Nécessite l’accès à tous les sites web, pour que l’extension puisse remarquer un identifiant que vous envoyez. Rien n’est enregistré tant que vous ne confirmez pas dans l’application Mosaic.",
  "ext.passkeys.toggle": "Utiliser Mosaic pour les clés d’accès",
  "ext.passkeys.hint":
    "Nécessite l’accès à tous les sites web pour répondre aux demandes de clés d’accès. Vous confirmez chacune dans l’application Mosaic.",
  "ext.passkeys.permissionDenied":
    "Mosaic a besoin d’accéder aux sites web pour répondre aux demandes de clés d’accès, et vous ne l’avez pas autorisé.",
  "ext.more.disconnect": "Se déconnecter de Mosaic",
  "ext.more.disconnectTitle": "Déconnecter ce navigateur ?",
  "ext.more.disconnectBody":
    "Ce navigateur ne pourra plus remplir d’identifiants tant que vous ne le reconnecterez pas. Vous approuvez la nouvelle connexion dans l’application Mosaic.",
  "ext.action.cancel": "Annuler",
  "ext.neverSave.menu": "Sites pour lesquels l’enregistrement n’est jamais proposé…",
  "ext.neverSave.title": "Enregistrement jamais proposé",
  "ext.neverSave.intro":
    "Mosaic ne propose pas d’enregistrer les identifiants sur ces sites. Retirez un site pour qu’on vous le redemande.",
  "ext.neverSave.empty":
    "Aucun site pour l’instant. Vous pouvez en ajouter un avec « Jamais pour ce site » quand Mosaic propose d’enregistrer un identifiant.",
  "ext.neverSave.remove": "Retirer",
  "ext.neverSave.removeLabel": "Retirer {origin} de la liste",
  "ext.neverSave.removed":
    "{origin} retiré. Mosaic peut de nouveau proposer d’y enregistrer les identifiants.",
  "ext.neverSave.close": "Fermer",
  "ext.connect.checking": "Connexion à Mosaic…",
  "ext.connect.hostMissing.title": "Mosaic n’est pas configuré pour ce navigateur",
  "ext.connect.hostMissing.body":
    "Installez l’application Mosaic sur cet ordinateur, puis redémarrez le navigateur.",
  "ext.connect.hostMissing.dev":
    "Développeurs : exécutez {command}, puis redémarrez le navigateur.",
  "ext.connect.hostForbidden.title": "Ce navigateur n’est pas autorisé à se connecter",
  "ext.connect.hostForbidden.body":
    "Mosaic ne communique qu’avec l’extension Mosaic officielle. Installez-la depuis la boutique de votre navigateur.",
  "ext.connect.desktopNotRunning.title": "Ouvrez Mosaic sur cet ordinateur",
  "ext.connect.desktopNotRunning.body":
    "L’extension fonctionne via l’application Mosaic. Lancez l’application, puis vérifiez de nouveau.",
  "ext.connect.check": "Vérifier de nouveau",
  "ext.connect.notPaired.title": "Se connecter à Mosaic",
  "ext.connect.notPaired.body":
    "Connectez ce navigateur à l’application Mosaic de cet ordinateur. Vous l’approuvez dans l’application.",
  "ext.connect.start": "Connecter",
  "ext.connect.pairing.title": "Confirmer dans Mosaic",
  "ext.connect.pairing.body":
    "Vérifiez que Mosaic affiche le même code, puis cliquez sur Autoriser dans l’application.",
  "ext.connect.pairing.code": "Code",
  "ext.connect.pairing.waiting": "En attente de votre réponse dans l’application Mosaic…",
  "ext.connect.protocolMismatch.title": "Mettez Mosaic à jour",
  "ext.connect.protocolMismatch.body":
    "L’extension et l’application Mosaic ne sont pas dans la même version. Mettez-les toutes deux à jour avec la dernière version.",
  "ext.connect.error.title": "Mosaic est injoignable",
  "ext.connect.error.body":
    "Vérifiez que l’application Mosaic est en cours d’exécution, puis réessayez.",
  "ext.locked.title": "Mosaic est verrouillé",
  "ext.locked.body": "Déverrouillez-le dans l’application Mosaic pour remplir sur {host}.",
  "ext.locked.bodyNoSite": "Déverrouillez-le dans l’application Mosaic pour l’utiliser ici.",
  "ext.locked.open": "Ouvrir Mosaic",
  "ext.locked.note":
    "Votre mot de passe principal n’est jamais saisi ailleurs que dans l’application Mosaic.",
  "ext.logins.forSite": "Pour {host}",
  "ext.logins.count": "{count, plural, one {# identifiant} other {# identifiants}}",
  "ext.logins.sameSite": "même site",
  "ext.logins.none": "Aucun identifiant enregistré pour {host}.",
  "ext.logins.noneHint":
    "Mosaic compare l’adresse complète, donc les sites qui imitent un autre site n’obtiennent jamais de correspondance.",
  "ext.logins.unsupported":
    "Mosaic ne peut pas remplir sur cette page. Recherchez dans votre coffre-fort pour copier un identifiant.",
  "ext.logins.shortcut": "{shortcut} remplit la première correspondance.",
  "ext.logins.frameNote":
    "Mosaic ne remplit que sur {host}, jamais dans des cadres intégrés d’autres sites.",
  "ext.item.untitled": "Sans titre",
  "ext.item.noUsername": "Aucun nom d’utilisateur",
  "ext.item.twoFactor": "2FA",
  "ext.item.fill": "Remplir",
  "ext.item.fillLabel": "Remplir {name}",
  "ext.item.copyMenu": "Options de copie pour {name}",
  "ext.item.copyUsername": "Copier le nom d’utilisateur",
  "ext.item.copyPassword": "Copier le mot de passe",
  "ext.item.copyCode": "Copier le code",
  "ext.item.blocked":
    "Bloqué : cette page n’est pas chiffrée et cet identifiant ne l’autorise pas.",
  "ext.search.label": "Autres éléments",
  "ext.search.placeholder": "Rechercher dans votre coffre-fort",
  "ext.search.note":
    "Les résultats restent dans cette fenêtre. La page ne reçoit jamais que l’identifiant que vous remplissez.",
  "ext.search.none": "Aucun résultat.",
  "ext.search.copy": "Copier",
  "ext.search.copyOnly": "Copie uniquement : non enregistré pour ce site",
  "ext.shortcut.copyUser": "{platform, select, mac {⌘C} other {Ctrl+C}}",
  "ext.shortcut.copyPassword": "{platform, select, mac {⇧⌘C} other {Ctrl+Maj+C}}",
  "ext.shortcut.copyCode": "{platform, select, mac {⌥⌘C} other {Ctrl+Alt+C}}",
  "ext.search.results": "{count, plural, one {# résultat} other {# résultats}}",
  "ext.frame.cross":
    "Le champ actif se trouve dans un cadre d’un autre site. Mosaic ne remplit pas à cet endroit.",
  "ext.frameConfirm.title": "Remplir dans un cadre intégré ?",
  "ext.frameConfirm.body":
    "Le champ se trouve dans un cadre sur {host}, pas dans la page elle-même. Ne remplissez ici que si vous faites confiance à ce cadre.",
  "ext.frameConfirm.confirm": "Remplir dans le cadre",
  "ext.warning.idn.title": "Vérifiez l’adresse",
  "ext.warning.idn.body":
    "Cette adresse utilise des caractères spéciaux et peut ressembler à un autre site. Vérifiez la barre d’adresse avant de vous connecter.",
  "ext.warning.lookalike.body":
    "Ce site n’est pas {saved}. Il lui ressemble, ce qui est une astuce courante d’hameçonnage. Fermez cet onglet, ou saisissez vous-même {saved}.",
  "ext.warning.youAreOn": "Vous êtes sur",
  "ext.warning.writtenAs": "Écrit comme",
  "ext.warning.savedFor": "Enregistré pour",
  "ext.warning.insecure.title": "Cette page n’est pas chiffrée",
  "ext.warning.insecure.body":
    "La page utilise http. D’autres personnes sur le réseau pourraient lire votre mot de passe. Mosaic ne remplit ici que les identifiants pour lesquels vous l’avez autorisé.",
  "ext.otp.title": "Remplir votre code à usage unique",
  "ext.otp.next": "Mot de passe rempli pour {name}. Ensuite : le code à deux facteurs.",
  "ext.otp.for": "Code pour {name}",
  "ext.otp.fill": "Remplir le code",
  "ext.otp.copy": "Copier le code",
  "ext.otp.note":
    "Le code va de Mosaic directement dans le champ. Cette fenêtre ne l’affiche jamais.",
  "ext.save.titleNew": "Enregistrer cet identifiant pour {host} ?",
  "ext.save.titleUpdate": "Mettre à jour l’identifiant enregistré pour {host} ?",
  "ext.save.generated": "Un mot de passe généré par Mosaic pour {host}.",
  "ext.save.updates": "Met à jour « {title} ».",
  "ext.save.username": "Nom d’utilisateur",
  "ext.save.noUsername": "Aucun",
  "ext.save.password": "Mot de passe",
  "ext.save.passwordHidden": "Masqué. Vous le voyez dans l’application Mosaic.",
  "ext.save.insecure": "Cette page n’est pas chiffrée (http).",
  "ext.save.action": "Enregistrer dans Mosaic…",
  "ext.save.notNow": "Pas maintenant",
  "ext.save.never": "Jamais pour ce site",
  "ext.save.waiting":
    "Confirmez dans l’application Mosaic. Rien n’est enregistré tant que vous ne le faites pas.",
  "ext.save.saved": "Enregistré dans Mosaic.",
  "ext.save.updated": "Mis à jour dans Mosaic.",
  "ext.save.cancelled": "Non enregistré.",
  "ext.save.dismissed": "Abandonné. Le mot de passe est oublié.",
  "ext.save.neverDone": "Mosaic ne proposera plus d’enregistrer sur {host}.",
  "ext.save.permissionAsk":
    "Mosaic demande à votre navigateur l’accès aux sites web. Autorisez-le pour activer cette fonction.",
  "ext.generator.title": "Générer un mot de passe pour {host}",
  "ext.generator.titleGeneric": "Générer un mot de passe",
  "ext.generator.value": "Mot de passe généré",
  "ext.generator.regenerate": "Générer un autre mot de passe",
  "ext.generator.type": "Type de mot de passe",
  "ext.generator.random": "Aléatoire",
  "ext.generator.passphrase": "Phrase secrète",
  "ext.generator.include": "Inclure",
  "ext.generator.rules.page": "Suit les règles de mot de passe de cette page",
  "ext.generator.rules.siteQuirks": "Suit les règles connues pour ce site",
  "ext.generator.fill": "Remplir",
  "ext.generator.fillHint":
    "Pour remplir, cliquez dans un champ de nouveau mot de passe sur la page, puis rouvrez cette fenêtre.",
  "ext.generator.copy": "Copier",
  "ext.generator.expired":
    "Ce mot de passe n’était plus conservé, un nouveau a donc été généré.",
  "ext.generator.filled": "Rempli. Mosaic proposera de l’enregistrer.",
  "ext.copied": "Copié. Effacement dans {seconds, plural, one {# seconde} other {# secondes}}.",
  "ext.copiedNoClear": "Copié.",
  "ext.filled": "Rempli.",
  "ext.filledCode": "Code rempli.",
  "ext.error.invalidRequest":
    "Mosaic n’a pas compris cette demande. Mettez à jour l’extension et l’application Mosaic.",
  "ext.error.notPaired": "Ce navigateur n’est pas connecté à Mosaic. Reconnectez-le.",
  "ext.error.protocolVersion":
    "L’extension et l’application Mosaic ne sont pas dans la même version. Mettez-les toutes deux à jour.",
  "ext.error.busy":
    "Mosaic est occupé avec une autre demande. Réessayez dans un instant.",
  "ext.error.locked": "Mosaic est verrouillé. Déverrouillez-le dans l’application Mosaic.",
  "ext.error.rateLimited":
    "Trop de demandes en peu de temps. Patientez un instant, puis réessayez.",
  "ext.error.originMismatch":
    "Cet identifiant n’est pas enregistré pour ce site web, Mosaic ne le remplira donc pas ici.",
  "ext.error.insecureOrigin":
    "Cette page n’est pas chiffrée et l’identifiant n’autorise pas le remplissage dessus. Rien n’a été rempli.",
  "ext.error.crossOriginFrame":
    "Ce champ se trouve dans un cadre d’un autre site. Mosaic ne remplit pas à cet endroit.",
  "ext.error.itemNotFound":
    "Cet élément n’existe plus. Il a peut-être été déplacé ou supprimé.",
  "ext.error.vaultUnavailable":
    "Ce coffre-fort est fermé. Ouvrez-le d’abord dans l’application Mosaic.",
  "ext.error.noOtp": "Cet élément n’a pas de code à usage unique.",
  "ext.error.otpNotFillable":
    "Ce code s’incrémente à chaque utilisation (HOTP), il ne peut donc pas être rempli ni copié d’ici. Utilisez-le dans l’application Mosaic.",
  "ext.error.fieldEmpty": "Ce champ est vide, il n’y a donc rien à remplir ni à copier.",
  "ext.error.clipboardUnavailable":
    "Mosaic n’a pas pu utiliser le presse-papiers. Réessayez, ou copiez-le dans l’application Mosaic.",
  "ext.error.cancelled":
    "Annulé dans l’application Mosaic. Réessayez quand vous serez prêt.",
  "ext.error.passkeyNotFound": "Mosaic n’a pas de clé d’accès pour ce site.",
  "ext.error.passkeyExcluded":
    "Vous avez déjà une clé d’accès pour ce compte dans Mosaic.",
  "ext.error.rpIdMismatch":
    "Ce site ne peut pas utiliser de clé d’accès pour ce domaine, Mosaic n’a donc pas répondu.",
  "ext.error.passkeyUnsupported":
    "Mosaic ne peut pas créer de clé d’accès avec les options demandées par ce site.",
  "ext.error.pairingDenied":
    "La connexion a été refusée dans l’application Mosaic. Choisissez Connecter pour réessayer, puis choisissez Autoriser.",
  "ext.error.pairingExpired":
    "La demande de connexion a expiré. Choisissez Connecter pour recommencer.",
  "ext.error.pairingConflict":
    "Une autre demande de connexion était ouverte. Choisissez Connecter pour réessayer.",
  "ext.error.pairingBlocked":
    "Trop de connexions ont été refusées. Patientez dix minutes, puis réessayez.",
  "ext.error.pairingLimit":
    "Cet ordinateur a trop de navigateurs connectés. Déconnectez-en un dans les réglages de Mosaic.",
  "ext.error.internal":
    "Mosaic a rencontré un problème inattendu. Réessayez, et si cela se répète, redémarrez l’application Mosaic.",
  "ext.error.desktopNotRunning":
    "Mosaic n’est pas en cours d’exécution sur cet ordinateur. Ouvrez-le, puis réessayez.",
  "ext.error.desktopNotAuthentic":
    "L’application qui a répondu n’est pas l’application Mosaic à laquelle ce navigateur est connecté. Rien n’a été envoyé.",
  "ext.error.timeout":
    "Mosaic n’a pas répondu à temps. Si une boîte de dialogue est ouverte dans l’application Mosaic, répondez-y là-bas.",
  "ext.error.frameTooLarge": "Il y avait trop de données à envoyer à Mosaic.",
  "ext.error.hostMissing":
    "L’assistant Mosaic n’est pas installé pour ce navigateur. Installez l’application Mosaic, puis redémarrez le navigateur.",
  "ext.error.hostForbidden":
    "Ce navigateur ou cette extension n’est pas autorisé à communiquer avec Mosaic.",
  "ext.error.hostExited": "La connexion à Mosaic s’est fermée. Réessayez.",
  "ext.error.unsupportedPage": "Mosaic ne peut pas remplir ce type de page.",
  "ext.error.noFields":
    "Aucun champ d’identifiant trouvé ici. Cliquez dans un champ, puis réessayez.",
  "ext.error.noMatch": "Mosaic n’a aucun identifiant enregistré pour ce site.",
  "ext.error.frameNeedsConfirmation":
    "Le champ se trouve dans un cadre. Ouvrez la fenêtre Mosaic et confirmez le cadre pour le remplir.",
  "ext.error.frameOpaque":
    "Le champ se trouve dans un cadre isolé (sandbox). Mosaic ne remplit pas à cet endroit.",
  "ext.error.originChanged":
    "La page a changé pendant que Mosaic remplissait. Rien n’a été rempli. Réessayez.",
  "ext.error.pageHidden": "La page n’est pas visible, Mosaic n’a donc pas rempli.",
  "ext.error.permissionDenied":
    "Mosaic a besoin d’accéder aux sites web pour remarquer les identifiants, et vous ne l’avez pas autorisé.",
  "ext.error.generatedExpired":
    "Mosaic ne conserve plus ce mot de passe. Générez-en un nouveau.",
  "ext.rotation.card.title": "Changement de mot de passe",
  "ext.rotation.card.for": "Pour {name}",
  "ext.rotation.card.started": "Commencé {when}",
  "ext.rotation.card.delivered": "Le nouveau mot de passe a déjà été rempli une fois.",
  "ext.rotation.card.note":
    "Le nouveau mot de passe n’est affiché que dans l’application Mosaic. Mosaic n’envoie jamais le formulaire à votre place.",
  "ext.rotation.card.list": "Changements de mot de passe en cours",
  "ext.rotation.change": "Changer le mot de passe…",
  "ext.rotation.changeLabel": "Changer le mot de passe… pour {name}",
  "ext.rotation.openPage": "Ouvrir la page de changement",
  "ext.rotation.fillChange": "Remplir le formulaire de changement",
  "ext.rotation.fillPending": "Remplir le nouveau mot de passe pour se connecter",
  "ext.rotation.worked": "Ça a marché",
  "ext.rotation.failed": "Ça n’a pas marché",
  "ext.rotation.fresh": "Recommencer avec un nouveau mot de passe",
  "ext.rotation.waiting": "En attente de votre action dans l’application Mosaic…",
  "ext.rotation.started":
    "Commencé. Choisissez Utiliser ce mot de passe dans l’application Mosaic.",
  "ext.rotation.startCancelled": "Rien n’a été commencé.",
  "ext.rotation.filled":
    "Formulaire de changement rempli. Vérifiez-le, puis envoyez-le vous-même.",
  "ext.rotation.opened": "Page de changement ouverte.",
  "ext.rotation.confirmed": "Terminé. Le nouveau mot de passe est maintenant enregistré dans Mosaic.",
  "ext.rotation.keptOld": "Mosaic a gardé l’ancien mot de passe.",
  "ext.rotation.resultCancelled": "Rien n’a changé. Le changement est toujours ouvert.",
  "ext.rotation.offer.title": "Une adresse associée",
  "ext.rotation.offer.body":
    "Cette page se trouve sur {host}, qui n’est pas enregistré pour {title}. L’ajouter pour que Mosaic puisse y remplir ?",
  "ext.rotation.offer.add": "Ajouter {host}",
  "ext.rotation.offer.added": "Ajouté. Le changement s’affiche maintenant ici.",
  "ext.rotation.offer.cancelled": "Non ajouté.",
  "ext.error.rotationNotFound":
    "Ce changement de mot de passe n’est plus ouvert. Commencez-en un nouveau depuis l’identifiant.",
  "ext.error.originNotRelated":
    "Cette adresse n’est pas sur le même site que cet identifiant, Mosaic ne l’a donc pas ajoutée.",
  "ext.error.rotationLimit":
    "Cet identifiant a déjà quatre changements inachevés. Terminez-en ou annulez-en un d’abord dans l’application Mosaic.",
  "ext.error.changeFormNotFound":
    "Mosaic n’a pas trouvé de formulaire de changement de mot de passe ici. Ouvrez la page de mot de passe de votre compte, puis réessayez.",
  "ext.error.changeFormAmbiguous":
    "Mosaic n’a pas pu déterminer quel champ est lequel. Copiez le nouveau mot de passe depuis Mosaic et collez-le vous-même.",
} satisfies Record<keyof typeof en, string>;
