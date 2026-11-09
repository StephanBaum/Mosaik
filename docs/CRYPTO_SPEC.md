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
| Public-key encryption (sharing, emergency access) | HPKE, mode Auth, DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, ChaCha20-Poly1305 | RFC 9180 | `hpke` (exact `=0.15.1` pin, features `alloc`, `x25519`, `chacha`, nothing else; mode Auth through the single-shot API only; the ephemeral key is one `random(32)` draw, ADR-0049) |
| Account login | OPAQUE-3DH, ristretto255, SHA-512 | RFC 9807 | `opaque-ke` (4.x, exact `=` pin, ADR-0046) |
| Passkeys (§16, ADR-0039) | ECDSA P-256 / SHA-256 (ES256) with RFC 6979 nonces; Ed25519 (EdDSA) reserved, not implemented | FIPS 186-5, RFC 6979, RFC 8032, WebAuthn L3 | `p256` (features `alloc`, `ecdsa`, `pkcs8`; encodings from `pkcs8`, `spki`, `der`); `sha2` (rpIdHash, clientDataJSON hash); `base64ct` (base64url) |
| OTP | HMAC-SHA-1/256/512 | RFC 2104, 4226, 6238 | `hmac`, `sha1`, `sha2` |
| Settings integrity (§11.1, ADR-0030) | HMAC-SHA-256 under an HKDF-derived key | RFC 2104, 5869 | `hmac`, `hkdf`, `sha2` |
| Sync checkpoints and folder backend (§10, §10.1, ADR-0043) | HMAC-SHA-256 under HKDF-derived keys; XChaCha20-Poly1305 for device info | RFC 2104, 5869, draft-irtf-cfrg-xchacha | `hmac`, `hkdf`, `sha2`, `chacha20poly1305` |
| Browser extension channel (§15, ADR-0036) | HKDF-SHA-256, HMAC-SHA-256, XChaCha20-Poly1305 | RFC 5869, RFC 2104, draft-irtf-cfrg-xchacha | `hkdf`, `hmac`, `sha2`, `chacha20poly1305` |
| Breach check | SHA-1 (k-anonymity prefix only) | HIBP range API | `sha1` |
| Randomness | OS CSPRNG | — | `getrandom`, called directly in `crates/core` and `crates/generator` (no direct `rand` / `rand_core` use, ADR-0011; `hpke` brings `rand_core` as a transitive dependency and is fed from a buffered `getrandom` draw, ADR-0049) |
| Constant-time compare | — | — | `subtle` |
| Secret memory | zero on drop | — | `zeroize`, in Mosaic's own containers (`crates/core` `secret`; `secrecy` is not used, ADR-0007) |
| Unicode normalization | NFC | UAX #15 | `unicode-normalization` |
| TLS (client) | TLS 1.3 only, OS trust store | RFC 8446 | `rustls` with the `ring` provider, through `reqwest` blocking (ADR-0046); the server runs behind a reverse proxy that terminates TLS |

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
- Labels added for sync (§10, §10.1): `checkpoint-key`, `checkpoint`, `session`,
  `sync/folder-mac-key`, `sync/folder-segment`, `sync/folder-account`,
  `sync/folder-blob-delete`, `sync/device-info-key`, `sync/device-info`,
  `sync/view` and `sync/view-key` (the plain SHA-256 prefixes of the view digest
  and its key commitment, §10); and, in the server only, `server/prelogin-key`,
  `server/totp-key` and `server/totp-at-rest` (§9.2, §9.3). `mosaic-core` derives
  every key from them; `mosaic-sync` and the server never see `AK`.
- Labels added for sharing and emergency access (§8, M6b): `wrap/identity` (§8.1),
  `share/vault`, `fingerprint`, `vault-roster` (§8.2), `emergency` (§8.4), `contacts-key`,
  `contacts` (§8.5), `sync/view2` and `sync/view-meta` (the plain SHA-256 prefixes of the
  version 2 view digest and its metadata commitment, §10); and, in the server only,
  `server/directory-key` and `server/directory-ipk` (§8.2, ADR-0052, ADR-0055). `sync/view`
  is **retired** (version 1 of the view digest, never released with sharing): it stays in the
  registry, so it can't be reused for another purpose.
- Label added for key rotation (§6.6, §10.1, M7): `sync/folder-rekey` (the MAC prefix of a
  folder re-key marker, ADR-0060). No other label, key or AEAD use is new in M7.
- **Algorithm IDs** in object headers: `0x01` XChaCha20-Poly1305 (every envelope of §6 to §7),
  `0x03` the HPKE seal of §8.2 and §8.4 (mode Auth, KEM `0x0020`, KDF `0x0001`, AEAD `0x0003`).
  `0x02` is unused. A decoder refuses any other value.
- **One registry.** Every label that any crate uses is a typed constant
  `mosaic_core::encoding::label::*` and is listed in `label::ALL`, which the
  uniqueness test covers; `mosaic-sync` and the server take their labels from
  there (the server's `subkey()` takes the typed label) and hold no label
  string literal. This includes `opaque-login` (the OPAQUE context, §9.1) and
  the three pre-login labels below.
- **Exception: the pre-login labels.** `prelogin/account-id`, `prelogin/salt` and
  `prelogin/profile` (§9.2) are fed to the HMAC as the raw label bytes, **without**
  the `mosaic/v1/` prefix (`HMAC(K_prelogin, label || normalized_email)`). They
  are registered and unique like the others, but their byte values are fixed as
  they are: a change would change every fake the server has answered with, and
  a returning client would see its fake salt change. The exception is
  deliberate, not a rule for new labels.
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
| Identity key pair `(isk, ipk)` (§8.1) | X25519 | `isk` wrapped by AK (`wrap/identity`); `ipk` public | `isk` while unlocked; an HPKE private key exists for one seal or open only |
| Contact-book key `K_contacts` (§8.5) | 256 bit | never stored | derived per operation inside the session; gone on lock |
| Opened share VK (§8.2, pending slot) | 256 bit | memory only | from opening an invite until it is accepted or dropped, at most 5 minutes, and gone on lock |
| Attachment key `FK` | 256 bit | inside its item's plaintext | while the file is processed |
| Quick-unlock `W` | 256 bit | released by the platform key (§6.2), never stored in the clear | until AK is wrapped (enrollment) or unwrapped (unlock) |
| Security-key PRF output | 256 bit | on the security key | until `W` or `W_hw` is derived |
| `W_hw` (§7.4) | 256 bit | never stored | until `TK` is wrapped or unwrapped |
| Hardware-vault key `TK` | 256 bit | wrapped per security key (§7.4) | while the hardware-protected vault is open, at most 5 minutes |
| Settings MAC key `K_settings` (§11.1) | 256 bit | never stored | while unlocked |
| Checkpoint MAC key `K_cp` (§10) | 256 bit | never stored | derived per operation inside the session; gone on lock |
| Folder MAC key `K_fold` (§10.1) | 256 bit | never stored | derived per operation inside the session; gone on lock |
| Device-info key `K_dev` (§10.1) | 256 bit | never stored | derived per operation inside the session; gone on lock |
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
  and stores the new profile. Profiles only ever increase. An upgrade is
  the §6.4 flow with the same password and a stronger profile (one prepare
  path, `change_credentials`; always a new salt). Offers never go above
  `standard` on their own; `strong` and `max` only by explicit user choice, with
  a slow-device warning and a guard when this device needs more than 2 s
  (ADR-0062). A device that has not seen the upgrade yet recovers silently at
  its next password unlock: the typed password, the stored Secret Key and the
  backend's parameters are tried first (§9.4 "Change applied elsewhere"), so a
  change the user already knows costs no extra prompt.
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
One function builds every credential change (`change_credentials`: §6.4, §6.5
and §5.3 share it). It always draws a new `account_salt`, derives `KEK'` and
`OPW'` from the new (or unchanged) password, the new (or unchanged) Secret Key
and the new (or unchanged) profile under the current account ID, and re-wraps
the same AK. Items and vault keys are not touched. The profile is never
weaker than the current one (§5.3); the client refuses a downgrade before any
Argon2id work. `change_credentials` takes the stored password wrapping and
verifies the current password and Secret Key against it (it must open to the
session's AK) before it produces anything, so a mistyped current password
yields nothing. A new salt on every change means the retired-salt rule (§9.4)
also catches a rollback to a salt this device replaced after a Secret Key
rotation or a profile upgrade (it covers only salts this device replaced,
never a salt it did not see).
The client re-registers OPAQUE with `OPW'` and the backend replaces the old
password wrapping atomically (§9.4). The quick-unlock wrapping is deleted and
other sessions end; hardware-key wrappings stay (they do not depend on `P`).
No new Emergency Kit is issued.

### 6.5 Secret Key rotation
Same flow as §6.4 with a new SK. A new Emergency Kit is issued, and the client
saves and verifies it (the user types the last group) **before** the backend
learns the change, so a failed save never leaves the account behind a Secret
Key nobody has. Until the commit the new SK sits in the OS keystore under a
pending name next to the current one (THREAT_MODEL R88); a job abandoned before the
backend has seen it deletes the pending entry. When a Secret Key is replaced
(rotation, re-key, adoption), the device stores its **retired commitment**
`SHA-256(ctx("secret-key/retired") || 0x00 || account_id || SK)` (no key is
kept; schema v9) and never removes it. A device that lost the answer or
was signed out recovers with the typed password and, when that fails, with
the Secret Key from the newest kit.

**What §6.4 and §6.5 do not do:** they stop *server login* with the old
secrets, nothing more. AK does not change. Anyone holding the old MP and SK
plus any old copy of the password wrapping (a server backup, an old local
database) can still recover AK and every VK. When a user rotates because of a
suspected compromise, the UI MUST say this and offer §6.6. The sentence is:
"This stops sign-in with the old Secret Key. Anyone who already copied your
vault data with your old password and Secret Key can still read it. Re-key the
account to stop that."

### 6.6 Full re-key (compromise recovery)

**What changes** (`=` same, `>=` not weaker):

| Operation | AK | SK | Salt | Profile | VKs | Identity | Quick unlock | HW unlock (0x03) | Other sessions | New kit |
|---|---|---|---|---|---|---|---|---|---|---|
| Password change §6.4 | = | = | new | >= | = | = | deleted | kept | ended | no |
| SK rotation §6.5 | = | new | new | >= | = | = | deleted | kept | ended | yes |
| Profile upgrade §5.3 | = | = | new | > | = | = | deleted | kept | ended | no |
| Vault rotation §7.1 | = | = | = | = | that vault `g+1` | = | kept | kept | kept | no |
| Full re-key | **new** | **new** | new | >= | every owned vault `g+1`, member vaults re-wrapped | **new** | deleted | **deleted** | ended | yes |

**Steps** (`rekey_account`, `prepare_rekey`, `commit_rekey`):
1. Preconditions: the device is synced to the head with no halt, finding or
   pending revocation. The new password differs from the current one (checked
   with one derivation under the current parameters) and the new Secret Key
   differs from the current one (constant-time comparison); the profile is not
   weaker. The new Emergency Kit is saved and verified first (§6.5).
2. Generate a new AK (independent of the old one: every key derived from AK,
   `K_fold`, `K_cp`, `K_dev`, `K_settings` and `K_contacts`, changes with it), a new
   identity key pair, and a new VK of generation `g+1` for every vault the user
   owns. Wrap the new AK under `KEK'` from the new password, the new SK, a new
   salt and the profile.
3. One local store transaction (`commit_rekey`): new parameters and password
   wrapping, `ak_epoch + 1` (§6.7), the old salt retired, **every** held VK of
   every generation re-wrapped under the new AK (local history stays readable),
   the new VKs stored, every readable record, metadata and roster of an owned
   vault re-sealed under `g+1` and queued, wrappings of member vaults re-wrapped,
   the identity replaced, the contact book (§8.5) re-sealed under the new
   `K_contacts` and queued, the quick-unlock wrapping and **every**
   hardware-key wrapping deleted (the app deletes the platform keys; TK vaults
   are not touched, because a compromised AK does not reveal TK, §7.4),
   peer checkpoints, view comparisons and lag timers cleared (they belong to the
   old epoch; own counters, `reserved` and `written` stay), one attachment
   re-seal job per attachment of an owned vault, and the WAL scrubbed.
4. Attachments are re-sealed one per transaction under a fresh attachment ID and
   key in the same vault (§7.3); the old ID is deleted once the referencing
   record is accepted. A blob that is neither local nor fetchable is listed as
   unreachable and keeps its old key (THREAT_MODEL R85).
5. Issue the new Emergency Kit, revoke all other sessions.

**Phases and resumption.** A journal row (never a secret) records the phase:
1 prepared (kit verified, pending SK in the keystore), 2 backend started
(hosted), 3 backend accepted (hosted), 4 folder written, 5 local committed,
6 attachments, 7 uploading (ADR-0061). A crash resumes from the row. After phase 3 the
prepared keys died with the process: the device adopts its own re-key like
another device would (§6.7) with the pending SK and the typed new password; the
user retypes the last group of the new kit first, so resuming never trusts the
keystore entry alone. The pending SK is deleted once a probe proves the backend did
not apply the change, and when the job ends either way. A resumed job that adopted
its own re-key (phase 2 or 3) queues the attachments of every owned vault again
(the prepared queue died with the process), moves to phase 6 and continues, so no
attachment keeps the old file key. The phase-2 journal write comes before anything
reaches the backend: if it fails, the request is not sent. An attachment whose
ciphertext does not open under its file key is listed as unreachable, skipped, and
never blocks the job. A change of backend is refused while a job exists; once the
local part is committed (phase 4 or later) the user can abandon the job after a
confirmation, and the attachments still to re-seal keep their old key and are
listed (THREAT_MODEL R85). Phases 1 to 3 are not abandoned that way: the backend may
already know the change.

**Hosted.** One atomic request (`/v1/account/rekey/start`, `finish`) under a
fresh login with the current `OPW`. `finish` carries the new OPAQUE record,
wrapped AK, salt, profile, the target epoch (`current + 1`), the new identity
(the only place the server accepts an identity replacement, ADR-0059), the new
VK of every owned vault with its expected head, for each owned shared vault the
roster at `g+1` and fresh seals from the **new** `isk` to every remaining member
and pending invitee, the re-wrapped current key of every vault the account is
a member of **and holds the current key of** (an unclaimed share, an unavailable
vault or a generation not pulled yet is skipped: nothing is sent for it, and the
server expects exactly the member vaults for which the account has a key row of
the current generation), and fresh emergency seals for every grant it gave. The
owner's plan is strict (nobody is locked out): a member key that is not verified,
or does not match the roster, stops the re-key with `memberKeyMismatch` and records
**no** finding, because a finding would block the retry (step 1). The coverage
rules decide the answer: the vault set must be exactly the owned set, each at
`current + 1` with the expected head, else `generation_taken` or `head_moved` and
nothing applies. The server applies everything in one transaction, deletes the
account's older-generation key rows, key shares to it and emergency seals
addressed to it, cancels its incoming pending invites, files a rotation request
for each owner of a vault it is a member of, ends every other session and
audits it. `rekey/finish` also ends pending OPAQUE and second-factor logins,
enforces the version-at-login rule of §9.4, and deletes server-held
hardware-key (PRF) AK wrappings, which belong to the old AK (this build stores none on the server, §12). A lost answer is
settled by a login with the new `OPW`: if the
wrapped AK equals the prepared one byte for byte and the epoch is the target,
the device commits; otherwise the next resume probes again or adopts.

**Other hosted devices** are signed out. Sign-in with the old SK fails; the
user types the new Secret Key and the password, and the device logs in,
fetches the account (with `ak_epoch` and the wrapped `isk`) and adopts (§6.7). The
adoption needs no clean sync: it lists the vaults shared to the account by
ownership alone and skips a vault of unknown owner or without its current key.
After an adoption every owned vault is **held** at the generation the device has:
the device keeps editing locally but pushes nothing of that vault until the key of
the next generation arrives by pull (every genuine re-key rotates every owned
vault), so no record is sealed under the retired key. Records sealed below the new
generation are re-sealed by the rule of §7.1.

**Shared vaults.** Only the owner rotates a shared vault (§8.3). The owner's
re-key rotates each owned shared vault inside the request; members' clients see
the owner's new `ipk`, `ContactChanged` blocks every share from the owner until
the member compares and accepts the new code (§8.5), then the claim proceeds
(THREAT_MODEL R84). Vaults shared **to** the re-keying user stay readable to the
old AK holder until their owner rotates: the request files a rotation request,
and the owner's devices queue a revocation with reason `MemberRekeyed` (nobody
is revoked) and rotate on their next run, sealing to the member's new `ipk` once
it is verified (THREAT_MODEL R86). Emergency grants as grantee: the server
deletes the seals addressed to the account, and each grantor re-seals on the
next run after verifying the new code. Contact pins of the re-keying account
are kept.

**Target epoch.** A re-key moves to `max(own epoch, highest epoch seen in the
backend) + 1` with checked arithmetic (a device halted on a verified marker
re-keys past it). An unverifiable account file counts only at exactly `own + 1`
(its header is unauthenticated, so a planted huge epoch cannot push the target out
of reach), and a dismissed file or marker counts for nothing. The folder adoption
jumps over epochs (§6.7), so a re-key that targeted `own + 5` is still adopted.

**Ownership.** Re-key needs to know which vaults the account owns. A vault
whose owner nothing records counts as owned if this device created it, or if
the backend is a **folder**: sharing is impossible there, so a device that
joined the folder re-keys every vault. On **hosted** it stays unknown and the
re-key refuses (`OwnerUnknown`, the fail-closed rule of §8.3).

**Re-keyed members.** When a member re-keyed, the owner's verified acceptance
of the member's new safety code (`verify_contact` after `accept_key_change`)
writes that fingerprint into the member's roster entry of every vault the owner
owns, at the vault's current generation, queued for upload. Accepting the key
alone writes nothing, and only the owner's device writes it, so the next
rotation's served-key check passes only for a code the owner compared.

**Folder.** See §10.1 "Re-key": an account file at the next version with the new
epoch, a full snapshot set under the new `K_fold`, a marker for devices of the
old epoch.

Settings integrity (§11.1) uses `K_settings`, which changes with AK: the app
re-MACs `settings.json` right after a commit or adoption.

Shared vaults owned by other people can only be rotated by their owners
(§8.3). The UI lists those vaults.

### 6.7 Account-key epoch and re-key adoption
The **account-key epoch** `ak_epoch: u32` starts at 1 and rises by exactly one
per full re-key. It is public metadata (§12) and names which AK a password
wrapping belongs to. It is never an input to a key derivation: different epochs
have independent random AKs. The store keeps it in the account row and a
trigger refuses a decrease.

A device compares the backend's epoch with its own:
- **Hosted.** The epoch is served by the backend and is not authenticated; a
  malicious backend can lie about it (THREAT_MODEL R90). Each sync run reads it
  (`GET /v1/account?epoch=1`) and a lower epoch is a rollback, refused and
  reported as an `AccountRollback` finding on every run. The same epoch with a
  wrapping that opens to a different AK is refused as an unlock failure (§9.4).
  Only `current + 1` can be adopted (checked arithmetic: `u32::MAX` is refused);
  any other higher epoch is a finding and moves nothing. A re-key re-reads the
  epoch after its fresh login and stops (`epoch_mismatch`) unless it is the one
  the re-key leaves.
- **Folder.** The backend epoch is the highest epoch of a **verified** account
  file; files of a lower epoch are ignored, not reported as a rollback (a holder
  of the old AK can plant them, R83). A file of a higher epoch that cannot be
  verified is a dismissible finding, not an adoption and not a halt (§10.1).
  **Any** epoch above the own one can be adopted, not only `current + 1`: a
  re-key targets `max(own, highest seen) + 1` (so a planted `own + 5` header
  can push a genuine re-key up), and a device can be two re-keys behind. The
  typed Secret Key is the authenticator: the file must open under keys derived
  from it and the password, its MAC must verify under the key it holds, and the
  Secret Key must not be the current one or a retired one.
- **Joining.** A device that joins stores the backend's epoch with the account:
  the verified account file's epoch on a folder, the served `akEpoch` on hosted
  (a claim, usable only because the wrapping opens under the typed keys). The
  stored epoch only rises. A device that switches a backend keeps the epoch of
  its own AK: a fresh hosted registration carries it (`akEpoch` in `register/finish`, at
  least 1), so folder and server agree and no `AccountRollback` follows.
  Without this, a device that joined after a re-key would target epoch 2 again.

There is **no adoption at an equal epoch.** Two concurrent re-keys are resolved
only by re-keying again, from a device that holds the current AK, to
`max(seen epochs) + 1`.

Adoption requires: the epoch is exactly `current + 1` (hosted) or above the
current one (folder); the wrapping's salt is
not retired; the wrapping opens under keys derived from the typed password and
the typed Secret Key; **the typed Secret Key differs from the current one and
from every retired commitment** (constant-time comparison over all of them). A
genuine re-key always rotates SK, so this stops a holder of the old password and
any old Secret Key from moving the device to an AK they made up, even when a
concurrent or replayed re-key is offered. There is no endorsement by the old AK,
because the old AK is what a compromise exposes (ADR-0058). Adoption is one
store transaction like the local part of the commit (§6.6 step 3): new
parameters and epoch, the Secret Key it replaces retired, history generations
of every held VK re-wrapped under the new AK, the identity replaced from the
backend's wrapped `isk` only after the derived `ipk` equals the served one, the
contact book re-sealed, quick and hardware unlock deleted, peer state cleared.
For the current generation of each member vault the device stores **the
server's own wrapping**, after checking in constant time that it opens to the
same VK as the held one; re-wrapping it locally would change the bytes the view
digest commits to and cause a false `ServerInconsistent` on the other devices.
It creates no new VK: the backend's `g+1` keys arrive by pull under the new AK,
and records this device sealed below them are re-sealed by the rule of §7.1.
Other devices show an in-app notice that a re-key happened elsewhere (there is
no email by design; THREAT_MODEL R91).

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

A shared vault (§8.2) has one more object of the same construction under its own label, the
member roster:
```
VaultRoster = header || nonce[24] || AEAD(VK, nonce, pad(roster), aad)
header      = version:u8 || alg:u8 || key_generation:u32
aad         = header || aad("vault-roster", vault_id, key_generation:u32)
```
The padded roster is at most 64 KiB. Its fields, merge rule and validity are in §8.2; membership
roles are `owner`, `manage`, `write` and `read` (server numbers 1 to 4).

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

Sharing and emergency access exist **on the hosted server only** (§8.6, ADR-0048).
Every construction below uses the primitives of §2 and no others.

### 8.1 Identity keys
Each account has an X25519 key pair `(isk, ipk)` for HPKE (DHKEM(X25519,
HKDF-SHA256), RFC 9180 §7.1.3).

- **Generation.** `ikm = random(32)`, then `(isk, ipk) = DeriveKeyPair(ikm)`
  (RFC 9180 §7.1.3). `ikm` is zeroized after the call. The 32 bytes kept as `isk` are
  the clamped scalar, which is how the `hpke` crate serializes the private key.
- **Storage.** `WrappedIdentityKey = WrappedKey` (§6 format, 74 bytes) with `W = AK` and
  `aad = header || aad("wrap/identity", account_id)`. `isk` is in memory only while the
  vault is unlocked (§11). `ipk` is public.
- **When.** Lazily, at the first hosted sync run of an unlocked session that has none.
  A folder store never generates one (§8.6).
- **Publication.** `ipk` and the wrapped `isk` are published on the hosted server, which
  stores both if none is set; the same pair again is accepted; a different pair is refused
  (`identity_exists`) and the stored pair is returned. A device **adopts** the served pair
  only if (a) its own key was never published, (b) the served wrapping opens under AK with
  the AAD above, and (c) the public key derived from the opened `isk` equals the served
  `ipk` (constant time). Otherwise it records the finding `IdentityMismatch` and sharing
  stays disabled until the user acknowledges. Adoption is safe because an unpublished key
  was never used for any seal. Every run compares the served identity with the local one;
  a difference after publication is `IdentityMismatch`.
- **No replacement before M7.** The server refuses to replace a published identity key. It
  changes only with a full re-key (§6.6, M7), which also invalidates the seals made with it.

### 8.2 Sharing a vault

Sharing is account to account on one hosted server. A "team" is a shared vault with
several members. There are no per-item ACLs: sharing an item means moving it into a shared
vault (§7.2, "Moving an item").

**Fingerprint and safety code.**
```
fpr = SHA-256(ctx("fingerprint") || account_id || ipk)        (32 bytes, stored in full)
```
The **safety code** is the first 130 bits of `fpr` as 26 Crockford Base32 characters
(alphabet `0123456789ABCDEFGHJKMNPQRSTVWXYZ`, most significant bits first). It is displayed in
groups `5-5-5-5-6` (26 characters do not split into groups of 5). Comparison accepts any
case, ignores spaces and hyphens and accepts the Crockford aliases (`I`, `L` for `1`, `O`
for `0`). The desktop has no camera, so verification is by reading the code aloud or on
the phone. The QR payload for mobile (M10) is `mosaic-contact:1:<account_id hex32>:<fpr hex64>`;
M6b defines it and does not use it.

**Directory fakes.** For an unknown address the directory's `ipk` is
`DeriveKeyPair(HMAC-SHA-256(K_dir, ctx("server/directory-ipk") || normalized_email)).public`
(RFC 9180 §7.1.3, the derivation of real identity keys) with
`K_dir = HKDF(server_secret, "", ctx("server/directory-key"), 32)`, so a fake is a valid X25519
key in the prime-order subgroup like every real `ipk` and can't be told apart by a validity
check (ADR-0055).

**ShareSeal.** For each recipient the sender seals VK with HPKE mode **Auth**, using the
sender's `isk` as the authentication key, the suite fixed by §2 (KEM `0x0020`, KDF `0x0001`,
AEAD `0x0003`):
```
ShareSeal (82 bytes) = version:u8 (1) || alg:u8 (0x03) || enc[32] || ct[48]
ct   = HPKE.SealAuth(pkR = ipk_recipient, info, aad, pt = VK[32], skS = isk_sender)
info = ctx("share/vault") || vault_id || sender_account_id || recipient_account_id
aad  = header || aad("share/vault", vault_id, key_generation:u32)        header = version || alg
```
`alg = 0x03` names "HPKE mode Auth with the §2 suite". A decoder rejects a version other
than 1, an alg other than `0x03` and any length other than 82 before any crypto; every
failure to open is the single `Decryption` error. The seal draws one `random(32)` for the
ephemeral key (§2: the OS CSPRNG, ADR-0049). The recipient opens the seal with their `isk`
and the sender's `ipk`, then re-wraps VK under their own AK (§7.1). Auth mode stops the
server from injecting a vault key it made up and claiming it came from a sender.

**Roles.** `owner` (1), `manage` (2), `write` (3), `read` (4) on the wire and in the server.
Owner: everything, can't be removed or leave, and the **only** role that issues rotation
shares (§8.3). Manage: invite (role at most its own; an invite carries the current key, so it
makes no new generation), change roles below `owner`, request a removal, write the roster.
Write: push records, metadata,
attachments. Read: pull. The only **cryptographic** line is "holds the current VK or not".
Within the holders every member can create validly encrypted items, metadata and rosters.
Read-only roles are enforced by the server only, and so is the roster: a read member, or a
write member who promotes themself in the roster, needs a malicious or compromised server
to get that write accepted (THREAT_MODEL R72). The one cryptographic role check is the claim
rule below: a rotation share is accepted only from the vault owner, whose account is fixed
when the vault is accepted and whose fingerprint is the verified contact pin. No roster
entry can grant rotation authority.

**Roster.** A per-vault replicated document (§7.5 layout) sealed under VK,
```
VaultRosterCiphertext = header || nonce[24] || AEAD(VK, nonce, pad(roster), aad)
header = version:u8 || alg:u8 || key_generation:u32
aad    = header || aad("vault-roster", vault_id, key_generation:u32)
```
(exactly the `VaultMeta` construction of §7.1; padded plaintext at most 64 KiB). Its keys are
`member:<account_id as 32 lowercase hex digits>` and its values
`{"email":E,"fpr":HEX64,"role":R,"since":G}` with `R` one of `owner`, `manage`, `write`,
`read`, `removed` and `G` a key generation (u32, at least 1).
- **Bound to its generation.** A roster belongs to the key generation it is sealed under.
  Two rosters merge only if they have the same generation (anything else is an error); the
  current roster never merges a roster of another generation, and older rosters are
  read-only inputs. A rotation moves the roster to `g + 1` explicitly (the content is carried
  over and the removals are written at `g + 1`).
- **Owner entry.** The owner's account is fixed when the roster is created (the vault's
  owner). A device that has no sharing row for the vault accepts a first roster only on
  positive evidence from the account's contact book (§8.5): an `owned:` entry for the vault
  and the roster's owner is this account, or a `member:` entry whose owner is the roster's
  owner. A vault with neither entry (or both) is of unknown ownership and is HELD: no roster
  or share is applied, a `VaultHeld` finding is recorded and the roster is fetched again on
  later syncs, once the contact book brings an entry (ADR-0056). There is no default owner
  for a vault without a sharing row, and neither a verified contact pin nor the server's
  grant list is a substitute. The engine also refuses the roster when the owner is an
  emergency grantee of this account. The owner's entry is the one written at creation:
  later writes to `member:<owner>` are ignored and never make the roster invalid.
- **Merge rule** (ADR-0050). The merge of two replicas is the document merge of §10. The
  *effective* value of a `member:*` register that holds several concurrent values is the one
  with the highest `since`; on a tie the most restrictive role (`removed` > `read` > `write` >
  `manage` > `owner`); then the largest stamp. Member registers are never shown as
  conflicts. A removal is a write of `role:"removed"` with `since = g + 1`, the generation of
  the rotation that locks the member out, never a deletion of the key. A concurrent role
  change made at `g` therefore cannot bring a removed member back, and a re-invite writes
  `since = ` the then-current generation, which is higher than the removal.
- **Validity**, checked on every open over every entry of every register: every member value
  parses; `fpr` is 32 bytes; `since` is at most the roster's own generation; the owner's
  register exists, and no other account has the role `owner`. The owner is the account the
  server reports as the vault's owner (which never changes after creation).
  An invalid roster raises the finding `RosterInvalid`: the vault stays readable, and no
  rotation share is accepted for it.
- **Created** by the owner's first invite: the owner's entry plus the invitee's. A vault that
  was never shared has no roster.
- **Server**: keeps one current roster per vault plus the last roster of every generation.
  Only `owner` and `manage` may write it, by compare-and-swap, sealed under the current
  generation.

**Invite and accept.** Sender A (role `owner` or `manage`), recipient B, vault V at
generation g:
1. A looks B up by email at the directory, which returns `(account_id, ipk)`. A pins B's `fpr`
   (§8.5), shows B's safety code, and requires the pin to be **verified** (the user compared
   the codes and confirmed). No seal is made to an unverified contact.
2. A writes `member:B = {email, fpr_B, role, since: g}` into the roster (creating it), seals
   `VK_g` to B as a `ShareSeal`, pushes the roster **first**, then stores the invite.
3. B lists the invites. The preview fetches A's `ipk` by account ID, checks A's pin (an
   unknown A is pinned unverified and B is shown A's code), opens the seal, then decrypts the
   vault metadata and the roster with `VK_g` from the objects the server serves to a
   pending invitee read-only. The preview fails unless the roster lists B with the invited
   role, a `since` of at most g, and B's own `fpr`. The opened VK waits in a pending slot
   bound to the session for at most 5 minutes.
4. B accepts with a user gesture and a verified pin of A, re-wraps `VK_g` under `AK_B` (§7.1)
   and the server adds the membership. The accept carries the generation `g` of the wrapped
   key; the server answers `invite_stale` unless `g` equals both the invite's generation and
   the vault's current one, so a wrapping of an older key can't be filed under a newer
   generation after a rotation refreshed the seal (ADR-0055). When the vault's owner is not
   A, the pin of the owner (named by the roster, fixed at accept) must be **verified** as
   well, because later rotation shares are accepted only from the owner (§8.3). The accept
   and the preview refuse a vault ID this device already knows: an open vault, a generation
   floor left by a deleted one, or a sharing row; the one exception is a vault whose access
   was lost, offered again by the same owner. The next run pulls the vault from the start.
5. If the vault rotated since the invite, the accept fails (`invite_stale`); A's next rotation
   refreshes every pending invite (§8.3), so the sender never re-invites by hand. An opened
   invite key is dropped from memory after its five minutes (not only refused).
6. Declining, or the sender cancelling an invite, after the seal was delivered leaves B
   holding `VK_g`: A's devices queue a revocation of B (§8.3).

**Required verification and key changes.** The first use of a contact (invite, grant,
accepting an invite) needs a verified pin. The one exception is opening an emergency release
from an unverified grantor, which is allowed after a warning (§8.4). A **key change** (the
server serves an `ipk` whose `fpr` differs from the pin) or conflicting pins blocks every
seal to that account and every seal from it until the user compares the new code and accepts
it; the new pin then starts unverified. Until a full re-key (M7) exists, any change is
presumed hostile and the dialog says so. The roster adds cross-member detection: every
member's client checks the served `ipk` of every other member against the `fpr` the inviting
manager wrote into the roster (THREAT_MODEL R71).

**Known limits of v1 (THREAT_MODEL R7, R71, R72):**
- HPKE Auth mode is not resistant to key-compromise impersonation. Anyone
  holding a recipient's `isk` can forge shares *to* that recipient.
- Every member holding VK can create validly encrypted items, metadata and rosters in that
  vault. Read-only roles and roster writes are enforced by the server only. Per-author item
  signatures are a candidate for a v2 format.
- The server distributes `ipk`. At first contact it can hand out its own key; only the
  safety-code comparison catches that, which is why verification is required before any
  seal. We have no key-transparency log.
- A member who still holds an old key can forge roster entries at that old generation (the
  roster is sealed under a key every member holds). The defence is not the roster: only the
  owner issues rotation shares, and the owner is identified by the pinned fingerprint
  (§8.3). A revocation waits for the owner's device; while the owner is offline the
  removed member keeps the current key.

### 8.3 Revoking access
A shared vault's key is never replaced by a key the server picks: **adoption (§10.2) does not
apply to a vault with the `shared` flag.** Revocation is a rotation of VK.

**Triggers** (kept as pending revocations on the owner's device until the rotation is
accepted; a manager's removal request waits there): removal of a member by the owner or a
manager; leave by a member (seen as a membership
change); an invite that was declined or cancelled after its seal was delivered (the recipient
may hold `VK_g`); and the server listing a member who is not in the roster, after a user
prompt (§8.3 "membership consistency").

**Revocation rotation** (on a device of the vault **owner** only; there is no manager
rotation in v1, and a revocation waits while the owner is offline):
1. Pull the vault to its head and fetch every attachment of the vault that is not local.
   Attachments that can't be fetched are listed in the revoke UI and keep their old ID and
   FK (they are not protected against the removed member until they are re-sealed).
2. Prepare, without changing the store: a new VK of generation `g + 1`; its wrapping under
   the owner's AK; a `ShareSeal` for every remaining member and for every pending invite
   (each `ipk` must match the roster `fpr`, and the account must be a contact this account
   **verified**: a roster entry a member wrote says nothing about who a key would go to. A
   changed key is a hard error and no seal is made. A roster member that is not verified is
   treated as unconfirmed: no seal is made to it, the rotation revokes it, and the finding
   `MemberUnverified` tells the user; a pending invitee that is unverified or can't be
   resolved by the email lookup loses its invite instead of blocking the rotation); a new roster at `g + 1` with the revoked members marked
   `removed, since: g + 1`; and emergency seals for the grants that cover the vault (§8.4).
3. Send **one atomic rotate request**: key row, member seals, refreshed invite seals, roster,
   membership removals, generation and head bump, conditional on the vault's current
   generation and head. The server refuses it (`members_mismatch`) unless the seals plus the
   revoked accounts cover every other member and every pending invite, and never revokes the
   owner.
4. On acceptance, commit: swap the key (as generated and uploaded), re-seal every record, the
   metadata and the roster under `g + 1`, re-seal every fetched attachment under a **fresh
   `attachment_id` and a fresh FK** (ADR-0017: the FK sits in item plaintext the removed
   member has read, and the attachment AAD carries no key generation), update the item
   references in the same transaction, queue the new blob uploads, and queue the old blob
   deletions only after the referencing records are accepted. All of this is one store
   transaction.
5. If the rotation is refused (`generation_taken`, `head_moved`) the device **never adopts**:
   it pulls, accepts the winner's `g + 1` only through the claim rule below, and prepares
   again at the next generation. The revocation intent persists, so a revocation is not
   lost to a race. At most three attempts per vault per run.

Until the rotation is accepted and the old key is gone, the removed member keeps whatever
they already downloaded. This cannot be avoided and the UI says so, in the sentence "<email>
keeps whatever they already downloaded. Mosaic changes the vault key now, and they can't
read anything new."

An unshared vault keeps the M6 path (key upload with adoption, §10.2); the server refuses
that path for a vault with more than one member or a pending invite.

**Removal by a manager.** `DELETE /v1/vaults/{v}/members/{a}` lets a manager or the owner
remove a member whose role is at most the caller's (never the owner, never the caller). The
server deletes the membership, keys and shares. The owner's next run finds the roster member
missing from the server's list and rotates, as for a leave. (Before this, a manager's
removal request waited on the manager's own device and nothing carried it to the owner.)

**Claim rule** (member side). The pull page of a vault lists the key shares addressed to
this account. For the lowest share above the held generation g, from sender S:
1. Fetch S's `ipk` by account ID. `Fingerprint::of(S, ipk_S)` must equal S's pin, which
   must be **verified** (the contact pin of the owner), and `fpr_S` in the roster this
   device holds at generation g.
2. S must be the vault owner: the account fixed when this device accepted the vault
   (`VaultRoster::authorizes_rotation`). No roster role or entry can authorize any other
   sender.
3. Open the seal (HPKE Auth with the own `isk` and `ipk_S`). The key's generation is the
   share's.
4. Open the roster of `g + 1` with the new key and repeat for the next share. Only the final
   key is claimed: its AK wrapping is uploaded, and the account's other devices then receive
   it as a normal key row.
Any failed step is the finding `ShareUntrusted`: the vault stays at g and its higher-generation
records stay deferred. This is what stops a removed member who colludes with the server: the
member is `removed` in the roster at `g + 1`, so a share of theirs for `g + 2` is refused.
The one cryptographic role check of the design is that a rotation share is accepted only from
the vault owner, identified by the pinned fingerprint.

**Membership consistency.** Each run compares the server's member list with the roster. A
member the server lists but the roster does not (an emergency release, or an account the
server injected) is shown as "unconfirmed". Unconfirmed members never trigger a rotation;
a rotation that runs anyway leaves them out of the new key and revokes them, unless a
manager verifies them and adds them to the roster. A member in the roster but not on the
server is the finding `MemberDropped` (recorded on the owner's device). The owner's next
rotation leaves them out of the key too: a leave, a manager's removal and a server's drop
look the same, and none of them may keep the key. A removed member's device gets `not_member`; the vault becomes
`access_lost` locally and its copy stays readable until the user removes it. The owner
leaving is refused.

### 8.4 Emergency access
Emergency access hands over **vault keys** to another account. It never recovers the
grantor's account: account recovery is only through the Emergency Kit.

The grantor seals the keys of vaults in scope to the grantee's `ipk` with HPKE Auth, one
`EmergencySeal` per vault, in the layout of a `ShareSeal`:
```
info = ctx("emergency") || grantor_account_id || grantee_account_id
aad  = header || aad("emergency", content_type:u8 = 0x02, vault_id, key_generation:u32)
```
Content type `0x02` is a vault key. Creating a grant gives every vault it covers the
owner-only roster and the `shared` flag on the grantor's device (ADR-0055), because a release
adds a member without any invite: the revocation rotation and the claim rule then run on the
shared path. A grant for an account that does not exist answers like one for a real account
and stores nothing. Content type `0x01` (the account key) is **reserved**:
no decoder accepts it, and a grant never contains AK (ADR-0048). Scope is `all` (every
AK-wrapped vault the grantor owns, including later ones) or `selected`. Only vaults where
the grantor is `owner` can be covered; vaults shared to the grantor are excluded, and the
server enforces that. Hardware-protected vaults (§7.4) cannot be covered.

**State machine (server-enforced).** `idle` → (grantee request) → `waiting` with
`release_at = requested_at + wait_days`. From `waiting`: the grantor declines (back to
`idle`, with a 24 hour cooldown before the next request); the grantor approves after a fresh
login (`approved`); or the grantee releases at `release_at` (`released`). From `approved` the
grantee releases at any time. `released` is final until the grantor deletes the grant, which
is allowed in any state and removes the seals and the memberships (with their keys and
shares) that came through the release; when the grant was `released`, the grantor's client
queues the revocations first, while the server still lists the memberships, and rotates the
vault keys at once. The grantee's device then gets `not_member` and no new records or keys.
`wait_days` is one of 1, 2, 7, 14, 30 (default 7), chosen by the grantor, and can't be
changed while `waiting`. The server notifies both accounts through its inbox counter; there
is no email. The wait must be long enough for a living grantor to open the app.

**Seal refresh.** Every hosted run on a grantor device re-seals, for each grant, every vault
in scope whose seal generation is below the local one (or that is missing under scope `all`),
and a revocation rotation includes the seals of the new generation.

**Release.** The grantee's release returns the seals; the server adds a `read` membership
with origin `emergency` to every vault in scope. The client checks the grantor pin (verified,
or a warning: an unverified grantor can only inject fake vault keys, not read anything),
opens each seal with the `info` and `aad` above (a seal for a vault ID the device already
knows is not opened: it is skipped if the vault is the grantor's already, and the finding
`ShareUntrusted` otherwise), claims the keys (the claim accepts an
emergency membership plus a seal of the current generation) and syncs the vaults read-only.

The *waiting period* is enforced by the server, not by crypto. A server colluding with the
grantee could release the blob early, and a grantor who does not open the app within the
wait does not refuse. Both are recorded as accepted risk (THREAT_MODEL R73). A release still
needs the grantor's real seal, which the server cannot forge (it holds no `isk`).

### 8.5 Contact book
The contact book is an account-level replicated document (§7.5 layout) of pins, synced
through the hosted server so that every device of the account shares them:
```
ContactsCiphertext = header || nonce[24] || AEAD(K_contacts, nonce, pad(book), aad)
header     = version:u8 || alg:u8
K_contacts = HKDF(AK, "", ctx("contacts-key"), 32)          (never stored)
aad        = header || aad("contacts", account_id)          (padded plaintext at most 256 KiB)
```
Entries: `contact:<account_id hex32>` → `{"email":E,"firstSeen":ms,"fpr":HEX64,"verified":bool}`.
The effective state of a contact when the server serves `fpr`:
`Unknown` (no pin), `Pinned{verified}` (same `fpr`), `Changed` (different `fpr`), or
`Conflicting` (a register with two distinct `fpr` values, from concurrent pins on different
devices). `Changed` and `Conflicting` block as in §8.2. Verifying writes one entry that
supersedes the others of the same fingerprint; accepting a key change replaces the pin with
an unverified one. The same document holds ownership, as three kinds of entry (ADR-0056):
`owned:<vault_id hex32>` → `true`, written when a vault is created here (and for a vault of
this device's own creation found without any entry, once, on upgrade) and before it first
becomes shared; `member:<vault_id hex32>` → the owner's account ID (hex32), written when this
account accepts a share or an emergency release, before the vault is stored; and
`emergency:<grantee account_id hex32>` → `*` or the comma-separated vault IDs a grant
covers, written before its seals are made. Entries are never removed. They are sealed under
`K_contacts`, so the server cannot forge one (it can only withhold or freeze the book, and
then the vault is held, not accepted). No `owned:` entry is written for a vault with a
`member:` entry. Before a seal or a grant goes out, the book is uploaded and nothing may
remain pending, else the operation stops with a retryable error. Deleting a grant rotates
exactly the vaults its `emergency:` entry names (every vault with a positive `owned:` entry
if there is none), whatever scope or list the server reports. Rotation plans never stop on
a key mismatch: a remaining member whose served key is not the roster's fingerprint, whom
the directory does not serve, or whose pin is `Changed` or `Conflicting` is revoked (an
invitee is cancelled) and a `MemberKeyMismatch` finding is recorded; a member nobody
verified gets a `MemberUnverified` finding (ADR-0056). A server that withholds the newest contact book from a new device lets that
device pin again for a contact it pinned meanwhile; the roster still catches it for members
of a shared vault (THREAT_MODEL R80). The server sees the ciphertext and its length (§12).

### 8.6 Folder backend: no sharing, no emergency access
A folder store cannot share, receive shares, or grant or receive emergency access, and
never holds identity keys, rosters or the contact book (ADR-0048). The reasons, all from the
current design:
1. Every folder integrity key (`K_fold`, `K_cp`, `K_dev`) is derived from one account's AK
   (§10.1). A second account could neither verify nor write a segment without new per-vault
   folder keys and formats.
2. There is no compare-and-swap (THREAT_MODEL R55). In a revocation the canonical-winner
   rule could pick a concurrent rotation by the member being removed, so the removed member
   would keep the key. The hosted server stops this with a membership check before the
   compare-and-swap.
3. No authority can stop a removed member writing. Members of a team folder, and its admin,
   keep write access to the files (R54).
4. There is no directory or mailbox for `ipk` and seals. Files that anyone with folder access
   can plant would make trust on first use worthless.
5. Nothing can enforce a waiting period. A pre-wrapped escrow in a folder means immediate
   access, which is just a share.
A user who wants sharing uses a hosted server. Team folders keep holding separate personal
accounts side by side (ADR-0042). The folder path `s/` stays reserved (§10.1). THREAT_MODEL R81.

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
  server stores hashed (SHA-256) along with its expiry. Both sides derive
  the token with `mosaic_core::sync_auth::session_token`; the server never
  sends it. When 2FA is enabled, login finish creates a *pending* session
  that only `POST /v1/login/totp` can upgrade (§9.3).
- **Pre-login.** A new device needs `(account_id, account_salt, profile)`
  before it can compute `OPW`. The server returns these for any email.
  - Email normalization (same on client and server): trim whitespace, apply
    NFC, split at the last `@`, lowercase the local part, and convert the
    domain to its lowercase IDNA A-label. Anything that is not a
    syntactically valid address is rejected.
  - For unknown emails, all three values are deterministic fakes:
    `HMAC-SHA-256(K_prelogin, label || normalized_email)` with
    `K_prelogin = HKDF(server_secret, "", ctx("server/prelogin-key"), 32)` (one
    key per purpose), and the labels
    `"prelogin/account-id"`, `"prelogin/salt"` and `"prelogin/profile"`. The
    profile comes from mapping the HMAC output onto the server's real profile
    distribution, rounded to 5 % buckets. opaque-ke's fake credential
    response is used, so KE2 looks the same as for a real account.
    A fake account ID is a UUIDv4: the first 16 bytes of the HMAC with the
    version nibble set to 4 and the variant bits to `10`. New real account IDs
    are UUIDv4 too (`AccountId::new_random`: 122 bits from the OS CSPRNG, no
    timestamp), so a real and a fake ID are both uniform random and cannot be
    told apart. Nothing in a fake depends on the time the server was set up.
    Account IDs created before this rule (UUIDv7, pre-release only) stay valid
    (THREAT_MODEL R58).
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
confidentiality. The UI says so. TOTP secrets (SHA-1, 6 digits, 30 s, the
replay guard of ADR-0008) are sealed at rest on the server:
`K_totp = HKDF(server_secret, "", ctx("server/totp-key"), 32)`,
`sealed = header || nonce[24] || XChaCha20-Poly1305(K_totp, nonce, secret, aad = header || ctx("server/totp-at-rest") || 0x00 || account_id)`
with the §3 header `version:u8 (1) || algorithm:u8 (1)`, so the AAD binds the
format. Wrong codes are limited three ways: five per pending session (counted
atomically before the code is checked), ten per account per hour from any
session (`rate_limited`), and a pending session ends after five minutes. A
matched time step is stored with a conditional update, so two concurrent
requests cannot both accept one code (replay guard).
WebAuthn for the sync account follows in M9.

### 9.4 Changing the registration
A request that re-registers OPAQUE, or that replaces or deletes the password
wrapping of AK, is accepted only if it carries a **fresh OPAQUE login** (KE3
completed within the last 5 minutes, in the same session) and passes account
2FA when enabled. The write is a compare-and-swap on the account record
version, so concurrent upgrades from two devices cannot interleave. A stolen
session token alone therefore cannot lock the user out.

- **Version at login.** The server stores the account version in the session
  row when the login completes (KE3). `password/start` and `password/finish`
  are refused with `account_version_mismatch` (and the current version) if
  the account version differs from the one stored in the session; the
  compare-and-swap of the write uses the session's version, not a number the
  client sends. A device whose session predates another device's change is
  refused and logs in again.
- **Other sessions end.** A successful change deletes every other session of
  the account in the same transaction (a stolen token does not outlive the
  change); the changing session stays and takes the new version.
- **Lost answer.** Once `password/finish` has been sent, a timeout, an
  unreachable server or a 5xx answer is not "nothing changed": the server may
  have committed. The device then checks at once, in the same call: it logs
  in with the new `OPW` (and the second factor if enabled) and, if the
  account's wrapped AK on the server is byte-equal to the one it prepared, the
  change went through and the device commits it locally like any success. If
  the check fails too, the outcome is unknown: the device reports it as such
  (`syncPasswordOutcomeUnknown`), keeps the password-changed flag set and does
  not commit; the user keeps both passwords until the recovery below
  (`sync_recover_password`) settles which one applies. The same byte-equal
  check is also the first step of a later retry: if the old `OPW` no longer
  logs in, the device tries the new one. Before `password/finish` is sent
  (the login, the second factor, the registration start), an unreachable
  server means nothing changed (`syncPasswordNeedsServer`). Any other outcome
  is the original error.
- **Change applied elsewhere.** The device can also lose a change without
  any retry that could see it: it was locked, closed or killed between the
  server accepting the change and its own commit. The server (or the folder's
  account file) then holds the new password and its wrapping of AK, the
  device the old ones. The user types the new password. The device derives
  `KEK` and `OPW` from it and the Secret Key under the **backend's** current
  parameters (the same derivation as a join, §5.4), logs in with `OPW` (and
  the second factor if enabled) and fetches the wrapped AK. It adopts the
  change only if that wrapping opens under the derived `KEK` **and the AK it
  yields equals this unlocked session's own AK** (constant-time comparison).
  Then it stores the backend's parameters and wrapping in the same
  transaction as a local change commit (quick unlock removed, §6.2). Any
  other outcome stores nothing. The rules, all checked before anything is
  stored:
  - **Account ID.** The parameters name this device's account
    (`AccountMismatch` otherwise).
  - **No downgrade.** The Argon2id profile is not weaker than the stored one
    (`ProfileDowngrade`).
  - **Retired salts.** Every salt a local commit or an adoption replaces is
    kept (public data, schema v6). This covers only salts **this device**
    replaced. A backend that offers a retired salt is
    offering an older password again and is refused (`PasswordRollback`,
    shown as `syncPasswordRollback`).
  - **Hosted account version.** The device persists the highest
    `account_version` it has seen from the backend and refuses a lower one.
  - **Folder versions.** The device keeps the highest account-file version it
    wrote or adopted (reset with the other sync state when the backend
    changes; the retired salts stay and still catch a rollback); a folder
    file at or below it is refused.
  - **Same key.** The wrapping opens to this session's own AK.

  This adds no primitive: it is the join derivation plus these checks. It
  cannot move the device to another account key, and it cannot return it to a
  password it already replaced. A malicious backend can still offer a
  password state this device never saw, but only one whose password the user
  types and whose wrapping opens to the same AK, which needs the
  Secret Key and the account's current or a newer password.

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
  checkpoint (exact encoding below)
  `HMAC-SHA-256(K_cp, ctx("checkpoint") || device_id || counter:u64 || seen_digest)`,
  where `K_cp = HKDF(AK, "", ctx("checkpoint-key"), 32)`, `counter` is a
  per-device monotonic counter and `seen_digest` is the SHA-256 of the
  device's seen state below. `device_id` is the stable per-install `InstallId`
  (not the writer ID). Other devices warn when a checkpoint shows data they
  have not received for longer than the grace period (10 min hosted, 24 h
  folder).
  - Encoding (hosted upload body and folder `head.mck`, max 4096 entries):
    `"MCKP" || version:u8 (1) || device_id[16] || counter:u64 || issued_at_ms:u64 || n:u32 || n x (vault_id[16] || generation:u32 || high_seq:u64 || view_digest[32]) || mac[32]`
    with `seen_digest = SHA-256(issued_at_ms:u64 || n:u32 || entries)`.
    (Version 1 was never released: the `view_digest` is part of it, and its definition changed
    again in M6b, before release, to `sync/view2`. The folder writes 32 zero bytes and a reader
    ignores an all-zero digest.)
    Entries are sorted by `vault_id` without duplicates; a decoder rejects
    anything else, a nil ID, trailing bytes and a bad MAC (checked in constant
    time before parsing). `high_seq` is the device's vault cursor (the last
    `server_seq` applied) after the sync in hosted mode, and the highest
    segment sequence the device wrote for the vault (0 if none) in folder mode.
    A checkpoint is longest at 4096 entries: 245,833 bytes (41-byte header,
    60 bytes per entry, 32-byte MAC). The hosted body caps follow from that
    (the raw `PUT` body at 245,833 bytes; the unpadded base64url text inside
    a JSON response at 327,778 characters).
  - Transports carry a checkpoint as opaque bytes, filed under the device ID
    of its path (`d/<device>/head.mck`, or the hosted `deviceId`); they never
    decode it. A reader decodes it only together with that ID and rejects the
    checkpoint when the `device_id` inside the body differs from it
    (`Path`): the MAC is checked first, and the filing, which the provider
    or the server controls, is checked against the authenticated body, so a
    checkpoint filed under another device's name is not read as that
    device's.
  - Versioning: a new checkpoint version MUST use a new HKDF label for
    `K_cp`'s MAC context (e.g. `checkpoint/v2`) or include the version byte
    in the MAC input.
  - Own-counter rules: a verified checkpoint for the device's own
    `InstallId` with a counter above its stored counter means the install
    identity is duplicated (a copied profile writing under this ID). A
    device persists the counter before it sends a checkpoint, and records a
    counter as published only once the backend accepted it. A hosted
    `checkpoint_stale` answer is **not** an acceptance: it means the server
    holds a checkpoint with an equal or higher counter, which an honest
    device never causes (it sends a higher counter every run), so it is a
    repeated request or a forged upload (a stolen session token can file a
    checkpoint with a huge counter under the device's name; the server cannot
    verify the MAC, R62). The engine treats it as handled for the outbox (the
    run goes on and nothing is retried in a loop) but leaves the published
    counter where it is, so the published counter never exceeds what this
    device really had accepted. The newest own
    checkpoint below the last published counter is a strike, counted once
    per published counter; the second strike means the backend was rolled
    back (one lost upload is tolerated, and a failed upload is no strike:
    the published counter did not move). Both rules stop automatic sync
    until the user acts. The halt, the strike count and the time of the
    first strike are stored (§11), so a lock, a restart or a new engine
    neither ends a halt nor resets a strike; only the user's explicit
    acknowledgement (`clear_sync_halt`) does.
  - Other devices' checkpoints: a device keeps, per other device, the
    verified checkpoint with the highest counter, with its entries (§11),
    and never accepts a lower counter: a replayed older `head.mck` neither
    replaces the newer one nor ends a lag. A checkpoint entry for a vault
    this device has no cursor for counts as lag when its `high_seq` is above
    0, unless this device deleted that vault itself or keeps it local
    (hardware-protected): a withheld new vault is reported like any
    withheld file. The time a device has lagged in a vault is stored (§11).
    A stored lag start that lies later than the device's clock now (the clock
    was set back) counts from now, so a backward jump cannot suppress a
    warning for days. A retry time of the backoff that lies further ahead
    than the longest delay it can set (plus a server's `Retry-After`) is
    dropped for the same reason.
  - **Served view and view digest (hosted).** A device keeps, per vault and
    record, the latest version the server showed it: the `server_seq` it was
    served or accepted under and the SHA-256 of the exact ciphertext bytes
    (`sync_served`, §11). Every record of a pull page overwrites its row,
    whether it is applied, ignored as dominated, deferred or unreadable, and
    so does every accepted upload (what was pushed, under the accepted
    `seq`): the last answer of the server wins, never a maximum. The base of
    an upload is the row's `seq`. The `view_digest` of a vault is
    `SHA-256( ctx("sync/view2") || 0x00 || vault_id || high_seq:u64 || key_commit[32] || meta_commit[32] || n:u32 || n x ( record_id[16] || seq:u64 || ct_hash[32] ) )`
    over every row of the vault sorted by `record_id`, with `high_seq` the
    vault cursor,
    `meta_commit = SHA-256( ctx("sync/view-meta") || 0x00 || vault_id || meta_seq:u64 || meta_ct_hash[32] || roster_seq:u64 || roster_ct_hash[32] )`
    (the vault metadata and the roster, §8.2, as served or accepted: an absent object counts as
    sequence 0 with a zero hash; this closes THREAT_MODEL R70) and
    `key_commit = SHA-256( ctx("sync/view-key") || 0x00 || vault_id || generation:u32 || wrapped_key_bytes )`,
    where `wrapped_key_bytes` is the wrapping of the vault key the device
    holds for its current generation, exactly as stored (the bytes the server
    served or accepted). The commitment binds the key into the digest, so two
    devices that adopted different keys for one generation (a server that
    answers `generation_taken` to two offline rotators and cross-serves their
    keys) disagree even when every record agrees. No secret is involved (the
    wrapping is ciphertext the server holds); the server sees 32 opaque bytes
    per vault (§12). Both formulas live in `mosaic-core` (`sync_auth`), which
    owns the labels.
    - A device publishes a non-zero digest for a vault only when its view is
      clean (no row has a `seq` above the cursor) and it holds a key that the
      server served or confirmed: a key this device generated that the server
      never accepted gets the all-zero digest (it is about to be replaced,
      §10.2). An open finding does **not** switch the digest off: the server
      must not learn that it was caught. To make the steady state clean, a
      device reads every vault it pushed to once more before it publishes.
    - When another device's stored checkpoint has a non-zero digest for a
      vault, the same generation and a `high_seq` equal to this device's
      cursor, and this device's view is clean, the digests must be equal.
      A difference means the server showed two devices different contents
      for the same head; the device records a **server finding** for the
      vault (`ServerInconsistent`; stored, §11) and reports it on every run
      until the user acknowledges it (not a halt). This detects a forged
      head that skips a record, an acknowledged upload that the server
      dropped, a page cut short and a vault rolled back with reused
      sequence numbers (and a key split), as soon as two devices reach the
      same head.
    - **Heads that never meet.** Digests are compared only at an equal head,
      so a server that keeps two devices' heads apart (one behind the other
      at every comparison) avoids the check; the lag rule above does not
      catch it either, because every device is within one record of the
      other's checkpoint. A device therefore stores, per (peer, vault), since
      when its runs have failed to compare (the first run without a
      comparison starts the count, a comparison ends it) and the peer's
      checkpoint counter then. If the peer's counter has risen since, and
      the failures span 48 hours of runs, the device records `NoCommonView` (device, vault, since) and
      reports it on every run until the user acknowledges it. Honest devices
      meet at the same head after the settle pull of a run, so they never
      raise it.
    - An upload acknowledged under a `seq` at or below the device's own
      cursor is impossible from an honest server: it is not accepted
      locally (the change stays queued) and is recorded as a finding at
      once.
    - A refused upload (`conflict`) whose following pull does not change the
      record's row is not explained by another device's write: the device
      reads the vault again from the start, once per run, and rebuilds the
      rows from the server's answers. A record that is still refused after
      that is a finding for the vault, and the change stays queued.
    - A pull page whose head is below the device's cursor means the vault
      was rolled back on the server: the device records `ServerRollback`
      for that vault, skips only that vault and keeps syncing the others
      (it is not a protocol error).
    - **Acknowledging a finding** (`clear_vault_findings(vaults)`, separate
      from `clear_sync_halt`, which clears only the rollback or identity
      halt, the strikes and `written`; a vault acknowledgement never touches
      those) clears the findings and `NoCommonView` of the vaults, forgets
      their key candidates of the current generation (a cleared key finding
      does not come back by itself), drops their served view and cursor and
      marks them for a rebuild. The next run reads each from the start; when
      that read is complete, every local record at the current generation
      that the server does not hold as it is (no served row, or another
      ciphertext) and that is not queued is queued again, so an edit that a
      lying server acknowledged and dropped, or that a rolled-back vault
      lost, is uploaded again and the acknowledgement does not make the loss
      permanent. Digests that peers published before the acknowledgement
      (and checkpoints stored while a rebuild is open) are not compared **for
      that vault**, so a peer's stale digest does not raise the finding
      again; the user acknowledges on every device. The skip is kept per
      (peer, vault) and only for the vaults being rebuilt: the digests of
      every other vault keep being compared, and a peer's next checkpoint
      stored while its vault is no longer being rebuilt compares again.
      The rebuild is bounded: a record that is still deferred (it does not
      open, or claims a generation the device has no key for) after
      `NO_COMMON_VIEW_GRACE_MS` (48 hours of runs) ends the rebuild of that
      vault and is recorded as `ServerInconsistent` for it, so a server that
      plants such a record cannot keep a vault out of the comparison for
      good. A vault flagged `ServerRollback` takes no
      uploads, metadata uploads or attachment transfers until then.
    - Status for the user: the halt, every vault finding and every
      `NoCommonView` are listed together (`sync_findings`), so one
      "acknowledge" action can clear what is set.
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
  dominates or equals does not change the stored version. It is not
  forgotten, though: where the received ciphertext is exactly the pending
  change (the server stored it and the answer was lost) the change is
  acknowledged, and where the stored record is a queued change of equal
  content (a re-seal after a rotation that another device already uploaded
  at the current generation) the received ciphertext replaces it and the
  change leaves the outbox. A received version never raises the base of an
  upload; the served view does (above). Otherwise the repairing merge runs; a
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

- **Epoch change.** Peer checkpoint state (the highest counters seen, the
  per-peer view comparisons, the lag timers and the stale-digest skips) belongs
  to one account-key epoch (§6.7) and is cleared when the epoch changes, whether
  by a local re-key or by adoption. The device's own counters, `reserved` and
  `written` are kept, so the checkpoint counter only grows across epochs.

### 10.1 Folder backend (ADR-0043)

Ownership on a folder: sharing is impossible on this backend, so every vault
of the account counts as owned for a re-key, including those a joined device
did not create (§6.6). The re-key's target epoch is `max(own, highest seen) + 1`.

The folder backend stores the same ciphertext objects as the server in a
directory that a file-sync client replicates (Dropbox, OneDrive, iCloud Drive,
Nextcloud, Syncthing, a network share). Every reader holds `AK`, so integrity
uses HMAC-SHA-256, not signatures; a forger needs `AK`.

- Keys, derived only inside `mosaic-core`:
  `K_fold = HKDF(AK, "", ctx("sync/folder-mac-key"), 32)` and
  `K_dev = HKDF(AK, "", ctx("sync/device-info-key"), 32)`; `K_cp` is §10.
- Formats (all big-endian; a decoder checks the total length first, verifies
  the MAC in constant time over everything before the trailing 32 bytes, only
  then parses, and rejects trailing bytes, unknown versions, nil IDs and IDs
  that differ from the path):
  - Segment `.mseg`: `"MSEG" || 1 || flags:u8 (bit0 snapshot, others 0) || account_id || vault_id || device_id || seq:u64 (>= 1, = file name) || created_at_ms:u64 || [snap_base:u64 || snap_parts:u32, present only when bit0 is set] || count:u32 (1..=4096) || count x (kind:u8 || len:u32 || body) || mac`,
    `mac = HMAC(K_fold, ctx("sync/folder-segment") || 0x00 || preceding bytes)`.
    The snapshot fields sit right after `created_at_ms`, so the MAC covers
    them. Rules: `1 <= snap_parts <= 65536`; `1 <= snap_base <= seq < snap_base + snap_parts`,
    so the part index is `seq - snap_base`; with the flag clear the fields
    are absent. Kinds: 1 item (`record_id || ItemCiphertext`), 2 tombstone
    (`record_id || TombstoneCiphertext`), 3 vault key (`generation:u32 || WrappedKey[74]`),
    4 vault metadata. Read limit 16 MiB; a writer keeps segments at or below
    8 MiB and 256 objects.
  - Account file `.macct`, version 2 (172 bytes): `"MACC" || 2 || account_id || account_version:u64 (>= 1) || ak_epoch:u32 (>= 1) || device_id || salt[16] || profile:u8 (1..=4) || wrapped_ak[74] || mac`.
    Version 1 (168 bytes, no epoch field) decodes as epoch 1; writers write version 2 from M7 on.
    `mac = HMAC(K_fold, ctx("sync/folder-account") || 0x00 || preceding bytes)`, under the epoch's `K_fold`.
    A joining device cannot check the MAC before it has `AK`: it uses the
    file only to attempt the join (the AEAD of `wrapped_ak` authenticates the
    parameters) and checks the MAC right after unlocking.
  - Re-key marker `.mrek` (93 bytes) at `a/<account>/account/rekey-<from_epoch:08x>-<device>.mrek`:
    `"MREK" || 1 || account_id || from_epoch:u32 || to_epoch:u32 || to_account_version:u64 || device_id || created_at_ms:u64 || mac`,
    `mac = HMAC(K_fold(from_epoch), ctx("sync/folder-rekey") || 0x00 || preceding bytes)`.
    It carries no key material; only devices of the old epoch can verify it. `to_epoch` is
    above `from_epoch` (a re-key jumps to `max(own, highest seen) + 1`, §6.6), `device_id` names the
    path's device, and a decoder rejects trailing bytes.
  - Device info `.mdev`: `"MDEV" || 1 || device_id || nonce[24] || XChaCha20-Poly1305(K_dev, nonce, plaintext, aad)`,
    `aad = header || aad("sync/device-info", account_id, device_id)` with
    `header = "MDEV" || 1 || device_id`. The plaintext is exactly 512
    bytes: `len:u16 || canonical JSON (createdAt, name, platform; len <= 510) || zero bytes`.
    A decoder rejects `len > 510` and any non-zero padding, so every `.mdev`
    has the same size (21 + 24 + 512 + 16 = 573 bytes) whatever the name.
  - Blob delete marker `.mdel` (93 bytes), at `blobs/<attachment>.<device>.mdel`
    (lowercase 32-hex IDs, one dot): `"MDEL" || 1 || attachment_id || vault_id || device_id || deleted_at_ms:u64 || mac`,
    `mac = HMAC(K_fold, ctx("sync/folder-blob-delete") || 0x00 || preceding bytes)`.
    The decoder checks `attachment_id`, `vault_id` and `device_id` against
    the path.
  - Checkpoint `head.mck`: as §10.
- **Single-writer rule.** Every path is written by exactly one install (its
  own `d/<device>/`, `log/<device>/`, its own `.macct` name, its own
  `<attachment>.<device>.mdel` marker); blobs are write-once. A sync
  client's conflict copy can therefore only appear for `head.mck`,
  `info.mdev` and the two fixed top-level files, and readers accept only
  strictly canonical names. A device removes only its own marker.
  - **Purge.** A blob is purged when this device first saw its newest marker
    more than the retention period (30 days) ago, no live local item refers
    to it, and the purging device's last sync run saw no other device
    behind, no unresolved key conflict and no object it could not explain
    (a stale device must not purge what others still use, and a record that
    cannot be read yet may refer to the blob). The age counts from the time
    this device first saw the marker, by its own clock, never from the time
    inside the marker: that time is whatever the writer's clock said, and a
    clock running ahead would purge a just-deleted blob that a concurrent
    edit restores. The first-seen time is stored per marker (§11) and starts
    again when a marker of that device changes.
  - **Re-assert and heal.** A holder re-asserts a blob that a marker or a
    deletion put at risk: it removes its own marker for every item reference
    it still has and uploads its local copy again if the blob is gone. A
    holder also checks the blob against its own copy: a blob whose size
    differs, or one of the same size it has not checked since it started, is
    opened chunk by chunk with the file key of the item that refers to it
    (§7.3), within the download budget of a run. A blob that does not
    authenticate (anyone with write access can plant a file of valid layout)
    is replaced by the holder's authentic copy through a temp file and a
    rename: the one exception to the write-once rule, allowed only to a
    holder with a copy that authenticates.
  - **Fetch.** A device that downloads a blob opens every chunk with the file
    key of the referencing item before it stores the file. A file that does
    not authenticate is not stored: the attachment stays missing and the run
    reports it.
- **Canonical key winner.** Folder mode has no compare-and-swap, so when
  several devices rotate to one generation the winner is the lowest
  `(device_id, wrapped key bytes)`; a device that holds a different key for
  that generation rotates again. The `device_id` of a key is the lowest
  device that wrote a segment carrying exactly those wrapped bytes, and
  every key is ranked that way, the held one and every candidate alike: for
  a key this device generated that includes itself, and for a key it
  received it never includes itself (a snapshot of this device's own log
  repeats keys others generated). A holder whose key came from an unknown
  writer assumes the worst and rotates (safe: a rotation loses nothing).
  The keys seen per generation, with their lowest writer, are stored (§11):
  a conflict learned in a run that failed afterwards, or before a lock, is
  resolved by the next run. A rotation prepared before the session locked
  is refused at its commit (the session tag); the engine drops it, takes
  the key the backend already holds from the device's own log if it
  was accepted, and otherwise prepares again.
- **Own log.** A reader reads its own log too, for vault-key objects only:
  a rotation whose key was written but not committed before a crash is
  otherwise never learned. Its records and metadata are this device's own
  and are skipped.
- **Own sequence numbers.** A device persists, per vault (§11), the highest
  sequence number it may have used (`reserved`, stored before the segment is
  written) and the highest it wrote successfully (`written`, stored after).
  The next segment is numbered above the highest own segment on disk that
  verifies, above `reserved` and above what the run wrote, so a folder that
  was rolled back never makes a device reuse a number its readers already
  marked read, and a failed snapshot set leaves the next set after its whole
  range (`snap_base .. snap_base + snap_parts - 1` is reserved before the
  first part is written, and every name of the range must be free: a range
  that collides with a file is moved past it). Files in the own log that do
  not verify (junk anyone with write access plants, even
  `ffffffffffffffff.mseg`) never count as segments. Anything above
  `reserved` was never written by this device: a file that does not verify
  is removed at once, whatever its modification time, and so is a directory
  (with its content) or a link (the link only) on such a name; a file that
  verifies stays and the writer steps past its name, at most 16 times.
  Files and directories on segment names at or below `reserved` that never
  verify are removed after 24 hours. A hole in the numbers of the verified
  own segments after the newest complete snapshot set (a skipped number, an
  acknowledged rollback, a change of backend) makes the housekeeping compact
  the vault, so the readers' lag closes.
- **Own log rollback.** Before a run writes anything, it checks every synced
  vault: if no file at or above `written` is present in the own log, the
  newest segments this device wrote are gone (a restored older folder, or
  deletions), and the run stops with a rollback halt before any push or
  checkpoint write. A file at or above `written` counts as present if it
  verifies or cannot be read at the moment (the sync client may not have
  delivered it whole); a file that was read in full and fails its MAC does
  not count, so planted junk cannot hide a rollback. After the user
  acknowledges the halt, `written` starts again at 0 and `reserved` stays:
  no number is ever reused. Every check that can halt a run (this one, the
  own checkpoint rules of §10) runs before the run writes anything, and a
  key rotation runs them too.
- **Hostile files.** Readers never follow a symlink or open a file that is not
  a regular file (checked before the open and again on the opened file, which
  on Unix is opened non-blocking so a pipe swapped in cannot hang the
  reader); a file is read through a cap of its maximum length plus one
  byte; at most 32 checkpoint candidates (`head.mck` first, then the
  newest by modification time) are read per device directory; the device
  list names only devices with a verified `head.mck` or `info.mdev`; a
  device deletes a conflict copy of its own `head.mck` only if the same run
  read it, and its own account files of a lower version once a verified file
  of a higher version exists from any device.
- **Joining.** The account files are tried in order (highest version, lowest
  device, at most 8) until one unlocks and its MAC verifies. A file whose
  wrapped key does not open is skipped; one that opens but fails its MAC is
  remembered and skipped; the join reports `FolderDamaged` if any candidate
  failed its MAC after unlocking and `WrongCredentials` only if none
  unlocked. Once a file opens, the account files of a higher version (at
  most 8) are MAC-checked with the key it opened: if one verifies, the
  password that opened the older file has been changed since, and the join
  is refused (`WrongCredentials`), so a re-planted old file does not revive a
  superseded password. The highest account version counts only files whose
  MAC verifies.
- **Reconnecting to a folder** (a connect, and the start of the folder's sync
  thread). The device compares the highest verified account file with its
  stored parameters and wrapping (the folder's account file as a backend, §9.4
  "Change applied elsewhere"): equal parameters, nothing to do. Different, and
  the file's salt is retired, or its version is at or below the device's
  stored folder account version: the folder offers an older password, so the
  device writes its own parameters and wrapping as a new account file at the
  highest version plus one, and records that version (a master password
  changed while the device used another backend, or the folder's old file,
  never stays live there; a join with the old password then fails). Different,
  newer and not retired: another device of the folder changed the password;
  the device sets the password-changed flag and the recovery of §9.4 finishes
  it.
- **Re-key (M7, ADR-0060).** The re-keying device runs to the head with no halt, prepares
  (the marker is MAC'd under the **old** `K_fold`, because the commit drops the old AK),
  commits locally (§6.6; it records, per synced vault, the floor `reserved` of its own log),
  then writes the version 2 account file at the highest verified version plus one, a full
  snapshot set of every vault under the new `K_fold`, the marker, the new `.mdev` and
  `head.mck`. Every step is idempotent and resumes from the journal row.
  - **Pre-re-key own segments.** After a re-key this device's segments with `seq <= floor` fail the
    MAC under the new `K_fold`. They are not junk: the 24-hour removal of unverifiable files
    at or below `reserved` does not apply to them. They are removed only when a complete
    new-epoch snapshot set exists in the own log **and** 30 days have passed since the re-key,
    so stale devices that still read the old epoch had time to read them. Files above the floor
    keep the ordinary rules.
  - **Stale devices.** Only a **verified** `.mrek` (MAC under the device's current
    `K_fold`) halts the run before any write (`AccountRekeyed`) and sets a re-key-pending
    flag; nothing is purged while it is set. An account file whose clear header names a
    higher epoch but cannot be verified raises a **dismissible finding** and no halt, so a
    planted file cannot stop every device that holds no key to check it. A dismissed file is
    recorded by path and SHA-256 and never raises again. Recovery: the user types the new
    Secret Key and the password; the device tries the account files of every higher epoch
    (§6.7), the files every verified marker names first, grouped by salt (one Argon2id
    derivation per salt, eight salts per attempt; the user can go on to the next page up to the
    scan cap of 64 files, so planted files and markers cannot crowd out the genuine one),
    opens one under the typed keys, checks its MAC under the derived new `K_fold`, adopts
    (§6.7), records its own floors and writes its own snapshot sets under the new epoch.
  - **A marker that cannot be followed.** A marker is MAC'd under the old `K_fold`, so a
    **holder of the old AK** (a thief of an old device) can write one that verifies: it halts
    every device of that epoch. Such a marker names no file that opens under the user's keys.
    After a deliberate user action (an adoption attempt that found no file that opens, then a
    confirmation) the marker is **dismissed**: recorded by path and SHA-256 like a dismissed
    account file, it never halts again, and the re-key-pending flag clears when no other
    undismissed verified marker is left. The device then keeps working at its epoch (the thief
    gains nothing the old AK did not give) or re-keys again. A folder writer who has no key can
    only delete files (denial of service, as deleting files already can).
  - **New-epoch readers** ignore every object that does not verify under the current epoch's keys
    and never accept an old-epoch object again (a holder of the old AK can still write them,
    THREAT_MODEL R83).
  - **Concurrent re-keys.** Two devices re-keyed to the same epoch with different AKs: a device
    that sees an account file of its own epoch that does not verify under its keys raises a
    `RekeyConflict` finding. There is **no adoption at an equal epoch** (§6.7). The user either
    keeps this device's re-key (the unverifiable equal-epoch file is dismissed: recorded by path
    and SHA-256, not deleted, because it is another device's file) or re-keys again from a
    device that holds the AK they want to keep, to `max(seen epochs) + 1`; every other device
    then adopts that one with its kit. A re-key is allowed while a device is halted on
    `AccountRekeyed`, targeting `max(seen epoch) + 1`. There is no automatic winner: choosing an AK
    nobody typed would make a kit useless (THREAT_MODEL R87).
- **Lag.** For another device's checkpoint entry `(vault, g, high_seq)` the
  folder reader counts the device as behind when `high_seq` is above the
  `contiguous` mark of that device in the vault's cursor. Behind for longer
  than 24 hours is reported as missing files. The time since which a device
  is behind in a vault is stored (§11), so locking or restarting the app
  does not restart the 24 hours.
- Snapshots (segments with the snapshot flag) carry the writer's whole
  readable state and let a device compact its own log. A snapshot may span
  several parts (`snap_parts`, at most 65536), each a segment of its own
  with contiguous sequence numbers `snap_base ..= snap_base + snap_parts - 1`.
  A writer deletes old segments only after every part is written. A reader
  applies the records of every verified part, like any segment, and marks
  the part's own number read once its unit is applied. When a verified part
  arrives it looks at the folder: if every sequence number
  `snap_base ..= snap_base + snap_parts - 1` is a listed file that verifies
  and names the same `(snap_base, snap_parts)`, the set is complete, and
  every number below `snap_base` is marked read (the set holds the writer's
  whole state). Completeness is read from the folder each time, never kept
  in memory, so it survives a lock, a deferred unit and a restart; sets that
  overlap or disagree cannot both be complete, because a number is one file.
  An incomplete set marks only the numbers of its own verified parts.

### 10.2 Hosted write rules

- Per-record compare-and-swap on `server_seq` (`baseSeq`); a new record
  (`baseSeq = null`) is accepted only if the record is absent.
- An upload must carry exactly the vault's current key generation, else the
  whole batch is refused (`generation_mismatch`).
- Key upload: generation must be `current + 1`, optionally conditional on
  the vault's `head_seq`; one wrapped key per (vault, account, generation).
- The re-key request is the only place the server replaces an identity key and the only
  one that replaces a key row at a generation a member holds (§6.6, ADR-0059).
- A client uploads every item of a run before any tombstone, so a moved
  item's copy reaches the server before the source's tombstone.
- Checkpoint upload: the raw checkpoint bytes, filed under the device in the
  path, which must be the session's own device. The server holds no key, so
  it checks only the length, the shape, that the body names the path's device
  and that the counter is above the stored one (`checkpoint_stale`); the
  clients verify the MAC (§10). A client does not count `checkpoint_stale` as
  published: it means the server already holds a checkpoint with this counter
  or a higher one. The run goes on, but the device's published counter
  stays, so a forged high counter filed by a stolen token (R62) causes no
  strike loop (§10, own-counter rules).
- **Hosted key winner.** The server keeps exactly one wrapped key per
  generation; it is canonical. A device remembers which of its vault keys
  the server confirmed (an accepted key upload, or the vault's creation).
  - The confirmed mark is written in the same store transaction as what it
    accompanies (the vault's creation, an accepted key upload, the commit of
    a rotation). A store that predates the mark treats a key this install
    generated, whose upload flag is clear and whose generation is the
    vault's current one, as confirmed (so it is never replaced); after a
    change of backend no key is confirmed until the new backend accepts it.
  - A device holds a key for generation `g` that it generated and the server
    never accepted (an offline rotation that got `generation_taken`) and
    learns the server's key for `g` (it arrives verified: it opens under
    AK). If more than one distinct verified key for `g` is known the server
    contradicts itself: it is a finding and nothing is adopted. Otherwise
    the device **adopts** it: the server's key replaces its own as the
    key for `g`, every record and the vault metadata it sealed under its own
    key is re-sealed under the adopted key and queued, its own key is
    dropped, and the vault is read again from the start. All of that is one
    store transaction. No generation `g + 1` is created, so a lost rotation
    costs no further rotation (the folder backend keeps the canonical
    winner and the rotation of §10.1).
  - A device holds a key that came from the server, or one the server
    confirmed, and a second valid key for the same generation shows up: the
    server contradicts itself. The device records a finding for the vault
    (`ServerInconsistent`, §10), falls back to the rank rule of §10.1 (the
    lowest device, then the lowest bytes wins) and rotates if it loses, but
    at most three times per vault per 24 hours (the log is stored, §11);
    after that the finding stays and the device stops rotating.
  - **Shared vaults never adopt** (M6b, ADR-0051). Adoption replaces a key that was never
    shared and that no other device could hold. It applies only to a vault without the local
    `shared` flag. A vault that is shared, or that has a pending invite, rotates through §8.3
    (an atomic rotate request, the claim rule, no adoption); the server refuses the plain
    key-upload path for it. The `shared` flag is monotone: it is set when the first invite
    is sent or the vault is accepted, and it outlives the vault's deletion and a change of
    backend, like the generation floor (§7.1). A change of backend is refused while any
    shared vault is live, and a reset of the sync state never clears the flag, so a backend
    switch can't turn a key shared with a member into one that adoption may replace.

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
- The sync configuration (server URL, email, folder path, device name), the
  per-install `InstallId`, sync cursors and the checkpoint counter are
  cleartext local metadata. None of it is derived from item plaintext.
- The persisted state of the sync engine (schema v4, v5) is cleartext local
  sync metadata of the same kind (v5 adds the served view `sync_served`:
  per hosted record the `server_seq` and the SHA-256 of the ciphertext the
  server last showed or accepted; the vault keys the server confirmed; the
  findings against the server per vault (and the peers whose heads never
  met, with the last comparison time per peer and vault); the conflict-driven
  rotations of the last day; and 32 bytes of view digest per vault in the stored peer
  checkpoints): the own sequence numbers per vault
  (`reserved`, `written`); the halt (rollback or identity duplicated), the
  last published checkpoint counter and the checkpoint strikes (count, time
  of the first, the counter it was counted at); the newest verified
  checkpoint of every other device with its entries (vault ID, generation,
  `high_seq`); the lag timers per (device, vault); the wrapped vault keys
  seen per generation with the lowest device that wrote them (ciphertext
  and IDs); and the time this device first saw each blob delete marker. All
  are IDs, counters, times and ciphertext; nothing is derived from item
  plaintext, and none holds a key. A change of backend clears them all but
  the halt and the reserved own sequence numbers (written starts again at
  0); the checkpoint counter only grows. Store v6 adds the salts this device
  retired (public values, kept across backends: they catch a rollback to an
  old master password) and the highest folder account-file version it wrote
  or adopted (cleared with the sync state on a change of backend); v7 adds the
  per-(peer, vault) skip of stale digests and the time since which a rebuild
  has had a deferred record (§10); v8 adds the identity key (`isk` wrapped, `ipk`, a published
  flag), the contact book ciphertext, the per-vault sharing state (the monotone `shared` flag, the own
  role, the owner account and the time access was lost), the roster ciphertext, the pending
  revocations and the sharing findings (all public data or ciphertext; none holds a key in the clear);
  v9 adds the account-key epoch (a trigger refuses a decrease), the rotation journal row, the attachment
  re-seal list and the folder re-key floors (public values and IDs; never a secret, §6.6).
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
| `account_id`, `account_salt`, Argon2 parameters, `account_version` (a counter of registration changes; a session remembers the version it logged in at, §9.4) | new-device derivation, compare-and-swap of a password change |
| `ak_epoch` (§6.7), retired-Secret-Key commitments (local only), rotation requests (vault, requester, time) and invites cancelled by a re-key | which AK a wrapping belongs to, owner notice of a member's re-key |
| OPAQUE registration record | login |
| identity public key `ipk` and the wrapped `isk` (§8.1) | sharing, adoption on a new device |
| wrapped AK (password method only; this build stores no PRF-wrapped AK and no PRF parameters (PRF salt, RP ID, credential ID) on the server: hardware-key unlock wrappings and hardware-protected vaults with their TK wrappings stay on the device, §6.3, §7.4), wrapped VKs, HPKE share seals | multi-device unlock, sharing |
| vault IDs, membership (`account_id` → `vault_id`, role, origin `owner`, `invite` or `emergency`) | access control |
| invites (sender, recipient, vault, role, generation, seal, times, delivered time), key-share seals until claimed, block lists | sharing (§8.2, §8.3), spam control |
| roster ciphertext per vault and generation (§8.2), contact book ciphertext and its length (§8.5) | sharing, pin sync across devices |
| removals (`vault_removals`: vault, account, the generation a re-invite has to pass), the per-account change counter `inbox_seq` (it moves on every invite, grant, request, decision and release, and wakes `/v1/poll`), the invite `hidden` flag (an invite from a blocked sender or to a full inbox is stored but never shown) | `reinvite_blocked`, change notification without email, spam control |
| audit event kinds 7 to 16 (identity published, invite sent, invite accepted, member removed, emergency granted, requested, declined or approved, released, account re-keyed, rotation requested), with account, time and device | the user-visible security log |
| emergency grants: grantor, grantee, wait days, scope, vault IDs and generations, state, request, decision and release times, seals (§8.4) | the waiting period, release |
| directory lookups (counted for rate limits, not logged) | abuse control |
| record IDs, ciphertext, padded size bucket, `server_seq`, timestamps of upload | sync |
| attachment IDs, ciphertext sizes and `deleted_at` (the soft-delete time, until the housekeeping removes the file) | storage quotas, soft delete |
| device IDs (per-install), device names (user-chosen, MAY be left generic), last-seen | session management |
| IP addresses: held in memory for rate limiting only, never written to disk (M6, stricter than the earlier 7 days) | rate limiting, abuse defense |
| checkpoints (device ID, counter, `issued_at_ms`, per vault generation, `high_seq` and an opaque 32-byte `view_digest`, which commits to the served records, the vault key's wrapping the server already holds, and the vault metadata and roster) | freeze and rollback detection, and the comparison of what the server showed each device (§10) |
| record kind (item or tombstone, so that deletions are visible as events), per-object lengths and object counts per vault | storage, sync, quotas |
| the device that generated each wrapped vault key (`device_id` of the key upload) | canonical key winner (§10.1), device list |
| 2FA configuration (TOTP secret for the *sync account*, WebAuthn credentials) | account 2FA |
| audit events (type, time, device — no content) | user-visible security log |

Quick-unlock wrappings (§6.2) never leave the device.

### 12.1 Folder backend: visible to the cloud provider and anyone with folder access

| Field | Notes |
|---|---|
| account ID, `account_salt`, Argon2 profile, wrapped AK (password method) | in `.macct`, with the account version |
| vault IDs, record IDs, attachment IDs and ciphertext sizes, counts | file names and sizes |
| ciphertext of records, tombstones, vault metadata, wrapped VKs, attachments | AEAD |
| device install IDs | directory names; device names are sealed under `K_dev` |
| checkpoint sequences, counters, vault generations and `issued_at_ms`; the view digest of each entry (32 zero bytes: the folder has no served view) | `head.mck` (MAC'd, not secret) |
| record kind: the segment object kind byte shows item, tombstone, vault key and metadata objects, so deletions are visible as events | `.mseg` (the kind bytes are inside the MAC'd file, not encrypted) |
| per-object lengths and object counts per segment and snapshot | `.mseg` |
| segment `created_at_ms`, the snapshot flag and the snapshot part fields (`snap_base`, `snap_parts`) | `.mseg` header (MAC'd, not secret) |
| blob delete markers: `deleted_at_ms` and the deleting device | `.mdel` (the file name and the MAC'd body) |
| which device wrote each wrapped vault key (the devices whose directory `log/<device>/` holds a key object with exactly those bytes; a device that repeats a key in a snapshot shows as a writer of it too) | the writing device's directory holds the key object |
| `ak_epoch` in `.macct` (clear header), re-key markers `.mrek` (epochs, new account version, writing device, time; MAC'd, no key material) | `.macct`, `.mrek` |
| size of `info.mdev`: constant (573 bytes), so the length of a device name is not visible | `.mdev` (padded plaintext, §10.1) |
| file creation and modification times | edit timing |

The provider, its admins and the other members of a shared folder can also
delete, withhold or roll back files, and a provider's version history keeps
old ciphertext and old password wrappings of AK (THREAT_MODEL R54).

In short, the hosted server sees the sharing graph (THREAT_MODEL R74). A folder store never holds identity keys,
rosters, the contact book or emergency grants (§8.6): §12.1 is unchanged.

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
origin or rpId policy (that is `mosaic_origin::webauthn_rp_id`, ADR-0040,
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
(cross-origin frames are not served, ADR-0040).

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

**Import and export** (ADR-0041). Import adds no primitive. A private key
arrives as base64url (the padded form is accepted too) or PEM `PRIVATE KEY`
PKCS#8, decoded strictly into a zeroizing buffer, validated with
`Es256Key::from_pkcs8_der` and stored as `to_pkcs8_der` (one encoding in the
vault). ES256 only: a well-formed PKCS#8 key of another algorithm or curve is
`passkeyUnsupported`, a damaged P-256 key or anything else is
`passkeyUnreadable` (`pkcs8_kind` reads the structure only). The rpId is
brought into canonical form by `mosaic_origin::canonical_rp_id`
(ASCII-lowercase, one trailing dot removed, the `webauthn_rp_id` canonical
domain check, a registrable domain in the public suffix list) or the passkey
is unreadable. CXF export writes each key as base64url PKCS#8 under fresh
random 16-byte account and item IDs per export (from `mosaic_core`'s `random`,
§3), so the file carries neither the vault ID nor an item ID. THREAT_MODEL R52.

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
