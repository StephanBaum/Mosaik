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
  "trash.retention": "Gelöschte Einträge bleiben 30 Tage erhalten.",
  "trash.rowMeta": "Gelöscht am {deletedAt, date, medium} · noch {days, plural, one {# Tag} other {# Tage}}",
  "trash.detailMeta": "{kind} · {vault} · Gelöscht am {deletedAt, date, medium}",
  "trash.purgeConfirm.title": "\"{title}\" endgültig löschen?",
  "trash.purgeConfirm.body": "Das lässt sich nicht rückgängig machen.",
  "vault.hardware.placeholderName": "Codes (Sicherheitsschlüssel)",
  "vault.hardware.title": "Dieser Tresor braucht deinen Sicherheitsschlüssel",
  "vault.hardware.body":
    "Die Codes hier bleiben sicher, auch wenn jemand dein Master-Passwort und deinen Secret Key kennt. Ohne deinen Sicherheitsschlüssel kannst du sie aber nicht sehen.",
  "vault.hardware.open": "Sicherheitsschlüssel verwenden",
  "vault.hardware.lockedNote": "Gesperrt. Deine anderen Tresore bleiben entsperrt.",
  "vault.hardware.lockNow": "Diesen Tresor sperren",
  "vault.hardware.closesIn": "Sperrt in {time}",
  "vault.hardware.backupTitle": "Ersatzschlüssel registrieren",
  "vault.hardware.backupBody":
    "Du hast 1 Schlüssel registriert. Wir empfehlen zwei: Verlierst du deinen einzigen Schlüssel, kannst du diesen Tresor nie wieder öffnen.",
  "vault.hardware.createOption": "Mit Sicherheitsschlüssel schützen (nur Codes)",
  "vault.hardware.tradeoff":
    "Codes in diesem Tresor brauchen jedes Mal deinen Sicherheitsschlüssel. Ein gestohlenes Master-Passwort allein reicht dann nicht, aber wenn du alle registrierten Schlüssel verlierst, sind diese Codes verloren.",
  "vault.hardware.removeNote":
    "Das Entfernen eines Schlüssels ändert den Schlüssel dieses Tresors nicht. Wurde ein Schlüssel gestohlen, verschiebe die Codes in einen neuen Tresor.",
  "vault.hardware.moveNote":
    "Wenn du einen Code, den du schon nutzt, in diesen Tresor verschiebst, schützt das frühere Kopien nicht: den alten Eintrag, seinen Verlauf, Backups und Exporte. Stelle wenn möglich beim Anbieter ein neues Secret aus und füge dieses hinzu.",
  "vault.hardware.pinBody":
    "Gib die PIN deines Schlüssels ein, falls er eine hat, und berühre ihn dann, wenn er blinkt.",
  "vault.hardware.keys": "Schlüssel",
  "vault.hardware.keysLabel": "Schlüssel dieses Tresors",
  "vault.hardware.eyebrow": "Hardwaregeschützte Codes",
  "vault.hardware.selfLocked": "Dieser Tresor hat sich selbst gesperrt. Öffne ihn mit deinem Sicherheitsschlüssel wieder.",
  "vault.hardware.closingSoon":
    "Dieser Tresor sperrt sich in weniger als 30 Sekunden. Öffne ihn danach mit deinem Sicherheitsschlüssel wieder.",
  "vault.hardware.keysTitle": "Schlüssel für diesen Tresor",
  "vault.hardware.codesOnly": "Nur Einmalcodes",
  "vault.create": "Neuer Tresor",
  "vault.create.name": "Name des Tresors",
  "vault.create.submit": "Tresor erstellen",
  "vault.menu":
    "Tresor-Aktionen",
  "vaultKey.menu":
    "Tresorschlüssel erneuern…",
  "vaultKey.title":
    "Schlüssel von {vault} erneuern",
  "vaultKey.body":
    "Mosaic macht einen neuen Schlüssel für diesen Tresor und gibt ihn an jedes Gerät und jede Person mit Zugriff weiter. Nutze das, wenn jemand den Zugriff verloren hat oder ein Gerät verloren ging.",
  "vaultKey.attachments":
    "Anhänge auch neu verschlüsseln",
  "vaultKey.attachments.hint":
    "Gibt auch jeder Datei in diesem Tresor einen neuen Schlüssel. Bei vielen oder großen Dateien dauert das länger.",
  "vaultKey.reauth":
    "Zum Erneuern eines Tresorschlüssels brauchst du dein Master-Passwort.",
  "vaultKey.submit":
    "Schlüssel erneuern",
  "vaultKey.working":
    "Schlüssel wird erneuert…",
  "vaultKey.progress":
    "Anhänge werden neu verschlüsselt: {done} von {total}.",
  "vaultKey.done":
    "Der Tresorschlüssel wurde erneuert.",
  "vaultKey.doneAttachments":
    "Der Tresorschlüssel wurde erneuert, und die Anhänge sind neu verschlüsselt.",
  "vaultKey.unfinished":
    "Der Tresorschlüssel wurde erneuert. Das Neuverschlüsseln der Anhänge läuft später weiter. Wenn Mosaic vorher schließt, wähle beim nächsten Mal „Schlüsseländerung fortsetzen“.",
  "vaultKey.unreachable":
    "{count, plural, one {# Anhang ist noch nicht auf diesem Gerät und behält seinen alten Schlüssel.} other {# Anhänge sind noch nicht auf diesem Gerät und behalten ihren alten Schlüssel.}}",
  "vaultKey.close":
    "Schließen",
  "vault.meta.title":
    "Tresor-Angaben wurden gekürzt",
  "vault.meta.body":
    "Zwei Geräte haben {fields} von „{vault}“ gleichzeitig geändert, und das Ergebnis war zu lang. Mosaic hat den längeren Wert behalten. Prüfe ihn und ändere ihn bei Bedarf.",
  "vault.meta.field.name":
    "den Namen",
  "vault.meta.field.icon":
    "das Symbol",
  "vault.meta.field.description":
    "die Beschreibung",
  "vault.meta.dismiss":
    "Ausblenden",
} satisfies Record<keyof typeof en, string>;
