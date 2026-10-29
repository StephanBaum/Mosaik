/** German UI strings: ssh. Informal "du". Same keys as English (checked by the type). */
import type { ssh as en } from "../en/ssh.ts";

export const ssh = {
  "ssh.title": "Eine App möchte deinen SSH-Schlüssel verwenden",
  "ssh.key": "Schlüssel",
  "ssh.fingerprint": "Fingerabdruck",
  "ssh.client": "Angefragt von",
  "ssh.client.unknown": "Ein unbekanntes Programm",
  "ssh.expires":
    "Wird automatisch abgelehnt in {seconds, plural, one {# Sekunde} other {# Sekunden}}",
  "ssh.deny": "Ablehnen",
  "ssh.allowOnce": "Einmal erlauben",
  "ssh.allowFor15Minutes": "15 Minuten erlauben",
  "ssh.queue": "{count, plural, one {# weitere Anfrage wartet} other {# weitere Anfragen warten}}",
  "ssh.signFailed.passphraseRequired":
    "Der Schlüssel ist verschlüsselt, aber der Eintrag hat keine Passphrase. Trage sie im Feld „Passwort“ des Eintrags ein.",
  "ssh.signFailed.wrongPassphrase":
    "Die Passphrase im Eintrag entsperrt den Schlüssel nicht. Korrigiere sie im Feld „Passwort“ des Eintrags.",
  "ssh.signFailed.keyMalformed":
    "Der private Schlüssel in diesem Eintrag ist nicht lesbar. Füge ihn noch einmal im OpenSSH-Format ein.",
  "ssh.signFailed.kdfTooExpensive":
    "Der Passphrase-Schutz des Schlüssels nutzt mehr Runden, als Mosaic erlaubt. Verschlüssle ihn neu mit „ssh-keygen -p -a 64“.",
  "ssh.signFailed.publicKeyMismatch":
    "Der private Schlüssel passt nicht zum öffentlichen Schlüssel des Eintrags. Prüfe beide Felder.",
  "ssh.signFailed.unsupported": "Mosaic signiert nur mit Ed25519- und ECDSA-P-256-Schlüsseln.",
  "ssh.signFailed.internal": "Das Signieren ist fehlgeschlagen. Versuche es noch einmal.",
  "ssh.skip.unsupported":
    "Wird nicht angeboten: Mosaic signiert nur mit Ed25519- und ECDSA-P-256-Schlüsseln.",
  "ssh.skip.malformed": "Wird nicht angeboten: Der Schlüssel ist nicht lesbar.",
  "ssh.skip.duplicate": "Wird nicht angeboten: Ein anderer Eintrag hat denselben Schlüssel.",
} satisfies Record<keyof typeof en, string>;
