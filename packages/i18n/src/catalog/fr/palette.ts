/**
 * French UI strings: palette. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { palette as en } from "../en/palette.ts";

export const palette = {
  "palette.title": "Palette de commandes",
  "palette.search": "Rechercher des éléments et des commandes",
  "palette.items": "Éléments",
  "palette.commands": "Commandes",
  "palette.empty": "Aucun résultat. Essayez un autre mot.",
  "palette.cmd.newItem": "Nouvel élément",
  "palette.cmd.generator": "Ouvrir le générateur",
  "palette.cmd.security": "Ouvrir le rapport de sécurité",
  "palette.cmd.settings": "Ouvrir les réglages",
  "palette.cmd.import": "Importer depuis une autre application",
  "palette.cmd.share": "Partager le coffre-fort…",
  "palette.cmd.invites": "Invitations",
  "palette.cmd.lock": "Verrouiller Mosaic",
  "palette.open": "Ouvrir la palette de commandes",
  "palette.hint.move": "pour naviguer",
  "palette.hint.open": "pour ouvrir",
  "palette.hint.close": "pour fermer",
} satisfies Record<keyof typeof en, string>;
