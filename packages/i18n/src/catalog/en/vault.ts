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
  "vault.empty":
    "Your vault is empty. Add your first login, or import from another password manager.",
  "vault.noResults":
    "Nothing matches “{query}”. Check the spelling, or search for a website instead.",
  "vault.new": "New item",
  "vault.selectItem": "Select an item to see its details.",
  "vault.trashEmpty": "Trash is empty. Deleted items stay here for 30 days.",
  "vault.favorite": "Favorite",
  "vault.unfavorite": "Remove from favorites",
  "vault.moveTo": "Move to vault",
} as const satisfies Record<string, string>;
