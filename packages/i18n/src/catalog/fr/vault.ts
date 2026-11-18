/**
 * French UI strings: vault. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { vault as en } from "../en/vault.ts";

export const vault = {
  "vault.itemCount": "{count, plural, =0 {Aucun élément} one {# élément} other {# éléments}}",
  "vault.deleteConfirm":
    "Supprimer « {title} » ? Vous pourrez le restaurer depuis la corbeille pendant 30 jours.",
  "vault.defaultName": "Personnel",
  "vault.unnamed": "Coffre-fort sans nom",
  "vault.vaults": "Coffres-forts",
  "vault.allItems": "Tous les éléments",
  "vault.items": "Éléments",
  "vault.search": "Rechercher dans votre coffre-fort",
  "vault.untitled": "Sans titre",
  "vault.noResults":
    "Aucun résultat pour « {query} ». Vérifiez l’orthographe, ou recherchez plutôt un site web.",
  "vault.new": "Nouvel élément",
  "vault.selectItem": "Sélectionnez un élément pour voir ses détails.",
  "vault.trashEmpty":
    "La corbeille est vide. Les éléments supprimés y restent 30 jours.",
  "vault.favorite": "Favori",
  "vault.unfavorite": "Retirer des favoris",
  "vault.moveTo": "Déplacer vers un coffre-fort",
  "vault.allVaults": "Tous les coffres-forts",
  "vault.category.favorites": "Favoris",
  "vault.category.logins": "Identifiants",
  "vault.category.passkeys": "Clés d’accès",
  "vault.category.codes": "Codes à usage unique",
  "vault.category.cards": "Cartes",
  "vault.category.notes": "Notes sécurisées",
  "vault.category.ssh": "Clés SSH",
  "vault.flag.otp": "2FA",
  "vault.moveToNamed": "Déplacer vers {name}",
  "vault.emptyTitle": "Votre coffre-fort est vide",
  "vault.emptyBody":
    "Ajoutez un identifiant, une clé d’accès, une carte ou une note. Tout est chiffré sur cet appareil avant la synchronisation.",
  "vault.addFirst": "Ajouter votre premier élément",
  "vault.paletteTip.before": "Appuyez sur",
  "vault.paletteTip.after": "à tout moment pour rechercher ou lancer une commande.",
  "trash.retention": "Les éléments supprimés sont conservés 30 jours.",
  "trash.rowMeta":
    "Supprimé le {deletedAt, date, medium} · {days, plural, one {# jour restant} other {# jours restants}}",
  "trash.detailMeta": "{kind} · {vault} · Supprimé le {deletedAt, date, medium}",
  "trash.purgeConfirm.title": "Supprimer définitivement « {title} » ?",
  "trash.purgeConfirm.body": "Vous ne pourrez pas annuler.",
  "vault.hardware.placeholderName": "Codes (clé de sécurité)",
  "vault.hardware.title": "Ce coffre-fort nécessite votre clé de sécurité",
  "vault.hardware.body":
    "Les codes ici restent protégés même si quelqu’un découvre votre mot de passe principal et votre clé secrète, mais vous ne pouvez pas les voir sans votre clé de sécurité.",
  "vault.hardware.open": "Utiliser la clé de sécurité",
  "vault.hardware.lockedNote": "Verrouillé. Vos autres coffres-forts restent déverrouillés.",
  "vault.hardware.lockNow": "Verrouiller ce coffre-fort",
  "vault.hardware.closesIn": "Verrouillage dans {time}",
  "vault.hardware.backupTitle": "Enregistrer une clé de secours",
  "vault.hardware.backupBody":
    "Vous avez 1 clé enregistrée. Nous en recommandons deux : si vous perdez votre seule clé, vous ne pourrez plus ouvrir ce coffre-fort.",
  "vault.hardware.createOption": "Protéger avec une clé de sécurité (codes uniquement)",
  "vault.hardware.tradeoff":
    "Les codes de ce coffre-fort exigent votre clé de sécurité à chaque fois, donc un mot de passe principal volé ne suffit pas, mais perdre toutes les clés enregistrées fait perdre ces codes.",
  "vault.hardware.removeNote":
    "Retirer une clé ne change pas la clé de ce coffre-fort. Si une clé a été volée, déplacez les codes vers un nouveau coffre-fort.",
  "vault.hardware.moveNote":
    "Déplacer vers ce coffre-fort un code que vous utilisez déjà ne protège pas les copies antérieures : l’ancien élément, son historique, les sauvegardes et les exports. Si vous le pouvez, demandez un nouveau secret au fournisseur et ajoutez-le à la place.",
  "vault.hardware.pinBody":
    "Saisissez le code PIN de votre clé si elle en a un, puis touchez la clé quand elle clignote.",
  "vault.hardware.keys": "Clés",
  "vault.hardware.keysLabel": "Clés de ce coffre-fort",
  "vault.hardware.eyebrow": "Codes protégés par matériel",
  "vault.hardware.selfLocked":
    "Ce coffre-fort s’est verrouillé. Utilisez votre clé de sécurité pour le rouvrir.",
  "vault.hardware.closingSoon":
    "Ce coffre-fort se verrouille dans moins de 30 secondes. Utilisez ensuite votre clé de sécurité pour le rouvrir.",
  "vault.hardware.keysTitle": "Clés de ce coffre-fort",
  "vault.hardware.codesOnly": "Codes à usage unique uniquement",
  "vault.create": "Nouveau coffre-fort",
  "vault.create.name": "Nom du coffre-fort",
  "vault.create.submit": "Créer le coffre-fort",
  "vault.menu": "Actions du coffre-fort",
  "vaultKey.menu": "Renouveler la clé du coffre-fort…",
  "vaultKey.title": "Renouveler la clé de {vault}",
  "vaultKey.body":
    "Mosaic crée une nouvelle clé pour ce coffre-fort et la transmet à chaque appareil et à chaque personne qui y a accès. Les copies qu’ils possèdent déjà restent lisibles pour eux ; la nouvelle clé protège ce qui change à partir de maintenant. Utilisez cette option si quelqu’un a perdu l’accès ou si un appareil a été perdu.",
  "vaultKey.attachments": "Renouveler aussi les clés des pièces jointes",
  "vaultKey.attachments.hint":
    "Donne aussi une nouvelle clé à chaque fichier de ce coffre-fort. Cela prend plus de temps avec de nombreux fichiers ou des fichiers volumineux.",
  "vaultKey.reauth": "Renouveler la clé d’un coffre-fort nécessite votre mot de passe principal.",
  "vaultKey.submit": "Renouveler la clé",
  "vaultKey.working": "Renouvellement de la clé…",
  "vaultKey.progress": "Renouvellement des clés des pièces jointes : {done} sur {total}.",
  "vaultKey.done": "La clé du coffre-fort a été renouvelée.",
  "vaultKey.doneAttachments":
    "La clé du coffre-fort a été renouvelée, et les clés des pièces jointes aussi.",
  "vaultKey.unfinished":
    "La clé du coffre-fort a été renouvelée. Le renouvellement des clés des pièces jointes se poursuivra plus tard. Si Mosaic se ferme avant, choisissez Poursuivre le renouvellement des clés la prochaine fois.",
  "vaultKey.unreachable":
    "{count, plural, one {# pièce jointe n’est pas encore sur cet appareil et garde son ancienne clé.} other {# pièces jointes ne sont pas encore sur cet appareil et gardent leur ancienne clé.}}",
  "vaultKey.close": "Fermer",
  "vault.meta.title": "Les détails du coffre-fort ont été raccourcis",
  "vault.meta.body":
    "Deux appareils ont modifié en même temps {fields} de « {vault} », et ensemble ils étaient trop longs pour être stockés. Mosaic a conservé une version, ou laissé le champ vide. Vérifiez-le et saisissez-le de nouveau si nécessaire.",
  "vault.edit.title": "Modifier le coffre-fort",
  "vault.meta.edit": "Modifier le coffre-fort",
  "vault.meta.field.name": "le nom",
  "vault.meta.field.icon": "l’icône",
  "vault.meta.field.description": "la description",
  "vault.meta.dismiss": "Fermer",
} satisfies Record<keyof typeof en, string>;
