# Mosaic Cryptography Specification

Status: **normative, v1 draft**. Code in `crates/core` (and anything that
touches keys) must match this document. If they disagree, the code is wrong
until this document is changed first, with an ADR in `docs/DECISIONS.md`.

Key words MUST, MUST NOT, SHOULD and MAY are used as in RFC 2119.

Mosaic is pre-audit. Nothing here has been externally reviewed yet.

---

## 1. Design goals

1. **Zero knowledge.** The sync server stores only ciphertext and the minimum
   metadata listed in §12. Stealing the server database MUST NOT enable an
   offline guessing attack on the master password.
2. **Two-secret derivation.** Unlocking needs the master password *and* the
   128-bit Secret Key. Only the device holds the Secret Key.
3. **Cheap re-keying.** A master password change re-wraps one key. It never
   re-encrypts items.
4. **No invented crypto.** Every primitive and construction is a published
   standard implemented by a maintained library (§2). Mosaic only composes
   them, using domain separation and explicit associated data.
5. **Crypto agility through versioning.** Every serialized object carries a
   format version and an algorithm ID. Decoders reject unknown values and MUST
   NOT silently downgrade.

## 2. Primitives and libraries

| Purpose | Algorithm | Standard | Rust crate |
|---|---|---|---|
| Password hashing / KSF | Argon2id v1.3 | RFC 9106 | `argon2` (RustCrypto) |
| Key derivation | HKDF-SHA-256 | RFC 5869 | `hkdf`, `sha2` |
| AEAD | XChaCha20-Poly1305 | draft-irtf-cfrg-xchacha, RFC 8439 | `chacha20poly1305` |
| Streaming AEAD (attachments) | STREAM (big-endian 32-bit counter) over XChaCha20-Poly1305 | Hoang, Reyhanitabar, Rogaway, Vizár 2015 | `chacha20poly1305` / `aead::stream` |
| Public-key encryption (sharing, emergency access) | HPKE, mode Auth, DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, ChaCha20-Poly1305 | RFC 9180 | `hpke` |
| Account login | OPAQUE-3DH, ristretto255, SHA-512 | RFC 9807 | `opaque-ke` |
| Passkeys (§16, ADR-0039) | ECDSA P-256 / SHA-256 (ES256) with RFC 6979 nonces; Ed25519 (EdDSA) reserved, not implemented | FIPS 186-5, RFC 6979, RFC 8032, WebAuthn L3 | `p256` (features `alloc`, `ecdsa`, `pkcs8`; encodings from `pkcs8`, `spki`, `der`); `sha2` (rpIdHash, clientDataJSON hash); `base64ct` (base64url) |
| OTP | HMAC-SHA-1/256/512 | RFC 2104, 4226, 6238 | `hmac`, `sha1`, `sha2` |
| Settings integrity (§11.1, ADR-0030) | HMAC-SHA-256 under an HKDF-derived key | RFC 2104, 5869 | `hmac`, `hkdf`, `sha2` |
| Browser extension channel (§15, ADR-0036) | HKDF-SHA-256, HMAC-SHA-256, XChaCha20-Poly1305 | RFC 5869, RFC 2104, draft-irtf-cfrg-xchacha | `hkdf`, `hmac`, `sha2`, `chacha20poly1305` |
| Breach check | SHA-1 (k-anonymity prefix only) | HIBP range API | `sha1` |
| Randomness | OS CSPRNG | — | `getrandom`, called directly in `crates/core` and `crates/generator` (no `rand` / `rand_core`, ADR-0011) |
| Constant-time compare | — | — | `subtle` |
| Secret memory | zero on drop | — | `zeroize`, in Mosaic's own containers (`crates/core` `secret`; `secrecy` is not used, ADR-0007) |
| Unicode normalization | NFC | UAX #15 | `unicode-normalization` |
| TLS | TLS 1.3 only | RFC 8446 | `rustls` |

Banned: OpenSSL bindings, `ring`-only APIs outside `rustls`, anything
unmaintained (enforced in `deny.toml`). Adding a crypto dependency needs an
ADR.

Interoperability only (these read or serve third-party formats and never protect Mosaic data):

| Purpose | Algorithm | Standard | Rust crate |
|---|---|---|---|
| SSH agent (ADR-0025) | Ed25519; ECDSA P-256/SHA-256 with RFC 6979 nonces; OpenSSH key decryption with bcrypt-pbkdf and AES-256-CTR, AES-256-GCM or chacha20-poly1305@openssh.com; SHA-256 fingerprints | draft-miller-ssh-agent, RFC 4251, 4253, 5656, 6979, 8332 (flags), 8709 | `ssh-key` (with `ssh-cipher`, `ssh-encoding`, `bcrypt-pbkdf`, `ed25519-dalek`, `p256`) |
| KeePass KDBX 4 import, read only (ADR-0021) | AES-256-CBC, ChaCha20, Salsa20 (inner stream), AES-KDF, Argon2d/Argon2id, HMAC-SHA-256 block MACs, SHA-256/512 | KeePass KDBX 4 | `aes`, `cbc`, `chacha20`, `salsa20`, `argon2`, `hmac`, `sha2` |

## 3. Notation and encoding

- `||` is byte concatenation. Integers are unsigned big-endian.
- `UUID` values are 16 raw bytes (RFC 9562, version 7 for new IDs; the
  item ID of a conflict copy from an oversized split is version 8, derived
  as in §10).
- `HKDF(ikm, salt, info, L)` is HKDF-Extract-then-Expand with SHA-256.
- `ctx(label)` is the ASCII string `"mosaic/v1/" || label`. Every derived key
  and every AEAD use has a unique label. Labels MUST NOT be reused.
- `aad(label, f1, f2, ...)` = `ctx(label) || 0x00 || f1 || f2 || ...` where
  every `fi` is fixed-length. Variable-length fields are prefixed with their
  length as `u32`. This makes the encoding injective.
- **Header binding.** Every AEAD object starts with a cleartext header:
  `version:u8 || alg:u8`, plus any further fields the object defines. The
  AAD actually passed to the AEAD is always `header || aad(label, ...)`, so
  every header field is authenticated and an object cannot be reinterpreted
  under a different version or algorithm.
- The nil UUID (`00…00`) is reserved and MUST be rejected as a vault, item,
  attachment or account ID.
- `random(n)` is `n` bytes from the OS CSPRNG. If the CSPRNG fails, the
  operation MUST fail. There is no fallback.
- `credential_id_hash = SHA-256(credential_id)` (32 bytes) for a WebAuthn
  credential ID. `PRF(x)` is the WebAuthn PRF extension: the CTAP 2.1
  `hmac-secret` output for the salt `SHA-256("WebAuthn PRF" || 0x00 || x)`
  (WebAuthn Level 3 §10.1.4), evaluated with user verification.

## 4. Secrets and their lifetimes

| Secret | Size | Where it lives | Lifetime in memory |
|---|---|---|---|
| Master password `MP` | variable | user's head | until derivation finishes |
| Secret Key `SK` | 128 bit | device OS keystore, Emergency Kit | until derivation finishes |
| Stretched password `P` | 256 bit | never stored | until derivation finishes |
| Root key `R` | 256 bit | never stored | until derivation finishes |
| Key-encryption key `KEK` | 256 bit | never stored | until AK is unwrapped |
| Account key `AK` | 256 bit | wrapped (§6) | while unlocked |
| Vault key `VK` | 256 bit | wrapped by AK (§7); in a hardware-protected vault by `TK` (§7.4) | while unlocked; in a hardware-protected vault only while it is open |
| Identity key pair `(isk, ipk)` | X25519 | `isk` wrapped by AK | while unlocked |
| Attachment key `FK` | 256 bit | inside its item's plaintext | while the file is processed |
| Quick-unlock `W` | 256 bit | released by the platform key (§6.2), never stored in the clear | until AK is wrapped (enrollment) or unwrapped (unlock) |
| Security-key PRF output | 256 bit | on the security key | until `W` or `W_hw` is derived |
| `W_hw` (§7.4) | 256 bit | never stored | until `TK` is wrapped or unwrapped |
| Hardware-vault key `TK` | 256 bit | wrapped per security key (§7.4) | while the hardware-protected vault is open, at most 5 minutes |
| Settings MAC key `K_settings` (§11.1) | 256 bit | never stored | while unlocked |
| Passkey private key (§16) | 256-bit scalar in PKCS#8 | item plaintext (encrypted under the item key, §7.2) | an `Es256Key` lives for one create or sign, then drops (zeroize on drop) |
| Browser pairing key `K_pair` (§15) | 256 bit | the desktop's `browser-pairings.json` and the extension's IndexedDB; in host and desktop memory while used | while a browser is paired, until it is disconnected |
| Browser channel auth key `K_auth` (§15) | 256 bit | never stored | during the handshake |
| Browser channel session keys `K_h2d`, `K_d2h` (§15) | 2 x 256 bit | never stored | while the connection is open |

Rules for every secret type in code:

- Wrapped in a type that zeroizes on drop (`Zeroizing<[u8; N]>`,
  `SecretBox<…>`) and does **not** implement `Clone`, `Copy`, `Display` or
  `Serialize`. `Debug` prints only `"[REDACTED]"`.
- Never logged, never formatted into error messages, never in panic payloads.
  Error types carry no secret-derived data.
- Comparisons use `subtle::ConstantTimeEq`.
- One exception to "no `Serialize`": the browser channel's wire wrapper
  `HexKey` in `crates/native-messaging` (§15) serializes `K_pair` as 64 hex
  digits, because the key must travel in the `pair` response and in the stored
  pairing of `hello`. It wraps `PairingKey` (which itself has no `Serialize`),
  prints `[REDACTED]` in `Debug`, builds its text in zeroizing memory and is
  used for those two bodies only. `PairingKey` implements `ConstantTimeEq`.
- Release builds use `panic = "abort"`.
- Swapping memory to disk is a residual risk; `mlock` would need `unsafe` and
  is deferred (ADR-0005).

## 5. Account secrets and key derivation

### 5.1 Master password preparation

`MP' = NFC(MP)` as UTF-8. No trimming or case folding. An empty password is
rejected. Client UI enforces the strength policy (zxcvbn-style estimate ≥ 3 of
4 *and* length ≥ 10). The crypto layer only rejects empty input.

### 5.2 Secret Key

- 16 bytes `SK = random(16)`, generated on the device at account creation.
  It is never sent to the server.
- Display format: `M1-` followed by 28 Crockford Base32 characters in 7 groups
  of 4, separated by `-`. They encode `SK || c`, where
  `c = SHA-256("mosaic/v1/secret-key-checksum" || SK)[0]`. That is 17 bytes,
  zero-padded to 140 bits. The checksum only catches typos. It is not a
  security mechanism.
- Parsing is case-insensitive and ignores `-` and whitespace. Crockford
  aliases are accepted (`O→0`, `I/L→1`). A wrong checksum gives the error
  "Secret Key mistyped", never "wrong password".
- It is shown once, in the Emergency Kit (PDF with account email, server URL,
  Secret Key and a blank field for the master password), and kept in the OS
  keystore (Windows Credential Manager/DPAPI, macOS/iOS Keychain, the Linux kernel keyring per login session (Secret Service is deferred: its transport crypto is outside §2; ADR-0020), Android Keystore-wrapped). Without a stored key, the user types it from the Emergency Kit.

### 5.3 Argon2id

```
P = Argon2id(password = MP', salt = account_salt, m, t, p, len = 32)
```

- `account_salt = random(16)`, created at account creation and stored on the
  server in plaintext (§12).
- Parameters are stored **per account**, because every device must derive
  the same `P`. They are always one of a small, fixed set of **profiles**.
  That way the parameters reveal little about an account, and fake pre-login
  answers can mimic them (§9.2):

  | Profile ID | m | t | p |
  |---|---|---|---|
  | `0x01` baseline | 128 MiB | 3 | 4 |
  | `0x02` standard (default) | 256 MiB | 3 | 4 |
  | `0x03` strong | 512 MiB | 3 | 4 |
  | `0x04` max | 1 GiB | 4 | 4 |

- **Calibration** at account creation picks the strongest profile that the
  creating device can allocate and that derives in about 1 s or less. It
  never picks below `baseline`, and it prefers `standard` whenever that takes
  2 s or less. Unknown profile IDs are rejected. Adding a profile is a spec
  change.
- Only listed profiles are accepted, so a hostile server cannot exhaust
  client resources through parameters. The largest profile uses 1 GiB.
- **Upgrade:** when the client policy requires a stronger profile than the
  stored one, the next successful unlock re-derives the keys under the new
  profile, re-wraps AK, re-registers OPAQUE (§9.2, under the rules of §9.4)
  and stores the new profile. Profiles only ever increase.
- Constrained contexts never run Argon2id. This includes the iOS AutoFill
  extension, which has a memory limit of roughly 120 MiB. They use quick
  unlock (§6.2).

### 5.4 Two-secret root key

```
S    = HKDF(ikm = SK, salt = account_id, info = ctx("secret-key"), 32)
R    = HKDF(ikm = P || S, salt = account_id, info = ctx("root"), 32)
KEK  = HKDF(ikm = R, salt = "", info = ctx("kek"), 32)
OPW  = HKDF(ikm = R, salt = "", info = ctx("opaque-password"), 32)
```

`account_id` is the 16-byte account UUID. The client generates it randomly at
signup and the server confirms it. New devices get it through pre-login
(§9.2). `KEK` and `OPW` are independent outputs of
the same root, so learning one reveals nothing about the other. `P`, `S` and
`R` are zeroized immediately after `KEK` and `OPW` are derived.

## 6. Account key (AK)

`AK = random(32)` at account creation. It is the single key that every unlock
method unlocks. Each method stores its own wrapping of AK:

```
WrappedKey = version:u8 (0x01) || alg:u8 (0x01 = XChaCha20-Poly1305)
          || nonce[24] || AEAD(key = W, nonce, plaintext = AK, aad)
aad        = aad("wrap/ak", account_id, method:u8, method_id[16])
```

### 6.1 Password unlock (`method = 0x01`)
`W = KEK`, `method_id = account_id`. This wrapping is synced so any device can
unlock with MP + SK.

### 6.2 Quick unlock (`method = 0x02`, device-local, never synced)
`W = random(32)` is protected by a hardware-bound, non-exportable platform key
that can only be used after user verification. Platform keystores hold
asymmetric or AES keys, not arbitrary secrets, so the mechanism is fixed per
platform:

| Platform | Mechanism |
|---|---|
| macOS / iOS | A Secure Enclave P-256 key created with access control `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`, `kSecAccessControlPrivateKeyUsage | kSecAccessControlBiometryCurrentSet` (biometry only; a change of enrolled fingers or faces invalidates it). `W` is encrypted with `SecKeyCreateEncryptedData` (`eciesEncryptionCofactorX963SHA256AESGCM`); the 113-byte result is stored as the wrapping's public parameters in the local store. |
| Android | A Keystore AES-256-GCM key (StrongBox when available) created with `setUserAuthenticationRequired(true)` and invalidated when biometric enrollment changes. It encrypts `W`. |
| Windows | A Windows Hello `KeyCredentialManager` RSA key and a stored random 32-byte `challenge`. `W = HKDF(ikm = Sign(challenge), salt = challenge, info = ctx("hello-unlock"), 32)`. This relies on RSA PKCS#1 v1.5 signatures being deterministic, so enrollment signs twice and fails if the two signatures differ. Enrollment also requires `GetAttestationAsync` to report `Success` or `TemporaryFailure` (a TPM-backed key). |
| Linux | No hardware binding by default. Quick unlock is unavailable unless a FIDO2 key is used (§6.3). |

The platform key is named by a random 16-byte `key_nonce` drawn per
enrollment and stored with the public parameters
(`0x02 || platform:u8 || key_nonce[16] || body`), never by the account ID.
ADR-0020 item 7 and ADR-0028 pin the Windows and macOS calls. Any deviation
from this table needs a spec change. `method_id` is an **enrollment ID**: a
fresh UUIDv7 drawn at enrollment. Its 48-bit timestamp is the enrollment
time; it is bound into the AAD, so the enrollment time cannot be changed
without breaking the wrapping. A store holds at most one quick-unlock
wrapping. Quick unlock is used only after the master password unlocked
once since the store was opened (once per app start; this is the reboot
policy), and only before the wrapping's maximum age (default 14 days,
configurable from 1 to 30 days; implementations MUST NOT accept more than
30 days). An enrollment time more than 5 minutes in the future counts as
expired. Expiry, a `W` that does not open the wrapping, a master password
change (§6.4) and a Secret Key rotation (§6.5) **delete** both the platform
key and the wrapping. Flagging them as expired is not enough. The session API
does not offer Secret Key rotation yet; when it does, it MUST delete every
quick-unlock wrapping in the same transaction, like a password change.
Enrolling requires the master password, and the vault API enforces it: the
enrollment call takes a proof (`Reauthenticated`) that only a successful
master-password and Secret Key check of the same unlocked session creates,
and refuses one older than 120 seconds (`REAUTH_GRANT`) or from another
session. The previous enrollment is replaced in one transaction. This
wrapping is never synced.

### 6.3 Hardware-key unlock (`method = 0x03`)
Uses the WebAuthn PRF extension (CTAP2 `hmac-secret`):
```
prf_salt = random(32)   (stored with the wrapping)
W = HKDF(ikm = PRF(prf_salt), salt = credential_id_hash, info = ctx("prf-unlock"), 32)
```
`method_id` = first 16 bytes of `SHA-256(credential_id)`. Assertions MUST use
`userVerification = "required"` (PIN or biometric on the key). Without it, a
stolen key plus a stolen locked device would unlock with neither MP nor SK.
The credential is non-discoverable (`rk = false`) and created for the RP ID
stored with the wrapping (`mosaic.invalid` for desktop enrollments; a
reserved name that no web origin can claim). The public parameters are
`0x01 || prf_salt[32] || rp_id_len:u8 || rp_id || credential_id_len:u16 ||
credential_id`, with `rp_id` 1–253 bytes of printable ASCII without spaces
and `credential_id` 16–1023 bytes; decoders are strict and reject a row
whose `method_id` is not `SHA-256(credential_id)[..16]`. Every assertion,
including the one at registration, MUST use user verification and the
client MUST check the UV flag of the returned authenticator data
(hmac-secret outputs differ with and without UV). At most 8 hardware keys
per account. Enrolling needs the master password, enforced by the vault API
as in §6.2 (a proof of the same session, at most 120 seconds old). A
`prf_salt` (and a `prf_salt_hw`, §7.4) MUST be new for its credential: the
vault refuses one that equals any stored salt of the same credential, in the
unlock wrapping or in any TK wrapping, so one PRF output never serves two
wrappings. After 5 rejected hardware-key unlocks in a row the client requires
the master password. This counter lives for one app run (a restart resets
it); the authenticator's own PIN retry counter is the actual brute-force
limit. The client reports PIN failures that never reach the vault through
`note_failed_hardware_attempt`.
This wrapping MAY be synced, so the hardware key can unlock any device that
can reach the key with the stored RP ID. `mosaic.invalid` is not a web
origin, so browser and mobile WebAuthn refuse it: it works only through raw
CTAP clients (the desktop app). A synced wrapping therefore unlocks another
device only if that device also speaks raw CTAP; web and mobile clients need
an enrollment of their own under a real RP ID.
Residual risk: someone holding the key, its PIN and a copy of the wrapping
can unlock (THREAT_MODEL R8).

### 6.4 Master password change
Derive a new `KEK'` (new `account_salt`, current parameters) from the new MP
and the same SK. Re-wrap AK with `KEK'`, re-register OPAQUE with `OPW'`, then
delete the old password wrapping atomically on the server. Items and vault keys
are not touched. The new profile is never weaker than the current one (§5.3);
the client refuses a downgrade.

### 6.5 Secret Key rotation
Same as §6.4 with a new SK. A new Emergency Kit is issued.

**What §6.4 and §6.5 do not do:** they stop *server login* with the old
secrets, nothing more. AK does not change. Anyone holding the old MP and SK
plus any old copy of the password wrapping (a server backup, an old local
database) can still recover AK and every VK. When a user rotates because of a
suspected compromise, the UI MUST say this and offer §6.6.

### 6.6 Full re-key (compromise recovery)
1. Generate a new AK, a new identity key pair, and a new VK for every vault
   the user owns.
2. Re-encrypt every item and attachment under the new VKs. Increment
   `key_generation` for each of those vaults.
3. Re-wrap everything under the new AK. Re-share the new VKs with all members
   (§8.2).
4. Re-register OPAQUE with the new MP and SK. Issue a new Emergency Kit.
5. Delete every old wrapping and quick-unlock enrollment. Revoke all
   sessions.

Shared vaults owned by other people can only be rotated by their owners
(§8.3). The UI lists those vaults.

## 7. Vaults and items

### 7.1 Vault key
`VK = random(32)` per vault.
```
WrappedVaultKey = WrappedKey with W = AK,
  aad = aad("wrap/vault", account_id, vault_id, key_generation:u32)
```
`key_generation` starts at 1 and increases on every rotation (§8.3). Clients
MUST store the highest accepted `key_generation` for each vault, and MUST
reject wrapped keys, shares, items and tombstones that carry a lower one.
Otherwise a server could roll back a rotation, and a removed member could
inject items under the old key.

A client MUST NOT replace a vault key it holds with a different key of the
same generation (it compares the unwrapped keys in constant time; the same
key again is a no-op). A stored wrapping that does not open under AK (a
damaged row) is not a held key: a received wrapping of that generation
that opens replaces it (ADR-0012 amendment). Two devices that rotate a vault concurrently create
two keys for one generation; each keeps its own and refuses the other, so
nothing it sealed becomes unreadable (ADR-0013). The sync server accepts
exactly one wrapped key per vault and generation (compare-and-swap on the
generation, M6), and a device uploads a rotation's wrapped key, and has it
accepted, before anything sealed under that generation. A device whose
rotation is refused rotates again to the next generation instead.

A client that accepts a newer generation (its own rotation or a received
wrapped key) re-seals every object it can read that is still sealed under
an older one, and queues it for upload. Peers that already accepted the
newer generation reject the older ciphertext, so a version that only this
client holds would otherwise never reach them. An interrupted re-seal is
finished at the next unlock.

A vault's metadata (name, icon, description) is an object type of its own:
```
VaultMeta = header || nonce[24] || AEAD(VK, nonce, pad(meta), aad)
header    = version:u8 || alg:u8 || key_generation:u32
aad       = header || aad("vault-meta", vault_id, key_generation:u32)
```
The padded metadata is at most 64 KiB (ADR-0007).

### 7.2 Item records
Every item is encrypted on its own with its vault's key:

```
ItemCiphertext = header || nonce[24] || AEAD(VK, nonce, pad(plaintext), aad)
header = version:u8 (0x01) || alg:u8 (0x01) || schema_version:u16 || key_generation:u32
aad    = header || aad("item", vault_id, item_id, schema_version:u16, key_generation:u32)
nonce  = random(24)    -- fresh for every encryption, never derived
```

- The sync `record_id` **is** the `item_id`, and `vault_id` comes from the
  record's vault binding. The decryptor therefore takes both from context,
  and a mismatch fails authentication.
- A deletion is a **tombstone**:
  `header || nonce || AEAD(VK, nonce, tombstone, header || aad("tombstone", vault_id, item_id, key_generation:u32))`
  with `header = version:u8 (0x01) || alg:u8 (0x01) || key_generation:u32`.
  Its plaintext holds the final version vector and the deletion time. It is
  not padded and is at most 64 KiB (ADR-0007).

- **Plaintext** is the canonical serialization of the item (§7.5, versioned
  by `schema_version`). It **includes** the item's version vector and the
  stamp of every field value, so they are authenticated. Revision metadata
  the server sees is only a routing hint (rollback handling: §10).
- **Padding** `pad(x)`: ISO/IEC 7816-4 (`0x80` then zeros) to the next multiple
  of 256 bytes, with a minimum padded length of 512 bytes. This hides fine
  length information such as password length. The padded plaintext is at
  most 1 MiB (ADR-0007). Unpadding is strict: only the canonical padding of
  some message is accepted. It runs on decrypted plaintext, so it is
  constant-time in the contents (it does not stop early at the marker),
  and its timing does not reveal the message length.
- **Re-encryption** on every edit uses a fresh nonce. With 192-bit random
  nonces, collision risk is negligible far beyond any realistic number of
  encryptions under one key.
- A decryption failure is reported as `ItemCorrupted { item_id }` and nothing
  else. There is no distinction between tag and padding failures. (In
  `crates/core` every object type reports the single error
  `Error::Decryption`; the item layer maps it to `ItemCorrupted`, ADR-0007.)
- **Moving an item** to another vault means decrypting it and re-encrypting it
  under the target VK with the new `vault_id` in the AAD. Each of its
  attachments is re-sealed under a fresh `attachment_id` and a fresh FK in
  the target vault. Attachment IDs are never reused, so the new copy is
  stored before the old one is deleted (ADR-0010).

### 7.3 Attachments
- `FK = random(32)` per attachment. FK, file name, MIME type and size live
  inside the owning item's encrypted plaintext.
- The content is encrypted with STREAM-BE32 over XChaCha20-Poly1305, using
  RustCrypto's STREAM implementation (`aead-stream` or `aead::stream`,
  depending on the `aead` version pinned in M1). File layout:
  `version:u8 || alg:u8 (0x02 = STREAM-BE32-XChaCha20-Poly1305) || chunk_log2:u8 (16) || nonce_prefix[19] || chunks…`.
  Every chunk uses `aad = header || aad("attachment", vault_id, item_id, attachment_id)`
  with `header = version || alg || chunk_log2`. The nonce prefix is
  authenticated through each chunk's nonce: chunk `i` (from 0) uses
  `nonce = nonce_prefix || i:u32 (big-endian) || last:u8`, with `last = 0x01`
  for the final chunk and `0x00` otherwise.
  The last chunk carries STREAM's last-block flag, so truncation is detected.
  Every chunk except the last holds exactly `2^chunk_log2` plaintext bytes.
  The last one holds 1 to `2^chunk_log2` bytes, or 0 bytes only for an
  empty file. Decoders reject any other layout (ADR-0007).
- Default size limit: 50 MiB per attachment, 1 GiB per account. The server
  enforces it, and clients check it before encrypting.

### 7.4 Password history, trash, TOTP-only vault
- Password history entries are part of the item plaintext, so they are
  encrypted with it.
- Trash is the field `trashed_at` inside the plaintext. The server does not
  learn it (no server-side hint, ADR-0012). A hard delete ("delete
  forever") replaces the record with its encrypted tombstone (§7.2), which
  syncs like any edit and keeps the record's rollback anchor (§10). A
  record row is purged locally only once the server no longer serves the
  record either (ADR-0010).
- **Hardware-protected TOTP vault:** a normal vault whose VK is wrapped *only*
  under a hardware-key wrapping of a separate key, not under AK. It uses its
  **own** `prf_salt_hw = random(32)`, never the §6.3 salt:
  ```
  TK            = random(32)
  W_hw          = HKDF(ikm = PRF(prf_salt_hw), salt = credential_id_hash, info = ctx("prf-hw-vault"), 32)
  WrappedTK     = WrappedKey with W = W_hw,
                  aad = aad("wrap/hw-vault", account_id, vault_id, method_id[16])
  WrappedVK     = WrappedKey with W = TK,
                  aad = aad("wrap/vault-under-tk", vault_id, key_generation:u32)
  ```
  `method_id = SHA-256(credential_id)[..16]`; the TK wrapping's parameters use
  the §6.3 encoding with `prf_salt_hw`. `userVerification = "required"` applies
  here too. A hardware-protected vault holds only items of type `totp`; every
  OTP configuration in it has `hardware_protected = true`, and outside it
  `false`. Opening it unwraps TK and its vault keys for at most 5 minutes;
  locking the app or closing the vault drops them. At most 8 keys per vault;
  the last one cannot be removed. Removing a key deletes its TK wrapping but
  does not change TK: anyone who kept that key, its PIN and a copy of the old
  wrapping can still open the vault (THREAT_MODEL R24). Trade-off shown in
  the UI: "Codes in this vault need your security key every time, so a stolen
  master password alone is not enough, but losing every registered key loses
  these codes."

### 7.5 Item plaintext encoding (schema_version 1)
The plaintext of items, vault metadata (§7.1) and tombstones (§7.2) is a
replicated document, serialized as compact JSON (RFC 8259) with object
members in a fixed order and no insignificant whitespace (ADR-0009):

```
item, vault meta = {"schema_version":1,"vv":VV,"fields":{KEY:[ENTRY,...],...}, EXTRA...}
tombstone        = {"schema_version":1,"vv":VV,"deleted":[ENTRY,...], EXTRA...}
VV    = {DEVICE_ID:counter,...}           -- counter: u64 >= 1
ENTRY = {"at":ms,"c":counter,"d":DEVICE_ID,"v":VALUE, EXTRA...}
```

- `DEVICE_ID` is the lowercase hyphenated UUID of the writer (§10: a
  fresh writer ID for every unlocked session). `(c, d)` is the Lamport
  stamp of the edit that wrote the value: `c` is one more than the largest
  counter in the document's `vv` when the edit starts. `at` is the device's wall clock in Unix milliseconds,
  shown to the user only and never used to order edits.
- `KEY` is 1 to 256 bytes of UTF-8 without control characters. Item keys:
  `type`, `created_at`, `trashed_at`, `title`, `notes`, `favorite`, one key
  per typed text or secret field (`login.username`, `login.password`,
  `card.number`, ...), one key per collection element (`url:<id>`,
  `field:<id>`, `attachment:<uuid>`, `tag:<name>`, `history:<id>`), the
  structured values `otp`, `passkey`, `ssh.key`, the counters
  `otp.counter` and `passkey.sign_count`, and `conflict_copy_of` (the item
  ID of the record a conflict copy was split off from, §10, ADR-0016).
  Vault metadata keys: `name`, `icon`, `description`. The table in
  `crates/core/src/model/item/` is the reference.
- A register (`[ENTRY,...]`) holds the values of one key that no replica
  has superseded yet: normally one, after concurrent edits several.
- Unknown keys, unknown members of an entry or a structured value, and
  unknown top-level members (`EXTRA`) are preserved and written back, so
  an older client never drops what a newer one wrote. New top-level
  members MUST only ever grow (they merge by the larger value, ADR-0009
  item 4); anything that can change goes under `fields`.
- The names `schema_version`, `vv`, `fields` and `deleted` are reserved in
  both layouts. A decoder MUST reject any of them as an unknown member,
  and every decoder is strict otherwise: duplicate keys or stamps, wrong
  types, an empty register, a counter of 0, a stamp not covered by `vv`
  and nesting deeper than 32 are `Malformed`, and any `schema_version`
  other than 1 is `UnsupportedVersion`. Fields map keys are serialized in
  byte order and entry members as shown, so equal documents encode to
  equal bytes.
- Strings are held in zeroizing buffers after decoding; secret fields are
  read into the crate's secret types (§4).

## 8. Sharing and emergency access

### 8.1 Identity keys
Each account has an X25519 key pair. `isk` is stored wrapped under AK
(`aad("wrap/identity", account_id)`). `ipk` is published on the server.

### 8.2 Sharing a vault
For each recipient, the sender seals VK with HPKE mode **Auth**, using the
sender's `isk` as the authentication key:
```
info = ctx("share/vault") || vault_id || sender_account_id || recipient_account_id
aad  = aad("share/vault", vault_id, key_generation:u32)
```
The recipient opens the seal with their `isk` and the sender's `ipk`, then
re-wraps VK under their own AK. Auth mode stops the server from injecting a
vault key it made up and claiming it came from a sender.

**Public-key authenticity:** the server distributes `ipk`. A malicious server
could hand out its own key (MITM). As mitigation, the UI shows a 130-bit safety
code for each contact. The code is the first 130 bits of
`SHA-256(ctx("fingerprint") || account_id || ipk)` as 26 Crockford Base32
characters, shown in groups of 5. An optional "verify in person" step
compares the codes by QR scan or by reading them aloud. Pinning starts at
the first share, and a key change triggers a warning. Recorded as residual risk
in the threat model.

**Known limits of v1 (THREAT_MODEL R7):**
- HPKE Auth mode is not resistant to key-compromise impersonation. Anyone
  holding a recipient's `isk` can forge shares *to* that recipient.
- Every member holding VK can create validly encrypted items in that vault.
  Read-only roles are enforced by the server only. Per-author item signatures
  are a candidate for a v2 format.

### 8.3 Revoking access
To revoke a member, rotate VK: generate a new VK, re-encrypt every item in the
vault, re-seal every attachment of the vault under a fresh `attachment_id`
and a fresh FK (as for a move, §7.2; the revoking client fetches every
attachment first and updates the references in the same pass, and the old
ciphertexts are deleted on the server), re-share the new VK with the
remaining members, and increment `key_generation`. Until the rotation
finishes, the removed member keeps whatever they already downloaded. This
cannot be avoided and is stated in the UI.

The attachment re-seal is required: the FK sits in item plaintext the removed
member has read, and the attachment AAD carries no key generation, so without
a fresh FK the removed member and the server together could replace an
attachment's ciphertext with one that authenticates (ADR-0017). An attachment
the revoking client cannot fetch is listed in the revoke UI and is not
protected against the removed member until it is re-sealed. A key rotation
without a revocation (no other account holds VK, as in M2) MAY keep
attachments as they are.

### 8.4 Emergency access
The grantor seals AK, or selected VKs, to the grantee's `ipk` with HPKE Auth:
```
info = ctx("emergency") || grantor_account_id || grantee_account_id
aad  = aad("emergency", content_type:u8 (0x01 = AK, 0x02 = VK), vault_id or nil, key_generation:u32 or 0)
```
A full re-key (§6.6) or a VK rotation invalidates existing seals, and they are
re-created automatically. The server holds the sealed blob and releases it only
after the waiting period, unless the grantor declines. The *waiting period* is
enforced by the server, not by crypto. A server colluding with the grantee
could release the blob early. Recorded as accepted risk.

## 9. Account authentication (OPAQUE)

### 9.1 Ciphersuite
`opaque-ke` (major version pinned in M6; 4.x at the time of writing) with
`OprfCs = Ristretto255`, `KeyExchange = TripleDh<Ristretto255, Sha512>`, and
`Ksf = Identity`. The login `context` is `"mosaic/v1/opaque-login"`.
Identity KSF is acceptable **only** because the OPAQUE password input is `OPW`
(§5.4). `OPW` is already Argon2id-stretched and has ≥ 128 bits of entropy from
SK, so there is nothing left to stretch. This is ADR-0003.

Identifiers: `client_identity = account_id`, `server_identity = server's
configured canonical origin`. Binding to the server's origin stops envelopes
from being reused across servers.

### 9.2 Registration and login
- Registration sends `OPW` through the OPAQUE registration flow. The server
  stores the registration record. It never sees `OPW`, `P`, `R` or SK.
- Login: OPAQUE KE1/KE2/KE3. The resulting `session_key` is used once:
  `session_token = HKDF(session_key, "", ctx("session"), 32)`, which the
  server stores hashed (SHA-256) along with its expiry.
- **Pre-login.** A new device needs `(account_id, account_salt, profile)`
  before it can compute `OPW`. The server returns these for any email.
  - Email normalization (same on client and server): trim whitespace, apply
    NFC, split at the last `@`, lowercase the local part, and convert the
    domain to its lowercase IDNA A-label. Anything that is not a
    syntactically valid address is rejected.
  - For unknown emails, all three values are deterministic fakes:
    `HMAC-SHA-256(server_secret, label || normalized_email)`, with the labels
    `"prelogin/account-id"`, `"prelogin/salt"` and `"prelogin/profile"`. The
    profile comes from mapping the HMAC output onto the server's real profile
    distribution, rounded to 5 % buckets. opaque-ke's fake credential
    response is used, so KE2 looks the same as for a real account.
  - Responses are rate-limited per IP and per normalized email. Residual:
    the profile distribution drifts over time and could give statistical
    hints. Accepted.
- **Server database compromise.** An attacker with the registration records
  must guess `OPW`, which includes the 128-bit SK. That is infeasible. This is
  the main reason the SK is required.

### 9.3 Second factor for the sync account
After OPAQUE succeeds, the server can require TOTP or a WebAuthn assertion
before it issues the `session_token`. This guards server access (for example,
deleting the account or reading ciphertext) but adds nothing to vault
confidentiality. The UI says so.

### 9.4 Changing the registration
A request that re-registers OPAQUE, or that replaces or deletes the password
wrapping of AK, is accepted only if it carries a **fresh OPAQUE login** (KE3
completed within the last 5 minutes, in the same session) and passes account
2FA when enabled. The write is a compare-and-swap on the account record
version, so concurrent upgrades from two devices cannot interleave. The
device that loses the race re-reads and retries. A stolen session token alone
therefore cannot lock the user out.

## 10. Sync protocol security

- Records are `(record_id, vault_id, ciphertext, server_seq, hint_version)`.
  The client trusts only the version vector inside the decrypted plaintext.
- **Rollback / replay:** the client keeps, per record, the highest version
  vector it has accepted. It keeps it as the record ciphertext it last
  accepted (an item or a tombstone) and reads the vector from that record's
  authenticated plaintext. No cleartext copy of a vector is stored (§11,
  ADR-0010). A record whose authenticated vector is dominated by
  the stored one is ignored. A record that is concurrent with the stored one
  goes to conflict resolution. The server can withhold updates (freeze attack)
  but cannot forge or roll back. Freeze detection: every device publishes a
  checkpoint
  `HMAC-SHA-256(K_cp, ctx("checkpoint") || device_id || counter:u64 || SHA-256(canonical seen vectors))`,
  where `K_cp = HKDF(AK, "", ctx("checkpoint-key"), 32)` and `counter` is a
  per-device monotonic counter. Other devices warn when the checkpoints they receive are
  older than their own state for longer than 24 h.
- **Deletions** are encrypted tombstones, so the server cannot forge a delete.
- **Conflict model** (ADR-0009, encoding §7.5). Every field is a
  multi-value register; the merge of two replicas is a pure join
  (commutative, associative, idempotent), so replicas that have seen the
  same edits hold the same state, whatever the order, duplicates or
  replays of delivery:
  - an entry survives the merge if both sides hold it, or one side holds
    it and the other side's `vv` does not cover its stamp; an entry one
    side has seen but no longer holds was superseded or removed there;
  - the displayed value of a field is the entry with the largest stamp
    (counter, then device ID); concurrent losers stay as **conflict
    copies**, shown to the user, until a device resolves or dismisses
    them. Concurrent edits of different fields, URLs, tags or custom
    fields never conflict;
  - `type`, `created_at`, `trashed_at`, `favorite` and `history:*` take the
    largest stamp and are never shown as conflicts; `otp.counter` and
    `passkey.sign_count` merge by maximum, and no resolution lowers them;
  - a password change moves the old value into `history:*`, so
    overwriting a value never loses it silently. A local change leaves at
    most 32 entries, trimmed by edit order: the stamp of the edit that
    moved the value there, which the key `history:{counter:016x}.{device}.{seq}`
    names (a repair that writes an entry back under a fresh stamp keeps its
    key, so it keeps its place; a key of another form falls back to its
    winning stamp). A merge of concurrent changes from several devices can
    hold a few more, and the next change on any device trims back to 32.
- **Merging** (ADR-0009): a client merges every received record into its
  own latest state of that record with the repairing merge
  (`Item::merge_and_repair` in `crates/core`) and persists only its result,
  never the plain join. A delete that races a concurrent write loses; every
  replica that had not seen the deletion writes back every value it held
  that the deletion removed (conflict copies included, also in a field
  that survives through a concurrent write; fields the received side holds
  are left to the merge), and the item stays flagged until the user
  dismisses or deletes again. Vault metadata merges the same way
  (`VaultMeta::merge_and_repair`, with the device-ID checks below); it is
  never deleted.
- **Oversized merges** (ADR-0016, amended in the fourth M2 review): if the
  repaired merge of two records that each fit does not fit the item size
  limit (§7.2), the merge does not fail. The record is the repaired merge
  reduced as a pure function of that merge, so every device that reaches
  the same state stores the same record, whichever versions it merged in
  which order: every content field keeps only the value it shows (its
  largest stamp), and if the record still does not fit, whole content
  fields go too, the largest encoded value first (then the larger key).
  Metadata, `history:*`, the counters, attachment references (their files
  are bound to the record ID, §7.3) and the `deleted` entries always stay.
  The vector is the merge's, so the record dominates both inputs. Every
  value that left the record, other than a repeat of a value the field
  still shows, becomes a conflict copy: a new item of the vault holding
  only that value under its key with its original stamp and
  `conflict_copy_of` (the record's item ID, stamped like the value; left
  out if the copy would not fit otherwise), with a version vector of
  exactly that stamp. The copy has no type of its own and is shown with
  the original's. It is a function of the value alone, because every
  device that splits the value off builds it: anything taken from the
  device's record could differ between devices under equal vectors, or
  carry a device's own stamp that its copy lacks (read as a reused writer
  ID). Its item ID is
  `UUIDv8(HKDF(VK, "", ctx("conflict-copy-id") || item_id || u32(len(key)) || key || counter:u64 || device_id, 16))`
  under the vault's current key, where `(counter, device_id)` is the
  value's stamp and the UUID version and variant bits are then set. So
  every device builds the same copy under the same ID, and nobody without
  `VK` can link a copy to its record (§12). A device stores a copy only if
  it holds no record of that ID yet. If the record does not fit even
  without any content field, or a value does not fit a copy of its own,
  the merge fails with `SizeLimit` and nothing changes.
- **Applying a received record** (ADR-0012, `mosaic-vault`
  `UnlockedSession::apply_remote_record`): the record MUST carry exactly
  the vault's highest accepted `key_generation` (§7.1; a lower one is a
  rollback, a higher one waits for its wrapped key). It is decrypted with
  the vault ID and record ID it was filed under. A record the client has
  no state for is stored as received. A record whose vector the stored one
  dominates or equals is ignored. Otherwise the repairing merge runs; a
  result equal to the received record is stored as received, any other
  result is sealed under the current key and queued for upload. A stored
  row that cannot be read (damaged, or sealed under a key the client no
  longer has) gives no rollback anchor: the received version replaces it,
  and the client reports this so the UI can flag the item. The same holds
  for vault metadata (`ReplacedUnreadable` from
  `apply_remote_vault_meta`).
- **Writer IDs** (the `DEVICE_ID` of stamps, §7.5) are fresh for every
  unlocked session: the client generates a new one at every unlock and
  never reads an old one back (ADR-0015). A restored backup or a copied
  profile therefore never writes under an ID another copy uses, and a local
  store is open in at most one session at a time (an exclusive lock on its
  resolved path, ADR-0010 item 10). The desktop app and the browser
  extension never share a store. Server-side device identity (§12) and the
  freeze checkpoints above use a stable per-install ID, not the writer ID.
- **Writer-ID collisions** (forged or corrupted records, or a UUIDv7
  collision) are handled without losing a value:
  - A received record that holds a counter for the client's own writer ID
    above the client's stored state for that record, or one of the
    client's own stamps with other contents, means the ID is reused: the
    client rotates its writer ID before it edits or merges again
    (`Error::DeviceIdReused`). Records the client holds no state for are
    not checked, because the copy of a moved item keeps the source's
    version vector (ADR-0012).
  - After such a rotation the old ID stays suspect for the session. For a
    record where it collides, the client's entries under it that the
    received side does not hold are written back under the new ID, and
    the received side's entries under it are kept although the client's
    vector covers their counters.
  - Equal version vectors with different contents are a collision too
    (honest replicas with equal vectors are equal): every value either
    side held that the merge dropped is written back.
  - Equal stamps with different contents are kept as a conflict. A
    password history key is written by one edit, so two writers under one
    ID can write the same `history:*` key; a value written back there gets
    a history key of its own (named after its fresh stamp), because the
    history shows one value per key.
  Each case is reported to the user; a value one writer had superseded can
  come back as a conflict copy. Among written-back values of a field, the
  one written last by the wall clock is shown.
- Message parsing on both ends is strict and length-bounded, and is covered by
  fuzz targets.

## 11. Local storage

- The local SQLite database holds the same ciphertext records as the server,
  plus the wrapped keys and the Argon2 parameters. Item plaintext, titles and
  URLs are never written to disk. The search index exists only in memory while
  unlocked.
- Nothing derived from item plaintext is stored in cleartext. That includes
  sync state such as version vectors, revisions and per-device counters.
  The only cleartext metadata is what the server sees as well (§12): IDs, key
  generations, sizes and sequence numbers. The highest accepted
  `key_generation` of a vault (§7.1) outlives the vault's deletion from the
  local store (ADR-0010).
- A removed or replaced wrapped key (quick-unlock
  expiry or invalidation §6.2, password change §6.4, re-key §6.6, a removed
  hardware key or TK wrapping) is erased from the database file and its
  write-ahead log before the call returns. Copies that the file system or
  the storage device keeps outside the database are out of reach
  (THREAT_MODEL R12).
- Locking zeroizes AK, every VK, `isk` and the decrypted item cache.
- Auto-lock triggers: idle timeout (default 10 min), OS sleep, screen lock,
  user switch, and app quit. All configurable, but they cannot all be
  disabled.
- Encrypted export: an `age`-compatible file with a passphrase recipient
  (scrypt) is under evaluation. Until an ADR decides, export uses the
  item-record format under a fresh key derived as in §5.3 from an export
  passphrase.

### 11.1 Settings integrity (ADR-0030)

`settings.json` is plaintext, so the protection settings in it carry a MAC.

- `K_settings = HKDF(AK, "", ctx("settings-mac"), 32)`.
- `settings_nonce = random(16)`. It is stored in the vault store in a
  local-only row (`settings_nonce`, never part of the outbox or of sync) and
  is replaced by a fresh `random(16)` in the same step as every MAC'd write
  of `settings.json`. A counter is not used.
- `mac = HMAC-SHA-256(K_settings, account_id[16] || settings_nonce[16] || canonical(settings))`,
  stored as 64 lowercase hex characters in the top-level field `mac`. A
  verifier compares in constant time. The nonce makes a restored older file
  (rollback) fail: its MAC was made under a nonce the store no longer holds.
- `canonical(settings)` is the UTF-8 JSON object
  `{"autoLockMinutes":A,"clipboardClearSeconds":C,"email":E,"kitVerified":K,"language":L,"lockOnScreenLock":S,"lockOnSleep":P,"quickUnlockMaxAgeDays":Q,"sshAgentEnabled":B,"theme":T}`:
  exactly these keys in this (ascending byte) order, no whitespace, `A` and `C`
  an unsigned decimal integer or `null`, `Q` an unsigned decimal integer, `S`,
  `P`, `K` and `B` `true` or `false`, `E` the account email as a JSON string
  (RFC 8259: `"` and `\` escaped, U+0000 to U+001F as `\b \t \n \f \r` or
  `\u00xx` in lowercase hex, everything else as raw UTF-8) or `null` when
  the file has no email, `K` the `kitVerified` flag of the account block
  (`false` without an account block), `L` one of `"system"`, `"en"`, `"de"`,
  `T` one of `"system"`, `"light"`, `"dark"`. `B` is the `sshAgentEnabled`
  flag. It is computed over the values as written to the file (before range
  clamping on read).
- While locked, the app uses for each protection setting the stricter of the
  file value and the default (10 min, 30 s, 14 days, both lock-on switches
  on), `sshAgentEnabled` reads `false`, and the quick-unlock expiry at the
  start of the app uses only the 30-day cap (§6.2).
- After the first unlock of a run it verifies the MAC against the store's
  `settings_nonce`. Valid applies the file. Missing or invalid (including a
  missing nonce) keeps the stricter values, sets `sshAgentEnabled` to
  `false` (before the SSH agent starts), clears `email` and `kitVerified`,
  and always rewrites the file with a fresh nonce and MAC. The user is told
  once (`settingsReset`) only if a protection field of the file differed from
  the stricter values or the file had `sshAgentEnabled: true`. The quick-unlock
  age the user chose applies from the reconcile after that unlock.
- `K_settings` lives in a zeroizing container while unlocked and is dropped
  on lock.
- Residual: a joint rollback of the store and `settings.json` to an earlier
  consistent pair still verifies. The full fix is to keep the settings as a
  synced record under §10 (planned with M6; THREAT_MODEL R28).

## 12. Metadata visible to the server

| Field | Why the server needs it |
|---|---|
| account email | login identifier, contact for emergency access |
| `account_id`, `account_salt`, Argon2 parameters | new-device derivation |
| OPAQUE registration record | login |
| identity public key `ipk` | sharing |
| wrapped AK (password and PRF methods, with the PRF methods' public parameters: PRF salt, RP ID, credential ID), TK wrappings of hardware-protected vaults with their public parameters, wrapped VKs, HPKE share seals | multi-device unlock, sharing |
| vault IDs, membership (`account_id` → `vault_id`, role) | access control |
| record IDs, ciphertext, padded size bucket, `server_seq`, timestamps of upload | sync |
| attachment IDs and ciphertext sizes | storage quotas |
| device IDs, device names (user-chosen, MAY be left generic), last-seen | session management |
| IP addresses (short retention, default 7 days) | rate limiting, abuse defense |
| 2FA configuration (TOTP secret for the *sync account*, WebAuthn credentials) | account 2FA |
| audit events (type, time, device — no content) | user-visible security log |

Quick-unlock wrappings (§6.2) never leave the device.

The server **cannot** see item titles, URLs, usernames, item types (beyond
size buckets), tags, vault names, or attachment names.

## 13. Randomness, generators, OTP

- Password generator (ADR-0011): every choice draws a 32-bit value from the
  OS CSPRNG (`getrandom`, read in 256-byte blocks that are wiped as they
  are used) and maps it to `0..n` by rejection sampling: draws
  `x < 2^32 mod n` are rejected, then `x mod n` is exactly uniform.
  Character-class minimums and site rules are met by generating whole
  candidates and checking them, never by forcing positions, so the output
  is uniform over the strings that qualify. Settings whose estimated
  acceptance is below 2^-10 are refused up front; at most 50 000
  candidates are tried. Passphrases come from the EFF large wordlist
  (7776 words, 12.9 bits per word); separators are printable ASCII. The
  displayed entropy is `log2(|charset|) × length` (or `12.925 × words`),
  and is shown as an upper bound whenever site rules or the separator can
  constrain the result.
- Site rules: Apple's `passwordrules` syntax (attribute values from pages,
  and the vendored Password Manager Resources quirks file, MIT). Input is
  untrusted: rule text above 4096 bytes of UTF-8 (checked before any of it
  is parsed) or with more than 32 distinct `required` sets is treated as
  malformed (no rules), as is any other malformed input. The generated
  length is at least 4 unless the site's `maxlength` is shorter, and then
  exactly that `maxlength`. The rules must also leave at least as many
  equally likely passwords as a 4-digit PIN: an alphabet of fewer than 2
  characters is refused, and a length `n` is used only if
  `n × log2(|alphabet|) + log2(p(n)) ≥ log2(10^4)` (about 13.3 bits),
  where `p(n)` is the estimated share of uniform candidates that satisfy
  the rules (exact for each kind of constraint, so a binding
  `max-repeating` counts the exact number of qualifying strings). There is
  no exception for a short site `maxlength`. Rules that only a shorter or
  weaker password can satisfy are refused, and the client generates
  without the site's rules.
- TOTP/HOTP: RFC 4226 dynamic truncation and RFC 6238 with T0 = 0. SHA-1,
  SHA-256 and SHA-512, 6 to 8 digits, period 1 to 300 s. Steam Guard: 5
  characters from `23456789BCDFGHJKMNPQRTVWXY`. Secrets are 1 to 128 bytes
  (1024 bits, the SHA-512 block size; the model refuses longer ones on write
  and decode, so every stored secret produces codes) and are stored inside
  item plaintext.
- OTP verification (account 2FA, §9.3) goes through `mosaic_totp::Otp::verify`:
  a constant-time comparison against every step in a bounded window
  (TOTP ±w steps, HOTP w counters ahead, w ≤ 10, recommended w = 1), and a
  replay guard that rejects any step at or below the last accepted one, which
  the caller stores before it accepts the login (ADR-0008).
- Security insights, reuse detection (ADR-0022): each login and Wi-Fi password is reduced to
  `HMAC-SHA-256(K, password)` under a 32-byte key `K` from the OS CSPRNG. `K` is drawn for each
  analysis and is never stored or derived from account keys. Equal tags form candidate groups,
  whose members are compared with `subtle::ConstantTimeEq`. Tags live in zeroizing memory for the
  duration of the call; nothing derived from a password leaves it.

## 14. Test requirements

- Known-answer tests: RFC 9106 Argon2id, the XChaCha20-Poly1305 draft test
  vector, RFC 5869 HKDF, RFC 4226 Appendix D, RFC 6238 Appendix B (all three
  hashes), RFC 9180 HPKE vectors (Auth mode, X25519/ChaCha20-Poly1305), and
  opaque-ke's RFC 9807 vectors.
- Property tests: encrypt/decrypt round trip for every object type. Flipping
  any bit of the ciphertext, nonce, header or AAD field causes decryption to
  fail. Changing `vault_id` or `item_id` causes decryption to fail.
- Golden-file tests for every serialized format, so format changes are visible
  in review.
- Fuzz targets for every decoder (§10, plus item and import parsers).

## 15. Browser extension channel (ADR-0036)

The browser extension talks to the desktop app through a native messaging host
(`crates/native-messaging`, `docs/NATIVE_MESSAGING.md`) that relays to a local
endpoint of the running desktop. This section is the cryptography of that
host-to-desktop link. The code is `mosaic_core::browser_channel`. The
sync server never sees any of it (§12 is unchanged).

**No key agreement.** The desktop draws a pairing and sends it in the clear
over the OS-protected endpoint after the user approved a pairing request in
the desktop app. The endpoint's OS access control (a Unix socket in a `0700`
directory with a peer-UID check, or a randomly named pipe published in a file
only the user can read) and the user's approval authenticate the pairing,
not any cryptographic exchange. A program that is not the user's cannot reach
the endpoint; a program that is (malware running as the user, P5) can also
read `K_pair` where it is stored and is not stopped by this construction
(THREAT_MODEL R38, R42, R45). Nothing here invents a primitive: HKDF-SHA-256,
HMAC-SHA-256 and XChaCha20-Poly1305 are used as in §2.

**Pairing.** At approval the desktop draws `pairing_id = random(16)` and
`K_pair = random(32)` and stores both (`K_pair` as in §4). The extension
stores them too. Up to 16 pairings exist. Removing a pairing on either side
makes the next handshake fail.

**Labels** (§3): `browser/auth`, `browser/handshake`, `browser/session`,
`browser/frame`.

**Handshake** (once per connection; `nonce_h = random(32)` by the host,
`nonce_d = random(32)` by the desktop):

```
K_auth  = HKDF(K_pair, salt = pairing_id, ctx("browser/auth"), 32)
T       = ctx("browser/handshake") || 0x00 || pairing_id || nonce_h || nonce_d
proof_d = HMAC-SHA-256(K_auth, T || 0x02)         the desktop proves first
proof_h = HMAC-SHA-256(K_auth, T || 0x01)
okm     = HKDF(K_pair, salt = pairing_id || nonce_h || nonce_d, ctx("browser/session"), 64)
K_h2d = okm[0..32]      K_d2h = okm[32..64]
```

Message order: host `hello {pairing_id, nonce_h}`, desktop `challenge
{nonce_d, proof_d}`, host `finish {proof_h}`; the desktop's first AEAD frame
is the state event. The host verifies `proof_d` before it sends `proof_h`.
Proofs, and the host's check whether a `hello` carries the pairing it already
holds, are compared with `subtle`. Any failure closes the connection. The
desktop answers an unknown `pairing_id` (at `hello`) and a wrong `proof_h`
(after `finish`) with the same message, `refused notPaired`, but not at the
same step: a dummy challenge for an unknown ID would make a host whose
pairing was removed on the desktop fail on `proof_d`, and the host learns
that the pairing is gone only from `notPaired`. The step tells a caller only
whether a 128-bit random `pairing_id` exists, which nobody can guess. The
roles are the last byte of the HMAC message
(`0x01` host, `0x02` desktop), so neither proof can be replayed as the other.
Both nonces are fresh per connection, so a recorded `hello` and `finish`
fail against a new `challenge`.

**Frames** (after the handshake, both directions):

```
frame = header || ct
header = 0x01 (version) || 0x01 (XChaCha20-Poly1305) || seq:u64 (big-endian)
ct     = XChaCha20-Poly1305(K_dir, nonce = 0^16 || seq:u64 (big-endian), plaintext,
                            AAD = header || aad("browser/frame", dir:u8, pairing_id))
```

`dir` is `0x01` host to desktop (key `K_h2d`) and `0x02` desktop to host
(key `K_d2h`). Sequence numbers start at 0 in each direction. The receiver
requires exactly the next number, so a replayed, dropped, reordered or
reflected frame fails. A frame that fails to open **poisons** the session:
every later `open` and `seal` fails with `Decryption`, so a caller that
ignores the error cannot go on using a connection that was tampered with
(the caller closes it anyway). The nonce is derived from the sequence number
instead of drawn at random (a deliberate exception to §7.2's "fresh random
nonce", ADR-0036): each key is used for one connection only, because it
depends on both connection nonces, and a sequence number is used at most
once per key. A sender whose sequence number would reach `u64::MAX` stops
(`SizeLimit`); no frame for that number exists, so a receiver at it can only
reject (`Decryption`). Every handshake or frame failure on the receiving side
is the one error `Decryption`.

**Randomness.** `pairing_id`, `K_pair`, both nonces, the Windows pipe name and
the pairing code all come from `mosaic_core`'s `random(n)` (§3, §13); the
native messaging crate has no CSPRNG of its own.

**Limits.** A frame is at most 262144 bytes on both links (the plaintext leaves
26 bytes for header and tag). The desktop closes an idle connection after
10 minutes.

**Known gaps.** No forward secrecy against a later theft of `K_pair` (traffic
is observable only by P5, who can read `K_pair` anyway), and the pairing code
shown at approval correlates the request with the user's click; it is not a
short authentication string (THREAT_MODEL R45).

**Tests.** A golden vector with fixed `K_pair`, `pairing_id` and nonces
checks `K_auth`, both proofs and both session keys against a Python
standard-library script (its source is in the test's doc comment) and pins one
sealed frame per direction. Tampering with the header, sequence number,
ciphertext or direction fails; out-of-order frames and the wrong role's key
fail; `Debug` is redacted. The decoders are fuzzed (`crates/native-messaging/fuzz`).

## 16. Passkeys (ADR-0039)

Mosaic is a software WebAuthn authenticator: ES256 only, `none` attestation,
discoverable credentials stored as `passkey` vault items. The code is
`mosaic-passkey`; it does the cryptography and the byte encodings, never
origin or rpId policy (that is `mosaic_origin::webauthn_rp_id`, ADR-0040 (lands with M5 T3),
called by the desktop). Nothing here invents a primitive: ECDSA P-256 with
SHA-256 and RFC 6979 nonces is `p256`; the key and public-key encodings are
the RustCrypto `pkcs8`, `spki` and `der` crates. The WebAuthn specification
publishes no private-key test vectors, so the checks are RFC 6979 known answers
plus an independent verifier that uses only `node:crypto` (below).

Notation as §3. `b64url` is RFC 4648 §5 base64url without padding
(`base64ct::Base64UrlUnpadded`).

**Keys.** Generate: `random(32)` (`mosaic_core`'s zeroizing secret type) read
as a P-256 scalar with `SigningKey::from_slice` (which delegates to
`p256::SecretKey::from_slice`); a zero or a value
`>= n` is rejected and redrawn, at most 8 tries, then the operation fails
(FIPS 186-5 A.4.2 rejection sampling; no `rand_core`, ADR-0011). Store:
`SigningKey::to_pkcs8_der()` (RustCrypto includes the public key in the PKCS#8
structure), copied into a zeroizing buffer, the library's own document dropped
(and zeroized). Load: `SigningKey::from_pkcs8_der`; anything that is not a
P-256 key is refused (a wrong curve, or an embedded public key that does not
match the scalar). Loading accepts a SEC1 `privateKey` of 24 to 31 bytes,
zero-padded to 32 by `SecretKey::from_slice`; T4 (import) re-encodes imported
keys so the stored form is always the 32-byte one. The key is the `private_key` field of the item's
`passkey` value (§7.5), inside the encrypted item plaintext, and is not a
`SecretField`.

**Signature.** `p256::ecdsa::SigningKey::sign(msg)`: RFC 6979 deterministic
nonce, SHA-256, DER encoding (`Signature::to_der`). No low-S normalization.
`msg = authenticatorData || SHA-256(clientDataJSON)`.

**rpIdHash** = `SHA-256(rpId)` over the bytes of the validated, canonical
(ASCII) rpId.

**clientDataJSON** (WebAuthn L3 5.8.1.1 limited verification format), byte for
byte, in this member order, without whitespace:

```
{"type":"webauthn.create"|"webauthn.get","challenge":"<b64url(challenge)>","origin":<CCD(origin)>,"crossOrigin":false}
```

`CCD(s)` is CollectedClientData's string form: `"`, then per code point
`0x20`, `0x21`, `0x23..0x5B` and `0x5D..0x10FFFF` unchanged, `0x22` as `\"`,
`0x5C` as `\\`, every other code point as `\u` and four **lowercase** hex
digits, then `"`. `crossOrigin` is always `false`; `topOrigin` is never emitted
(cross-origin frames are not served, ADR-0040, lands with M5 T3).

**authenticatorData** =
`rpIdHash (32) || flags (1) || signCount (u32, 4) [|| attestedCredentialData]`;
`attestedCredentialData = AAGUID (16 zero bytes) || u16 len(credId) || credId || COSE_Key`.
Flags: UP `0x01`, UV `0x04`, BE `0x08`, BS `0x10`, AT `0x40`; ED (`0x80`) is
never set. Get: `0x19` (UV = 0) or `0x1D` (UV = 1), 37 bytes. Create: `0x59`
or `0x5D`, 37 + 16 + 2 + 16 + 77 = 148 bytes. BE = BS = 1 because the vault
syncs: the credential is multi-device and backed up (THREAT_MODEL R48). The
AAGUID is zero so it does not identify Mosaic users. The credential ID is
`random(16)`.

**COSE_Key** (ES256), 77 bytes in CTAP2 canonical order:
`A5 01 02 03 26 20 01 21 58 20 <X:32> 22 58 20 <Y:32>` (kty EC2, alg -7,
crv P-256, x, y).

**attestationObject** (format `none`), 178 bytes for a 148-byte authData,
members in the order `fmt`, `attStmt`, `authData`:
`A3 63 "fmt" 64 "none" 67 "attStmt" A0 68 "authData" <bstr header> <authData>`,
that is `A3 63 66 6D 74 64 6E 6F 6E 65 67 61 74 74 53 74 6D 74 A0 68 61 75 74 68 44 61 74 61`
followed by the header and the data. A CBOR byte-string header for length `L`
is `0x40 | L` below 24, `58 L` below 256, `59 L (u16)` below 65536; larger is
refused. The encoder is hand-written for these three fixed structures; Rust
never decodes CBOR.

**SPKI** (`getPublicKey()`), 91 bytes: the constant prefix
`3059301306072A8648CE3D020106082A8648CE3D030107034200 || 04 || X || Y`,
built without a fallible step and checked against `to_public_key_der` in a test.

**Sign count.** A credential Mosaic creates has count 0, which is never
written. An imported counter `c > 0` signs with `c + 1`, saturating at
`u32::MAX`, and the caller persists that value
(`ItemEdit::set_passkey_sign_count`, merged by maximum, §7.5) **before** the
signature leaves. A counter of 0 stays 0. The crate enforces the order in its
types: `assert` returns a `PendingAssertion` whose signature is private, and it
is released only with a `CountPersisted` token (built after the store write) or,
for a counter of 0, with `into_response_without_count` (which refuses while a
count is pending).

**User verification.** The UV flag is set only if the desktop saw a user
verification (unlock, quick unlock or re-authentication) in the last 300
seconds and the request did not say `discouraged`; the desktop dialog click is
presence only. Residual risk: THREAT_MODEL R49.

**Randomness.** The key scalar and the credential ID come from `mosaic_core`'s
`random(n)` (§3, §13); the passkey crate has no CSPRNG of its own.

**Known-answer tests** (`crates/passkey/fixtures/vectors.json`). The key is the
RFC 6979 A.2.5 P-256 key,
`d = C9AFA9D845BA75166B5C215767B1D6934E50C3DB36E89B127B8A622B120F6721`.
V1 `cose_public_key`; V2 the SPKI; V3 and V4 the signatures over `sample` and
`test` (RFC 6979 A.2.5, SHA-256), the vectors an independent standard fixes;
V5 `rpIdHash("example.com")`; V6 and V7 the clientDataJSON for get and create;
V8 `CCD` escaping; V9 a full create (authData and attestationObject); V10 a
get (authData, signature); V11 the imported-counter rule. V1, V2 and V5 to V11
are also checked by an independent verifier written in TypeScript
(`apps/extension/test/webauthn/verifier.ts`) with its own CBOR, COSE and
authData parser and `node:crypto` verification of the ES256 signature.
V8 is also built by Rust from a hostile string (quote, backslash, U+0007,
U+007F, U+2028, U+00FC, U+1F600) and compared with an independent CCDToString
written from the WebAuthn text. Property tests: generate, store as PKCS#8,
load and sign verifies with `VerifyingKey`; the COSE bytes parsed back equal
the verifying key. Garbage, non-P-256 PKCS#8 (Ed25519, RSA, secp256k1, P-384)
and a P-256 PKCS#8 whose embedded public key does not match the scalar are
refused. Fuzz targets: `crates/passkey/fuzz` (`b64url`, `pkcs8`).
