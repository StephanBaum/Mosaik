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
| Passkeys | ECDSA P-256 (ES256); Ed25519 (EdDSA) | FIPS 186-5, RFC 8032, WebAuthn L3 | `p256`, `ed25519-dalek` |
| OTP | HMAC-SHA-1/256/512 | RFC 2104, 4226, 6238 | `hmac`, `sha1`, `sha2` |
| Breach check | SHA-1 (k-anonymity prefix only) | HIBP range API | `sha1` |
| Randomness | OS CSPRNG | — | `getrandom` (directly in `crates/core`); `rand_core::OsRng` where a `rand` API is needed (generators, §13) |
| Constant-time compare | — | — | `subtle` |
| Secret memory | zero on drop | — | `zeroize`, in Mosaic's own containers (`crates/core` `secret`; `secrecy` is not used, ADR-0007) |
| Unicode normalization | NFC | UAX #15 | `unicode-normalization` |
| TLS | TLS 1.3 only | RFC 8446 | `rustls` |

Banned: OpenSSL bindings, `ring`-only APIs outside `rustls`, anything
unmaintained (enforced in `deny.toml`). Adding a crypto dependency needs an
ADR.

## 3. Notation and encoding

- `||` is byte concatenation. Integers are unsigned big-endian.
- `UUID` values are 16 raw bytes (RFC 9562, version 7 for new IDs).
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

## 4. Secrets and their lifetimes

| Secret | Size | Where it lives | Lifetime in memory |
|---|---|---|---|
| Master password `MP` | variable | user's head | until derivation finishes |
| Secret Key `SK` | 128 bit | device OS keystore, Emergency Kit | until derivation finishes |
| Stretched password `P` | 256 bit | never stored | until derivation finishes |
| Root key `R` | 256 bit | never stored | until derivation finishes |
| Key-encryption key `KEK` | 256 bit | never stored | until AK is unwrapped |
| Account key `AK` | 256 bit | wrapped (§6) | while unlocked |
| Vault key `VK` | 256 bit | wrapped by AK (§7) | while unlocked |
| Identity key pair `(isk, ipk)` | X25519 | `isk` wrapped by AK | while unlocked |
| Attachment key `FK` | 256 bit | inside its item's plaintext | while the file is processed |

Rules for every secret type in code:

- Wrapped in a type that zeroizes on drop (`Zeroizing<[u8; N]>`,
  `SecretBox<…>`) and does **not** implement `Clone`, `Copy`, `Display` or
  `Serialize`. `Debug` prints only `"[REDACTED]"`.
- Never logged, never formatted into error messages, never in panic payloads.
  Error types carry no secret-derived data.
- Comparisons use `subtle::ConstantTimeEq`.
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
  keystore (Windows Credential Manager/DPAPI, macOS/iOS Keychain, Linux Secret
  Service, Android Keystore-wrapped).

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
| macOS / iOS | A Secure Enclave P-256 key with a biometry-or-passcode access control. `W` is encrypted with `SecKeyCreateEncryptedData` (`eciesEncryptionCofactorX963SHA256AESGCM`) and the result is stored in the Keychain. |
| Android | A Keystore AES-256-GCM key (StrongBox when available) created with `setUserAuthenticationRequired(true)` and invalidated when biometric enrollment changes. It encrypts `W`. |
| Windows | A Windows Hello `KeyCredentialManager` RSA key and a stored random 32-byte `challenge`. `W = HKDF(ikm = Sign(challenge), salt = challenge, info = ctx("hello-unlock"), 32)`. This relies on RSA PKCS#1 v1.5 signatures being deterministic, so enrollment signs twice and fails if the two signatures differ. |
| Linux | No hardware binding by default. Quick unlock is unavailable unless a FIDO2 key is used (§6.3). |

An M3 ADR pins the exact API calls, and any deviation from this table needs a
spec change. `method_id` = device ID. Quick unlock expires after a
configurable maximum age (default 14 days) or under the reboot policy. Expiry
**deletes** both the platform key and the wrapping. Flagging them as expired
is not enough.

### 6.3 Hardware-key unlock (`method = 0x03`)
Uses the WebAuthn PRF extension (CTAP2 `hmac-secret`):
```
prf_salt = random(32)   (stored with the wrapping)
W = HKDF(ikm = PRF(prf_salt), salt = credential_id_hash, info = ctx("prf-unlock"), 32)
```
`method_id` = first 16 bytes of `SHA-256(credential_id)`. Assertions MUST use
`userVerification = "required"` (PIN or biometric on the key). Without it, a
stolen key plus a stolen locked device would unlock with neither MP nor SK.
This wrapping MAY be synced, so the hardware key can unlock any device.
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

- **Plaintext** is the canonical serialization of the item (schema in
  `crates/core`, versioned by `schema_version`). It **includes** the item's
  revision and version vector, so they are authenticated. Revision metadata the
  server sees is only a routing hint (rollback handling: §10).
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
  under the target VK with the new `vault_id` in the AAD.

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
- Trash is a flag inside the plaintext plus a server-side tombstone hint. A
  hard delete removes the record.
- **Hardware-protected TOTP vault:** a normal vault whose VK is wrapped *only*
  under a hardware-key wrapping of a separate key, not under AK:
  `TK = random(32)`. For each registered hardware key, TK is wrapped under
  `W_hw = HKDF(ikm = PRF(prf_salt_hw), salt = credential_id_hash, info = ctx("prf-hw-vault"), 32)`.
  It uses its **own** `prf_salt_hw = random(32)`, never the §6.3 salt, with
  `aad("wrap/hw-vault", account_id, vault_id, method_id)`. The vault's VK is
  wrapped under TK with `aad("wrap/vault-under-tk", vault_id, key_generation:u32)`.
  `userVerification = "required"` applies here too. Trade-off shown in
  the UI: "Codes in this vault need your security key every time, so a stolen
  master password alone is not enough, but losing every registered key loses
  these codes."

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
vault, re-share the new VK with the remaining members, and increment
`key_generation`. Until the rotation finishes, the removed member keeps
whatever they already downloaded. This cannot be avoided and is stated in the
UI.

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
  vector it has accepted. A record whose authenticated vector is dominated by
  the stored one is ignored. A record that is concurrent with the stored one
  goes to conflict resolution. The server can withhold updates (freeze attack)
  but cannot forge or roll back. Freeze detection: every device publishes a
  checkpoint
  `HMAC-SHA-256(K_cp, ctx("checkpoint") || device_id || counter:u64 || SHA-256(canonical seen vectors))`,
  where `K_cp = HKDF(AK, "", ctx("checkpoint-key"), 32)` and `counter` is a
  per-device monotonic counter. Other devices warn when the checkpoints they receive are
  older than their own state for longer than 24 h.
- **Deletions** are encrypted tombstones, so the server cannot forge a delete.
- Message parsing on both ends is strict and length-bounded, and is covered by
  fuzz targets.

## 11. Local storage

- The local SQLite database holds the same ciphertext records as the server,
  plus the wrapped keys and the Argon2 parameters. Item plaintext, titles and
  URLs are never written to disk. The search index exists only in memory while
  unlocked.
- Locking zeroizes AK, every VK, `isk` and the decrypted item cache.
- Auto-lock triggers: idle timeout (default 10 min), OS sleep, screen lock,
  user switch, and app quit. All configurable, but they cannot all be
  disabled.
- Encrypted export: an `age`-compatible file with a passphrase recipient
  (scrypt) is under evaluation. Until an ADR decides, export uses the
  item-record format under a fresh key derived as in §5.3 from an export
  passphrase.

## 12. Metadata visible to the server

| Field | Why the server needs it |
|---|---|
| account email | login identifier, contact for emergency access |
| `account_id`, `account_salt`, Argon2 parameters | new-device derivation |
| OPAQUE registration record | login |
| identity public key `ipk` | sharing |
| wrapped AK (password and PRF methods), wrapped VKs, HPKE share seals | multi-device unlock, sharing |
| vault IDs, membership (`account_id` → `vault_id`, role) | access control |
| record IDs, ciphertext, padded size bucket, `server_seq`, timestamps of upload | sync |
| attachment IDs and ciphertext sizes | storage quotas |
| device IDs, device names (user-chosen, MAY be left generic), last-seen | session management |
| IP addresses (short retention, default 7 days) | rate limiting, abuse defense |
| 2FA configuration (TOTP secret for the *sync account*, WebAuthn credentials) | account 2FA |
| audit events (type, time, device — no content) | user-visible security log |

The server **cannot** see item titles, URLs, usernames, item types (beyond
size buckets), tags, vault names, or attachment names.

## 13. Randomness, generators, OTP

- Password generator: uniform choice from the character set by rejection
  sampling (`rand::distr::Uniform` over `OsRng`, unbiased). Passphrases come
  from the EFF large wordlist (7776 words, 12.9 bits per word). The displayed
  entropy is `log2(|charset|) × length` (or `12.925 × words`), and is shown as
  an upper bound whenever site rules constrain the result.
- TOTP/HOTP: RFC 4226 dynamic truncation and RFC 6238 with T0 = 0. SHA-1,
  SHA-256 and SHA-512, 6 to 8 digits, period 1 to 300 s. Steam Guard: 5
  characters from `23456789BCDFGHJKMNPQRTVWXY`. Secrets are stored inside item
  plaintext.

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
