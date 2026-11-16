/**
 * French UI strings: breach. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { breach as en } from "../en/breach.ts";

export const breach = {
  "breach.card.title": "Vérification de fuites",
  "breach.card.off":
    "Désactivée. Mosaic ne contacte jamais Have I Been Pwned tant que vous ne l’activez pas.",
  "breach.card.about":
    "Compare vos mots de passe aux fuites de données connues. Seul un fragment de 5 caractères d’un code brouillé par mot de passe quitte cet appareil.",
  "breach.mode.label": "Vérifier par rapport aux fuites de données connues",
  "breach.mode.off": "Désactivé",
  "breach.mode.manual": "Seulement quand je le demande",
  "breach.mode.weekly": "Chaque semaine, après le déverrouillage",
  "breach.card.checkNow": "Vérifier maintenant",
  "breach.card.stop": "Arrêter la vérification",
  "breach.card.turnOff": "Désactiver",
  "breach.progress.starting": "Vérification de vos mots de passe…",
  "breach.progress.step": "Vérification… {done} sur {total}",
  "breach.result.never": "Pas encore vérifié.",
  "breach.result.last":
    "Dernière vérification le {date, date, medium}. {found, plural, =0 {Aucun mot de passe n’a été trouvé dans une fuite.} one {# mot de passe a été trouvé dans une fuite.} other {# mots de passe ont été trouvés dans une fuite.}}",
  "breach.result.finished":
    "{checked, plural, one {# mot de passe vérifié} other {# mots de passe vérifiés}}. {found, plural, =0 {Aucun n’a été trouvé dans une fuite.} one {# a été trouvé dans une fuite.} other {# ont été trouvés dans une fuite.}}",
  "breach.result.upToDate":
    "Tout a été vérifié récemment. Rien de nouveau à rechercher.",
  "breach.error.offline":
    "Impossible de joindre Have I Been Pwned. Les derniers résultats sont affichés. Réessayez une fois en ligne.",
  "breach.error.rateLimited":
    "Have I Been Pwned a demandé à Mosaic de ralentir. Réessayez dans quelques minutes.",
  "breach.error.protocol":
    "Have I Been Pwned a envoyé une réponse à laquelle Mosaic ne fait pas confiance, donc certains mots de passe n’ont pas été vérifiés. Réessayez plus tard.",
  "breach.error.cancelled":
    "La vérification a été arrêtée. Les résultats déjà obtenus sont conservés.",
  "breach.consent.title":
    "Vérifier vos mots de passe par rapport aux fuites de données connues ?",
  "breach.consent.body":
    "Mosaic transforme chaque mot de passe en un code brouillé impossible à inverser (un hachage) et envoie à Have I Been Pwned (api.pwnedpasswords.com) uniquement les 5 premiers caractères de ce code. Le service connaît votre adresse IP et ces fragments de 5 caractères. Il ne voit jamais un mot de passe, votre adresse e-mail ni le site auquel un mot de passe correspond. Les résultats restent sur cet appareil.",
  "breach.consent.detail":
    "De nombreux autres mots de passe partagent chaque fragment de 5 caractères, donc la réponse ne révèle pas le vôtre. Il s’agit tout de même d’une requête vers un autre service, c’est pourquoi la fonction reste désactivée tant que vous n’êtes pas d’accord. Vous pouvez la désactiver à tout moment.",
  "breach.consent.agree": "Activer et vérifier maintenant",
  "breach.consent.decline": "Pas maintenant",
} satisfies Record<keyof typeof en, string>;
