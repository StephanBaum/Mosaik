/** German UI strings: vault. Informal "du". Same keys as English (checked by the type). */
import type { vault as en } from "../en/vault.ts";

export const vault = {
  "vault.itemCount": "{count, plural, =0 {Keine Einträge} one {# Eintrag} other {# Einträge}}",
  "vault.deleteConfirm":
    "„{title}“ löschen? Du kannst den Eintrag 30 Tage lang aus dem Papierkorb wiederherstellen.",
  "vault.defaultName": "Persönlich",
  "vault.unnamed": "Unbenannter Tresor",
  "vault.vaults": "Tresore",
  "vault.allItems": "Alle Einträge",
  "vault.items": "Einträge",
  "vault.search": "Tresor durchsuchen",
  "vault.untitled": "Ohne Titel",
  "vault.empty":
    "Dein Tresor ist leer. Lege deinen ersten Login an oder importiere aus einem anderen Passwortmanager.",
  "vault.noResults":
    "Nichts passt zu „{query}“. Prüfe die Schreibweise oder suche stattdessen nach einer Website.",
  "vault.new": "Neuer Eintrag",
  "vault.selectItem": "Wähle einen Eintrag, um Details zu sehen.",
  "vault.trashEmpty": "Der Papierkorb ist leer. Gelöschte Einträge bleiben hier 30 Tage.",
  "vault.favorite": "Favorit",
  "vault.unfavorite": "Aus Favoriten entfernen",
  "vault.moveTo": "In Tresor verschieben",
} satisfies Record<keyof typeof en, string>;
