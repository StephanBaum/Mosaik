/**
 * Spanish UI strings: sharing. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { sharing as en } from "../en/sharing.ts";

export const sharing = {
  "sharing.title": "Compartir",
  "sharing.body":
    "Comparte cajas fuertes con personas que conoces. Todo sigue cifrado: solo las personas que invitas pueden leer una caja fuerte compartida, y Mosaic solo comparte con personas cuyo código de seguridad hayas comparado.",
  "sharing.notHosted.none":
    "Compartir necesita un servidor de Mosaic. Activa primero la sincronización con un servidor de Mosaic en los ajustes de sincronización.",
  "sharing.notHosted.folder":
    "Compartir no está disponible con una carpeta de sincronización. Una carpeta no puede saber quién puede leer una caja fuerte, así que Mosaic mantiene las cajas fuertes de una carpeta privadas para ti. Para compartir, desconecta la carpeta y conecta un servidor de Mosaic en los ajustes de sincronización.",
  "sharing.notHosted":
    "Compartir necesita un servidor de Mosaic. Conecta uno primero en los ajustes de sincronización.",
  "sharing.syncSettings": "Abrir ajustes de sincronización",
  "sharing.ownCode.title": "Tu código de seguridad",
  "sharing.ownCode.body":
    "Lee este código a la persona con quien compartes, en persona o por teléfono. Si coincide con lo que ella ve de ti, nadie se interpone entre ustedes.",
  "sharing.ownCode.pending":
    "Tu código de seguridad aparece tras la primera sincronización con el servidor.",
  "sharing.contacts.title": "Personas que has buscado",
  "sharing.contacts.empty": "Aún no has buscado a nadie.",
  "sharing.contacts.verified": "Verificado",
  "sharing.contacts.notVerified": "Sin verificar",
  "sharing.contacts.changed": "Requiere una nueva comprobación",
  "sharing.contacts.verify": "Comparar código…",
  "sharing.contacts.verifyOf": "Comparar código con {email}",
  "sharing.contacts.review": "Revisar cambio…",
  "sharing.contacts.reviewOf": "Revisar el cambio de {email}",
  "sharing.share.open": "Compartir…",
  "sharing.share.title": "Compartir {vault}",
  "sharing.share.dialog": "Compartiendo: {vault}",
  "sharing.share.roleStep": "Elige un rol para {email}",
  "sharing.lookup.email": "Su correo electrónico",
  "sharing.lookup.find": "Buscar",
  "sharing.lookup.searching": "Buscando…",
  "sharing.lookup.notFoundHint":
    "Mosaic no puede decirte si una dirección tiene una cuenta. Si el código de arriba no coincide con lo que ve la persona, no compartas.",
  "sharing.code.title": "Comparar códigos de seguridad",
  "sharing.code.compare":
    "Compara este código con {email}, en persona o por teléfono. No uses correo ni chat.",
  "sharing.code.compareNew":
    "Compara el código nuevo con {email}, en persona o por teléfono. No uses correo ni chat.",
  "sharing.code.shown": "Código que Mosaic muestra para {email}",
  "sharing.code.typed": "Código que te leyó (opcional)",
  "sharing.code.typedMatch": "Es el código de arriba.",
  "sharing.code.attest":
    "Comparé este código con {email} por teléfono o en persona y es el mismo.",
  "sharing.code.matches": "Los códigos coinciden",
  "sharing.code.mismatch":
    "Los códigos son distintos. No compartas. Puede que alguien se interponga entre tú y {email}.",
  "sharing.code.mismatchHelp":
    "Revisa la dirección y pide a {email} que lea el código de nuevo. Si sigue siendo distinto, cancela y avísale por otra vía.",
  "sharing.code.verified": "{email} está verificado.",
  "sharing.code.status.new": "Contacto nuevo",
  "sharing.code.status.pinned": "Aún sin verificar",
  "sharing.code.status.verified": "Verificado",
  "sharing.code.status.changed": "Clave cambiada",
  "sharing.code.status.conflicting": "Los dispositivos no coinciden",
  "sharing.code.conflicting":
    "Tus dispositivos vieron claves distintas para {email}. Vuelve a comparar el código con la persona antes de compartir.",
  "sharing.verify.title": "Verificar a {email}",
  "sharing.role.label": "Rol",
  "sharing.role.owner": "Propietario",
  "sharing.role.manage": "Administrador",
  "sharing.role.write": "Editor",
  "sharing.role.read": "Lector",
  "sharing.role.desc.manage": "Puede invitar y quitar personas y cambiar roles.",
  "sharing.role.desc.write": "Puede añadir y cambiar elementos.",
  "sharing.role.desc.read": "Solo puede leer.",
  "sharing.role.honesty":
    "Cualquiera que pueda leer un elemento puede copiarlo. Un lector no puede cambiar la caja fuerte, pero nada le impide quedarse con lo que ve, y quitar a alguien después no recupera lo que ya descargó.",
  "sharing.invite.send": "Enviar invitación",
  "sharing.invite.sent":
    "Invitación enviada a {email}. Debe abrir Mosaic y aceptarla.",
  "sharing.invite.sentTitle": "Invitación enviada",
  "sharing.invite.pending": "Esperando a {email}",
  "sharing.invite.expires": "Caduca {when}",
  "sharing.invite.cancel": "Retirar invitación",
  "sharing.invite.cancelOf": "Retirar la invitación de {email}",
  "sharing.invite.cancelled": "Invitación retirada.",
  "sharing.inbox.title": "Invitaciones",
  "sharing.inbox.nav": "Invitaciones",
  "sharing.inbox.navCount": "Invitaciones, {count} en espera",
  "sharing.inbox.intro":
    "Personas que te invitaron a una caja fuerte compartida. Mosaic comprueba cada invitación antes de mostrártela.",
  "sharing.inbox.empty":
    "No hay invitaciones ahora mismo. Cuando alguien te invite a una caja fuerte compartida, aparecerá aquí.",
  "sharing.inbox.from": "De {email}",
  "sharing.inbox.review": "Revisar",
  "sharing.inbox.reviewOf": "Revisar la invitación de {email}",
  "sharing.inbox.refresh": "Actualizar",
  "sharing.inbox.preview.title": "¿Unirte a {vault}?",
  "sharing.inbox.preview.body": "{email} te invita como {role}.",
  "sharing.inbox.preview.unnamed": "una caja fuerte compartida",
  "sharing.inbox.preview.owner": "La caja fuerte pertenece a {email}.",
  "sharing.inbox.verifySender":
    "Antes de unirte, compara el código de seguridad de {email}.",
  "sharing.inbox.verifyOwner":
    "{sender} no es el propietario. Antes de unirte, compara también el código de seguridad del propietario: es quien puede cambiar la clave de la caja fuerte.",
  "sharing.inbox.ownerCode": "Código del propietario, {email}",
  "sharing.inbox.accept": "Unirse a la caja fuerte",
  "sharing.inbox.readyTitle": "Todo comprobado. ¿Unirte a esta caja fuerte?",
  "sharing.inbox.decline": "Rechazar",
  "sharing.inbox.block": "Rechazar y bloquear al remitente",
  "sharing.inbox.block.title": "¿Rechazar y bloquear a este remitente?",
  "sharing.inbox.block.body":
    "Mosaic rechaza la invitación e ignora cualquier invitación de {email} a partir de ahora. No puedes deshacerlo en Mosaic.",
  "sharing.inbox.unreadable":
    "No se puede abrir esta invitación. Puede estar dañada o no ser para ti. Pide a {email} que la envíe de nuevo.",
  "sharing.inbox.joined":
    "Te uniste a la caja fuerte. Aparecerá en tu lista de cajas fuertes en un momento.",
  "sharing.inbox.declined": "Invitación rechazada.",
  "sharing.inbox.outgoing": "Invitaciones que enviaste",
  "sharing.inbox.outgoing.empty": "Ninguna de tus invitaciones está esperando respuesta.",
  "sharing.inbox.unknownVault": "una caja fuerte",
  "sharing.members.title": "Personas con acceso",
  "sharing.members.you": "Tú",
  "sharing.members.unverified": "Sin verificar",
  "sharing.members.unconfirmed": "No está en la lista de miembros",
  "sharing.members.unconfirmed.help":
    "El servidor incluye a esta persona, pero la lista de miembros de la propia caja fuerte no. El próximo cambio de clave la deja fuera. Si no la conoces, quítala.",
  "sharing.members.dropped": "Ya no está en el servidor",
  "sharing.members.dropped.help":
    "Esta persona está en la lista de miembros de la caja fuerte, pero el servidor ya no la incluye. Puede que se haya ido. Comprueba que esto es lo esperado.",
  "sharing.members.invited": "Invitado",
  "sharing.members.removing": "Se está quitando",
  "sharing.members.verified": "Verificado",
  "sharing.members.verify": "Verificar…",
  "sharing.members.verifyOf": "Verificar a {email}",
  "sharing.members.remove": "Quitar",
  "sharing.members.removeOf": "Quitar a {email}",
  "sharing.members.remove.title": "¿Quitar a {email}?",
  "sharing.members.remove.confirm":
    "{email} conserva lo que ya descargó. Mosaic cambia ahora la clave de la caja fuerte y la persona no podrá leer nada nuevo.",
  "sharing.members.remove.confirmManager":
    "{email} conserva lo que ya descargó. El servidor deja de enviarle actualizaciones ahora. El dispositivo del propietario de la caja fuerte cambia la clave la próxima vez que sincronice y, hasta entonces, solo el servidor mantiene fuera a {email}.",
  "sharing.members.cancel": "Cancelar",
  "sharing.members.removed": "Quitado.",
  "sharing.members.leave": "Salir de la caja fuerte",
  "sharing.members.leave.title": "¿Salir de esta caja fuerte?",
  "sharing.members.leave.confirm":
    "La copia de este dispositivo sigue siendo legible, pero deja de actualizarse. Para volver a unirte necesitas una invitación nueva.",
  "sharing.members.left": "Saliste de la caja fuerte.",
  "sharing.members.roleOf": "Rol de {email}",
  "sharing.members.changeRole": "Cambiar rol",
  "sharing.members.changeRoleOf": "Cambiar el rol de {email}",
  "sharing.members.invite": "Invitar a alguien…",
  "sharing.members.empty": "Por ahora solo tú puedes abrir esta caja fuerte.",
  "sharing.members.unknownEmail": "Dirección desconocida",
  "sharing.members.managerNote":
    "Puedes invitar y quitar personas. Solo el dispositivo del propietario cambia la clave de la caja fuerte, así que una eliminación se completa cuando el dispositivo del propietario ha sincronizado.",
  "sharing.members.readerNote":
    "Puedes leer esta caja fuerte, pero no cambiar quién tiene acceso.",
  "sharing.rotation.running":
    "Hay una eliminación esperando a que el dispositivo del propietario cambie la clave de la caja fuerte. Hasta que sincronice, solo el servidor mantiene fuera a la persona quitada.",
  "sharing.rotation.unprotected":
    "{count, plural, one {# archivo conserva} other {# archivos conservan}} la protección anterior porque este dispositivo no {count, plural, one {lo tiene} other {los tiene}}. Una persona que quitaste aún puede abrir{count, plural, one {lo} other {los}} si conservó la clave anterior. Añade{count, plural, one {lo} other {los}} de nuevo desde un dispositivo que {count, plural, one {lo tenga} other {los tenga}}.",
  "sharing.keyChanged.title": "El código de seguridad de {email} cambió",
  "sharing.keyChanged.body":
    "El servidor muestra ahora una clave distinta para {email}. Eso ocurre cuando la persona vuelve a configurar Mosaic, y también es lo que parecería un ataque al servidor. No se comparte nada con la clave nueva hasta que compares los códigos con ella, en persona o por teléfono.",
  "sharing.keyChanged.old": "Código que comparaste antes",
  "sharing.keyChanged.new": "Código nuevo",
  "sharing.keyChanged.accept": "El código nuevo coincide",
  "sharing.finding.region": "Avisos de compartición",
  "sharing.finding.summary":
    "{count, plural, one {# aviso de compartición} other {# avisos de compartición}}",
  "sharing.finding.review": "Revisar",
  "sharing.finding.collapse": "Contraer",
  "sharing.finding.dismiss": "Descartar",
  "sharing.finding.unknownEmail": "una persona desconocida",
  "sharing.finding.verify": "Verificar…",
  "sharing.finding.remove": "Quitar",
  "sharing.finding.removeOf": "Quitar a {email} de {vault}",
  "sharing.finding.identityMismatch":
    "El servidor muestra para tu cuenta una clave de compartición distinta de la que creó este dispositivo, por lo que compartir está en pausa. No compartas nada y revisa tu servidor y tus otros dispositivos. Después elige “Ya lo comprobé…”.",
  "sharing.finding.rosterInvalid":
    "La lista de miembros de {vault} está dañada o alguien la cambió sin permiso. No añadas personas hasta que hayas comprobado quién tiene acceso. Después elige “Ya lo comprobé…”.",
  "sharing.finding.shareUntrusted":
    "Se rechazó una clave nueva de {vault}: la envió {sender}, pero no tiene permiso para cambiar la clave de esta caja fuerte. La caja fuerte se queda con su clave anterior. Pregunta al propietario si la cambió. Después elige “Ya lo comprobé…”.",
  "sharing.finding.memberUnverified":
    "No has verificado a {email} en {vault}. Mosaic no le envía claves nuevas y el próximo cambio de clave lo deja fuera. Compara su código de seguridad o quítalo.",
  "sharing.finding.memberDropped":
    "{email} está en la lista de miembros de {vault}, pero el servidor ya no lo incluye. Puede que se haya ido o que el servidor lo haya quitado. Comprueba que esto es lo esperado y quítalo si no lo es.",
  "sharing.finding.memberKeyMismatch":
    "Mosaic recibió una clave para {email} que no coincide con la que verificaste, así que se le quitó de {vault}. Vuelve a comparar los códigos de seguridad con la persona antes de invitarla de nuevo.",
  "sharing.finding.vaultHeld":
    "{vault} está en espera hasta que tus otros dispositivos confirmen quién es su propietario. Abre Mosaic en un dispositivo que ya use esta caja fuerte.",
  "sharing.finding.accessLost":
    "Ya no tienes acceso a {vault}. La copia de este dispositivo sigue siendo legible, pero no se actualiza. Pide al propietario que te invite de nuevo.",
  "sharing.finding.rotationBlocked":
    "No se pudo cambiar la clave de {vault} después de quitar a alguien, así que esa persona aún tiene la clave anterior. Vuelve a sincronizar. Si sigue así, abre la lista de miembros y comprueba quién tiene acceso.",
  "emergency.title": "Acceso de emergencia",
  "emergency.body":
    "Nombra a personas que pueden pedir acceso a tus cajas fuertes si te ocurre algo. No es una recuperación de la cuenta: obtienen acceso de solo lectura a las cajas fuertes que elijas, solo tras un tiempo de espera, y puedes rechazarlo durante ese tiempo. No sustituye a tu Kit de emergencia.",
  "emergency.folderUnsupported":
    "El acceso de emergencia necesita el servidor de Mosaic. Una carpeta de sincronización no puede imponer un tiempo de espera.",
  "emergency.granted.title": "Personas que pueden pedir acceso",
  "emergency.granted.empty": "Nadie puede pedir acceso a tus cajas fuertes.",
  "emergency.granted.add": "Añadir una persona…",
  "emergency.granted.scopeAll": "Todas tus cajas fuertes",
  "emergency.granted.scopeSome": "{count, plural, one {# caja fuerte} other {# cajas fuertes}}",
  "emergency.granted.wait": "Espera {days, plural, one {# día} other {# días}}",
  "emergency.granted.waitingUntil":
    "Pidió acceso. Se abre {when}, salvo que lo rechaces.",
  "emergency.granted.approved": "Aprobaste la solicitud.",
  "emergency.granted.released": "Se concedió el acceso. Puede leer las cajas fuertes.",
  "emergency.granted.idle": "Sin solicitud",
  "emergency.granted.remove": "Quitar acceso",
  "emergency.granted.removeOf": "Quitar el acceso de {email}",
  "emergency.granted.remove.title": "¿Quitar el acceso de emergencia de {email}?",
  "emergency.granted.remove.body":
    "Ya no podrá pedir acceso. Si su acceso ya se había concedido, Mosaic cambia las claves de las cajas fuertes que abrió; conserva lo que ya descargó.",
  "emergency.granted.removed": "Acceso de emergencia quitado.",
  "emergency.grant.title": "Permitir acceso de emergencia",
  "emergency.grant.lookup": "¿Quién debería poder pedirlo?",
  "emergency.grant.optionsTitle": "Qué puede abrir {email}",
  "emergency.grant.scope": "Qué puede abrir",
  "emergency.grant.scope.all": "Todas mis cajas fuertes, incluidas las nuevas",
  "emergency.grant.scope.selected": "Solo las cajas fuertes que elija",
  "emergency.grant.scope.pick": "Elige al menos una caja fuerte.",
  "emergency.grant.wait": "Tiempo de espera",
  "emergency.grant.wait.option": "{days, plural, one {# día} other {# días}}",
  "emergency.grant.waitHint":
    "Abre Mosaic al menos una vez dentro de este tiempo tras una solicitud, para que puedas verla y rechazarla. Cuando se acabe el tiempo, la persona obtiene acceso de solo lectura sin que se te vuelva a preguntar.",
  "emergency.grant.serverTrust":
    "El servidor controla el tiempo de espera. Si la persona que nombras y quien administra el servidor se ponen de acuerdo, podrían saltárselo. El servidor por sí solo no puede leer tus cajas fuertes.",
  "emergency.grant.vaultKeysOnly":
    "Esto abarca solo las claves de tus cajas fuertes. No da acceso a tu cuenta, a tu contraseña maestra ni a tu Clave secreta.",
  "emergency.grant.submit": "Permitir acceso de emergencia",
  "emergency.grant.done": "{email} ya puede pedir acceso de emergencia.",
  "emergency.grant.needVerify": "Compara primero el código de seguridad de {email}.",
  "emergency.received.title": "Personas que te nombraron",
  "emergency.received.empty": "Nadie te ha nombrado para acceso de emergencia.",
  "emergency.received.request": "Pedir acceso",
  "emergency.received.requestOf": "Pedir acceso a {email}",
  "emergency.received.request.title": "¿Pedir acceso a {email}?",
  "emergency.received.request.body":
    "{email} verá tu solicitud la próxima vez que abra Mosaic. Pasados {days, plural, one {# día} other {# días}} podrás abrir sus cajas fuertes, de solo lectura, salvo que la persona lo rechace.",
  "emergency.received.requested":
    "Solicitud enviada. {email} la verá la próxima vez que abra Mosaic.",
  "emergency.received.requestHint":
    "{email} ve tu solicitud en Mosaic. Pasados {days, plural, one {# día} other {# días}} podrás abrir sus cajas fuertes, de solo lectura, salvo que la persona lo rechace.",
  "emergency.received.countdown": "Podrás abrir sus cajas fuertes {when}.",
  "emergency.received.waitingLive": "Tu solicitud está en espera.",
  "emergency.received.claim": "Abrir sus cajas fuertes",
  "emergency.received.claimOf": "Abrir las cajas fuertes de {email}",
  "emergency.received.claimed":
    "{count, plural, one {Se añadió # caja fuerte} other {Se añadieron # cajas fuertes}}, de solo lectura.",
  "emergency.received.cooldown": "{email} lo rechazó. Puedes volver a pedirlo {when}.",
  "emergency.received.verifyFirst":
    "Compara el código de seguridad de {email} antes de abrir sus cajas fuertes. Mosaic solo usa una clave que hayas verificado.",
  "emergency.received.verify": "Comparar código…",
  "emergency.received.verifyOf": "Comparar código con {email}",
  "emergency.received.approved": "{email} aprobó tu solicitud.",
  "emergency.received.waitDays": "Espera {days, plural, one {# día} other {# días}}",
  "emergency.banner.region": "Solicitud de acceso de emergencia",
  "emergency.banner.body":
    "{email} pidió acceso de emergencia a tus cajas fuertes. Se abre {when}, salvo que lo rechaces.",
  "emergency.banner.refuse": "Rechazar",
  "emergency.banner.approve": "Aprobar ahora…",
  "emergency.banner.approve.title": "¿Aprobar ahora el acceso de emergencia?",
  "emergency.banner.approve.body":
    "{email} obtiene acceso de solo lectura a tus cajas fuertes de inmediato. Escribe tu contraseña maestra para confirmar.",
  "emergency.banner.approve.confirm": "Aprobar",
  "emergency.banner.refused": "Solicitud rechazada.",
  "emergency.banner.approved": "Acceso aprobado.",
} satisfies Record<keyof typeof en, string>;
