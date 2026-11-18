/**
 * Spanish UI strings: vault. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { vault as en } from "../en/vault.ts";

export const vault = {
  "vault.itemCount": "{count, plural, =0 {Sin elementos} one {# elemento} other {# elementos}}",
  "vault.deleteConfirm":
    "¿Eliminar “{title}”? Puedes restaurarlo desde la papelera durante 30 días.",
  "vault.defaultName": "Personal",
  "vault.unnamed": "Caja fuerte sin nombre",
  "vault.vaults": "Cajas fuertes",
  "vault.allItems": "Todos los elementos",
  "vault.items": "Elementos",
  "vault.search": "Buscar en tu caja fuerte",
  "vault.untitled": "Sin título",
  "vault.noResults":
    "Nada coincide con “{query}”. Revisa la ortografía o busca por sitio web.",
  "vault.new": "Nuevo elemento",
  "vault.selectItem": "Selecciona un elemento para ver sus detalles.",
  "vault.trashEmpty": "La papelera está vacía. Los elementos eliminados se quedan aquí 30 días.",
  "vault.favorite": "Favorito",
  "vault.unfavorite": "Quitar de favoritos",
  "vault.moveTo": "Mover a la caja fuerte",
  "vault.allVaults": "Todas las cajas fuertes",
  "vault.category.favorites": "Favoritos",
  "vault.category.logins": "Inicios de sesión",
  "vault.category.passkeys": "Llaves de acceso",
  "vault.category.codes": "Códigos de un solo uso",
  "vault.category.cards": "Tarjetas",
  "vault.category.notes": "Notas seguras",
  "vault.category.ssh": "Claves SSH",
  "vault.flag.otp": "2FA",
  "vault.moveToNamed": "Mover a {name}",
  "vault.emptyTitle": "Tu caja fuerte está vacía",
  "vault.emptyBody":
    "Añade un inicio de sesión, una llave de acceso, una tarjeta o una nota. Todo se cifra en este dispositivo antes de sincronizarse.",
  "vault.addFirst": "Añade tu primer elemento",
  "vault.paletteTip.before": "Pulsa",
  "vault.paletteTip.after": "en cualquier momento para buscar o ejecutar un comando.",
  "trash.retention": "Los elementos eliminados se conservan 30 días.",
  "trash.rowMeta":
    "Eliminado el {deletedAt, date, medium} · {days, plural, one {queda # día} other {quedan # días}}",
  "trash.detailMeta": "{kind} · {vault} · Eliminado el {deletedAt, date, medium}",
  "trash.purgeConfirm.title": "¿Eliminar \"{title}\" definitivamente?",
  "trash.purgeConfirm.body": "No se puede deshacer.",
  "vault.hardware.placeholderName": "Códigos (llave de seguridad)",
  "vault.hardware.title": "Esta caja fuerte necesita tu llave de seguridad",
  "vault.hardware.body":
    "Los códigos de aquí siguen seguros aunque alguien conozca tu contraseña maestra y tu Clave secreta, pero no podrás verlos sin tu llave de seguridad.",
  "vault.hardware.open": "Usar llave de seguridad",
  "vault.hardware.lockedNote": "Bloqueada. Tus otras cajas fuertes siguen desbloqueadas.",
  "vault.hardware.lockNow": "Bloquear esta caja fuerte",
  "vault.hardware.closesIn": "Se bloquea en {time}",
  "vault.hardware.backupTitle": "Registra una llave de respaldo",
  "vault.hardware.backupBody":
    "Tienes 1 llave registrada. Recomendamos dos: si pierdes tu única llave, no podrás volver a abrir esta caja fuerte.",
  "vault.hardware.createOption": "Proteger con una llave de seguridad (solo códigos)",
  "vault.hardware.tradeoff":
    "Los códigos de esta caja fuerte necesitan tu llave de seguridad cada vez, así que una contraseña maestra robada no basta, pero si pierdes todas las llaves registradas, perderás estos códigos.",
  "vault.hardware.removeNote":
    "Quitar una llave no cambia la clave de esta caja fuerte. Si robaron una llave, mueve los códigos a una caja fuerte nueva.",
  "vault.hardware.moveNote":
    "Mover a esta caja fuerte un código que ya usas no protege las copias anteriores: el elemento antiguo, su historial, las copias de seguridad y las exportaciones. Si puedes, genera un secreto nuevo en el proveedor y añade ese.",
  "vault.hardware.pinBody":
    "Escribe el PIN de tu llave si tiene uno y luego toca la llave cuando parpadee.",
  "vault.hardware.keys": "Llaves",
  "vault.hardware.keysLabel": "Llaves de esta caja fuerte",
  "vault.hardware.eyebrow": "Códigos protegidos por hardware",
  "vault.hardware.selfLocked":
    "Esta caja fuerte se bloqueó sola. Usa tu llave de seguridad para volver a abrirla.",
  "vault.hardware.closingSoon":
    "Esta caja fuerte se bloquea en menos de 30 segundos. Después, usa tu llave de seguridad para volver a abrirla.",
  "vault.hardware.keysTitle": "Llaves de esta caja fuerte",
  "vault.hardware.codesOnly": "Solo códigos de un solo uso",
  "vault.create": "Nueva caja fuerte",
  "vault.create.name": "Nombre de la caja fuerte",
  "vault.create.submit": "Crear caja fuerte",
  "vault.menu": "Acciones de la caja fuerte",
  "vaultKey.menu": "Renovar clave de la caja fuerte…",
  "vaultKey.title": "Renovar la clave de {vault}",
  "vaultKey.body":
    "Mosaic crea una clave nueva para esta caja fuerte y se la pasa a cada dispositivo y persona con acceso. Las copias que ya tienen siguen siendo legibles para ellos; la clave nueva protege lo que cambie a partir de ahora. Úsalo si alguien perdió el acceso o se perdió un dispositivo.",
  "vaultKey.attachments": "Renovar también las claves de los adjuntos",
  "vaultKey.attachments.hint":
    "Da también una clave nueva a cada archivo de esta caja fuerte. Tarda más con muchos archivos o archivos grandes.",
  "vaultKey.reauth": "Renovar la clave de una caja fuerte requiere tu contraseña maestra.",
  "vaultKey.submit": "Renovar clave",
  "vaultKey.working": "Renovando la clave…",
  "vaultKey.progress": "Renovando las claves de los adjuntos: {done} de {total}.",
  "vaultKey.done": "Se renovó la clave de la caja fuerte.",
  "vaultKey.doneAttachments":
    "Se renovó la clave de la caja fuerte y también las claves de los adjuntos.",
  "vaultKey.unfinished":
    "Se renovó la clave de la caja fuerte. La renovación de las claves de los adjuntos continuará más tarde. Si Mosaic se cierra antes, elige Continuar la rotación de claves la próxima vez.",
  "vaultKey.unreachable":
    "{count, plural, one {# adjunto aún no está en este dispositivo y conserva su clave anterior.} other {# adjuntos aún no están en este dispositivo y conservan su clave anterior.}}",
  "vaultKey.close": "Cerrar",
  "vault.meta.title": "Se acortaron los detalles de la caja fuerte",
  "vault.meta.body":
    "Dos dispositivos cambiaron a la vez {fields} de “{vault}” y, juntos, eran demasiado largos para guardarse. Mosaic conservó una versión o dejó el campo vacío. Revísalo y vuelve a escribirlo si hace falta.",
  "vault.edit.title": "Editar caja fuerte",
  "vault.meta.edit": "Editar caja fuerte",
  "vault.meta.field.name": "el nombre",
  "vault.meta.field.icon": "el icono",
  "vault.meta.field.description": "la descripción",
  "vault.meta.dismiss": "Descartar",
} satisfies Record<keyof typeof en, string>;
