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

## ADR-0008: OTP verification, timing and label rules in mosaic-totp

- Status: accepted (M2 integration, 2026-10-26)
- Date: 2026-10-26
- Milestone: M2

### Context
CRYPTO_SPEC §13 fixes how codes are generated but not how they are checked.
M6 needs server-side account 2FA (§9.3) and M4 may check codes too. A
consumer comparing code strings with `==` gets a variable-time comparison
and no replay protection. Review of the M2 totp track also found that code
generation did secret-dependent table lookups that the base32 codec avoids
(ADR-0007 item 12), that `otpauth://` labels dropped a mismatching issuer
prefix and accepted invisible characters, and that the advisory secret
length check fired for the 80-bit secrets most services issue.

### Decision
1. **Verification lives in the crate.** `Otp::verify(candidate, unix_time,
   window, last_used)` computes every candidate step (TOTP and Steam:
   `T - window ..= T + window`; HOTP: `counter ..= counter + window`, look-ahead
   only) and compares each with `subtle` (`Code::ct_matches`), without an
   early exit. `window ≤ 10` (`MAX_VERIFY_WINDOW`), else `InvalidWindow`;
   recommended `1` (RFC 6238 §5.2). Steps at or below `last_used` never
   match; the largest matching step is returned and the caller must store it
   (HOTP: set `counter = step + 1`) before accepting the login. The
   candidate is not normalized.
2. **Code generation is table-free, best effort.** Dynamic truncation reads
   every 4-byte window of the MAC and keeps the one at the offset with
   `subtle` selects. Steam characters are a `subtle` select over all 26
   entries. Divisions and remainders use constant divisors only. This is
   not a guarantee (compilers decide), and the residual leak concerns a
   short-lived HMAC-derived code, not the key.
3. **Labels.** When the `issuer` parameter is present and the label prefix
   differs from it (ASCII case and surrounding whitespace ignored), the whole
   label becomes the account name instead of losing the prefix. A blank
   prefix is still removed. Issuer and account names additionally reject
   `Zl`/`Zp`, soft hyphen, zero-width space, U+2060..U+206F, BOM, Hangul and
   Khmer fillers, interlinear annotation, musical formatting and tag
   characters. ZWNJ and ZWJ stay allowed between characters (Persian, Indic
   scripts, emoji sequences), but not at the start or end. The whole URI is
   trimmed of ASCII whitespace before parsing.
4. **Secret strength.** `OtpSecret::strength()` returns `Weak` (< 10 bytes),
   `BelowRfc4226` (10 to 15 bytes) or `Rfc4226` (≥ 16 bytes). The UI warns
   only on `Weak`.
5. **Dependencies** (added at acceptance, M2 integration). The crates that
   CRYPTO_SPEC §2 names for OTP, pinned with `=` and default features off:
   `hmac =0.13.0` (`zeroize`; already in the tree through `hkdf`) and
   `sha1 =0.11.0` (`zeroize`; its default `alloc` and `oid` features are
   not needed). `sha2` and `subtle` are shared with `mosaic-core`. The HMAC
   and SHA-1 states zeroize on drop with these features. `mosaic-totp`
   does not depend on `mosaic-core`.

Amendments (third M2 integration review, 2026-10-27):

- A `Code` keeps its characters in a heap box filled in place, like
  `mosaic-core`'s fixed-size secrets, so returning it by value through
  `hotp`, `code_at` and `mosaic_vault::session::OtpCode` moves only the
  pointer and leaves no unwiped stack copy of a live code.
- `OtpAuth::issuer` and `account_name` are `SecretText` (zeroizing,
  redacted `Debug`, constant-time `==`) instead of `String`: they are item
  content the server never sees (CRYPTO_SPEC §7.5, §12). Parsing decodes
  them into zeroizing buffers, and `to_uri` builds the label in zeroizing
  buffers reserved at their worst-case size. `SecretText` gained `new`,
  `From<String>`, `len`, `is_empty` and `PartialEq`/`Eq`. This changes the
  type of two public fields (pre-freeze; CORE_API updated).

### Alternatives considered
- Leaving verification to M6: every consumer would reimplement window and
  replay logic, and the easy path (`==`) is the wrong one.
- A boolean `verify`: the caller could not store the matched step, so
  replays inside the window would pass.
- Documenting generation as variable-time instead of fixing it: the fix is
  a few lines and keeps the crate consistent with ADR-0007 item 12.
- Rejecting all `Cf` characters including ZWNJ/ZWJ: would make Persian
  account names and emoji ZWJ sequences unimportable.
- Exposing the dropped label prefix as a separate field: it would not
  survive formatting, so `parse(to_uri(x)) == x` would break.

### Consequences
M6 stores the last accepted step per account 2FA secret and calls `verify`
with it. Imports that relied on a mismatching prefix being dropped now show
it in the account name, which the user can edit.


## ADR-0009: M2 item model, plaintext format and sync merge

- Status: accepted (M2 integration, 2026-10-26)
- Date: 2026-10-26
- Milestone: M2

### Context
Brief §5 asks for per-item records with version vectors, per-field
last-writer-wins with conflict copies, and no silent data loss. CRYPTO_SPEC
§7.2 leaves the item plaintext schema to `crates/core`, and §10 trusts
only the authenticated version vector. Review of the M2 model track found
that a pure merge alone cannot meet "never lose data" in four places: a
delete racing an edit, two writers sharing a device ID, field keys the
decoder rejects, and member names both plaintext layouts reuse.

### Decision
1. **Replicated document.** Each item, vault metadata and tombstone is a
   version vector `{DeviceId: u64}` plus one register per field key. Every
   value carries the Lamport stamp `(counter, device)` of the edit that
   wrote it. The field value is the entry with the largest stamp. Wall
   clocks are display only. `Item::merge` is the causal join: commutative,
   associative and idempotent for any input. Concurrent losers stay as
   conflict copies until a device resolves or dismisses them.
2. **Plaintext.** Canonical compact JSON, `schema_version` 1:
   `{"schema_version","vv","fields"}` for items and vault metadata,
   `{"schema_version","vv","deleted"}` for tombstones. Unknown fields,
   unknown members of known values and unknown top-level members are kept
   and written back. The four names `schema_version`, `vv`, `fields` and
   `deleted` are reserved in both layouts: a decoder rejects any of them
   as an unknown member (`Malformed`), so neither layout can smuggle in the
   other's member and re-encode it as a duplicate key. Nesting depth is at
   most 32. Strings live in zeroizing `Text`; `serde_json` may keep short
   unwiped copies of strings with escape sequences in its scratch buffer
   while parsing (accepted residual, like ADR-0005).
3. **Field keys.** 1 to 256 bytes, no control characters. The decoder
   checks it, and so does every write, so the edit API cannot produce a
   record that no device can decode. `ElementId::from_stored` validates its
   input, and `set_url` / `set_custom_field` fail on an element that does
   not exist instead of creating one.
4. **Top-level members are grow-only.** Unknown top-level members have no
   clock and merge by the larger value. A newer client therefore can never
   lower or remove one: a stale replica brings the old value back. New
   top-level members must only ever grow (flags that only turn on, sets
   that only gain elements). Anything that changes or can be removed goes
   under `fields`, where it gets a per-key clock.
5. **Deletion.** A delete is one edit that removes every field and writes
   a `deleted` entry; its tombstone is the version vector plus the
   deletion entries (§7.2). Against stale replicas the delete wins. A
   delete that races a concurrent *write* loses: the write survives and
   the item is flagged (`deleted_concurrently`). An edit that only removed
   fields does not conflict with a delete. Edits of a tombstone fail with
   `ItemDeleted`; `Item::undelete(type)` revives it explicitly with a type
   and `created_at` (the undelete time).
6. **Repairing merge, the store contract (CRYPTO_SPEC §10).** The pure
   merge keeps only the concurrent write; it cannot know which removed
   fields the editing device still wants. Every store therefore merges a
   received record into its own latest state with
   `Item::merge_and_repair(received, device)` and persists only that
   result; `Item::merge` is for tests, simulations and comparisons. The
   repair runs as a local edit, with fresh stamps of `device`:
   - If the merge is flagged by a deletion this replica had not seen,
     every field this replica held that the merge dropped is written back
     with all its values, conflict copies included, and the original
     write times. The flag stays until the user dismisses it or deletes
     again. A device that edited, or merely held, the item while another
     device deleted it keeps every field it had.
   - The deleting device cannot repair; it shows the item with an unknown
     type and the deletion notice until a repair arrives.
   - The repair restores only this replica's own state, never values from
     the received record: those could be stale, and their fresh stamps
     would outrank newer edits.
   - It can bring back a field that a third device removed in the same
     concurrent window. The item is flagged either way, so the user sees
     it.
   Without a repair the result equals the pure merge.
7. **Device IDs.** A `DeviceId` belongs to exactly one install and one
   writing process. The desktop app and the browser extension never share
   one, even on the same machine. A restored backup or a copied profile
   takes a new ID. Two writers with one ID produce equal stamps for
   different edits, and the pure merge keeps one of them (larger content,
   which keeps the laws). Detection, best effort:
   - `merge_and_repair` fails with `DeviceIdReused` when the received
     record holds a counter for `device` above this replica's own state.
     The store rotates its device ID and merges again with the new one.
   - When both replicas hold one stamp with different contents in the
     same field, the repair writes the dropped value back (both show up
     as a conflict) and sets `MergeOutcome::stamp_collision`; the store
     warns the user.
   Reuse across different fields of one edit cannot be detected from the
   state, so unique IDs are the actual guarantee.
8. **Conflict classification.** `Item::conflicts` lists content fields
   only. `type`, `created_at`, `trashed_at`, `favorite` and `history:*`
   take the value with the largest stamp and are never shown as a
   conflict. The counters `otp.counter` and `passkey.sign_count` merge by
   maximum. `resolve_conflict` on a counter accepts only the largest
   value and `dismiss_conflict` keeps it, so neither lowers a HOTP counter
   or a passkey signature count.
9. **Secret construction for consumers.** `SecretString::from_str_copy`
   (one exact allocation) and `From<String>`, and a public
   `Text::to_secret`, so M3 to M6 can build `CustomFieldValue::Hidden` and
   `SshKeyMaterial` without detours.

Amendments (M2 integration review, 2026-10-26):

- Item 7: `merge_and_repair` also fails with `DeviceIdReused` when both
  replicas hold one of `device`'s own stamps with different contents. Two
  writers that each made one edit from the same state leave the counters
  equal, so only the colliding stamp shows the reuse; after the rotation
  the merge keeps both values as a conflict.
- Conflict copies: an edit of a field that keeps conflict copies
  supersedes the winner and every other entry holding the same value (they
  show as one value, so the editor saw all of them). Before, only the
  winner went, and a second equal entry turned into a spurious conflict.
- Password history is trimmed by edit order (the Lamport stamp of each
  entry's winning value), never by the wall-clock `changed_at` (display
  only), and the entry the current edit adds is never trimmed. Trimming by
  `changed_at` let a device with a slow clock drop the value it had just
  moved into the history.
- OTP secrets are 1 to 128 bytes (`MAX_OTP_SECRET_LEN`, was 512), the same
  limit as `mosaic-totp`, checked on every write and decode, so no stored
  configuration fails to produce codes (CRYPTO_SPEC §13).

Amendments (second M2 integration review, 2026-10-27):

- Item 6: the delete-race repair works per entry, not per field. For every
  field the received side does not hold, each value this replica held that
  the merge dropped is written back, also when the field itself survives
  through a concurrent local write. Before, only whole missing fields came
  back, so the conflict copy an editor kept on purpose (its write
  superseded only the shown value) was dropped for good (sync review c1).
  Fresh stamps would put a restored copy above the value the replica
  showed, so that value is written again on top when it survived; the
  shown value does not change. Fields the received side holds are left to
  the merge, because a value dropped there may have been superseded by a
  write that saw it.
- `VaultMeta::merge_and_repair` applies item 7 and the stamp-collision
  repair to vault metadata (it is never deleted, so there is no
  delete-race repair). Two writers with one device ID that rename a vault
  from the same state no longer lose one name to the pure join.

Amendments (third M2 integration review, 2026-10-27):

- Item 7: "reuse across different fields cannot be detected" was too
  optimistic about the consequence: such a reuse made the pure merge drop
  both writers' values and everything they superseded, and the documented
  recovery (rotate, merge again) lost them too. ADR-0015 makes writer IDs
  fresh per unlock, so the case needs a forged or corrupted record, and
  `merge_and_repair` now writes back every value the merge dropped when
  the vectors are equal but the contents are not, or when an ID the caller
  rotated away from collides (`merge_and_repair_after_reuse`).
- Item 5 (no silent loss): a merge result that exceeds the item size
  limit is split deterministically instead of failing (ADR-0016).
- The 32-entry password history bound holds per edit: concurrent changes
  merged from several devices can leave a few more entries until the next
  change trims them by edit order (CRYPTO_SPEC §10).

Amendments (fourth M2 integration review, 2026-10-27):

- Password history is trimmed by the stamp encoded in each entry's key
  (`history:{counter:016x}.{device}.{seq}`, the edit that moved the value
  there), not by the register's winning stamp. A delete-race repair
  writes every restored entry back under a fresh stamp, so the oldest
  entry looked like the newest edit, and the next change dropped the value
  the user had just replaced (sync review). The key keeps its place
  through any repair; a key of another form falls back to its winning
  stamp.
- A collision repair (item 7, ADR-0015) that writes a value back into a
  `history:*` key moves it to a history key of its own, named after its
  fresh stamp: two writers under one ID write the same history key with
  different old passwords, and the history shows one value per key, so the
  written-back one was invisible (found by the reuse property test).

### Alternatives considered
- Repair inside `merge`: the merge must stay a pure join, or replicas do
  not converge.
- Keeping `type` and `created_at` in tombstones: §7.2 defines the tombstone
  as the version vector and the deletion time, and the repair restores
  them anyway.
- Delete wins over a concurrent write: loses the write, against brief §5.
- Restoring only when this replica made a concurrent edit itself: fails
  when its edit reached it again folded into another device's merge.
- Dropping a foreign reserved member on decode instead of rejecting it:
  silent data change; strict rejection matches §10.
- Treating equal stamps with different content as an error in `merge`:
  the merge would no longer be total, and one corrupt record would stop
  sync for the whole item.

### Consequences
M3 must call `merge_and_repair` for every received record, persist its
result, rotate the device ID on `DeviceIdReused`, and show
`stamp_collision` and `deleted_concurrently`. M4 gives the extension its
own device ID. Future schema work adds data under `fields`, never as a
changing top-level member. The fuzz targets `item_plaintext` and
`item_merge` get seeds with the other layout's reserved member and a
libFuzzer dictionary of the member names.


## ADR-0010: Local store state, anti-rollback and key-wrapping rules

- Status: accepted
- Date: 2026-10-26
- Milestone: M2

### Context
The review of the M2 local store (`mosaic-vault::storage`) found gaps that
CRYPTO_SPEC §7, §10 and §11 do not settle. The store kept an opaque
`accepted_state` blob per record in cleartext, but the version vector it
was meant to hold sits inside the item plaintext, which §11 keeps off disk.
Deleting a vault also deleted its anti-rollback mark (§7.1). The schema had
no place for the §7.4 TK wrappings. Deletes dropped queued local changes.
A §6.6 re-key could not re-wrap older vault-key generations, and nothing
made it atomic. Removed wrappings stayed in WAL frames. Whether a moved
item keeps its attachment IDs was not specified. The schema is still at
version 1 and unreleased, so these are fixed in schema v1 itself, without a
migration.

### Decision
1. **No cleartext sync state.** The store keeps no caller-defined blob per
   record. The highest accepted version vector of a record (§10) is the
   vector inside the record ciphertext the client last accepted: before
   writing a pulled record, the client decrypts the stored record and the
   new one and compares their authenticated vectors. Nothing derived from
   item plaintext (vectors, revisions, device counters) is written in
   cleartext (§11).
2. **Generation floors.** When a vault is deleted, its highest accepted
   `key_generation` is kept in `vault_generation_floors`. Floors never
   decrease and are never deleted. A vault comes back only at its floor or
   above (`KeyGenerationRollback` otherwise). Schema triggers enforce
   this: a vault row cannot be deleted before its floor is recorded.
   A device that never held the vault still trusts the first generation it
   sees, as before.
3. **Hardware-protected TOTP vault (§7.4).** Each vault has an immutable
   `key_wrap`: AK (`wrap/vault`) or TK (`wrap/vault-under-tk`), so the
   caller knows which unwrap applies and a TOTP vault's key cannot be
   re-filed under AK. TK wrappings live in their own table `tk_wrappings`,
   keyed by `(vault_id, method_id)`, with the per-key `prf_salt_hw` in
   `params`. They never share a key with the §6.3 AK wrappings in
   `unlock_methods`, so neither can replace the other. A vault's last TK
   wrapping cannot be removed (that would lose its keys); the vault is
   deleted instead.
4. **No silent loss on delete.** Deleting a record or a vault that has a
   queued local change (a record in the outbox, or an attachment not yet
   uploaded) fails with `PendingLocalChange` unless the caller passes
   `PendingPolicy::Discard`. Deleting a record row is a local purge that
   also drops the record's rollback anchor (item 1). A deletion the user
   makes, including a hard delete from the trash (§7.4), is therefore
   stored as a tombstone, which keeps the anchor. Rows are purged only
   when the server no longer serves the record either.
5. **Re-key and re-wrap.** A held vault key of any generation can be
   re-wrapped in place, but no key is ever added below the accepted
   generation. `rekey_account` applies a §6.6 re-key in one transaction:
   the new password parameters and wrapping, exactly the given set of other
   unlock methods (all other old wrappings are deleted), and the re-wrap of
   every vault key held under AK. If any key under AK is missing from the
   re-key, nothing changes (`RekeyIncomplete`). A crash cannot leave vault
   keys under one AK and the unlock methods under another.
6. **Attachments on a move.** Moving an item to another vault (§7.2)
   re-seals each attachment under a fresh `attachment_id` and FK in the
   target vault. Attachment IDs stay globally unique (the server keys
   storage quotas by them, §12), and the new copy is stored before the old
   one is deleted.
7. **Erasing removed wrappings.** `secure_delete` overwrites deleted rows
   in the database file. Every call that removes or replaces a wrapped key
   also checkpoints the WAL into the database and truncates it to zero
   bytes (`wal_checkpoint(TRUNCATE)`, `journal_size_limit = 0`) before it
   returns. File-system and SSD remnants and backups are out of reach
   (THREAT_MODEL R12).
8. **Open without side effects.** A database is checked read-only
   (`application_id`, `user_version`, schema objects) before the first
   persistent pragma. A foreign or newer file is refused byte-for-byte
   untouched, and its Unix permissions are left as they are. An accepted
   store's files are made owner-only (`0600`, directory `0700`) on every
   open.
9. **Corrupt rows do not block the rest.** Record listings (`records`,
   `outbox`, `records_below_generation`) report a row that fails
   validation as a `CorruptRecord` with its IDs, in place, and still return
   the other rows. `records_below_generation` also lists the queued local
   edits that must be re-sealed after a rotation (§7.1) before upload.
10. **One live session per store** (second M2 integration review,
    2026-10-27). `Store::open` takes an exclusive advisory OS lock on the
    sidecar file `<path>-lock` (`std::fs::File::try_lock`, no new
    dependency) after the read-only check of item 8 and before the first
    write, and holds it until the store is dropped or closed. A second
    open of the same file, from this process or another one, fails at once
    with `Error::StoreInUse`. Two sessions on one store share one device
    ID and each edit from its own decrypted cache, so the second writer's
    upsert replaced the first one's edit without a conflict, a history
    entry or `DeviceIdReused` (sync review t1). The lock file stays behind,
    empty and owner-only on Unix; a foreign file gets none. The lock is
    advisory, so a non-Mosaic program can still open the database; it
    holds only ciphertext.

Amendments (third M2 integration review, 2026-10-27):

- Item 10: the lock file and the attachment directory are named after the
  resolved path (`std::fs::canonicalize`: symbolic links, `.`, `..`,
  relative paths; on Windows without the `\\?\` prefix). Before, a
  symbolic link to the database got its own lock file and attachment
  directory while SQLite followed the link to the same database, so two
  sessions could open one store again. A hard link cannot be detected this
  way (SQLite has the same caveat); with fresh writer IDs per unlock
  (ADR-0015) two sessions on one store through hard links no longer share
  an ID, but each may still overwrite the other's upsert from a stale cache
  (THREAT_MODEL R13).
- `Store::mark_uploaded` returns `false` instead of `StaleServerSeq` for a
  late acknowledgement of a change that a newer queued change (a merge
  with a version pulled after the upload) already superseded; it never
  lowers the stored `server_seq`.
- `Store::readable_vault_keys` lists the wrapped keys of a vault that
  decode and leaves damaged rows out, so a session loading a vault with
  one damaged key row still reads the other generations.

### Alternatives considered
- `PRAGMA locking_mode = EXCLUSIVE`: SQLite takes the lock only at the
  first read or write, a second process waits for the busy timeout before
  it fails with a generic `SQLITE_BUSY`, and in WAL mode the setting also
  moves the WAL index into heap memory. The sidecar lock fails at once
  with a specific error and leaves SQLite's own locking as it is.
- Comparing each local upsert with the stored row (outbox sequence or a
  ciphertext hash) as a second guard: with one live session the cache is
  the only writer's, so the check would only cost a read per edit. Left
  out; revisit if a store ever gets a second writer by design.
- Sealing `accepted_state` under a key derived from AK (for example
  `HKDF(AK, ctx("local-state"))`): this adds a new key and object type to
  the crypto core for a value the stored ciphertext already carries,
  authenticated. Rejected.
- Marking deleted vaults instead of deleting their rows: this keeps
  records, keys and metadata of a vault the user removed. A one-row floor
  keeps only the public generation number. Rejected.
- Storing TK wrappings in `unlock_methods` under a new method byte: this
  would mix AK and TK wrappings in one key space, and a TK wrapping is per
  vault, not per account. Rejected.
- Keying attachments by `(vault_id, attachment_id)`: this keeps IDs on a
  move, but the server sees and keys attachment IDs globally, and the
  per-vault file layout would add complexity. Rejected in favor of fresh
  IDs.
- Keeping a minimal anchor row after a hard record delete: without the
  ciphertext it holds no authenticated vector, and with it the row is a
  tombstone in all but name. Tombstones are the anchor instead.

### Consequences
Schema v1 is final as of the M2 integration commit (which added the
`local_device` table in place, because no store had shipped). Every later
change goes through a migration (`LATEST_VERSION`, CORE_API).
The store's API takes no plaintext-derived state at all. The sync layer
(M6) decrypts the stored record to compare versions, which costs one AEAD
open per pulled record. Leaving and re-joining a vault keeps its floor, so
an owner who really resets a vault must give it a new `vault_id`. Re-keys
must list every held key, which the store can enumerate
(`vault_keys`). Users who purge data locally lose rollback protection for
those records, which is why only tombstones are used for user deletions.

## ADR-0011: Password generator randomness and untrusted site rules

- Status: accepted
- Date: 2026-10-26
- Milestone: M2

### Context
Brief §5 asks for a CSPRNG-only generator with rejection sampling, per-site
rules in Apple's `passwordrules` syntax and a live entropy display.
CRYPTO_SPEC §2 and §13 named `rand::distr::Uniform` over `rand_core::OsRng`.
The extension will read `passwordrules` attribute values from pages that may
be hostile, and review of the M2 generator track found quadratic work on
large rule sets, unzeroized password copies, a slow retry path and
separators that hide word boundaries.

### Decision
1. **No `rand`.** `mosaic-generator` draws from the OS CSPRNG through
   `getrandom` directly, as `mosaic-core` does (ADR-0007 item 14). It reads
   256-byte blocks (`getrandom::fill`), hands out 32-bit words and wipes
   each word from the buffer as it is used; the buffer is zeroized on drop.
   Bytes are used as the OS returns them, never expanded in userspace. A
   crate-private `RandomSource` trait lets tests inject a seeded generator;
   callers cannot supply one.
2. **Rejection sampling.** `uniform_below(n)` rejects draws
   `x < 2^32 mod n` and returns `x mod n`, which is exactly uniform. After
   64 consecutive rejections the source is treated as broken.
3. **Constraints by generate-then-check.** Class minimums and site rules
   are met by generating whole candidates and retrying, never by forcing
   characters into positions, so the result is uniform over qualifying
   strings. The acceptance probability is estimated up front (each kind of
   constraint exactly, their product as an approximation); settings below
   2^-10 are refused (`ConstraintsTooStrict` / `RulesUnsatisfiable`), and at
   most 50 000 candidates are tried (`RetryLimit`).
4. **Site rules are untrusted input.** Rule text above 4096 bytes
   (`MAX_RULES_LEN`) or with more than 32 distinct `required` sets
   (`MAX_REQUIRED_CLASSES`) is malformed, which like any malformed rule set
   means "no rules". The length search moves down as well as up and never
   goes below 4 characters unless the site's `maxlength` is shorter.
   Amended in the third M2 integration review: the code bounded 4096
   characters (up to 16 KiB of UTF-8) and, when no length of 4 or more was
   likely enough, fell back down to the site's `minlength` (1 by default),
   so a hostile `passwordrules` attribute such as
   `allowed: [ab]; max-consecutive: 1; max-sequential: 1;` produced a
   one-character password. The bound now counts bytes before anything is
   parsed, and rules that only a shorter password can satisfy fail with
   `RulesUnsatisfiable` / `ConstraintsTooStrict`; with a site `maxlength`
   below 4 the length is exactly that `maxlength`.
   Amended again in the fourth M2 integration review: the length floor
   alone did not stop a hostile page, because the alphabet can shrink
   instead. `allowed: [a]` produced twenty `a`s (0 bits), and
   `allowed: [abcd]; max-repeating: 1` passed the floor with only the 24
   orderings of `abcd` (4.6 bits). Site rules now also need an entropy
   floor: the alphabet has at least 2 characters, and a length `n` is used
   only if `n × log2(|alphabet|) + log2(p(n))`, the estimated number of
   qualifying passwords (`p(n)` is the acceptance estimate of item 3,
   exact for `max-repeating`), is at least that of a 4-digit PIN
   (`log2(10^4)`, about 13.3 bits). Rules below it fail with
   `RulesUnsatisfiable` (alphabet) or `ConstraintsTooStrict`, and the
   client generates without them. The review offered an exception for a
   site `maxlength` too short for the floor even with the full printable
   alphabet (`maxlength` of 1 or 2); it is not taken, because the page sets
   `maxlength` too, and the exception would let it force a 2-character
   password. A real site with such a limit gets a password generated
   without its rules, which it may reject; the user then picks one by hand.
   Password checks against rules stream the password and keep only
   zeroizing state.
5. **Passphrases.** EFF large wordlist (CC BY 3.0 US, attribution in
   `crates/generator/data/README.md`), 3 to 20 words, separators of
   printable ASCII only. The entropy is flagged as an upper bound when the
   separator could hide word boundaries: without capitalization, an empty
   separator or one other than exactly `-` that contains `a-z` or `-`;
   with capitalization, one that contains uppercase letters. Exactly `-`
   without capitalization stays exact, because the wordlist decodes
   uniquely with it (no hyphenated word starts with another word).
6. **Vendored data.** Apple's `quirks/password-rules.json` (MIT, license
   file kept next to it) and the EFF list are embedded byte-for-byte with
   their SHA-256 recorded; `serde_json` parses the quirks file at first use.

### Alternatives considered
- `rand` / `rand_core::OsRng` with `Uniform`: correct, but two more crates
  for a few lines of tested code over the same OS source.
- Forcing one character of each required class into random positions: not
  uniform over qualifying strings, and detectable (a negative-control test
  shows it).
- Rejecting oversized rule text with an error: the page controls it, and
  "no rules" is the safe fallback that still produces a strong password.

### Consequences
CRYPTO_SPEC §2 and §13 now name `getrandom` and these bounds. The worst-case
generation time is bounded (about 0.1 s in release) and reachable only with
correlated rule sets. M4 passes page rule text to the generator unchanged
and shows "generated without the site's rules" when it was malformed.

## ADR-0012: Session layer over the store and the model (M2)

- Status: accepted
- Date: 2026-10-26
- Milestone: M2

### Context
M2 ends with a frozen core API for M3 to M6 (brief §7, `docs/CORE_API.md`).
The model (ADR-0009) and the store (ADR-0010) each settle their own layer;
what the apps call is the combination: unlock, the decrypted cache, edits as
sync events, applying received records, moves, attachments, key rotation and
one-time codes. Integration testing found one rule that did not hold across
layers: the device-ID reuse check fired on moved items.

### Decision
1. **Typestate lock.** `LockedSession` owns the store and holds no key.
   Unlocking (password + Secret Key, §6.1), creating or joining an account
   yields an `UnlockedSession` holding AK, every VK it could unwrap, the
   decrypted records and the search index. `lock()` consumes it; every key
   and plaintext zeroizes on drop. A failed unlock returns the locked
   session unchanged.
2. **Device ID per store.** Each store holds one device ID (table
   `local_device`, public per §12), created on first unlock. It is replaced
   only by `rotate_device_id` after `DeviceIdReused`. The desktop app and
   the extension use separate stores (ADR-0009 item 7).
3. **Edits are sync events.** Every local change is one model edit by this
   device, sealed under the vault's current key and queued in the outbox.
   An edit closure runs on a copy, so a failing or empty edit stores
   nothing. A password change keeps the old value in the history.
4. **Received records** (CRYPTO_SPEC §10): exactly the highest accepted
   key generation; decrypted under the IDs they were filed under; unknown
   records stored as received; dominated or equal ones ignored; others
   merged with `merge_and_repair`. A result equal to the received record
   is stored byte-for-byte as received (not re-uploaded), anything else is
   re-sealed and queued (`Origin::Merged`). The stored `server_seq` never
   moves back. Vault metadata merges with the pure join (it is never
   deleted, so there is nothing to repair).
5. **Reuse check only for held records.** A record this device holds no
   state for is not checked for `DeviceIdReused`: the copy of a moved item
   keeps the source's version vector, so a device that edited the item
   legitimately sees its own counter on a record it never stored.
   Detection stays best effort, as ADR-0009 item 7 says (it also covers
   equal counters with a colliding stamp since the M2 integration
   review).
6. **Deletes and moves.** "Delete forever" writes a tombstone and removes
   the record's attachment files; nothing user-initiated deletes a record
   row (ADR-0010 item 4). A move writes the copy (fields, history, conflict
   copies) to the target vault under a new item ID, re-seals each
   attachment under a fresh ID and key (ADR-0010 item 6), and only then
   tombstones the source. A crash in between leaves a duplicate, never a
   loss. Trash is the plaintext field `trashed_at` only; the server gets no
   trash hint (CRYPTO_SPEC §7.4 changed accordingly, which removes metadata
   rather than adding any).
7. **Key generations** (§7.1): `rotate_vault_key` accepts generation
   `n + 1` in the store first, then re-seals every readable record and the
   metadata under it and queues the records. Accepting a newer wrapped key
   from the server does the same for everything still below it, not only
   for queued changes: a record this device uploaded under the old key may
   never have reached the rotating device, which rejects it from now on.
   An unlock finishes a re-seal that was interrupted. A different key for
   a generation already held is refused (ADR-0013). Older keys stay held
   for records that could not be re-sealed.
8. **Search index.** In memory only: lowercase zeroizing copies of title,
   user name, email, URLs and tags of live items, never secrets or notes.
   Built at unlock, updated on every change, dropped at lock.
9. **Attachments.** 50 MiB per file (`MAX_ATTACHMENT_LEN`) and 1 GiB per
   account (`MAX_ACCOUNT_ATTACHMENT_BYTES`, sum of the sizes referenced by
   live items), checked before anything is encrypted (§7.3). The file is
   stored before the reference is added; a failed reference removes it.
10. **One-time codes.** `otp_code` converts the item's `OtpConfig` to
    `mosaic-totp` (`mosaic_vault::otp`). For HOTP it saves the advanced
    counter before returning the code, so a code is never shown twice.
11. **Degraded vaults and rows.** A vault whose key does not open under AK,
    or a hardware-protected TOTP vault (§7.4, needs TK, M3), is listed as
    unavailable instead of failing the unlock. A record that does not
    decrypt is listed (`corrupt_records`) and replaced by the next valid
    version received.

Amendments (second M2 integration review, 2026-10-27):

- Item 4: received vault metadata merges with
  `VaultMeta::merge_and_repair` (ADR-0009 amendment), not the pure join:
  the reuse check applies (`DeviceIdReused`), and an equal stamp with
  other contents keeps both names as a conflict and is reported as
  `ApplyOutcome::Merged { stamp_collision: true }`.
- Item 4: renaming a vault without readable metadata (not received yet,
  or damaged) writes new metadata under a fresh one-off writer ID, not
  this device's ID. Restarting this device's counter would reuse stamps
  of the device's lost edits, so the rename could silently replace, or be
  replaced by, a name it never saw. Under a fresh ID it is concurrent with
  any earlier metadata, and the merge keeps both names. Older metadata
  that holds this device's own edits then reports `DeviceIdReused`, as
  for a restored backup.
- Item 11: a vault that is unavailable because its current wrapped key
  row is damaged, missing or does not open under AK is repaired by
  `apply_remote_vault_key` with a wrapping that opens: of the current
  generation, the unreadable row is replaced (`rewrap_vault_key`, or
  `accept_vault_key` if the row is missing); of a newer one, it is
  accepted. Then the vault is loaded (metadata, records, search index)
  and re-sealed if needed. This needs no new rule: the row holds nothing
  this session could ever read, and the new wrapping authenticates the
  vault and the generation under AK. A readable row is never replaced (a
  different key of that generation is still `VaultKeyConflict`), a lower
  generation is a rollback, and a hardware-protected vault stays
  unavailable. If another key row of the vault is damaged, the unlock
  reads the current generation's row on its own. Before, such a vault
  refused every key and stayed locked out until a full re-key, and its
  peer's rotation was never accepted.
- The store's `KeyGenerationRollback` and `VaultKeyConflict` reach callers
  of `apply_remote_vault_key` and `rotate_vault_key` as the session's
  variants, so a rollback is recognized (and dropped, never retried)
  whichever layer caught it.

Amendments (third M2 integration review, 2026-10-27):

- Item 2 is replaced by ADR-0015: the writer ID is fresh for every unlock
  and only recorded in `local_device`, never read back.
- Item 4: a merge that does not fit the item limit is split
  (`ApplyOutcome::Split`, ADR-0016). A record whose local row exists but
  cannot be read is replaced by the received version and reported as
  `ApplyOutcome::ReplacedUnreadable` (no rollback anchor; THREAT_MODEL
  R17), not as `New`. The rollback mapping above also covers re-sharing a
  vault that was deleted locally below its generation floor.
- Item 11: loading a vault reads every key row that decodes
  (`readable_vault_keys`), so one damaged row costs only its own
  generation; a repair by a newer generation then re-seals every readable
  older record and the metadata (tested).
- Item 7: `rotate_vault_key` re-seals records and metadata only, not
  attachments. That is enough while no other account can hold a vault key
  (M2); revocation needs ADR-0017.

Amendments (fourth M2 integration review, 2026-10-27):

- Item 4: received vault metadata that replaces a metadata row the session
  could not read (damaged, or sealed under a generation whose key is not
  held) is reported as `ApplyOutcome::ReplacedUnreadable`, like a record
  (THREAT_MODEL R17), not as `New`; the session remembers at load that such
  a row existed.
- Item 4: an oversized merge now reports `ApplyOutcome::Split
  { conflict_copies }` (a count; the copies carry `conflict_copy_of`), see
  the ADR-0016 amendment.

### Alternatives considered
- A single session type with an `is_locked` flag: every method would need a
  runtime check, and a forgotten one would read a dropped key.
- Giving a moved item a fresh version vector: needs a rebasing API in the
  model that drops stamps of conflict copies, or loses them. Rejected in
  favor of item 5.
- Re-uploading every merged record: costs an upload per received record
  and keeps devices sending each other identical ciphertext.
- Persisting the search index encrypted: §11 keeps it in memory only.

### Consequences
M3 builds the UI on `UnlockedSession`; M6 adds the network around
`pending_uploads`, `mark_uploaded` and the `apply_remote_*` methods. Open for
M6: wrapped vault keys and vault metadata have no upload queue yet (the
store keeps them, and M6 must add pending flags or upload them on change);
the server should refuse uploads below a vault's current key generation
(a peer re-seals what it holds once it accepts the new key, item 7), and
accepts one wrapped key per vault and generation (ADR-0013). A rotation
makes every device upload every record of the vault once; peers ignore
the duplicates. Attachments: a delete or move that loses against a
concurrent edit has already pruned the files on the deleting device, so
the M6 server garbage-collects an attachment blob only once no live record
version references it, and a device fetches the file of any attachment
reference it holds no file for. Known gaps of the merge (PROGRESS M2-R7 to
M2-R10, THREAT_MODEL R15) are left for M6, where the server's record
history makes their fixes possible.

## ADR-0013: Concurrent vault-key rotations and re-sealing on acceptance

- Status: accepted
- Date: 2026-10-26
- Milestone: M2

### Context
The M2 integration review (crypto, sync-chaos) found that two devices that
rotate one vault at the same time each create a random key for the same
generation `n + 1`. The store accepted a wrapping of a generation it
already held by replacing it, and the session replaced the key in memory.
Both wrappings are valid AK wrappings under the same AAD, so nothing
noticed: every record the device had sealed under its own key became
unreadable for good, on every device. The review also found that a device
accepting a newer key re-sealed only its queued changes, so a record it
had uploaded under the old key, and that the rotating device never
received, stayed visible on its author only (peers reject the old
generation, §7.1).

### Decision
1. **Never replace a held key.** `apply_remote_vault_key` compares a
   received key of a generation it already holds with the held one in
   constant time (`VaultKey`'s equality uses `subtle`): the same key is a
   no-op, a different one fails with `Error::VaultKeyConflict
   { generation }` and changes nothing. `Store::accept_vault_key` refuses
   any wrapping of a held generation other than the stored bytes
   (`storage::Error::VaultKeyConflict`); `rewrap_vault_key` is the only way
   to replace a wrapping.
2. **Resolution (M6).** The server accepts exactly one wrapped key per
   vault and generation, by compare-and-swap on the generation. A device
   uploads a rotation's wrapped key, and has it accepted, before anything
   sealed under that generation. When the server refuses it (another
   device's rotation won), the device rotates again to `n + 2` from its own
   state, which re-seals everything it holds; the winner accepts `n + 2`
   and re-seals what it holds, so both converge and nothing sealed under
   the losing key ever leaves the device. Until M6, the conflict is
   reported and both keys stay where they are; records sealed under the
   other key fail to authenticate (`ItemCorrupted`) and change nothing.
3. **Re-seal on acceptance.** Accepting a newer generation (a rotation or
   a received key) re-seals every readable record and the metadata below
   it under the new key and queues the records; unlock finishes a re-seal
   that was interrupted (`records_below_generation`). Peers that already
   hold the same version ignore the duplicate.

### Alternatives considered
- Keeping both keys of one generation: the store and the AAD identify a
  key by vault and generation only, so every object would need a key ID,
  a format change for a case the server can prevent.
- Letting the later key win: loses everything sealed under the earlier one.
- Re-sealing only queued changes and relying on merges: a version the
  rotating device never held never reaches it again.

### Consequences
`Error::VaultKeyConflict` is a new (additive) session error; CORE_API
lists it. A rotation costs one upload of every record per device. CRYPTO_SPEC
§7.1 states the rule; M6 implements the compare-and-swap and the retry.

## ADR-0014: Visual direction and the holo-foil primary action

- Status: accepted
- Date: 2026-10-27
- Milestone: M3 (design, ahead of the desktop UI)

### Context
The owner reviewed the screen mockups ("Mosaic — App Screens" canvas,
snapshot in `docs/design/`) over several rounds. They chose:
- a near-monochrome look in the Linear/shadcn style;
- DM Sans for the UI and IBM Plex Mono for secrets;
- conventional status colours;
- one signature primary-action treatment: a holographic foil with angled
  rainbow stripes, picked from WebGL studies after flat accent colours
  (blue, then green) were rejected.

### Decision
1. **Tokens.** `packages/design-tokens` holds:
   - the zinc palette, with near-black `accent` (inverted in dark mode)
   - radii of 6, 12 and 14
   - DM Sans and IBM Plex Mono, both SIL OFL 1.1, bundled with each app and
     never loaded from a CDN
   - `primaryAction`, which describes the foil
2. **One holo button per screen.** The single primary action of a screen
   renders as a fully round pill or circle with the holo foil. Its label
   and icon are `#09090B` on light and dark themes alike. Nothing else uses
   the foil.
3. **Implementation.** The foil is rendered by the dependency-free WebGL2
   module prototyped in `docs/design/holo/holo-button.js`: one shared
   context, one canvas, and on-demand rendering. For M3 it is ported to
   TypeScript inside the app. The rules for that port:
   - it is cosmetic only and runs no logic tied to secrets
   - the label stays real DOM text
   - `prefers-reduced-motion` turns the animation into a still frame
   - frames that stay slow downgrade the rendering
   - without WebGL2, or if a shader fails, it falls back to a flat
     `#E4E4E7` button
   - the Tauri CSP allows no remote resources for it
4. **Mockups.** The design-canvas mockups use a baked still of the foil,
   since the canvas format cannot run WebGL.

### Alternatives considered
- **A flat brand accent** (electric blue, then viper green): both tried and
  rejected by the owner as too colourful or odd.
- **CSS gradients** for the foil: rejected as looking cheap. A baked image
  is used only in the static mockups.
- **Other shader studies** (AR glass, particles, prism, aurora, liquid
  metal, HUD): explored and dropped.

### Consequences
- The desktop and extension UIs depend on WebGL2 for the signature look but
  stay fully usable without it.
- The extension popup must keep the shader inside its own page and never
  inject it into websites (brief §4).
- An accessibility review must confirm the label contrast over the brightest
  part of the foil, at least 4.5:1, in both themes.

## ADR-0015: Fresh writer IDs per unlock, and merges that survive an ID reuse

- Status: accepted
- Date: 2026-10-27
- Milestone: M2

### Context
The third M2 review (sync-chaos, high) showed that a reused device ID loses
data silently on every device. Stamps are max+1 Lamport counters, so two
writers with one ID (a restored backup or a copied profile, which kept its
ID because it lived in the database) produce the same stamp `(X, c)` for
different edits. When they edit different fields, each side's version
vector covers the other's stamp, and the pure merge drops both sides'
values and the values they superseded; the collision check (same stamp,
same field) sees nothing. In the restored-backup variant the reuse is
detected (`DeviceIdReused`), but the documented recovery, rotating the ID
and merging again, loses the same values: the old ID's stamps are still
covered.

### Decision
1. **Fresh writer ID per unlock.** `UnlockedSession` creates a new
   `DeviceId` at every unlock, account creation and join, and records it in
   the store's `local_device` row (diagnostics only, never read back). A
   restored backup or a copied store therefore never writes under an ID
   another copy uses; with the store lock (ADR-0010 item 10) two writers
   never share one. What can still carry a session's own stamps is a forged
   or corrupted record (or a UUIDv7 collision), so the checks below stay as
   defence in depth. CRYPTO_SPEC §10 says "writer ID"; the M6 server's
   device identity (sessions, §12) and the freeze checkpoints (§10) use a
   stable per-install ID of their own, decided in M6.
2. **Equal vectors with different contents are a collision.** Honest
   replicas with equal vectors have seen the same edits and are equal. When
   they differ, `merge_and_repair` writes back every value either side held
   that the merge dropped, as local edits of the merging device, and sets
   `stamp_collision`. If one of the differing entries carries the merging
   device's own ID, it is `DeviceIdReused`.
3. **Merging after a rotation.** `rotate_device_id` keeps the old ID as
   retired for the rest of the session, and the session merges with
   `merge_and_repair_after_reuse(received, device, retired)`. For a retired
   ID that collides in a record (the received side has a counter for it
   above the local one, or both hold one of its stamps in a field with
   other contents), the
   local entries under it that the received side does not hold are taken
   out before the merge and written back under the new ID, and the local
   vector stops covering that ID, so the received side's entries under it
   survive as well. A value one writer had superseded can come back as a
   conflict copy; that is the price of losing nothing.
4. **Display.** Written-back values keep their write times, and per field
   the value written last by the wall clock is shown (it is written on top
   again if needed). The wall clock only picks what is displayed among
   values that cannot be ordered; it never decides what is kept.

Amendments (fourth M2 integration review, 2026-10-27):

- The recovery is now tested under random histories: two writers under one
  ID and honest devices edit the title, the password, tags and URLs from
  one base; once the writers have met (each applying the other's version,
  rotating on `DeviceIdReused`), any gossip order converges and loses no
  value that no edit superseded (`reused_writer_ids_lose_nothing_once_the_writers_meet`,
  and the `item_merge` fuzz mode 2). The limit is pinned as well: an honest
  device that merges two colliding versions before the writers met, where
  the vectors differ, the counters are equal and no field holds one stamp
  with two contents, sees nothing and drops both writers' values, as the
  join must (`reuse_that_shows_nowhere_is_not_detected`). That case needs
  a forged or corrupted record, since writer IDs are fresh per unlock
  (item 1); the checks stay defence in depth, as item 1 says.
- A value written back into a `history:*` key gets a key of its own
  (ADR-0009 amendment).

### Alternatives considered
- A stable install nonce outside the database (OS keystore, M3): a restored
  backup on the same machine finds the same keystore entry and reuses the
  ID, which is exactly the failing case.
- Keeping the store's ID and detecting a restore (a counter outside the
  database): fragile, and a copy taken together with its sidecar is
  undetectable.
- Retiring an ID for good across sessions: after the restored install
  rotates, the old install keeps writing under the old ID legitimately, and
  every one of its later edits would resurrect superseded values.

### Consequences
Version vectors gain one entry per session that edits a record (about 44
bytes of plaintext each). Normal items are edited rarely; an HOTP item gains
an entry per session that shows a code. Pruning vector entries is a future
format change. The reuse tests now force a shared ID in unit tests, and an
integration test copies a store file and checks that both copies edit
without loss. M2-R3 is closed except for hard links (ADR-0010 amendment).

## ADR-0016: Merges that exceed the item size limit

- Status: accepted (items 1 and 2 replaced in the fourth M2 integration review, 2026-10-27)
- Date: 2026-10-27
- Milestone: M2

### Context
The third M2 review (sync-chaos, medium) found that two sealable replicas
can merge into an item above the 1 MiB plaintext limit: no single field has
a size cap, and conflict copies add up inside one register. With a 600 kB
note edited on two devices, `apply_remote_record` failed with `SizeLimit`
on both, every time; neither stored the other's version, so the devices
diverged for good and each edit never reached the other device.

### Decision
1. **Split instead of failing.** When the repaired merge does not fit,
   `Item::split_oversized(received, item_id, device, at)` decides, the same
   on every device:
   - The record keeps one of the two versions: the one with the longer
     canonical plaintext (then the larger bytes). Attachment references of
     both are merged as usual, because their files are bound to the record
     ID by the attachment AAD. The version vector is the join of both, so
     the result dominates both inputs and every device converges on it. It
     is a pure function of the two versions (no new stamp).
   - The device whose own version was not kept stores it as a new item of
     the same vault (fresh item ID, without attachment references and the
     deletion flag), marked `conflict_copy_of = <item ID>` (a new item key,
     metadata like `trashed_at`, shown to the user, dismissable). A deleted
     version gets no copy.
   The session stores the copy first, then the kept version (queued), and
   reports `ApplyOutcome::Split { conflict_copy }`.
2. **Fallbacks.** If the kept version with the other side's attachment
   references does not fit, it is kept without them; if even that does not
   fit (both versions within a few bytes of the limit), the error stays.
   The mark is left off a copy it would push over the limit.
3. **Vault metadata** (64 KiB, three short fields) is not split; an
   oversized metadata merge still fails (PROGRESS M2-R16).

Amendment (fourth M2 integration review, 2026-10-27): items 1 and 2 are
replaced. The split was a function of the pair of versions a device merged
and of which side was local, and that broke the rule it was meant to keep:
- Only the device whose own version lost made a copy. A device that kept
  its own version made none, and its peer, pulling the split record first,
  fast-forwarded to it (it dominates the peer's version) and never split,
  so the peer's committed edit was on no device (sync review, high).
- Whether a split happened depended on the repaired merge, which differs
  by device (only a replica that missed a deletion writes values back).
  One device split while its peer stored the plain merge under the same
  vector, the peer got `DeviceIdReused`, and the split record had dropped
  the deletion entry, so the delete was undone without a notice (sync
  review, medium).
- Three devices that each merged a different pair reached the same joined
  vector with different kept versions, and the equal-vector collision
  check then raised `DeviceIdReused` on honest devices (found by the new
  oversized case of the sync simulation).

New decision:
1. **The record is a function of the merge result.** When the repaired
   merge does not fit, `Item::split_oversized(received, item_id,
   vault_key, device, reused)` reduces that merge: every content field
   keeps only the value it shows (its winner); if the record still does
   not fit, whole content fields leave it, the largest encoded value first
   (then the larger key). Metadata, `history:*`, the counters, attachment
   references (bound to the record ID by the attachment AAD) and the
   `deleted` entries always stay, so a concurrent delete keeps its notice.
   The vector is the merge's. Every device that reaches the same merge
   result stores the same record, whatever pair it merged in which order;
   a device whose repair added write-backs stores a record that dominates
   the others', so they fast-forward to it.
2. **Every value that left is a conflict copy, built the same everywhere.**
   A value other than a repeat of what the field still shows becomes a new
   item of the vault holding only the value under its key with its
   original stamp and `conflict_copy_of` stamped like the value (left out
   if it would not fit), with a vector of exactly that stamp. No new stamp
   and nothing from the device's record (not even the type; the UI shows
   the original's through the mark), so every device builds the identical
   copy. A first version took the type and creation time from the record:
   the sync simulation's oversized case then saw a device's copy carry the
   receiving device's own creation stamp that its local copy lacked, which
   read as `DeviceIdReused`.
3. **Deterministic copy IDs** (reverses the alternative rejected below):
   `UUIDv8(HKDF(VK, "", ctx("conflict-copy-id") || item_id ||
   u32(len(key)) || key || counter:u64 || device_id, 16))` under the
   vault's current key, with the value's stamp. HKDF-SHA-256 is the
   primitive §2 already lists for every derivation; the label is new and
   used for nothing else (§3). The ID depends only on the record and the
   value, so copies made by different devices are one record instead of
   duplicates. The server sees a random-looking record ID, as for any item
   (§12); without `VK` it cannot link the copy to the record. A device
   stores a copy only if it holds no record of that ID (it may have been
   received, edited or deleted since). Two devices at different key
   generations derive different IDs; that leaves a visible duplicate, not
   a loss.
4. **Fallback.** If the record does not fit even without any content field
   (its history or attachment references alone exceed the limit), or a
   value does not fit a copy of its own, the merge fails with `SizeLimit`
   and nothing changes. The old fallback that dropped the other side's
   attachment references while the joined vector covered them, so that
   their files were pruned, is gone (sync review, low).
5. The session reports `ApplyOutcome::Split { conflict_copies }`, the
   number of copies; they carry `conflict_copy_of`.

What a user sees changes: the record shows what the merge would have shown,
and each conflict copy of a field the merge could not hold is an item of its
own, rather than one whole version kept and the other copied.

### Alternatives considered
- Capping each value so the worst-case union fits: the number of concurrent
  copies is unbounded, so no cap guarantees it, and large notes would be
  refused at edit time.
- Dropping the oldest conflict copies: silent loss.
- Keeping each device's own version: the devices never converge.
- A deterministic copy ID (for one copy instead of one per device whose
  version lost): it would have to be derived from item content and would
  then be visible to the server, or need a new key derivation. Duplicate
  copies are visible and lose nothing. Reversed by the fourth-round
  amendment: without one, the device that kept its own version had to make
  no copy, which lost the other version (see above); the key derivation
  uses HKDF under VK, so nothing becomes visible to the server.
- (Fourth round) Making every device that splits copy the version it does
  not keep, but keeping the pair-based choice of the record: three devices
  that merge different pairs still reach one vector with different
  records.
- (Fourth round) Joining the deletion entries into the kept version and
  splitting only when the plain join does not fit: fixes the two-device
  delete race but not the three-device case above.

### Consequences
A third device that held the losing version also makes a copy, so a split
can leave identical copies; the user deletes extras. Attachments that only
the losing version referenced stay with the record (the merge keeps them),
never in the copy. CRYPTO_SPEC §7.5 lists the new key and §10 the rule.
(Fourth round: copies made by different devices are now one record; a
split leaves one copy per value that left the record, and the record keeps
the shown value of every field. CRYPTO_SPEC §3 and §10 state the copy ID.)

## ADR-0017: Attachment integrity across a revocation

- Status: accepted (rule for M6/M7; nothing to implement in M2)
- Date: 2026-10-27
- Milestone: M2

### Context
The third M2 review (crypto, medium) found that a vault-key rotation, the
revocation mechanism of CRYPTO_SPEC §8.3, keeps every attachment's FK and
attachment ID. The FK sits in item plaintext the removed member already
decrypted, and the attachment AAD carries no key generation. A removed
member and the server together could therefore replace any existing
attachment's ciphertext with one that authenticates, and clients would
accept it under the unchanged FK from the current item plaintext. In M2 no
other account can hold a vault key (sharing is M6, rotation as revocation
M7), so the gap is not reachable yet.

### Decision
Revoking a member re-seals every attachment of the vault under a fresh
attachment ID and a fresh FK, as a move does (ADR-0010 item 6), and updates
the references in the same pass that re-encrypts the items; the old
ciphertexts are deleted on the server. The revoking client fetches every
attachment first; one it cannot fetch is listed in the revoke UI and its
reference is removed or re-sealed once fetched. Until the pass finishes,
the UI says that attachments are not yet protected against the removed
member. CRYPTO_SPEC §8.3 states the rule. The M2 `rotate_vault_key` (no
sharing yet) does not re-seal attachments and is documented so.

### Alternatives considered
- A key generation in the attachment AAD: a format change for every file
  that does not help on its own, because the attack needs only the FK,
  which the removed member keeps and can seal under any AAD. Only a fresh
  FK (in plaintext the removed member cannot read) removes it.
- Accepting the gap and showing it in the revoke UI: possible, but the
  re-seal is cheap next to re-encrypting every item.

### Consequences
A revocation uploads every attachment of the vault again. M6/M7 implement
the pass with the sharing and revoke flows; THREAT_MODEL lists the gap
until then.
