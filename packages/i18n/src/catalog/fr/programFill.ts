/**
 * French UI strings: programFill. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { programFill as en } from "../en/programFill.ts";

export const programFill = {
  "programFill.chip.label": "Remplir avec Mosaic",
  "programFill.chip.locked": "Déverrouillez Mosaic pour remplir",
  "programFill.list.title": "Identifiants pour {program}",
  "programFill.list.none": "Aucun identifiant pour {program} pour l’instant",
  "programFill.list.signerChanged":
    "Le signataire de ce programme a changé. Associer de nouveau ?",
  "programFill.list.fileChanged":
    "Ce programme a changé depuis son association. Associer de nouveau ?",
  "programFill.list.suggestions": "Suggestions",
  "programFill.list.search": "Rechercher des identifiants",
  "programFill.list.link": "Associer un identifiant…",
  "programFill.list.block": "Ne plus proposer pour ce programme",
  "programFill.link.packaged": "{name} · Application du Store · Éditeur {publisher}",
  "programFill.link.packagedNoPublisher": "{name} · Application du Store",
  "programFill.link.signed": "{exe} · Signé par {signer}, vérifié par Windows",
  "programFill.link.unsigned": "{exe} · Non signé · {path}",
  "programFill.link.unsignedNote":
    "Mosaic ne remplira que ce fichier précis. S’il change, Mosaic redemandera.",
  "programFill.link.lookalike":
    "Attention : ce nom ressemble à {similar}, que vous avez déjà associé.",
  "programFill.link.hashChange": "Fichier associé {old}…, ce fichier {new}…",
  "programFill.link.login": "Identifiant : {title}",
  "programFill.link.confirm": "Associer et se connecter",
  "programFill.link.confirmFill": "Associer et remplir",
  "programFill.link.confirmUnsigned": "Associer ce programme non signé",
  "programFill.link.confirmAgain": "Confirmer à nouveau pour associer",
  "programFill.notice.elevated":
    "Mosaic ne peut pas remplir les programmes exécutés en tant qu’administrateur. Copiez plutôt le mot de passe depuis Mosaic.",
  "programFill.notice.noField": "Mosaic ne voit aucun champ d’identifiant ici.",
  "programFill.notice.aborted":
    "Mosaic a arrêté le remplissage parce que la fenêtre a changé. Vérifiez le champ.",
  "programFill.notice.abortedCleared":
    "Mosaic a arrêté le remplissage parce que la fenêtre a changé, et a effacé ce qu’il avait saisi.",
  "programFill.notice.modifiersHeld":
    "Relâchez Maj, Ctrl, Alt et la touche Windows, puis choisissez de nouveau l’identifiant.",
  "programFill.notice.notResponding":
    "Ce programme n’a pas répondu à temps. Rien d’autre n’a été rempli.",
  "programFill.notice.close": "Fermer",
  "programFill.settings.title": "Remplissage dans les programmes",
  "programFill.settings.description":
    "Mosaic surveille quel champ a le focus dans les autres programmes de cet appareil afin d’y proposer vos identifiants. Rien n’est stocké ni envoyé.",
  "programFill.settings.enable": "Proposer les identifiants dans les programmes",
  "programFill.settings.submit": "Se connecter après le remplissage",
  "programFill.settings.hotkey": "Raccourci pour ouvrir la liste",
  "programFill.settings.hotkeyNone": "Aucun raccourci",
  "programFill.settings.hotkeySave": "Enregistrer le raccourci de la liste",
  "programFill.settings.blocked": "Programmes pour lesquels Mosaic ne propose pas d’identifiants",
  "programFill.settings.blockedNone": "Aucun",
  "programFill.settings.unblock": "Proposer à nouveau",
  "programFill.settings.notWindows": "Uniquement sous Windows pour l’instant.",
  "programFill.settings.mosaicElevated":
    "Mosaic s’exécute en tant qu’administrateur et ne remplit donc pas les programmes. Démarrez-le normalement.",
  "programFill.settings.hostFailed":
    "Mosaic n’a pas pu commencer à surveiller les programmes. Redémarrez Mosaic pour réessayer.",
  "programFill.card.title": "Remplir aussi les identifiants dans les programmes",
  "programFill.card.turnOn": "Activer",
  "programFill.card.notNow": "Pas maintenant",
  "programFill.item.program": "Programme",
  "programFill.item.storeApp": "Application du Store",
  "programFill.item.signedBy": "Signé par {signer}",
  "programFill.item.unsigned": "Non signé : {path}",
  "programFill.item.submit": "Se connecter après le remplissage",
  "programFill.item.unlink": "Dissocier",
} satisfies Record<keyof typeof en, string>;
