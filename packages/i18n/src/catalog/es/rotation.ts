/**
 * Spanish UI strings: rotation. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { rotation as en } from "../en/rotation.ts";

export const rotation = {
  "rotation.section.title": "Contraseña y claves",
  "rotation.section.body":
    "Cambia tu contraseña maestra, obtén una Clave secreta nueva o pasa toda tu cuenta a claves nuevas. Cada cambio te muestra lo que ocurrirá antes de tocar nada.",
  "rotation.action.password": "Cambiar contraseña maestra",
  "rotation.action.password.body":
    "Elige una contraseña maestra nueva. Tu Clave secreta y tus cajas fuertes se quedan como están.",
  "rotation.action.password.button": "Cambiar…",
  "rotation.action.secretKey": "Renovar Clave secreta",
  "rotation.action.secretKey.body":
    "Obtén una Clave secreta nueva y un Kit de emergencia nuevo. Úsalo si perdiste tu kit o si alguien pudo verlo.",
  "rotation.action.secretKey.button": "Renovar…",
  "rotation.action.profile": "Mejorar la protección",
  "rotation.action.profile.body":
    "Dificulta adivinar tu contraseña maestra usando más memoria en cada intento. Desbloquear tarda un poco más.",
  "rotation.action.profile.button": "Mejorar…",
  "rotation.action.rekey": "Renovar todas las claves de la cuenta",
  "rotation.action.rekey.body":
    "Renueva todas las claves de tu cuenta: una Clave secreta nueva, claves nuevas para las cajas fuertes y un código de seguridad nuevo. Úsalo si alguien pudo copiar los datos de tu caja fuerte junto con tu contraseña y tu Clave secreta.",
  "rotation.action.rekey.button": "Renovar claves…",
  "rotation.action.rekey.needsSync":
    "Renovar todas las claves actualiza la copia de tu servidor o carpeta de sincronización, así que necesita la sincronización. Configúrala primero.",
  "rotation.title.password": "Cambiar contraseña maestra",
  "rotation.title.secretKey": "Renovar Clave secreta",
  "rotation.title.profile": "Mejorar la protección",
  "rotation.title.rekey": "Renovar todas las claves de la cuenta",
  "rotation.step": "Paso {current} de {total}",
  "rotation.back": "Atrás",
  "rotation.stop.title": "¿Detener este cambio?",
  "rotation.stop.body": "El Kit de emergencia que acabas de guardar no funcionará. Conserva tu kit anterior.",
  "rotation.stop.confirm": "Detener cambio",
  "rotation.stop.keep": "Seguir",
  "rotation.next": "Continuar",
  "rotation.close": "Cerrar",
  "rotation.review.title.password": "Antes de cambiar tu contraseña",
  "rotation.review.title.secretKey": "Antes de renovar tu Clave secreta",
  "rotation.review.title.profile": "Antes de mejorar la protección",
  "rotation.review.title.rekey": "¿Se comprometió algo?",
  "rotation.review.changes": "Qué cambia",
  "rotation.review.stays": "Qué se queda igual",
  "rotation.review.password.changes":
    "Tu contraseña maestra. El desbloqueo rápido de este dispositivo se desactiva y puedes volver a configurarlo después.",
  "rotation.review.password.stays":
    "Tu Clave secreta, tus cajas fuertes y elementos, tu código de seguridad y tus contactos de emergencia.",
  "rotation.review.password.devices":
    "Tus otros dispositivos cierran la sesión de sincronización. En cada uno, escribe una vez la nueva contraseña maestra y la sincronización continúa.",
  "rotation.review.secretKey.changes":
    "Tu Clave secreta y tu Kit de emergencia. El desbloqueo rápido de este dispositivo se desactiva y puedes volver a configurarlo después.",
  "rotation.review.secretKey.stays":
    "Tu contraseña maestra, tus cajas fuertes y elementos, tu código de seguridad y tus contactos de emergencia. Las llaves de seguridad que registraste siguen funcionando.",
  "rotation.review.secretKey.devices":
    "Tus otros dispositivos cierran la sesión de sincronización. En cada uno, escribe una vez tu contraseña y la nueva Clave secreta del kit nuevo.",
  "rotation.review.secretKey.limit":
    "Esto impide iniciar sesión con la Clave secreta anterior. Quien ya haya copiado los datos de tu caja fuerte con tu contraseña y tu Clave secreta anteriores aún puede leerlos. Renueva todas las claves de la cuenta para evitarlo.",
  "rotation.review.secretKey.compromised": "Se comprometió algo: mejor renovar todas las claves",
  "rotation.review.profile.changes":
    "Cuánta memoria necesita cada intento de desbloqueo, para ti y para cualquiera que intente adivinar. El desbloqueo rápido de este dispositivo se desactiva y puedes volver a configurarlo después.",
  "rotation.review.profile.stays":
    "Tu contraseña maestra, tu Clave secreta, tus cajas fuertes y tu código de seguridad. Las llaves de seguridad que registraste siguen funcionando.",
  "rotation.review.profile.devices":
    "Tus otros dispositivos cierran la sesión de sincronización. En cada uno, escribe una vez tu contraseña maestra y la sincronización continúa. Los dispositivos más lentos tardan un poco más en desbloquear.",
  "rotation.review.rekey.intro":
    "Renovar todas las claves es para el caso de que alguien pueda tener los datos de tu caja fuerte y tu contraseña y Clave secreta. Si solo perdiste un dispositivo, basta con renovar la Clave secreta o cambiar tu contraseña.",
  "rotation.review.rekey.yes": "Sí, renovar todas las claves de mi cuenta",
  "rotation.review.rekey.no": "No, mejor solo renovar la Clave secreta",
  "rotation.review.rekey.limit":
    "Renovar todas las claves no recupera lo que alguien ya copió. Las copias antiguas en servidores y copias de seguridad siguen cifradas con las claves anteriores, y las claves anteriores aún las abren.",
  "rotation.review.rekey.title": "Qué hace renovar todas las claves",
  "rotation.review.rekey.vaults":
    "{count, plural, one {# caja fuerte que solo usas tú recibe una clave nueva.} other {# cajas fuertes que solo usas tú reciben claves nuevas.}}",
  "rotation.review.rekey.keys":
    "Una Clave secreta nueva, un Kit de emergencia nuevo, un código de seguridad nuevo y una clave de cuenta nueva.",
  "rotation.review.rekey.devices":
    "Tus otros dispositivos cierran la sesión y no pueden sincronizar hasta que escribas en cada uno la nueva Clave secreta y tu nueva contraseña maestra.",
  "rotation.review.rekey.shared":
    "{count, plural, one {Compartes # caja fuerte.} other {Compartes # cajas fuertes.}} Cada una recibe una clave nueva.",
  "rotation.review.rekey.sharedRow":
    "{name}, {count, plural, one {# miembro} other {# miembros}}",
  "rotation.review.rekey.sharedMembers":
    "Pide a cada persona con quien compartes que compare tu nuevo código de seguridad. Hasta que lo haga, no recibe claves nuevas de ti y no puede abrir la caja fuerte renovada.",
  "rotation.review.rekey.memberVaults":
    "{count, plural, one {# caja fuerte pertenece a otra persona.} other {# cajas fuertes pertenecen a otras personas.}} Se pide a sus propietarios que las renueven. Hasta que lo hagan, tus claves anteriores aún pueden leerlas.",
  "rotation.review.rekey.grants":
    "{count, plural, one {# persona tiene acceso de emergencia a tus cajas fuertes.} other {# personas tienen acceso de emergencia a tus cajas fuertes.}} Su acceso se renueva para las claves nuevas. Cada una de ellas también tiene que comparar tu nuevo código de seguridad.",
  "rotation.review.rekey.quick":
    "El desbloqueo rápido de este dispositivo se desactiva. Puedes volver a configurarlo después.",
  "rotation.review.rekey.hardwareKeys":
    "{count, plural, one {# llave de seguridad que registraste para desbloquear se desactiva.} other {# llaves de seguridad que registraste para desbloquear se desactivan.}} Puedes volver a registrarlas después.",
  "rotation.review.rekey.hardwareVaults":
    "{count, plural, one {# caja fuerte protegida por hardware no se ve afectada.} other {# cajas fuertes protegidas por hardware no se ven afectadas.}}",
  "rotation.review.rekey.attachments":
    "{count, plural, one {# adjunto no está en este dispositivo.} other {# adjuntos no están en este dispositivo.}} Se descargan primero para poder protegerlos de nuevo. Los que no se puedan alcanzar conservan la clave anterior.",
  "rotation.review.rekey.folder":
    "Tu carpeta de sincronización recibe un archivo de cuenta nuevo. Los demás dispositivos encuentran allí una nota de que las claves cambiaron.",
  "rotation.review.rekey.unavailable":
    "Renovar todas las claves no está disponible: la sincronización no está configurada.",
  "rotation.review.loading": "Comprobando qué afecta renovar todas las claves…",
  "rotation.review.rekey.needsClean":
    "Renovar todas las claves necesita que la sincronización esté al día y sin avisos. Sincroniza ahora, revisa los avisos que haya y vuelve a intentarlo.",
  "rotation.credentials.title": "Confirma que eres tú",
  "rotation.credentials.password.title": "Tus contraseñas maestras",
  "rotation.credentials.current": "Contraseña maestra actual",
  "rotation.credentials.new": "Contraseña maestra nueva",
  "rotation.credentials.confirm": "Escribe de nuevo la contraseña maestra nueva",
  "rotation.credentials.mismatch": "Las dos contraseñas son distintas.",
  "rotation.credentials.same":
    "La contraseña maestra nueva tiene que ser distinta de la actual.",
  "rotation.credentials.hint":
    "Una frase de contraseña larga es mejor que una corta y complicada. No se puede recuperar: Mosaic no la conoce.",
  "rotation.credentials.code": "Código de 6 dígitos de tu app de autenticación",
  "rotation.credentials.profile": "Nivel de protección",
  "rotation.credentials.profile.standard": "Estándar: 256 MB de memoria por intento",
  "rotation.credentials.profile.strong": "Fuerte: 512 MB de memoria por intento",
  "rotation.credentials.profile.max": "Máximo: 1 GB de memoria por intento",
  "rotation.credentials.profile.slow":
    "Con este nivel, desbloquear tarda a este dispositivo más de dos segundos. Otros dispositivos más antiguos serán aún más lentos.",
  "rotation.credentials.profile.force": "Mejorar de todos modos",
  "rotation.kit.title": "Guarda tu nuevo Kit de emergencia",
  "rotation.kit.body":
    "Tu nueva Clave secreta está en este kit. Guárdalo primero: Mosaic cambia tu cuenta solo después de que confirmes que lo tienes, porque sin él no puedes iniciar sesión en un dispositivo nuevo.",
  "rotation.kit.save": "Guardar nuevo Kit de emergencia",
  "rotation.kit.saved": "Guardado. Consérvalo en un lugar seguro, aparte de tus dispositivos.",
  "rotation.kit.cancelled": "Aún no se guardó nada.",
  "rotation.kit.verify": "Escribe el último grupo de tu nueva Clave secreta desde el kit guardado.",
  "rotation.kit.confirm": "Confirmar kit",
  "rotation.kit.mistyped": "Ese no es el final de la nueva Clave secreta. Revisa el kit guardado.",
  "rotation.kit.verified": "Kit confirmado.",
  "rotation.confirm.title.password": "Cambia tu contraseña maestra",
  "rotation.confirm.title.secretKey": "Renueva tu Clave secreta",
  "rotation.confirm.title.profile": "Mejora la protección",
  "rotation.confirm.title.rekey": "Renueva todas las claves de tu cuenta",
  "rotation.confirm.body.rekey":
    "Esto cierra la sesión de tus otros dispositivos y renueva todas las claves. Mantén Mosaic abierto hasta que termine.",
  "rotation.confirm.body.secretKey":
    "Esto cierra la sesión de sincronización de tus otros dispositivos. Mantén Mosaic abierto hasta que termine.",
  "rotation.confirm.verb.password": "Cambiar contraseña",
  "rotation.confirm.verb.secretKey": "Renovar Clave secreta",
  "rotation.confirm.verb.profile": "Mejorar protección",
  "rotation.confirm.verb.rekey": "Renovar claves de la cuenta",
  "rotation.run.title": "Renovando tus claves…",
  "rotation.run.keepOpen":
    "Mantén Mosaic abierto. Con muchos adjuntos puede tardar unos minutos.",
  "rotation.progress.label": "Progreso",
  "rotation.progress.count": "{done} de {total}",
  "rotation.progress.done": "hecho",
  "rotation.phase.prepared": "Preparando",
  "rotation.phase.backendStarted": "Pidiendo a tu cuenta que cambie a las claves nuevas",
  "rotation.phase.backendAccepted": "Tu cuenta aceptó las claves nuevas",
  "rotation.phase.folderWritten": "La carpeta de sincronización tiene el archivo de cuenta nuevo",
  "rotation.phase.localCommitted": "Este dispositivo ya usa las claves nuevas",
  "rotation.phase.attachments": "Protegiendo de nuevo los adjuntos",
  "rotation.phase.uploading": "Enviando los datos protegidos",
  "rotation.phase.done": "Terminado",
  "rotation.done.title.password": "Contraseña maestra cambiada",
  "rotation.done.title.secretKey": "Clave secreta renovada",
  "rotation.done.title.profile": "Protección mejorada",
  "rotation.done.title.rekey": "Claves de la cuenta renovadas",
  "rotation.done.body.password":
    "Usa la nueva contraseña maestra a partir de ahora. Tus otros dispositivos la pedirán una vez.",
  "rotation.done.body.secretKey":
    "Guarda a salvo el nuevo Kit de emergencia y elimina el anterior. Tus otros dispositivos pedirán una vez la nueva Clave secreta.",
  "rotation.done.body.profile":
    "Desbloquear ahora necesita más memoria. Tus otros dispositivos pedirán tu contraseña una vez.",
  "rotation.done.body.rekey":
    "Guarda a salvo el nuevo Kit de emergencia y elimina el anterior. Tus otros dispositivos pedirán una vez la nueva Clave secreta y tu nueva contraseña maestra.",
  "rotation.done.members":
    "Pide a todas las personas con quienes compartes una caja fuerte que comparen tu nuevo código de seguridad. Hasta que lo hagan, no reciben claves nuevas de ti.",
  "rotation.done.quick": "Configura de nuevo el desbloqueo rápido",
  "rotation.done.quick.body": "El desbloqueo rápido se desactivó.",
  "rotation.done.hardware": "Registra de nuevo las llaves de seguridad",
  "rotation.done.hardware.body":
    "{count, plural, one {Se desactivó # llave de seguridad para desbloquear.} other {Se desactivaron # llaves de seguridad para desbloquear.}}",
  "rotation.done.unreachable.note":
    "Estos archivos no se pudieron leer y no se volvieron a cifrar. Conservan su clave de archivo anterior hasta que los quites o los vuelvas a añadir, así que quien tenga tus claves anteriores y una copia de un archivo aún podría abrirlo.",
  "rotation.done.unreachable.file": "{item}: {file}",
  "rotation.done.unreachable.unknownItem": "Elemento desconocido",
  "rotation.done.unreachable.unknownFile": "archivo desconocido",
  "rotation.done.unreachable.more":
    "{count, plural, one {y # archivo más} other {y # archivos más}}",
  "rotation.done.unfinished":
    "La última parte continúa más tarde. Si Mosaic se cierra antes de terminar, elige Continuar la rotación de claves la próxima vez.",
  "rotation.resume.title": "Cambio de claves sin terminar",
  "rotation.resume.body.password":
    "Se interrumpió un cambio de claves. No se perdió nada. Elige Continuar para volver a escribir tus contraseñas o descártalo.",
  "rotation.resume.body.secretKey":
    "Se interrumpió la renovación de tu Clave secreta. No se perdió nada. Continúa para terminarla.",
  "rotation.resume.body.rekey":
    "Se interrumpió la renovación de las claves de tu cuenta. No se perdió nada. Continúa para terminarla.",
  "rotation.resume.body.profile":
    "Se interrumpió la mejora de la protección. No se perdió nada. Continúa para terminarla.",
  "rotation.resume.continue": "Continuar la rotación de claves",
  "rotation.resume.discard": "Descartar…",
  "rotation.resume.running": "Continuando…",
  "rotation.resume.progress":
    "{done} de {total} hechos. Mantén Mosaic abierto hasta que termine.",
  "rotation.resume.dialog.title": "Continuar la rotación de claves",
  "rotation.resume.dialog.password": "Contraseña maestra que tienes ahora",
  "rotation.resume.dialog.group": "Último grupo de la nueva Clave secreta",
  "rotation.resume.dialog.body":
    "Puede que las claves de tu cuenta ya hayan cambiado. Para asegurarnos de que eres tú, escribe tu contraseña maestra y el último grupo de la nueva Clave secreta del nuevo Kit de emergencia.",
  "rotation.resume.dialog.submit": "Continuar",
  "rotation.resume.discard.title": "¿Descartar este cambio de claves?",
  "rotation.resume.discard.body":
    "Se eliminan la nueva Clave secreta y el nuevo Kit de emergencia de este intento. Tu cuenta se queda como está.",
  "rotation.resume.discard.confirm": "Descartar cambio de claves",
  "rotation.resume.abandon": "Renunciar al resto…",
  "rotation.resume.stalled":
    "No se pudo terminar de continuar. Si sigue fallando, puedes detenerlo aquí.",
  "rotation.abandon.title": "¿Dejar de volver a cifrar tus archivos?",
  "rotation.abandon.body":
    "Tus nuevas claves ya están en uso. Mosaic deja de volver a cifrar los adjuntos que siguen esperando. Conservan su clave de archivo anterior, así que quien tenga tus claves anteriores y una copia de un archivo aún podría abrirlo.",
  "rotation.abandon.stays":
    "Todo lo que ya se volvió a cifrar se queda como está. Quita o vuelve a añadir después los archivos en espera para protegerlos con la clave nueva.",
  "rotation.abandon.confirm": "Dejar de volver a cifrar",
  "rotation.abandon.keep": "Seguir intentándolo",
  "rotation.marker.title": "Aviso de renovación de claves sin confirmar",
  "rotation.marker.hint":
    "Si no renovaste tus claves en otro dispositivo, puede que este aviso no sea genuino. Puedes ignorarlo.",
  "rotation.marker.open": "Ignorar este aviso…",
  "rotation.marker.open.label": "Ignorar este aviso: {name}",
  "rotation.marker.confirm":
    "Mosaic encontró un aviso de renovación de claves que no pudo confirmar. Si no renovaste las claves de tu cuenta en otro dispositivo, puedes ignorarlo.",
  "rotation.marker.ignore": "Ignorar aviso",
  "rotation.marker.keep": "Conservar aviso",
  "rotation.ignored.title": "Avisos de renovación de claves ignorados",
  "rotation.ignored.body":
    "Avisos que elegiste ignorar. Restaura uno si sí renovaste las claves de tu cuenta en otro dispositivo.",
  "rotation.ignored.row": "Aviso de renovación de claves de tu carpeta de sincronización",
  "rotation.ignored.restore": "Restaurar",
  "rotation.ignored.restore.label": "Restaurar el aviso {name}",
  "rotation.resume.done": "Cambio de claves terminado.",
  "rotation.recovery.title": "Las claves de tu cuenta se renovaron en otro dispositivo",
  "rotation.recovery.body":
    "Un dispositivo tuyo renovó las claves de esta cuenta. Para seguir sincronizando, escribe tu contraseña maestra y la Clave secreta del nuevo Kit de emergencia.",
  "rotation.recovery.password": "Contraseña maestra (la nueva, si la cambiaste allí)",
  "rotation.recovery.secretKey": "Clave secreta del nuevo Kit de emergencia",
  "rotation.recovery.submit": "Usar las claves nuevas",
  "rotation.recovery.warning":
    "¿No renovaste tú las claves? Entonces puede que lo haya hecho alguien con tu contraseña y tu Clave secreta anteriores. No escribas nada aquí; revisa primero tus otros dispositivos.",
  "rotation.recovery.notice":
    "La sincronización está en pausa hasta que hagas esto en este dispositivo.",
  "rotation.recovery.open": "Usar las claves nuevas…",
  "rotation.recovery.keyHint":
    "Si tus claves se renovaron en otro dispositivo, escribe también la nueva Clave secreta de su Kit de emergencia.",
  "rotation.recovery.done": "Este dispositivo ya usa las claves nuevas.",
  "rotation.recovery.optional":
    "Clave secreta del Kit de emergencia más reciente (solo si la renovaste)",
  "rotation.file.unknown":
    "Apareció un archivo de cuenta desconocido en tu carpeta de sincronización. No se puede comprobar con tus claves: puede ser un resto, o alguien cambió tu carpeta. Tus datos no se ven afectados.",
  "rotation.file.dismiss": "Descartar",
  "rotation.file.conflict.title": "Dos dispositivos renovaron las claves a la vez",
  "rotation.file.conflict":
    "Otro dispositivo renovó las claves de tu cuenta en el mismo momento que este, así que ya no coinciden. Vuelve a renovar todas las claves desde este dispositivo para unificarlas: las claves de este dispositivo prevalecen y el otro pedirá la nueva Clave secreta.",
  "rotation.file.conflict.rekey": "Renovar todas las claves de nuevo desde este dispositivo",
  "rotation.file.conflict.keep": "Ocultar aviso",
  "rotation.resume.body.vault":
    "Se interrumpió el nuevo cifrado de los adjuntos de una caja fuerte. No se perdió nada. Continúa para terminarlo.",
} satisfies Record<keyof typeof en, string>;
