/**
 * Spanish UI strings: mobile. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { mobile as en } from "../en/mobile.ts";

export const mobile = {
  "mobile.app.name": "Mosaic",
  "mobile.nav.vault": "Caja fuerte",
  "mobile.nav.search": "Buscar",
  "mobile.nav.settings": "Ajustes",
  "mobile.common.cancel": "Cancelar",
  "mobile.common.done": "Listo",
  "mobile.common.close": "Cerrar",
  "mobile.common.continue": "Continuar",
  "mobile.unlock.title": "Desbloquear Mosaic",
  "mobile.unlock.passwordLabel": "Contraseña maestra",
  "mobile.unlock.action": "Desbloquear",
  "mobile.unlock.quick": "Usar desbloqueo rápido",
  "mobile.unlock.failed":
    "{attempts, plural, one {No funcionó # vez. Mosaic no puede restablecer tu contraseña maestra. Si la cambiaste en otro dispositivo, usa la nueva.} other {No funcionó # veces. Mosaic no puede restablecer tu contraseña maestra. Si la cambiaste en otro dispositivo, usa la nueva.}}",
  "mobile.lock.now": "Bloquear ahora",
  "mobile.vault.title": "Tu caja fuerte",
  "mobile.vault.count": "{count, plural, one {# elemento} other {# elementos}}",
  "mobile.vault.emptyTitle": "Aún no hay elementos",
  "mobile.vault.emptyBody":
    "Desliza hacia abajo para sincronizar con tus otros dispositivos o añade tu primer inicio de sesión.",
  "mobile.search.placeholder": "Buscar en tu caja fuerte",
  "mobile.copy.done":
    "Copiado. Se borra del portapapeles en {seconds, plural, one {# segundo} other {# segundos}}.",
  "mobile.setup.scanTitle": "Escanea tu código de configuración",
  "mobile.setup.scanHint":
    "En tu equipo, abre Mosaic, ve a Ajustes > Sincronización y elige “Configurar un teléfono”. Apunta la cámara al código.",
  "mobile.autofill.title": "Rellenar desde Mosaic",
  "mobile.autofill.target": "¿Rellenar en {app}?",
  "mobile.settings.title": "Ajustes",
  "mobile.settings.about": "Acerca de Mosaic",
  "mobile.settings.version": "Versión {version}",
  "mobile.common.back": "Atrás",
  "mobile.common.notNow": "Ahora no",
  "mobile.onboarding.step": "Paso {current} de {total}",
  "mobile.onboarding.welcome.title": "Configura Mosaic en este teléfono",
  "mobile.onboarding.welcome.body":
    "Inicia sesión en tu cuenta de Mosaic para traer aquí tus contraseñas. Todo permanece cifrado en este teléfono.",
  "mobile.onboarding.welcome.start": "Empezar",
  "mobile.onboarding.details.title": "Tu cuenta",
  "mobile.onboarding.details.server": "Dirección del servidor",
  "mobile.onboarding.details.serverHint": "https://sync.example.com",
  "mobile.onboarding.details.email": "Correo electrónico",
  "mobile.onboarding.details.secretKey": "Clave secreta",
  "mobile.onboarding.details.secretKeyHelp":
    "De tu Kit de emergencia: letras y números en grupos, que empiezan por M1.",
  "mobile.onboarding.details.deviceName": "Nombre de este teléfono",
  "mobile.onboarding.details.deviceNameDefault": "Teléfono Android",
  "mobile.onboarding.details.checking": "Comprobando el servidor",
  "mobile.onboarding.password.title": "Tu contraseña maestra",
  "mobile.onboarding.password.body":
    "Mosaic nunca la envía a ninguna parte. Desbloquea tu caja fuerte en este teléfono.",
  "mobile.onboarding.password.action": "Iniciar sesión",
  "mobile.onboarding.password.working": "Iniciando sesión. Puede tardar unos segundos.",
  "mobile.onboarding.second.title": "Un paso más",
  "mobile.onboarding.second.totpLabel": "Código de 6 dígitos",
  "mobile.onboarding.second.totpAction": "Verificar",
  "mobile.onboarding.second.keyAction": "Usa tu llave de seguridad",
  "mobile.onboarding.second.keyWaiting":
    "Esperando tu llave de seguridad. Termina en el navegador y vuelve aquí. El código que aparece allí debería ser {code}.",
  "mobile.onboarding.quick.title": "Desbloquea más rápido",
  "mobile.onboarding.quick.body":
    "Usa tu huella para desbloquear Mosaic la próxima vez. Tras reiniciar el teléfono sigues necesitando tu contraseña maestra una vez.",
  "mobile.onboarding.quick.enable": "Activar desbloqueo rápido",
  "mobile.onboarding.quick.unavailable":
    "Añade una huella en los ajustes de tu teléfono para usar el desbloqueo rápido.",
  "mobile.onboarding.autofill.title": "Rellena inicios de sesión automáticamente",
  "mobile.onboarding.autofill.body":
    "Deja que Mosaic ofrezca tus inicios de sesión en otras apps y en el navegador. Puedes cambiarlo más tarde en los ajustes del sistema.",
  "mobile.onboarding.autofill.open": "Abrir ajustes de autocompletar",
  "mobile.onboarding.done.title": "Todo listo",
  "mobile.onboarding.done.body": "Tu caja fuerte está en este teléfono y desbloqueada.",
  "mobile.onboarding.done.action": "Abrir tu caja fuerte",
  "mobile.unlock.account": "Desbloqueando {email}",
  "mobile.unlock.secretKey": "Clave secreta",
  "mobile.unlock.secretKeyHelp":
    "Este teléfono no tiene guardada tu Clave secreta. Escríbela desde tu Kit de emergencia.",
  "mobile.unlock.enrollNext": "Desbloquear con mi huella la próxima vez",
  "mobile.unlock.working": "Desbloqueando. Puede tardar unos segundos.",
  "mobile.unlock.biometricTitle": "Desbloquear Mosaic",
  "mobile.unlock.biometricSubtitle": "Confirma que eres tú",
  "mobile.unlock.usePassword": "Usar contraseña maestra",
  "mobile.unlock.busyTitle": "Mosaic está ocupado",
  "mobile.unlock.retry": "Reintentar",
  "mobile.home.unlockedAs": "Desbloqueado como {email}",
  "error.notFound":
    "Ese elemento ya no existe. Puede que se haya eliminado en otro dispositivo.",
  "error.syncFailed":
    "Mosaic no pudo sincronizar. Comprueba tu conexión y vuelve a intentarlo.",
  "error.passwordInApp":
    "Aquí no se puede desbloquear con la contraseña maestra. Abre la app de Mosaic y desbloquéala allí.",
  "error.folderProviderUnsupported":
    "Este teléfono no puede acceder a la carpeta en la nube de esta cuenta. Inicia sesión de nuevo en el proveedor de la carpeta o usa un equipo de escritorio.",
  "error.folderProviderSignedOut":
    "Tu servicio en la nube cerró el inicio de sesión o lo rechazó. Inicia sesión de nuevo para seguir sincronizando.",
  "error.folderNotFound":
    "Mosaic no encuentra esa carpeta o no es una carpeta de Mosaic. Elige la carpeta con la que se sincroniza tu equipo.",
  "error.folderStorageFull":
    "Tu almacenamiento en la nube está lleno. Libera espacio allí y vuelve a sincronizar.",
  "error.folderLinkExpired": "El inicio de sesión tardó demasiado. Vuelve a empezar.",
  "error.folderLinkInvalid":
    "El inicio de sesión no funcionó o se canceló. Vuelve a empezar.",
  "error.folderServerTlsTooOld":
    "Este servidor es demasiado antiguo para una conexión segura. Mosaic necesita TLS 1.3. Pide a quien administra el servidor que lo actualice.",
  "mobile.error.syncPasswordRollback":
    "La carpeta ofrece una contraseña maestra más antigua de la que conoce este teléfono. Revisa la carpeta y tu servicio en la nube.",
  "error.setupCodeInvalid":
    "Ese código de configuración no es válido o ha caducado. Crea uno nuevo en tu equipo y escanéalo de nuevo.",
  "error.notEnoughMemory":
    "Este dispositivo no tiene suficiente memoria libre para desbloquear tu caja fuerte. Cierra otras apps y vuelve a intentarlo.",
  "error.noMatch":
    "Mosaic no tiene ningún inicio de sesión para esta app o sitio web. Abre Mosaic para añadir uno.",
  "error.insecureOrigin":
    "Esta página no usa una conexión segura, así que Mosaic no la rellenará. Abre la versión https de la página.",
  "error.appNotLinked":
    "Esta app no está vinculada a un sitio web guardado. Vincúlala primero en Mosaic y vuelve a intentarlo.",
  "error.foreignFrame":
    "Este campo pertenece a otro sitio incrustado en la página, así que Mosaic no lo rellenará.",
  "error.assetLinksUnavailable":
    "Mosaic no pudo comprobar que esta app pertenece al sitio web. Comprueba tu conexión y vuelve a intentarlo.",
  "error.appNotVerified":
    "Esta app no está verificada para ese sitio web, así que Mosaic no usará sus llaves de acceso. Usa el sitio web en un navegador.",
  "error.rpIdInvalid":
    "El nombre del sitio web de esta solicitud no es válido. Cancela y empieza de nuevo.",
  "error.passkeyNotFound":
    "Mosaic no tiene ninguna llave de acceso para este sitio web. Inicia sesión de otra forma.",
  "error.passkeyExcluded":
    "Ya tienes una llave de acceso para esta cuenta en Mosaic. Inicia sesión con ella.",
  "error.passkeyUnsupportedAlgorithm":
    "Este sitio web necesita un tipo de llave de acceso que Mosaic aún no puede crear. Elige otra forma de iniciar sesión.",
  "error.passkeySessionExpired":
    "Esta solicitud de llave de acceso ha caducado. Empieza de nuevo desde la app.",
  "error.qrNotFound":
    "Mosaic no encontró ningún código QR en esa imagen. Vuelve a intentarlo con una imagen más nítida.",
  "error.storageUnavailable":
    "Mosaic no pudo acceder al almacenamiento seguro de este dispositivo. Reinicia tu teléfono y vuelve a intentarlo.",
  "mobile.common.tryAgain": "Reintentar",
  "mobile.onboarding.preAudit":
    "Software previo a una auditoría: aún sin revisión de seguridad externa.",
  "mobile.onboarding.start": "Empezar",
  "mobile.onboarding.server": "Dirección del servidor",
  "mobile.onboarding.serverPlaceholder": "https://",
  "mobile.onboarding.email": "Correo electrónico",
  "mobile.onboarding.secretKey": "Clave secreta",
  "mobile.onboarding.secretKeyHint":
    "La encuentras en tu Kit de emergencia o en Ajustes en tu equipo. Las mayúsculas y los guiones no importan.",
  "mobile.onboarding.deviceName": "Nombre de este teléfono",
  "mobile.onboarding.passwordHint":
    "Mosaic la comprueba en este teléfono. Nunca se envía al servidor.",
  "mobile.onboarding.signIn": "Iniciar sesión",
  "mobile.onboarding.working": "Iniciando sesión. Puede tardar unos segundos.",
  "mobile.onboarding.secondFactorTitle": "Un paso más",
  "mobile.onboarding.totpLabel": "Código de tu app de autenticación",
  "mobile.onboarding.verify": "Verificar",
  "mobile.onboarding.securityKey": "Usa tu llave de seguridad",
  "mobile.onboarding.securityKeyWaiting":
    "Termina el inicio de sesión en tu navegador. Comprueba que muestra el código {code} y vuelve aquí.",
  "mobile.onboarding.autofillSteps":
    "Abre la app Ajustes, toca Contraseñas, luego Opciones de contraseñas y activa Mosaic.",
  "mobile.onboarding.openSettings": "Abrir ajustes",
  "mobile.onboarding.notNow": "Ahora no",
  "mobile.unlock.secretKeyNeeded":
    "Este teléfono no tiene tu Clave secreta. Escríbela una vez para desbloquear.",
  "mobile.unlock.quickOptionFaceId": "Desbloquear con Face ID la próxima vez",
  "mobile.unlock.biometricReason": "Desbloquea tu caja fuerte de Mosaic",
  "mobile.unlock.faceIdReason":
    "Mosaic usa Face ID para desbloquear tu caja fuerte rápidamente. Tu contraseña maestra también funciona siempre.",
  "mobile.unlock.quickInvalidated":
    "Cambiaron tus ajustes de Face ID. Desbloquea con tu contraseña maestra para volver a activar el desbloqueo rápido.",
  "mobile.unlock.storeBusy": "Mosaic está ocupado. Vuelve a intentarlo en un momento.",
  "mobile.unlock.rebootNote":
    "Desbloquea una vez con tu contraseña maestra después de reiniciar.",
  "mobile.capture.hidden": "Oculto mientras se graba tu pantalla",
  "mobile.deviceLock.required.title": "Primero configura un bloqueo de pantalla",
  "mobile.deviceLock.required.body":
    "Mosaic guarda tus claves tras el bloqueo de pantalla de tu teléfono. Configura un PIN, un patrón o una contraseña en los ajustes del sistema y vuelve.",
  "mobile.deviceLock.required.action": "Abrir ajustes de seguridad",
  "mobile.common.show": "Mostrar",
  "mobile.common.hide": "Ocultar",
  "mobile.common.copy": "Copiar",
  "mobile.common.save": "Guardar",
  "mobile.common.remove": "Quitar",
  "mobile.common.edit": "Editar",
  "mobile.common.working": "Trabajando",
  "mobile.vault.add": "Añadir elemento",
  "mobile.vault.addLogin": "Inicio de sesión",
  "mobile.vault.addNote": "Nota segura",
  "mobile.vault.untitled": "Sin título",
  "mobile.vault.favorite": "Favorito",
  "mobile.search.hint": "Busca por título, nombre de usuario, sitio web o etiqueta.",
  "mobile.search.none": "Ningún elemento coincide con {query}.",
  "mobile.detail.favoriteAdd": "Añadir a favoritos",
  "mobile.detail.favoriteRemove": "Quitar de favoritos",
  "mobile.detail.delete": "Mover a la papelera",
  "mobile.detail.deleteTitle": "¿Mover “{title}” a la papelera?",
  "mobile.detail.deleteBody": "El cambio se sincroniza con tus otros dispositivos.",
  "mobile.detail.otpLeft": "{seconds, plural, one {Queda # segundo} other {Quedan # segundos}}",
  "mobile.detail.passkey": "Llave de acceso de {site}",
  "mobile.detail.attachments":
    "{count, plural, one {Hay # archivo adjunto. Ábrelo en tu equipo.} other {Hay # archivos adjuntos. Ábrelos en tu equipo.}}",
  "mobile.detail.linkedApp": "App: {name}",
  "mobile.detail.needsLinking": "Hay que vincularlo antes de poder rellenar",
  "mobile.detail.linkApp": "Vincular una app",
  "mobile.detail.linkSharedHint": "Las apps no se pueden vincular a inicios de sesión de una caja fuerte compartida.",
  "mobile.editor.newLogin": "Nuevo inicio de sesión",
  "mobile.editor.editLogin": "Editar inicio de sesión",
  "mobile.editor.newNote": "Nueva nota",
  "mobile.editor.editNote": "Editar nota",
  "mobile.editor.title": "Título",
  "mobile.editor.username": "Nombre de usuario o correo",
  "mobile.editor.password": "Contraseña",
  "mobile.editor.passwordKeep": "Déjalo vacío para conservar la contraseña actual.",
  "mobile.editor.websites": "Sitios web",
  "mobile.editor.websiteHint": "https://example.com",
  "mobile.editor.websiteAdd": "Añadir un sitio web",
  "mobile.editor.websiteRemove": "Quitar el sitio web {number}",
  "mobile.editor.notes": "Notas",
  "mobile.editor.favorite": "Marcar como favorito",
  "mobile.editor.saving": "Guardando",
  "mobile.editor.otpTitle": "Código de un solo uso",
  "mobile.editor.otpHave": "Este inicio de sesión tiene un código de un solo uso.",
  "mobile.editor.otpInput": "Clave de configuración o enlace otpauth",
  "mobile.editor.otpAdd": "Añadir el código",
  "mobile.editor.otpReady": "Listo: {name}. Se guarda con el inicio de sesión.",
  "mobile.editor.otpRemove": "Quitar el código",
  "mobile.editor.otpRemoved": "El código se quita al guardar.",
  "mobile.editor.generate": "Generar una contraseña",
  "mobile.generator.title": "Generador de contraseñas",
  "mobile.generator.password": "Caracteres",
  "mobile.generator.passphrase": "Palabras",
  "mobile.generator.length": "Longitud: {length}",
  "mobile.generator.lowercase": "Letras minúsculas",
  "mobile.generator.uppercase": "Letras mayúsculas",
  "mobile.generator.digits": "Dígitos",
  "mobile.generator.symbols": "Símbolos",
  "mobile.generator.ambiguous": "Omitir caracteres que se parecen",
  "mobile.generator.wordCount": "Palabras: {count}",
  "mobile.generator.separator": "Separador",
  "mobile.generator.capitalize": "Poner las palabras en mayúscula inicial",
  "mobile.generator.appendDigit": "Añadir un dígito",
  "mobile.generator.run": "Generar",
  "mobile.generator.use": "Usar esta contraseña",
  "mobile.generator.strength": "Unos {bits} bits de seguridad",
  "mobile.link.title": "Vincular una app",
  "mobile.link.choose": "Elige la app que vincular a {login}.",
  "mobile.link.filter": "Buscar una app",
  "mobile.link.app": "App",
  "mobile.link.package": "ID de la app",
  "mobile.link.installer": "Instalada desde",
  "mobile.link.installerUnknown": "Origen desconocido",
  "mobile.link.certificate": "Firma del desarrollador (inicio)",
  "mobile.link.verified": "Verificada por {host}",
  "mobile.link.notVerified": "Sin verificar",
  "mobile.link.unavailable": "No se pudo comprobar",
  "mobile.link.checking": "Comprobando el sitio web",
  "mobile.link.warning":
    "Vincula solo apps en las que confíes. Mosaic no puede confirmar que esta app pertenezca a {host}.",
  "mobile.link.warningNoHost":
    "Vincula solo apps en las que confíes. Este inicio de sesión no tiene ningún sitio web que Mosaic pueda comprobar.",
  "mobile.link.confirm": "Rellenar {login} en esta app a partir de ahora",
  "mobile.link.done": "Vinculada. Mosaic rellena {login} en esta app.",
  "mobile.link.noApps": "No se encontraron otras apps.",
  "mobile.settings.security": "Seguridad",
  "mobile.settings.autoLock": "Bloquear al salir de la app",
  "mobile.settings.autoLock.now": "Inmediatamente",
  "mobile.settings.autoLock.s30": "Tras 30 segundos",
  "mobile.settings.autoLock.s60": "Tras 1 minuto",
  "mobile.settings.autoLock.s300": "Tras 5 minutos",
  "mobile.settings.idle": "Bloquear cuando dejes de tocar la pantalla",
  "mobile.settings.idleOption": "{minutes, plural, one {Tras # minuto} other {Tras # minutos}}",
  "mobile.settings.clipboard": "Borrar los valores copiados",
  "mobile.settings.clipboardOption":
    "{seconds, plural, one {Tras # segundo} other {Tras # segundos}}",
  "mobile.settings.quick": "Desbloqueo rápido",
  "mobile.settings.quickOn": "El desbloqueo rápido está activado.",
  "mobile.settings.quickOff": "El desbloqueo rápido está desactivado.",
  "mobile.settings.quickEnable": "Activar",
  "mobile.settings.quickDisable": "Desactivar",
  "mobile.settings.quickPassword": "Escribe tu contraseña maestra para activar el desbloqueo rápido.",
  "mobile.settings.sync": "Sincronización",
  "mobile.about.preAudit":
    "Mosaic es software previo a una auditoría. Aún no se ha realizado ninguna auditoría de seguridad independiente.",
  "mobile.about.privacy":
    "Ningún dato de análisis, seguimiento ni informe de fallos sale de este teléfono.",
  "mobile.about.licences": "Licencias de código abierto",
  "mobile.licences.title": "Licencias de código abierto",
  "mobile.licences.intro": "Mosaic para Android está creado con estas partes de código abierto.",
  "mobile.licences.rust":
    "El núcleo en Rust usa crates de código abierto. Sus licencias figuran en el repositorio de código fuente del proyecto.",
  "mobile.autofill.unlockEntry": "Desbloquear Mosaic",
  "mobile.autofill.noLogin": "No hay inicio de sesión para {app}",
  "mobile.autofill.noLoginHint": "Toca para vincular un inicio de sesión",
  "mobile.autofill.cannotCheck": "No se pudo comprobar esta app",
  "mobile.autofill.warning": "Comprueba primero la dirección",
  "mobile.autofill.noUsername": "Sin nombre de usuario",
  "mobile.autofill.otpEntry": "Código de un solo uso",
  "mobile.link.pickLogin": "Elige el inicio de sesión que Mosaic debe rellenar en {app}.",
  "mobile.credential.unlockEntry": "Desbloquear Mosaic",
  "mobile.credential.passwordEntry": "Contraseña",
  "mobile.credential.passkeyEntry": "Llave de acceso",
  "mobile.credential.createEntry": "Guardar llave de acceso en Mosaic",
  "mobile.credential.createTitle": "¿Guardar una llave de acceso?",
  "mobile.credential.createBody": "Mosaic guardará una llave de acceso para {site} ({user}).",
  "mobile.credential.createVault": "Guardar en",
  "mobile.credential.vaultDefault": "Caja fuerte personal",
  "mobile.credential.createConfirm": "Guardar llave de acceso",
  "mobile.credential.verifyTitle": "Confirma que eres tú",
  "mobile.credential.verifySignIn": "Iniciar sesión en {site}",
  "mobile.credential.verifyCreate": "Guardar una llave de acceso para {site}",
  "mobile.credential.verifyFailed": "Mosaic no pudo confirmar que eres tú. Vuelve a intentarlo.",
  "mobile.credential.unsupported":
    "Mosaic no puede responder a esta solicitud de inicio de sesión. Cancela y elige otra forma de iniciar sesión.",
  "mobile.settings.fill": "Rellenar en otras apps",
  "mobile.settings.autofillOn": "Mosaic es tu servicio de autocompletar.",
  "mobile.settings.autofillOff": "Mosaic aún no es tu servicio de autocompletar.",
  "mobile.settings.autofillEnable": "Usar Mosaic para autocompletar",
  "mobile.settings.credentialHint":
    "Android también puede ofrecer tus llaves de acceso y contraseñas de Mosaic cuando una app te pide iniciar sesión.",
  "mobile.settings.credentialEnable": "Usar Mosaic para llaves de acceso y contraseñas",
  "mobile.scan.setupAction": "Escanear código de configuración",
  "mobile.scan.otpAction": "Escanear un código QR",
  "mobile.scan.otpTitle": "Escanea el código de un solo uso",
  "mobile.scan.otpHint":
    "Apunta la cámara al código QR del sitio web. Mosaic lo lee solo en este teléfono.",
  "mobile.scan.preview": "Vista de la cámara",
  "mobile.scan.looking": "Buscando un código QR",
  "mobile.scan.notSetupCode": "No es un código de configuración de Mosaic",
  "mobile.scan.notSetupCodeHelp":
    "Apunta la cámara al código que Mosaic muestra en tu equipo, en Ajustes, en Sincronización.",
  "mobile.scan.rationale":
    "Mosaic usa la cámara solo para leer este código QR. No se guarda ni se envía nada.",
  "mobile.scan.allow": "Permitir la cámara",
  "mobile.scan.denied":
    "Mosaic no puede usar la cámara. Permítela en los ajustes de este teléfono para escanear o escribe los datos a mano.",
  "mobile.scan.openSettings": "Abrir ajustes",
  "mobile.scan.unavailable": "Mosaic no pudo iniciar la cámara. Escribe los datos a mano.",
  "mobile.onboarding.details.scanned":
    "El servidor, el correo y la Clave secreta se rellenaron a partir del código de configuración. Revísalos y continúa.",
  "mobile.onboarding.details.scanOr": "O escribe los datos a mano",
  "mobile.onboarding.details.keyFromCode": "Clave secreta del código de configuración",
  "mobile.settings.passkeyIdentities": "Usar Mosaic para llaves de acceso",
  "mobile.settings.passkeyIdentitiesHint":
    "Mosaic informa al sistema del sitio web y del nombre de usuario de cada llave de acceso para que pueda ofrecerla. Las claves nunca salen de Mosaic.",
  "mobile.settings.passwordIdentities": "Sugerir inicios de sesión sobre el teclado",
  "mobile.settings.passwordIdentitiesHint":
    "Mosaic informa al sistema del sitio web y del nombre de usuario de cada inicio de sesión. El sistema guarda esta lista en el teléfono, fuera del cifrado de Mosaic.",
  "mobile.settings.identitiesPassword": "Escribe tu contraseña maestra para activar esto.",
  "mobile.licences.introIos": "Mosaic para iPhone está creado con estas partes de código abierto.",
  "mobile.ext.openApp": "Abre Mosaic y desbloquéalo una vez con tu contraseña maestra.",
  "mobile.ext.noAccount":
    "Mosaic aún no tiene ninguna cuenta en este iPhone. Abre Mosaic para configurarlo.",
  "mobile.ext.loading": "Abriendo tu caja fuerte",
  "mobile.ext.thisSite": "este sitio",
  "mobile.ext.matches": "Inicios de sesión para {site}",
  "mobile.ext.noMatches": "No hay inicio de sesión para {site}",
  "mobile.ext.noMatchesHint":
    "Mosaic rellena solo los inicios de sesión guardados para este sitio. Abre Mosaic para añadir este sitio a un inicio de sesión.",
  "mobile.ext.searchAll": "Buscar en todos los inicios de sesión",
  "mobile.ext.hideAll": "Ocultar otros inicios de sesión",
  "mobile.ext.notSavedFor": "No guardado para {site}",
  "mobile.ext.fillHint": "Rellena este inicio de sesión",
  "mobile.ext.warnPunycode":
    "Esta dirección está escrita con caracteres especiales: {unicode}. Comprueba que es el sitio que quieres.",
  "mobile.ext.warnLookalike":
    "Esta dirección se parece a {site}, que guardaste en Mosaic. Comprueba que es el sitio que quieres.",
  "mobile.ext.otpTitle": "Elige un código de un solo uso",
  "mobile.ext.passkeyTitle": "Elige una llave de acceso",
  "mobile.ext.configTitle": "Autocompletar de Mosaic",
  "mobile.ext.configBody":
    "Mosaic ofrece tus inicios de sesión, llaves de acceso y códigos de un solo uso cuando tocas un campo o un botón de inicio de sesión. Cambias lo que ofrece en Ajustes, en la app de Mosaic.",
  "mobile.ext.confirmFillTitle": "¿Rellenar el inicio de sesión de {site}?",
  "mobile.ext.confirmFillBody":
    "iOS no le dice a Mosaic en qué página estás, así que Mosaic no puede comprobarlo. Rellena solo si es el sitio que quieres. Después, Mosaic pide Face ID o Touch ID.",
  "mobile.ext.confirmFillButton": "Rellenar este inicio de sesión",
  "mobile.settings.passwordIdentitiesLimit":
    "Cuando tocas una sugerencia, iOS no le dice a Mosaic en qué página estás. Por eso Mosaic muestra el sitio guardado y rellena solo después de que lo confirmes y desbloquees con Face ID o Touch ID.",
} satisfies Record<keyof typeof en, string>;
