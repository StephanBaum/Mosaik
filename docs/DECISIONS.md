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

- Status: proposed
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

- Status: proposed
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

### Alternatives considered
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
The store's API takes no plaintext-derived state at all. The sync layer
(M6) decrypts the stored record to compare versions, which costs one AEAD
open per pulled record. Leaving and re-joining a vault keeps its floor, so
an owner who really resets a vault must give it a new `vault_id`. Re-keys
must list every held key, which the store can enumerate
(`vault_keys`). Users who purge data locally lose rollback protection for
those records, which is why only tombstones are used for user deletions.

