/**
 * Spanish UI strings: cli. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { cli as en } from "../en/cli.ts";

export const cli = {
  "cli.err.storeInUse":
    "La app de Mosaic tiene esta caja fuerte abierta. Ciérrala o dale a la CLI su propio dispositivo con `mosaic setup own-store`.",
  "cli.err.noStore":
    "No se encontró ninguna caja fuerte en {path}. Configura primero Mosaic en la app o usa --store.",
  "cli.err.noAccount": "Este almacén aún no tiene ninguna cuenta.",
  "cli.err.wrongCredentials": "Contraseña maestra o Clave secreta incorrecta.",
  "cli.err.secretKeyInvalid":
    "Esa Clave secreta no es válida. Compárala con tu Kit de emergencia.",
  "cli.err.inputRequired": "Hace falta una entrada, pero --no-input está activado.",
  "cli.err.promptTimeout": "No se recibió ninguna entrada en {seconds} segundos.",
  "cli.err.notFound": "No se encontró nada para {ref}.",
  "cli.err.ambiguousRef":
    "Más de un elemento coincide con {ref}. Usa una referencia más exacta o el ID del elemento.",
  "cli.err.invalidRef": "{ref} no es una referencia válida.",
  "cli.err.noSuchField": "Este elemento no tiene el campo {field}.",
  "cli.err.noOtp": "Este elemento no tiene contraseña de un solo uso.",
  "cli.err.revealRequired":
    "{field} es secreto. Añade --reveal para mostrarlo o --copy para ponerlo en el portapapeles.",
  "cli.err.vaultUnavailable":
    "Esa caja fuerte está protegida por hardware o no está disponible aquí.",
  "cli.err.rotationInProgress": "Termina primero el cambio de clave en la app de Mosaic.",
  "cli.err.secretInArgv":
    "{flag} pondría un secreto en la línea de comandos. Usa el aviso interactivo, --password-stdin o --password-fd.",
  "cli.err.unknownCommand": "Comando desconocido {name}. Ejecuta `mosaic help`.",
  "cli.err.unknownOption": "Opción desconocida {name}.",
  "cli.err.missingValue": "La opción {name} necesita un valor.",
  "cli.err.missingArgument": "Falta el argumento {name}.",
  "cli.err.unexpectedArgument": "Argumento inesperado {value}.",
  "cli.err.invalidValue": "Valor no válido para {name}.",
  "cli.err.nonUtf8": "Los argumentos deben ser UTF-8 válido.",
  "cli.err.fdUnsupported": "--password-fd no está disponible en este sistema.",
  "cli.err.sessionExpired": "La sesión de desbloqueo terminó. Escribe tu contraseña maestra.",
  "cli.err.sessionNotCached":
    "No se pudo guardar ninguna sesión de desbloqueo (el almacén de claves del sistema o la carpeta de ejecución no está disponible). Cada comando pedirá tu contraseña maestra.",
  "cli.err.twoFactorMethodUnsupported":
    "Esta cuenta inicia sesión con una llave de seguridad, que la línea de comandos no puede usar. Añade un código de una app de autenticación en la app de escritorio e inténtalo de nuevo.",
  "cli.err.envSecretIgnored":
    "{name} está definida pero se ignora. Mosaic nunca lee secretos del entorno.",
  "cli.err.exportUnreadable":
    "No se puede leer el archivo: la frase de contraseña es incorrecta o el archivo está dañado.",
  "cli.err.importFailed": "No se pudo importar el archivo.",
  "cli.err.importTooLarge": "El archivo es demasiado grande para importarlo.",
  "cli.err.syncNotConfigured": "La sincronización no está configurada para este almacén.",
  "cli.err.syncHalted": "La sincronización está detenida. Ejecuta `mosaic sync --status`.",
  "cli.err.syncFailed": "Falló la sincronización: {reason}.",
  "cli.err.syncSessionRevoked":
    "Las claves de esta cuenta se renovaron en otro dispositivo, por lo que esta sesión terminó. Inicia sesión de nuevo con la nueva Clave secreta de tu Kit de emergencia más reciente.",
  "cli.err.syncSessionPasswordChanged":
    "Tu contraseña maestra se cambió en otro dispositivo, por lo que esta sesión terminó. Inicia sesión de nuevo con la nueva contraseña.",
  "cli.err.syncSessionDeviceRemoved":
    "Este dispositivo se quitó de tu cuenta desde otro dispositivo, por lo que esta sesión terminó. Inicia sesión de nuevo para volver a añadirlo.",
  "cli.err.syncSessionOperator":
    "El operador del servidor terminó esta sesión. Inicia sesión de nuevo para seguir sincronizando.",
  "cli.err.fileExists": "{path} ya existe. Mosaic no sobrescribe archivos.",
  "cli.err.fileRead": "No se puede leer {path}.",
  "cli.err.fileWrite": "No se puede escribir en {path}.",
  "cli.err.passphraseMismatch": "Las dos frases de contraseña no coinciden.",
  "cli.err.passphraseWeak": "Usa una frase de contraseña de al menos {min} caracteres.",
  "cli.err.badEnvName": "{name} no es un nombre de variable de entorno válido.",
  "cli.err.spawnFailed": "No se puede iniciar {command}.",
  "cli.err.noVaultChosen": "Hay varias cajas fuertes. Elige una con --vault.",
  "cli.err.notSupported": "{what} no está disponible en esta versión de la CLI.",
  "cli.err.storeExists": "{path} ya existe.",
  "cli.err.internal": "Algo salió mal.",
  "cli.prompt.password": "Contraseña maestra: ",
  "cli.prompt.secretKey": "Clave secreta (de tu Kit de emergencia): ",
  "cli.prompt.filePassword": "Contraseña del archivo: ",
  "cli.prompt.exportPassphrase": "Frase de contraseña del archivo de exportación: ",
  "cli.prompt.exportPassphraseAgain": "Repite la frase de contraseña: ",
  "cli.prompt.importPassphrase": "Frase de contraseña del archivo de exportación: ",
  "cli.prompt.confirmImport": "¿Importar {count} elementos en {vault}? [y/N] ",
  "cli.prompt.newPassword": "Nueva contraseña maestra (la que estableciste en otro lugar): ",
  "cli.prompt.twoFactor": "Código de verificación en dos pasos: ",
  "cli.out.recovered": "Recuperado. Este dispositivo ahora usa tu nueva contraseña maestra.",
  "cli.prompt.acknowledge": "Escribe la palabra {word} para continuar: ",
  "cli.warn.revealToTerminal":
    "Los valores secretos se imprimen en el terminal y quedan en su historial de desplazamiento.",
  "cli.out.unlocked":
    "Desbloqueado durante {minutes} minutos. Ejecuta `mosaic lock` cuando termines.",
  "cli.out.locked": "Bloqueado. Sesión eliminada.",
  "cli.out.concealed": "••••••••",
  "cli.out.copied": "Copiado. Se borra en {seconds} segundos.",
  "cli.out.noItems": "No hay elementos.",
  "cli.out.ambiguousCandidate": "{vault} / {title} ({id})",
  "cli.out.attachment": "{name} ({size} bytes, {mime})",
  "cli.label.id": "ID",
  "cli.label.vault": "Caja fuerte",
  "cli.label.type": "Tipo",
  "cli.label.title": "Título",
  "cli.label.otp": "Código de un solo uso",
  "cli.label.attachments": "Adjuntos",
  "cli.status.store": "Almacén",
  "cli.status.profile": "Perfil",
  "cli.status.account": "Cuenta",
  "cli.status.session": "Sesión",
  "cli.status.sessionNone": "bloqueado (sin sesión)",
  "cli.status.sessionUntil": "desbloqueado hasta {time} (ms Unix)",
  "cli.status.sync": "Sincronización",
  "cli.status.syncOff": "no configurada",
  "cli.status.syncOn": "{backend}, detenida: {halted}, hallazgos: {findings}",
  "cli.import.preview": "{count} elementos para importar en {vault}: {summary}",
  "cli.import.duplicates": "{count} parecen duplicados y se omiten.",
  "cli.import.warnings": "{count} entradas no se pudieron leer y se omiten.",
  "cli.import.done": "Se importaron {imported} elementos ({skipped} omitidos, {failed} con error).",
  "cli.import.dryRun": "Simulación: no se importó nada.",
  "cli.import.cancelled": "Cancelado. No se importó nada.",
  "cli.export.done": "Se exportaron {items} elementos de {vaults} cajas fuertes a {path}.",
  "cli.export.attachmentWarn": "{count} elementos tienen adjuntos. Están incluidos.",
  "cli.export.attachmentsLeftOut":
    "{count} archivos adjuntos no están en la exportación. Usa --include-attachments.",
  "cli.export.unavailableVaults": "Se omitieron {count} cajas fuertes protegidas por hardware.",
  "cli.sync.done": "Sincronización terminada: {pulled} recibidos, {pushed} enviados.",
  "cli.sync.finding": "Hallazgo: {kind}.",
  "cli.sync.clean": "La sincronización está bien. Sin hallazgos.",
  "cli.sync.acknowledged": "Hallazgos borrados.",
  "cli.sync.acknowledgeWord": "acknowledge",
  "cli.setup.done":
    "Este dispositivo está configurado como su propio almacén. Ejecuta `mosaic sync`.",
  "cli.ssh.agentStatus": "El agente SSH se ejecuta en la app de Mosaic. La CLI no ejecuta uno.",
  "cli.ssh.noKeys": "No hay claves SSH.",
  "cli.err.agentContext":
    "Esto parece una sesión de un agente de IA. Mostrar un secreto aquí lo pondría en el contexto del agente. Usa `mosaic agent mcp` o añade --i-know si lo ejecutas tú mismo.",
  "cli.err.agentAlreadyPaired":
    "Ya existe un emparejamiento llamado {name}. Añade --replace para emparejar de nuevo.",
  "cli.err.desktopNotRunning": "La app de Mosaic no se está ejecutando.",
  "cli.err.agentAccessOff":
    "El acceso de agentes está desactivado. Actívalo en la app de Mosaic, en Ajustes.",
  "cli.err.agentLocked": "La caja fuerte está bloqueada. Desbloquea primero la app de Mosaic.",
  "cli.err.pairingDenied": "El emparejamiento se denegó en la app de Mosaic.",
  "cli.err.pairingExpired": "La solicitud de emparejamiento agotó su tiempo.",
  "cli.err.pairingLimit":
    "La app de Mosaic ya tiene el máximo de emparejamientos que permite. Quita uno primero.",
  "cli.err.pairingNameTaken":
    "Ya existe un emparejamiento con este nombre; quítalo primero o elige otro nombre.",
  "cli.err.pairingBlocked":
    "El emparejamiento está bloqueado durante unos minutos tras varias denegaciones.",
  "cli.err.notPaired": "Este cliente no está emparejado. Ejecuta `mosaic agent pair --name {name}`.",
  "cli.err.agentRefused": "La app de Mosaic rechazó la solicitud ({reason}).",
  "cli.err.agentIo": "Falló la conexión con la app de Mosaic.",
  "cli.agent.pairCode": "Compara este código con el que muestra Mosaic: {code}",
  "cli.agent.paired": "Emparejado como {name}.",
  "cli.agent.status":
    "Escritorio: {desktop}. Caja fuerte: {vault}. Acceso de agentes: {access}. Emparejados: {paired}.",
} satisfies Record<keyof typeof en, string>;
