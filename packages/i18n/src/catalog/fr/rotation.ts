/**
 * French UI strings: rotation. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { rotation as en } from "../en/rotation.ts";

export const rotation = {
  "rotation.section.title": "Mot de passe et clés",
  "rotation.section.body":
    "Changez votre mot de passe principal, obtenez une nouvelle clé secrète ou faites passer tout votre compte à de nouvelles clés. Chaque changement vous montre ce qui va se passer avant de toucher à quoi que ce soit.",
  "rotation.action.password": "Changer le mot de passe principal",
  "rotation.action.password.body":
    "Choisissez un nouveau mot de passe principal. Votre clé secrète et vos coffres-forts restent inchangés.",
  "rotation.action.password.button": "Changer…",
  "rotation.action.secretKey": "Renouveler la clé secrète",
  "rotation.action.secretKey.body":
    "Obtenez une nouvelle clé secrète et un nouveau kit d’urgence. Utilisez cette option si votre kit a été perdu ou si quelqu’un a pu le voir.",
  "rotation.action.secretKey.button": "Renouveler…",
  "rotation.action.profile": "Renforcer la protection",
  "rotation.action.profile.body":
    "Rendez la devinette de votre mot de passe principal plus difficile en utilisant plus de mémoire à chaque essai. Le déverrouillage prend un peu plus de temps.",
  "rotation.action.profile.button": "Renforcer…",
  "rotation.action.rekey": "Régénérer les clés du compte",
  "rotation.action.rekey.body":
    "Renouvelez toutes les clés de votre compte : une nouvelle clé secrète, de nouvelles clés de coffres-forts et un nouveau code de sécurité. Utilisez cette option si quelqu’un a pu copier les données de votre coffre-fort ainsi que votre mot de passe et votre clé secrète.",
  "rotation.action.rekey.button": "Régénérer…",
  "rotation.action.rekey.needsSync":
    "Régénérer les clés renouvelle la copie sur votre serveur ou dossier de synchronisation, la synchronisation est donc nécessaire. Configurez-la d’abord.",
  "rotation.title.password": "Changer le mot de passe principal",
  "rotation.title.secretKey": "Renouveler la clé secrète",
  "rotation.title.profile": "Renforcer la protection",
  "rotation.title.rekey": "Régénérer les clés du compte",
  "rotation.step": "Étape {current} sur {total}",
  "rotation.back": "Retour",
  "rotation.stop.title": "Arrêter ce changement ?",
  "rotation.stop.body":
    "Le kit d’urgence que vous venez d’enregistrer ne fonctionnera pas. Gardez votre ancien kit.",
  "rotation.stop.confirm": "Arrêter le changement",
  "rotation.stop.keep": "Continuer",
  "rotation.next": "Continuer",
  "rotation.close": "Fermer",
  "rotation.review.title.password": "Avant de changer votre mot de passe",
  "rotation.review.title.secretKey": "Avant de renouveler votre clé secrète",
  "rotation.review.title.profile": "Avant de renforcer la protection",
  "rotation.review.title.rekey": "Quelque chose a-t-il été compromis ?",
  "rotation.review.changes": "Ce qui change",
  "rotation.review.stays": "Ce qui reste identique",
  "rotation.review.password.changes":
    "Votre mot de passe principal. Le déverrouillage rapide sur cet appareil est désactivé, et vous pourrez le configurer de nouveau ensuite.",
  "rotation.review.password.stays":
    "Votre clé secrète, vos coffres-forts et éléments, votre code de sécurité et vos contacts d’urgence.",
  "rotation.review.password.devices":
    "Vos autres appareils sont déconnectés de la synchronisation. Sur chacun, saisissez une fois le nouveau mot de passe principal et la synchronisation reprend.",
  "rotation.review.secretKey.changes":
    "Votre clé secrète et votre kit d’urgence. Le déverrouillage rapide sur cet appareil est désactivé, et vous pourrez le configurer de nouveau ensuite.",
  "rotation.review.secretKey.stays":
    "Votre mot de passe principal, vos coffres-forts et éléments, votre code de sécurité et vos contacts d’urgence. Les clés de sécurité que vous avez enregistrées continuent de fonctionner.",
  "rotation.review.secretKey.devices":
    "Vos autres appareils sont déconnectés de la synchronisation. Sur chacun, saisissez une fois votre mot de passe et la nouvelle clé secrète du nouveau kit.",
  "rotation.review.secretKey.limit":
    "Cela empêche la connexion avec l’ancienne clé secrète. Toute personne qui a déjà copié les données de votre coffre-fort avec votre ancien mot de passe et votre ancienne clé secrète peut toujours les lire. Régénérez les clés du compte pour l’empêcher.",
  "rotation.review.secretKey.compromised":
    "Quelque chose a été compromis : régénérer plutôt les clés",
  "rotation.review.profile.changes":
    "La quantité de mémoire nécessaire à chaque tentative de déverrouillage, pour vous comme pour quiconque essaie de deviner. Le déverrouillage rapide sur cet appareil est désactivé, et vous pourrez le configurer de nouveau ensuite.",
  "rotation.review.profile.stays":
    "Votre mot de passe principal, votre clé secrète, vos coffres-forts et votre code de sécurité. Les clés de sécurité que vous avez enregistrées continuent de fonctionner.",
  "rotation.review.profile.devices":
    "Vos autres appareils sont déconnectés de la synchronisation. Sur chacun, saisissez une fois votre mot de passe principal et la synchronisation reprend. Les appareils plus lents mettent un peu plus de temps à déverrouiller.",
  "rotation.review.rekey.intro":
    "Régénérer les clés est destiné au cas où quelqu’un pourrait avoir les données de votre coffre-fort ainsi que votre mot de passe et votre clé secrète. Si vous avez seulement perdu un appareil, renouveler la clé secrète ou changer votre mot de passe suffit.",
  "rotation.review.rekey.yes": "Oui, régénérer les clés de mon compte",
  "rotation.review.rekey.no": "Non, renouveler seulement la clé secrète",
  "rotation.review.rekey.limit":
    "Régénérer les clés ne reprend pas ce que quelqu’un a déjà copié. Les anciennes copies sur des serveurs et dans des sauvegardes restent chiffrées avec les anciennes clés, et les anciennes clés les ouvrent toujours.",
  "rotation.review.rekey.title": "Ce que fait la régénération des clés",
  "rotation.review.rekey.vaults":
    "{count, plural, one {# coffre-fort que vous seul utilisez reçoit une nouvelle clé.} other {# coffres-forts que vous seul utilisez reçoivent de nouvelles clés.}}",
  "rotation.review.rekey.keys":
    "Une nouvelle clé secrète, un nouveau kit d’urgence, un nouveau code de sécurité et une nouvelle clé de compte.",
  "rotation.review.rekey.devices":
    "Vos autres appareils sont déconnectés et ne peuvent plus synchroniser tant que vous n’avez pas saisi sur chacun la nouvelle clé secrète et votre nouveau mot de passe principal.",
  "rotation.review.rekey.shared":
    "{count, plural, one {Vous partagez # coffre-fort.} other {Vous partagez # coffres-forts.}} Chacun reçoit une nouvelle clé.",
  "rotation.review.rekey.sharedRow":
    "{name}, {count, plural, one {# membre} other {# membres}}",
  "rotation.review.rekey.sharedMembers":
    "Demandez à chaque personne avec qui vous partagez de comparer votre nouveau code de sécurité. Tant qu’elle ne l’a pas fait, elle ne reçoit aucune nouvelle clé de votre part et ne peut pas ouvrir le coffre-fort renouvelé.",
  "rotation.review.rekey.memberVaults":
    "{count, plural, one {# coffre-fort appartient à quelqu’un d’autre.} other {# coffres-forts appartiennent à d’autres personnes.}} Leurs propriétaires sont invités à les renouveler. Tant qu’ils ne l’ont pas fait, vos anciennes clés peuvent toujours les lire.",
  "rotation.review.rekey.grants":
    "{count, plural, one {# personne a un accès d’urgence à vos coffres-forts.} other {# personnes ont un accès d’urgence à vos coffres-forts.}} Leur accès est renouvelé pour les nouvelles clés. Chacune doit aussi comparer votre nouveau code de sécurité.",
  "rotation.review.rekey.quick":
    "Le déverrouillage rapide sur cet appareil est désactivé. Vous pourrez le configurer de nouveau ensuite.",
  "rotation.review.rekey.hardwareKeys":
    "{count, plural, one {# clé de sécurité que vous avez enregistrée pour le déverrouillage est désactivée.} other {# clés de sécurité que vous avez enregistrées pour le déverrouillage sont désactivées.}} Vous pourrez les enregistrer de nouveau ensuite.",
  "rotation.review.rekey.hardwareVaults":
    "{count, plural, one {# coffre-fort protégé par matériel n’est pas concerné.} other {# coffres-forts protégés par matériel ne sont pas concernés.}}",
  "rotation.review.rekey.attachments":
    "{count, plural, one {# pièce jointe n’est pas sur cet appareil.} other {# pièces jointes ne sont pas sur cet appareil.}} Elles sont d’abord téléchargées pour pouvoir être protégées de nouveau. Celles qui restent inaccessibles gardent l’ancienne clé.",
  "rotation.review.rekey.folder":
    "Votre dossier de synchronisation reçoit un nouveau fichier de compte. Les autres appareils y trouvent une note indiquant que les clés ont changé.",
  "rotation.review.rekey.unavailable":
    "La régénération des clés n’est pas disponible : la synchronisation n’est pas configurée.",
  "rotation.review.loading": "Vérification de ce que touche la régénération des clés…",
  "rotation.review.rekey.needsClean":
    "La régénération des clés exige une synchronisation à jour et sans alerte. Synchronisez maintenant, examinez les éventuelles alertes, puis réessayez.",
  "rotation.credentials.title": "Confirmez que c’est vous",
  "rotation.credentials.password.title": "Vos mots de passe principaux",
  "rotation.credentials.current": "Mot de passe principal actuel",
  "rotation.credentials.new": "Nouveau mot de passe principal",
  "rotation.credentials.confirm": "Saisissez de nouveau le nouveau mot de passe principal",
  "rotation.credentials.mismatch": "Les deux mots de passe sont différents.",
  "rotation.credentials.same":
    "Le nouveau mot de passe principal doit être différent de l’actuel.",
  "rotation.credentials.hint":
    "Une longue phrase secrète vaut mieux qu’un mot de passe court et compliqué. Vous ne pouvez pas le récupérer : Mosaic ne le connaît pas.",
  "rotation.credentials.code": "Code à 6 chiffres de votre application d’authentification",
  "rotation.credentials.profile": "Niveau de protection",
  "rotation.credentials.profile.standard": "Standard : 256 Mo de mémoire par essai",
  "rotation.credentials.profile.strong": "Renforcé : 512 Mo de mémoire par essai",
  "rotation.credentials.profile.max": "Maximal : 1 Go de mémoire par essai",
  "rotation.credentials.profile.slow":
    "Avec ce niveau, le déverrouillage prend plus de deux secondes sur cet appareil. Les autres appareils, plus anciens, seront encore plus lents.",
  "rotation.credentials.profile.force": "Renforcer quand même",
  "rotation.kit.title": "Enregistrer votre nouveau kit d’urgence",
  "rotation.kit.body":
    "Votre nouvelle clé secrète se trouve dans ce kit. Enregistrez-le d’abord : Mosaic ne modifie votre compte qu’après que vous avez confirmé l’avoir, car sans lui vous ne pouvez pas vous connecter sur un nouvel appareil.",
  "rotation.kit.save": "Enregistrer le nouveau kit d’urgence",
  "rotation.kit.saved": "Enregistré. Conservez-le en lieu sûr, à l’écart de vos appareils.",
  "rotation.kit.cancelled": "Rien n’a encore été enregistré.",
  "rotation.kit.verify":
    "Saisissez le dernier groupe de votre nouvelle clé secrète depuis le kit enregistré.",
  "rotation.kit.confirm": "Confirmer le kit",
  "rotation.kit.mistyped":
    "Ce n’est pas la fin de la nouvelle clé secrète. Vérifiez le kit enregistré.",
  "rotation.kit.verified": "Kit confirmé.",
  "rotation.confirm.title.password": "Changer votre mot de passe principal",
  "rotation.confirm.title.secretKey": "Renouveler votre clé secrète",
  "rotation.confirm.title.profile": "Renforcer la protection",
  "rotation.confirm.title.rekey": "Régénérer les clés de votre compte",
  "rotation.confirm.body.rekey":
    "Cela déconnecte vos autres appareils et renouvelle toutes les clés. Gardez Mosaic ouvert jusqu’à la fin.",
  "rotation.confirm.body.secretKey":
    "Cela déconnecte vos autres appareils de la synchronisation. Gardez Mosaic ouvert jusqu’à la fin.",
  "rotation.confirm.verb.password": "Changer le mot de passe",
  "rotation.confirm.verb.secretKey": "Renouveler la clé secrète",
  "rotation.confirm.verb.profile": "Renforcer la protection",
  "rotation.confirm.verb.rekey": "Régénérer les clés du compte",
  "rotation.run.title": "Renouvellement de vos clés…",
  "rotation.run.keepOpen":
    "Gardez Mosaic ouvert. Avec de nombreuses pièces jointes, cela peut prendre quelques minutes.",
  "rotation.progress.label": "Progression",
  "rotation.progress.count": "{done} sur {total}",
  "rotation.progress.done": "terminé",
  "rotation.phase.prepared": "Préparation",
  "rotation.phase.backendStarted": "Demande à votre compte de passer aux nouvelles clés",
  "rotation.phase.backendAccepted": "Votre compte a accepté les nouvelles clés",
  "rotation.phase.folderWritten": "Le dossier de synchronisation contient le nouveau fichier de compte",
  "rotation.phase.localCommitted": "Cet appareil utilise maintenant les nouvelles clés",
  "rotation.phase.attachments": "Protection des pièces jointes avec les nouvelles clés",
  "rotation.phase.uploading": "Envoi des données protégées",
  "rotation.phase.done": "Terminé",
  "rotation.done.title.password": "Mot de passe principal changé",
  "rotation.done.title.secretKey": "Clé secrète renouvelée",
  "rotation.done.title.profile": "Protection renforcée",
  "rotation.done.title.rekey": "Clés du compte régénérées",
  "rotation.done.body.password":
    "Utilisez désormais le nouveau mot de passe principal. Vos autres appareils vous le demandent une fois.",
  "rotation.done.body.secretKey":
    "Conservez le nouveau kit d’urgence en lieu sûr et supprimez l’ancien. Vos autres appareils vous demandent une fois la nouvelle clé secrète.",
  "rotation.done.body.profile":
    "Le déverrouillage nécessite maintenant plus de mémoire. Vos autres appareils vous demandent une fois votre mot de passe.",
  "rotation.done.body.rekey":
    "Conservez le nouveau kit d’urgence en lieu sûr et supprimez l’ancien. Vos autres appareils vous demandent une fois la nouvelle clé secrète et votre nouveau mot de passe principal.",
  "rotation.done.members":
    "Demandez à toutes les personnes avec qui vous partagez un coffre-fort de comparer votre nouveau code de sécurité. Tant qu’elles ne l’ont pas fait, elles ne reçoivent aucune nouvelle clé de votre part.",
  "rotation.done.quick": "Configurer de nouveau le déverrouillage rapide",
  "rotation.done.quick.body": "Le déverrouillage rapide a été désactivé.",
  "rotation.done.hardware": "Enregistrer de nouveau les clés de sécurité",
  "rotation.done.hardware.body":
    "{count, plural, one {# clé de sécurité a été désactivée pour le déverrouillage.} other {# clés de sécurité ont été désactivées pour le déverrouillage.}}",
  "rotation.done.unreachable.note":
    "Ces fichiers n’ont pas pu être lus et n’ont pas été rechiffrés. Ils gardent leur ancienne clé de fichier jusqu’à ce que vous les retiriez ou les ajoutiez de nouveau, donc toute personne qui possède vos anciennes clés et une copie d’un fichier pourrait encore l’ouvrir.",
  "rotation.done.unreachable.file": "{item} : {file}",
  "rotation.done.unreachable.unknownItem": "Élément inconnu",
  "rotation.done.unreachable.unknownFile": "fichier inconnu",
  "rotation.done.unreachable.more":
    "{count, plural, one {et # autre fichier} other {et # autres fichiers}}",
  "rotation.done.unfinished":
    "La dernière partie se poursuivra plus tard. Si Mosaic se ferme avant la fin, choisissez Poursuivre le renouvellement des clés la prochaine fois.",
  "rotation.resume.title": "Changement de clés inachevé",
  "rotation.resume.body.password":
    "Un changement de clés a été interrompu. Rien n’est perdu. Choisissez Poursuivre pour saisir de nouveau vos mots de passe, ou abandonnez-le.",
  "rotation.resume.body.secretKey":
    "Le renouvellement de votre clé secrète a été interrompu. Rien n’est perdu. Poursuivez pour le terminer.",
  "rotation.resume.body.rekey":
    "La régénération des clés de votre compte a été interrompue. Rien n’est perdu. Poursuivez pour la terminer.",
  "rotation.resume.body.profile":
    "Le renforcement de la protection a été interrompu. Rien n’est perdu. Poursuivez pour le terminer.",
  "rotation.resume.continue": "Poursuivre le renouvellement des clés",
  "rotation.resume.discard": "Abandonner…",
  "rotation.resume.running": "Poursuite…",
  "rotation.resume.progress":
    "{done} sur {total} terminé. Gardez Mosaic ouvert jusqu’à la fin.",
  "rotation.resume.dialog.title": "Poursuivre le renouvellement des clés",
  "rotation.resume.dialog.password": "Mot de passe principal actuel",
  "rotation.resume.dialog.group": "Dernier groupe de la nouvelle clé secrète",
  "rotation.resume.dialog.body":
    "Les clés de votre compte ont peut-être déjà changé. Pour vérifier que c’est bien vous, saisissez votre mot de passe principal et le dernier groupe de la nouvelle clé secrète du nouveau kit d’urgence.",
  "rotation.resume.dialog.submit": "Continuer",
  "rotation.resume.discard.title": "Abandonner ce changement de clés ?",
  "rotation.resume.discard.body":
    "La nouvelle clé secrète et le nouveau kit d’urgence de cette tentative sont supprimés. Votre compte reste tel quel.",
  "rotation.resume.discard.confirm": "Abandonner le changement de clés",
  "rotation.resume.abandon": "Renoncer au reste…",
  "rotation.resume.stalled":
    "La poursuite n’a pas abouti. Si elle échoue encore, vous pouvez vous arrêter ici.",
  "rotation.abandon.title": "Arrêter de rechiffrer vos fichiers ?",
  "rotation.abandon.body":
    "Vos nouvelles clés sont déjà utilisées. Mosaic arrête de rechiffrer les pièces jointes qui attendent encore. Elles gardent leur ancienne clé de fichier, donc toute personne qui possède vos anciennes clés et une copie d’un fichier pourrait encore l’ouvrir.",
  "rotation.abandon.stays":
    "Tout ce qui est déjà rechiffré reste tel quel. Retirez ou ajoutez de nouveau plus tard les fichiers en attente pour les protéger avec la nouvelle clé.",
  "rotation.abandon.confirm": "Arrêter le rechiffrement",
  "rotation.abandon.keep": "Continuer d’essayer",
  "rotation.marker.title": "Avis de régénération des clés non confirmé",
  "rotation.marker.hint":
    "Si vous n’avez pas renouvelé vos clés sur un autre appareil, cet avis n’est peut-être pas authentique. Vous pouvez l’ignorer.",
  "rotation.marker.open": "Ignorer cet avis…",
  "rotation.marker.open.label": "Ignorer cet avis : {name}",
  "rotation.marker.confirm":
    "Mosaic a trouvé un avis de régénération des clés qu’il n’a pas pu confirmer. Si vous n’avez pas régénéré les clés de votre compte sur un autre appareil, vous pouvez l’ignorer.",
  "rotation.marker.ignore": "Ignorer l’avis",
  "rotation.marker.keep": "Conserver l’avis",
  "rotation.ignored.title": "Avis de régénération des clés ignorés",
  "rotation.ignored.body":
    "Les avis que vous avez choisi d’ignorer. Restaurez-en un si vous avez bien régénéré les clés de votre compte sur un autre appareil.",
  "rotation.ignored.row": "Avis de régénération des clés de votre dossier de synchronisation",
  "rotation.ignored.restore": "Restaurer",
  "rotation.ignored.restore.label": "Restaurer l’avis {name}",
  "rotation.resume.done": "Changement de clés terminé.",
  "rotation.recovery.title": "Les clés de votre compte ont été renouvelées sur un autre appareil",
  "rotation.recovery.body":
    "Un de vos appareils a renouvelé les clés de ce compte. Pour continuer à synchroniser, saisissez votre mot de passe principal et la clé secrète du nouveau kit d’urgence.",
  "rotation.recovery.password":
    "Mot de passe principal (le nouveau, si vous l’avez changé là-bas)",
  "rotation.recovery.secretKey": "Clé secrète du nouveau kit d’urgence",
  "rotation.recovery.submit": "Utiliser les nouvelles clés",
  "rotation.recovery.warning":
    "Vous n’avez pas renouvelé vos clés vous-même ? Alors quelqu’un qui a votre ancien mot de passe et votre ancienne clé secrète l’a peut-être fait. Ne saisissez rien ici ; vérifiez d’abord vos autres appareils.",
  "rotation.recovery.notice":
    "La synchronisation est suspendue tant que vous ne faites pas cela sur cet appareil.",
  "rotation.recovery.open": "Utiliser les nouvelles clés…",
  "rotation.recovery.keyHint":
    "Si vos clés ont été renouvelées sur un autre appareil, saisissez aussi la nouvelle clé secrète de son kit d’urgence.",
  "rotation.recovery.done": "Cet appareil utilise maintenant les nouvelles clés.",
  "rotation.recovery.optional":
    "Clé secrète du kit d’urgence le plus récent (uniquement si vous l’avez renouvelée)",
  "rotation.file.unknown":
    "Un fichier de compte inconnu est apparu dans votre dossier de synchronisation. Il ne peut pas être vérifié avec vos clés : il s’agit peut-être d’un reste, ou quelqu’un a modifié votre dossier. Vos données ne sont pas affectées.",
  "rotation.file.dismiss": "Fermer",
  "rotation.file.conflict.title": "Deux appareils ont renouvelé les clés en même temps",
  "rotation.file.conflict":
    "Un autre appareil a renouvelé les clés de votre compte au même moment que celui-ci, et les deux ne concordent donc plus. Régénérez de nouveau les clés depuis cet appareil pour les réunir : les clés de cet appareil l’emportent alors et l’autre demande la nouvelle clé secrète.",
  "rotation.file.conflict.rekey": "Régénérer de nouveau les clés depuis cet appareil",
  "rotation.file.conflict.keep": "Masquer l’avis",
  "rotation.resume.body.vault":
    "Le rechiffrement des pièces jointes d’un coffre-fort a été interrompu. Rien n’est perdu. Poursuivez pour le terminer.",
} satisfies Record<keyof typeof en, string>;
