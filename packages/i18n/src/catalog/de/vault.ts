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
  "vault.noResults":
    "Nichts passt zu „{query}“. Prüfe die Schreibweise oder suche stattdessen nach einer Website.",
  "vault.new": "Neuer Eintrag",
  "vault.selectItem": "Wähle einen Eintrag, um Details zu sehen.",
  "vault.trashEmpty": "Der Papierkorb ist leer. Gelöschte Einträge bleiben hier 30 Tage.",
  "vault.favorite": "Favorit",
  "vault.unfavorite": "Aus Favoriten entfernen",
  "vault.moveTo": "In Tresor verschieben",
  "vault.allVaults": "Alle Tresore",
  "vault.category.favorites": "Favoriten",
  "vault.category.logins": "Logins",
  "vault.category.passkeys": "Passkeys",
  "vault.category.codes": "Einmalcodes",
  "vault.category.cards": "Karten",
  "vault.category.notes": "Sichere Notizen",
  "vault.category.ssh": "SSH-Schlüssel",
  "vault.flag.otp": "2FA",
  "vault.moveToNamed": "Nach {name} verschieben",
  "vault.emptyTitle": "Dein Tresor ist leer",
  "vault.emptyBody":
    "Füge ein Login, einen Passkey, eine Karte oder eine Notiz hinzu. Alles wird auf diesem Gerät verschlüsselt, bevor es synchronisiert wird.",
  "vault.addFirst": "Ersten Eintrag hinzufügen",
  "vault.paletteTip.before": "Drücke",
  "vault.paletteTip.after": "jederzeit, um zu suchen oder einen Befehl auszuführen.",
} satisfies Record<keyof typeof en, string>;
