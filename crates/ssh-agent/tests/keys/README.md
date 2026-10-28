# Test keys

Throwaway SSH keys for the `mosaic-ssh-agent` tests, generated with
`ssh-keygen` (OpenSSH 8.8) for this purpose only. They protect nothing and
are published on purpose. The encrypted ones use the passphrase
`correct horse battery staple`.

| File | Type | Encryption |
|---|---|---|
| `ed25519` | Ed25519 | none |
| `ed25519_encrypted` | Ed25519 | aes256-ctr, bcrypt-pbkdf 16 rounds |
| `ed25519_encrypted_chacha` | Ed25519 | chacha20-poly1305@openssh.com |
| `ed25519_encrypted_gcm` | Ed25519 | aes256-gcm@openssh.com |
| `ecdsa_p256` | ECDSA P-256 | none |
| `ecdsa_p256_encrypted` | ECDSA P-256 | aes256-ctr |
| `ecdsa_p384` | ECDSA P-384 (not served) | none |
| `rsa` | RSA 2048 (not served) | none |

Fingerprints (`ssh-keygen -lf`) are checked in the tests.
