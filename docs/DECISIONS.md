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

### Amendment (UI fidelity, 2026-11-02)
The radii 4, 8 and 10 (alongside 6, 12 and 14) and the weight 700 for page
titles come from the approved mockups in `docs/design/screens`. They are
tokens in `packages/design-tokens` (`radius`, `size`). This changes no rule
about the foil.

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

## ADR-0018: Desktop IPC types are hand-written on both sides and checked by generated fixtures

- Status: accepted
- Date: 2026-10-27
- Milestone: M3

### Context
The desktop webview (TypeScript) and its Rust core share about 35 commands,
their requests and responses, events and one error enum
(`docs/DESKTOP_IPC.md`). Five tracks build on that contract in parallel, so
the two sides must not drift, and a mismatch has to fail CI rather than show up
at runtime. The usual tools generate TypeScript from Rust:
`tauri-specta`/`specta` (2.0 still a release candidate, rc.26 at this date) or
`ts-rs`.

### Decision
1. **Hand-written types on both sides.** Rust in
   `apps/desktop/src-tauri/src/contract/`, TypeScript in
   `apps/desktop/src/ipc/contract/`, one module per area, with the same names.
   Wire rules: camelCase JSON; tags in `kind`, `state`, `op` or `type`; every
   key always present (`null` for "no value", no optional properties in
   TypeScript); requests reject unknown keys; string enums are defined with
   `wire_enum!` in Rust and as `as const` arrays in TypeScript.
2. **Generated fixtures as the check.** A Rust test builds samples of every
   request (per command, each one round-tripped through the Rust
   deserializer), every response, every event variant, every error code and the
   value list of every string enum. It writes them to
   `apps/desktop/src/ipc/contract.fixtures.ts` as
   `export const fixtures = {...} satisfies ContractFixtures`. The test fails
   when the file is stale (`MOSAIC_UPDATE_FIXTURES=1` regenerates it).
   - `tsc` then checks every sample against the TypeScript types, including
     excess properties, missing keys and unknown tags. `ContractFixtures`
     maps every command name to its request and response types, so a command
     that exists on one side only also fails.
   - A Vitest test compares the command list and every string enum as sets,
     which catches TypeScript values that Rust never emits.
   - Exhaustive `match`es in the sample builders stop a new Rust variant from
     compiling until it has a sample.
3. **One command list.** `src-tauri/src/commands/names.rs` is included by
   `build.rs` (the ACL manifest) and read by the fixture and capability tests.
   A mock-runtime test invokes every command through the real config and ACL
   to prove that it is registered and allowed.

### Alternatives considered
- **tauri-specta / specta:** generates bindings from the command signatures.
  Rejected for now. It is a release candidate, so pinning a pre-release that
  sits in every command's type would need an ADR. Its proc-macros add a
  dependency on every contract type. The generated code would need review on
  each regeneration. Revisit when 2.0 is stable. The hand-written types map
  one to one.
- **ts-rs:** generates TypeScript per type but not the command map, so it
  still needs a parity check for commands, and it adds a derive to every type.
- **Runtime validation (zod) in the webview:** it catches drift only at
  runtime and adds a dependency. The fixtures catch it at build time.

### Consequences
A contract change touches both sides and regenerates the fixtures. The
procedure is in `DESKTOP_IPC.md` §8. The checks cover shapes and value sets,
not semantics: a field Rust fills wrongly still type-checks. The fixtures also
document the contract by example.

## ADR-0019: Desktop shell security baseline and the M3 dependencies

- Status: accepted
- Date: 2026-10-27
- Milestone: M3

### Context
M3 starts the Tauri v2 desktop app (brief §2). The webview renders vault
content and is the largest attack surface on the desktop (THREAT_MODEL §4.2).
The skeleton fixes the baseline that every M3 track builds on, and the
dependencies it needs.

### Decision
1. **Tauri 2.12 without default features** (`tauri =2.12.5` with `wry`,
   `compression` and `x11`; `tauri-build =2.7.4`). Left out:
   - `dynamic-acl`, so capabilities are fixed at build time
   - `dbus` (tao's XDG portal)
   - `devtools`, so release builds have no web inspector
   - `common-controls-v6`: native menus and dialogs, which Mosaic does not use.
     Without it, `cargo test` runs on Windows, where test binaries carry no app
     manifest.

   No shell, fs, http, dialog, clipboard or updater plugin is installed. The
   one plugin is `tauri-plugin-single-instance =2.5.4`, registered first: a
   second launch focuses the running window, so one install never opens two
   store sessions (CORE_API; the store lock also refuses them).
2. **ACL: only Mosaic's commands.** `build.rs` declares every command in an
   app manifest. That makes Tauri check each invoke against
   `capabilities/main.json`, which grants exactly the `allow-<command>`
   permissions for the window `main`: no `core:*` permission and no remote
   origin. Events from Rust use a `Channel` passed to `subscribe_events`
   instead of `core:event`. Tests check the capability against the command
   list and invoke every command through the real ACL.
3. **CSP.** Production:
   `default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src ipc: http://ipc.localhost; base-uri 'none'; form-action 'none'; frame-ancestors 'none'; object-src 'none'`.
   Tauri adds hashes for its own init scripts. The rules that keep this
   working:
   - No `unsafe-eval` and no remote origin.
   - No `unsafe-inline` styles. The design tokens become a static stylesheet
     (a Vite virtual module), and React sets `style` through the CSSOM.
   - Vite never inlines assets as `data:` URIs.

   `devCsp` relaxes only what Vite's dev server needs (inline styles and
   scripts, its WebSocket) and applies only to `tauri dev`. Also set:
   `freezePrototype: true` and `withGlobalTauri: false`. Playwright runs the
   mock build under the production CSP and fails on any violation. A Rust test
   pins the CSP's key directives.
4. **Isolation pattern: deferred to M9.** THREAT_MODEL §4.2 lists it for
   "webview reaches the Tauri API surface". Tauri's isolation feature
   encrypts IPC between a sandboxed iframe and the core with AES-GCM, a
   crypto dependency outside CRYPTO_SPEC §2. Its hook could only re-check what
   Rust already validates: the command set is closed, inputs are typed and
   checked in Rust, and no remote or third-party content is loaded. M9
   revisits it together with the frontend dependency review. THREAT_MODEL
   §4.2 is updated accordingly.
5. **Secrets at the boundary.** Secrets enter as `SecretIn` and leave only as
   `SecretOut`, in four responses. Copying to the clipboard, import file
   reading and SSH signing run in Rust (`DESKTOP_IPC.md` §6). THREAT_MODEL R19
   records the residual exposure in the webview.
6. **Fonts.** DM Sans and IBM Plex Mono come from the `@fontsource` packages
   (OFL-1.1). Vite bundles them, and the build ships the OFL texts as
   `licenses/*-OFL.txt`. Nothing loads from a CDN, and a Playwright test fails
   on any request leaving the app's origin.
7. **ICU formatting:** `intl-messageformat =12.1.4` (FormatJS, BSD-3-Clause)
   in `packages/i18n`. This settles the "formatter is chosen in M3" note there.
8. **Mock IPC.** It is selected by build mode. Non-mock builds replace it with
   a stub at build time, so no mock code or sample data ships.
9. **License and advisory exceptions (cargo-deny, cargo-audit),** each scoped
   to named crates or IDs:
   - MPL-2.0 for `cssparser`, `cssparser-macros`, `dtoa-short` and `selectors`
     (Tauri's build-time HTML handling) and `option-ext` (the `dirs` stack).
     MPL-2.0 is file-level copyleft: it binds changes to those files only, and
     Mosaic uses them unmodified.
   - RUSTSEC-2024-0370 (`proc-macro-error`, unmaintained, a build-time
     proc-macro of gtk-rs 0.18).
   - RUSTSEC-2024-0429 (`glib::VariantStrIter` unsoundness). Nothing in the
     tree calls it; checked in tao, wry, muda, tauri, gtk and webkit2gtk.
   - RUSTSEC-2026-0370, -0371 and -0372 (`zvariant`, `zbus_names` and
     `zvariant_derive` folded into zbus 6). zbus 5 comes through the
     single-instance plugin on Linux.

   All are Linux-only or build-time, and none is an exploitable
   vulnerability. Revisit when Tauri moves to gtk4 and zbus 6.
   `.cargo/audit.toml` mirrors the advisory list.
10. **JavaScript dependencies,** all exact pins:
    - App: `react`/`react-dom 19.3.1`, `@tauri-apps/api 2.12.4`, the two
      `@fontsource` packages `5.3.0`.
    - Dev: `vite 8.3.5` (already in the tree through Vitest),
      `@vitejs/plugin-react 6.1.2`, `@tauri-apps/cli 2.12.4`, `jsdom 29.1.1`
      (30.x needs Node 22.22), `@testing-library/react 16.3.3`, `/dom 10.4.2`
      and `/user-event 14.6.8`, `@playwright/test 1.64.1`,
      `@axe-core/playwright 4.14.0`, `@types/react`/`-dom 19.3.0`, and
      `eslint-plugin-react-hooks 7.1.1` (rules of hooks).

    None of them needs an install script, so `onlyBuiltDependencies` stays
    empty. Playwright browsers are installed explicitly
    (`playwright install chromium`).

### Alternatives considered
- **Tauri default features:** they bring in dynamic ACL and the D-Bus portal
  for nothing Mosaic uses.
- **The official clipboard and dialog plugins:** they expose JavaScript APIs
  that would put secrets and file contents into the webview. Their Rust sides
  (or `arboard` / `rfd`) can be used from Rust by the tracks without granting
  the webview any permission.
- **Allowing MPL-2.0 globally:** rejected. Exceptions per crate keep any new
  MPL dependency visible.
- **Dropping single-instance** to avoid zbus 5: the store lock already refuses
  a second session, but the second launch would then show an error instead of
  focusing the open window.

### Consequences
The webview can do nothing but call Mosaic's commands. Every new command
needs its permission in the capability (a test enforces it). The
Linux-only advisories stay until upstream moves. Tracks that need native
dialogs or the clipboard add Rust-only crates with a justification, and an ADR
if a crate is security-relevant (the OS keystore crate, an SSH crate).

### Amendment (2026-11-04): navigation guard
The red-team review of the desktop IPC (THREAT_MODEL R33) found two gaps in
this baseline: the CSP does not cover top-level navigations, and Tauri's IPC
origin check ignores the port, so `http://tauri.localhost:<port>/` served by
any local process would count as the app. A navigation guard
(`apps/desktop/src-tauri/src/navigation.rs`, a plugin on the app builder)
now refuses every load except the bundled frontend (`http://tauri.localhost`
on Windows, `tauri://localhost` elsewhere, with no port or credentials). The
Vite dev server is allowed only in builds without the `custom-protocol`
feature, which `tauri build` always sets. `window.open` stays refused (no
new-window handler). Tests in `redteam_surface_tests.rs` check that the guard
is registered and what it refuses.

## ADR-0020: Desktop shell OS integration (Secret Key store, clipboard, auto-lock, shortcut, dialogs, Emergency Kit)

- Status: accepted
- Date: 2026-10-29
- Milestone: M3

### Context
M3 needs the Secret Key kept on the device (CRYPTO_SPEC §5.2, CORE_API), a
clipboard that clears itself and stays out of clipboard history (UX_GUIDELINES
§8), auto-lock on idle, sleep and screen lock (CRYPTO_SPEC §11), a global
shortcut (UX_GUIDELINES §4) and an Emergency Kit PDF, all under
`forbid(unsafe_code)` and without webview plugin APIs (ADR-0019).

### Decision
1. **Secret Key store:** `keyring =3.6.3` (`windows-native`, `apple-native`,
   `linux-native`): Windows Credential Manager (DPAPI, per user), the macOS
   login Keychain, the Linux kernel keyring (keyutils; per login session, lost
   at logout or reboot, after which the unlock screen asks for the key). Secret
   Service is not used: keyring's backends for it need DH and AES transport
   crypto outside CRYPTO_SPEC §2. An in-memory layer sits in front, so a
   keystore failure is never fatal: the user types the key from the kit.
   Amendment: the in-memory layer holds the Secret Key only when the OS
   keystore write failed (kept for the process lifetime so the kit can still
   be saved); OS reads are not cached; `secret_key_stored` caches only a
   boolean.
2. **Clipboard:** `arboard =3.7.0`, written from Rust only. Windows:
   `ExcludeClipboardContentFromMonitorProcessing`,
   `CanIncludeInClipboardHistory = 0`, `CanUploadToCloudClipboard = 0`; macOS:
   `org.nspasteboard.ConcealedType`; Linux X11/XWayland:
   `x-kde-passwordManagerHint`. It clears after the configured time on the
   monotonic or the wall clock, whichever comes first, only if the clipboard
   still holds Mosaic's value (constant-time compare with `subtle`), and on
   lock and quit. License exception BSL-1.0 for `clipboard-win` and
   `error-code` (deny.toml).
3. **Auto-lock:** idle is `max(monotonic, wall)` since the last command or
   window focus; a resume is detected by a clock gap (tick 2 s, gap 30 s);
   Linux also uses logind `PrepareForSleep` and the screen-saver
   `ActiveChanged` signal through `zbus =5.19.0` (already in the tree). Windows
   and macOS sleep, screen-lock and user-switch notifications are not used:
   they need unsafe FFI. The triggers cannot all be turned off (§11):
   `update_settings` refuses it and a state file that says so is read as the
   default idle timeout. `app_status` reports whether screen-lock detection
   exists, and the settings screen says so.
   Amendment: a trigger counts only if this system can deliver it: screen lock
   counts only where `screenLockDetection` is true (today Linux with the D-Bus
   source running); sleep counts everywhere (clock-gap detector). The idle
   tick applies the same floor at runtime.
4. **Global shortcut:** `tauri-plugin-global-shortcut =2.4.1`, registered
   from Rust during setup (a failure is reported as `conflict` or
   `unavailable`, never fatal), with no JavaScript permission. It focuses the
   main window and, while unlocked, emits `quickSearchRequested`. No second
   webview.
5. **Dialogs:** `rfd =0.16.1` (`gtk3`) called from Rust, without Common
   Controls v6 (ADR-0019).
6. **Emergency Kit:** `pdf-writer =0.15.0`, A4, base-14 Helvetica and Courier
   (WinAnsi), no embedded or external objects. Rendered only after the save
   dialog returned a path, into preallocated buffers that are zeroized after
   writing; a half-written file is removed. The Secret Key comes from the
   keystore inside Rust; the webview sends only the labels. Outside onboarding
   (from account creation until the kit is verified or the vault locks),
   saving needs the master password.
7. **Quick unlock, Windows calls (pins CRYPTO_SPEC §6.2):**
   `KeyCredentialManager.IsSupportedAsync`,
   `RequestCreateAsync("Mosaic quick unlock {account_id}", ReplaceExisting)`,
   `OpenAsync(name)`, `KeyCredential.RequestSignAsync(challenge)` with a
   32-byte challenge from `SecretBytes::random` (enrollment signs twice and
   compares in constant time), `DeleteAsync(name)`, through `windows =0.62.2`
   WinRT projections (no unsafe). `W` is derived in `mosaic-core`. Quick unlock
   is **not offered in M3**: `mosaic-vault` has no session methods for the
   method-0x02 wrapping yet, so `unlock_quick` answers `quickUnlockUnavailable`
   without any OS prompt.
8. **Screen capture:** `contentProtected` on the main window (Windows
   `WDA_EXCLUDEFROMCAPTURE`, macOS `NSWindowSharingNone`; no effect on Linux).

### Alternatives considered
- `tauri-plugin-dialog`: forces Common Controls v6, which breaks `cargo test`
  on Windows.
- `tauri-plugin-clipboard-manager`: no exclusion formats, and a JavaScript API.
- keyring's Secret Service backends: transport crypto outside §2.
- A quick-search second window: more surface, and it needs secret commands in
  a second capability.
- Unsafe FFI for the Windows and macOS sleep and lock notifications:
  forbidden; M9 may revisit it with a reviewed exception.

### Consequences
PROGRESS M3-R1 is partly resolved (Linux keeps the key per session only).
The gaps are PROGRESS M3-R5 and M3-R7 to M3-R11 and THREAT_MODEL R21.
CRYPTO_SPEC §5.2 names the Linux kernel keyring.

## ADR-0021: Import parsers and the read-only KDBX primitives

- Status: accepted, pending the M3 crypto-reviewer pass over `crates/import/src/kdbx/`
- Date: 2026-10-29
- Milestone: M3

### Context
Brief §5 requires importers for Enpass JSON, Bitwarden JSON, KeePass KDBX 4,
1Password 1PUX and CSV. These files are untrusted input. KDBX is encrypted
with primitives Mosaic does not use for its own data, and 1PUX is a zip
archive.

### Decision
1. **KDBX 4, read only:** `aes =0.9.3` and `cbc =0.2.1` (AES-256-CBC outer
   cipher, AES-KDF), `chacha20 =0.10.2` (outer cipher, inner stream),
   `salsa20 =0.11.0` (inner stream), plus `argon2`, `hmac` and `sha2` from §2
   for Argon2d/Argon2id, the block HMACs and the header hashes, all with
   `zeroize`. They only read the user's file and never protect Mosaic data, so
   CRYPTO_SPEC §2 lists them as interoperability-only. Password only (no key
   files); Twofish and KDBX 3.1 are refused. Limits are checked before any
   derivation runs: Argon2 at most 1 GiB of memory, 64 GiB of total work and
   256 lanes; AES-KDF at most 2·10⁹ rounds. Every HMAC block is verified before
   anything is decrypted. A wrong password and a modified file give the same
   error.
2. **Archives and XML:** `zip =9.1.2` (deflate only, no encryption),
   `flate2 =1.1.10` (`rust_backend`) and `quick-xml =0.42.0`, the last two
   already in the tree. Limits: file 256 MiB (checked on the file's metadata
   before reading), 100,000 entries, bounded zip member count, decompressed
   payload and 1PUX `export.data` size, JSON and folder depth, fields and
   attachments per entry, and attachments at most 1 GiB per import.
3. **Mapping:** unknown categories become secure notes with a warning;
   archived items get the tag `archived`; passkeys are left out with a warning
   (M5). Duplicates (same site and user name, or same type, title and user
   name) are only suggested; nothing is skipped without the user's choice.
4. **Desktop:** Rust opens the file through the native dialog, reads it into a
   zeroizing buffer and parses it outside the session lock. The webview sees
   counts, titles, subtitles, warnings and duplicate hints. The parsed data is
   dropped on commit, cancel, a new preview or lock.

### Alternatives considered
- The `keepass` crate: it decrypts everything into plain `String`s and pulls
  in more crypto.
- Unbounded parsers checked only by fuzzing: rejected in favor of explicit
  limits.

### Consequences
`serde_json`, `quick-xml` and `zip` keep unwiped scratch copies (THREAT_MODEL
R22). The import limits are listed in PROGRESS M3-R19. Six fuzz targets run
in the CI fuzz smoke.

## ADR-0022: Reuse detection by a keyed hash under a per-call key

- Status: accepted, pending the M3 crypto-reviewer pass over `crates/insights/src/reuse.rs`
- Date: 2026-10-29
- Milestone: M3

### Context
Finding equal passwords without comparing every pair needs a value derived
from each password. A plain SHA-256 kept in memory would be a ready-made
dictionary target if memory leaked (a crash dump, swap). A map keyed by the
password copies passwords into keys that are not zeroized.

### Decision
1. Each login and Wi-Fi password becomes `HMAC-SHA-256(K, password)`, with a
   32-byte `K` from `SecretBytes::random` drawn once per `analyze` call. `K` is
   never stored, never derived from account keys, and is zeroized on return.
   There is no domain-separation label, because `K` has no other use.
2. Indices are sorted by tag. Within a group of equal tags, passwords are
   compared with `subtle::ConstantTimeEq`, so the groups are exact even under a
   tag collision. Empty passwords never form a group.
3. Tags live in a zeroizing buffer allocated at its final size. The result is
   indices only.
4. No new dependency: `hmac`, `sha2`, `subtle` and `zeroize` are already in §2.

### Alternatives considered
- Pairwise constant-time comparison: 12.5 million comparisons for 5,000 items.
- Plain SHA-256, or a map keyed by the password.
- A key derived from the account key: stable tags that nothing needs, tied to
  a long-lived secret.

### Consequences
Sorting leaks the tag order through timing, which reveals nothing without
`K`. `ct_eq` leaks a length difference only on a tag collision. One stack
copy of each tag is made before it moves into the zeroizing buffer. Not
externally reviewed (pre-audit).

## ADR-0023: Shipping the 2FA Directory data, and what counts as 2FA

- Status: accepted
- Date: 2026-10-29
- Milestone: M3

### Decision
1. **Data and license:** the 2FA Directory API v3 TOTP list (downloaded
   2026-10-27; its SHA-256 is recorded in `crates/insights/data/README.md` and
   checked by a test), embedded with `include_str!`. The MIT license
   (2factorauth and Contributors; Josh Davis) ships unchanged next to it and is
   kept in the package by `include`. The dashboard shows
   `insights.attribution.twoFactor` (DE and EN). Updates come only with
   releases; nothing is fetched at run time.
2. **Matching:** a host matches its own entry or its closest listed parent,
   down to two labels; IPv4 hosts match only exactly. Domains in `EXACT_ONLY`
   match only exactly or with `www.`: carrd.co, gov.uk, myfritz.net, omg.lol,
   pythonanywhere.com and square.site (on the Public Suffix List, checked
   2026-10-28), plus squarespace.com, substack.com, tumblr.com, weebly.com and
   wordpress.com (hosting platforms that give users subdomains). The check is
   repeated on every data update; no Public Suffix List is shipped.
3. **What counts as 2FA:** an item's own OTP or an OTP custom field. Such an
   item covers its URLs and every OTP issuer it holds. A passkey for the same
   site lowers the finding from Medium to Low and does not hide it, because
   the password alone still signs in.
4. **Severity:** Medium at most, because the data is crowdsourced and can be
   out of date. Documentation links are shown as text; the app opens no
   external link.

### Alternatives considered
- Fetching the directory at run time: a network request that reveals that
  someone uses Mosaic, plus a remote input to trust.
- Shipping the Public Suffix List: about 230 KB under MPL-2.0, for 11 domains.
- Letting a passkey hide the finding.

### Consequences
New platforms in future data need a manual review. Users see a Low hint for
sites where they use a passkey.

## ADR-0024: A hand-written strength estimate instead of zxcvbn

- Status: accepted
- Date: 2026-10-29
- Milestone: M3

### Context
The Rust zxcvbn ports copy the password into many ordinary `String`s that
nobody zeroizes, and they embed large lists.

### Decision
1. A documented heuristic: the generator's brute-force bound, lowered by
   matches against a short word list (with reversal and common
   substitutions), sequences, QWERTY and QWERTZ keyboard runs, repeated blocks
   of every length, and years and dates, combined by dynamic programming.
   Every copy of the password is a zeroizing buffer. Only the first 128
   characters are analyzed.
2. Weak means below 60 bits (the generator's Fair bucket). Severity is High
   below 36 bits.
3. Accepted limit: dictionary passwords outside the list score as brute force
   (`Tr0ub4dor&3` scores 72.3 bits, where zxcvbn gives about 28). A larger,
   licensed list is follow-up work in M7, together with the breach check.
4. Checks: unit tests, an invariant proptest (never negative, never above the
   brute-force bound, deterministic, only 128 characters, a doubled block
   costs at most the block plus 1 bit) and two fuzz targets in the CI fuzz
   smoke.
5. Cost: about 1 to 6 ms per 128-character password in a release build, up to
   about 25 ms for adversarial periodic strings. The analysis runs under the
   session lock. This is accepted for M3; it moves off the lock if the
   dashboard ever blocks other commands noticeably.

### Consequences
The dashboard under-reports dictionary passwords outside the list (PROGRESS
M3-R16). The README stays labeled pre-audit.

## ADR-0025: SSH agent crates, algorithms and approval model

- Status: accepted, pending the M3 crypto-reviewer pass over `crates/ssh-agent/src/keys.rs` and `core.rs`
- Date: 2026-10-29
- Milestone: M3

### Context
Brief §5 asks for an SSH agent with approval for every use. CRYPTO_SPEC §2
had no SSH row.

### Decision
1. **Crate:** `ssh-key =0.7.0-rc.11` (RustCrypto), with `std`, `ed25519`,
   `p256` and `encryption`. Move to 0.7.0 when it is released; the stable
   0.6.7 is blocked by RUSTSEC-2026-0387 (crypto-bigint 0.5). Feature
   unification turns on `zeroize` in `ed25519-dalek =3.0.0` and
   `bcrypt-pbkdf =0.11.0`. `tokio =1.54.1` (already in the tree through Tauri)
   provides the listener, timers and channels.
2. **Algorithms:** Ed25519 (RFC 8709) and ecdsa-sha2-nistp256 (RFC 5656,
   RFC 6979 nonces) only. RSA is refused: the `rsa` crate has the unpatched
   RUSTSEC-2023-0071, and signing time is observable by the server and by
   local clients. The RFC 8332 flag rule is implemented for later. P-384,
   P-521, DSA, `sk-*` keys and certificates are refused.
3. **Keys:** OpenSSH private keys are decrypted (bcrypt-pbkdf with at most
   1024 rounds; aes256-ctr, aes256-gcm or chacha20-poly1305@openssh.com) once
   per approved request, off the async executor, and wiped afterwards. Nothing
   is cached. The passphrase is the item's `login.password`
   (`SecretField::Password`).
4. **Approval:** allow once, allow for 15 minutes (ceiling configurable,
   default 15 min), or deny. A request is denied after 25 s, and at most 4
   prompts are open at once. Grants are bound to the public key and the client
   executable path, only where the executable is known (Linux). Grants are
   dropped on lock and when the key leaves the set. Listing identities needs
   no approval (as with ssh-agent). Add, remove, lock, unlock, smartcard
   requests and every extension, including `session-bind@openssh.com`, get
   `SSH_AGENT_FAILURE`.
5. **Endpoints:**
   - Unix: a socket with mode 0600 in a private 0700 directory, with a peer UID
     check. `$XDG_RUNTIME_DIR/mosaic/agent.sock` (Linux), else
     `<app local data>/ssh/agent.sock`.
   - Windows: `\\.\pipe\openssh-ssh-agent`, first instance only, remote
     clients rejected, default DACL. An explicit SDDL needs unsafe and is
     deferred.
   - Limits: 32 connections, 256 KiB per message, a 10 s body timeout.
6. **Desktop:** off by default. Turning it on needs an unlocked vault, and the
   choice is stored on the device. The agent runs only while unlocked: it
   starts after unlock and stops before the `locked` event and at quit. The
   key set is rebuilt after every item change. The approval dialog gives Deny
   the initial focus (Esc, expiry and lock also deny), enables the allow
   buttons 1 s after it appears or after the window regains focus, and never
   takes focus itself (it only asks for taskbar attention).

### Alternatives considered
- `ssh-key 0.6.7`: carries the advisory.
- `russh-keys`: a larger surface and the `rsa` crate.
- Serving RSA: the advisory.
- Granting until lock: a longer window than needed.

### Consequences
On Windows and macOS the client process is unknown (PID only on macOS), so
there are no grants and the prompt shows the client as unknown. The agent
cannot start while the Windows OpenSSH Authentication Agent service runs
(`sshAgentEndpointInUse`). THREAT_MODEL R20; PROGRESS M3-R12 to M3-R15 and
M3-R18.

## ADR-0026: M3 integration: services, contract changes and UI rules

- Status: accepted
- Date: 2026-10-29
- Milestone: M3

### Decision
1. **Concrete services instead of adapter traits.** `adapters::ImportService`
   (`mosaic-import`) and `adapters::SshAgentService` (`mosaic-ssh-agent`) are
   concrete types, and `security_report` calls `mosaic-insights` directly. No
   command answers `notImplemented` any more. `unlock_quick` answers
   `quickUnlockUnavailable` (ADR-0020 item 7).
2. **Import outside the lock.** Import parsing (including the KDBX key
   derivation) runs without the session lock. The duplicate pass and the
   commit run under it. One preview is held at a time.
3. **The Emergency Kit check is stored per account (`kitVerified`).** An
   unlocked account without it is shown the kit step again: save the kit again
   with the master password, then type its last group.
4. **Contract changes:**
   - New commands: `save_emergency_kit` and `app_status`.
   - New events: `sshSignFailed` and `quickSearchRequested`.
   - New error codes: `importTooLarge`, `importUnsupported`, `importEncrypted`
     and `sshAgentEndpointInUse`.
   - Richer import preview and result.
   - SSH: `canRemember`, `requestedAt`, `skipped` and `error`.
   - Renamed: `SshDecision.allowUntilLock` becomes `allowFor15Minutes`.
   - `security_report` findings changed from `kinds` to `issues`, and
     `reusedWith` now carries `{ vaultId, itemId }`.
5. **The webview opens no external link.** 2FA documentation URLs are shown as
   text; no opener plugin is granted.
6. **No new JavaScript dependencies.** Icons are vendored from Lucide (ISC),
   with the notice shipped as `licenses/lucide-ISC.txt`.
7. **i18n catalogs are split per area** (`packages/i18n/src/catalog/{en,de}/*.ts`),
   with parity checked per module at compile time and by the parity test.

### Alternatives considered
- Keeping the adapter traits: one implementation each, and the fakes tested
  the shell rather than the crates.
- Starting the SSH agent whenever an SSH key item exists: it would expose keys
  without the user choosing to.
- A webview file picker: it needs the dialog and fs permissions.

### Consequences
`DESKTOP_IPC.md` marks every command live except `unlock_quick`. PROGRESS
records the renames.

## ADR-0027: Holo foil accents

- Status: accepted
- Date: 2026-11-02
- Decided by: the owner
- Amends: ADR-0014 decision 2 ("Nothing else uses the foil")
- Milestone: M3 (desktop UI fidelity)

### Context
The owner compared the app with the approved mockups and asked for a few small
foil accents beyond the primary action, so the signature look carries through
the interface. ADR-0014 and UX_GUIDELINES §2 forbid this today (the logo is
strictly black and white; progress and selection are neutral).

### Decision
Four static accents may use the foil: (1) the fourth tile of the logo mark,
(2) a 3 px bar at the left edge of the active sidebar item, (3) the completed
segments of the setup progress bar, (4) the "+" tile of the empty-vault
illustration. Rules for every accent:
1. **Static only.** No hover, no animation, no ambient sheen. One frame,
   painted once and cached per size and theme.
2. **Decorative.** The canvas is `aria-hidden` and carries no text.
3. **Never the surface of an interactive control** other than the primary
   action. The nav bar is a mark inside the item, which keeps its neutral
   selected background.
4. **Never the only carrier of meaning.** Selection and progress stay shown by
   the neutral styling and the text (`aria-current`, "Step 2 of 5").
5. **Flat fallback.** Without WebGL2, after a lost context, without a
   provider or with `prefers-reduced-transparency`, the parent's flat fill
   shows (`--color-faint`, or `--color-accent` for progress).
6. **One WebGL2 context.** Accents render through `components/HoloSurface.tsx`
   and the shared renderer (`holo/renderer.ts`), the same context and program
   as the primary action. Nothing is loaded remotely.

The primary action itself also gains a press ripple and keyboard-focus waves
(still the same foil, still cosmetic), and the shader floors its sheen so the
foil never turns grey. Reduced motion shows still frames.

Amendment (2026-11-03, final mockup review): at a few pixels the button foil
averages out to near-white, so the small accents read as grey on a light page.
Static surfaces therefore render with more iridescence and a saturation lift
that scales with size: full on the logo tile, nav bar and progress segments,
close to the button foil on the 48 px empty-state tile. Same shader and
palette, still static and decorative.

### Alternatives considered
- Foil on the success badge and toasts: rejected, status colours stay
  conventional (UX §2).
- Only one or two accents: the owner chose all four.

### Consequences
UX_GUIDELINES §2 names the four accents. Any further foil use needs another
ADR.

## ADR-0028: Quick unlock, hardware-key unlock and hardware-protected TOTP vaults

- Status: accepted; amended after the crypto-reviewer pass (items 3, 4, 8, 10, 11)
- Date: 2026-11-03
- Milestone: M3
- Amends: ADR-0020 item 7 (Windows calls), CRYPTO_SPEC §6.2, §6.3, §7.4, §11, §12

### Context
Brief M3 asks for unlock by password, biometrics and hardware key. CRYPTO_SPEC
§6.2, §6.3 and §7.4 describe the wrappings, `mosaic-vault` stores them
(`unlock_methods`, `tk_wrappings`) but had no session methods, so quick unlock
was not offered (PROGRESS M3-R6, M2-R6), and re-authentication had to lock and
unlock (M3-R8). Writer IDs are per session (ADR-0015), so §6.2's
"`method_id` = device ID" has no stable ID to use.

### Decision
1. **Derivations live in `mosaic-core`** (`unlock` module), all HKDF-SHA-256
   with new labels `hello-unlock`, `prf-unlock`, `prf-hw-vault` and AAD labels
   `wrap/hw-vault`, `wrap/vault-under-tk`. No new primitive.
2. **Quick unlock (§6.2).** `method_id` is a fresh UUIDv7 per enrollment; its
   timestamp is the enrollment time. At most one quick-unlock wrapping per
   store. It expires after the configured maximum age (default 14 days,
   1 to 30 days; `mosaic-vault` never accepts more than 30), and it is used
   only after the master password unlocked once since the store was opened
   (once per app start). Expiry, a wrapping that does not open, a master
   password change and a Secret Key rotation delete the wrapping (and the app
   deletes the platform key). Enrolling needs the master password, and the
   vault API enforces it: `verify_master_password` returns a
   `Reauthenticated` proof (not `Clone`, bound to the session, with its
   creation `Instant`), and `enroll_quick_unlock` and `enroll_hardware_key`
   take it and refuse one older than `REAUTH_GRANT` (120 s) or from another
   session. A previous enrollment is replaced in one transaction. The
   session API does not offer Secret Key rotation yet; when it does it must
   delete the quick-unlock wrapping like a password change.
3. **Windows (pins ADR-0020 item 7, adds one call):** after
   `RequestCreateAsync`, `KeyCredential.GetAttestationAsync()` must report
   `Success` or `TemporaryFailure`; `NotSupported` or `UnknownError` (no TPM,
   a software key) deletes the key and refuses enrollment. Enrollment signs
   with the `KeyCredential` object `RequestCreateAsync` returned, never with
   one re-opened by name, so the attested key is the signing key. At unlock
   the key is opened by name and not attested again (that is a network call);
   a key swapped in under the name signs differently, so `W` differs and the
   unwrap fails (a swap can deny an unlock, not grant one). **Key name:** the
   platform key is named `Mosaic quick unlock {key_nonce_hex}`, where
   `key_nonce` is 16 random bytes drawn per enrollment and stored in the
   platform parameters (version `0x02`: `0x02 || platform || key_nonce[16] ||
   body`). The name is per store and enrollment, not per account: a copied
   store never shares or deletes another store's key, and the name cannot be
   predicted from the account ID. Version `0x01` (named by account) never
   shipped and no longer decodes; such an enrollment counts as stale and the
   user enrolls again. A key that lost its enrollment (a crash between the
   two steps) cannot be found by name any more; every failure path deletes
   its own key, so only that window leaves one. **Residual (THREAT_MODEL
   R23):** for an unpackaged Win32 app Windows may not isolate Hello keys
   per app, and the key name is not secret. A process of the same user that
   gets one Hello gesture (or the Hello PIN, for example by keylogging) could
   sign the plaintext challenge from the store, derive `W` and unwrap the
   account key offline with a copy of the store. MSIX package identity (M9)
   is the tracked mitigation; until then the manual hardware check in
   PROGRESS ("Needs owner") must show whether a second unsigned executable
   can open and sign with Mosaic's key.
4. **macOS:** a Secure Enclave P-256 key (`kSecAttrTokenIDSecureEnclave`,
   data-protection keychain, label `Mosaic quick unlock {key_nonce_hex}`, see
   item 3) with
   access control `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` and flags
   `kSecAccessControlPrivateKeyUsage | kSecAccessControlBiometryCurrentSet`
   (Touch ID only; a fingerprint change invalidates it). `W = random(32)` is
   encrypted to the key's public key with
   `kSecKeyAlgorithmECIESEncryptionCofactorX963SHA256AESGCM` and the
   113-byte result is stored as the wrapping's parameters. Enrollment
   decrypts once (one Touch ID prompt) and compares in constant time. Calls
   go through `security-framework =3.7.0` (feature `OSX_10_15`) and the
   constants of `security-framework-sys =2.17.0`, both already in the tree
   through `keyring`; no `unsafe` in Mosaic's crates. Needs Developer ID
   signing with an application-identifier and keychain-access-groups
   entitlement; unsigned builds report Touch ID as unavailable.
5. **Linux:** no biometric quick unlock.
6. **Hardware keys (§6.3, §7.4).** WebAuthn PRF semantics: the CTAP
   hmac-secret salt is `SHA-256("WebAuthn PRF" || 0x00 || prf_salt)`;
   `credential_id_hash = SHA-256(credential_id)`. Non-discoverable
   credentials (`rk = false`), RP ID `mosaic.invalid`, user verification on
   every operation (also the assertion at registration) and the UV flag
   checked in the returned authenticator data. At most 8 keys for unlock and
   8 per hardware-protected vault. Five rejected hardware-key unlocks in a row
   (counted per app run; PIN failures the vault cannot see are reported with
   `note_failed_hardware_attempt`) require the master password; the
   authenticator's PIN retry counter is the brute-force limit. A `prf_salt`
   must be new for its credential (no reuse across the unlock wrapping and
   any TK wrapping). `mosaic.invalid` only works through raw CTAP clients,
   not browser or mobile WebAuthn.
7. **Hardware-protected TOTP vaults (§7.4)** hold `ItemType::Totp` items only,
   open with a registered key for at most 5 minutes, and close on lock. The
   last key cannot be removed; removing a key does not rotate TK.
8. **Re-authentication** uses `UnlockedSession::verify_master_password`; a
   wrong password no longer locks, five in a row do. That lock uses
   `UnlockedSession::lock_requiring_password`, which sets the failed-attempt
   counter to `MAX_FAILED_QUICK_ATTEMPTS` so no quick method is offered or
   accepted until the next password unlock; a plain `lock` resets the counter
   and would let the next Hello prompt reopen the guessing. The proof of a
   right password is **consumed** by a successful enrollment (kept after a
   failed one), so the next enrollment asks again.
9. **Transport.** The FIDO2 transport is a separate decision (ADR-0029):
   until it lands the app reports hardware keys as unsupported and shows no
   hardware-key control. On Windows, FIDO devices are reachable without
   administrator rights only through `webauthn.dll`, which needs `unsafe` FFI;
   Windows has no hardware-key unlock in M3.
10. **Hardware-vault lifetime.** The 5-minute deadline of an open
    hardware-protected vault is kept as a monotonic and a wall-clock reading
    and the vault closes when either passed; every open hardware vault closes
    when the tick sees a suspend, whatever the lock-on-sleep setting says; an
    open vault without a deadline closes at the next tick; a deadline is
    forgotten only after the close succeeded.
11. **Reauthentication for key and secret exits.** `add_hardware_vault_key`
    and moving an item out of a hardware-protected vault need a right
    master password within the last 2 minutes (the commands do not take a
    password: the UI calls `reauthenticate` first). The transport MUST check
    the UV flag of every assertion (`HardwareKeyPlatform` docs). Secrets
    (`W`, PRF output, PIN) are dropped right after their last use, not at
    the end of the command.

### Alternatives considered
- `method_id` = the stored device ID: it changes with every unlock (ADR-0015).
- Expiry from the row's `updated_at`: cleartext, editable.
- macOS biometry-or-passcode (`kSecAccessControlUserPresence`): the login
  password then unlocks Mosaic, weaker than the brief's "biometrics".
- Keeping the ECIES ciphertext in its own Keychain item: two places to delete.
- `authenticator 0.5` (Mozilla): unmaintained `serde_cbor` (RUSTSEC-2021-0127),
  MPL-2.0, `libudev`, old RustCrypto majors, raw HID on Windows needs admin.
- `ctap-hid-fido2 3.6`: needs `ring` or `aws-lc-rs` for ECDH (banned outside
  rustls / C crypto), raw HID on Windows.
- `webauthn-authenticator-rs`: OpenSSL (banned).
- WebAuthn inside the webview: no WebAuthn in WebKitGTK, entitlements on
  macOS, and the PRF output would sit in the JavaScript heap.

### Consequences
PROGRESS M3-R6 is closed for Windows Hello and Touch ID once WP1–WP6 land and
manual hardware checks pass; hardware keys stay open until ADR-0029 (owner).
M3-R8 and M2-R6 (API side) close. THREAT_MODEL R8 is updated and R23–R25 are
added. The M6 sync client must never upload method `0x02` rows.

Notes for the later work packages. WP4 (desktop commands): call
`verify_master_password`, keep the returned `Reauthenticated` only for the
enrollment that follows (it expires after 120 s), and call
`LockedSession::note_failed_hardware_attempt` on every PIN failure. WP6 (UI):
when a user moves an existing TOTP secret into a hardware-protected vault, say
that earlier copies (the old item, its history, backups, exports) are not
protected, and recommend re-issuing the secret at the provider (THREAT_MODEL
R26). The strings go through `packages/i18n` in DE and EN.

## ADR-0030: Settings integrity (a MAC under a key derived from AK)

- Status: accepted
- Date: 2026-11-03
- Milestone: M3 (red-team findings)
- Amends: CRYPTO_SPEC §2, §11 (new §11.1); THREAT_MODEL §4.2, R28, R29
- Note: ADR-0029 stays reserved for the FIDO2 transport (ADR-0028 item 9).

### Context
`settings.json` is plaintext and was parsed with only range clamps. A local
process that can write the file (P5) could set `clipboardClearSeconds: null`,
`autoLockMinutes: null` or the longest quick-unlock age without the master
password, which `update_settings` demands for the same change in the app. The
red-team suite showed that only the CRYPTO_SPEC §11 floor and the 30-day cap
held.

### Decision
1. **A MAC over the settings.** `settings.json` gets a top-level `mac`
   (64 lowercase hex characters):
   `HMAC-SHA-256(K_settings, canonical(settings))` with
   `K_settings = HKDF-SHA-256(ikm = AK, salt = "", info = ctx("settings-mac"), 32)`.
   No new primitive: HMAC-SHA-256 is in the spec (§2, OTP row), and the `hmac`
   crate is already a workspace dependency (OTP) and in the tree through
   `hkdf`; `mosaic-core` now depends on it directly. `mosaic-core` offers
   `AccountKey::settings_mac_key()`, a zeroizing `SettingsMacKey` with `tag`
   and a constant-time `verify`.
2. **Canonical form** (CRYPTO_SPEC §11.1): the seven settings fields plus
   `sshAgentEnabled` and the account block's `email` and `kitVerified` as one
   JSON object, keys in ascending byte order, no whitespace, integers in
   decimal, `null`, `true`/`false`, enum values as their wire strings, the
   email as an RFC 8259 string. The file's own `version`, the account ID and
   `mac` are not part of it (the account ID is part of the MAC message).
3. **Locked: only stricter values apply.** The key exists only while the vault
   is unlocked. From the file, a locked app uses per field the stricter of the
   file value and the default (auto-lock: the smaller minutes, `null` becomes
   the default; clipboard: the smaller seconds, `null` becomes the default;
   quick-unlock age: the smaller days; lock on sleep and on screen lock:
   always on). Theme and language come from the file. A weaker file value
   therefore never reaches a decision made before the first unlock, notably
   the quick-unlock expiry check.
4. **First unlock after the start: verify.** A valid MAC (constant-time
   compare) applies the file as it is. A missing or invalid MAC keeps the
   stricter-than-default values from item 3, rewrites the file with a fresh
   MAC and, if a weakening field differed, emits the event `settingsReset`
   once so the UI shows "Your security settings were changed outside Mosaic
   and were reset to safe defaults." Theme and language survive either way.
   A file without `mac` (written before this ADR) is handled the same way; a
   user who had chosen weaker values sees the notice once and sets them again
   (the cost of not trusting an unauthenticated file).
5. **Writes.** Every save while unlocked writes the MAC of the settings being
   written. While locked, a save keeps the settings section and MAC of the
   file as they are (a new theme or language stays in memory and is written at
   the next unlock), so a locked write never invalidates a good MAC.
6. **Key lifetime.** `K_settings` is derived once per unlock, held in a
   zeroizing container in the backend and dropped on every lock.
7. **Rollback binding (amendment).** The MAC'd message is
   `account_id[16] || settings_nonce[16] || canonical(settings)`, where
   `settings_nonce = random(16)` lives in a local-only row of the vault store
   (never in the outbox, never synced) and is replaced by a fresh random value
   in the same step as every MAC'd write of the file. A restored older file
   carries a MAC under an older nonce and fails: strict values, rewrite,
   notice. A random nonce, not a counter, so the value reveals nothing about
   how often settings changed and cannot be predicted. The store gets one
   additive table (schema version 2).
8. **SSH flag and account fields (amendment).** `sshAgentEnabled`, `email` and
   `kitVerified` are inside the MAC. Locked, `sshAgentEnabled` reads `false`.
   After an unlock whose MAC is invalid it is `false` before the agent starts,
   `email` is cleared and `kitVerified` is `false`. Without this, a local
   writer could switch the agent on at the next unlock: any process that can
   reach its endpoint could then list the public keys without approval (each
   signature still needs the user's approval, listing does not), or flip the
   kit flag to skip the re-check.
9. **Rewrite and notice (amendment).** An invalid MAC always rewrites the file,
   even when the values equal the strict ones, so a stale or forged MAC does
   not survive. The notice is shown only if a protection field differed or the
   file had `sshAgentEnabled: true`.
10. **Start-up expiry (amendment).** At the start of the app, before any
    unlock, the quick-unlock expiry deletion runs only against the 30-day
    spec cap (§6.2); the verified age applies at the reconcile after the
    unlock, so an unverified file cannot make the start-up check stricter or
    weaker than the spec allows.
11. **Look-alike names (amendment).** Vault names and item titles refuse the
    Unicode format characters (Cf) and default-ignorable code points
    (`mosaic_core::is_spoofing_char`, one table of ranges, no dependency):
    soft hyphen, U+034F, Hangul fillers, U+180E, U+200B to U+200F, U+202A to
    U+202E, U+2060 to U+206F, U+3164, variation selectors U+FE00 to U+FE0F,
    BOM, U+FFF9 to U+FFFB and the tag block. Imports strip them from titles
    and report `titleCleaned` on the entry, so a file is never refused for it.
    The UI shows typed text in a bidirectional isolate (`<bdi>`). Cost: an
    emoji sequence that needs U+200D or U+FE0F cannot be typed in a name.

### Alternatives considered
- Signing with a key from the OS keystore: ties settings to one OS API and
  does not exist for the CLI; the MAC works with the AK every build has.
- Refusing a bad file outright: it would also throw away theme and language;
  resetting only the weakening fields is the more useful conservative reading.
- Encrypting the settings: they are not secret; integrity is the goal.

### Consequences
- Malware that runs while the vault is unlocked can still change settings
  through the app or by reading memory (R1); the MAC defends the file at rest
  and the locked window (THREAT_MODEL R28).
- While locked, a user with a longer quick-unlock age than the default gets
  the default age enforced until the first unlock after each start (R28).
- A joint rollback of the store and `settings.json` to an earlier consistent
  pair still verifies (R28). The full fix is to keep the settings as a synced
  record under CRYPTO_SPEC §10 in M6.
- A crash between replacing the nonce and writing the file leaves a file the
  MAC rejects: the next unlock resets to the strict values and tells the user
  (fail safe, not fail open).
- The SSH approval prompt names only the client process where the OS gives a
  name (THREAT_MODEL R29, unchanged by this ADR).

## ADR-0031: One-time-code setup from QR codes (screen, clipboard, image file) and manual entry

- Status: accepted
- Date: 2026-11-04
- Milestone: M3 follow-up (brief §5 "2FA / TOTP")
- Amends: DESKTOP_IPC §3, §5, §6; THREAT_MODEL §4.2, §5 (R30–R32)

### Context
The brief asks for "QR scanning from screen and image; otpauth:// import/export".
The item editor had no way to add a one-time code (`otp: null` always). Setting
up 2FA should take one click: most sites show a QR code in the browser next to
Mosaic. The decoded `otpauth://` URI carries a long-lived shared secret, which is
worth more than any 30-second code, so where it lives matters.

### Decision
1. **Four sources, in this UI order:** *Scan screen* (one click), *Paste
   screenshot* (an image on the clipboard, also Ctrl/Cmd+V in the dialog), *Choose
   image…* (native file dialog, PNG or JPEG), and *Can't scan? Enter the key*
   (an `otpauth://` link or a bare base32 key; issuer and account prefilled from
   the item's title and user name).
2. **Rust decodes, the webview never sees the secret from a scan.** Every source
   is parsed in Rust (`mosaic-qr`, `mosaic-totp`) into an `OtpAuth` held in a
   pending slot behind an opaque ID. The webview receives a preview without the
   secret (issuer, account, scheme, algorithm, digits, period, secret-length
   class), a live code as `SecretOut` (like `totp_code`), and saves with
   `OtpChange::SetPending`. Slots expire after 10 minutes (monotonic and wall
   clock, the earlier wins), at most 4 exist (the oldest is dropped), they are
   consumed by a successful save, dropped by `otp_pending_discard`, and all are
   dropped on lock. Manual entry sends the typed text as `SecretIn` (it is in the
   webview anyway because the user typed it, R19).
3. **Confirmation before saving.** Nothing is stored until the user confirms a
   screen showing issuer, account (both editable), digits, period, algorithm and
   the current code. The editor saves the item; the dialog never writes.
4. **Screen capture rules.** Capture happens only on a click on *Scan screen*,
   never automatically or repeatedly, one frame per display (at most 8). Mosaic's
   main window is hidden for the capture (shown again and focused afterwards on
   every path) so the code behind it is visible. Pixels go from the OS buffer
   straight into zeroizing buffers, are converted to grey levels, and are wiped as
   soon as the QR search ends. They are never written to disk, logged, sent to the
   webview or kept as a thumbnail. Only an `otpauth://` payload survives. Any
   other QR content is dropped unread; errors say only "not a setup code", never
   what the code contained.
5. **Platforms.** Windows: GDI capture of the virtual screen (`win-screenshot`).
   Linux: X11 only (`x11rb` `GetImage` on the root window); under Wayland
   (`WAYLAND_DISPLAY` set or `XDG_SESSION_TYPE=wayland`) the button is hidden,
   because the screenshot portal writes a file. macOS: hidden in this version
   (item 8). `app_status.otpScreenScan` tells the UI.
6. **Clipboard.** Rust reads the image (arboard `image-data`) only on the user's
   action. After a decode that found a code (a good one, a migration, several or
   an invalid one) it empties the clipboard, but only if it still holds the same
   image (size and SHA-256 of the pixels), because the screenshot is the secret in
   another form and the user may have copied something else while the scan ran.
   The UI says that clipboard
   history, the Snipping Tool's auto-save or a saved screenshot file may still hold
   a copy.
7. **Hostile images.** PNG and JPEG only (by magic bytes); file ≤ 20 MiB checked
   before reading; width and height ≤ 16384 and ≤ 40 MP checked from the header
   before decoding; decoder allocations ≤ 512 MiB (`image::Limits`); frames are
   box-downscaled to ≤ 8 MP of grey levels before QR detection; at most 16 QR
   codes per frame are decoded; one scan runs at a time (`busy`). The clipboard
   image is decoded by arboard before Mosaic can check it (R31). `rqrr` 0.11.0
   tries every viable finder-pattern pair and clones the prepared image per
   attempt, so a screen full of fake finder squares ran for hours. It is patched
   in `vendor/rqrr` (a patch of the existing dependency, not a new crate; see
   `vendor/rqrr/MOSAIC_PATCH.md`): no groups if there are more than 64 finder
   patterns, and at most 256 grouping attempts per image. Drop the patch once
   upstream bounds this. The JPEG decoder (zune-jpeg) runs `unsafe` SIMD code on
   attacker-chosen bytes and a decoder panic aborts the app (`panic = "abort"`);
   the decoder set is unchanged, and the `scan_image` fuzz target (JPEG seed
   included) is the mitigation (R31).
8. **macOS screen capture is deferred.** When it is added it uses ScreenCaptureKit
   through a maintained safe wrapper (or `CGDisplayCreateImage` from
   `objc2-core-graphics` if ScreenCaptureKit has none), asks for Screen Recording
   only on the first click with `CGRequestScreenCaptureAccess`, checks
   `CGPreflightScreenCaptureAccess` before every capture, and on denial shows
   "Mosaic isn't allowed to see your screen. Allow it in System Settings → Privacy
   & Security → Screen Recording, then restart Mosaic — or paste a screenshot
   instead." with a button that opens that settings pane, never a loop of prompts.
   That needs its own ADR amendment and an owner decision.
9. **`otpauth-migration://`** (Google Authenticator export, several accounts in a
   protobuf) is recognized and refused with its own message. Batch import, if
   wanted, goes into `mosaic-import` later.
10. **Several codes.** Two or more different `otpauth://` payloads in one scan are
    refused (`otpQrMultiple`); identical ones count once; unrelated QR codes next
    to one `otpauth://` code are ignored.
11. **Hardware-protected vaults (ADR-0028).** If the account has a
    hardware-protected vault, the confirmation offers to keep the code there, with
    the trade-off in one sentence for each choice. The code then becomes a `totp`
    item in that vault and the login item keeps no code. The choice is enabled only
    while that vault is open.
12. **The preview code is shown at once**, not masked: the site usually asks for it
    to finish setup. It can be copied through Rust and hides on blur and lock
    (UX_GUIDELINES §8 exception, scoped to this dialog).

### Alternatives considered
- Returning the decoded URI to the webview as `SecretOut` and sending it back as
  `SecretIn` in `OtpChange::Set`: simpler, but the long-lived secret then sits in
  the JavaScript heap for every scan. Rejected for the scan paths; `Set { uri }`
  stays for compatibility.
- `xcap` for all platforms: pipewire/libclang/xcb in the lock and the Linux build.
- The Wayland screenshot portal: writes a file.
- Decoding in the webview with a JS QR library: secret and pixels in the JS heap.
- Accepting the first of several codes: the user may save the wrong account.

### Dependencies
`rqrr =0.11.0` (pure-Rust QR decoder, a quirc port; no `unsafe`), `image =0.25.10`
(`png`, `jpeg` through zune-jpeg; bounded by `Limits`; arboard's `image-data`
pulls it anyway), `arboard` `image-data` (clipboard images), `win-screenshot
=4.0.14` (Windows only; its only dependency is `windows 0.62.2`, already in the
tree; one maintainer, owner decision O3 accepts it), `x11rb =0.13.2` (Linux only,
already in the tree through arboard) and `qrcodegen =1.8.0` (test fixtures only).
None of them is a cryptographic primitive.

### Consequences
New crate `mosaic-qr` with fuzz targets in CI. New commands `otp_scan`,
`otp_parse`, `otp_pending_code`, `otp_pending_discard`; new draft op
`setPending`; new copy source `pendingOtp`; `app_status.otpScreenScan`; twelve
error codes. THREAT_MODEL gains R30–R32. Export of `otpauth://` (brief) is not
part of this ADR.

## ADR-0032: Liquid-glass unlock and lock reveal

- Status: accepted
- Date: 2026-11-04
- Decided by: the owner (review of `docs/design/holo/reveal-studies.html`)
- Amends: ADR-0027 item 6 ("One WebGL2 context") and the UX_GUIDELINES §2 motion line
- Milestone: M3 (desktop UI fidelity)

### Context
The owner approved a reveal for locking and unlocking: a lens of glass that grows
from the click, shows the vault through it and closes again on lock. The studies
(`docs/design/holo/reveal-studies.html`) are DOM-driven prototypes with fixed
values. Porting them touches security, because a lock animation that keeps the
live vault on screen after the backend has locked would keep secrets visible,
and it touches the 120/200 ms motion rule and the single WebGL2 context.

### Decision
**Unlock.** The live vault DOM is revealed through a `clip-path` polygon that
follows an irregular lens outline (a sum of sines, written once in JS for the
polygon and once in GLSL for the band). The lens grows from the click point on
the Unlock or quick-unlock control, or from the centre of the Unlock button after
a keyboard submit. A WebGL2 canvas, which never receives pointer events, draws
only the glass rim and bevel band. The band samples a snapshot of the vault,
never the unlock screen, and the canvas is empty when the reveal ends. The
vault pieces enter as the rim reaches them (nav items, footer controls, top
bar, list header, rows, detail pane, pages); the logo is never choreographed
(the foil stays static, ADR-0027). The owner's values are fixed in
`apps/desktop/src/reveal/timing.ts`, including a 1.9 s duration, an explicit
exception to the 120/200 ms motion rule. While the lens runs the vault is
usable inside it and the unlock layer is `inert` and `aria-hidden`. Focus goes
to the vault's `main` when the reveal starts.

**Lock never keeps the live vault on screen.** We chose the conservative option
on every point that touches security or state:
1. The `lock` command is sent first and never waits for the animation.
2. In the same task the vault is painted into a sealed snapshot and, in the same
   React commit, unmounted. Revealed values, one-time codes, generated passwords
   and typed passwords leave the DOM before the first lock frame.
3. The snapshot painter never reads `[data-secret]` content (it paints the static
   `data-mask` text instead), never reads an input or textarea value, and skips
   dialogs, media and visually hidden text. Every component that renders a secret
   carries `data-secret` (UX_GUIDELINES §8). Non-secret item text (titles, hosts)
   can stay visible as pixels until the lens closes, and is then freed.
4. The closing animation (about 1.4 s) starts when the locked session arrives and
   plays over the picture, with the real unlock screen live and focused
   underneath. Clicks and typing reach it at once.
5. If the lock fails, or the locked session does not arrive within 4 s, the vault
   is put back.
6. Locks the app did not start (idle, sleep, screen lock, tray) switch instantly:
   the user may not be looking, and a picture of the vault must not linger after
   the backend has locked. A lock from the shortcut or the palette closes into
   the centre of the Lock button. Onboarding and Emergency Kit transitions stay
   instant.

**Fallbacks.** The band uses a second WebGL2 context, created lazily and only for
this cosmetic band (it amends ADR-0027 item 6; the foil renderer is unchanged).
Without WebGL2, after a lost context, when the snapshot is too large for a
texture, after a build failure, or after three frames slower than 50 ms, the
transition falls back to the clip-path lens alone. With `prefers-reduced-motion`
it is a 250 ms crossfade with no glass. Without clip-path or a 2D canvas it is an
instant swap (also what the jsdom tests get). A hidden window or a resize jumps
straight to the end state.

**Origin.** The last pointer press on the Unlock control (valid for 30 s, so a
slow quick-unlock prompt still counts) or on the Lock button (valid for 2 s)
sets the origin; a key press clears it. Otherwise the centre of the control, then
of the stage.

### Alternatives considered
- Keeping the live vault under the closing lens (as the study does): rejected,
  the vault would show secrets after the backend locked and its hooks would call
  IPC against a locked backend.
- Animating event-driven locks too: rejected, see decision 6.
- A canvas-only reveal (no `clip-path`): the DOM is the picture on unlock and
  `clip-path` also clips hit testing, so the vault is clickable exactly where it
  is visible.
- Reading the live text into the snapshot (`html2canvas`, `foreignObject`):
  rejected, it reads values; a display list built by our own walker has explicit
  exclusion rules and runs under the CSP without blobs or workers.
- Drawing the band with the foil renderer's context: rejected, its program, size
  and `preserveDrawingBuffer` setting are the foil's.

### Consequences
New `apps/desktop/src/reveal/` (display-list collector, canvas painter, WebGL2
band, drivers, phase machine, stage). The unlock and vault screens now live in
layers of a `RevealStage`, only the active layer carries `id="main"`, and
`useShell.lock` seals the vault after sending `lock`. Fidelity limits of the
snapshot: no box shadows, gradients, pseudo-elements or text ellipsis, text
drawn by the canvas rather than the layout engine. E2E specs wait for a reveal
to settle before axe runs. Known cost: each UI unlock and lock in the e2e suite
takes about 2 s longer.

## ADR-0033: Forms for each item kind, custom fields and SSH key paste

- Status: accepted
- Date: 2026-11-05
- Decided by: the owner's direction (per-type forms and custom fields before M4); the details are the most conservative options
- Milestone: M3 (desktop app)

### Context
The editor showed the login form for every kind. The `ItemDraft` contract could already
express custom fields and URL edits, but the UI never sent them and the mock ignored them.
Two things the contract could not express: editing a hidden or one-time-code custom field
without revealing it (`Update.value` was required), and setting SSH key material. Core has
no setter for an item's type, no way to clear key material and no key generation.
`docs/design/screens/D10-NewItem.dc.html` (the login form) is the only mockup; every kind
follows its structure (title, kind fields, websites, one-time code, tags, notes, a ghost
"Add custom field" button).

### Decision
1. **The kind of an existing item cannot change.** The kind picker stays for new items.
   Core has no setter and `type` is metadata where the largest stamp wins, so a kind change
   made on two devices at once would pick one kind and leave fields behind that the other
   kind does not show; the SSH agent and hardware vaults gate on the type. A user who needs
   another kind creates a new item.
2. **While creating, changing the kind keeps a value for every field key** in the form.
   Only the fields in the selected kind's layout are sent, plus title, notes, tags and
   custom fields.
3. **One layout table per kind** (`screens/vault/kinds/layouts.ts`) drives both the editor
   and the detail. Fields that hold a value but are not in the layout (imports, passkeys,
   newer kinds) are listed under "Other fields", so nothing is hidden.
4. **Custom fields cannot be reordered.** `CustomFieldChange` has no position; elements
   are stored one key each and ordered by element ID, which is creation order. Ordering
   would need a core model change.
5. **`CustomFieldChange::Update.value` is `SecretIn | null`.** `null` keeps the stored
   value. It is allowed only when `kind` equals the stored kind and the element exists;
   otherwise the result is `invalidInput`. The label checks are the same as for a value.
   This is how a hidden or one-time-code field is renamed without being revealed.
6. **New custom fields can be** text, hidden, url, email, phone or date. A one-time code is
   not offered (the item's own one-time code covers it). Existing one-time-code custom
   fields, which come from imports, show as a read-only row that can be renamed and
   removed.
7. **SSH keys are paste only.** `ItemDraft.sshKey = { op: "set", privateKey }`, for `sshKey`
   items only. Rust parses (does not decrypt) the OpenSSH key, derives the public key and
   fingerprint and stores `SshKeyMaterial`; any key `ssh_key::PrivateKey::from_openssh`
   parses is accepted, encrypted ones included, so there is no KDF cost. Text that does
   not parse, or is longer than 16 KiB, is `sshKeyInvalid`. The passphrase is the
   `password` secret, labelled "Passphrase", because that is what `AgentKey::from_item`
   reads. There is no key generation: core and the repo have none. Keys the agent cannot
   serve (RSA, P-384, ...) are stored, and the detail says so. The material can be
   replaced but not removed, because core has no clear.
8. **The private key never goes back to the webview.** No reveal, no copy, no export.
   `ItemDetail.sshKey` carries the public key, fingerprint, algorithm, `encrypted` and
   `agentSupported`; `CopyField::SshPublicKey` copies the public key. The textarea that
   takes the key is masked and cleared with the form.
9. **Secret fields in edit mode start empty and mean "keep".** A "Remove" button next to
   each stored secret marks it for clearing (`value: null`). Card numbers are sent without
   spaces; the revealed number is shown in groups (4-6-5 for American Express). The expiry
   is stored as `MM/YYYY` and an expiry that cannot be turned into that blocks saving;
   the Luhn, length and email hints never block.

### Alternatives considered
- Letting the kind change on edit through a new core setter: rejected, see 1; it needs a
  CRDT rule for concurrent kind changes first.
- A separate `rename_custom_field` command: rejected, `Update` with `null` is the same
  edit with the same sync event as every other change.
- Generating SSH keys in Rust: rejected, it adds a crypto dependency path (ADR needed) and
  a key the user must back up elsewhere; paste covers keys users already have.
- Decrypting the pasted key to check the passphrase: rejected, it costs KDF time per save
  and keeps decrypted key material in memory for nothing.
- Showing the private key on request: rejected, the use of a key is signing; nobody needs
  to read it back, and it is the one secret that must never reach JavaScript.

### Consequences
`CustomFieldChange::Update.value` is optional on the wire (the TS type and the fixtures
changed), `ItemDraft` has a new key `sshKey` that is always present (`null`),
`ItemDetail.sshKey`, `CopyField::SshPublicKey` and `ErrorCode::SshKeyInvalid` are new.
`mosaic-ssh-agent` exports `material_from_openssh`, `describe_material`, `KeyInfo` and
`MAX_PRIVATE_KEY_TEXT_LEN`. The mock applies URL and custom-field edits and seeds a card,
an identity and an SSH key item. Attachments for documents are a later change; the document
form carries a hint to save first. THREAT_MODEL needs no new risk: the private key text is
in zeroizing memory from the IPC body to the model, parsed keys are dropped before the
parse function returns, and nothing secret reaches an error or `Debug` output.


## ADR-0034: Encrypted file attachments with an in-memory image preview and Save as

- Status: accepted
- Date: 2026-11-05
- Decided by: the owner's direction (files before M4; preview in memory and Save as only); the details are the most conservative options
- Milestone: M3 (desktop app)

### Context
Items can carry files (CRYPTO_SPEC §7.3: a file key in the item, STREAM ciphertext in the
store; the session API exists since M2), but the desktop app had no way to add, open or
export one. The webview has no fs or dialog permission (ADR-0019), the CSP has no `blob:`,
`data:`, `frame-src` or `worker-src` (and the red-team test asserts it), and a decrypted file
is exactly what Mosaic is built not to leave lying around.

### Decision
1. **All file input and output happens in Rust.** Paths come only from native dialogs that
   Rust opens (`Dialogs` trait, rfd, ADR-0019), never from the webview. Requests carry IDs
   only, with `deny_unknown_fields`. The webview gets no fs or dialog permission.
2. **How bytes move.** Add: Rust reads the file. Save as: Rust writes the file. Preview: the
   webview receives **downscaled RGBA pixels** as one binary IPC response
   (`tauri::ipc::Response`, an `ArrayBuffer` in JavaScript). It never receives the file. No
   base64 and no file bytes in JSON. No chunking is needed, because the preview is at most
   16 MiB plus 8 bytes and files never cross the IPC boundary.
3. **The image preview stays under the current CSP.** Rust decodes the image and the webview
   paints raw pixels into a `<canvas>` with `putImageData`. The CSP is unchanged. Decoding
   uses `mosaic_qr::decode_image`: magic-byte format detection, PNG and JPEG only, the
   dimension, pixel and allocation limits (16384 px per side, 40 MP, 512 MiB, 50 MiB file),
   output into a zeroizing buffer. This is the decoder surface already accepted in ADR-0031.
   GIF, WebP, HEIC and others get Save as only. EXIF orientation and colour profiles are
   ignored. The result is box-downscaled by the smallest integer factor that brings both
   sides to at most 2048 px and the area to at most 4194304 pixels.
4. **No PDF preview in this track.** PDFs get Save as only. The trade-off, for the owner:
   - (a) **Pure-Rust renderer.** The only realistic candidate is `hayro`. It is young and
     unaudited, and it brings a large new parsing surface: font parsing, JPX, JBIG2 and
     CCITT decoders and a CPU rasterizer. It needs its own ADR plus `cargo deny` and an
     inspection of its unsafe code. A crash or logic bug would run in the vault process.
     Value: medium.
   - (b) **pdf.js.** It needs `worker-src` (a CSP change that fails the red-team test) and
     about 1.5 MB of JavaScript. A pdf.js bug would execute inside the vault window, which
     has full IPC access, so it is equivalent to XSS.
   - (c) **The webview's built-in viewer.** It needs a frame plus a `blob:` or `data:` URL (a
     CSP change) and runs PDFium or PDFKit on hostile input inside the vault window.
     WebKitGTK on Linux has no viewer at all, so it is not cross-platform.
   - (d) **Save as only, preview later.** This is the choice: zero new attack surface.

   The likely follow-up is (a), rendering in a separate sandboxed helper process, behind its
   own ADR.
5. **Rust holds no decrypted state between calls.** Preview is one call: decrypt into a
   `SecretVec`, decode into a `Zeroizing` buffer, downscale into a `Zeroizing` buffer, then
   move into the response. `mosaic_qr::decode_image` runs `zune-jpeg` directly over the
   borrowed plaintext (no input copy) and one `image` PNG decoder over the same borrow, checks
   the limits from the header before allocating, and decodes into a buffer that it reserves
   itself and that is wiped on drop, also when the decode fails. The buffers that Mosaic
   owns are wiped before the call returns, except the `Vec` handed to Tauri. Save as decrypts
   after the dialog closes, writes, and wipes. So "drop on preview close, lock and idle tick"
   holds in Rust because nothing is kept. The residual is THREAT_MODEL R35: Tauri's response
   buffer and the webview's copy are ordinary heap memory, and so is the decoders' own
   scratch (the PNG inflate window and row buffers, the zune-jpeg coefficient planes), which
   the decoder crates free without wiping (R31). Only one preview runs at a time (`busy`), and the lock is checked again
   before the pixels are returned.
6. **Webview hygiene for previews.** Right after painting, the pixel `ArrayBuffer` is
   overwritten with zeros. On close, lock and window blur (through
   `useSecrets().register`, the same rule as revealed secrets) the canvas is cleared and set
   to `width = height = 0`. An answer that arrives after the dialog closed is wiped and
   dropped (a generation counter, as in `SecretField`). The canvas wrapper carries
   `data-secret=""` with no `data-mask`, so the reveal snapshot paints the box only and never
   visits the subtree; the preview also sits inside `<dialog>`, which the snapshot skips. The
   file list shows no thumbnails.
7. **Save as.** Rust writes only to the path the save dialog returns. The dialog suggests
   `safe_file_name(name)`: spoofing and control characters removed (an RTL override can
   disguise `exe` as `jpg`), `/ \ : * ? " < > |` replaced by `_`, trailing dots and spaces
   trimmed, a leading dot replaced, Windows device names prefixed, at most 200 bytes with the
   extension kept. Overwriting relies on the native dialog's own confirmation (a manual
   check on each OS is listed in PROGRESS). The write is atomic: a temporary file
   `.<16 hex>.mosaic-partial` in the same folder, created new (mode 0600 on Unix, so no
   execute bit), written, synced and renamed over the target; on any error after the file was
   created the temporary file is removed. On Windows the Mark of the Web
   (`:Zone.Identifier`, `ZoneId=3`) is written to the temporary file before the rename (NTFS
   moves the stream with the file) and again after it as a fallback, best effort, so that
   opening the file triggers SmartScreen and Office Protected View. macOS and Linux get no
   quarantine marker yet (THREAT_MODEL R36). SSH private keys are never exported (ADR-0033); they are not attachments.
8. **Adding a file** is refused with `itemTypeNotAllowed` for items in hardware-protected
   vaults and is hidden in the trash. Preview and Save as are refused for trashed items
   (`itemDeleted`); restore the item first. Adding, previewing and saving re-check the account
   (and, for adding, the vault protection and the trash state) after a dialog, so a lock and
   unlock into another account during the dialog stores or returns nothing.
   The size is checked before reading (50 MiB) and the read stops one byte past it, so a file
   that grows meanwhile is refused; the quota (1 GiB per account) is checked before
   encrypting; a lock during the dialog or the read stores nothing.
9. **Not in this change:** rename (it needs a new session method), drag and drop, and
   thumbnails. On drag and drop: Tauri's drag-drop handler gives paths to Rust
   (`WindowEvent::DragDrop`) and the webview never sees file contents, but sending a drop to
   the right item needs a confirmation flow plus a red-team check that `tauri://drag-drop`
   events cannot reach the webview without `core:event`. That becomes a follow-up ADR.

### Alternatives considered
- Sending the file to the webview and showing it through an `<img>` or `<object>`: rejected,
  it needs `blob:` or `data:` in the CSP and puts decrypted bytes and a second decoder in the
  window that holds full IPC access.
- Decoding in the webview (`createImageBitmap`): rejected for the same reason; the decoder
  would run on hostile bytes inside the vault window.
- Chunked transfer of the file to the webview: rejected, the file never needs to arrive there.
- Writing the saved file in place instead of temp file plus rename: rejected, a failed write
  would destroy the existing target.
- Thumbnails in the file list: rejected, they would decode every image on every view and keep
  pixels of files the user did not open.

### Consequences
Four commands (`attachment_add`, `attachment_preview`, `attachment_save`, `attachment_remove`),
six error codes, `AttachmentEntry.{addedAt, available, previewable}`, and THREAT_MODEL R34 to
R37. The dependency set is unchanged. The first binary response is documented in
DESKTOP_IPC.md §3 and §6.

## ADR-0035: Browser extension architecture, gestures and UI surface

- Status: accepted (2026-11-06; amended in T4 of M4; reviewed by extension-attacker and ux-reviewer, findings fixed)
- Date: 2026-11-05
- Decided by: the principal engineer, from brief §2 and §4 (the most conservative options)
- Milestone: M4 (browser extension)

### Context
Brief §2 fixes Manifest V3, TypeScript, React for the popup, and native messaging to the
desktop app, which holds the unlocked vault. Brief §4 (fill rules) is the highest-risk area
of the product. Chrome, Edge and Firefox must come from one codebase.

### Decision
1. **One tree, two targets.** `apps/extension` builds `chromium` (Chrome and Edge) and
   `firefox` from one source tree with Vite 8: three plain builds (popup and background as
   ES modules; the content and capture scripts as IIFEs), the manifest generated per target.
   No CRXJS and no `webextension-polyfill`: the code uses `chrome.*` with promises, which
   Firefox MV3 supports. `@types/chrome` is the only new dev dependency.
2. **No inline UI in M4.** The extension never inserts DOM into a page. Trusted gestures are
   a click in the popup, the `fill-login` command (Ctrl+Shift+L, Command+Shift+L on macOS)
   and the context-menu items "Fill login" and "Generate password". Brief §4.4 allows
   popup-only fill, and it removes the clickjacking surface.
3. **Content scripts on demand.** They are injected with `activeTab` and `scripting` after a
   gesture. Save capture is the only persistent script, and it is opt-in through
   `optional_host_permissions`. Permissions are `activeTab`, `scripting`, `nativeMessaging`,
   `contextMenus` and `storage`; a production manifest has no `host_permissions`.
4. **The extension holds no vault, store or device ID** (ADR-0009 item 7 and ADR-0015 do not
   apply until a standalone extension vault exists, which needs its own ADR). Unlock happens
   only in the desktop app; the master password never enters the browser.
5. **Storage.** The pairing lives in the extension IndexedDB (background only). Pending
   saves and fill state live in `storage.session` (memory). Never `storage.local` or
   `storage.sync`.
6. **The popup sees no vault secrets.** Copying happens in desktop Rust; OTP codes are filled
   or copied, never displayed; generated values are shown. The primary action uses the static
   holo gradient (ADR-0027).
7. **Mockup deviations.** E01 has no master-password field. E04 moves the title and the vault
   choice to the desktop dialog. E05 has no "Fill anyway" for a non-matching origin. E07 shows
   no code and has no per-site toggle (the OTP is filled in the same gesture when the form
   has an OTP field).

### Alternatives considered
- An inline icon in a closed shadow root with occlusion checks: rejected for M4 as the
  largest attack surface; revisit with its own red-team pass.
- Unlock in the popup: the master password would enter browser memory; brief §2 places the
  unlocked vault in the desktop app.
- `webextension-polyfill`: unnecessary in MV3.
- CRXJS: an extra build dependency.

### Consequences
A fill needs one click or a shortcut. Users keep the desktop app running and unlocked.
End-to-end builds need host permissions, so `activeTab` gating is checked manually and by a
manifest test. Firefox has no automated end-to-end test. T1 (the channel) is in
`crates/native-messaging` and `mosaic_core::browser_channel`; the extension itself follows in
T3 and T4.

### Amendment (M4 T4): the popup and the mockup deviations

The popup is React with the holo primary button as a static stripe tile (the PNG tiles from
`docs/design/holo/`, shipped inside the package, light and dark; no WebGL, and the page CSP
`img-src 'self'` already allows them), built from the shared tokens, in German and
English (`ext.*`). Four mockups differ from what is built, each on purpose:

- **E01 (locked).** No master-password field, no "or" divider and no Windows Hello button: the
  popup says "Mosaic is locked" and offers "Open Mosaic". Unlocking, with any method, happens
  in the desktop app (item 4). The footer's "Sync paused while locked" becomes "Mosaic is
  locked": the popup does not know the sync state.
- **E04 (save prompt).** No title field and no vault field. The popup shows the site, the user
  name and a hidden password; "Save in Mosaic…" asks the desktop, which opens its own dialog
  with the title, the vault and the unencrypted-page checkbox (ADR-0038). The password has no
  "Show password" button: it never needs to be seen in the browser. The update card lower in the
  mockup is the same card with another title ("Update the saved login for {host}?") and the
  name of the login it changes.
- **E05 (address warning).** No "Fill anyway…" and no "Don't fill" button, since there is
  nothing to choose: Mosaic does not fill a site whose address does not match the saved one.
  The only way to fill such an address is to change the login in the desktop app (the per-URL
  HTTP override or eTLD+1 matching, ADR-0037 item 6). The warning shows the address in Unicode
  and as written, and for a lookalike the saved address beside it.
- **E07 (one-time code).** No code on screen and no 30-second ring; no "fill the code
  automatically" checkbox (item 7). The card has "Fill code" and "Copy code"; the code goes
  from the desktop into the field or onto the clipboard (item 6).

Smaller differences: the header has two views (logins and search, generator) and no lock
button: E02's Lock button is omitted, because locking stays in the desktop app and the
extension has no lock op (`lock` is on the never-callable list); the generator's "Fill and save as new login"
becomes "Fill" (the filled password is then offered for saving like a captured one, ADR-0038
item 4); the footer shows the link to the desktop instead of the sync state, and the More
menu holds the save-capture switch and "Disconnect from Mosaic". The popup needs React and
its test libraries, so the dependency note in item 1 reads: `@types/chrome` for the types,
React 19 and the two bundled fonts for the popup, the same versions as the desktop app.

## ADR-0036: Native messaging host, local endpoint and pairing

- Status: accepted (2026-11-06; reviewed by crypto-reviewer, findings fixed)
- Date: 2026-11-05
- Decided by: the principal engineer, from brief §4.7 (the most conservative options); accepted after the crypto review
- Milestone: M4 (browser extension)

### Context
Brief §4.7 requires a channel authenticated and bound to the installed desktop app.
THREAT_MODEL §4.4 asks for a user-confirmed pairing and authenticated messages.
CRYPTO_SPEC §2 has no key-agreement primitive outside HPKE, and adding one (or a protocol
that needs one) is a crypto decision of its own.

### Decision
1. **A separate small binary.** `mosaic-native-host` (crate `mosaic-native-messaging`, whose
   library the desktop shares) relays the browser stdio to the running desktop app. It
   never launches the desktop and never holds vault data.
2. **The host checks its caller.** The argument the browser passes is compared with the
   pinned IDs: the Chrome Web Store and Edge Add-ons IDs (owner), the Firefox add-on ID, and
   the dev ID only in debug builds. Anything else exits with code 2 without connecting.
3. **Endpoints.** Unix: a socket in a `0700` directory with peer-UID checks on both sides
   (the `mosaic-ssh-agent` transport rules, copied; M9 extracts one shared crate). Windows:
   a random pipe name per desktop start, created as first instance with remote clients
   rejected, published in a discovery file under `%LOCALAPPDATA%` that other accounts
   cannot read (they cannot squat the name beforehand).
4. **Pairing is approved in the desktop.** The desktop shows a code that the popup also
   shows; Allow has a 1 s guard; one pending request at a time (a second cancels both); at
   most 3 requests a minute; a 10-minute block after 3 denials; only while unlocked. The
   desktop then draws `pairing_id` and `K_pair` and returns them over the endpoint.
5. **Every connection runs a PSK handshake** (CRYPTO_SPEC §15): mutual HMAC proofs over both
   nonces, the desktop first, then HKDF-derived per-direction XChaCha20-Poly1305 keys with
   strict sequence numbers. The nonce of a frame is derived from its sequence number
   (an exception to the random nonces of §7.2, argued in §15: one key per connection, one
   number once per key).
6. **`K_pair` is stored in the clear** in `browser-pairings.json` (mode 0600 on Unix) and in
   the extension IndexedDB. Both copies are equally exposed to the same user; a stolen key
   is useless while the vault is locked.
7. **Dev builds register the host with `mosaic-native-host install`**, which writes the host
   manifests (and, on Windows, `HKCU` registry keys through `reg.exe`). The M9 installer
   takes over.

### Alternatives considered
- The desktop binary as the host: loads the webview stack for every browser connection and
  conflicts with the single-instance lock.
- An X25519 short authentication string through `hpke`: needs a commitment scheme, which is
  protocol design outside the spec, and gains nothing against the only attacker who can sit
  between host and desktop (P5).
- Noise through `snow`: a new crypto dependency.
- TLS-PSK: `rustls` has no external PSK.
- Keystore storage for `K_pair`: no gain on Windows and the key is lost at logout on Linux.
- Confirming the pairing in the extension (mockup D06): a process that claims to be the
  extension could confirm itself.

### Consequences
The crypto review was done before this ADR was accepted. New residual risks R38 (same-user
malware), R39 (the dev ID), R42 (Windows endpoint identity) and R45 (no forward secrecy, the
code is not a SAS) are recorded. The owner supplies the store IDs and confirms the Firefox
add-on ID. The installer, signing and the pipe SDDL come in M9. The host is built on
`tokio` with `io-std`; `serde_json` gains `raw_value` so request bodies are relayed without
being parsed.

### Amendment (crypto review follow-ups, 2026-11-05)
- **Windows discovery lease.** The desktop keeps `endpoint.json` open (write access,
  `FILE_SHARE_READ`) for its lifetime and the host trusts the file only if a read probe with
  `FILE_SHARE_READ` fails with a sharing violation. A file nothing holds (a crash) is stale and
  refused, which closes the cross-account stale-pipe-name capture of R42. No new crate and no
  unsafe code (std `OpenOptionsExt::share_mode`).
- **Poisoned sessions.** A frame that fails to open ends the session for good: every later
  `seal` and `open` fails with `Decryption`.
- **Unknown pairing ID.** Answered with `refused notPaired` at `hello`, not with a dummy
  challenge: a dummy challenge would make a host whose pairing was removed fail on `proof_d`
  and never learn that the pairing is gone.
- **Bodies are borrowed.** Envelopes borrow their bodies from zeroizing frame buffers and
  secrets are serialized into exactly sized zeroizing buffers (R40).
- **Randomness** for the pipe name and the pairing code comes from `mosaic_core` (CRYPTO_SPEC §2).

## ADR-0037: Fill policy and origin matching in desktop Rust

- Status: accepted (2026-11-06; reviewed by crypto-reviewer and extension-attacker, findings fixed)
- Date: 2026-11-05
- Decided by: the principal engineer, from brief §4.1 to 4.3, 4.5 and 4.8 (the most conservative options)
- Milestone: M4 (browser extension)

### Context
Brief §4 sets the fill rules. Item URLs already carry `UrlMatch` and `allow_http`.
`mosaic-insights::url` deliberately does not decide fills. The desktop holds the vault, so it
is the natural place for every match decision.

### Decision
1. **The desktop decides every match** through the new crate `mosaic-origin`: WHATWG parsing
   through `url` (already in the tree), eTLD+1 through `psl` (new; MIT or Apache-2.0; a
   build-time snapshot of the public suffix list), and the frame origin the browser reports.
   The extension never sees a saved URL.
2. **Default exact origin.** eTLD+1 only for a URL that asks for it (`Etld1`), domains only
   (never IPs, single labels or bare suffixes), scheme equal. A page on plain HTTP matches
   only a URL with `allow_http` (loopback excepted); a saved `http://` URL never matches an
   `https://` page.
3. **Frames.** The desktop refuses a fill unless `origin == topOrigin` (`crossOriginFrame`
   for a `sameOriginFrame` request whose origin differs, `invalidRequest` for a `top`
   request that does). The extension sends only frames whose whole ancestry is same-origin
   and visible, and asks the user to confirm every non-top frame; cross-origin, sandboxed
   and opaque frames are never filled.
4. **One fill returns one item's username and password, or its one-time code** (TOTP and
   Steam only; HOTP is refused with `otpNotFillable` because filling would advance the
   counter; `copy` of the `totp` field of an HOTP login is refused the same way), to one
   frame, pinned by `documentId` where the browser has one. Fields are
   detected again and visibility is checked again at fill time; forms are never submitted.
   A fill that wants both a login and a code returns the code only when the item has a
   time-based one. The vault side is `Backend::browser_fill`; the item must be a live login
   outside the trash in an open vault, else `itemNotFound` (its type is not revealed) or
   `vaultUnavailable`.
5. **Warnings.** An IDN host shows both forms; a lookalike warning (same name with another
   suffix, or edit distance 1 for names of 4 or more characters) appears only when nothing
   matches; plain HTTP is flagged. `listForOrigin` lists a login that matches only through a
   URL without the HTTP override as `blocked: "insecure"`.
6. **No fill of a non-matching item.** The per-URL HTTP override in the desktop editor is
   the only "fill anyway".
7. **Activity.** Only `fill` and `copy` count as user activity (`Backend::touch`), so a
   script polling the endpoint cannot keep the vault from auto-locking.
8. **Rate limits** (per pairing, sliding windows): `fill` 10 per 10 s, `copy` 20 per minute,
   `listForOrigin` and `search` together 60 per minute, `generate` 30 per minute, `checkSave`
   10 per minute (`proposeSave` counts against it too), `proposeSave` 3 per minute; `focus` 1 per 2 s and pairing requests 3 per
   minute globally.

### Alternatives considered
- Matching in TypeScript: duplicated logic, and the extension would need every saved URL.
- `publicsuffix` with a runtime list: a file to ship and to update.
- A confusables skeleton through `unicode-security`: another dependency; the IDN warning and
  exact matching cover it.

### Consequences
Saved URLs without a scheme behave as `https`. Sites that move logins between subdomains
need eTLD+1 on the URL. Any script at the matching origin (an XSS, a confirmed same-origin
frame) can read a filled credential, and eTLD+1 extends that to sibling subdomains (R41). A
paired extension can list login titles through `search`, test candidate passwords through
`checkSave` and copy any login to the clipboard (R43). The pairing prompt and the save
prompt follow the SSH approval pattern (ADR-0025): one at a time, a 1 s Allow guard, expiry
and lock close them, and the captured password never reaches the webview. The `psl` snapshot
is only as new as the release.

## ADR-0038: Save and update prompts

- Status: accepted (2026-11-06; reviewed by extension-attacker and ux-reviewer, findings fixed)
- Date: 2026-11-05
- Decided by: the principal engineer, from brief §9 and §4 (the most conservative options)
- Milestone: M4 (browser extension)

### Context
Brief §9 puts save and update prompts into M4. Seeing what a user types needs a script on
every site, and writing to the vault from the browser is an integrity risk (A4): a page can
feed the script, and a decision taken in the extension alone would write to the vault.

### Decision
1. **Capture is off by default.** The popup's "Offer to save logins" switch asks the browser
   for the optional host permission (`https://*/*`, `http://*/*`) in the click handler, then
   the background registers `capture.js` with `scripting.registerContentScripts`. Turning the
   switch off, or the browser taking the permission back, unregisters it. The script is never
   in the manifest's `content_scripts`, and `check-dist` checks that.
2. **What counts as a submit.** A trusted click on a button, a submit or button input, or a
   `role=button` element, or the Enter key in a text box, in the form (or the nearest
   container of five levels with a password field, never the whole page). The `submit` event
   is not a trigger: the browser raises it as a trusted event for a script's `requestSubmit()`
   and `button.click()`, so a page could fake it; a page can also fire it after any real
   click, so the capture is evidence that the user did something, not that the user
   submitted these values (R44). At most one capture per 2 s. The script reads the fields of
   that form only, takes passwords of 1 to 1024 characters, and sends `captured` (user name,
   password, and for a change form the new password) to the background.
3. **The background decides where it came from.** Origin and tab come from the browser
   (`sender.origin`, `sender.tab`), never from the message. The top frame, or a same-origin
   frame whose ancestors are all same-origin, counts; cross-origin, opaque and hidden-ancestry
   frames are dropped, as are sites marked "Never". With the desktop unlocked it asks
   `checkSave`: `unchanged` is dropped; `new` or `update` is kept as a pending save in
   `storage.session` for five minutes, with the badge `1`. Locked, the capture is kept
   without an answer and the popup asks when it opens.
4. **A pending save is bound to its tab and origin.** It is shown, and sent to the desktop,
   only for the tab's own origin; on another origin (or after five minutes, or when the tab
   closes or leaves the site) it is dropped. A generated password that the popup or the
   context menu filled creates the same pending save, from the value that reached the page
   (a hold is bound to the tab and origin it was made for, is taken once, and the popup's
   Fill never substitutes another password for the one it showed).
5. **The desktop confirms.** The popup shows E04 without title and vault fields. "Save in
   Mosaic…" sends `proposeSave`; the desktop shows its own dialog (title, vault, update or
   new, the HTTP override checkbox on an unencrypted page, Allow after 1 s) and writes the
   item itself; the password stays in Rust. An update puts the old password into the item's
   history. "Not now" drops the pending save; "Never for this site" stores the origin in the
   extension's IndexedDB (`neverSave:<origin>`) and drops it.
6. **Time.** The popup waits for the desktop's answer for up to 305 s (`SAVE_PROMPT_TIMEOUT`
   plus a margin); the host relay waits 302 s for `proposeSave` (`SAVE_PROMPT_TIMEOUT_SECS`
   plus 2 s, so it answers before the popup gives up) and 15 s for every other request.

### Alternatives considered
- Saving directly from the popup: a single decision in the extension would write to the vault.
- A banner injected into the page: rejected by ADR-0035.
- Capture without opt-in: broad host access by default.
- `submit` as the trigger: not a user gesture (item 2).

### Consequences
Two confirmations (popup and desktop). A pending save survives a service-worker restart but
not a browser restart. Logins submitted from a custom control that is not a button (a `div`
with a click handler and no role) are not noticed. The "Never for this site" list has no
screen yet; clearing the extension's data clears it. R43, R44.

## ADR-0039: Passkey authenticator, encodings and storage

- Status: accepted (2026-11-06; reviewed by crypto-reviewer, extension-attacker and ux-reviewer, findings fixed)
- Date: 2026-11-06
- Decided by: the principal engineer, from brief section 5 "Passkeys" and M5 (the most conservative options)
- Milestone: M5 (passkeys)

### Context
Brief section 5 makes Mosaic a passkey provider: passkeys are vault items, used in the browser
and the apps. CRYPTO_SPEC section 2 already named `p256` for "Passkeys". The model already has
`PasskeyCredential` (credential ID, rp ID, user handle, names, a `private_key` buffer, a COSE
algorithm, a discoverable flag), `ItemType::Passkey` and a sign count merged by maximum. The
`crates/passkey` crate was an empty scaffold. There is no CBOR crate in the tree and `pkcs8` /
`spki` were not in the lockfile.

### Decision
1. **ES256 only** for create and get (D1). EdDSA is not implemented; an imported EdDSA key is
   kept out with a warning (ADR-0041). Every relying party must accept ES256, and one
   algorithm means less code to review.
2. **Primitives and encodings** (D2). ECDSA P-256 with SHA-256 and RFC 6979 nonces from `p256`
   0.14.0 with the features `alloc`, `ecdsa` and `pkcs8`. The `pkcs8` feature adds two new crates,
   `pkcs8` 0.11.0 and `spki` 0.8.1 (both stable releases, same RustCrypto train as `der` 0.8.3 and
   `sec1` 0.8.1 already in the tree). They are encoding crates, not primitives: PKCS#8 is what
   the model documents and what Bitwarden, KeePassXC and CXF carry; RustCrypto parsing of
   untrusted DER beats hand parsing. `spki` comes with `pkcs8`; the passkey crate uses it for
   `from_public_key_der` / `to_public_key_der` in its tests. The SPKI that `getPublicKey()`
   returns is the constant 26-byte prefix, `04`, X and Y, built without a fallible step and
   checked against `to_public_key_der` in a test. No new
   primitive, no `rand_core` (ADR-0011): the key scalar is drawn with `mosaic_core`'s
   `random(32)` and rejection sampling (FIPS 186-5 A.4.2).
3. **CBOR is written by hand for three fixed structures** (D3): the COSE_Key, the `none`
   attestation object and byte-string headers. There is no CBOR decoder in Rust. `serde_cbor`
   is unmaintained and `ciborium` brings a whole CBOR stack for three constant layouts.
4. **Authenticator data** (D4): credential ID `random(16)`, AAGUID 16 zero bytes, attestation
   `none`, flags create `UP|BE|BS|AT` (plus `UV`), get `UP|BE|BS` (plus `UV`). A zero AAGUID
   does not identify Mosaic users.
5. **Sign count** (D5): 0 for credentials Mosaic creates, never written. An imported counter
   `c > 0` signs with `c + 1` and is persisted before the signature is returned, saturating at
   `u32::MAX`, so a relying party that saw `c` does not flag a clone. The crate enforces it in
   types: `assert` returns a `PendingAssertion` whose signature is private; it is released with
   a `CountPersisted` token (built after the store write) or, for a counter that stays 0, with
   `into_response_without_count`, which refuses while a count is pending.
6. **User verification** (D6): UV is set only if the desktop saw a user verification in the
   last 300 seconds (unlock, quick unlock, re-authentication). Otherwise, if the relying party
   asks `required` or `preferred`, the dialog asks for the master password through the
   existing re-authentication path (with its 5-failure lock). `discouraged` needs only the
   dialog click and sets UV = 0.
7. **Storage** (D7): one `passkey` item per credential. The private key is
   `PasskeyCredential.private_key` (a zeroizing buffer holding PKCS#8 DER) in the encrypted
   item plaintext, not a `SecretField`, so no reveal, copy or `SecretRef` can reach it. The public
   key is not stored; it is derived. Login items never carry a `passkey`. No core format change.
8. **Test vectors.** The WebAuthn specification has no private-key vectors. Known answers are
   RFC 6979 A.2.5 (signatures over `sample` and `test`) and pinned encodings (V1 to V11 in
   CRYPTO_SPEC section 16), and an independent verifier written in TypeScript with only
   `node:crypto` and its own CBOR reader checks the public key, authData, attestation object
   and signatures the Rust code emits.

### Alternatives considered
- `passkey-rs`, `webauthn-authenticator-rs`: OpenSSL or large extra dependency trees, and they
  put policy we want in our own reviewed code.
- `ciborium` or another CBOR crate: a general codec for three constant layouts.
- A random AAGUID per install: fingerprints users. A registered AAGUID (to show "Mosaic" next
  to the passkey) is an owner decision.
- "Unlocked" as user verification indefinitely: a stolen unattended session would sign with UV
  forever. The 300 s window plus the password path is the conservative middle without a
  per-operation biometric API.

### Consequences
CRYPTO_SPEC sections 2, 4 and 16. Two new encoding crates (`pkcs8` 0.11.0, `spki` 0.8.1),
only through `p256`'s feature. The lockfile also gains `serdect`, an optional dependency of the
RustCrypto crates (a weak feature entry); `cargo tree -e features -i serdect` shows it is not
built for any workspace member (lockfile-only). Residual risks R48 (synced passkeys can be copied and carry no
clone detection), R49 (UV means "recently unlocked plus a dialog click") and R53 (imported
counters keep incrementing). Follow-ups: biometric UV through a quick-unlock re-verification
proof (needs a vault API and a crypto review); EdDSA if relying parties or imports need it.

---

## ADR-0040: Browser passkey interception

- Status: accepted (2026-11-06; reviewed by crypto-reviewer, extension-attacker and ux-reviewer, findings fixed)
- Date: 2026-11-06
- Decided by: the principal engineer, from brief section 5 "intercept safely, fall back to the platform authenticator" and M5 (the most conservative options)
- Milestone: M5 (passkeys)

### Context
Mosaic is a software passkey authenticator (ADR-0039). To use it in a browser, something has to
see the page's `navigator.credentials.create` and `.get` calls. Only the page's own JavaScript
world can intercept them, and that world is hostile: the page can replace whatever runs there,
watch its messages and forge answers. The interception must therefore be useful without being
trusted, must not take over sites that Mosaic cannot serve, and must follow the rule of the save
prompt (ADR-0038) and the SSH approval: Mosaic signs or writes only after its own window.

### Decision
1. **A main-world shim plus an isolated bridge, off by default** (D8). `passkey-main.js` (main
   world) and `passkey-bridge.js` (isolated world) are registered with
   `scripting.registerContentScripts` only after the user turns on "Use Mosaic for passkeys" in
   the popup and grants the optional access to `https://*/*` (the pattern of ADR-0038). They
   run at `document_start` in the top frame only (`allFrames: false`) of https pages, are never in
   the manifest, and are unregistered when the switch goes off or the browser takes the access
   back. A page that is already open keeps them until it reloads, so the background checks the
   setting on every request.
2. **The shim is not trusted** (the trust analysis). The shim runs in the page's context. The
   page can disable it, route around it, observe its postMessage traffic and forge requests and
   answers. All of that is acceptable because everything the page can do through the shim it can
   already do with `navigator.credentials` for its own origin, and a forged answer fools only
   the page that forged it. The trust boundary is **bridge, background, desktop**:
   - The origin comes from the browser: `MessageSender.origin` (Chromium) or the origin of
     `MessageSender.url` (Firefox), and it must equal the bridge's isolated-world
     `window.origin`, which the page cannot redefine. It must be https, and the tab must still
     show that origin.
   - Only `frameId === 0` is served; a cross-origin child frame cannot post into the top window
     with `event.source === window`, and subframes get no scripts at all.
   - The request ID is made by the background (16 random bytes as hex), never taken from the
     page; the page's own counter is only a handle that is bound to tab, document and frame.
   - The relying party ID is checked in Rust (see 4); `clientDataJSON` is built in Rust from the
     browser's origin; every create and every sign-in needs the desktop dialog.
   - The bridge listens to its own window only (`event.source === window`) and answers only
     messages of exactly the shim's shape and size. `content.js` and `capture.js` still listen to
     no page message (`check-dist.mjs`).
3. **Native fallback** (D11). When Mosaic does not handle a request, the shim calls the saved
   native `create` or `get` with the page's original options object and `this`, so the browser
   produces its correct native behaviour and errors. Not handled: the setting off, a locked
   desktop, no pairing, no desktop, no matching credential, unsupported options (conditional or
   silent mediation, a cross-platform authenticator, enterprise attestation, the PRF and
   large-blob extensions, any size limit exceeded, serialization failures), any subframe, any
   non-https page, a tab that is not focused and visible, and the user choosing "Use another
   device or security key". The shim also falls back when the bridge does not acknowledge a
   request within 1 s (a registered shim without a bridge must not hang the page).
4. **The relying party ID rule is stricter than WebAuthn** (from the T1 review,
   `mosaic_origin::webauthn_rp_id`). WebAuthn L3 5.1.3 step 8 lets a page use its host or a
   registrable domain suffix of it. Mosaic additionally requires: https only; the host a domain,
   not an IP address or loopback; a given rp ID ASCII-lowercase and canonical (no trailing dot, at
   most 253 bytes); equal to the host or a dot-suffix of it; not a public suffix (it has a
   registrable domain in the public suffix list); and **at or under the host's registrable
   domain**, that is `rp == site || rp.ends_with("." + site)` where `site` is the registrable
   domain of the host. The last condition keeps a tenant of a wildcard public suffix
   (`a.b.platformsh.site`, `a.b.kawasaki.jp`) from claiming a parent that is shared with other
   tenants, which a plain suffix test would allow. The fake desktop of the e2e build uses only
   host equality or dot-suffix and is labelled as a fake; the real rule lives in Rust.
5. **Messages and limits.** Page to bridge (`mosaic-passkey-req`, `-abort`; bridge to page
   `-ack`, `-res`) and bridge to background (`passkey`, `passkeyAbort`) carry exactly the keys in
   `src/shared/passkey-messages.ts`, with the limits of `src/shared/passkey-limits.ts` that equal
   those of the desktop ops: challenge 16 to 1024 bytes, user ID 1 to 64 bytes, names at most 256
   UTF-16 units, rp ID at most 253, at most 16 algorithms, lists of at most 64 credential IDs of
   1 to 1023 bytes. The bridge allows 2 requests in flight per page; the background 10 messages a
   second per tab. The extension waits 305 s for `passkeyCreate` and `passkeyGet` (the desktop
   dialog's 300 s plus margin) and 20 s for everything else; `passkeyCancel` is never held back by
   the in-flight cap and always answers `{}`.
6. **Desktop approval** (D9). Every create and sign-in is approved in the desktop's
   `BrowserPasskeyDialog` (1 s Allow guard, rp ID and origin shown, account picker, "Use another
   device or security key"). The E06 content of the extension mockup moves into that desktop
   dialog: an extension click is a page-adjacent surface, and phishing-resistant signatures are
   not given on the strength of one.
7. **Error mapping.** `cancelled`, `busy` and `timeout` end the page's call with
   `NotAllowedError`; `passkeyExcluded` with `InvalidStateError`; `useAnotherDevice` and every
   other code (`rateLimited`, `passkeyNotFound`, `passkeyUnsupported`, `rpIdMismatch`,
   `insecureOrigin`, `crossOriginFrame`, `locked`, `notPaired`, `desktopNotRunning`,
   `invalidRequest`, ...) hand the call to the browser. `rateLimited` is a fallback because the
   desktop's windows belong to the pairing: as an error, one site could spend them silently and
   make every other site's call fail for a minute (extension-attacker pass). A page that aborts
   (`AbortSignal`) rejects with the signal's reason and the background sends `passkeyCancel` for
   exactly that request; a closed, replaced or navigating tab cancels its open requests. The
   background holds a request from its arrival, so a cancel that comes before the desktop was
   asked stops it from being sent, and the shim posts an abort when it falls back for want of the
   bridge's acknowledgement.
8. **Not in M5** (D12): conditional mediation (passkey autofill UI), PRF, large blob, related
   origins (`/.well-known/webauthn`), cross-origin iframes, OS passkey providers (ADR-0041).

### Alternatives considered
- `chrome.webAuthenticationProxy`: Chromium only, takes over every request in the browser with
  no per-request fallback to the platform authenticator.
- A content script only in the isolated world: it cannot see or replace the page's
  `navigator.credentials`.
- In-page or extension-window approval: it would trust an extension click with a phishing
  resistant signature.
- Serving same-origin iframes: rare and a larger trust question (which frame did the user
  mean); not served, conservative.
- Trusting the shim's origin or a page-chosen request ID: the page controls the shim.

### Consequences
Residual risks R47 (the page-controlled shim and two scripts on every https page), R50 (timing
shows whether Mosaic holds a passkey for the page's own rp ID; conditional mediation is not
supported) and R51 (the desktop trusts the extension's origin claim, as for `fill`, R43).
Firefox registers MAIN-world scripts from version 128 (`strict_min_version` is 128); this is
from the documentation and is not covered by the Playwright suite, which cannot load Firefox
extensions. Chromium is covered by `e2e/hostile/passkey-*.spec.ts` with a CDP virtual
authenticator standing for the browser's own WebAuthn.

Amendment at the M5 close (2026-11-06, extension-attacker finding): the rate windows belong to the
pairing, so one page could spend them for every site. The desktop now also keeps a window per
(pairing, origin), 2 `passkeyCreate` and 6 `passkeyGet` a minute, checked before the pairing's; a
request one of them refuses counts in neither. At most 256 origins are kept per pairing and bucket.
`NATIVE_MESSAGING.md` section 6 and `EXTENSION_SECURITY.md` section 11.3 have the details.


## ADR-0041: Passkey import, CXF export and OS passkey providers

- Status: accepted (2026-11-06; reviewed by crypto-reviewer, extension-attacker and ux-reviewer, findings fixed)
- Date: 2026-11-06
- Decided by: the principal engineer, from brief section 5 "Passkeys" and M5 (the most conservative options)
- Milestone: M5 (passkeys), track T4

### Context
Brief section 5 asks for passkey import and CXF import and export. Until M5 the importers only
warned `passkeysNotImported` for Bitwarden `fido2Credentials`, KeePassXC `KPEX_PASSKEY_*` and 1PUX
passkeys. Passkeys now exist as `passkey` items (ADR-0039), with an ES256 key as PKCS#8 DER.

### Decision
1. **Import** (D13). Passkeys come from three formats and each becomes **its own `passkey` item**,
   written right after the entry it came with; the preview counts include them as `passkey`.
   - Bitwarden JSON: each `fido2Credentials[]` member. `credentialId` is a UUID string (its 16 raw
     bytes) or `b64.<base64url>`; `keyType` must be `public-key`, `keyAlgorithm` `ECDSA` and
     `keyCurve` `P-256` (another value is `passkeyUnsupported`, a missing one is
     `passkeyUnreadable`); `keyValue` is base64url PKCS#8; `userHandle` is base64url (null is
     `passkeyUnreadable`); `counter` is a string of ASCII digits or a non-negative integer, at most
     `u32::MAX`, and becomes the item's sign count (a float, a flag, a sign, a space or a larger
     value is `passkeyUnreadable`);
     `discoverable` is `"true"` or `"false"`; `creationDate` is RFC 3339 (the import time if absent
     or unreadable).
   - KeePassXC KDBX: the `KPEX_PASSKEY_*` attributes. `KPEX_PASSKEY_PRIVATE_KEY_PEM` is stripped of
     its `BEGIN/END PRIVATE KEY` lines and whitespace **into a zeroizing buffer** and decoded with
     strict standard base64; credential ID and user handle are base64url. A missing required
     attribute is `passkeyUnreadable`.
   - FIDO CXF v1.0 JSON (new `ImportFormat::Cxf`): see 3.
   - 1PUX still holds no private key; it keeps `passkeysNotImported`.
   - Every key goes through `mosaic_passkey::Es256Key::from_pkcs8_der` and is stored as
     `to_pkcs8_der` (one encoding in the vault). A well-formed PKCS#8 key of another algorithm
     (Ed25519, RSA, another curve) is `passkeyUnsupported`; a P-256 key that does not load (a
     mismatched public key) and anything else that does not parse, and every size out of range
     (credential ID 1 to 1023 bytes, user handle 1 to 64), is `passkeyUnreadable`
     (`mosaic_passkey::pkcs8_kind` tells the two apart from the structure). Base64url is read
     unpadded or padded (the padded form is accepted); anything else, such as the standard
     alphabet, is refused. The **rp ID** is canonicalised by `mosaic_origin::canonical_rp_id`:
     ASCII-lowercased, one trailing dot removed, then it must be a canonical domain name (the
     check `webauthn_rp_id` applies: no U-label, IP address, URL, space, port or
     direction-changing character, no empty label, at most 253 bytes) with a registrable domain in
     the public suffix list (so `com` is refused). The canonical form is stored, so rp IDs that
     differ only in case are one credential; anything else is `passkeyUnreadable`. A passkey item
     without a title gets its rp ID, cleaned like a title (`strip_spoofing`). User names and display names go through
     the same `mosaic_passkey::sanitize_name` the desktop applies to names from sites (moved from
     the desktop into `mosaic-passkey`; one implementation).
   - The parsers are hostile-input code: sizes are checked before anything is allocated, base64url
     is decoded strictly (no whitespace, no non-canonical tails), keys live in zeroizing buffers,
     nothing from the file is in an error. The import fuzz crate gets a `cxf` target and passkey
     seeds.
   - A passkey whose credential ID and rp ID are already in the account, or appeared earlier in the
     file, marks a passkey-only entry as a duplicate in the preview (it starts unticked). `commit`
     also skips such a passkey (in the vault or written earlier in the same commit) and counts it
     in `skipped`, so a ticked duplicate never makes a second item. A favorite mark carries over to
     passkey-only items.
   - Caps: at most 100 passkeys per entry and 10,000 per import (`MAX_IMPORT_PASSKEYS`). The check
     comes before a key is parsed, so a hostile file cannot cause elliptic-curve work for passkeys
     that are dropped; the rest are left out with the entry warning `passkeysLimit` and the file
     warning `passkeysSkipped`. A CXF item adds its `scope.urls` once, not once per `basic-auth`
     credential (a file with thousands of both was quadratic).
2. **Export** (D13). `export_passkeys_cxf { vaultId, password | null }` writes the ES256 passkeys
   of one vault as a CXF v1.0 JSON file. It needs a right master password (a non-empty `password`
   is checked first, wrong ones count towards the five-failure lock) and **takes** the two-minute
   re-authentication (`take_reauth_grant`), so one password pays for one export; a cancelled dialog
   or a failed write hands it back. Rust opens the native save dialog (ADR-0034 pattern, default
   name `mosaic-passkeys-YYYY-MM-DD.json`), builds the file in one zeroizing buffer sized by a
   counting pass (it never grows, so no stale copy of a key stays behind) only after the dialog
   returned a path, and writes it with `write_file_atomic` (a temporary file next to the target,
   mode 0600 on Unix, the folder's ACL is inherited on Windows, the Mark of the Web on Windows).
   A crash between the write and the rename can leave a `.<hex>.mosaic-partial` file holding all
   keys in the chosen folder (R52). The account ID and every item ID in the file are fresh random
   16-byte values per export, so the file carries neither the vault ID nor an item ID. Nothing goes to the system temp directory. Settings
   has an "Export passkeys" panel with the warning "The file contains your passkeys unencrypted.
   Anyone with the file can sign in as you. Delete it after importing." and the password dialog.
   A write error reuses `attachmentSaveFailed`; a vault with no ES256 passkey answers
   `itemNotFound` before the password is spent.
3. **CXF field names.** Checked against the published FIDO Credential Exchange Format v1.0
   Proposed Standard (2025-08-14, `fidoalliance.org/specs/cx/cxf-v1.0-ps-20250814.html`). Header:
   `version {major, minor}`, `exporterRpId`, `exporterDisplayName`, `timestamp` (Unix seconds),
   `accounts[]`; account: `id`, `username`, `email`, `collections`, `items`; item: `id`,
   `creationAt`, `modifiedAt` (seconds), `title`, `subtitle`, `favorite`, `scope {urls,
   androidApps}`, `credentials[]`, `tags`; `type` is a member of each **credential**, not of the
   item; passkey: `credentialId`, `rpId`, `username`, `userDisplayName`, `userHandle` (all
   base64url) and `key` (base64url PKCS#8 DER); `basic-auth`: optional `username` and `password`
   (`EditableField`: `fieldType`, `value`), the URLs come from the item's `scope.urls`; `note`:
   `content`. The importer reads `version.major == 1` (else `importUnsupported`), passkeys,
   `basic-auth` (as a login with the scope URLs), `note` (as the notes), item `tags` and
   `favorite`; any other credential type is not imported and the entry gets `unmappedFields`.
   Two deviations from the M5 spec text, which came from memory: there is no `type` on the item,
   and `basic-auth` has no `urls` (they are the item's `scope`). Not verified: the standard
   defines the data model in CDDL; the JSON used here is the serialization the member names imply,
   and no CXF file from a third-party exporter was available to test against. A file from another
   exporter may differ in details (extra members are ignored).
4. **OS passkey providers are deferred.** Windows 11 third-party passkey plugin API is reachable
   only through `webauthn.dll` COM/FFI, which needs `unsafe` (forbidden in every crate); macOS
   needs a signed Credential Provider app extension (the Apple Developer ID is already an owner
   item); Linux has no OS API. Mobile providers belong to M10.

### Alternatives considered
- An encrypted export: CXF has no encrypted file form (CXP is a working draft). The brief asks for
  CXF, so the export is plaintext, gated and labelled.
- Importing EdDSA and RSA keys: not implemented anywhere else (ADR-0039 D1); a key that cannot be
  used would only look imported.
- Writing the export through a temporary file in the system temp directory: plaintext keys would
  leave the destination folder.

### Consequences
THREAT_MODEL R52 (an unencrypted export file holding private keys) and R53 (imported counters).
PROGRESS M3-R19 is updated: passkeys are imported except from 1PUX. DESKTOP_IPC gets
`export_passkeys_cxf`, the `cxf` import format and the warnings `passkeyUnreadable` and
`passkeyUnsupported`. `mosaic-core` gains `PasskeyPublic` and `Item::passkey_public`, so code that
lists or filters passkeys never copies a private key; the private key is copied only for the one
credential being signed with or exported. Follow-ups: owner decision on dropping the export; a CXF
file from a second implementation as a fixture when one is available.

## ADR-0042: M6 scope and one protocol over two transports

- Status: accepted
- Date: 2026-11-06
- Decided by: the principal engineer, from brief section M6 and the owner direction of 2026-11-02 (own-cloud-folder backend)
- Milestone: M6 (sync)

### Context
The brief's M6 bundles the sync server, sharing, organisations, emergency access and WebAuthn 2FA. The
owner also wants a backend that needs no server: a user-chosen directory that Dropbox, OneDrive, iCloud
Drive, Nextcloud, Syncthing, a NAS share or a Dropbox Business team folder replicates. Sharing needs the
`hpke` crate vetted, identity keys and revocation (THREAT_MODEL R18); it would double the milestone.

### Decision
1. **One sync protocol, two transports.** The sync engine talks to a blocking `Transport` trait
   (`mosaic-sync`, `crates/sync/src/transport.rs`). `hosted` is the Mosaic server, `folder` is a
   directory. The engine, the merge rules and the key rules are the same for both.
2. **In M6:** record, tombstone, vault-key and vault-metadata sync including the key and metadata upload
   queue (M2-R1, R11, R16); server generation rules (ADR-0013); attachments with soft-delete GC;
   checkpoints with freeze and rollback detection; a stable per-install device ID; the device list and
   hosted sign-out of a device; OPAQUE registration, login and re-registration with session tokens and
   TOTP 2FA for the sync account; joining a new device from either backend; the master-password change
   carried to the backend.
3. **Deferred:** sharing, organisations, team vaults and emergency access (M6b); WebAuthn 2FA for the
   sync account (M9); settings as a synced record (M7, M3-R23, R28); move pointers (M2-R8, R9) and
   "flag source if copy unreadable" (M7); full re-key and Secret Key rotation (M7); hardware-protected
   (TK) vaults stay on their device (R60); publishing the server image (M9).
4. **Sync is off by default** and there is no built-in server URL (`HOSTED_DEFAULT_URL = None` until the
   owner picks a domain).
5. **Long-poll replaces WebSocket** (a change to the brief): a 25 s `GET /v1/poll` returns on a change.
   **SQLite replaces PostgreSQL** for the server; the storage boundary keeps Postgres possible. The
   Compose deployment is shipped but is built and smoke-tested only in CI (the owner forbids local
   Docker).
6. **A Dropbox Business team folder is a directory on disk.** There is no Dropbox API. In M6 a team folder
   holds several people's separate personal accounts side by side (`a/<account_id>/...`); the layout
   reserves `s/` for shared vaults in M6b. Other members and the admin are treated like the server (R54).

### Alternatives considered
- WebSocket push: a new parser and dependency surface and reverse-proxy problems; long-poll needs
  neither.
- Provider APIs (Dropbox SDK and similar): network dependencies and OAuth tokens in the client.
- Including sharing in M6: `hpke` vetting and revocation would double the milestone.

### Consequences
Brief deviations: SQLite, long-poll, Compose never run locally, sync off by default. M6b gets its own
spec. THREAT_MODEL R54 to R60 are added.

## ADR-0043: Folder backend format and integrity

- Status: accepted
- Date: 2026-11-06
- Decided by: the principal engineer, from ADR-0042 (the most conservative options)
- Milestone: M6 (sync)

### Context
A file-sync client replicates a directory with its own conflict handling: it may deliver a half-written
file, deliver files out of order, rename a file to "(conflicted copy)" or keep old versions. The folder
holds only ciphertext, but a hostile or careless provider can delete, replace or withhold files.

### Decision
1. **Per-device append-only segments** (`.mseg`) with an HMAC under `K_fold`
   (`HKDF(AK, "", ctx("sync/folder-mac-key"), 32)`); the MAC is checked in constant time before parsing.
2. **Single-writer paths**: every path is written by exactly one install, so a cloud conflict copy can
   only appear for `head.mck`, `info.mdev` and two fixed top-level files. Blobs are write-once with
   identical content for an ID everywhere.
3. **A strict name parser** accepts only the canonical file and directory names; every other name
   (conflict copies of every provider, temporary and OS files) is ignored. The one exception is a device
   directory's `*.mck`, where the verified candidate with the highest counter wins.
4. **Checkpoints as head files** (the hosted checkpoint bytes, `head.mck`, rewritten with tmp and
   rename); snapshots and compaction of a device's own log; blob delete markers (`.mdel`) with 30 days
   of retention and no purge while a live local item references the blob; device info sealed under
   `K_dev` (`HKDF(AK, "", ctx("sync/device-info-key"), 32)`).
5. **Writes** go through `.<16 hex>.mtmp`, `sync_all` and `rename`; a write-once name is checked for
   absence first.
6. **Account file** `.macct` holds salt, profile and the password wrapping of AK with an
   `account_version`; the current file has the highest version.

### Alternatives considered
- One SQLite file in the folder: file-sync clients corrupt it.
- One file per record: far too many files and conflict copies.
- Signatures (Ed25519): not needed, every reader holds `AK`, so HMAC from CRYPTO_SPEC section 2 suffices
  and a forger needs `AK`.

### Consequences
R54 (observers and deleters) and R55 (no compare-and-swap: concurrent rotations are resolved by the
canonical winner, delete races R15 and R16 remain in folder mode). Files are only appended until a
device compacts its own log.

### Amendments (crypto review of M6 T1, 2026-11-06)
Formats are in CRYPTO_SPEC section 10.1.
- Snapshots may span several parts: `snap_base` and `snap_parts` follow `created_at_ms` in the MAC'd
  header; a reader marks older sequence numbers read only when the whole set is verified.
- The delete marker carries `vault_id` and lives at `blobs/<attachment>.<device>.mdel` (93 bytes), so
  every install writes only its own marker (single-writer rule).
- Device info is padded to a constant 512-byte plaintext (`len:u16 || JSON || zeros`).
- Transports carry checkpoints as raw bytes filed under the path's device ID; the body must name the
  same device (`Path` error otherwise).
- A new checkpoint version must use a new MAC label or put the version byte into the MAC input.

### Amendments (M6 T2 engine and folder transport, 2026-11-06)
Rules added while implementing the engine; none changes a byte format.
- The folder transport reads this install's own log for vault-key objects only (a pushed but not
  committed rotation is recovered after a crash); a key received from another device is never
  attributed to the reader in the canonical-winner rule.
- A blob is purged only when the purging device's run saw no other device behind, besides the
  retention period and the absence of a live local reference; holders re-assert (remove their own
  marker, upload the local copy again if the blob is gone).
- Compaction triggers on the own segments written since the last complete snapshot set (more than 64
  segments or 32 MiB); it writes every part first and deletes the older own segments last.
- The transport trait gained `PullPage::metas` (a folder page can hold one metadata object per
  segment; the engine merges them), `checkpoint_high_seq` (the folder answers with the highest own
  segment sequence) and `maintain` (compaction, cleanup and blob purge, with the session answered
  through `MaintainContext`). (Amended 2026-11-06: the folder transport holds no key; it derives them per operation from a shared
  session handle, see the ADR-0045 amendment.)
- An engine whose own checkpoint on the backend is below its last published counter counts one strike
  per counter value and halts on the second (`RollbackDetected`); a verified checkpoint of its own
  install above its counter halts at once (`IdentityDuplicated`). Both stay halted, across locks and
  restarts, until `clear_halt` (the halt is stored; amended 2026-11-06).

## ADR-0044: Server stack and deployment

- Status: accepted
- Date: 2026-11-06
- Decided by: the principal engineer, from ADR-0042
- Milestone: M6 (sync)

### Context
The brief wants a self-hostable sync server. Self-hosters want one process, one data directory and no
extra database.

### Decision
axum and rusqlite (bundled, WAL, one connection behind `spawn_blocking`); the endpoints of the M6 spec
with JSON bodies that reject unknown keys on both sides; OPAQUE per CRYPTO_SPEC section 9; sessions with a
SHA-256 token hash, idle and absolute lifetimes and a fresh-login window; TOTP 2FA sealed at rest;
in-memory rate limits with no persisted IP addresses; quotas per account; a TOML config
(`deny_unknown_fields`) and a hand-parsed CLI; TLS at a reverse proxy; a distroless image built only in
CI.

### Alternatives considered
- PostgreSQL with sqlx: an extra service for self-hosters; the storage boundary keeps it possible.
- Built-in TLS: certificates belong in the proxy.

### Consequences
Single-node scale (thousands of accounts); backups through `VACUUM INTO`. THREAT_MODEL section 4.5 says
rusqlite with bound parameters.

### Amendment (2026-11-06, T3: the server as built)
Choices that the spec left open, each the most conservative one:
- **One connection, no `spawn_blocking`.** The database is one `rusqlite` connection behind a mutex. On a
  multi-threaded runtime a query runs inside `block_in_place`; on a current-thread runtime (the tests, which
  pause the tokio clock, where `spawn_blocking` would let the paused clock jump ahead) it runs inline.
  `secure_delete` is on and temporary storage is in memory.
- **Per-account change counters** for the long poll (a global counter would wake every client for every
  write on the server and show one account the activity of the others). A counter starts at the clock, so a
  restart never repeats a value a client holds. A second poll of a session is refused with
  `too_many_polls` and releases the older one. Checkpoint uploads do not bump the counter (they would
  start an endless round of syncs).
- **A checkpoint is uploaded only by its own device** (path device = session device, else 403): the
  server cannot verify the MAC, so it limits who can raise a device's counter. The counter only grows
  (`checkpoint_stale`); a removed device's checkpoint is deleted.
- **Page size.** A page of changes holds at most 500 records and at most 8 MiB of ciphertext (the client
  reads at most 16 MiB of JSON), and always at least one record.
- **Quotas** (`quota_exceeded` with `{kind}`): `recordCount`, `recordBytes`, `attachmentBytes`,
  `devices` (32, not counting removed devices) and `vaults` (1000); a batch that would pass a record
  quota fails as a whole.
- **Rate limits** key IPv6 clients by their /64. `X-Forwarded-For` is read only from a peer in
  `trusted_proxies`, and only its last hop.
- **Invite codes** are 20 Crockford base32 characters (100 bits), stored as SHA-256, single use,
  redeemed in the registration transaction. `register/start` takes no code (the request has no field
  for it), so `invalid_invite` comes only from `register/finish`.
- **Registration is stateless.** `register/start` answers the same for a registered and an
  unregistered email; `account_exists` comes at `finish` (THREAT_MODEL R58).
- **Attachments** are stored by `PUT` after the structural check of `validate_attachment_ciphertext`
  (which accepts any canonical STREAM length: a truncation at a chunk boundary is found only by the
  authentication tag at the client). An identical, soft-deleted attachment is restored by a repeated
  `PUT`. Blobs are never served to another vault: a mismatch of vault is `not_found`.
- **Password change** does not end the account's other sessions (the spec says nothing; ending them
  would sign out devices that have not yet seen the new password). The change is audited.
- **Image and compose**: base images are pinned by digest (builder `rust:1.99.1-bookworm`, runtime
  `gcr.io/distroless/cc-debian12:nonroot`, proxy `caddy:2.11-alpine`); the server container sits on an
  internal network without a route to the internet and with a fixed address for the proxy, which
  `trusted_proxies` names.

### Amendment (2026-11-07, T3 review follow-ups)
- **Pre-login does not reveal an account (H1).** New account IDs are random UUIDv4
  (`AccountId::new_random`) and a fake pre-login ID is a UUIDv4 from the HMAC, with no timestamp:
  real and fake IDs are both uniform random. The backdating rule of the first amendment and
  its `init_ms` input are gone. UUIDv7 account IDs from before this change (pre-release) stay valid.
- **One key per purpose (I2).** `K_prelogin` and `K_totp` are HKDF subkeys of `server_secret` with the
  labels `server/prelogin-key` and `server/totp-key`; `server/totp-at-rest` is only the AAD label of a
  sealed secret, which now carries a version and algorithm byte (I3). Labels are in the core registry.
- **TOTP limits (M1, L1)**: attempts counted atomically before the check (five per pending session),
  ten failures per account per hour persisted, pending sessions expire after five minutes, the used
  step is stored with a conditional update.
- **Resource limits (M2, M3, L5, L8)**: rate-limit buckets are never dropped unless full, each scope
  has its own capacity and a full map refuses new keys; per-route body limits before authentication
  (4 KiB prelogin, 16 KiB register and login); a global concurrency limit (`max_concurrent_requests`,
  default 512); 32 long-polls per account; pending OPAQUE logins capped per IP prefix (16) and evicted
  oldest first.
- **Sessions and devices (L2, L3, L4)**: key uploads and vault creates must carry the session's own
  device ID; a login ends the same device's other sessions and at most 64 sessions per account exist
  (the oldest idle one is evicted); at most 10 000 key generations per vault (`max_key_generations`).
- **Files (L6)**: on Unix the data directory is 0700, the database, WAL, blobs and the secret file are
  0600, and the server refuses a secret file that is group- or world-accessible.
- **Deployment (L7)**: Caddy runs with `cap_drop: ALL`, `NET_BIND_SERVICE`, `no-new-privileges` and a
  read-only root file system.
- **Slowloris (I7)**: axum's `serve` offers no header read timeout; the reverse proxy is the
  load-bearing defence (SERVER.md says so).

## ADR-0045: Sync engine rules

- Status: accepted
- Date: 2026-11-06
- Decided by: the principal engineer, from ADR-0012, ADR-0013 and the M2 review findings
- Milestone: M6 (sync)

### Context
The engine has to close the open M2 findings (no upload queue for vault keys and metadata, lost edits
around rotations, delete races) without a trusted server and over a backend without compare-and-swap.

### Decision
1. **Key and metadata upload queue** (store schema v3): `upload_pending` and `local_origin` on vault keys,
   `remote_created`, `meta_upload_pending` and `meta_server_seq` on vaults.
2. **Two-phase rotation with head CAS**: prepare a key without changing state, upload it conditional on
   the vault head, commit locally only when accepted; `HeadMoved` pulls, pushes and retries with the same
   key.
3. **Per-record base CAS** on upload; **items before tombstones** within a run.
4. **Canonical key winner and "loser rotates again"** in both transports.
5. **Deferral of records ahead of their key**: the cursor never passes a record that needs a generation
   the device does not hold yet.
6. **Attachments**: soft delete on the backend, re-assert by a holder, fetch-missing with a download
   budget.
7. **Checkpoints**: published after every sync; lag and rollback rules of CRYPTO_SPEC section 10.
8. **`reset_sync_state` on every backend connect**: everything is queued again and the backend's content
   is merged back, so a backend wiped or switched loses nothing.

### Alternatives considered
- Server last-writer-wins: loses unmerged versions.
- Accepting lower key generations: breaks CRYPTO_SPEC section 7.1.

### Consequences
Closes M2-R1, R2, R10, R11, R16 and M3-R25 (server part); mitigates M2-R7, R12 and R13 in hosted mode;
leaves R55 for folder mode.

### Amendment, 2026-11-06 (M6 T2 review: ADR-0043 and ADR-0045)
The crypto review and the sync-chaos pass of M6 T2 changed these rules; CRYPTO_SPEC sections 4, 10, 10.1,
11 and 12.1 carry the text.
1. **Derived keys do not leave the session.** `UnlockedSession::sync_keys` and `SyncKeys` are removed.
   The folder transport holds `SessionCrypto<H>` over a shared session handle (`SessionHandle`; the
   desktop's `Arc<Mutex<Option<UnlockedSession>>>` implements it): `K_cp`, `K_fold` and `K_dev` are
   derived per operation inside the session and gone on lock, and a locked session makes every operation
   fail closed (`WireError::Locked`, `TransportError::Locked`, `SyncError::Locked`). `KeyedCrypto` is
   test-only again (`test-util`). The engine still never holds the session across a transport call.
2. **Persisted engine state** (store schema v4, additive): own sequence numbers, halt, strikes, last
   published counter, peer checkpoints, lag timers, keys seen per generation, marker first-seen times. A
   halt ends only through `clear_sync_halt` (a user action); a rollback or a lock cannot reset a strike
   or a timer. Alternative rejected: keeping them in the engine and requiring the app to never rebuild
   it, which an attacker who can make the app restart defeats.
3. **Own sequence numbers (ADR-0043).** `reserved` is stored before a segment is written and `written`
   after; numbering goes above the verified disk state and `reserved`; a log whose highest file is below
   `written` is a rollback and halts the run before it writes (the compaction floor of the first draft
   is not needed: a device never deletes its newest segment). Alternative rejected: checking that every
   number up to `written` exists, which a planted name or a skipped number would turn into a false halt.
4. **Planted names** in the own log are stepped past (16 attempts), never counted for the numbering or
   the checkpoint, and removed after 24 hours.
5. **Snapshot sets** complete from the folder state each time a part arrives, not from memory; a failed
   set's range is reserved so the next set starts after it. The parts of a set are never marked read in
   advance (only the numbers below `snap_base`).
6. **Blobs** are authenticated with the item's file key before they are stored; a holder may overwrite a
   blob that fails authentication (the one exception to write-once, via temp file and rename); purge
   ages from the first-seen time of the marker on this device's clock and waits for no lag, no open key
   conflict and no unexplained object.
7. **Canonical winner** ranks every key, held or candidate, by the lowest device that wrote exactly its
   bytes; the observations are stored.
8. **Checkpoints**: strikes compare the seen own counter with the last published counter (one strike per
   value, persisted); the highest verified checkpoint per other device is kept and a lower one is never
   accepted; a checkpoint entry of a vault without a cursor counts as lag (unless deleted locally).
9. **Hostile folder files**: symlinks and non-regular files are never read, reads are capped, at most 32
   checkpoint candidates per device directory, the device list needs a verified file, and a join tries
   account files in order (at most 8) until one unlocks and verifies.
10. A lost or stolen device keeps `AK` and can forge folder objects until a re-key (M7); THREAT_MODEL R57a.

### Amendment, 2026-11-06, round 2 (M6 T2 re-review)
11. A join refuses a password that a higher-version account file (MAC-verified with the key it opened)
    supersedes. A halted engine does not rotate; the halting checks run before the first write of a
    run or a rotation.
12. The rollback check counts a file at or above `written` only if it verifies or cannot be read now.
13. Names above `reserved` in the own log are removed at once (files that do not verify, directories,
    links); snapshot ranges are checked name by name; a hole in the own numbers makes the housekeeping
    compact. A backend change keeps `reserved` (conservative; the hole is closed by compaction).
14. Files are opened non-blocking on Unix and re-checked as regular files after the open (a new
    `libc` edge for `mosaic-sync` on Unix, already in the lock file).
15. A holder's failed heal is counted in the report and never aborts a run.

## ADR-0046: Network and OPAQUE dependencies

- Status: accepted
- Date: 2026-11-06
- Decided by: the principal engineer, from CRYPTO_SPEC section 2 and ADR-0044
- Milestone: M6 (sync)

### Context
M6 adds an HTTP client and server and the OPAQUE implementation that CRYPTO_SPEC section 9 already
names. Crypto dependencies need an ADR (CLAUDE.md).

### Decision
- `opaque-ke`, latest 4.x, exact `=` pin (Meta-maintained, NCC Group audited), minimal features; the
  ciphersuite of CRYPTO_SPEC section 9.1 (ristretto255, TripleDH over SHA-512, identity KSF).
- `axum`, latest 0.8.x, for the server.
- `reqwest =0.13.5` (already in the tree through Tauri) with `blocking` and rustls using the **ring**
  provider (no aws-lc-rs C build), `min_tls_version(TLS_1_3)` and the OS trust store through
  `rustls-platform-verifier`. If reqwest 0.13.5 has no ring-provider feature, use `rustls-no-provider`
  plus a direct `rustls` dependency with `ring` and `use_preconfigured_tls`.
- `toml =1.1.8` for the server config; `tracing` and `tracing-subscriber` (fmt only) for logs.
- `cargo deny`: a new licence (for example `CDLA-Permissive-2.0` from `webpki-root-certs`) gets a
  crate-scoped exception with a reason; `deny.toml` is not loosened otherwise.

### Alternatives considered
- `ureq`: cannot pin TLS 1.3 as directly.
- `native-tls`: the OpenSSL bindings are banned.

### Consequences
Each crate enters in the track that uses it: T1 (protocol foundation) adds none of the network or OPAQUE
crates; the `opaque` module comes with the server and client tracks. No telemetry crate is added.

### Amendment (2026-11-06, T3: the dependencies as pinned)
- `opaque-ke =4.0.1` is the newest stable 4.x (4.1 is a pre-release). Its SHA-512 comes from `sha2 0.10`
  (the digest 0.10 generation opaque-ke is written against), added as the renamed dependency
  `sha2_0_10`; it was already in the tree. Features: `ristretto255` only (no `argon2`, `std` or
  `serde`); the random number generator is Mosaic's own `OsRng` over `getrandom` (ADR-0011), so no
  `rand` feature is switched on. `wire::opaque` in `mosaic-sync` (feature `opaque`) defines the suite,
  the context and the identifiers once for client and server.
- **RUSTSEC-2026-0387** (`crypto-bigint 0.5.5`, timing variability) enters through `elliptic-curve 0.13`
  (`opaque-ke` and `voprf`). It is ignored in `deny.toml` (and must be in `.cargo/audit.toml`) with this
  reason: the ristretto255 suite does every secret group and scalar operation in `curve25519-dalek` and
  never calls the arithmetic of `crypto-bigint`; no released `opaque-ke` uses `elliptic-curve 0.14`.
  Revisit with every `opaque-ke` release.
- `axum =0.8.9` with `http1`, `tokio`, `matched-path` and `query` only (no `json`: bodies are parsed with
  `mosaic_sync::wire::json`, which rejects unknown keys); `tower-http =0.6.11` with `limit` and
  `timeout` (in the tree through Tauri); `tower =0.5.3` (`util`, tests); `toml =1.1.8` with `parse` and
  `serde`; `tracing =0.1.44` and `tracing-subscriber =0.3.23` (`fmt`, `std`, `ansi`: no environment
  filter, no JSON, no exporters). The server also uses `hkdf`, `hmac`, `sha2` and `chacha20poly1305`
  directly for K_totp, the pre-login fakes, hashes and the sealed TOTP secrets: the CRYPTO_SPEC section 2
  primitives, nothing new. `cargo deny` needed no licence exception.

### Amendment (2026-11-07, M6 T4: the HTTP client as built)
Choices that the spec left open, each the most conservative one:
- **Dependencies (ADR-0046 option 2).** `reqwest =0.13.5` has no ring-provider feature, so the workspace takes
  `reqwest` with `blocking` and `rustls-no-provider` only (no default features: no cookies, redirects are off,
  no compression, no HTTP/2), a direct `rustls =0.23.46` with the `ring` provider and `std` (no `tls12`
  feature of its own; hyper-rustls switches `tls12` on in the tree, and the config below stays TLS 1.3 only
  whatever the features say (M6 T4 review follow-ups); 0.23.46 is the first release without RUSTSEC-2026-0285, -0346, -0348 and -0349, which
  `cargo deny` reports for the 0.23.40 that Tauri's tree had cached) and `rustls-platform-verifier =0.7.0` (the OS trust store: SChannel, Security.framework, the
  platform's certificate stores). `mosaic_sync::http` builds the `ClientConfig` itself and passes it to
  reqwest as the preconfigured backend: `builder_with_provider(ring)`, `with_protocol_versions([TLS13])`,
  `with_platform_verifier()`, no client certificate. A unit test pins the version list to TLS 1.3. `ring`
  (ISC) and `rustls` are new to the lock file. `webpki-root-certs` (CDLA-Permissive-2.0, root certificate
  data) gets a crate-scoped licence exception in `deny.toml`, as ADR-0046 foresaw. It is used only on
  `wasm32` and in the platform verifier's own dev-dependencies, not as a Linux fallback (Linux reads the
  system roots through `rustls-native-certs`; corrected in the T4 review follow-ups); the licence list is not loosened otherwise. The crates are optional
  and only built with the `http` feature of `mosaic-sync`.
- **URL rule.** `canonical_origin`: `https://host[:port]` or `http://` for `127.0.0.1`, `[::1]` and
  `localhost`; lowercase; no path, query, fragment, credentials, percent escapes or non-ASCII host
  (punycode only); port 1 to 65535. `TransportError::InsecureUrl` for plain `http` to another host or another
  scheme, `InvalidUrl` for everything else. The client sends the same origin string the OPAQUE login is bound
  to, and `GET /v1/info` must name exactly that origin (`ServerMismatch`) and protocol 1 (`Incompatible`),
  checked before every login, registration and session.
- **`TransportError` grows** `Rejected(Rejection)` (the hosted codes with no variant of their own:
  `AccountExists`, `RegistrationClosed`, `InvalidInvite`, `InvalidEmail`, `LoginFailed`, `DeviceRevoked`,
  `TwoFactorInvalid`, `BadRequest`, `TooLarge`, `AttachmentExists`, `CheckpointStale`, `TooManyPolls`,
  `DeviceIsCurrent`, `AccountVersionMismatch { account_version }`), `InsecureUrl` and `InvalidUrl`. They
  carry no body, ID or address. `login_failed` stays one answer for a wrong password, an unknown email and a
  server that names another identity (CRYPTO_SPEC §9.2).
- **Responses are strict.** Unknown or missing JSON keys, wrong types, IDs that are not lowercase hyphenated
  non-nil UUIDs and non-canonical base64url are `Protocol`. A success body is read up to its cap (16 MiB JSON;
  the attachment ciphertext maximum) and refused before reading when `Content-Length` says more; an error
  body is cut at 64 KiB. A page of changes must have strictly ascending sequence numbers above the cursor and
  at most the head, a cursor that does not move back, `more` only with records, and every ciphertext must
  decode as its kind; a records response must answer every record in order with its ID. `Retry-After` is
  honoured as whole seconds (1 to 86 400, 60 when absent or in date form).
- **Idempotent retries.** `checkpoint_stale` on the checkpoint upload counts as done (a repeated request; a
  device whose counter fell behind the server's is caught by the engine's check of its own checkpoint, which
  runs before anything is written). A records request is split to fit the server's `maxRequestBytes` (the
  engine batches by ciphertext bytes, and base64 adds a third); the batches of one call are separate
  requests, so a `generation_mismatch` in a later one leaves the earlier ones accepted, which the engine
  resolves by its pull (their echo acknowledges them, ADR-0045 amendment below).
- **Session token.** The bearer header is built from the token and marked sensitive; the token is never
  in an error or a log, `HttpTransport` has no `Debug` output of it. The transport learns the device's
  own install ID (for the checkpoint path) from `GET /v1/devices` (`current`), not from the caller
  (changed: `HttpTransport::connect` takes the engine's install ID, see the T4 review follow-ups).
- **Client functions** (`mosaic_sync::http::hosted`): `prelogin`, `register`, `login`, `login_totp`,
  `logout`, `join_login` (pre-login, the Argon2id derivation and the OPAQUE login) and
  `join_fetch_account` (`GET /v1/account`, whose account ID, salt and profile must equal the keys'),
  `password_change` (fresh login with the old `OPW`, the second factor when the account has one,
  re-registration, one upload guarded by the account version of the session's login; see the T4 review
  follow-ups). The join is two
  functions instead of one because a pending session needs its TOTP code between them.
- **Hosted pull cursor, change counter.** A cursor is the 8-byte big-endian `server_seq` (empty = 0);
  the change token of the long poll is the 8-byte counter (none yet: a value the counter never has, so the
  first call answers at once); a client-side timeout of the poll is "nothing changed".

### Amendment (2026-11-07, M6 T4: engine rules found by the hosted simulations; ADR-0045)
The hosted branches of the engine had never run against a server. Three rules were wrong or missing:
- **(Superseded by the T4 review follow-ups below: the base is the served view.) A received version the
  device already contains moves the upload base forward.** The server accepts a
  change only on top of the version it holds (`baseSeq`). `apply_remote_record` ignored a received record
  that the local record dominates or equals, and left the stored `server_seq` (the next `baseSeq`) behind;
  the pull cursor had passed that version, so the pending change was refused as a conflict on every attempt
  and the devices never converged. Now the ignored record raises the stored sequence number
  (`Store::raise_server_seq`, never lowering), and if its ciphertext is the pending change itself (the
  server stored it and the response was lost, or the request was repeated) the change is acknowledged.
- **Hosted: the server's key is the canonical key.** The canonical-winner rule by lowest device ID is
  right for a folder, where every key reaches every device. The server keeps one key per generation: a
  legacy key (an offline rotation) that lost the race for the slot never reaches another device, so a
  device holding it must rotate again whatever the device IDs say; otherwise it keeps a key nobody else
  has and every record either side seals is unreadable to the other. On the hosted backend any key of the
  current generation that a pull brought in and that differs from the held one makes the held one lose.
  (Superseded: the loser adopts the server's key, see the T4 review follow-ups below.)
- Unchanged by this track and confirmed by the simulations: records are pushed items before tombstones; a
  rotation offers its key on top of the head (`head_moved` pulls, pushes and retries the same key).

### Amendment (2026-11-07, M6 T4: the T3 review changes the client respects)
After the server's review fixes (one session per device, a login ends the device's other sessions, at most
64 sessions per account, pending sessions of five minutes, per-session and per-account TOTP attempts, key
uploads and vault creates only with the session's own device ID, long polls capped per account, per-route
body limits and a global concurrency limit): the client sends the `deviceId` of its own install in every
login, vault creation and key upload (the engine already does); a second login of a device ends the
transport that holds the first session (`Unauthorized`), so the desktop keeps one transport per device and
signs in again after `Unauthorized`; the second-factor code is a `SecretText` in the request. The
simulations log in each device once and sign in again where a test opens a second path for a device.

### Amendment (2026-11-07, M6 T4 review follow-ups: served view, view digest, key winner; ADR-0045)
The crypto and sync-chaos reviews of T4 (findings C-M1, C-M2, C-L1 to C-L4, C-Info, S1 to S10) change these
rules. The normative text is CRYPTO_SPEC §9.4, §10 and §10.2.
- **The base of an upload is the served view, not a maximum.** The store (schema v5) keeps
  `sync_served(vault_id, record_id, seq, ct_hash)`: the latest `server_seq` and the SHA-256 of the ciphertext
  the server served or accepted for each record. Every pulled record (applied, ignored as dominated,
  deferred or unreadable) and every accepted upload overwrites the row. A push is based on the row's `seq`.
  `raise_server_seq` no longer runs on a received record (a forged `seq` on a replayed old version wedged
  the item's uploads for good, S4); an echo of the pending change still acknowledges it. A refused upload
  whose following pull does not change the row makes the device read the vault again from the start, once
  per run; still refused after that, the vault gets a `ServerInconsistent` finding and the change stays
  queued. Stored sequence numbers in `records` stay as a local monotonic mark (folder uses them; the hosted
  base does not).
- **A view digest in every hosted checkpoint entry.** The checkpoint format (version 1, never released)
  gains `view_digest[32]` per entry: `SHA-256(ctx("sync/view") || 0x00 || vault_id || high_seq || n || n x
  (record_id || seq || ct_hash))` over the served rows sorted by record ID. A device publishes it only for a
  clean view (no row beyond its cursor, no open finding) and reads every vault it pushed to once more before
  it publishes. Devices at the same head with the same generation compare digests; a difference is a
  persisted `ServerInconsistent` finding, not a halt. This closes the silent cases that the counters and
  `high_seq` cannot see: a forged head (S5), an acknowledged but dropped upload (S6), a page cut short
  (S7) and a vault rolled back with reused sequence numbers (S8). An all-zero digest means none (folder;
  readers ignore it). The server sees one opaque 32-byte value per vault and checkpoint (CRYPTO_SPEC §12).
  An upload acknowledged under a `seq` at or below the device's own cursor is impossible and is a finding at
  once. A pull page whose head is below the cursor is a persisted `ServerRollback` for that vault only (S9);
  the run no longer fails with `Protocol`, other protocol errors still do. Findings are cleared only by the
  user (`clear_sync_halt`), which also rebuilds the served view and cursor of the flagged vaults and makes
  the device ignore the digests peers published before that moment.
  Schema v5 recreates `sync_peer_checkpoint` for the longer entries: the stored newest checkpoints of other
  devices are lost once and fetched again by the next run (a replayed old checkpoint could be accepted in
  that one run; the MAC and the own-counter rules are unchanged).
- **Hosted key winner (S1, C-M1).** The server's key for a generation is canonical. A device holding a key it
  generated and the server never accepted (an offline rotation that got `generation_taken`) **adopts** the
  server's key (`UnlockedSession::adopt_remote_vault_key`): one store transaction swaps the key row, re-seals
  every record and the metadata it sealed under its own key, queues them and drops its own key; the vault
  cursor is reset and the vault is read again. No generation `g + 1` is made, so the loser neither rotates
  again nor leaves records unreadable to the devices that never saw its key (the winner can be gone). A key
  the server confirmed (an accepted upload; the vault's creation) is never replaced: a second valid key for
  that generation is the server contradicting itself, reported as `ServerInconsistent`, resolved by the rank
  rule of §10.1, with at most three such rotations per vault per 24 hours (persisted). Folder mode is
  unchanged (canonical winner, loser rotates).
- **Cheaper rotations (S10).** A received record of the same version at the current generation replaces a
  queued re-seal of equal content and the queued copy leaves the outbox, so a rotation costs one upload per
  record instead of one per device.
- **Clock jumps (S2, S3).** A backoff retry time further ahead than the longest delay it can set (plus a
  server's `Retry-After`) is dropped; a stored lag start in the future counts from now.
- **Password change (C-M2, C-L3).** The session row stores the account version at KE3; `password/start` and
  `password/finish` compare against it (`PasswordFinishRequest` loses `expectedAccountVersion`), and a
  successful change deletes the account's other sessions in the same transaction. A retry after a lost
  `password/finish` answer logs in with the new `OPW` and, if the account's wrapped AK equals the prepared
  one, returns as success.

### Amendment (2026-11-07, M6 T4 review follow-ups: the HTTP client; ADR-0046)
- **Secrets (C-L1, C-L2).** opaque-ke's `session_key` and `export_key` are wiped right after use in login
  and in both registrations. The bearer value is built in a `Zeroizing<String>` and kept as one sensitive
  `HeaderValue` per `HttpTransport`; the copies `reqwest` and `hyper` make while they send are outside the
  crate's reach (THREAT_MODEL R57). `zeroize` joins `mosaic-sync` behind the `http` feature (already in the
  workspace).
- **Resolution, proxies and roots (C-Info).** `localhost` is pinned to `127.0.0.1` with
  `ClientBuilder::resolve`, so a plain-http loopback connection does not depend on the resolver. `reqwest`
  honours `HTTPS_PROXY`, `ALL_PROXY` and `NO_PROXY` from the environment but not the system proxy
  settings; on Linux `rustls-native-certs` reads `SSL_CERT_FILE` and `SSL_CERT_DIR`, which can add roots.
  Both are accepted: the environment is the user's own (a process that sets it already runs as the user,
  R57a), TLS 1.3 and the OPAQUE binding to the origin still apply, and pinning would break corporate and
  self-hosted setups (THREAT_MODEL R67).
- **One origin normalizer.** `mosaic_sync::wire::origin::canonical_origin` serves the client (the URL it is
  given) and the server (`canonical_origin` of its config): lowercase, the default port removed (443 for
  https, 80 for http loopback), and a port that is not 1 to 65535 in digits only (`0`, `+443` and any other
  character are refused). The server's earlier normalizer accepted `+443`.
- **Own device.** `HttpTransport::connect(base, token, install)` takes the engine's install ID; the
  checkpoint path no longer depends on the server's `current` device flag.
- **Corrected text.** See the notes in the T4 amendment above on `tls12` and on `webpki-root-certs`.

### Amendment (2026-11-07, M6 T4 re-review follow-ups; ADR-0045, ADR-0046)
The crypto re-review of the first follow-ups (R-M1 to R-M4, R-L1 to R-L7, I1 to I5) changes these rules; the
normative text is CRYPTO_SPEC §10 and §10.2.
- **The view digest commits to the key (R-M1).** Adoption could split a vault: a server that answers
  `generation_taken` to two offline rotators and cross-serves their keys makes each adopt the other's, and
  the served views still agree (the same records). The digest input gains
  `key_commit = SHA-256(ctx("sync/view-key") || 0x00 || vault_id || generation || wrapped_key_bytes)` after
  `high_seq`, over the wrapping the device holds for its current generation (the bytes the server served or
  accepted). A key this install generated and the server never accepted publishes the all-zero digest.
  The two formulas moved into `mosaic-core` (`sync_auth::view_digest`, `view_key_commit`; labels
  `sync/view` and `sync/view-key` in the label registry): no literal context string in `mosaic-sync` (I2).
  Checkpoint v1 is still unreleased, so it changed in place again; the KAT script, vectors and fuzz seeds
  follow. A device keeps publishing the real digest while a finding is open (I1): the server must not learn
  that it was caught.
- **More than one candidate key is a contradiction (R-L4).** On hosted, two or more distinct verified keys
  for one generation are a `ServerInconsistent` finding and nothing is adopted.
- **Heads that never meet (R-M2).** Digests compare only at an equal head, so a server that keeps two
  devices' heads one apart avoids them, and the lag rule does not see it either. A device stores, per
  (peer, vault), since when its runs have failed to compare (the first such run starts the count, a
  comparison ends it) and the peer's counter then; a peer whose counter has risen with failures spanning
  48 hours of runs is `NoCommonView` (a single run after a long pause or a clock jump does not count) (persisted, cleared by
  the user). Honest pairs meet at the same head after the settle pull.
- **A login in flight does not survive a password change (R-M3).** The server's pending login records the
  account version at `login/start`; `login/finish` refuses with `login_failed` if it moved, and a password
  change drops the account's pending logins.
- **Acknowledging a finding does not make the loss permanent (R-M4, R-L1, R-L2).** `clear_vault_findings`
  (per vault: findings, `NoCommonView`, the served view, the cursor and the current-generation key
  candidates; the vault is marked for a rebuild) is separate from `clear_sync_halt` (the halt, strikes and
  `written`); `sync_findings` lists both so one UI action clears what is set. After the full re-read of a
  marked vault, every local record at the current generation that the server does not hold as it is and
  that is not queued is queued again. Digests of checkpoints stored before the acknowledgement, or while a
  rebuild is open, are not compared (otherwise a peer's stale digest raised the finding again); the user
  acknowledges on every device.
- **Smaller (R-L3, R-L5, R-L6, R-L7).** The confirmed mark is written in the transaction of the vault's
  creation, of the accepted key upload and of the rotation commit; the v5 migration seeds it for a key this
  install generated with a clear upload flag at the vault's current generation (the conservative reading).
  A vault flagged `ServerRollback` takes no record, metadata or attachment uploads. The v5 migration drops
  hosted cursors (the served view starts empty; the next run reads the vaults again and no digest is
  published from a half-known view). The bearer value is encoded straight into zeroizing buffers and the
  OPAQUE results are `mut` from their creation.
- **Forward constraint (I5).** Adoption must not resolve revocation rotations once sharing (§8, M6b)
  ships; see CRYPTO_SPEC §10.2.
- **Dev note (I3).** Server databases created before the first follow-ups edited migration v1 in place
  (`sessions.account_version`, the checkpoint body limit) must be recreated; the server is unreleased.


## ADR-0047: Sync in the desktop app

- Status: accepted
- Date: 2026-11-07
- Decided by: the implementing engineer, from ADR-0042 to ADR-0046 and the T5 brief
- Milestone: M6 (sync)

### Context
The desktop runs the engine, keeps a session token and takes the folder choice from a native dialog. The brief
leaves the lifetime of the token, the order of the fresh-login flows, the wait loop and the acknowledgement open.

### Decision
- **The token** is kept in the OS keystore (`mosaic-sync-token-<account id>`, base64url, memory fallback), never in
  the vault store or the settings file. `SessionToken` cannot be rebuilt from bytes, so `mosaic-sync::http` has
  `StoredToken` (zeroizing, redacted, not `Clone`) and a `Token` trait that `connect` and the hosted functions
  accept. It is deleted on disconnect, on a 401 and when the device is removed.
- **One session per device (ADR-0044)** means every fresh login of this device replaces its running session. The
  flows that need a fresh login (second-factor setup, device removal, password change) stop the sync thread
  first, store the token the login granted before anything else can fail, and start the thread again from the
  keystore. A device whose second factor is on needs the code before the login, not after (a login without it
  would end the session for nothing). The server accepts a second-factor code once, so removing the second
  factor takes two codes: the first attempt answers `syncTwoFactorRequired` and the next one reuses the fresh
  session for four minutes.
- **Lock** ends the thread by generation: a handle of an older generation fails closed with `Locked`, whatever the
  slot holds; the next unlock builds a new engine from the persisted state. Sync work never touches the idle
  timer.
- **Waiting** is done in slices (five seconds for the server, two for a folder) so that a poked local change is
  seen within the slice; the long poll of the server cannot be interrupted. A halt stops the automatic runs until
  `sync_acknowledge`, which clears the halt and the findings against the server behind one confirm and syncs.
- **The master password change** goes to the server first and to the device afterwards; an unreachable server
  refuses it (`syncPasswordNeedsServer`) as long as `password/finish` has not been sent. After that a timeout, an
  unreachable server or a 5xx is an unknown outcome, not "nothing changed" (milestone review M2): the device
  checks at once with the new password and commits locally if the server's wrapped AK is byte-equal to the prepared
  one; if the check fails too the error is `syncPasswordOutcomeUnknown` with `passwordChanged` set, and
  `sync_recover_password` settles it. The folder writes the new account file first. A password change leaves quick
  unlock removed, as before.
- **The folder path** never reaches the webview as input: a random `choiceId` names it for five minutes (ADR-0034).

### M6 milestone review follow-ups (2026-11-07)
- **Reconnecting a folder** (M3): on a connect and at the start of the folder thread the device compares the highest
  verified account file with its stored parameters. A retired salt, or a version at or below the stored one with
  other parameters, is replaced by the device's own parameters at the highest version plus one; a newer file that is
  not retired sets `passwordChanged` (CRYPTO_SPEC §10.1). The stored folder account version is cleared on every change
  of backend (L1); the retired salts stay and still catch a rollback.
- **Pending logins and locks** (I2): `set_pending_in` rechecks the lock epoch after it sets and undoes the pending
  login if it moved; the second-factor connect path takes the same route.
- **Digest skip and rebuild bound** (M1), **`checkpoint_stale`** (L4) and the label registry (L2): CRYPTO_SPEC §3, §10.
  The published counter does not advance on a stale answer: a forged high counter from a stolen token then costs one
  strike and no halt loop. The price: a device whose own checkpoint was shadowed that way stays stale until it is
  removed and signs in again (R62), and its rollback detection is suppressed meanwhile.

### T5 review follow-ups (2026-11-07)
- **Password change versus lock and crash.** A change in flight (`SyncService::begin_change`) defers a lock for up to
  three minutes (`LOCK_DEFER_CAP`); then the lock goes ahead. The local commit follows the server's answer at once;
  the token is saved after it and a refusing keystore does not undo the change (status `signedOut`). The change the
  server holds and the device missed is recovered with the new password: CRYPTO_SPEC §9.4 "Change applied elsewhere",
  `UnlockedSession::adopt_remote_password` (the join derivation plus a constant-time equality of the account key),
  `hosted::recover_password`, `FolderTransport::verified_account_file`, the command `sync_recover_password` (a wrong
  password counts towards the five-failure lock) and the flag `SyncOverview.passwordChanged` with the error
  `syncPasswordChanged`. The lost-answer check of §9.4 (byte-equal wrapped AK) stays, and now also runs straight after a finish whose
  answer was lost (milestone review M2), not only on a later retry.
- **Older generations.** Every write of the sync thread checks its generation under the lock of the view; the token
  delete of a 401 is a compare-and-delete (constant time) serialized with every token save, and leaves the status alone
  when a newer login replaced the token.
- **Signing in again keeps the sync state** (cursors, checkpoints, key observations, strikes, halt, findings) when the
  server is the same; another server resets it, as before. The UI offers no other server while signed out, so no extra
  confirmation was added.
- **Incomplete logins.** A token is saved only after its session is complete. A login that waits for a code (device
  removal, second-factor setup) is kept as `PendingKind::Fresh` for five minutes and the next call with a code
  finishes it; whether the account has a second factor is read first when unknown. A failed save is an error.
- **Locks and pending state.** A flow records the lock epoch (`SyncService::epoch`, counts locks and resets) and drops
  its result if it changed. Expired pending logins and joins (with their keys and Secret Key), folder choices and the
  fresh-login mark are swept on every sync command and on lock.
- **Bounds.** Server address 2048 bytes, invite code 128, `choiceId` 64, device name 64, second-factor codes 6 to 8
  ASCII digits, checked before any network call. `sync_check_server` works only while unlocked or while the app has no
  account.
- **Native confirmation.** `sync_acknowledge` asks in a native dialog (`Dialogs::confirm`), which the webview cannot
  skip; the webview supplies the texts. The in-app dialog is gone: one button under the list of warnings that need a
  check. The fresh-login reuse (four minutes) applies to `sync_totp_disable` only.

### T5 verify follow-ups (2026-11-07)
- The hosted thread compares the server's wrapped AK and parameters (`GET /v1/account`) with the device's at start; a
  difference sets `passwordChanged` and pauses sync until `sync_recover_password`. A change whose local commit failed
  (the app locked past the cap) keeps the old, revoked token, sets the flag and does not save the new token.
- Retired salts and the folder account version (vault schema v6, public data) make a rollback to an older master
  password refusable: `adopt_remote_password` refuses a retired salt (`syncPasswordRollback`), and a folder file at or
  below the last version this device wrote or adopted. CRYPTO_SPEC §9.4 states the rules and corrects the closing claim.
- A sleep lock is not deferred. Every lock drops pending logins and joins, also in the onboarding; the idle tick sweeps
  expired ones; a result that comes after a lock is not kept (`set_pending_in`).
- The native acknowledgement is OK/Cancel of the system's kind with texts that live in Rust (`sync/strings.rs`, equal to
  the catalog), chosen by the language setting; the request has no fields. New `windows` features `Foundation_Collections`
  and `System_UserProfile` read the user's first preferred language through the safe WinRT API (`GetUserDefaultLocaleName`
  is unsafe, and the crate forbids unsafe code); Linux and macOS read `LC_ALL`, `LC_MESSAGES`, `LANG`.

## ADR-0048: M6b scope: sharing and emergency access on the hosted server

- Status: accepted
- Date: 2026-11-08
- Decided by: the principal engineer, from the M6b spec and ADR-0004, ADR-0042
- Milestone: M6b (sharing and emergency access)

### Context
ADR-0042 deferred sharing, organisations, team vaults and emergency access to M6b. M6 ships two transports (the
hosted server and a cloud folder). CRYPTO_SPEC §8 existed as a design sketch. THREAT_MODEL R3, R7, R18 and R70 point
at this milestone.

### Decision
1. **Hosted only.** Vault sharing and emergency access exist on the hosted server and nowhere else. A folder store
   cannot share, receive shares, or grant or receive emergency access, and never holds identity keys, rosters or the
   contact book. Reasons (CRYPTO_SPEC §8.6): (1) every folder integrity key derives from one account's AK, so a second
   account cannot verify or write a segment; (2) there is no compare-and-swap (R55), so in a revocation the canonical
   winner rule can pick a concurrent rotation by the member being removed; (3) nothing can stop a removed member
   writing, and members and admins of a team folder keep write access (R54); (4) there is no directory or mailbox for
   keys and seals, and planted files would make trust on first use worthless; (5) nothing can enforce a waiting period,
   and a pre-wrapped escrow is just a share. Owner decision OD1 may revisit this with new folder formats. `s/` stays
   reserved.
2. **Item sharing is moving an item into a shared vault** (CRYPTO_SPEC §7.2). There are no per-item ACLs. A "team" is
   a shared vault with several members.
3. **Emergency access covers vault keys only** (content type `0x02`). AK is never sealed: content type `0x01` is
   reserved and decoders reject it (OD2). Only vaults the grantor owns, and not hardware-protected (TK) vaults.
4. **Verification is required** before any invite, grant or accepted invite (a one-click attestation after comparing
   safety codes; OD3 may relax it for invites). Opening an emergency release from an unverified grantor is allowed
   after a warning, because that can only inject fake vault keys.
5. **R70 is closed in M6b with `sync/view2`.** The hosted view digest also commits to the vault metadata and the new
   roster through `meta_commit`. M6 is unreleased, so the checkpoint version stays 1 and the label `sync/view` is
   retired (it stays in the registry; `view_digest` stays in `mosaic-core` until the engine moved to `view_digest2`).
6. **Safety code display** is `5-5-5-5-6` groups, because 26 characters do not split into groups of 5. The QR payload
   `mosaic-contact:1:<account_id hex32>:<fpr hex64>` is defined for the mobile app and not used on the desktop, which
   has no camera.
7. Out of scope: organisations, admin policies, ownership transfer and groups (OD6); full re-key and identity key
   rotation (M7); per-author item signatures (R7, v2).

### Alternatives considered
- Folder sharing with per-vault folder keys: a new spec chapter and still no revocation guarantee (R54).
- Emergency sealing of AK: a release would hand over the whole account, including future vaults and sessions, with
  one server-enforced wait; vault keys keep the blast radius to the vaults named.
- Trust on first use without verification: the server could substitute a key at first contact unnoticed (R71).

### Consequences
THREAT_MODEL R71 to R81 are added; R3, R7, R18 and R70 are updated. The folder backend is unchanged.

## ADR-0049: The `hpke` dependency and the HPKE random number generator

- Status: accepted (amends ADR-0004 vetting and ADR-0011)
- Date: 2026-11-08
- Decided by: the principal engineer; vetted by the T1 implementer from the crate source and registry metadata
- Milestone: M6b

### Context
ADR-0004 chose HPKE (RFC 9180) mode Auth and said the `hpke` crate needs vetting in M6. CRYPTO_SPEC §2 fixes the suite:
DHKEM(X25519, HKDF-SHA256) (`0x0020`), HKDF-SHA256 (`0x0001`), ChaCha20-Poly1305 (`0x0003`). ADR-0011 keeps `rand`
and `rand_core` out of Mosaic's own code.

### Decision
1. **Crate and pin.** `hpke = "=0.15.1"` (rozbb/rust-hpke, MIT or Apache-2.0, published 2026-10-28 and the latest
   release when pinned, 16 million downloads; checksum
   `8f512e325a1a71bf3273fb9b7fc9e5d96dac136fb4a95d0e6637001b12ea3e09`). The M6b spec text named `=0.13.0` before
   the latest release was checked; 0.13 is built on `rand_core` 0.9 and `generic-array`, the previous RustCrypto
   generation, while 0.15 uses `aead` 0.6, `chacha20poly1305` 0.11, `hkdf` 0.13, `sha2` 0.11 and `rand_core` 0.10,
   the same versions as the rest of the tree. 0.15 also serializes X25519 private keys clamped and pulls in the
   `crypto-bigint` and `ctutils` fixes (RUSTSEC-2026-0387 and -0385), which only the unused NIST-curve features reach.
2. **Features.** `default-features = false, features = ["alloc", "x25519", "chacha"]`. That drops `getrandom`,
   `mlkem`, `aes`, `nistp` and `shake`, so the build contains exactly the §2 suite: X25519 (`x25519-dalek` 3 and
   `curve25519-dalek` 5, BSD-3-Clause, already allowed), HKDF-SHA256, ChaCha20-Poly1305. The crate's own source
   contains no `unsafe`. No other KEM, KDF or AEAD is compiled in.
3. **Only mode Auth, only single-shot.** The code calls `single_shot_seal_with_rng` and `single_shot_open` with
   `OpModeS::Auth` and `OpModeR::Auth`, never Base, PSK or the streaming contexts.
4. **Randomness without an infallible adapter.** `hpke` takes a `rand_core::CryptoRng`, whose methods can't fail. The
   spec draft proposed an `HpkeRng` that aborts the process when the CSPRNG fails. Instead, `mosaic_core::rng::HpkeRng`
   serves exactly the 32 bytes of one ephemeral `ikm`, drawn beforehand with `rng::fill` (the usual
   `Error::Randomness`), from a buffer. `hpke` draws exactly 32 bytes per seal (`gen_keypair_with_rng` reads the `ikm`
   of one X25519 key). If a future version ever asked for more, the shortfall is zero bytes plus a flag and the seal
   fails with `Error::Randomness`; a seal is never made from weak bytes and the process never aborts. The buffer is
   zeroized on drop, and tests inject a fixed `ikm` through the same type, so KATs need no extra code path. `rand_core`
   is therefore a transitive dependency only; Mosaic's code still calls `getrandom` directly (ADR-0011).
5. **Private keys.** `isk` is stored as the 32 clamped bytes `hpke` serializes, in Mosaic's own zeroizing container;
   an `hpke` private key exists only for the duration of one operation.
6. **Known-answer tests.** The unit test `share::kat::rfc9180_a23_auth` reproduces RFC 9180 A.2.3 (the Auth vector of
   exactly this suite) through the crate, and `crates/core/tests/vectors/share_kat.py`, a stdlib-only implementation of
   X25519, ChaCha20-Poly1305 and HPKE Auth, reproduces the same vector and the Mosaic seals.
7. `deny.toml` needs no exception: all new crates are permissive (MIT, Apache-2.0, BSD-3-Clause).
8. **Facts about the crate's assurance** (T1 review L3). The crate has had no paid audit. The only outside review we
   know of is Cloudflare's internal review of 0.8. 0.15.0 is new and changed the X25519 private-key serialization (the
   clamped form, item 5). The release date matches the changelog (2026-10-27 for 0.15.0, 2026-10-28 for 0.15.1). The
   guard against a silent behaviour change is the RFC 9180 A.2.3 known-answer test (item 6), which runs on every build.

### Alternatives considered
- `=0.13.0` as the spec draft said: older RustCrypto generation, a second `rand_core` and `generic-array` in the tree.
- Hand-written HPKE on `x25519-dalek`: invented crypto, forbidden.
- An aborting `fill_bytes` (the spec draft): a process abort inside a password manager on a CSPRNG failure; the buffered
  draw returns an error to the caller instead.

### Consequences
Adds `hpke`, `x25519-dalek`, `curve25519-dalek`, `hybrid-array` and their small dependencies. CRYPTO_SPEC §2 names the
crate, its pin and features.

## ADR-0050: Roster object, merge rule and acceptance rule

- Status: accepted
- Date: 2026-11-08
- Decided by: the principal engineer
- Milestone: M6b

### Context
A shared vault needs a member list that every member can check. Roles can be enforced by the server only (R72), but the
rotation chain after a revocation must not depend on the server's word.

### Decision
1. **The roster** is a replicated document (CRYPTO_SPEC §7.5 layout) sealed under VK as `VaultRosterCiphertext`, exactly
   the `VaultMeta` construction with the label `vault-roster`. Keys `member:<account_id hex32>`, values
   `{"email","fpr","role","since"}` with roles `owner`, `manage`, `write`, `read`, `removed`.
2. **Merge rule.** The document merge decides what is kept. The effective value of a member register is the entry with
   the highest `since`, then the most restrictive role (`removed` > `read` > `write` > `manage` > `owner`), then the
   largest stamp. Member registers never show as conflicts. A removal writes `removed` with `since = g + 1`, never a key
   deletion, so a concurrent role change at `g` can't resurrect a removed member and a re-invite (`since` = the current
   generation) beats the removal. The effective view is a function of the merged document, so the merge stays
   commutative, associative and idempotent (property test).
3. **Validity** on every open: values parse, `fpr` is 32 bytes, `since` at most the roster's generation, exactly one
   `owner` and it is the server's `owner_account_id`. An invalid roster is the finding `RosterInvalid`; the vault stays
   readable and no rotation share is accepted for it.
4. **Acceptance rule (amended after the T1 crypto review).** Roles stay server-enforced. The one cryptographic
   boundary is the claim rule (CRYPTO_SPEC §8.3): a rotation share is accepted only from the vault **owner**, the
   account fixed at accept, whose served `ipk` matches the verified contact pin (`VaultRoster::authorizes_rotation`).
   No roster data can grant rotation authority: every member who holds an old key can write a roster entry at the old
   generation, so a roster-based "manager may rotate" rule would let a removed manager (or any member promoting
   themself) mint the next key. Managers still invite (an invite carries the current key and makes no generation).
   Limit: a revocation waits for the owner's device; while the owner is offline the removed member keeps the key.
   Rosters are bound to their generation (merging two generations is an error), the owner's entry is fixed at
   creation (later writes to it are ignored), and `validate` checks every entry of every register.
5. The server keeps the current roster and the last roster of each generation, and lets only `owner` and `manage` write
   it by compare-and-swap.

### Alternatives considered
- A manager may rotate when the previous roster lists them: rejected (above).
- Largest stamp wins: a stale role change could beat a removal.
- Per-member signatures: a v2 candidate (R7).

### Consequences
R72 and R79 record the limits. The roster joins the view digest (`meta_commit`, R70).

## ADR-0051: Shared-vault rotation, claims and the `shared` flag

- Status: accepted (amends ADR-0013 and ADR-0045 for shared vaults)
- Date: 2026-11-08
- Decided by: the principal engineer
- Milestone: M6b

### Context
M6 resolves concurrent rotations by adoption: the server's key for a generation replaces the device's own, because that
key was never shared. For a shared vault the server's key for a generation could be one a removed member can read, and
the CRYPTO_SPEC §10.2 forward constraint said such rotations need their own rule.

### Decision
1. **Triggers.** A removal by a manager, a leave, an invite that was declined or cancelled after delivery (OD4), and a
   server-listed member the roster lacks (after a user prompt). They persist as pending revocations until accepted.
2. **One atomic rotate request.** The new key row, the shares for every remaining member and pending invite, the new
   roster, the membership removals, the generation and the head bump go in one request, conditional on the current
   generation and head. The server refuses it (`members_mismatch`) unless the shares plus the revoked accounts cover the
   other members and the pending invites.
3. **No adoption.** After `generation_taken` or `head_moved` the device pulls, accepts the winner's next generation only
   through the claim rule, and prepares again at the next generation. At most three attempts per vault per run.
4. **Attachments** are re-sealed under fresh IDs and file keys in the same store transaction as the key swap (ADR-0017);
   old blobs are deleted only after the referencing records are accepted. Unfetchable ones are listed (R76).
5. **The `shared` flag** is monotone and outlives the vault and backend changes, like the generation floor. Adoption is
   forbidden for a vault that has it; a backend change is refused while any shared vault is live. A backend switch
   therefore can't make a shared key adoptable (the second §10.2 constraint).
6. Unshared vaults keep the M6 path; the server refuses it for a vault with several members or a pending invite.

### Alternatives considered
- Adoption with a membership filter: still lets the server pick the key for the generation.
- A multi-request rotation: a crash between requests leaves some members without the new key.

### Consequences
CRYPTO_SPEC §8.3 and §10.2 state the rule. R75 and R76 record what remains.

## ADR-0052: Directory and invite abuse controls

- Status: accepted
- Date: 2026-11-08
- Decided by: the principal engineer
- Milestone: M6b

### Context
Inviting by email address needs a directory. A directory that answers only for registered accounts is an enumeration
oracle, and invites can be used for spam.

### Decision
1. **Fakes.** The lookup answers every valid email with `(account_id, ipk)`. For an unknown address the account ID is
   the pre-login fake (the same HMAC, so the pre-login and directory answers agree) and the `ipk` is
   `HMAC-SHA-256(K_dir, ctx("server/directory-ipk") || normalized_email)` with the top bit cleared, where
   `K_dir = HKDF(server_secret, "", ctx("server/directory-key"), 32)`. A registered account without a published key gets
   its real ID and the fake `ipk`. Invites to fakes are stored like real ones and expire.
2. **Limits.** Lookups per session and per account, invites per account, 50 members per vault, 200 pending outgoing and
   100 pending incoming invites per account (extra incoming invites are dropped silently), 20 grants given and received.
3. **Blocks.** A recipient can decline with a block; blocked senders' invites vanish silently.
4. **No email.** Invites go to the account address plus a safety-code comparison; clients pull their inbox. No mail is
   sent. There is no message text, only the vault name inside the ciphertext. Logs never hold lookup targets.

### Alternatives considered
- Answer only for existing accounts: enumeration for free.
- Email invitations: a new outbound channel and a new secret in the server.

### Consequences
R74 and R77 record the residuals (timing, unaccepted invites). The two labels live in `mosaic_core::encoding::label`.

## ADR-0053: Emergency access state machine and limits

- Status: accepted
- Date: 2026-11-08
- Decided by: the principal engineer
- Milestone: M6b

### Context
Brief §emergency access asks for a waiting period the grantor can refuse. Without email, the server can't notify a
grantor.

### Decision
1. **States** `idle`, `waiting`, `approved`, `released`; transitions as in CRYPTO_SPEC §8.4, enforced by the server. A
   decline returns to `idle` with a 24 hour cooldown; approve needs a fresh login; `released` is final until the grantor
   deletes the grant, which also queues revocations of the emergency memberships.
2. **Waits** of 1, 2, 7, 14 or 30 days (default 7), the grantor's choice, fixed while `waiting` (OD5).
3. **Notification** through the server's inbox counter, which wakes the long poll; the desktop shows a persistent banner
   with Refuse and Approve now. The grant dialog tells the grantor to open the app within the wait.
4. **Scope** all owned vaults (kept fresh automatically) or selected ones; one `EmergencySeal` per vault; the server
   adds a `read` membership with origin `emergency` at release.
5. **Not recovery.** Account recovery stays with the Emergency Kit.

### Alternatives considered
- Immediate release with notification only: no chance to refuse.
- Sealing AK (OD2): hands over the whole account.

### Consequences
R73 records that the wait and the refusal depend on the server and the grantor's attention.

## ADR-0054: The contact book as a synced, AK-derived AEAD object

- Status: accepted
- Date: 2026-11-08
- Decided by: the principal engineer
- Milestone: M6b

### Context
Pins must follow the user across devices, or a new device would pin again and a server could substitute a key there.
Storing pins as plaintext would show whom the user talks to.

### Decision
The contact book is an account-level replicated document (§7.5 layout) sealed under `K_contacts = HKDF(AK, "",
ctx("contacts-key"), 32)` as `ContactsCiphertext` (label `contacts`, bound to the account ID, at most 256 KiB padded)
and synced through the server with compare-and-swap and a merge. `K_contacts` is derived per operation and never
stored. Entries `contact:<account_id hex32>` hold email, first seen, `fpr` and `verified`. Concurrent pins of different
fingerprints leave two values in a register, which is `Conflicting` and blocks like a key change; concurrent pins of the
same fingerprint agree and the verified flag wins.

### Alternatives considered
- Pins in the local store only: every new device would start from trust on first use.
- Plaintext pins on the server: the server would learn the contact list's keys.

### Consequences
R80 records that a server can withhold the newest book from a new device; the roster still catches it for vault members.

## ADR-0055: M6b milestone review follow-ups

- Status: accepted
- Date: 2026-11-08
- Decided by: the principal engineer
- Milestone: M6b

### Context
The milestone review of M6b (four High, four Medium, four Low, one Info finding) found gaps between CRYPTO_SPEC §8 and
the code. Every fix below is accepted as the reviewer proposed it, with the deviations named.

### Decision
1. **H1, known vaults.** An invite or an emergency release never hands a key to a vault ID the device knows (open
   vault, generation floor, sharing row); the one exception is an `access_lost` row of the same owner. Preview, accept
   and commit check it; a release skips such a seal (a finding unless the vault is the grantor's already).
2. **H2, verified contacts only.** A rotation seals only to contacts with a verified pin. Roster members and invitees
   that are not verified are unconfirmed: the rotation revokes them (the owner's rotation is already running, and
   blocking it would leave the removed member holding the key), records `MemberUnverified`, and seals nothing to them.
   Deviation from "revoked after a prompt": the prompt applies to server-listed unconfirmed members, which never trigger
   a rotation themselves.
3. **H3, never-shared vaults.** A grant gives each covered vault the owner-only roster and the sharing row.
   `DELETE /v1/emergency/grants/{grantee}` deletes the origin-2 memberships, keys and shares, and the grantor's client
   queues the revocations first and rotates. A device without a sharing row accepts a first roster only for owner = self
   or a verified-pinned owner with the roster's fingerprint (deviation: the proposed "owner = self only" would break the
   second device of an invitee, which has no row yet), and the engine refuses an owner that is an emergency grantee.
4. **H4, a manager's Remove.** `DELETE /v1/vaults/{v}/members/{a}` (manager or owner; target role at most the caller's;
   never the owner or the caller). The owner finds the roster member missing from the server and rotates.
5. **M1.** The accept carries the generation; `invite_stale` unless it equals the invite's and the current one.
6. **M2.** The directory's fake `ipk` is `DeriveKeyPair(HMAC(K_dir, ctx || email)).public`
   (`IdentityPublicKey::derived_from`), a valid prime-order-subgroup key.
7. **M3.** The pull ignores a record under a reserved row ID (`FF..FF`, `FE..FE`) and does not note it in the served
   view. The reserved rows stay in the served-rows table (the alternative of own columns was not needed).
8. **M4.** The claim rule needs a verified owner pin; accepting an invite from a non-owner sender needs the owner's pin
   verified too (the preview reports `owner_code` and `owner_state`).
9. **L1.** An unresolvable invitee loses its invite instead of blocking the revocation. **L2.** `MemberDropped` is a
   finding on the owner's device; the rotation still leaves a dropped member out of the key (a leave, a removal and a
   server drop look the same). **L3.** `put_grant` answers 204 for an unknown account and stores nothing; the directory's
   by-ID lookup no longer serves the recipient of an invite someone else sent. **L4.** Opened invite keys are swept
   each run.
10. **I1.** CRYPTO_SPEC §12 lists `vault_removals`, `inbox_seq`, the audit kinds 7 to 14 and the invite `hidden` flag.
11. **Clean-up.** `sync_auth::view_digest` (the retired digest) is removed; the label `sync/view` stays registered.
    The sharing finding kind 5 (`MemberUnverified`) joins the unreleased store schema v8 in place.

### Alternatives considered
- Blocking a rotation on unverified roster members: keeps the removed member in.
- Own columns for the served metadata and roster: more schema for no extra protection once the pull ignores the IDs.

### Consequences
THREAT_MODEL R71, R72, R77, R79 are updated and R82 is added.

Amended by ADR-0056: the rule that a first roster may name a verified contact as owner is replaced by positive owner authorization from the contact book, and a key mismatch no longer blocks a rotation.

## ADR-0056: M6b verify follow-ups (positive owner authorization, grant-delete rotation)

### Context
The verify passes on ADR-0055 found: a second device of the grantor trusts the server's grant list when it sees a first roster for a vault it never saw shared (High), and a check that fails open when the `owned:` entry is missing (the server can freeze the contact book); a vault accepted on another device defaulting to owner = this account; deleting a grant rotated only what the server reported; a key mismatch of one member stopped the whole revocation rotation.

### Decision
- Ownership is positive evidence in the contact book (sealed under `K_contacts`): `owned:<vault>` on creation and before a vault is first shared, `member:<vault>` = owner at accept (invite or emergency release), `emergency:<grantee>` = the grant's cover before its seals. There is no default owner for a vault without a sharing row. A first roster is accepted only from the owner an entry names; a vault with no entry is held (finding `VaultHeld`, retried on later syncs). The verified-contact exception and the engine's grant-list check no longer decide. On upgrade, only vaults whose first key this device generated are marked owned; everything else stays unknown until an entry arrives.
- `owned:` is never written for a vault with a `member:` entry; `owned_vaults` and every owner-only path use `owns_vault`, which needs a positive entry or a sharing row naming this account.
- The owner roster, the sharing row, the set entry and the grant cover are written before the seals and `put_grant`; the contact book is uploaded and nothing may stay pending, or the grant stops with a retryable conflict.
- Deleting a grant queues a revocation and rotates exactly the vaults its recorded cover names (all positively owned vaults if there is no record), ignoring the server's state, scope and list.
- A new member vault and its sharing row are stored in one store transaction.
- A remaining member with a key mismatch, a member the directory does not serve, or an invitee with a Changed or Conflicting pin is revoked or cancelled and recorded as `MemberKeyMismatch` (schema v8 gained the kind in place, as it is unreleased; so did `VaultHeld`). The rotation goes ahead. The session still refuses to seal to a mismatching key.

### Consequences
A malicious server can lock a member out by serving a wrong key (it could already drop memberships); the finding tells the owner, and the two kinds say whether it looks like a man in the middle. It can also keep a second device from using a vault by withholding the contact book; the finding says so. THREAT_MODEL R72 and R82 are updated.

## ADR-0057: M7 scope: key rotation, credential rotation is M7b

- Status: accepted
- Date: 2026-11-08

### Context
The brief's M7 ("Rotation") means credential rotation: the semi-automatic change-password flow, connectors, a scheduler, HIBP and a rotate-now queue. ADR-0042 item 3 and THREAT_MODEL R57a assign key rotation (full re-key, Secret Key rotation) to M7 as well. The owner's brief for this work is key rotation.

### Decision
- M7 covers key rotation only: master password change screen, Secret Key rotation, KDF profile upgrade, vault key rotation UI, and the full re-key on both transports.
- Credential rotation (the brief's M7) becomes M7b with its own spec and runs next, before M8 (they share no code, so the owner may swap them).
- ADR-0042's non-key items (settings as a synced record, move pointers, the flag on an unreadable copy) are retargeted to M9.
- Rotating the OPAQUE server setup or the server secret (R61) stays in M9. CLI rotation commands are not part of M7 (M8 only recovers).

### Alternatives considered
- One milestone for both kinds of rotation: too large to review, and the two share no code.

### Consequences
Nothing of the brief is dropped; it moves to M7b. Store schema v9 and server migration v3 belong to M7.

## ADR-0058: Account-key epochs and re-key adoption

- Status: accepted
- Date: 2026-11-08

### Context
A full re-key gives the account a new AK. Other devices and backends need a public way to tell which AK a password wrapping belongs to, and a way for a device to move to the new AK that an attacker holding the old password, Secret Key and AK cannot abuse.

### Decision
- `ak_epoch: u32`, starting at 1, raised by exactly one per re-key. It is public metadata (account row, server account record, folder account file) and never an input to a key derivation: different epochs have independent random AKs. A store trigger refuses a decrease.
- Backend epoch below the device's: rollback, refused (hosted; in a folder lower files are ignored). Equal epoch with a wrapping that opens to a different AK: refused. Higher epoch: only adoption moves the device, and only `current + 1` (checked arithmetic).
- Adoption requires epoch `current + 1` (hosted; on a folder any higher epoch, ADR-0063), an unretired salt, a wrapping that opens under the typed password and typed Secret Key, and a typed Secret Key that differs from the current one **and every retired commitment** (constant time). Retired commitments are `SHA-256(ctx("secret-key/retired") || 0x00 || account_id || SK)` for every Secret Key the device replaced (label `secret-key/retired`). There is no adoption at an equal epoch (amendment, review M1): concurrent re-keys are resolved by re-keying again to `max(seen) + 1`.
- On adoption the server's wrapping of each member vault's current generation is stored (after a constant-time check that it opens to the same VK), so the view digest does not change (review M3). Every genuine re-key rotates the Secret Key, so a holder of only the old secrets cannot invent an AK and have a device adopt it.
- No endorsement by the old AK (a signed hand-over): the old AK is what a compromise exposes, so a signature from it proves nothing. The new kit's Secret Key typed by the user is the proof.
- Identity rollover notices signed by the old `isk` are not built, for the same reason (OD-M7-4): members re-verify the owner's new safety code.

### Alternatives considered
- The old AK wraps or signs the new one: useless against the attacker the re-key is for.
- Automatic adoption of any higher epoch: lets a malicious backend or old-secret holder move the device.

### Consequences
A user who lost the new kit can't adopt on another device (the kit is the recovery path). THREAT_MODEL R83 and R84. No new primitive: the epoch is a counter, adoption uses the existing wrapping.

## ADR-0059: Hosted re-key as one atomic request

- Status: accepted
- Date: 2026-11-08

### Context
A hosted re-key changes the OPAQUE record, wrapped AK, identity, key rows of every owned and member vault, shares, rosters, seals and sessions. Applying it in pieces would leave the account half-moved.

### Decision
- Two calls under a fresh login with the current `OPW`: `rekey/start` (OPAQUE registration request with the target epoch `current + 1`) and `rekey/finish`, which the server applies in one transaction or not at all.
- Coverage rules: the vaults in the request are exactly the set the account owns, each at generation `current + 1` with the expected head (`generation_taken`, `head_moved`: nothing applied); for an owned vault with members or pending invites the M6b rotate rules apply (seals from the new `isk` for every member, invite seals for every pending invite; the request revokes nobody); member re-wraps are exactly the vaults where the account is a non-owner member at the current generation; emergency seals cover every grant the account gave.
- The request is the only place an identity key is replaced and the only one that replaces a key row at a generation a member holds. Side effects: older-generation key rows of the account deleted, key shares and emergency seals addressed to it deleted, incoming pending invites cancelled, a rotation request filed per member vault, every other session ended, audit kinds 15 and 16.
- The profile may not be weaker than the stored one (`profile_downgrade`). Lost answer: a login with the new `OPW` shows the wrapped AK and epoch, which the device compares byte for byte.
- Core adds `change_credentials` (one prepare path for password, Secret Key and profile changes), `rekey_account`, and the folder re-key MAC (`sync/folder-rekey`, under the existing `K_fold`). No new primitive.

### Alternatives considered
- Per-vault re-key calls: a crash leaves vaults at different AKs.
- Letting the server rotate member vaults: the server holds no keys; only owners rotate (ADR-0051).

### Consequences
Members get a rotation request, which is advisory (THREAT_MODEL R86). The request body is capped (4 MiB, 1000 vaults).

## ADR-0060: Folder re-key: account file v2, marker, pre-re-key segment retention, no automatic winner

- Status: accepted
- Date: 2026-11-08

### Context
A folder has no authority: no compare-and-swap, no sessions. Other devices of the old epoch must notice a re-key, and the re-keying device's own old segments no longer verify under the new `K_fold`.

### Decision
- Account file version 2 (172 bytes) adds `ak_epoch:u32`; version 1 decodes as epoch 1. A re-key marker `.mrek` (93 bytes, no key material) is MAC'd under the **old** `K_fold` with the new label `sync/folder-rekey`, built during prepare because the commit drops the old AK.
- A stale device halts before any write only on a **verified** marker (`AccountRekeyed`); an unverifiable account file with a higher epoch is a dismissible finding (review M2); it recovers with the new Secret Key and the password (ADR-0058). A marker verifies under the old `K_fold`, so a holder of the old AK can forge one: it halts devices, and after a failed adoption and a confirmation the user dismisses it (ADR-0063).
- New-epoch readers ignore everything that does not verify under the current epoch's keys and never accept an old-epoch object.
- Own segments at or below the re-key floor are kept until a complete new-epoch snapshot set exists and 30 days (`BLOB_RETENTION_MS`) have passed, so stale devices can still read them. Closes the M6 known item on old-`K_fold` own segments.
- Two concurrent re-keys to the same epoch raise a `RekeyConflict` finding on the device that sees the other; the user keeps theirs or re-keys again to `max(seen) + 1` (no equal-epoch adoption). No automatic winner: choosing an AK nobody typed would make that kit useless.

### Alternatives considered
- Delete old segments at once: stale devices lose data before they adopt.
- Pick the lowest device ID as winner: the losing side's kit becomes useless without the user knowing.

### Consequences
THREAT_MODEL R83 and R87. The sync KAT script gains the version 2 account file and the marker; a `rekey_marker` fuzz target follows with the sync track.

## ADR-0061: Rotation journal, pending Secret Key in the keystore, wizard and resumption

- Status: accepted
- Date: 2026-11-08

### Context
A re-key spans a backend request, a local transaction and many attachment and upload steps. A crash must leave the user able to continue, and a Secret Key nobody has saved must never become the account's key.

### Decision
- A one-row journal in the store records kind, phase (1 prepared, 2 backend started, 3 backend accepted, 4 folder written, 5 local committed, 6 attachments, 7 uploading), epochs and counts. It holds no secret. The row is written in the transaction that enters each phase.
- The new Secret Key lives in the OS keystore under a pending name from phase 1 until commit or adoption moves it to the normal entry; abandoning a job before the backend has seen it deletes it. The kit is saved and verified before the backend learns the change.
- After phase 3 the prepared keys died with the process: the device adopts its own re-key with the pending Secret Key and the typed new password.
- A change of backend is refused while a job exists. Each attachment re-seal is its own transaction.
- The desktop uses one wizard (why, current password, new password, kit, review, run, done) and a resume banner after unlock.

### Alternatives considered
- Keeping the prepared keys in the keystore for resume: more secret material at rest for no gain.

### Consequences
THREAT_MODEL R88 (same exposure as the Secret Key, R21).

## ADR-0062: KDF profile policy

- Status: accepted
- Date: 2026-11-08

### Context
CRYPTO_SPEC §5.3 lets a client upgrade the profile at an unlock. A stronger profile slows weaker devices, and a backend must not be able to force a change.

### Decision
- Profiles only rise (a weaker one is refused before any Argon2id work). An upgrade is the password-change flow with a stronger profile and a new salt.
- Offers never go above `standard` automatically; `strong` and `max` only by explicit choice, with a slow-device warning and a guard when this device's derivation exceeds 2 s (the request may force it).
- A device that has not seen an upgrade recovers silently at its next password unlock with the typed password, the stored Secret Key and the backend's parameters.

### Alternatives considered
- Forced upgrades by policy: a backend or a bad policy could lock slow devices out.

### Consequences
THREAT_MODEL R89.

## ADR-0063: M7 milestone review follow-ups (folder epoch jumps, joins, holds)

- Status: accepted
- Date: 2026-11-08

### Context
The milestone review of the key rotation backend (3 high, 5 medium, 5 low findings) showed that a planted account-file header could strand other devices on a folder, that a device joining after a re-key started at epoch 1, and that an adopted device could seal at the retired generation.

### Decision
- **Folder adoption takes any higher epoch** (ADR-0058 amended). A re-key targets `max(own, highest seen) + 1`, so the epoch can jump; the typed Secret Key is the authenticator (the file must open under it, its MAC must verify under the key it holds, and the Secret Key is neither the current one nor a retired one). Hosted adoption stays at `current + 1`. The marker's `to_epoch` is any value above `from_epoch`. `highest_epoch_seen` counts verified markers and unverified files at exactly `own + 1`, and ignores dismissed ones.
- **A marker the device cannot follow is dismissible.** A marker verifies under the old `K_fold`, which a thief of an old device holds. After a failed adoption and a confirmation the user dismisses it (path and SHA-256, like a dismissed file); the pending flag clears when no other undismissed verified marker is left (ADR-0060 amended, THREAT_MODEL R92).
- **Candidates are tried by salt, marker-named files first, with paging.** One derivation per salt, eight salts per attempt, `skip` to page on up to the 64-file scan cap.
- **A device that joins stores the backend's epoch** (folder: the verified file's; hosted: the served `akEpoch`), only rising; the hosted re-key re-reads the epoch after its fresh login and stops unless it is the one it leaves; each hosted run compares it with the own epoch (`AccountRollback` finding for a lower one).
- **Hosted adoption needs no clean sync** and skips unknown or unavailable member vaults. After an adoption every owned vault is held at the generation the device has: no push until the next generation's key arrives (table `rekey_hold` in schema v9, which is unreleased, so edited in place).
- A resumed job that adopted its own re-key queues the attachments of every owned vault and continues at phase 6. The phase-2 journal write failing stops the request before it is sent. An attachment that does not open is listed as unreachable and skipped. A committed job can be abandoned after a confirmation.
- Invite state 5 (cancelled by the recipient's re-key) reads as `Cancelled` on the sender's side. A member vault whose current key the device does not hold is skipped in a re-key. A strict plan records no finding. The identity epoch is tied to `account.ak_epoch` in the v9 triggers.
- Not done: the reason-6 (`MemberRekeyed`) revocation row. The rotation request reaches the owner as the server's `rotationRequested` flag, which names no requester, and the flag stays until the owner rotates, so the rotation survives a restart without a row.

### Alternatives considered
- Adopt only `current + 1` on a folder and refuse jumps: any planted `own + 5` header could then strand every other device after one genuine re-key.
- Let a marker clear itself after a timeout: it would let a thief's marker expire a genuine one too.

### Consequences
THREAT_MODEL R92 to R94; ADR-0058 and ADR-0060 amended above. The app asks before dismissing a marker and before abandoning a committed job, and calls `join_account_with_keys_at` with the served epoch when it joins a hosted account.
