/**
 * French UI strings: otp. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { otp as en } from "../en/otp.ts";

export const otp = {
  "otp.section.label": "Code à usage unique (2FA)",
  "otp.section.empty":
    "Ajoutez le code depuis la page de configuration à deux facteurs du site.",
  "otp.section.required": "Ajoutez un code à usage unique pour enregistrer cet élément.",
  "otp.source.screen": "Scanner l’écran",
  "otp.source.clipboard": "Coller une capture d’écran",
  "otp.source.file": "Choisir une image…",
  "otp.source.manual": "Impossible de scanner ? Saisissez la clé",
  "otp.source.screenHint":
    "Mosaic s’efface un instant, regarde votre écran une seule fois et ne conserve que le code.",
  "otp.source.clipboardHint":
    "{platform, select, mac {Appuyez sur Cmd+Ctrl+Maj+4, sélectionnez le code QR, puis collez-le ici.} windows {Appuyez sur Win+Maj+S, sélectionnez le code QR, puis collez-le ici.} other {Copiez une capture d’écran du code QR, puis collez-la ici.}}",
  "otp.dialog.title": "Configurer un code à usage unique",
  "otp.working.screen": "Recherche d’un code QR sur votre écran…",
  "otp.working.image": "Lecture du code QR…",
  "otp.manual.title": "Saisir la clé de configuration",
  "otp.manual.input": "Clé de configuration ou lien otpauth://",
  "otp.manual.inputHint":
    "Le site l’affiche à côté du code QR, souvent sous « Impossible de scanner ? » ou « Saisir manuellement ». Les espaces n’ont pas d’importance.",
  "otp.manual.issuer": "Service",
  "otp.manual.account": "Compte",
  "otp.manual.advanced": "Avancé",
  "otp.manual.algorithm": "Algorithme",
  "otp.manual.digits": "Chiffres",
  "otp.manual.period": "Nouveau code toutes les",
  "otp.manual.periodValue": "{seconds} secondes",
  "otp.manual.advancedHint":
    "Ne modifiez ces valeurs que si le site le demande. La plupart des sites utilisent SHA-1, 6 chiffres et 30 secondes.",
  "otp.manual.continue": "Continuer",
  "otp.confirm.title": "Est-ce le bon code ?",
  "otp.confirm.issuer": "Service",
  "otp.confirm.account": "Compte",
  "otp.confirm.details":
    "{scheme, select, hotp {Basé sur un compteur, {digits} chiffres} steam {Steam Guard, 5 caractères} other {{digits} chiffres, nouveau code toutes les {period} secondes}}",
  "otp.confirm.algorithmValue":
    "{algorithm, select, sha256 {SHA-256} sha512 {SHA-512} other {SHA-1}}",
  "otp.confirm.codeLabel": "Code actuel",
  "otp.confirm.codeHint":
    "Si le site demande un code pour terminer la configuration, saisissez celui-ci.",
  "otp.confirm.hotpNote":
    "Les codes basés sur un compteur apparaissent après l’enregistrement, pour que le compteur reste synchronisé avec le site.",
  "otp.confirm.weak":
    "Ce secret est anormalement court. Il fonctionne, mais la protection à deux facteurs de ce site est plus faible que d’habitude.",
  "otp.confirm.use": "Utiliser ce code",
  "otp.confirm.back": "Scanner de nouveau",
  "otp.confirm.copy": "Copier le code",
  "otp.summary.noIssuer": "Code à usage unique",
  "otp.summary.pending": "Enregistré avec cet élément.",
  "otp.summary.removed": "Le code à usage unique sera retiré à l’enregistrement.",
  "otp.action.replace": "Remplacer",
  "otp.action.remove": "Retirer le code à usage unique",
  "otp.action.keep": "Le conserver",
  "otp.action.cancelSetup": "Ne pas ajouter",
  "otp.storage.label": "Où conserver le code",
  "otp.storage.item": "Avec cet élément",
  "otp.storage.itemTradeoff":
    "Pratique, mais toute personne qui ouvre ce coffre-fort obtient votre mot de passe et votre second facteur ensemble.",
  "otp.storage.hardware": "Dans {vault} (nécessite votre clé de sécurité)",
  "otp.storage.hardwareClosed": "Ouvrez d’abord {vault} avec votre clé de sécurité.",
  "otp.toast.clipboardCleared": "Capture d’écran lue. Mosaic a effacé le presse-papiers.",
  "otp.toast.savedInVault": "Code enregistré dans {vault}.",
  "otp.clipboardCopies":
    "{platform, select, windows {Si l’historique du presse-papiers ou l’Outil Capture d’écran a conservé une copie de la capture, supprimez-la à cet endroit.} mac {Si vous avez aussi enregistré la capture d’écran dans un fichier, supprimez-le.} other {Si un gestionnaire de presse-papiers a conservé une copie de la capture d’écran, supprimez-la à cet endroit.}}",
} satisfies Record<keyof typeof en, string>;
