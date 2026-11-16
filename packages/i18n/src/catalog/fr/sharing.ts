/**
 * French UI strings: sharing. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { sharing as en } from "../en/sharing.ts";

export const sharing = {
  "sharing.title": "Partage",
  "sharing.body":
    "Partagez des coffres-forts avec des personnes que vous connaissez. Tout reste chiffré : seules les personnes que vous invitez peuvent lire un coffre-fort partagé, et Mosaic ne partage qu’avec des personnes dont vous avez comparé le code de sécurité.",
  "sharing.notHosted.none":
    "Le partage nécessite un serveur Mosaic. Activez d’abord la synchronisation avec un serveur Mosaic dans les réglages de synchronisation.",
  "sharing.notHosted.folder":
    "Le partage n’est pas disponible avec un dossier de synchronisation. Un dossier ne peut pas savoir qui a le droit de lire un coffre-fort, Mosaic garde donc les coffres-forts d’un dossier privés. Pour partager, déconnectez le dossier et connectez un serveur Mosaic dans les réglages de synchronisation.",
  "sharing.notHosted":
    "Le partage nécessite un serveur Mosaic. Connectez-en un d’abord dans les réglages de synchronisation.",
  "sharing.syncSettings": "Ouvrir les réglages de synchronisation",
  "sharing.ownCode.title": "Votre code de sécurité",
  "sharing.ownCode.body":
    "Lisez ce code à la personne avec qui vous partagez, en personne ou au téléphone. S’il correspond à ce qu’elle voit pour vous, personne ne s’interpose entre vous.",
  "sharing.ownCode.pending":
    "Votre code de sécurité apparaît après la première synchronisation avec le serveur.",
  "sharing.contacts.title": "Personnes que vous avez recherchées",
  "sharing.contacts.empty": "Vous n’avez encore recherché personne.",
  "sharing.contacts.verified": "Vérifié",
  "sharing.contacts.notVerified": "Non vérifié",
  "sharing.contacts.changed": "Nouvelle vérification nécessaire",
  "sharing.contacts.verify": "Comparer le code…",
  "sharing.contacts.verifyOf": "Comparer le code avec {email}",
  "sharing.contacts.review": "Examiner le changement…",
  "sharing.contacts.reviewOf": "Examiner le changement pour {email}",
  "sharing.share.open": "Partager…",
  "sharing.share.title": "Partager {vault}",
  "sharing.share.dialog": "Partage : {vault}",
  "sharing.share.roleStep": "Choisir un rôle pour {email}",
  "sharing.lookup.email": "Son adresse e-mail",
  "sharing.lookup.find": "Rechercher",
  "sharing.lookup.searching": "Recherche…",
  "sharing.lookup.notFoundHint":
    "Mosaic ne peut pas vous dire si une adresse a un compte. Si le code ci-dessus ne correspond pas à ce que la personne voit, ne partagez pas.",
  "sharing.code.title": "Comparer les codes de sécurité",
  "sharing.code.compare":
    "Comparez ce code avec {email}, en personne ou au téléphone. N’utilisez ni e-mail ni messagerie.",
  "sharing.code.compareNew":
    "Comparez le nouveau code avec {email}, en personne ou au téléphone. N’utilisez ni e-mail ni messagerie.",
  "sharing.code.shown": "Code affiché par Mosaic pour {email}",
  "sharing.code.typed": "Code qu’elle vous a lu (facultatif)",
  "sharing.code.typedMatch": "C’est le code ci-dessus.",
  "sharing.code.attest":
    "J’ai comparé ce code avec {email} par téléphone ou en personne, et il est identique.",
  "sharing.code.matches": "Les codes correspondent",
  "sharing.code.mismatch":
    "Les codes diffèrent. Ne partagez pas. Quelqu’un s’interpose peut-être entre vous et {email}.",
  "sharing.code.mismatchHelp":
    "Vérifiez l’adresse et demandez à {email} de relire le code. S’il diffère toujours, annulez et prévenez la personne par un autre moyen.",
  "sharing.code.verified": "{email} est vérifié.",
  "sharing.code.status.new": "Nouveau contact",
  "sharing.code.status.pinned": "Pas encore vérifié",
  "sharing.code.status.verified": "Vérifié",
  "sharing.code.status.changed": "Clé modifiée",
  "sharing.code.status.conflicting": "Désaccord entre appareils",
  "sharing.code.conflicting":
    "Vos appareils ont vu des clés différentes pour {email}. Comparez de nouveau le code avec la personne avant de partager.",
  "sharing.verify.title": "Vérifier {email}",
  "sharing.role.label": "Rôle",
  "sharing.role.owner": "Propriétaire",
  "sharing.role.manage": "Gestionnaire",
  "sharing.role.write": "Éditeur",
  "sharing.role.read": "Lecteur",
  "sharing.role.desc.manage": "Peut inviter et retirer des personnes et modifier les rôles.",
  "sharing.role.desc.write": "Peut ajouter et modifier des éléments.",
  "sharing.role.desc.read": "Peut lire uniquement.",
  "sharing.role.honesty":
    "Toute personne qui peut lire un élément peut le copier. Un lecteur ne peut pas modifier le coffre-fort, mais rien ne l’empêche de garder ce qu’il voit, et retirer quelqu’un plus tard ne reprend pas ce qu’il a déjà téléchargé.",
  "sharing.invite.send": "Envoyer l’invitation",
  "sharing.invite.sent":
    "Invitation envoyée à {email}. La personne doit ouvrir Mosaic et l’accepter.",
  "sharing.invite.sentTitle": "Invitation envoyée",
  "sharing.invite.pending": "En attente de {email}",
  "sharing.invite.expires": "Expire {when}",
  "sharing.invite.cancel": "Retirer l’invitation",
  "sharing.invite.cancelOf": "Retirer l’invitation de {email}",
  "sharing.invite.cancelled": "Invitation retirée.",
  "sharing.inbox.title": "Invitations",
  "sharing.inbox.nav": "Invitations",
  "sharing.inbox.navCount": "Invitations, {count} en attente",
  "sharing.inbox.intro":
    "Les personnes qui vous ont invité à un coffre-fort partagé. Mosaic vérifie chaque invitation avant de vous la montrer.",
  "sharing.inbox.empty":
    "Aucune invitation pour le moment. Quand quelqu’un vous invite à un coffre-fort partagé, l’invitation apparaît ici.",
  "sharing.inbox.from": "De {email}",
  "sharing.inbox.review": "Examiner",
  "sharing.inbox.reviewOf": "Examiner l’invitation de {email}",
  "sharing.inbox.refresh": "Actualiser",
  "sharing.inbox.preview.title": "Rejoindre {vault} ?",
  "sharing.inbox.preview.body": "{email} vous invite en tant que {role}.",
  "sharing.inbox.preview.unnamed": "un coffre-fort partagé",
  "sharing.inbox.preview.owner": "Le coffre-fort appartient à {email}.",
  "sharing.inbox.verifySender":
    "Avant de rejoindre, comparez le code de sécurité de {email}.",
  "sharing.inbox.verifyOwner":
    "{sender} n’est pas le propriétaire. Avant de rejoindre, comparez aussi le code de sécurité du propriétaire : c’est lui qui peut changer la clé du coffre-fort.",
  "sharing.inbox.ownerCode": "Code du propriétaire, {email}",
  "sharing.inbox.accept": "Rejoindre le coffre-fort",
  "sharing.inbox.readyTitle": "Tout est vérifié. Rejoindre ce coffre-fort ?",
  "sharing.inbox.decline": "Refuser",
  "sharing.inbox.block": "Refuser et bloquer l’expéditeur",
  "sharing.inbox.block.title": "Refuser et bloquer cet expéditeur ?",
  "sharing.inbox.block.body":
    "Mosaic refuse l’invitation et ignorera désormais toutes les invitations de {email}. Vous ne pourrez pas annuler cela dans Mosaic.",
  "sharing.inbox.unreadable":
    "Cette invitation ne peut pas être ouverte. Elle est peut-être endommagée ou ne vous est pas destinée. Demandez à {email} de la renvoyer.",
  "sharing.inbox.joined":
    "Vous avez rejoint le coffre-fort. Il apparaît dans votre liste de coffres-forts dans un instant.",
  "sharing.inbox.declined": "Invitation refusée.",
  "sharing.inbox.outgoing": "Invitations que vous avez envoyées",
  "sharing.inbox.outgoing.empty": "Aucune de vos invitations n’attend de réponse.",
  "sharing.inbox.unknownVault": "un coffre-fort",
  "sharing.members.title": "Personnes ayant accès",
  "sharing.members.you": "Vous",
  "sharing.members.unverified": "Non vérifié",
  "sharing.members.unconfirmed": "Absent de la liste des membres",
  "sharing.members.unconfirmed.help":
    "Le serveur liste cette personne, mais la liste des membres du coffre-fort ne la contient pas. Le prochain changement de clé l’exclura. Si vous ne la connaissez pas, retirez-la.",
  "sharing.members.dropped": "N’est plus sur le serveur",
  "sharing.members.dropped.help":
    "Cette personne figure dans la liste des membres du coffre-fort, mais le serveur ne la liste plus. Elle est peut-être partie. Vérifiez que c’est attendu.",
  "sharing.members.invited": "Invité",
  "sharing.members.removing": "Retrait en cours",
  "sharing.members.verified": "Vérifié",
  "sharing.members.verify": "Vérifier…",
  "sharing.members.verifyOf": "Vérifier {email}",
  "sharing.members.remove": "Retirer",
  "sharing.members.removeOf": "Retirer {email}",
  "sharing.members.remove.title": "Retirer {email} ?",
  "sharing.members.remove.confirm":
    "{email} conserve tout ce qu’elle a déjà téléchargé. Mosaic change maintenant la clé du coffre-fort, et elle ne peut rien lire de nouveau.",
  "sharing.members.remove.confirmManager":
    "{email} conserve tout ce qu’elle a déjà téléchargé. Le serveur cesse maintenant de lui envoyer les mises à jour. L’appareil du propriétaire du coffre-fort change la clé du coffre-fort à sa prochaine synchronisation, et d’ici là seul le serveur tient {email} à l’écart.",
  "sharing.members.cancel": "Annuler",
  "sharing.members.removed": "Retiré.",
  "sharing.members.leave": "Quitter le coffre-fort",
  "sharing.members.leave.title": "Quitter ce coffre-fort ?",
  "sharing.members.leave.confirm":
    "La copie sur cet appareil reste lisible, mais n’est plus mise à jour. Pour le rejoindre de nouveau, il vous faut une nouvelle invitation.",
  "sharing.members.left": "Vous avez quitté le coffre-fort.",
  "sharing.members.roleOf": "Rôle de {email}",
  "sharing.members.changeRole": "Changer le rôle",
  "sharing.members.changeRoleOf": "Changer le rôle de {email}",
  "sharing.members.invite": "Inviter quelqu’un…",
  "sharing.members.empty": "Pour l’instant, vous seul pouvez ouvrir ce coffre-fort.",
  "sharing.members.unknownEmail": "Adresse inconnue",
  "sharing.members.managerNote":
    "Vous pouvez inviter et retirer des personnes. Seul l’appareil du propriétaire change la clé du coffre-fort, un retrait est donc complet une fois que l’appareil du propriétaire a synchronisé.",
  "sharing.members.readerNote":
    "Vous pouvez lire ce coffre-fort, mais pas modifier qui y a accès.",
  "sharing.rotation.running":
    "Un retrait attend que l’appareil du propriétaire change la clé du coffre-fort. Tant qu’il n’a pas synchronisé, seul le serveur tient la personne retirée à l’écart.",
  "sharing.rotation.unprotected":
    "{count, plural, one {# fichier garde} other {# fichiers gardent}} l’ancienne protection, car cet appareil ne {count, plural, one {l’a} other {les a}} pas. Une personne que vous avez retirée peut encore {count, plural, one {l’ouvrir} other {les ouvrir}} si elle a gardé l’ancienne clé. {count, plural, one {Ajoutez-le} other {Ajoutez-les}} de nouveau depuis un appareil qui {count, plural, one {l’a} other {les a}}.",
  "sharing.keyChanged.title": "Le code de sécurité de {email} a changé",
  "sharing.keyChanged.body":
    "Le serveur montre maintenant une clé différente pour {email}. Cela arrive quand la personne a reconfiguré Mosaic, et c’est aussi à quoi ressemblerait une attaque sur le serveur. Rien n’est partagé avec la nouvelle clé tant que vous n’avez pas comparé les codes avec la personne, en personne ou au téléphone.",
  "sharing.keyChanged.old": "Code que vous aviez comparé auparavant",
  "sharing.keyChanged.new": "Nouveau code",
  "sharing.keyChanged.accept": "Le nouveau code correspond",
  "sharing.finding.region": "Alertes de partage",
  "sharing.finding.summary":
    "{count, plural, one {# alerte de partage} other {# alertes de partage}}",
  "sharing.finding.review": "Examiner",
  "sharing.finding.collapse": "Réduire",
  "sharing.finding.dismiss": "Fermer",
  "sharing.finding.unknownEmail": "une personne inconnue",
  "sharing.finding.verify": "Vérifier…",
  "sharing.finding.remove": "Retirer",
  "sharing.finding.removeOf": "Retirer {email} de {vault}",
  "sharing.finding.identityMismatch":
    "Le serveur montre pour votre compte une clé de partage différente de celle créée par cet appareil, le partage est donc suspendu. Ne partagez rien, et vérifiez votre serveur et vos autres appareils. Choisissez ensuite « J’ai vérifié… ».",
  "sharing.finding.rosterInvalid":
    "La liste des membres de {vault} est endommagée, ou quelqu’un l’a modifiée sans autorisation. N’ajoutez personne avant d’avoir vérifié qui a accès. Choisissez ensuite « J’ai vérifié… ».",
  "sharing.finding.shareUntrusted":
    "Une nouvelle clé pour {vault} a été refusée : {sender} l’a envoyée, mais n’a pas le droit de changer la clé de ce coffre-fort. Le coffre-fort garde son ancienne clé. Demandez au propriétaire s’il l’a changée. Choisissez ensuite « J’ai vérifié… ».",
  "sharing.finding.memberUnverified":
    "Vous n’avez pas vérifié {email} dans {vault}. Mosaic ne lui envoie aucune nouvelle clé, et le prochain changement de clé l’exclura. Comparez son code de sécurité, ou retirez-la.",
  "sharing.finding.memberDropped":
    "{email} figure dans la liste des membres de {vault}, mais le serveur ne la liste plus. Elle est peut-être partie, ou le serveur l’a retirée. Vérifiez que c’est attendu, et retirez-la si ce n’est pas le cas.",
  "sharing.finding.memberKeyMismatch":
    "Mosaic a reçu pour {email} une clé qui ne correspond pas à celle que vous avez vérifiée, elle a donc été retirée de {vault}. Comparez de nouveau les codes de sécurité avec elle avant de l’inviter à nouveau.",
  "sharing.finding.vaultHeld":
    "{vault} est en attente que vos autres appareils confirment à qui il appartient. Ouvrez Mosaic sur un appareil qui utilise déjà ce coffre-fort.",
  "sharing.finding.accessLost":
    "Vous n’avez plus accès à {vault}. La copie sur cet appareil reste lisible mais n’est plus mise à jour. Demandez au propriétaire de vous inviter de nouveau.",
  "sharing.finding.rotationBlocked":
    "La clé de {vault} n’a pas pu être changée après le retrait de quelqu’un, cette personne détient donc encore l’ancienne clé. Synchronisez de nouveau. Si le problème persiste, ouvrez la liste des membres et vérifiez qui a accès.",
  "emergency.title": "Accès d’urgence",
  "emergency.body":
    "Désignez des personnes qui peuvent demander l’accès à vos coffres-forts s’il vous arrive quelque chose. Ce n’est pas une récupération de compte : elles obtiennent un accès en lecture seule aux coffres-forts que vous choisissez, seulement après un délai d’attente, et vous pouvez refuser pendant ce délai. Cela ne remplace pas votre kit d’urgence.",
  "emergency.folderUnsupported":
    "L’accès d’urgence nécessite le serveur Mosaic. Un dossier de synchronisation ne peut pas imposer de délai d’attente.",
  "emergency.granted.title": "Personnes qui peuvent demander l’accès",
  "emergency.granted.empty": "Personne ne peut demander l’accès à vos coffres-forts.",
  "emergency.granted.add": "Ajouter une personne…",
  "emergency.granted.scopeAll": "Tous vos coffres-forts",
  "emergency.granted.scopeSome": "{count, plural, one {# coffre-fort} other {# coffres-forts}}",
  "emergency.granted.wait": "Délai d’attente : {days, plural, one {# jour} other {# jours}}",
  "emergency.granted.waitingUntil":
    "A demandé l’accès. Il s’ouvre {when} sauf si vous refusez.",
  "emergency.granted.approved": "Vous avez approuvé la demande.",
  "emergency.granted.released": "L’accès a été libéré. La personne peut lire les coffres-forts.",
  "emergency.granted.idle": "Aucune demande",
  "emergency.granted.remove": "Retirer l’accès",
  "emergency.granted.removeOf": "Retirer l’accès de {email}",
  "emergency.granted.remove.title": "Retirer l’accès d’urgence de {email} ?",
  "emergency.granted.remove.body":
    "Elle ne pourra plus faire de demande. Si son accès avait déjà été libéré, Mosaic change les clés des coffres-forts qu’elle a ouverts ; elle conserve ce qu’elle a déjà téléchargé.",
  "emergency.granted.removed": "Accès d’urgence retiré.",
  "emergency.grant.title": "Autoriser l’accès d’urgence",
  "emergency.grant.lookup": "Qui doit pouvoir faire la demande ?",
  "emergency.grant.optionsTitle": "Ce que {email} peut ouvrir",
  "emergency.grant.scope": "Ce qu’elle peut ouvrir",
  "emergency.grant.scope.all": "Tous mes coffres-forts, y compris les nouveaux",
  "emergency.grant.scope.selected": "Uniquement les coffres-forts que je choisis",
  "emergency.grant.scope.pick": "Choisissez au moins un coffre-fort.",
  "emergency.grant.wait": "Délai d’attente",
  "emergency.grant.wait.option": "{days, plural, one {# jour} other {# jours}}",
  "emergency.grant.waitHint":
    "Ouvrez Mosaic au moins une fois dans ce délai après une demande, pour pouvoir la voir et refuser. Une fois le délai écoulé, la personne obtient un accès en lecture seule sans autre question posée à vous.",
  "emergency.grant.serverTrust":
    "Le serveur garde le délai d’attente. Si la personne que vous désignez et celle qui exploite le serveur s’entendent, elles pourraient le contourner. Le serveur seul ne peut pas lire vos coffres-forts.",
  "emergency.grant.vaultKeysOnly":
    "Cela ne couvre que les clés de vos coffres-forts. Cela ne donne pas accès à votre compte, à votre mot de passe principal ni à votre clé secrète.",
  "emergency.grant.submit": "Autoriser l’accès d’urgence",
  "emergency.grant.done": "{email} peut maintenant demander l’accès d’urgence.",
  "emergency.grant.needVerify": "Comparez d’abord le code de sécurité de {email}.",
  "emergency.received.title": "Personnes qui vous ont désigné",
  "emergency.received.empty": "Personne ne vous a désigné pour l’accès d’urgence.",
  "emergency.received.request": "Demander l’accès",
  "emergency.received.requestOf": "Demander l’accès à {email}",
  "emergency.received.request.title": "Demander l’accès à {email} ?",
  "emergency.received.request.body":
    "{email} voit votre demande à sa prochaine ouverture de Mosaic. Après {days, plural, one {# jour} other {# jours}}, vous pouvez ouvrir ses coffres-forts, en lecture seule, sauf si elle refuse.",
  "emergency.received.requested":
    "Demande envoyée. {email} la voit à sa prochaine ouverture de Mosaic.",
  "emergency.received.requestHint":
    "{email} voit votre demande dans Mosaic. Après {days, plural, one {# jour} other {# jours}}, vous pouvez ouvrir ses coffres-forts, en lecture seule, sauf si elle refuse.",
  "emergency.received.countdown": "Vous pouvez ouvrir ses coffres-forts {when}.",
  "emergency.received.waitingLive": "Votre demande est en attente.",
  "emergency.received.claim": "Ouvrir ses coffres-forts",
  "emergency.received.claimOf": "Ouvrir ses coffres-forts : {email}",
  "emergency.received.claimed":
    "{count, plural, one {# coffre-fort a été ajouté} other {# coffres-forts ont été ajoutés}}, en lecture seule.",
  "emergency.received.cooldown": "{email} a refusé. Vous pouvez redemander {when}.",
  "emergency.received.verifyFirst":
    "Comparez le code de sécurité de {email} avant d’ouvrir ses coffres-forts. Mosaic n’utilise qu’une clé que vous avez vérifiée.",
  "emergency.received.verify": "Comparer le code…",
  "emergency.received.verifyOf": "Comparer le code avec {email}",
  "emergency.received.approved": "{email} a approuvé votre demande.",
  "emergency.received.waitDays": "Délai d’attente : {days, plural, one {# jour} other {# jours}}",
  "emergency.banner.region": "Demande d’accès d’urgence",
  "emergency.banner.body":
    "{email} a demandé l’accès d’urgence à vos coffres-forts. Il s’ouvre {when} sauf si vous refusez.",
  "emergency.banner.refuse": "Refuser",
  "emergency.banner.approve": "Approuver maintenant…",
  "emergency.banner.approve.title": "Approuver l’accès d’urgence maintenant ?",
  "emergency.banner.approve.body":
    "{email} obtient immédiatement un accès en lecture seule à vos coffres-forts. Saisissez votre mot de passe principal pour confirmer.",
  "emergency.banner.approve.confirm": "Approuver",
  "emergency.banner.refused": "Demande refusée.",
  "emergency.banner.approved": "Accès approuvé.",
} satisfies Record<keyof typeof en, string>;
