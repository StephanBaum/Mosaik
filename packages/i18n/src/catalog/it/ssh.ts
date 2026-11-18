/**
 * Italian UI strings: ssh. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { ssh as en } from "../en/ssh.ts";

export const ssh = {
  "ssh.title": "Un'app vuole usare la tua chiave SSH",
  "ssh.key": "Chiave",
  "ssh.fingerprint": "Impronta",
  "ssh.client": "Richiesto da",
  "ssh.client.unknown": "Un programma sconosciuto",
  "ssh.expires":
    "Rifiutata automaticamente tra {seconds, plural, one {# secondo} other {# secondi}}",
  "ssh.deny": "Nega",
  "ssh.allowOnce": "Consenti una volta",
  "ssh.allowFor15Minutes": "Consenti per 15 minuti",
  "ssh.queue":
    "{count, plural, one {# altra richiesta in attesa} other {# altre richieste in attesa}}",
  "ssh.signFailed.passphraseRequired":
    "La chiave è cifrata, ma l'elemento non ha una frase d'accesso. Aggiungila nel campo Password dell'elemento.",
  "ssh.signFailed.wrongPassphrase":
    "La frase d'accesso nell'elemento non sblocca la chiave. Correggila nel campo Password dell'elemento.",
  "ssh.signFailed.keyMalformed":
    "La chiave privata in questo elemento non può essere letta. Incollala di nuovo in formato OpenSSH.",
  "ssh.signFailed.kdfTooExpensive":
    "La protezione con frase d'accesso della chiave usa più cicli di quanti ne consenta Mosaic. Cifrala di nuovo con \"ssh-keygen -p -a 64\".",
  "ssh.signFailed.publicKeyMismatch":
    "La chiave privata non corrisponde alla chiave pubblica dell'elemento. Controlla entrambi i campi.",
  "ssh.signFailed.unsupported":
    "Mosaic firma solo con chiavi Ed25519 ed ECDSA P-256.",
  "ssh.signFailed.internal": "La firma non è riuscita. Riprova.",
  "ssh.skip.unsupported": "Non offerta: Mosaic firma solo con chiavi Ed25519 ed ECDSA P-256.",
  "ssh.skip.malformed": "Non offerta: la chiave non può essere letta.",
  "ssh.skip.duplicate": "Non offerta: un altro elemento ha la stessa chiave.",
  "ssh.description": "Un'app ha chiesto all'agente SSH di Mosaic di accedere con questa chiave.",
  "ssh.origin.forwarded": "Un server a cui sei collegato chiede di usare questa chiave.",
  "ssh.origin.bindFailed":
    "Mosaic non riesce a capire da dove arriva questa richiesta: il controllo dell'host della connessione non è riuscito. Consentila solo se te l'aspetti.",
  "ssh.escDenies": "per negare",
} satisfies Record<keyof typeof en, string>;
