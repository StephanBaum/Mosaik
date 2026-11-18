/**
 * Spanish UI strings: ext. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { ext as en } from "../en/ext.ts";

export const ext = {
  "ext.manifest.name": "Mosaic",
  "ext.manifest.description":
    "Rellena inicios de sesión desde tu caja fuerte de Mosaic. Tu contraseña maestra se queda en la app de escritorio.",
  "ext.command.fill": "Rellenar inicio de sesión con Mosaic",
  "ext.command.generate": "Generar una contraseña con Mosaic",
  "ext.save.badgeTitle": "¿Guardar inicio de sesión?",
  "ext.loading": "Cargando…",
  "ext.site.secure": "Sitio actual, conexión segura (https): {host}",
  "ext.site.insecure": "Sitio actual, sin cifrar (http): {host}",
  "ext.site.none": "Esta pestaña no tiene sitio web",
  "ext.nav.label": "Vistas",
  "ext.nav.logins": "Inicios de sesión y búsqueda",
  "ext.nav.generator": "Generador de contraseñas",
  "ext.footer.open": "Abrir Mosaic",
  "ext.footer.more": "Más opciones",
  "ext.footer.connected": "Conectado a Mosaic",
  "ext.footer.locked": "Mosaic está bloqueado",
  "ext.footer.offline": "Sin conexión con Mosaic",
  "ext.more.saveOffers": "Ofrecer guardar inicios de sesión",
  "ext.more.saveOffersHint":
    "Necesita acceso a todos los sitios web para que la extensión detecte un inicio de sesión que envíes. No se guarda nada hasta que lo confirmes en la app de Mosaic.",
  "ext.passkeys.toggle": "Usar Mosaic para llaves de acceso",
  "ext.passkeys.hint":
    "Necesita acceso a todos los sitios web para responder a las solicitudes de llaves de acceso. Confirmas cada una en la app de Mosaic.",
  "ext.passkeys.permissionDenied":
    "Mosaic necesita acceso a los sitios web para responder a las solicitudes de llaves de acceso y no lo permitiste.",
  "ext.more.disconnect": "Desconectar de Mosaic",
  "ext.more.disconnectTitle": "¿Desconectar este navegador?",
  "ext.more.disconnectBody":
    "Este navegador no podrá rellenar inicios de sesión hasta que lo vuelvas a conectar. Apruebas la nueva conexión en la app de Mosaic.",
  "ext.action.cancel": "Cancelar",
  "ext.neverSave.menu": "Sitios para los que nunca se ofrece guardar…",
  "ext.neverSave.title": "Nunca se ofrece guardar",
  "ext.neverSave.intro":
    "Mosaic no ofrece guardar inicios de sesión en estos sitios. Quita un sitio para que se vuelva a preguntar allí.",
  "ext.neverSave.empty":
    "Aún no hay sitios. Puedes añadir uno con “Nunca para este sitio” cuando Mosaic ofrezca guardar un inicio de sesión.",
  "ext.neverSave.remove": "Quitar",
  "ext.neverSave.removeLabel": "Quitar {origin} de la lista",
  "ext.neverSave.removed": "Se quitó {origin}. Mosaic puede volver a ofrecer guardar inicios de sesión allí.",
  "ext.neverSave.close": "Cerrar",
  "ext.connect.checking": "Conectando con Mosaic…",
  "ext.connect.hostMissing.title": "Mosaic no está configurado para este navegador",
  "ext.connect.hostMissing.body":
    "Instala la app de Mosaic en este equipo y reinicia el navegador.",
  "ext.connect.hostMissing.dev": "Desarrolladores: ejecuten {command} y reinicien el navegador.",
  "ext.connect.hostForbidden.title": "Este navegador no tiene permiso para conectarse",
  "ext.connect.hostForbidden.body":
    "Mosaic solo se comunica con la extensión oficial de Mosaic. Instálala desde la tienda de tu navegador.",
  "ext.connect.desktopNotRunning.title": "Abre Mosaic en este equipo",
  "ext.connect.desktopNotRunning.body":
    "La extensión funciona a través de la app de Mosaic. Inicia la app y vuelve a comprobar.",
  "ext.connect.check": "Comprobar de nuevo",
  "ext.connect.notPaired.title": "Conectar con Mosaic",
  "ext.connect.notPaired.body":
    "Conecta este navegador con la app de Mosaic de este equipo. Lo apruebas en la app.",
  "ext.connect.start": "Conectar",
  "ext.connect.pairing.title": "Confirma en Mosaic",
  "ext.connect.pairing.body":
    "Comprueba que Mosaic muestra el mismo código y haz clic en Permitir allí.",
  "ext.connect.pairing.code": "Código",
  "ext.connect.pairing.waiting": "Esperando tu respuesta en la app de Mosaic…",
  "ext.connect.protocolMismatch.title": "Actualiza Mosaic",
  "ext.connect.protocolMismatch.body":
    "La extensión y la app de Mosaic tienen versiones distintas. Actualiza ambas a la última versión.",
  "ext.connect.error.title": "No se puede conectar con Mosaic",
  "ext.connect.error.body": "Comprueba que la app de Mosaic se esté ejecutando y vuelve a intentarlo.",
  "ext.locked.title": "Mosaic está bloqueado",
  "ext.locked.body": "Desbloquéalo en la app de Mosaic para rellenar en {host}.",
  "ext.locked.bodyNoSite": "Desbloquéalo en la app de Mosaic para usarlo aquí.",
  "ext.locked.open": "Abrir Mosaic",
  "ext.locked.note": "Tu contraseña maestra solo se escribe en la app de Mosaic.",
  "ext.logins.forSite": "Para {host}",
  "ext.logins.count": "{count, plural, one {# inicio de sesión} other {# inicios de sesión}}",
  "ext.logins.sameSite": "mismo sitio",
  "ext.logins.none": "No hay ningún inicio de sesión guardado para {host}.",
  "ext.logins.noneHint":
    "Mosaic compara la dirección completa, así que los sitios que se le parecen nunca coinciden.",
  "ext.logins.unsupported":
    "Mosaic no puede rellenar en esta página. Busca en tu caja fuerte para copiar un inicio de sesión.",
  "ext.logins.shortcut": "{shortcut} rellena la primera coincidencia.",
  "ext.logins.frameNote":
    "Mosaic rellena solo en {host}, nunca dentro de marcos incrustados de otros sitios.",
  "ext.item.untitled": "Sin título",
  "ext.item.noUsername": "Sin nombre de usuario",
  "ext.item.twoFactor": "2FA",
  "ext.item.fill": "Rellenar",
  "ext.item.fillLabel": "Rellenar {name}",
  "ext.item.copyMenu": "Opciones de copia para {name}",
  "ext.item.copyUsername": "Copiar nombre de usuario",
  "ext.item.copyPassword": "Copiar contraseña",
  "ext.item.copyCode": "Copiar código",
  "ext.item.blocked":
    "Bloqueado: esta página no está cifrada y este inicio de sesión no lo permite.",
  "ext.search.label": "Otros elementos",
  "ext.search.placeholder": "Buscar en tu caja fuerte",
  "ext.search.note":
    "Los resultados se quedan en esta ventana. La página solo recibe el inicio de sesión que rellenas.",
  "ext.search.none": "No se encontró nada.",
  "ext.search.copy": "Copiar",
  "ext.search.copyOnly": "Solo copiar: no está guardado para este sitio",
  "ext.shortcut.copyUser": "{platform, select, mac {⌘C} other {Ctrl+C}}",
  "ext.shortcut.copyPassword": "{platform, select, mac {⇧⌘C} other {Ctrl+Mayús+C}}",
  "ext.shortcut.copyCode": "{platform, select, mac {⌥⌘C} other {Ctrl+Alt+C}}",
  "ext.search.results": "{count, plural, one {# resultado} other {# resultados}}",
  "ext.frame.cross":
    "El campo con el foco está en un marco de otro sitio. Mosaic no rellena allí.",
  "ext.frameConfirm.title": "¿Rellenar dentro de un marco incrustado?",
  "ext.frameConfirm.body":
    "El campo está en un marco de {host}, no en la propia página. Rellena aquí solo si confías en este marco.",
  "ext.frameConfirm.confirm": "Rellenar en el marco",
  "ext.warning.idn.title": "Comprueba la dirección",
  "ext.warning.idn.body":
    "Esta dirección usa caracteres especiales y puede parecerse a otro sitio. Revisa la barra de direcciones antes de iniciar sesión.",
  "ext.warning.lookalike.body":
    "Este sitio no es {saved}. Se le parece, un truco habitual de phishing. Cierra esta pestaña o escribe {saved} tú mismo.",
  "ext.warning.youAreOn": "Estás en",
  "ext.warning.writtenAs": "Escrito como",
  "ext.warning.savedFor": "Guardado para",
  "ext.warning.insecure.title": "Esta página no está cifrada",
  "ext.warning.insecure.body":
    "La página usa http. Otras personas en la red podrían leer tu contraseña. Mosaic rellena aquí solo los inicios de sesión para los que lo permitiste.",
  "ext.otp.title": "Rellena tu código de un solo uso",
  "ext.otp.next": "Contraseña rellenada para {name}. Siguiente: el código de verificación en dos pasos.",
  "ext.otp.for": "Código para {name}",
  "ext.otp.fill": "Rellenar código",
  "ext.otp.copy": "Copiar código",
  "ext.otp.note":
    "El código va de Mosaic directamente al campo. Esta ventana nunca lo muestra.",
  "ext.save.titleNew": "¿Guardar este inicio de sesión para {host}?",
  "ext.save.titleUpdate": "¿Actualizar el inicio de sesión guardado para {host}?",
  "ext.save.generated": "Una contraseña que Mosaic generó para {host}.",
  "ext.save.updates": "Actualiza “{title}”.",
  "ext.save.username": "Nombre de usuario",
  "ext.save.noUsername": "Ninguno",
  "ext.save.password": "Contraseña",
  "ext.save.passwordHidden": "Oculta. La ves en la app de Mosaic.",
  "ext.save.insecure": "Esta página no está cifrada (http).",
  "ext.save.action": "Guardar en Mosaic…",
  "ext.save.notNow": "Ahora no",
  "ext.save.never": "Nunca para este sitio",
  "ext.save.waiting": "Confirma en la app de Mosaic. No se guarda nada hasta que lo hagas.",
  "ext.save.saved": "Guardado en Mosaic.",
  "ext.save.updated": "Actualizado en Mosaic.",
  "ext.save.cancelled": "No se guardó.",
  "ext.save.dismissed": "Descartado. La contraseña se olvida.",
  "ext.save.neverDone": "Mosaic no volverá a ofrecer guardar en {host}.",
  "ext.save.permissionAsk":
    "Mosaic pide a tu navegador acceso a los sitios web. Permítelo para activar esto.",
  "ext.generator.title": "Generar una contraseña para {host}",
  "ext.generator.titleGeneric": "Generar una contraseña",
  "ext.generator.value": "Contraseña generada",
  "ext.generator.regenerate": "Generar otra contraseña",
  "ext.generator.type": "Tipo de contraseña",
  "ext.generator.random": "Aleatoria",
  "ext.generator.passphrase": "Frase de contraseña",
  "ext.generator.include": "Incluir",
  "ext.generator.rules.page": "Sigue las reglas de contraseña de esta página",
  "ext.generator.rules.siteQuirks": "Sigue las reglas conocidas de este sitio",
  "ext.generator.fill": "Rellenar",
  "ext.generator.fillHint":
    "Para rellenar, haz clic en un campo de contraseña nueva de la página y vuelve a abrir esta ventana.",
  "ext.generator.copy": "Copiar",
  "ext.generator.expired":
    "Esa contraseña ya no estaba guardada en memoria, así que se generó una nueva.",
  "ext.generator.filled": "Rellenado. Mosaic ofrecerá guardarla.",
  "ext.copied": "Copiado. Se borra en {seconds, plural, one {# segundo} other {# segundos}}.",
  "ext.copiedNoClear": "Copiado.",
  "ext.filled": "Rellenado.",
  "ext.filledCode": "Código rellenado.",
  "ext.error.invalidRequest":
    "Mosaic no pudo entender esa solicitud. Actualiza la extensión y la app de Mosaic.",
  "ext.error.notPaired": "Este navegador no está conectado a Mosaic. Vuelve a conectarlo.",
  "ext.error.protocolVersion":
    "La extensión y la app de Mosaic tienen versiones distintas. Actualiza ambas.",
  "ext.error.busy": "Mosaic está ocupado con otra solicitud. Vuelve a intentarlo en un momento.",
  "ext.error.locked": "Mosaic está bloqueado. Desbloquéalo en la app de Mosaic.",
  "ext.error.rateLimited":
    "Fueron demasiadas solicitudes en poco tiempo. Espera un momento y vuelve a intentarlo.",
  "ext.error.originMismatch":
    "Este inicio de sesión no está guardado para este sitio web, así que Mosaic no lo rellenará aquí.",
  "ext.error.insecureOrigin":
    "Esta página no está cifrada y el inicio de sesión no permite rellenar en ella. No se rellenó nada.",
  "ext.error.crossOriginFrame":
    "Ese campo está en un marco de otro sitio. Mosaic no rellena allí.",
  "ext.error.itemNotFound":
    "Ese elemento ya no existe. Puede que se haya movido o eliminado.",
  "ext.error.vaultUnavailable": "Esa caja fuerte está cerrada. Ábrela primero en la app de Mosaic.",
  "ext.error.noOtp": "Este elemento no tiene ningún código de un solo uso.",
  "ext.error.otpNotFillable":
    "Este código aumenta con cada uso (HOTP), así que no se puede rellenar ni copiar desde aquí. Úsalo en la app de Mosaic.",
  "ext.error.fieldEmpty": "Ese campo está vacío, así que no hay nada que rellenar ni copiar.",
  "ext.error.clipboardUnavailable":
    "Mosaic no pudo usar el portapapeles. Vuelve a intentarlo o cópialo en la app de Mosaic.",
  "ext.error.cancelled": "Se canceló en la app de Mosaic. Vuelve a intentarlo cuando estés listo.",
  "ext.error.passkeyNotFound": "Mosaic no tiene ninguna llave de acceso para este sitio.",
  "ext.error.passkeyExcluded": "Ya tienes una llave de acceso para esta cuenta en Mosaic.",
  "ext.error.rpIdMismatch":
    "Este sitio no puede usar una llave de acceso para ese dominio, así que Mosaic no respondió.",
  "ext.error.passkeyUnsupported":
    "Mosaic no puede crear una llave de acceso con las opciones que pidió este sitio.",
  "ext.error.pairingDenied":
    "La conexión se denegó en la app de Mosaic. Elige Conectar para volver a intentarlo y luego elige Permitir.",
  "ext.error.pairingExpired":
    "La solicitud de conexión agotó su tiempo. Elige Conectar para empezar de nuevo.",
  "ext.error.pairingConflict":
    "Había otra solicitud de conexión abierta. Elige Conectar para volver a intentarlo.",
  "ext.error.pairingBlocked":
    "Se denegaron demasiadas conexiones. Espera diez minutos y vuelve a intentarlo.",
  "ext.error.pairingLimit":
    "Este equipo tiene demasiados navegadores conectados. Desconecta uno en los ajustes de Mosaic.",
  "ext.error.internal":
    "Mosaic encontró un problema inesperado. Vuelve a intentarlo y, si se repite, reinicia la app de Mosaic.",
  "ext.error.desktopNotRunning":
    "Mosaic no se está ejecutando en este equipo. Ábrelo y vuelve a intentarlo.",
  "ext.error.desktopNotAuthentic":
    "La app que respondió no es la app de Mosaic a la que está conectado este navegador. No se envió nada.",
  "ext.error.timeout":
    "Mosaic no respondió a tiempo. Si hay un cuadro de diálogo abierto en la app de Mosaic, respóndelo allí.",
  "ext.error.frameTooLarge": "Eran demasiados datos para enviar a Mosaic.",
  "ext.error.hostMissing":
    "El ayudante de Mosaic no está instalado para este navegador. Instala la app de Mosaic y reinicia el navegador.",
  "ext.error.hostForbidden":
    "Este navegador o esta extensión no tiene permiso para comunicarse con Mosaic.",
  "ext.error.hostExited": "Se cerró la conexión con Mosaic. Vuelve a intentarlo.",
  "ext.error.unsupportedPage": "Mosaic no puede rellenar en este tipo de página.",
  "ext.error.noFields":
    "No se encontraron campos de inicio de sesión aquí. Haz clic en un campo y vuelve a intentarlo.",
  "ext.error.noMatch": "Mosaic no tiene ningún inicio de sesión guardado para este sitio.",
  "ext.error.frameNeedsConfirmation":
    "El campo está dentro de un marco. Abre la ventana de Mosaic y confirma el marco para rellenarlo.",
  "ext.error.frameOpaque": "El campo está en un marco aislado. Mosaic no rellena allí.",
  "ext.error.originChanged":
    "La página cambió mientras Mosaic rellenaba. No se rellenó nada. Vuelve a intentarlo.",
  "ext.error.pageHidden": "La página no está visible, así que Mosaic no rellenó.",
  "ext.error.permissionDenied":
    "Mosaic necesita acceso a los sitios web para detectar inicios de sesión y no lo permitiste.",
  "ext.error.generatedExpired":
    "Mosaic ya no tiene esa contraseña. Genera una nueva.",
  "ext.rotation.card.title": "Cambio de contraseña",
  "ext.rotation.card.for": "Para {name}",
  "ext.rotation.card.started": "Iniciado {when}",
  "ext.rotation.card.delivered": "La contraseña nueva ya se rellenó una vez.",
  "ext.rotation.card.note":
    "La contraseña nueva solo se muestra en la app de Mosaic. Mosaic nunca envía el formulario por ti.",
  "ext.rotation.card.list": "Cambios de contraseña en curso",
  "ext.rotation.change": "Cambiar contraseña…",
  "ext.rotation.changeLabel": "Cambiar contraseña… de {name}",
  "ext.rotation.openPage": "Abrir página de cambio",
  "ext.rotation.fillChange": "Rellenar formulario de cambio",
  "ext.rotation.fillPending": "Rellenar la contraseña nueva para iniciar sesión",
  "ext.rotation.worked": "Funcionó",
  "ext.rotation.failed": "No funcionó",
  "ext.rotation.fresh": "Empezar de nuevo con una contraseña nueva",
  "ext.rotation.waiting": "Esperándote en la app de Mosaic…",
  "ext.rotation.started": "Iniciado. Elige Usar esta contraseña en la app de Mosaic.",
  "ext.rotation.startCancelled": "No se inició nada.",
  "ext.rotation.filled": "Formulario de cambio rellenado. Revísalo y envíalo tú mismo.",
  "ext.rotation.opened": "Se abrió la página de cambio.",
  "ext.rotation.confirmed": "Listo. La contraseña nueva ya está guardada en Mosaic.",
  "ext.rotation.keptOld": "Mosaic conservó la contraseña anterior.",
  "ext.rotation.resultCancelled": "No cambió nada. El cambio sigue abierto.",
  "ext.rotation.offer.title": "Una dirección relacionada",
  "ext.rotation.offer.body":
    "Esta página está en {host}, que no está guardado para {title}. ¿Añadirlo para que Mosaic pueda rellenar allí?",
  "ext.rotation.offer.add": "Añadir {host}",
  "ext.rotation.offer.added": "Añadido. El cambio se muestra ahora aquí.",
  "ext.rotation.offer.cancelled": "No se añadió.",
  "ext.error.rotationNotFound":
    "Ese cambio de contraseña ya no está abierto. Inicia uno nuevo desde el inicio de sesión.",
  "ext.error.originNotRelated":
    "Esa dirección no está en el mismo sitio que este inicio de sesión, así que Mosaic no la añadió.",
  "ext.error.rotationLimit":
    "Este inicio de sesión ya tiene cuatro cambios sin terminar. Termina o cancela uno primero en la app de Mosaic.",
  "ext.error.changeFormNotFound":
    "Mosaic no encontró aquí un formulario de cambio de contraseña. Abre la página de contraseña de tu cuenta y vuelve a intentarlo.",
  "ext.error.changeFormAmbiguous":
    "Mosaic no pudo distinguir qué campo es cuál. Copia la contraseña nueva desde Mosaic y pégala tú mismo.",
} satisfies Record<keyof typeof en, string>;
