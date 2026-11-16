/**
 * Spanish UI strings: ssh. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { ssh as en } from "../en/ssh.ts";

export const ssh = {
  "ssh.title": "Una app quiere usar tu clave SSH",
  "ssh.key": "Clave",
  "ssh.fingerprint": "Huella digital",
  "ssh.client": "Solicitado por",
  "ssh.client.unknown": "Un programa desconocido",
  "ssh.expires":
    "Se rechaza automáticamente en {seconds, plural, one {# segundo} other {# segundos}}",
  "ssh.deny": "Denegar",
  "ssh.allowOnce": "Permitir una vez",
  "ssh.allowFor15Minutes": "Permitir durante 15 minutos",
  "ssh.queue":
    "{count, plural, one {# solicitud más en espera} other {# solicitudes más en espera}}",
  "ssh.signFailed.passphraseRequired":
    "La clave está cifrada, pero el elemento no tiene frase de contraseña. Añádela en el campo Contraseña del elemento.",
  "ssh.signFailed.wrongPassphrase":
    "La frase de contraseña del elemento no desbloquea la clave. Corrígela en el campo Contraseña del elemento.",
  "ssh.signFailed.keyMalformed":
    "No se puede leer la clave privada de este elemento. Pégala de nuevo en formato OpenSSH.",
  "ssh.signFailed.kdfTooExpensive":
    "La protección con frase de contraseña de la clave usa más rondas de las que Mosaic permite. Vuelve a cifrarla con \"ssh-keygen -p -a 64\".",
  "ssh.signFailed.publicKeyMismatch":
    "La clave privada no coincide con la clave pública del elemento. Revisa ambos campos.",
  "ssh.signFailed.unsupported": "Mosaic solo firma con claves Ed25519 y ECDSA P-256.",
  "ssh.signFailed.internal": "No se pudo firmar. Inténtalo de nuevo.",
  "ssh.skip.unsupported": "No se ofrece: Mosaic solo firma con claves Ed25519 y ECDSA P-256.",
  "ssh.skip.malformed": "No se ofrece: no se puede leer la clave.",
  "ssh.skip.duplicate": "No se ofrece: otro elemento tiene la misma clave.",
  "ssh.description": "Una app pidió al agente SSH de Mosaic iniciar sesión con esta clave.",
  "ssh.origin.forwarded": "Un servidor al que estás conectado pide usar esta clave.",
  "ssh.origin.bindFailed":
    "Mosaic no puede saber de dónde viene esta solicitud: falló la comprobación del host de la conexión. Permítela solo si la esperas.",
  "ssh.escDenies": "para denegar",
} satisfies Record<keyof typeof en, string>;
