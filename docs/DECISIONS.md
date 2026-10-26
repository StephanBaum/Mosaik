# Architecture Decision Records

Newest last. ADRs are never edited after they are accepted, except to change
their status. To change a decision, add a new ADR that supersedes the old one.

## Template

```markdown
## ADR-NNNN: <title>

- Status: proposed | accepted | superseded by ADR-XXXX
- Date: YYYY-MM-DD
- Milestone: Mx

### Context
What forces are at play, and what problem are we solving?

### Decision
What we do. Be specific enough to implement and review against.

### Alternatives considered
Each option, and why it was rejected.

### Consequences
What gets easier, what gets harder, new risks, and follow-up work.
```

---

## ADR-0001: Product name "Mosaic"

- Status: accepted
- Date: 2026-10-25
- Milestone: M0

### Context
The brief used the working name "Tessera". The owner named the product
"Mosaic".

### Decision
Use "Mosaic" everywhere: crate names `mosaic-*`, the CLI binary `mosaic`, npm
scope `@mosaic/*`, and the crypto domain-separation prefix `mosaic/v1/`.
Secret Key display strings start with `M1-` (format version 1).

### Alternatives considered
Keeping "Tessera" internally was rejected. Two names cause confusion, and the
crypto labels become permanent once data exists, so this was the last cheap
moment to rename.

### Consequences
The `mosaic/v1/` labels are frozen once M1 ships. Renaming later would need a
format version bump. Trademark and namespace availability (crates.io, npm,
domains, stores) still has to be checked by the owner. See PROGRESS
"Needs owner".

## ADR-0002: Intermediate account key (AK) between KEK and vault keys

- Status: accepted
- Date: 2026-10-25
- Milestone: M0

### Context
The brief has the KEK wrap the vault keys directly. But several unlock
methods (password, OS quick unlock, hardware-key PRF) must each be able to
unlock everything, and a password change must stay cheap.

### Decision
Use a random 256-bit account key AK. Each unlock method wraps AK, and AK wraps
the vault keys and the identity private key (CRYPTO_SPEC §6–§8).

### Alternatives considered
- KEK wraps every VK directly: a password change and every new unlock method
  must re-wrap N vault keys, and quick-unlock methods would need N wrappings
  each.
- Deriving vault keys from KEK: changing the password would then force
  re-encrypting everything. Rejected.

### Consequences
A password change re-wraps exactly one key. One more key layer to review. The
brief's intent ("re-wraps keys, never re-encrypts items") is preserved.

## ADR-0003: OPAQUE input is the two-secret derived OPW, with Identity KSF

- Status: accepted
- Date: 2026-10-25
- Milestone: M0

### Context
OPAQUE prevents offline guessing without server compromise. After a server
database compromise, an attacker can still run a dictionary attack on the
OPAQUE password input. The brief requires that a stolen server database yield
nothing brute-forceable.

### Decision
The OPAQUE password input is `OPW = HKDF(R, "mosaic/v1/opaque-password")`,
where `R` combines the Argon2id-stretched master password with the 128-bit
Secret Key. OPAQUE's internal KSF is `Identity` (CRYPTO_SPEC §5.4, §9.1). The
KEK is derived locally from the same root, never from OPAQUE's `export_key`.

### Alternatives considered
- Master password as the OPAQUE input with an Argon2 KSF: brute-forceable
  after a DB compromise, and it does not use the Secret Key. Rejected.
- Deriving the KEK from `export_key`: unlocking would need the server, which
  breaks offline-first. Rejected.
- SRP (as 1Password does): the brief mandates OPAQUE, and OPAQUE has a
  stronger security proof.

### Consequences
Argon2id runs once per unlock, not twice. Changing the master password or
Secret Key requires OPAQUE re-registration. New devices need pre-login
parameters, which are protected against account enumeration by fake responses.

## ADR-0004: HPKE (RFC 9180) Auth mode for sharing and emergency access

- Status: accepted
- Date: 2026-10-25
- Milestone: M0

### Context
The brief asks for "per-recipient X25519 public-key wrapping". A hand-rolled
X25519 + HKDF + AEAD scheme would be an invented protocol.

### Decision
Use HPKE with DHKEM(X25519, HKDF-SHA256), HKDF-SHA256 and ChaCha20-Poly1305 in
Auth mode, so recipients can verify which account sealed the key.

### Alternatives considered
- Custom ECIES: violates "no invented crypto".
- HPKE Base mode: the server could inject vault keys under an arbitrary sender.
- `age`: built for files. Its X25519 recipients have no sender
  authentication.

### Consequences
Adds the `hpke` crate, which needs vetting in M6. A residual risk remains:
the server is trusted to distribute public keys honestly unless contacts are
verified (THREAT_MODEL R3).

## ADR-0005: No `mlock` for secret memory (for now)

- Status: accepted (revisit in M9)
- Date: 2026-10-25
- Milestone: M0

### Context
Secrets can be paged to swap. Preventing that needs `mlock`/`VirtualLock`,
which means `unsafe` FFI or an extra dependency that uses `unsafe`.

### Decision
Use zeroization and short lifetimes only. Do not lock memory pages in M1–M8.
Keep core crates `#![forbid(unsafe_code)]`.

### Alternatives considered
The `memsec`/`region` crates: they add `unsafe` dependencies, and locked-page
limits differ by platform.

### Consequences
Residual risk R5. In M9, re-evaluate locking a small, fixed-size secret arena
inside a separately audited crate.

A related, smaller residue is stack copies inside upstream crates that we
cannot zeroize without patching them: `hmac` builds its `key ^ ipad/opad`
block in an unzeroized stack buffer, and `hkdf`'s expand leaves its last
output block on the stack. For Mosaic's derivation that is `R`-equivalent
key material and a copy of `KEK`/`OPW` (CRYPTO_SPEC §5.4). Our code zeroizes
everything it owns, including the HKDF PRK (ADR-0007 item 9). Residual
risk R11.

## ADR-0006: Shared TypeScript packages and pnpm

- Status: accepted
- Date: 2026-10-25
- Milestone: M0

### Context
The desktop app and the extension must share UI strings (DE/EN from day one)
and design tokens. The owner's tooling standard is pnpm.

### Decision
A pnpm workspace (`apps/*`, `packages/*`) with `packages/design-tokens`
(tokens and a WCAG contrast test) and `packages/i18n` (DE/EN ICU catalogs and
a parity test). The ICU formatter library is chosen in M3. Dependency
lifecycle scripts are disabled by default (`onlyBuiltDependencies: []`).

TypeScript is pinned to 6.0.x because typescript-eslint 8.x does not support
TypeScript 7 yet. Revisit when it does.

### Alternatives considered
- Duplicating tokens and strings per app: they drift.
- npm or yarn: the owner's standard is pnpm, and pnpm's strict
  `node_modules` layout reduces phantom-dependency risk.

### Consequences
`packages/` is a top-level directory not listed in the brief's suggested
layout.

## ADR-0007: M1 format details left open by CRYPTO_SPEC

- Status: accepted with amendments (M1 crypto review, 2026-10-26).
  Amendments made at acceptance: item 4 now requires the unpadding scan to
  be constant-time in the plaintext; item 10 records that the fuzz crate
  builds only with `--cfg fuzzing`.
- Date: 2026-10-25
- Milestone: M1

### Context
Implementing `crates/core` exposed details that CRYPTO_SPEC §5.3 and §7 did
not pin down. Decoders must be strict and length-bounded (§10), so every
object needs a maximum size and a single valid layout. The tombstone header
and the attachment header were not fully specified.

### Decision
1. **Size limits.** Padded item plaintext ≤ 1 MiB. Padded vault metadata
   ≤ 64 KiB. Tombstone plaintext ≤ 64 KiB. Attachments ≤ 50 MiB (already in
   §7.3). Sealing above a limit fails with `SizeLimit`. Decoders check the
   limit before any crypto.
2. **Tombstone header** = `version:u8 (0x01) || alg:u8 (0x01) || key_generation:u32`,
   the same layout as the vault-metadata header. Tombstone AAD has no
   `schema_version`, so the item header does not fit. Tombstones are not
   padded, as §7.2 states.
3. **Attachment header** (the AAD-bound part) = `version || alg || chunk_log2`.
   The 19-byte nonce prefix is authenticated through each chunk's nonce, the
   same way the nonce of the other objects is. Only `chunk_log2 = 16` is
   accepted. Canonical chunking: every chunk except the last is full. The last
   chunk is non-empty, except that an empty file is one empty last chunk.
   Decoders reject every other layout.
4. **Strict unpadding.** `unpad` accepts only the canonical `pad` output:
   a multiple of 256 bytes, at least 512, a `0x80` marker followed by zeros,
   and no extra padding blocks. The input is decrypted plaintext, so the
   scan is constant-time in its contents: it visits every byte, tracks the
   last non-zero byte with `subtle` selects and branches only on the final
   validity flag. Its timing therefore does not reveal the message length
   that padding hides.
5. **Errors.** `crates/core` reports every authentication or padding failure
   of every object as the single error `Error::Decryption`. The item layer
   (M2) maps it to §7.2's `ItemCorrupted { item_id }`. A header
   `key_generation` that differs from the supplied key's generation is the
   separate error `KeyGenerationMismatch`, checked before decryption. The
   generation is cleartext, so this reveals nothing. Generation 0 is invalid
   (`InvalidKeyGeneration`).
6. **Calibration (§5.3).** "About 1 s or less" means a measured time
   ≤ 1.000 s. `standard` is measured first. If it is ≤ 1 s, `strong` and
   then `max` are tried while each stays ≤ 1 s. If `standard` is between
   1 s and 2 s, `standard` is chosen. If it is slower than 2 s, `baseline`
   is chosen. If `standard` cannot be allocated, `baseline` is measured once
   to confirm it can be allocated, otherwise calibration fails.
   Measurement is injectable, so tests are deterministic.
7. **Argon2 lanes run sequentially.** The `argon2` crate's `parallel`
   feature (rayon) is not enabled, so `p = 4` lanes are computed on one
   thread. The output is identical, only slower. Calibration measures real
   time, so it simply picks a lower profile on slow devices. Enabling
   `parallel` (and adding rayon) is deferred to M3, with a dependency review.
8. **Secret Key parsing.** The `M1` prefix is required. It goes through the
   same normalization as the data characters (case, `-`, whitespace,
   aliases). Non-zero pad bits, a wrong length, a missing prefix or an
   invalid character give `SecretKeyMalformed`. A checksum mismatch gives
   `SecretKeyMistyped`.
9. **Zeroization.** `chacha20poly1305/zeroize` (cipher key and keystream
   state) and `sha2/zeroize` are enabled. Through `digest/zeroize` the
   latter wipes the SHA-256 block cores on drop, including the inner and
   outer cores inside `Hmac` and therefore inside `Hkdf`. It does **not**
   cover temporaries in the upstream code: `Hkdf::new` drops the extracted
   PRK unwiped, `hmac`'s key setup leaves `key ^ ipad/opad` on the stack, and
   `expand` leaves its output block on the stack. `kdf::hkdf_sha256`
   therefore calls `Hkdf::extract` itself and zeroizes the returned PRK. The
   other two are upstream residue (ADR-0005, THREAT_MODEL R11). Argon2's
   working memory is a caller-owned buffer that is reserved fallibly
   (`try_reserve_exact`) and zeroized after use.
10. **Fuzz crate supply chain.** `crates/core/fuzz` has its own workspace
    and lockfile, so CI runs `cargo deny` and `cargo audit` on it
    separately (`--manifest-path` / `-f`). `libfuzzer-sys` bundles LLVM's
    libFuzzer under the permissive NCSA license; `deny.toml` allows NCSA for
    that crate only. It is never linked into Mosaic. The fuzz targets use
    fixed keys (the golden keys) so crashes reproduce; `VaultKey` gets a
    `#[cfg(fuzzing)]`-only constructor for that, which normal builds do not
    contain. The fuzz crate therefore builds only with `--cfg fuzzing`
    (`cargo fuzz build`); a plain `cargo check` stops with a
    `compile_error!` that says so. Seeds live in `crates/core/fuzz/seeds/` (committed, built from
    the golden files); `corpus/` stays ignored. CI pins `cargo-fuzz`, a
    dated nightly, `cargo-llvm-cov` and `cargo-audit`.
11. **Profiles never decrease.** `change_master_password` takes the
    account's current `PasswordParams` and refuses a weaker profile with
    `Error::ProfileDowngrade` (§5.3, §6.4). The same or a stronger profile
    is accepted, which is the §5.3 upgrade path.
12. **Secret Key encoding is branch-free.** Mapping between characters and
    5-bit values uses constant-time selects (`subtle`) over the whole
    alphabet and range arithmetic, with no table lookup or early exit on a
    secret character. Only length and validity checks, which are not
    secret, are ordinary branches.
13. **AAD labels.** `Aad` and `Label` are crate-private. A `Label` can only
    be built by a `const fn` that rejects NUL bytes, and every label is a
    `const` item, so the `ctx(label) || 0x00` separator is unambiguous by
    construction (a NUL label is a compile error).
14. **Dependencies.** Direct dependencies of `mosaic-core`, all pinned with
    `=`, default features off:
    - `argon2 =0.6.0` (`alloc`, `zeroize`): Argon2id, §5.3.
    - `hkdf =0.13.0`, `sha2 =0.11.1` (`zeroize`): HKDF-SHA-256, the
      Secret Key checksum, the hardware-key `method_id`, §5.2–§5.4, §6.3.
    - `chacha20poly1305 =0.11.0` (`alloc`, `zeroize`): XChaCha20-Poly1305,
      §6, §7.
    - `aead-stream =0.6.0` (`alloc`): RustCrypto's STREAM, which §7.3 names.
      With `aead` 0.6 STREAM moved out of `aead::stream` into this crate.
    - `zeroize =1.9.1`: wiping secrets on drop, §4.
    - `subtle =2.6.1`: constant-time comparison and selection, §4, §5.2.
    - `getrandom =0.4.3`: the OS CSPRNG, §13. It is called directly; a
      `rand_core` dependency would only wrap it.
    - `unicode-normalization =0.1.26`: NFC, §5.1.
    - `thiserror =2.0.21`: derives `Display`/`Error` for the single error
      enum without hand-written boilerplate. Build-time only (proc macro).

    Dev-only: `proptest =1.12.0` (property tests, §14) and
    `hex-literal =1.1.0` (KAT vectors). Fuzz-only: `libfuzzer-sys =0.4.10`.
    Not added, although the M1 plan listed them: `secrecy` (the crate has
    its own containers in `secret`, which also give constant-time equality,
    redacted `Debug` and heap boxing for fixed-size keys, so `secrecy` would
    add a layer without removing code) and a direct `rand_core` (see
    `getrandom`).

### Alternatives considered
- No explicit size limits: decoders would be unbounded, against §10.
- Reusing the item header for tombstones: `schema_version` would then sit
  in the header but not in the AAD fields, which is confusing to review.
- Putting the nonce prefix into the attachment AAD header: harmless, but
  inconsistent with the other objects, where the nonce is not AAD.
- Distinct errors for tag and padding failures: forbidden by §7.2.

### Consequences
The limits are format constants. Raising them is a spec change, but it does
not need a version bump, because old objects stay valid. Golden files in
`crates/core/tests/golden/` pin every layout.
