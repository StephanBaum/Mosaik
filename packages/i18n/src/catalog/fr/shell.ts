/**
 * French UI strings: shell. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { shell as en } from "../en/shell.ts";

export const shell = {
  "shell.emergencyKit.title": "Kit d’urgence Mosaic",
  "shell.emergencyKit.intro":
    "Conservez cette page en lieu sûr, par exemple dans un tiroir fermé à clé. Votre clé secrète et votre mot de passe principal, ensemble, déverrouillent votre coffre-fort sur un nouvel appareil. Personne, pas même Mosaic, ne peut récupérer l’un ou l’autre à votre place.",
  "shell.emergencyKit.email": "E-mail du compte",
  "shell.emergencyKit.accountId": "Identifiant du compte",
  "shell.emergencyKit.signInUrl": "Adresse de connexion",
  "shell.emergencyKit.signInUrlLocal":
    "Cet appareil uniquement (aucun serveur de synchronisation configuré)",
  "shell.emergencyKit.secretKey": "Clé secrète",
  "shell.emergencyKit.masterPassword": "Mot de passe principal",
  "shell.emergencyKit.masterPasswordHint":
    "Ne l’écrivez sur la ligne à la main que si cette page reste en lieu sûr. Sinon, gardez-le en mémoire.",
  "shell.emergencyKit.footer":
    "Mosaic est un logiciel avant audit : il n’a pas encore été examiné par un auditeur de sécurité externe.",
  "shell.shortcut.conflict":
    "Une autre application utilise déjà {shortcut}. Fermez cette application ou changez son raccourci, puis redémarrez Mosaic pour utiliser la recherche rapide.",
  "keys.ctrl": "Ctrl",
  "keys.shift": "Maj",
  "keys.alt": "Alt",
  "keys.space": "Espace",
  "keys.esc": "Échap",
  "keys.enter": "Entrée",
} satisfies Record<keyof typeof en, string>;
