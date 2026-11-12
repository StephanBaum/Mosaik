/** English UI strings: vault. ICU MessageFormat. */
export const vault = {
  "vault.itemCount": "{count, plural, =0 {No items} one {# item} other {# items}}",
  "vault.deleteConfirm": "Delete “{title}”? You can restore it from Trash for 30 days.",
  "vault.defaultName": "Personal",
  "vault.unnamed": "Unnamed vault",
  "vault.vaults": "Vaults",
  "vault.allItems": "All items",
  "vault.items": "Items",
  "vault.search": "Search your vault",
  "vault.untitled": "Untitled",
  "vault.noResults":
    "Nothing matches “{query}”. Check the spelling, or search for a website instead.",
  "vault.new": "New item",
  "vault.selectItem": "Select an item to see its details.",
  "vault.trashEmpty": "Trash is empty. Deleted items stay here for 30 days.",
  "vault.favorite": "Favorite",
  "vault.unfavorite": "Remove from favorites",
  "vault.moveTo": "Move to vault",
  "vault.allVaults": "All vaults",
  "vault.category.favorites": "Favorites",
  "vault.category.logins": "Logins",
  "vault.category.passkeys": "Passkeys",
  "vault.category.codes": "One-time codes",
  "vault.category.cards": "Cards",
  "vault.category.notes": "Secure notes",
  "vault.category.ssh": "SSH keys",
  "vault.flag.otp": "2FA",
  "vault.moveToNamed": "Move to {name}",
  "vault.emptyTitle": "Your vault is empty",
  "vault.emptyBody":
    "Add a login, a passkey, a card or a note. Everything is encrypted on this device before it syncs.",
  "vault.addFirst": "Add your first item",
  "vault.paletteTip.before": "Press",
  "vault.paletteTip.after": "anytime to search or run a command.",
  "trash.retention": "Deleted items are kept for 30 days.",
  "trash.rowMeta": "Deleted {deletedAt, date, medium} · {days, plural, one {# day} other {# days}} left",
  "trash.detailMeta": "{kind} · {vault} · Deleted {deletedAt, date, medium}",
  "trash.purgeConfirm.title": "Delete \"{title}\" forever?",
  "trash.purgeConfirm.body": "You can't undo this.",
  "vault.hardware.placeholderName": "Codes (security key)",
  "vault.hardware.title": "This vault needs your security key",
  "vault.hardware.body":
    "Codes here stay safe even if someone learns your master password and Secret Key, but you can't see them without your security key.",
  "vault.hardware.open": "Use security key",
  "vault.hardware.lockedNote": "Locked. Your other vaults stay unlocked.",
  "vault.hardware.lockNow": "Lock this vault",
  "vault.hardware.closesIn": "Locks in {time}",
  "vault.hardware.backupTitle": "Register a backup key",
  "vault.hardware.backupBody":
    "You have 1 key registered. We recommend two: if you lose your only key, you can't open this vault again.",
  "vault.hardware.createOption": "Protect with a security key (codes only)",
  "vault.hardware.tradeoff":
    "Codes in this vault need your security key every time, so a stolen master password alone is not enough, but losing every registered key loses these codes.",
  "vault.hardware.removeNote":
    "Removing a key doesn't change this vault's key. If a key was stolen, move the codes to a new vault.",
  "vault.hardware.moveNote":
    "Moving a code you already use into this vault doesn't protect earlier copies: the old item, its history, backups and exports. If you can, issue a new secret at the provider and add that instead.",
  "vault.hardware.pinBody":
    "Enter your key's PIN if it has one, then touch the key when it blinks.",
  "vault.hardware.keys": "Keys",
  "vault.hardware.keysLabel": "Keys for this vault",
  "vault.hardware.eyebrow": "Hardware-protected codes",
  "vault.hardware.selfLocked": "This vault locked itself. Use your security key to open it again.",
  "vault.hardware.closingSoon": "This vault locks in less than 30 seconds. Use your security key to open it again afterwards.",
  "vault.hardware.keysTitle": "Keys for this vault",
  "vault.hardware.codesOnly": "One-time codes only",
  "vault.create": "New vault",
  "vault.create.name": "Vault name",
  "vault.create.submit": "Create vault",
  "vault.menu":
    "Vault actions",
  "vaultKey.menu":
    "Renew vault key…",
  "vaultKey.title":
    "Renew the key of {vault}",
  "vaultKey.body":
    "Mosaic makes a new key for this vault and passes it to every device and person who has access. Copies they already have stay readable to them; the new key protects what changes from now on. Use this after someone lost access or a device was lost.",
  "vaultKey.attachments":
    "Also renew the keys of attachments",
  "vaultKey.attachments.hint":
    "Gives every file in this vault a new key too. This takes longer with many or large files.",
  "vaultKey.reauth":
    "Renewing a vault key needs your master password.",
  "vaultKey.submit":
    "Renew key",
  "vaultKey.working":
    "Renewing the key…",
  "vaultKey.progress":
    "Renewing attachment keys: {done} of {total}.",
  "vaultKey.done":
    "The vault key was renewed.",
  "vaultKey.doneAttachments":
    "The vault key was renewed, and the attachment keys were renewed too.",
  "vaultKey.unfinished":
    "The vault key was renewed. Renewing the attachment keys continues later. If Mosaic closes first, choose Continue key rotation next time.",
  "vaultKey.unreachable":
    "{count, plural, one {# attachment isn't on this device yet and keeps its old key.} other {# attachments aren't on this device yet and keep their old key.}}",
  "vaultKey.close":
    "Close",
  "vault.meta.title":
    "Vault details were shortened",
  "vault.meta.body":
    "Two devices changed the {fields} of “{vault}” at the same time, and together they were too long to store. Mosaic kept one version, or left the field empty. Check it and type it again if needed.",
  "vault.edit.title":
    "Edit vault",
  "vault.meta.edit":
    "Edit vault",
  "vault.meta.field.name":
    "name",
  "vault.meta.field.icon":
    "icon",
  "vault.meta.field.description":
    "description",
  "vault.meta.dismiss":
    "Dismiss",
} as const satisfies Record<string, string>;
