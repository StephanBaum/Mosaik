/**
 * Spanish UI strings: errors. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { errors as en } from "../en/errors.ts";

export const errors = {
  "error.notImplemented": "Esto aún no está disponible. Llegará en una versión posterior de Mosaic.",
  "error.noAccount":
    "Aún no hay ninguna cuenta en este dispositivo. Crea una o inicia sesión con tu Kit de emergencia.",
  "error.accountExists": "Este dispositivo ya tiene una cuenta de Mosaic. Desbloquéala.",
  "error.locked": "Mosaic está bloqueado. Desbloquéalo para continuar.",
  "error.alreadyUnlocked": "Mosaic ya está desbloqueado. Puedes seguir donde lo dejaste.",
  "error.wrongCredentials":
    "Esa contraseña maestra o Clave secreta no funcionó. Revisa las mayúsculas y la distribución del teclado y vuelve a intentarlo.",
  "error.secretKeyRequired":
    "Este dispositivo no tiene tu Clave secreta. Escríbela desde tu Kit de emergencia.",
  "error.secretKeyMalformed":
    "Eso no es una Clave secreta. La tuya empieza por “M1-” y está en tu Kit de emergencia.",
  "error.secretKeyMistyped":
    "Esa Clave secreta tiene una errata. Compárala con tu Kit de emergencia, carácter por carácter.",
  "error.emptyPassword": "La contraseña maestra está vacía. Escríbela para continuar.",
  "error.quickUnlockUnavailable":
    "El desbloqueo rápido no está configurado en este dispositivo. Desbloquea con tu contraseña maestra.",
  "error.storeInUse":
    "Mosaic ya está abierto en este equipo. Cierra la otra ventana y vuelve a intentarlo.",
  "error.storeUnavailable":
    "Mosaic no pudo abrir el archivo de tu caja fuerte en este dispositivo. Comprueba que el disco esté disponible y tenga espacio libre, y reinicia Mosaic.",
  "error.vaultNotFound":
    "Esa caja fuerte ya no existe. Puede que se haya eliminado en otro dispositivo.",
  "error.vaultUnavailable":
    "Esta caja fuerte no se puede abrir en esta sesión. Desbloquéala con su llave de hardware o sincroniza e inténtalo de nuevo.",
  "error.itemNotFound":
    "Ese elemento ya no existe. Puede que se haya movido o eliminado en otro dispositivo.",
  "error.itemDeleted":
    "Ese elemento se eliminó definitivamente, así que no se puede abrir. Elige otro elemento.",
  "error.sameVault": "El elemento ya está en esa caja fuerte. Elige otra caja fuerte.",
  "error.noOtp": "Este elemento no tiene ningún código de un solo uso. Edítalo para añadir uno.",
  "error.fieldEmpty":
    "Ese campo está vacío, así que no hay nada que copiar ni mostrar. Edita el elemento para rellenarlo.",
  "error.invalidInput":
    "Algo de este formulario no es válido. Revisa los campos resaltados y vuelve a intentarlo.",
  "error.sshKeyInvalid":
    "Esto no es una clave privada de OpenSSH. Pega la clave completa, incluidas las líneas BEGIN y END.",
  "error.reauthRequired": "Este cambio necesita tu contraseña maestra. Escríbela para confirmar.",
  "error.attachmentTooLarge": "Ese archivo supera los 50 MiB. Elige un archivo más pequeño.",
  "error.attachmentQuota":
    "Tus archivos usarían más del 1 GiB disponible. Quita algunos adjuntos y vuelve a intentarlo.",
  "error.attachmentNotFound": "Este archivo ya no forma parte del elemento. Vuelve a cargar el elemento.",
  "error.attachmentUnavailable":
    "Este archivo no se puede abrir en este dispositivo. Puede que aún no esté sincronizado o que esté dañado.",
  "error.attachmentFileUnreadable": "No se puede leer ese archivo. Elige otro archivo.",
  "error.attachmentSaveFailed": "No se pudo guardar el archivo allí. Elige otra carpeta.",
  "error.attachmentPreviewUnsupported":
    "Este archivo no tiene vista previa. Usa Guardar como para abrirlo.",
  "error.attachmentPreviewTooLarge":
    "Esta imagen es demasiado grande para la vista previa. Usa Guardar como para abrirla.",
  "error.clipboardUnavailable":
    "Mosaic no pudo usar el portapapeles. Cierra las apps que puedan estar usándolo y vuelve a intentarlo.",
  "error.importFileUnreadable":
    "Mosaic no pudo leer ese archivo. Comprueba que aún exista y que no esté abierto en otra app, y vuelve a intentarlo.",
  "error.importDeleteNotAllowed":
    "Importa primero el archivo. Mosaic solo elimina una exportación después de que sus elementos estén en tu caja fuerte.",
  "error.importDeleteFailed":
    "No se pudo eliminar el archivo. Puede que esté abierto en otra app o sea de solo lectura. Elimínalo tú mismo y vacía la papelera de reciclaje.",
  "error.importFormatInvalid":
    "Ese archivo no tiene el formato que elegiste. Elige el formato correspondiente o vuelve a exportar el archivo.",
  "error.importPreviewExpired":
    "Se canceló esta importación. Elige el archivo de nuevo para empezar otra vez.",
  "error.sshRequestExpired":
    "Esa solicitud de SSH ya terminó. Vuelve a ejecutar el comando si aún lo necesitas.",
  "error.sshAgentUnavailable":
    "No se pudo iniciar el agente SSH. Comprueba que no se esté ejecutando otro agente SSH y vuelve a activarlo.",
  "error.unlockDelayed":
    "Demasiados intentos incorrectos seguidos. Espera unos segundos y vuelve a intentarlo.",
  "error.tooManyAttempts":
    "Demasiadas respuestas incorrectas. Mosaic se bloqueó. Desbloquéalo con tu contraseña maestra para continuar.",
  "error.sshAllowTooSoon":
    "Fue demasiado rápido para leer la solicitud. Revisa la clave y el programa y elige Permitir de nuevo.",
  "error.browserRequestExpired":
    "Esa solicitud del navegador ya terminó. Vuelve a pedirla desde la extensión si aún la necesitas.",
  "error.browserAllowTooSoon":
    "Fue demasiado rápido para leer la solicitud. Revisa los detalles y elige de nuevo.",
  "error.browserPairingNotFound":
    "Ese navegador ya no está conectado. Vuelve a abrir Ajustes para ver la lista actual.",
  "error.browserHostMissing":
    "Esta copia de Mosaic no tiene instalado ningún conector de navegador, así que no hay nada que reparar. Instala Mosaic con su instalador para conectar un navegador.",
  "error.agentOriginInvalid":
    "Los agentes solo pueden enviar una clave a un sitio que empiece por https:// y nombre solo el sitio, como https://api.example.com. Revisa la dirección.",
  "error.agentBindingInvalid":
    "Mosaic no puede usar este ajuste para agentes. Revisa el campo, los sitios y cómo se envía la clave.",
  "error.agentValueUnusable":
    "Este valor es demasiado corto o tiene un salto de línea, así que los agentes no pueden usarlo. Una clave necesita al menos 12 caracteres en una sola línea.",
  "error.agentVaultShared":
    "Los agentes solo pueden usar elementos de una caja fuerte que te pertenezca y no esté compartida. Mueve primero el elemento a una de tus propias cajas fuertes.",
  "error.agentAccessOff":
    "El acceso de agentes está desactivado. Actívalo en Ajustes, en Agentes.",
  "error.agentRequestExpired":
    "Esa solicitud del agente ya terminó. El agente puede volver a pedirla si aún la necesita.",
  "error.agentAllowTooSoon":
    "Fue demasiado rápido para leer la solicitud. Revisa los detalles y elige de nuevo.",
  "error.agentPairingNotFound":
    "Ese agente ya no está conectado. Vuelve a abrir Ajustes para ver la lista actual.",
  "error.agentEndpointUnavailable":
    "Mosaic no pudo abrir la conexión que usan los agentes. Cierra otras copias de Mosaic y vuelve a intentarlo.",
  "error.kitNotVerified":
    "Guarda primero el nuevo Kit de emergencia y confirma su último grupo. Mosaic no cambia nada antes de eso.",
  "error.kitGroupMismatch":
    "Ese no es el último grupo de la nueva Clave secreta. Revisa el nuevo Kit de emergencia.",
  "error.rotationInProgress":
    "Otro cambio de claves sigue abierto. Termínalo con Continuar la rotación de claves o descártalo primero.",
  "error.rotationNone": "No hay ningún cambio de claves que continuar.",
  "error.rotationNotFound":
    "Ese cambio de contraseña ya no está abierto. Puede que se haya cerrado en otro dispositivo o en el navegador. Vuelve a abrir el elemento para ver cómo está.",
  "error.rotationLimit":
    "Ya hay cuatro cambios de contraseña abiertos de este elemento. Termina o cierra uno en el elemento primero.",
  "error.rotationOpen":
    "Este inicio de sesión tiene un cambio de contraseña abierto. Restáuralo desde la papelera y elige Funcionó o No funcionó en el elemento. Después podrás eliminarlo definitivamente.",
  "error.rotationNotSupported":
    "Mosaic no puede guiar este cambio. Necesita un inicio de sesión con una dirección https. Cambia tú mismo la contraseña en el sitio.",
  "error.historyNotFound":
    "Esa contraseña anterior ya no está en el historial. Vuelve a abrir el elemento para ver la lista actual.",
  "error.originNotRelated":
    "Esa página no pertenece al sitio de este inicio de sesión, así que Mosaic no la añadió. Edita las direcciones del elemento si debería.",
  "error.breachConsentRequired":
    "La comprobación de filtraciones está desactivada. Actívala y acepta primero lo que envía.",
  "error.breachRunning":
    "Ya hay una comprobación de filtraciones en curso. Espera a que termine o detenla primero.",
  "error.rotationTooFar":
    "Este cambio ya llegó a tu cuenta, así que no se puede descartar. Continúalo para terminarlo.",
  "error.rotationJumpTooLarge":
    "Este dispositivo está demasiado atrasado respecto a los demás para ponerse al día. Vuelve a configurarlo: quita la cuenta de este dispositivo y vuelve a unirte con tu Kit de emergencia.",
  "error.rotationNotApplied":
    "El cambio nunca llegó a tu cuenta, así que no se cambió nada. Empieza de nuevo o descártalo.",
  "error.secretKeyUnchanged":
    "Esa Clave secreta ya no funciona. Usa la del Kit de emergencia que guardaste en el dispositivo que hizo el cambio.",
  "error.passwordUnchanged":
    "La contraseña maestra nueva tiene que ser distinta de la actual.",
  "error.profileDowngrade":
    "Ese nivel no es más fuerte que el que usa tu cuenta ahora. Elige uno más alto.",
  "error.profileTooSlow":
    "Con ese nivel, desbloquear tardaría a este dispositivo más de dos segundos. Elige un nivel más bajo o mejora de todos modos.",
  "error.rekeyNeedsCleanSync":
    "Renovar todas las claves necesita que la sincronización esté al día y sin avisos. Sincroniza ahora, revisa los avisos que haya y vuelve a intentarlo.",
  "error.rekeyMemberKeyMismatch":
    "Aún no has comparado el código de seguridad de todas las personas de una caja fuerte compartida, así que esa caja fuerte no se puede renovar. Compara sus códigos en los ajustes de compartición de la caja fuerte y vuelve a intentarlo.",
  "error.rekeyBusy":
    "Tus cajas fuertes siguieron cambiando mientras Mosaic trabajaba. Espera un momento y vuelve a intentarlo.",
  "error.rekeyOutcomeUnknown":
    "Mosaic no pudo confirmar si tu cuenta aceptó las claves nuevas. Elige Continuar la rotación de claves para averiguarlo.",
  "error.rekeyNeedsServer":
    "Renovar todas las claves necesita tu servidor o carpeta de sincronización, y no se puede acceder a él. Comprueba la conexión y vuelve a intentarlo.",
  "error.rekeyNoAccountFile":
    "Nada de tu carpeta de sincronización se abre con esta contraseña y Clave secreta. Revisa el nuevo Kit de emergencia.",
  "error.syncAccountRekeyed":
    "Las claves de tu cuenta se renovaron en otro dispositivo. Usa la Clave secreta del nuevo Kit de emergencia para continuar.",
  "error.syncEpochRollback":
    "Tu sistema de sincronización muestra claves de cuenta más antiguas que las de este dispositivo. No se cambió nada.",
  "error.syncRekeyRefused":
    "Las claves que ofrece tu sistema de sincronización no son las siguientes a las de este dispositivo. No se cambió nada. Revisa tus otros dispositivos.",
  "error.pickerStale": "Esta lista está desactualizada. Haz clic en el campo de nuevo.",
  "error.pickerTooSoon": "Espera un momento y confirma de nuevo.",
  "error.programFillUnavailable": "Rellenar en programas no está disponible en este dispositivo.",
  "error.busy": "Mosaic aún está terminando el último paso. Espera un momento y vuelve a intentarlo.",
  "error.internal":
    "Algo inesperado detuvo esta acción. Vuelve a intentarlo y reinicia Mosaic si sigue ocurriendo.",
  "error.importTooLarge":
    "Este archivo es demasiado grande o complejo para importarlo. Exporta menos elementos a la vez y vuelve a intentarlo.",
  "error.importUnsupported":
    "Mosaic no puede leer esta versión del formato. Vuelve a exportar en un formato actual (para KeePass: KDBX 4 con AES o ChaCha20) y vuelve a intentarlo.",
  "error.importEncrypted":
    "Esta exportación está protegida con contraseña. Vuelve a exportar sin cifrado, impórtala y elimina el archivo.",
  "error.sshAgentEndpointInUse":
    "Ya se está ejecutando otro agente SSH (en Windows, el servicio OpenSSH Authentication Agent). Deténlo y vuelve a activar el agente SSH de Mosaic.",
  "error.cancelled": "Cancelado. No cambió nada.",
  "error.quickUnlockExpired":
    "El desbloqueo rápido caducó. Desbloquea con tu contraseña maestra y vuelve a activarlo en Ajustes.",
  "error.quickUnlockInvalidated":
    "El desbloqueo rápido dejó de funcionar en este dispositivo y se desactivó. Desbloquea con tu contraseña maestra.",
  "error.passwordRequired":
    "Escribe tu contraseña maestra esta vez. Después, el desbloqueo rápido vuelve a funcionar.",
  "error.hardwareKeyUnsupported":
    "Esta llave de seguridad o este sistema no se puede usar con Mosaic.",
  "error.hardwareKeyNotFound": "No se encontró ninguna llave de seguridad. Insértala y vuelve a intentarlo.",
  "error.hardwareKeyTimeout": "No se tocó la llave a tiempo. Vuelve a intentarlo.",
  "error.hardwareKeyPinRequired": "Escribe el PIN de tu llave de seguridad.",
  "error.hardwareKeyPinInvalid":
    "PIN incorrecto. Tras varios intentos fallidos, la llave se bloquea sola.",
  "error.hardwareKeyPinBlocked":
    "Esta llave está bloqueada por demasiados PIN incorrectos. Restablécela con la herramienta de su fabricante; sus registros se perderán.",
  "error.hardwareKeyNoPin":
    "Primero establece un PIN en esta llave con la herramienta de su fabricante.",
  "error.hardwareKeyNotRecognized":
    "Esta llave no está registrada aquí. Prueba con otra llave o con tu contraseña maestra.",
  "error.hardwareKeyLimit": "Registraste el máximo de llaves permitido (8). Quita una primero.",
  "error.hardwareKeyLast":
    "Esta es la única llave de la caja fuerte. Registra otra antes de quitarla.",
  "error.itemTypeNotAllowed": "Esta caja fuerte solo guarda códigos de un solo uso.",
  "error.otpScanUnsupported":
    "El escaneo de pantalla no está disponible en este sistema. Pega una captura de pantalla.",
  "error.otpQrNotFound":
    "No se encontró ningún código QR. Asegúrate de que todo el código sea visible y vuelve a intentarlo.",
  "error.otpQrNotOtp":
    "Este código QR no es un código de configuración de verificación en dos pasos. Busca el código en la página de configuración de 2FA del sitio.",
  "error.otpQrMigration":
    "Este es un código de exportación de Google Authenticator, que Mosaic aún no puede leer. Vuelve a configurar cada cuenta en su sitio o escribe su clave de configuración.",
  "error.otpQrMultiple":
    "Hay más de un código de configuración visible. Muestra solo el que quieras y vuelve a intentarlo.",
  "error.otpQrInvalid":
    "Este código de configuración usa ajustes que Mosaic no admite. Escribe la clave de configuración.",
  "error.otpUriInvalid": "Este no es un enlace otpauth:// válido. Vuelve a copiar el enlace completo.",
  "error.otpKeyInvalid":
    "Esta clave no es válida. Solo usa las letras de la A a la Z y los dígitos del 2 al 7; comprueba que no haya erratas.",
  "error.otpImageUnreadable": "Mosaic no puede leer esta imagen. Usa un archivo PNG o JPEG.",
  "error.otpImageTooLarge":
    "Esta imagen es demasiado grande. Usa una captura de pantalla solo del código QR.",
  "error.otpClipboardNoImage":
    "El portapapeles no tiene ninguna imagen. Haz una captura de pantalla del código QR y pégala aquí.",
  "error.otpPendingExpired": "El código escaneado ya no está disponible. Escanéalo de nuevo.",
  "error.syncNotConfigured":
    "La sincronización aún no está configurada en este dispositivo. Configúrala primero en Ajustes.",
  "error.syncAlreadyConfigured":
    "La sincronización ya está configurada. Desconecta primero para usar otro servidor u otra carpeta.",
  "error.syncServerUnreachable":
    "Mosaic no pudo conectar con el servidor. Comprueba la dirección y tu conexión a internet y vuelve a intentarlo.",
  "error.syncServerInvalid":
    "Esto no parece un servidor de Mosaic. Comprueba la dirección; debería ser como https://sync.example.com.",
  "error.syncServerInsecure":
    "Mosaic solo se conecta a servidores mediante https, para que nadie por el camino pueda leer ni cambiar tus datos. Usa una dirección https://.",
  "error.syncServerMismatch":
    "Este no es el servidor con el que se sincroniza esta cuenta. Comprueba la dirección.",
  "error.syncServerIncompatible":
    "Este servidor ejecuta una versión que no coincide con tu Mosaic. Actualiza Mosaic o pide al propietario del servidor que lo actualice.",
  "error.syncRegistrationClosed":
    "Este servidor no acepta cuentas de sincronización nuevas. Inicia sesión en una existente o consulta al propietario del servidor.",
  "error.syncInviteInvalid":
    "Este código de invitación no es válido o ya se usó. Compruébalo o pide uno nuevo al propietario del servidor.",
  "error.syncAccountExists":
    "Ya tienes una cuenta de sincronización en este servidor. Elige Iniciar sesión.",
  "error.syncAccountMismatch":
    "La cuenta de sincronización de ese servidor pertenece a otra cuenta de Mosaic. Comprueba la dirección y tu correo.",
  "error.syncTwoFactorRequired":
    "Esta cuenta de sincronización necesita un código de tu app de autenticación. Escribe el código de 6 dígitos.",
  "error.syncTwoFactorInvalid":
    "Ese código no es correcto o ha caducado. Escribe el código actual de 6 dígitos de tu app de autenticación.",
  "error.syncSignedOut":
    "El servidor cerró la sesión de este dispositivo. Inicia sesión de nuevo para seguir sincronizando. Tus cambios están a salvo en este dispositivo.",
  "error.syncRateLimited": "Demasiados intentos. Espera unos minutos y vuelve a intentarlo.",
  "error.syncQuotaExceeded":
    "El servidor o la carpeta no tiene espacio para tus datos. Libera espacio o pide más al propietario del servidor.",
  "error.syncFolderUnavailable":
    "Mosaic no puede usar esta carpeta. Comprueba que exista, que tu servicio en la nube haya terminado de sincronizarla y que Mosaic pueda escribir en ella.",
  "error.syncFolderNotEmpty":
    "Esta carpeta contiene otros archivos, así que Mosaic no escribirá allí. Elige una carpeta vacía o crea una nueva.",
  "error.syncFolderNoAccount":
    "Esta carpeta no contiene datos de sincronización de esta cuenta. Elige la carpeta con la que se sincroniza tu otro dispositivo.",
  "error.syncFolderDamaged":
    "Algunos archivos de sincronización de esta carpeta están dañados. Deja que tu servicio en la nube termine de sincronizar o restaura la carpeta desde una copia de seguridad y vuelve a intentarlo.",
  "error.syncPasswordOutcomeUnknown":
    "No pudimos confirmar si el servidor cambió tu contraseña maestra. Conserva ambas contraseñas hasta que la sincronización confirme cuál se aplica.",
  "error.syncWebauthnUnavailable":
    "Tu servidor no puede usar una llave de seguridad para esto ahora, o esta cuenta no tiene ninguna. Usa un código de una app de autenticación.",
  "error.syncWebauthnLimit":
    "Esta cuenta ya tiene el máximo de llaves de seguridad que permite el servidor. Quita una que ya no uses y vuelve a intentarlo.",
  "error.syncHandoffExpired": "El paso en el navegador agotó su tiempo. Vuelve a empezar.",
  "error.syncHandoffFailed":
    "Demasiados códigos incorrectos o comprobaciones fallidas en el navegador. Vuelve a empezar el paso.",
  "error.syncPasswordNeedsServer":
    "Cambiar tu contraseña maestra necesita tu servidor de sincronización, y no se puede acceder a él. Comprueba tu conexión a internet y vuelve a intentarlo.",
  "error.syncDeviceIsCurrent":
    "No puedes quitar el dispositivo que estás usando. Usa Desconectar.",
  "error.syncChoiceExpired": "La elección de carpeta caducó. Elige la carpeta de nuevo.",
  "error.syncFreshLoginExpired":
    "La comprobación de tu contraseña caducó. Escribe de nuevo tu contraseña maestra.",
  "error.syncPasswordRollback":
    "El servidor ofreció una contraseña maestra más antigua. Revisa el servidor.",
  "error.syncPasswordChanged":
    "Tu contraseña maestra se cambió en el servidor o en la carpeta. Escribe la nueva abajo.",
  "error.sharingNotHosted":
    "Compartir necesita un servidor de Mosaic. Conecta uno primero en los ajustes de sincronización.",
  "error.setupCodeNeedsServer": "Un teléfono solo puede unirse a una cuenta que se sincroniza.",
  "error.sharingContactUnverified":
    "Compara primero el código de seguridad de esta persona. Mosaic solo comparte con personas que hayas verificado.",
  "error.sharingKeyChanged":
    "La clave de esta persona cambió o el código no coincide. Búscala de nuevo y compara el código nuevo antes de continuar.",
  "error.sharingRoleForbidden":
    "Tu rol en esta caja fuerte no permite eso. Pregunta al propietario o a un administrador.",
  "error.sharingInviteExists":
    "Ya hay una invitación en espera para esta persona y esta caja fuerte. Retírala primero si quieres enviar una nueva.",
  "error.sharingInviteStale":
    "La invitación cambió entretanto, por ejemplo tras un cambio de clave. Ábrela de nuevo.",
  "error.sharingInviteExpired":
    "Esta invitación caducó o se retiró. Pide al remitente que te invite de nuevo.",
  "error.sharingInviteUnreadable":
    "No se puede abrir esta invitación. Puede estar dañada o no ser para esta cuenta. Pide al remitente que te invite de nuevo.",
  "error.sharingInviteNotInRoster":
    "La lista de miembros de la caja fuerte no coincide con esta invitación, así que Mosaic no la abrió. Pide al remitente que te invite de nuevo.",
  "error.sharingOwnerCannotLeave":
    "El propietario no puede salir de una caja fuerte. Quita a las demás personas en su lugar.",
  "error.sharingAlreadyMember": "Esta persona ya es miembro de la caja fuerte.",
  "error.sharingReinviteBlocked":
    "Quitaste a esta persona hace poco. Podrás invitarla de nuevo cuando la clave de la caja fuerte haya cambiado. Sincroniza y vuelve a intentarlo.",
  "error.sharingInviteLimit":
    "Hay demasiadas invitaciones en espera. Espera las respuestas o retira algunas.",
  "error.sharingVaultKnown":
    "Este dispositivo ya tiene esa caja fuerte, así que Mosaic no aceptó una clave para ella. Pregunta al remitente qué caja fuerte compartió.",
  "error.syncHasSharedVaults":
    "No puedes cambiar de servidor mientras haya cajas fuertes compartidas en este dispositivo. Sal primero de las cajas fuertes compartidas o pide a sus propietarios que te quiten.",
  "error.emergencyWaiting":
    "Hay una solicitud en espera, así que esto no se puede cambiar ahora. Recházala o espera.",
  "error.emergencyCooldown":
    "Tu última solicitud fue rechazada. Podrás volver a pedir acceso tras el periodo de espera de un día.",
  "error.emergencyNotGranted":
    "No hay nada que reclamar: no hay ninguna solicitud abierta o la persona no te concedió acceso.",
  "error.shortcutUnavailable":
    "Otro programa ya usa este atajo o Mosaic no puede registrarlo. Elige otra combinación.",
  "error.shortcutUnsupported":
    "Tu sistema no permite que las apps definan un atajo global. Abre Mosaic desde su ventana.",
} satisfies Record<keyof typeof en, string>;
