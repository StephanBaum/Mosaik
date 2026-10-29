/** English UI strings: ssh. ICU MessageFormat. */
export const ssh = {
  "ssh.title": "An app wants to use your SSH key",
  "ssh.key": "Key",
  "ssh.fingerprint": "Fingerprint",
  "ssh.client": "Requested by",
  "ssh.client.unknown": "An unknown program",
  "ssh.expires": "Refused automatically in {seconds, plural, one {# second} other {# seconds}}",
  "ssh.deny": "Deny",
  "ssh.allowOnce": "Allow once",
  "ssh.allowFor15Minutes": "Allow for 15 minutes",
  "ssh.queue": "{count, plural, one {# more request waiting} other {# more requests waiting}}",
  "ssh.signFailed.passphraseRequired":
    "The key is encrypted, but the item has no passphrase. Add it in the item's Password field.",
  "ssh.signFailed.wrongPassphrase":
    "The passphrase in the item doesn't unlock the key. Correct it in the item's Password field.",
  "ssh.signFailed.keyMalformed":
    "The private key in this item can't be read. Paste it again in OpenSSH format.",
  "ssh.signFailed.kdfTooExpensive":
    "The key's passphrase protection uses more rounds than Mosaic allows. Re-encrypt it with \"ssh-keygen -p -a 64\".",
  "ssh.signFailed.publicKeyMismatch":
    "The private key doesn't match the item's public key. Check both fields.",
  "ssh.signFailed.unsupported": "Mosaic signs only with Ed25519 and ECDSA P-256 keys.",
  "ssh.signFailed.internal": "Signing failed. Try again.",
  "ssh.skip.unsupported": "Not served: Mosaic signs only with Ed25519 and ECDSA P-256 keys.",
  "ssh.skip.malformed": "Not served: the key can't be read.",
  "ssh.skip.duplicate": "Not served: another item has the same key.",
} as const satisfies Record<string, string>;
