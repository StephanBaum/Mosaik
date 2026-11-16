/**
 * French UI strings: credentialRotation. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { credentialRotation as en } from "../en/credentialRotation.ts";

export const credentialRotation = {
  "credentialRotation.tablist": "Vues de sécurité",
  "credentialRotation.tab.overview": "Vue d’ensemble",
  "credentialRotation.tab.rotate": "À changer maintenant",
  "credentialRotation.tab.dueHidden": ", {count} à changer",
  "credentialRotation.queue.title": "À changer maintenant",
  "credentialRotation.queue.intro":
    "Les mots de passe à changer en priorité. Mosaic vous guide, mais vous changez vous-même chaque mot de passe sur le site.",
  "credentialRotation.queue.empty": "Rien à changer pour le moment.",
  "credentialRotation.queue.count":
    "{count, plural, one {# mot de passe à changer} other {# mots de passe à changer}}",
  "credentialRotation.queue.list": "Mots de passe à changer",
  "credentialRotation.queue.snoozedHeading": "Reportés",
  "credentialRotation.queue.snoozedUntil": "Reporté jusqu’au {date, date, medium}",
  "credentialRotation.queue.reason.weak": "Faible",
  "credentialRotation.queue.reason.reused": "Réutilisé",
  "credentialRotation.queue.reason.old": "Ancien",
  "credentialRotation.queue.reason.breached": "Dans une fuite",
  "credentialRotation.queue.reason.unfinished": "Changement inachevé",
  "credentialRotation.queue.change": "Changer le mot de passe",
  "credentialRotation.queue.continue": "Poursuivre le changement",
  "credentialRotation.queue.editItem": "Modifier l’élément",
  "credentialRotation.queue.manualHint":
    "Mosaic ne peut pas guider celui-ci. Ouvrez l’élément et changez-le vous-même.",
  "credentialRotation.queue.actionFor": "{action} : {title}",
  "credentialRotation.queue.snooze": "Reporter",
  "credentialRotation.queue.snoozeFor": "Reporter {title}",
  "credentialRotation.queue.snoozeDays": "{days, plural, one {# jour} other {# jours}}",
  "credentialRotation.queue.snoozed":
    "Reporté de {days, plural, one {# jour} other {# jours}}.",
  "credentialRotation.queue.dismiss": "Ignorer",
  "credentialRotation.queue.dismissFor": "Ignorer {title}",
  "credentialRotation.queue.dismissed":
    "Ignoré. Il revient quand le mot de passe change.",
  "credentialRotation.queue.undo": "Annuler",
  "credentialRotation.panel.title": "Changer le mot de passe de {title}",
  "credentialRotation.panel.step": "Étape {current} sur {total}",
  "credentialRotation.panel.stays":
    "Le changement pour {title} reste ouvert. Le nouveau mot de passe est conservé dans l’élément jusqu’à ce que vous choisissiez Ça a marché ou Ça n’a pas marché.",
  "credentialRotation.panel.cancel": "Annuler",
  "credentialRotation.steps.new.title": "Choisir un nouveau mot de passe",
  "credentialRotation.steps.new.intro":
    "Mosaic enregistre le nouveau mot de passe dans cet élément avant que vous l’utilisiez où que ce soit, pour qu’il ne puisse pas se perdre. Le mot de passe actuel reste en place jusqu’à votre confirmation.",
  "credentialRotation.steps.new.label": "Nouveau mot de passe",
  "credentialRotation.steps.new.use": "Utiliser ce mot de passe",
  "credentialRotation.steps.new.saving": "Enregistrement…",
  "credentialRotation.steps.site.title": "Le changer sur le site",
  "credentialRotation.steps.site.pageLabel": "Page de changement",
  "credentialRotation.steps.site.body":
    "Ouvrez la page de mot de passe du site dans votre navigateur, cliquez sur Mosaic dans la barre d’outils et choisissez Remplir le formulaire de changement. Mosaic n’envoie jamais le formulaire.",
  "credentialRotation.steps.site.copyLink": "Copier le lien de la page de changement",
  "credentialRotation.steps.site.linkCopied": "Lien copié.",
  "credentialRotation.steps.site.linkHint":
    "Le lien n’est pas un secret. Si le site n’a pas de page à cette adresse, trouvez ses réglages de mot de passe à la main.",
  "credentialRotation.steps.site.noLink":
    "Cet identifiant n’a pas d’adresse https, Mosaic n’a donc aucune page de changement à proposer. Changez le mot de passe sur le site à la main.",
  "credentialRotation.steps.site.copyPassword": "Copier le nouveau mot de passe",
  "credentialRotation.steps.site.next": "J’ai essayé",
  "credentialRotation.steps.result.title": "Le site a-t-il accepté le nouveau mot de passe ?",
  "credentialRotation.steps.done.failed": "Mot de passe actuel conservé",
  "credentialRotation.steps.done.notSure": "Changement laissé ouvert",
  "credentialRotation.steps.result.body":
    "Mosaic ne voit pas le site. Dites-lui ce qui s’est passé pour que le bon mot de passe reste dans cet élément. Rien n’est supprimé dans les deux cas.",
  "credentialRotation.steps.result.working": "Enregistrement…",
  "credentialRotation.result.worked": "Ça a marché",
  "credentialRotation.result.failed": "Ça n’a pas marché",
  "credentialRotation.result.notSure": "Pas encore sûr",
  "credentialRotation.result.workedDone":
    "Terminé. Le nouveau mot de passe est maintenant celui de cet élément. L’ancien est dans son historique.",
  "credentialRotation.result.failedDone":
    "Le mot de passe actuel est conservé. Le nouveau reste dans l’historique comme non confirmé, au cas où le site l’aurait appliqué malgré tout.",
  "credentialRotation.result.notSureDone":
    "Le changement reste ouvert sur cet élément. Revenez quand vous saurez.",
  "credentialRotation.banner.title": "Changement de mot de passe commencé le {date, date, medium}",
  "credentialRotation.banner.prepared":
    "Le nouveau mot de passe est conservé ici jusqu’à votre confirmation. Aucune page ne l’a encore reçu.",
  "credentialRotation.banner.delivered":
    "Le nouveau mot de passe est conservé ici jusqu’à votre confirmation. L’extension de navigateur l’a saisi dans le formulaire du site.",
  "credentialRotation.banner.reveal": "Afficher le nouveau mot de passe",
  "credentialRotation.banner.hide": "Masquer le nouveau mot de passe",
  "credentialRotation.banner.copy": "Copier le nouveau mot de passe",
  "credentialRotation.banner.newPassword": "Nouveau mot de passe",
  "credentialRotation.banner.copyLink": "Copier le lien de la page de changement",
  "credentialRotation.history.title": "Historique des mots de passe",
  "credentialRotation.history.list": "Mots de passe précédents",
  "credentialRotation.history.entry": "Remplacé le {date, date, medium}",
  "credentialRotation.history.unconfirmed": "Non confirmé",
  "credentialRotation.history.unconfirmedEntry": "Essayé le {date, date, medium}",
  "credentialRotation.history.unconfirmedHint":
    "Le site a peut-être appliqué celui-ci. Restaurez-le si la connexion avec le mot de passe actuel échoue.",
  "credentialRotation.history.reveal": "Afficher le mot de passe du {date, date, medium}",
  "credentialRotation.history.hide": "Masquer le mot de passe du {date, date, medium}",
  "credentialRotation.history.restore": "Restaurer",
  "credentialRotation.history.restoreFor": "Restaurer le mot de passe du {date, date, medium}",
  "credentialRotation.history.restored":
    "Restauré. Le mot de passe précédent est dans l’historique.",
  "credentialRotation.policy.label": "Me rappeler de changer ce mot de passe",
  "credentialRotation.policy.none": "Jamais",
  "credentialRotation.policy.every": "Tous les {days} jours",
  "credentialRotation.policy.help":
    "Un rappel place l’élément dans la liste À changer maintenant. Mosaic n’envoie aucune notification et ne fait aucune requête pour cela.",
  "credentialRotation.url.help":
    "Facultatif. L’adresse https de la page de mot de passe du site, sur le même site que cet identifiant. Mosaic ne l’ouvre que lorsque vous choisissez Ouvrir la page de changement dans l’extension de navigateur.",
  "credentialRotation.url.invalid":
    "Mosaic n’utilisera pas cette adresse. Elle doit commencer par https:// et se trouver sur le même site que cet identifiant.",
  "credentialRotation.settings.title": "Rappels de mots de passe",
  "credentialRotation.settings.body":
    "Après le déverrouillage, Mosaic peut vous indiquer combien de mots de passe méritent d’être changés.",
  "credentialRotation.settings.reminders":
    "Afficher après le déverrouillage le nombre de mots de passe à changer",
  "credentialRotation.settings.remindersHint":
    "Un message dans l’application uniquement. Aucune notification, aucune requête réseau.",
  "credentialRotation.reminder.toast":
    "{count, plural, one {# mot de passe à changer} other {# mots de passe à changer}}",
  "credentialRotation.reminder.show": "Afficher",
  "credentialRotation.dialog.beginTitle": "Commencer un changement de mot de passe ?",
  "credentialRotation.dialog.beginBody":
    "{browser} veut changer le mot de passe de {title} sur {host}. Mosaic conserve le nouveau mot de passe avec cet identifiant, et garde l’actuel jusqu’à ce que vous confirmiez que le site l’a accepté.",
  "credentialRotation.dialog.site": "Site web",
  "credentialRotation.dialog.login": "Identifiant",
  "credentialRotation.dialog.password": "Nouveau mot de passe",
  "credentialRotation.dialog.hidden": "Masqué",
  "credentialRotation.dialog.show": "Afficher",
  "credentialRotation.dialog.hide": "Masquer",
  "credentialRotation.dialog.regenerate": "En générer un autre",
  "credentialRotation.dialog.regenerated": "Un nouveau mot de passe a été généré.",
  "credentialRotation.dialog.allow": "Utiliser ce mot de passe",
  "credentialRotation.dialog.cancel": "Annuler",
  "credentialRotation.dialog.decideLater": "Décider plus tard",
  "credentialRotation.dialog.untitled": "cet identifiant",
  "credentialRotation.dialog.attempt":
    "Commencé le {at, date, medium} à {at, time, short} · {stage}{n, plural, =0 {} one { · # autre changement ouvert} other { · # autres changements ouverts}}",
  "credentialRotation.dialog.stagePrepared": "aucune page n’a encore le nouveau mot de passe",
  "credentialRotation.dialog.stageDelivered":
    "le nouveau mot de passe a été saisi dans une page",
  "credentialRotation.dialog.resultTitle":
    "{host} a-t-il accepté le nouveau mot de passe ?",
  "credentialRotation.dialog.resultBody":
    "{browser} indique que le changement a fonctionné. Si vous confirmez, le nouveau mot de passe devient celui de {title}, et l’actuel passe dans son historique.",
  "credentialRotation.dialog.confirm": "Ça a marché",
  "credentialRotation.dialog.keepOld": "Garder l’ancien mot de passe",
  "credentialRotation.dialog.failedTitle": "Garder l’ancien mot de passe ?",
  "credentialRotation.dialog.failedBody":
    "{browser} indique que le changement n’a pas fonctionné. Mosaic garde le mot de passe actuel de {title}. La nouvelle valeur reste dans son historique, au cas où le site l’aurait changé malgré tout.",
  "credentialRotation.addOrigin.title": "Ajouter {host} à {title} ?",
  "credentialRotation.addOrigin.body":
    "Mosaic remplira aussi cet identifiant sur {host}. Ne l’ajoutez que si {host} fait vraiment partie du même site.",
  "credentialRotation.addOrigin.idn":
    "Cette adresse utilise des caractères spéciaux. Votre navigateur peut l’afficher sous la forme {unicode}. Vérifiez-la attentivement.",
  "credentialRotation.addOrigin.add": "Ajouter l’adresse",
  "credentialRotation.addOrigin.cancel": "Pas maintenant",
} satisfies Record<keyof typeof en, string>;
