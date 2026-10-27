# Progress

Resume from here in every session. Requirements: `docs/BRIEF.md`. Decisions:
`docs/DECISIONS.md`.

## Status

| Milestone | Status |
|---|---|
| M0 Foundations | ✅ done (2026-10-25) |
| M1 Crypto core | ✅ done (2026-10-26, `c43d93c`, CI fix `51ba5b2`) |
| M2 Vault model + local store | 🔄 tracks merged, session layer integrated, four milestone review rounds fixed and gated locally (fuzzed in Docker); integration not yet committed, CI not yet run |
| M3 Desktop app | — |
| M4 Browser extension | — |
| M5 Passkeys | — |
| M6 Sync server | — |
| M7 Rotation | — |
| M8 CLI | — |
| M9 Hardening | — |
| M10 Mobile | — |

## M0 Foundations — done

- Cargo workspace (`crates/*`, Rust 1.99.1 pinned in `rust-toolchain.toml`,
  workspace lints: `unsafe_code = forbid`, clippy pedantic plus
  no-unwrap/panic/print in library code, release `panic = "abort"`).
  The crates are scaffolds that state the milestone they land in.
- pnpm workspace with `packages/design-tokens` (WCAG contrast tests for both
  themes) and `packages/i18n` (DE/EN catalogs, key and ICU-argument parity
  tests).
- Supply chain: `deny.toml` (advisories, licenses, bans, sources), lockfiles,
  dependency lifecycle scripts disabled, Dependabot, GitHub Actions pinned by
  SHA with read-only permissions.
- CI (`.github/workflows/ci.yml`): Rust fmt/clippy/test and TS
  lint/typecheck/test on Linux, Windows and macOS. `cargo deny`, `cargo audit`
  and `pnpm audit`.
- Docs: `THREAT_MODEL.md`, `CRYPTO_SPEC.md`, `UX_GUIDELINES.md`,
  `DECISIONS.md` (template plus ADR-0001…0006), `CLAUDE.md`, README.
- Subagents in `.claude/agents/`: crypto-reviewer, extension-attacker,
  sync-chaos, ux-reviewer, test-engineer.
- Design review by crypto-reviewer: 3 high, 7 medium, 3 low findings, all
  addressed in CRYPTO_SPEC (key generations against rotation rollback, an
  honest description of rotation plus a full re-key flow, profile-based
  Argon2 parameters with enumeration-resistant pre-login, header-bound AAD,
  per-platform quick unlock, 130-bit safety codes, OPAQUE re-registration
  authorization). Residual items are THREAT_MODEL R7–R10.

Gate run locally on Windows: all green. CI runs on GitHub
(`StephanBaum/Mosaik`).

## M1 Crypto core — plan

Scope (CRYPTO_SPEC §3–§7, §14):
1. `mosaic-core::secret`: zeroizing secret types with redacted `Debug` and no
   `Clone`.
2. `mosaic-core::encoding`: `ctx()` / `aad()` builders, envelope header,
   padding.
3. `mosaic-core::kdf`: NFC preparation, Argon2id profiles (table, calibration,
   calibration), HKDF derivations (`S`, `R`, `KEK`, `OPW`).
4. `mosaic-core::secret_key`: generation, `M1-` Crockford format with
   checksum, parsing with aliases.
5. `mosaic-core::aead`: XChaCha20-Poly1305 envelopes for wrapped keys, items
   and vault metadata. STREAM for attachments.
6. `mosaic-core::keys`: AK/VK generation, wrap and unwrap, password change.
7. Tests: KATs (RFC 9106, XChaCha draft, RFC 5869), proptest round-trip and
   tamper tests, golden files.
8. `crates/core/fuzz`: decoders for envelope, secret key and wrapped key.
9. CI: add `cargo llvm-cov` (≥ 90 % on core) and a bounded fuzz smoke job.
10. Review: crypto-reviewer and test-engineer, then fix or accept findings.

New dependencies to justify in the commit: `argon2`, `hkdf`, `sha2`,
`chacha20poly1305`, `zeroize`, `secrecy`, `subtle`, `getrandom`/`rand_core`,
`unicode-normalization`, `thiserror`, and (dev) `proptest`, `hex-literal`.

Risks: Argon2 calibration flakiness in CI (calibration takes an injected
clock, and tests use fixed parameters). Windows/macOS differences in
`getrandom`: none expected.

## M1 Crypto core — implementation state (2026-10-25)

Steps 1–10 of the plan are implemented, and the step 10 review findings are
fixed or accepted (see below). Committed as `c43d93c` (CI fix `51ba5b2`).

- `crates/core/src/`: `error` (one `thiserror` enum; every authentication or
  padding failure is `Error::Decryption`), `rng`, `secret` (zeroizing
  containers and typed keys AK, VK, KEK/W, OPW, FK, SK), `ids` (UUIDv7, nil
  rejected, `KeyGeneration`), `encoding` (`ctx`, `Aad` builder with header
  binding, strict ISO/IEC 7816-4 pad/unpad), `kdf` (NFC, the §5.3 profile
  table, `stretch` with a fallibly reserved and zeroized Argon2 buffer,
  injectable calibration plus a real-clock adapter, the §5.4 HKDF chain),
  `secret_key` (`M1-` Crockford format, checksum, aliases, strict pad bits,
  last-group check), `aead` (crate-private XChaCha20-Poly1305, random
  nonces only), `envelope` (`WrappedKey`, `ItemCiphertext`,
  `VaultMetaCiphertext`, `TombstoneCiphertext`), `attachment` (STREAM-BE32
  via `aead-stream`, 64 KiB chunks, 50 MiB limit), `keys` (AK/VK generation
  and wrapping, `create_account`, `unlock_with_password`,
  `change_master_password`).
- Tests: KATs for RFC 9106 Argon2id, draft-irtf-cfrg-xchacha A.3.1 and
  RFC 5869 cases 1–3. Proptests for round trips, single-bit tampering and
  wrong context for every object, pad/unpad, and Secret Key round trip and
  typos (exhaustive single-substitution detection ≥ 99 %). Golden files in
  `crates/core/tests/golden/` (regenerate with `MOSAIC_BLESS=1`, review the
  diff). A password-change test shows that item ciphertexts are byte-for-byte
  unchanged. Only `tests/baseline_profile.rs` runs a real profile; everything
  else uses 64 KiB test parameters. The 50 MiB attachment round trip is
  `#[ignore]` (slow in debug builds); run it with
  `cargo test --release -p mosaic-core -- --ignored`.
- Coverage (`cargo llvm-cov -p mosaic-core`): **98.5 % lines** (97.0 %
  regions) after the step 10 fixes. The number includes the in-file
  unit-test modules.
- Fuzzing: `crates/core/fuzz` (its own workspace, excluded from the main one)
  has the targets `secret_key_parse`, `wrapped_key` (decode and open),
  `item_ciphertext` (item, vault metadata, tombstone), `attachment_open` and
  `unpad`. Each one also round-trips sealed data. Native Windows MSVC does
  not work: the VS 2019 ASan runtime does not match nightly's LLVM
  (`STATUS_DLL_INIT_FAILED`), and `-s none` fails to link the sancov
  symbols. The 60 s runs used Docker (`rustlang/rust:nightly`, Linux,
  ASan). Every target ran without a crash: `secret_key_parse` 6.7 M
  executions, `wrapped_key` 0.79 M, `item_ciphertext` 0.28 M,
  `attachment_open` 0.35 M, `unpad` 2.6 M. Keep the corpus off Windows bind
  mounts, which cut throughput by about 1000×. The runs found one
  performance issue: every attachment open reserved and zeroized a full
  64 KiB chunk buffer, even for tiny inputs. The buffer is now sized to the
  data. After the step 10 rework (fixed golden keys, tamper and structured
  modes, committed seeds) every target ran 60 s in the same Docker setup
  without a crash: `secret_key_parse` 2.2 M executions, `wrapped_key`
  0.47 M, `item_ciphertext` 0.58 M, `attachment_open` 65 k (with
  `-max_len=270000` and multi-chunk structured inputs) and `unpad` 3.5 M.
  Run with `cargo +nightly fuzz run <t> corpus/<t> seeds/<t>`; the repo's
  `rust-toolchain.toml` otherwise selects stable.
- CI: new `coverage` job (fails below 90 % lines on mosaic-core) and
  `fuzz-smoke` job (pinned nightly, every target from `cargo fuzz list` for
  60 s, seeded).
- Spec clarifications plus ADR-0007 (accepted with amendments in the M1
  review, 2026-10-26): size limits (item 1 MiB padded, vault metadata 64 KiB padded,
  tombstone 64 KiB), the tombstone header, the AAD header of attachments and
  their canonical chunking, strict unpadding, the error mapping
  (`Decryption` in core, `ItemCorrupted` in M2), how calibration reads the
  thresholds, sequential Argon2 lanes, Secret Key prefix rules, and the
  zeroize features.
- Dependency changes: M0 had no Rust dependencies, so the whole
  `[workspace.dependencies]` table is new in M1 (about 50 packages in
  `Cargo.lock`, mostly RustCrypto internals). Direct dependencies of
  `mosaic-core`, each pinned with `=` and with default features off:
  `argon2 =0.6.0` (Argon2id, §5.3), `hkdf =0.13.0` and `sha2 =0.11.1`
  (HKDF-SHA-256, checksums, §5.2–§5.4), `chacha20poly1305 =0.11.0`
  (XChaCha20-Poly1305, §6–§7), `aead-stream =0.6.0` (STREAM, which §7.3
  names; `aead` 0.6 no longer has `aead::stream`), `zeroize =1.9.1` (§4),
  `subtle =2.6.1` (constant-time compare and select), `getrandom =0.4.3`
  (OS CSPRNG), `unicode-normalization =0.1.26` (NFC, §5.1) and
  `thiserror =2.0.21` (derives the error enum; build-time proc macro).
  `chacha20poly1305` and `sha2` enable `zeroize`. Dev-only: `proptest
  =1.12.0`, `hex-literal =1.1.0`. Fuzz-only (own lockfile, never linked into
  Mosaic): `libfuzzer-sys =0.4.10`. The M1 plan also listed `secrecy` and
  `getrandom`/`rand_core`: `secrecy` was dropped because `secret` already
  provides the containers (zeroize on drop, no `Clone`, redacted `Debug`,
  constant-time equality, boxed fixed-size keys), and a direct `rand_core`
  was dropped because `getrandom` is used directly. The full justification
  is ADR-0007 item 14 and goes into the M1 commit message. Crypto crates
  build at `opt-level = 3` in dev and test profiles.
- Known gaps: attachments are sealed and opened in memory only (no streaming
  API yet). Argon2 runs without the `parallel` feature, so the 4 lanes run
  sequentially (ADR-0007 item 7). NFC normalization leaves short-lived copies
  inside `unicode-normalization` we cannot zeroize. Our HKDF code zeroizes
  the PRK it receives; `hmac`'s key-pad block and `hkdf`'s last output block
  stay on the stack upstream (THREAT_MODEL R11). Checking
  `key_generation >= highest accepted` per vault is client state and belongs
  to M2. Identity keys, HPKE, OPAQUE and quick unlock are later milestones.

### M1 review (step 10), 2026-10-26

crypto-reviewer, test-engineer and a gate review reported 2 high, 7 medium
and 11 low findings. All were fixed with regression tests, except the ones
in the table below:

- Crypto: a password change or Secret Key rotation now takes the current
  `PasswordParams` and refuses a weaker profile (`Error::ProfileDowngrade`,
  §5.3). The HKDF PRK is extracted and zeroized explicitly. Secret Key
  symbol mapping is branch-free and lookup-free (`subtle` selects and range
  arithmetic), checked against a table-based reference for every value and
  for every character up to U+07FF. `Aad` and `Label` are crate-private and
  labels are checked for NUL at compile time (no `debug_assert`).
- Tests: authentic-but-badly-padded item and vault-metadata bodies fail with
  `Decryption` (verified by mutating the error mapping). STREAM output is
  compared chunk by chunk with an independent per-chunk XChaCha20-Poly1305
  construction (`attachment::reference`), golden attachments open through
  it, and `golden/attachment_multi.hex` pins two multi-chunk files. Every
  tamper and wrong-context check asserts the exact error. Cross-type
  decoding (item, metadata, tombstone) now really parses and fails with
  `Decryption`. The decoder property test draws valid headers and boundary
  lengths, and a coverage test shows every decoder reaches `open`. Strict
  decoding covers meta and tombstone headers, sub-minimum and misaligned
  bodies and the exact size limit. NFC has a proptest (equivalence,
  idempotence, no reallocation) plus worst-case expansions. The baseline
  profile has a vector computed independently with argon2-cffi and a
  stdlib HKDF (`tests/vectors/derive_keys_baseline.py`, which also
  reproduces `golden/derived_keys.hex`).
- Fuzzing: fixed (golden) keys, tamper modes on authentic golden objects,
  a structured multi-chunk attachment mode, a `matches_last_group`
  reference check, committed seeds in `crates/core/fuzz/seeds/`, and CI
  running every target from `cargo fuzz list` with the seeds and
  `-max_len=270000` for attachments.
- CI and supply chain: pinned `cargo-llvm-cov@0.9.1`, `cargo-fuzz@0.13.2`,
  `cargo-audit@0.22.2` and `nightly-2026-10-25`; `cargo deny` and
  `cargo audit` also run on the fuzz lockfile (NCSA allowed for
  `libfuzzer-sys` only, ADR-0007 item 10).

Follow-up review round (crypto, tests, gate), 2026-10-26: 3 medium and 6
low findings, all fixed with regression tests, each checked by mutating
the code it guards:

- Unpadding of item and vault-metadata plaintext is constant-time in the
  contents (`subtle` selects over the whole buffer, one final branch) and
  is compared against the old branching version at every marker position
  and on random inputs (CRYPTO_SPEC §7.2, ADR-0007 item 4).
- Every public seal path (item, wrapped AK, wrapped VK, vault metadata,
  tombstone, attachment nonce prefix) is checked for a fresh, non-zero
  nonce. Before, only items were, so a constant nonce in the others would
  have passed every test.
- Size limits are tested at exactly the limit and one above it for
  tombstone seal and decode and vault-metadata seal. A maximum-size
  attachment ciphertext is checked cheaply in every build, and CI now
  runs `cargo test --release -p mosaic-core -- --ignored` (the 50 MiB
  round trip).
- The Argon2 memory reservation is a separate function whose failure path
  (`Error::Allocation`) is tested. Buffers that must not reallocate
  (Secret Key display and last group, attachment and AEAD plaintext) are
  checked by capacity, through `#[cfg(test)]` `capacity()` accessors.
- Secret Key prefix leniency (`ml`, `MI`, `M-1`, ...) is pinned, along
  with the rejected prefixes `M2`, `X1`, `N1` and `MO`.
- ADR-0007 is accepted (with the two amendments listed in its status).
- The fuzz crate builds only with `--cfg fuzzing`. A plain `cargo check`
  now stops with a `compile_error!` that explains this, instead of only
  E0599. `cargo fuzz build` and CI are unaffected.

## M2 Vault model + local store — state (2026-10-26)

Four tracks were built in parallel worktrees, each reviewed and fixed, then
merged into `main` with `--no-ff`: `ecc082c` model, `a18972d` store,
`1135d46` generator, `6bd2174` totp. ADR numbers were settled at the
merges: totp ADR-0008, model ADR-0009, store ADR-0010 (the track had
written 0009). The integration step (session layer, docs, CI) sits on top
of the merges and is **not committed yet**. M2 is done once the integration
is committed after the milestone reviews (crypto-reviewer, sync-chaos,
test-engineer) and CI is green.

What M2 contains:

- `mosaic-core::model`: every item type of brief §5 (login, passkey, TOTP,
  note, card, identity, SSH key, API key, Wi-Fi, document), custom fields,
  tags, favorites, URLs, attachments, password history (32 entries),
  trash; vault metadata; tombstones. Replicated documents with version
  vectors and per-field multi-value registers; `merge` is a pure join and
  `merge_and_repair` is the store contract (ADR-0009, CRYPTO_SPEC §7.5 and
  §10). Strict canonical JSON plaintext, schema_version 1.
- `mosaic-vault::storage`: offline-first SQLite store (bundled SQLite,
  STRICT tables, triggers repeating the §7.1 rules), ciphertext only,
  outbox, attachments as files, generation floors, TK wrappings, atomic
  §6.6 re-key, WAL scrubbing (ADR-0010).
- `mosaic-generator`: random passwords, EFF passphrases, Apple
  `passwordrules` and the vendored quirks file, entropy and strength
  (ADR-0011).
- `mosaic-totp`: HOTP, TOTP, Steam Guard, `otpauth://` import and export,
  constant-time verification with a replay guard (ADR-0008).
- Integration (uncommitted): `mosaic-vault::session` and
  `mosaic-vault::otp` (ADR-0012):
  - `LockedSession` / `UnlockedSession`: create account (Emergency Kit data:
    account ID and Secret Key display), join on a new device, unlock with
    master password and Secret Key, lock (drops and zeroizes AK, every VK,
    the decrypted items and the search index), change master password,
    device-ID rotation.
  - Vaults: create, list, rename (encrypted metadata), several per account,
    key rotation with monotonic generations.
  - Items: add, update (password changes go to the history), get, list
    with filters, in-memory search over title, user name, email, URLs and
    tags, favorite, trash and restore, delete forever (tombstone), move
    between vaults (re-encrypted under the target VK, attachments re-sealed).
  - Attachments: add, get, remove; 50 MiB per file and 1 GiB per account,
    checked before encryption.
  - `apply_remote_record` (version-vector rollback rejection, repairing
    merge, exact key generation), `apply_remote_vault_key`,
    `apply_remote_vault_meta`, `pending_uploads`, `mark_uploaded`. The
    network sync is M6.
  - `otp_code` (current code of an item; HOTP advances its counter first)
    and conversions between the model's `OtpConfig` and `mosaic-totp`.
  - The store gained a `local_device` table in schema v1 (still
    unreleased, so no migration), holding the device ID. Schema v1 is final
    with the integration commit; later changes are migrations (ADR-0010).
- Integration tests in `crates/vault/tests/`: `session.rs` (26 end-to-end
  tests), `session_no_plaintext.rs` (scans the database, WAL and attachment
  files for a marker, the master password and the Secret Key while open and
  after close), `sync_sim.rs` (three devices editing two vaults offline,
  records and rotated keys delivered shuffled with duplicates and replays;
  rounds mix field edits, trash, deletes, key rotations, moves between the
  vaults and attachments: convergence after every round, no lost password
  or tag, a delete or move loses against a concurrent write and wins
  otherwise, every stored record and the metadata at the accepted
  generation, equal attachment usage, persisted state equals memory; 3
  seeds × 6 rounds in the normal run, 60 seeds × 12 rounds in the
  `--ignored` release run), plus session unit tests (corrupt and damaged
  rows, unavailable vaults, interrupted re-seals, the generation rule,
  search ranking).
- Integration finding, fixed with a regression test
  (`a_moved_item_reaches_a_device_that_edited_it`): the device-ID reuse
  check rejected the copy of a moved item on every device that had edited
  the item, because the copy keeps the source's version vector. The check
  now runs only for records the device holds (ADR-0012 item 5,
  CRYPTO_SPEC §10, THREAT_MODEL R13).
- Docs: `docs/CORE_API.md` (frozen API for M3 to M6 and its change policy),
  CRYPTO_SPEC §2 and §13 (generator randomness without `rand`), §7.4 (trash
  is plaintext only, hard delete is a tombstone), §7.5 (new: item plaintext
  encoding), §10 (conflict model, applying received records, reuse check).
  ADR-0008 (with its dependency item) and ADR-0009 accepted, ADR-0011
  (generator) and ADR-0012 (session) added. THREAT_MODEL R13, R14 and the
  M2 review log.
- CI: the release `--ignored` run covers the whole workspace; coverage gate
  (90 % lines) for mosaic-core, mosaic-totp, mosaic-generator and
  mosaic-vault; fuzz smoke and `cargo deny` / `cargo audit` for all four
  fuzz crates (the new vault target `apply_remote` runs there through
  `cargo fuzz list`). Actions stay pinned by SHA.
- Dependencies added in M2 (justified in the track commits): `serde
  =1.0.229`, `serde_json =1.0.151`, `base64ct =1.8.3` (model),
  `rusqlite =0.40.2` (bundled) and dev `tempfile =3.27.0` (store),
  `hmac =0.13.0` and `sha1 =0.11.0` (totp, ADR-0008 item 5). The
  integration adds no third-party dependency (`mosaic-vault` now depends on
  `mosaic-totp` and `zeroize`, both already in the workspace). The review
  fixes add only a dev-dependency edge: `mosaic-generator` uses the
  workspace's `sha2` in a test (known-answer hashes of the vendored data).
- Also in the working tree, not part of the session work: a visual
  direction change in `packages/design-tokens` (zinc neutrals, near-black
  primary action inverted in dark mode, DM Sans / IBM Plex Mono, radii
  6/12/14), its test and `docs/UX_GUIDELINES.md` §2, plus the untracked
  mockups in `docs/design/`. They belong in a separate `style(design-tokens)`
  / `docs(design)` commit with a ux-reviewer sign-off. The review fix for
  them is in the token test only: the accent fill is checked at 3:1 against
  every background again (WCAG 1.4.11), so UX_GUIDELINES' "UI components
  ≥ 3:1, tested in CI" holds; a fill that cannot meet it needs an outline
  token that is tested instead.

Gate after the integration review fixes (Windows, 2026-10-26; superseded
by the second round below: the `--ignored` run it lists had not completed):
`cargo fmt
--check`, `clippy --workspace --all-targets -D warnings`, `cargo test
--workspace` (388 passed, 4 ignored), `cargo test --release --workspace --
--ignored` (the 50 MiB attachment round trip, the baseline Argon2 profiles
and the 60-seed sync simulation; RELEASE_IGNORED), `cargo deny check` for the
workspace and all four fuzz crates, `cargo audit --deny warnings` for all
five lockfiles, `pnpm install --frozen-lockfile`, `pnpm lint` (0
warnings), `pnpm typecheck`, `pnpm test` (105 passed: 99 before the review
fix, which adds the six accent checks) and `pnpm audit`: all green. The
vault fuzz lockfile was refreshed (it predated the model's `serde` /
`serde_json` and now also has `mosaic-totp`).

Coverage (`cargo llvm-cov -p <crate>`, lines; all above the 90 % gate):

| Crate | Lines | Regions |
|---|---|---|
| mosaic-core | 98.73 % | 97.20 % |
| mosaic-totp | 99.85 % | 99.48 % |
| mosaic-generator | 99.16 % | 98.70 % |
| mosaic-vault | 97.61 % | 92.72 % |

Fuzz smoke in Docker (`rustlang/rust:nightly`, Linux, ASan, seeds and
dictionaries as in CI), 60 s per target, no crash: core `attachment_open`
34 k runs, `item_ciphertext` 94 k, `item_merge` 1.7 k, `item_plaintext`
31 k, `secret_key_parse` 1.1 M, `unpad` 1.2 M, `wrapped_key` 0.29 M; vault
`attachment_validate` 0.47 M, `record_body` 0.27 M; totp `base32_secret`
2.0 M, `otpauth_uri` 2.3 M; generator `password_rules` 0.17 M,
`site_lookup` 4.2 M. The runs shared the CPU with the gate, so the slow
targets (`item_merge`, `item_plaintext`) got few executions; give them a
longer run in the M2 review. The new `apply_remote` target ran FUZZ_APPLY
in the same Docker setup without a crash.

### M2 integration review, 2026-10-26

crypto-reviewer, sync-chaos, test-engineer and a gate review reported 2
high (one issue, found by two reviewers), 11 medium and 17 low findings.
Fixed with regression tests (each checked against the old code), except
the accepted risks in the table below:

- Crypto: two devices that rotate one vault concurrently no longer replace
  each other's key. A received key of a held generation is compared in
  constant time (the same key is a no-op, another one is
  `Error::VaultKeyConflict`), and the store refuses any other wrapping of a
  held generation (ADR-0013, CRYPTO_SPEC §7.1; the M6 server serializes
  rotations). Accepting a newer generation re-seals and queues every
  readable record and the metadata, not only queued changes, and unlock
  finishes an interrupted re-seal (ADR-0012 item 7). A colliding stamp of
  the device's own ID is `DeviceIdReused` even when the counters are equal
  (ADR-0009 amendment, CRYPTO_SPEC §10). The search index lowercases into
  a buffer of the exact final size (no unwiped reallocation), and
  `VaultSummary`'s `Debug` no longer prints vault names.
- Sync model: an edit supersedes every conflict entry with the shown value
  (no spurious conflict after two devices set the same value), and the
  password history is trimmed by edit order, never by wall clock, and never
  drops the value the edit just moved there (ADR-0009 amendments).
- Session robustness: a row that fails the store's own checks is listed
  and replaced by the next valid version instead of failing every apply;
  a damaged key or metadata row makes the vault unavailable or nameless
  instead of failing the unlock.
- OTP: the model's secret limit is 128 bytes like `mosaic-totp` (was 512),
  so no stored configuration fails to produce codes; `verify` returning the
  largest matching step is pinned with a precomputed repeated code; the
  HOTP doc says "never twice on one device" (two offline devices share a
  code, tested).
- Tests: exact error variants instead of `is_err()` / `Error::Core(_)`
  (AAD binding, rollback, foreign vault, empty password, in-memory
  attachments, OTP ranges, model validation), the Equal-and-equal branch
  with and without a pending row, pending corrupt rows, the attachment
  quota (exact boundary) and the add-attachment rollback, damaged rows at
  unlock, a failed load returning the locked session, a key for a
  hardware-protected vault, SHA-256 known answers for the vendored
  wordlist and quirks file, cross-type and wrong-context opens in the model
  tests, the extended 3-device simulation, and the fuzz target
  `apply_remote` (session-level sync entry points, mutated valid records,
  concurrent rotations; `session::fuzzing` builds sessions without
  Argon2id under `cfg(fuzzing)` only).
- Gate: the leftover workflow worktrees and their merged branches and a
  Git Bash crash dump were removed, and `.claude/worktrees/` and
  `*.stackdump` are ignored. CORE_API now lists the ciphertext types'
  methods per type and the public items it had missed (`SecretText`, the
  `uri` module, `OtpKind::DEFAULT_TOTP`/`STEAM`, the `Digits`/`Period`
  constants, `validate_attachment_ciphertext`, `MAX_UNLOCK_PARAMS_LEN`,
  `LATEST_VERSION`, the `envelope` constants). The CRYPTO_SPEC §2
  randomness row and ADR-0011, already fixed in the tree, land with the
  integration commit, before any further code.

### M2 integration review, second round, 2026-10-27

crypto-reviewer, sync-chaos, test-engineer and a gate review reported 2
high, 4 medium and 14 low findings (some found by two reviewers). The
previous gate paragraph above claimed a green `--ignored` release run that
had not completed (its log was empty and the simulation and the model had
changed after it); it is superseded by the gate below. Fixed with
regression tests, each checked against the old code, except the accepted
risks in the table:

- High (sync): two processes could unlock one store and lose an edit
  without a trace (shared device ID, stale caches, upserts without a
  compare). `Store::open` now takes an exclusive OS lock on the sidecar
  `<path>-lock` (std `File::try_lock`, no new dependency) and a second
  open fails at once with `storage::Error::StoreInUse` (ADR-0010 item 10,
  CRYPTO_SPEC §10, THREAT_MODEL R13). Tests: `a_store_is_open_only_once`,
  `two_sessions_on_one_store_cannot_lose_an_edit`. The per-upsert compare
  as defence in depth is left out (ADR-0010 alternatives).
- High (gate): `three_devices_converge_many_seeds` failed on every run.
  Replayed seed 229651, round 9: device 0 trashed and restored the item
  while device 1 changed its password and deleted it. Device 0's net edit
  only removed what it had written, so the delete winning is correct
  (ADR-0009 item 5); the oracle was wrong. The simulation now logs trash,
  restore and attachment adds and removes as their own ops, counts a write
  as concurrent only if the same device did not undo it later in the
  round, prints the ops of a failing item, and reports the seed and round
  of any failure. `regression_seed_229651` runs in the normal suite.
- Medium (crypto, tests): a vault made unavailable by a damaged, missing
  or non-opening current key row could never recover. `apply_remote_vault_key`
  now repairs it with a wrapping that opens: the same generation replaces
  the unreadable row (`rewrap_vault_key`, or `accept_vault_key` for a
  missing row), a newer one is accepted, then the vault is loaded and
  re-sealed. A readable row is never replaced, a lower generation is a
  rollback, a hardware-protected vault stays unavailable; unlock reads the
  current row alone if another key row is damaged (ADR-0012 amendment,
  CRYPTO_SPEC §7.1). Tests: `a_damaged_current_key_is_repaired_by_a_wrapping_that_opens`,
  `a_peer_rotation_repairs_a_vault_whose_key_row_is_damaged`,
  `repairing_a_damaged_key_keeps_the_generation_rule`,
  `a_repair_never_replaces_a_readable_key`.
- Medium (sync, c1): the delete-race repair restored only whole missing
  fields, so a conflict copy that an editor kept on purpose was lost when
  its field survived through the editor's write. The repair now restores
  every dropped entry of a field the received side does not hold and
  writes the shown value again on top (ADR-0009 amendment, CRYPTO_SPEC
  §10). Tests: `c1_conflict_copy_lost_in_delete_race`; the model proptest's
  repairing sync now also asserts that no value of such a field is lost
  and the shown value does not change.
- Medium (sync, t4): accepted for M6 as M2-R12 (below), with the
  characterization test `a_record_in_flight_during_a_rotation_is_refused_by_peers`.
- Medium (gate): this paragraph; M2 stays open until the integration is
  committed and CI is green.
- Low (crypto, sync, tests): vault metadata used the pure join. It now
  merges with the new `VaultMeta::merge_and_repair` (reuse check, both
  names of a shared stamp kept, `stamp_collision` reported truthfully;
  ADR-0009 and ADR-0012 amendments). Tests:
  `merge_and_repair_keeps_both_names_of_a_shared_stamp`,
  `one_rename_each_under_a_shared_device_id_keeps_both_names`.
- Low (tests): renaming a vault without readable metadata wrote under this
  device's ID with a restarted counter, so the old metadata was then
  silently dominated. It now writes under a fresh one-off writer ID, and
  the extended `damaged_key_or_metadata_rows_degrade_instead_of_failing_the_unlock`
  checks the reload, the generation and that both names survive.
- Low (crypto): a rolled-back wrapped key reached callers as
  `Error::Store(KeyGenerationRollback)`. `apply_remote_vault_key` and
  `rotate_vault_key` now map the store's rollback and conflict to the
  session variants (`store_refusals_of_a_rotation_are_session_errors`; the
  session test, the simulation and the fuzz target match the session
  variant).
- Low (tests): the uncovered branches. The "merge equals the received
  record but reports a collision" branch is reachable (equal values with
  different write times) and now tested
  (`a_same_value_stamp_collision_is_stored_as_received_and_reported`); the
  store's `VaultKeyConflict` mapping is covered through `rotate_vault_key`;
  `change_master_password` against another account key is
  `a_password_check_against_another_account_key_fails`.
- Low (tests): the device-ID schema test matches the constraint violation,
  covers 15- and 17-byte blobs, and a wrong-length row (written with
  `ignore_check_constraints`) reads as `Corrupt`.
- Low (tests): the password-history proptest
  (`password_history_trim_by_edit_order`: 2 or 3 replicas, up to 80
  changes each, clocks skewed by up to 20 days either way, repairing
  syncs; at most 32 entries, the replaced value kept, exactly the oldest
  entries by stamp trimmed, nothing lost by a sync).
- Low (tests): the `apply_remote` fuzz oracle asserts that a delivery with
  changed authenticated content (bit flip, other record or vault ID,
  other kind) is refused with `ItemCorrupted` for that record,
  `UnsupportedVersion`, a generation error, `UnknownVault` or
  `VaultUnavailable`, and checks at
  the end of each input that the target holds every producer item
  (exactly, or containing it for items the target edited) unless the
  devices hold different keys. Mutated IDs are derived from the input;
  the remaining nondeterminism (OS randomness for keys, IDs and nonces) is
  documented in the target and accepted as M2-R14.
- Low (gate): CORE_API lists the missing public items (calibration helpers
  and profile costs stable; `PreparedPassword` / `StretchedPassword`,
  `AccountSalt`'s methods and the attachment format constants restricted;
  `OtpSecret::meets_rfc4226_minimum`, `CharacterClass::contains` /
  `is_unicode`, `Strength::from_bits`; the round wrote `Entropy::from_bits`,
  corrected in the third round), plus `VaultMeta::merge_and_repair` and
  `StoreInUse`.
- Low (gate): the totp and vault fuzz lockfiles had `ctutils 0.4.6`
  against the main lockfile's 0.4.5; re-synced (`cargo update -p ctutils
  --precise 0.4.5`), every shared package now has the main lockfile's
  version, and Dependabot covers the four fuzz directories.
- Low (sync, c2): accepted as M2-R13 (below), folded into the M2-R7
  redesign.


Gate after the second review round (Windows, 2026-10-27, every run
completed and its log read): `cargo fmt --all --check`, `cargo clippy
--workspace --all-targets -- -D warnings`, `cargo test --workspace` (404
passed, 4 ignored), `cargo test --release --workspace -- --ignored` (4
passed: the 50 MiB attachment round trip, the two baseline Argon2
profiles, and the 60-seed × 12-round sync simulation in 288 s), `cargo
deny check` for the workspace and all four fuzz crates, `cargo audit
--deny warnings` for all five lockfiles, `pnpm install --frozen-lockfile`,
`pnpm lint` (0 warnings), `pnpm typecheck`, `pnpm test` (105 passed; the
design-tokens case "keeps its label readable on the solid fallback", added
afterwards, makes 106, see the third round) and `pnpm audit`: all green. Coverage (`cargo llvm-cov -p <crate>`, lines /
regions): mosaic-core 98.77 % / 97.26 %, mosaic-vault 97.90 % / 92.93 %.
Fuzz (`apply_remote`, Docker `rustlang/rust:nightly`, ASan, seeds as in
CI): the strengthened oracle's first 150 s run found an oracle gap, not a
bypass: a bit flip in the item header's `schema_version` is refused with
`UnsupportedVersion` before decryption, which the oracle now accepts as a
refusal. That input is the new seed `tampered_schema_version`. The rerun,
180 s, 3112 executions (the target builds two sessions per input, about
17 exec/s), found no crash; the generated corpus is not committed.

### M2 integration review, third round, 2026-10-27

crypto-reviewer, sync-chaos, test-engineer and a gate review reported 1
high, 9 medium and 14 low findings. Fixed with regression tests (each
checked against the old code where a test could fail on it), except the
accepted risks in the table:

- High (sync): a reused device ID lost data silently on every device. Two
  writers with one ID that edit different fields produce the same stamp,
  each side's vector covers the other's, and the pure merge dropped both
  sides' values; the documented recovery (rotate, merge again) lost them
  too. Now (ADR-0015, CRYPTO_SPEC §10): every unlock writes under a fresh
  writer ID (the stored one is never read back), so a restored backup or a
  copied store can no longer share one; equal vectors with different
  contents are a certain collision and every dropped value is written back;
  and after `rotate_device_id` the old ID stays suspect for the session, so
  `merge_and_repair_after_reuse` keeps both writers' values. The value
  written last by the wall clock is shown. Tests: both schedules of the
  finding in `model_sync.rs` (`reused_device_id_edits_of_different_fields_lose_nothing`,
  `restored_backup_with_a_reused_id_loses_nothing_after_rotation`), three
  devices at the session level (`edits_of_different_fields_under_a_shared_id_lose_nothing`),
  and `a_copied_store_never_writes_under_the_original_writer_id` (a copied
  store file, both copies edit, nothing lost). The shared-ID session tests
  moved to unit tests, which force the ID; through the public API it can
  no longer be shared. The sync simulation gets no ID-reuse case for the
  same reason.
- Medium (crypto): a hostile `passwordrules` attribute could force a
  one-character password (`allowed: [ab]; max-consecutive: 1;
  max-sequential: 1;`): the length search fell back to the site's
  minlength. It now never goes below 4 unless the site's maxlength is
  shorter (then exactly that), and refuses otherwise (ADR-0011 amendment).
  Tests: `hostile_rules_never_force_a_password_below_the_floor`, proptest
  `generated_length_respects_the_floor`.
- Medium (crypto): a revocation (§8.3) kept every attachment's FK and ID,
  so a removed member and the server could replace attachment contents.
  Spec first: CRYPTO_SPEC §8.3 now requires re-sealing every attachment
  under a fresh ID and FK, ADR-0017; implemented with sharing in M6/M7
  (accepted for M2 as M2-R15, THREAT_MODEL R18; `rotate_vault_key`'s doc
  says it is not a revocation).
- Medium (sync): a merge of two sealable replicas could exceed 1 MiB and
  then never converge. Such a merge is now split deterministically
  (ADR-0016, CRYPTO_SPEC §10): every device keeps the same version in the
  record (longer plaintext, attachments merged, vectors joined), and the
  device whose own version lost stores it as a new item marked
  `conflict_copy_of` (`ApplyOutcome::Split`). Tests:
  `an_oversized_merge_is_split_deterministically`,
  `concurrent_edits_of_a_large_note_converge_through_a_split`.
- Medium (sync): the delivery-order dependence of the delete-race restore
  (M2-R7) is re-confirmed; `characterization_delete_race_restore_depends_on_delivery_order`
  pins both orders, so the M6 redesign has to flip it.
- Medium (tests): attachment pruning after a received record had no test
  (two surviving mutants). `received_removals_prune_attachment_files`
  covers removal and deletion, FastForward and Merged (both mutants now
  fail), and the simulation checks that device 0 holds no orphaned
  attachment row or file.
- Medium (tests): the re-seal after a repair by a newer generation had no
  test (surviving mutant). `a_repair_by_a_newer_generation_reseals_every_readable_older_record`
  kills it. Loading a vault now reads every key row that decodes
  (`Store::readable_vault_keys`), so one damaged row no longer hides the
  other generations and turns their records into anchor-less rows.
- Medium (tests): a symbolic link to the database got its own lock file.
  `Store::open` resolves the path first and names the lock file and the
  attachment directory after it (ADR-0010 amendment). Tests:
  `a_symlink_to_an_open_store_is_refused` (Unix), `a_store_path_is_resolved_to_one_spelling`.
  Hard links stay a residual (THREAT_MODEL R13).
- Medium (gate): CORE_API listed `Entropy::from_bits` (it is
  `Strength::from_bits`) and missed `PasswordOptions` and
  `PassphraseOptions`. Both are listed with their fields, `Default` and
  `entropy()`, and are now `#[non_exhaustive]`, like `ItemFilter`, so a
  new option is additive (CORE_API change policy updated).
- Low (crypto): the unknown-vault path of `apply_remote_vault_key` passed a
  floor rollback through as `Error::Store`; it is the session's
  `KeyGenerationRollback` now (extended `store_refusals_of_a_rotation_are_session_errors`).
- Low (crypto): `mosaic_totp::Code` holds its characters in a heap box, so
  returning it by value leaves no stack copy; `OtpAuth`'s issuer and
  account name are zeroizing `SecretText` (ADR-0008 amendments). Tests:
  `a_code_is_boxed_and_moves_without_copying_its_characters`,
  `labels_are_zeroizing_and_redacted`.
- Low (crypto): `passwordrules` text is bounded at 4096 bytes, as §13 says
  (was characters); boundary test with multi-byte input at 4096 and 4097
  bytes.
- Low (sync): a damaged local row replaced by a received version is
  reported as `ApplyOutcome::ReplacedUnreadable` (no rollback anchor,
  THREAT_MODEL R17), not `New`.
- Low (sync): `mark_uploaded` returns `false` for a late acknowledgement
  of a change a newer merge superseded, instead of `StaleServerSeq`
  (`a_late_upload_ack_after_a_newer_pull_is_not_an_error`, store test and
  property model updated).
- Low (sync): the 32-entry history bound holds per edit; merges can hold
  more until the next change (CRYPTO_SPEC §10). Tests:
  `concurrent_password_changes_can_exceed_the_history_bound_until_the_next_change`,
  and the history proptest checks that one change after converging trims
  back to 32.
- Low (tests): deterministic versions of the seed 229651 scenario at the
  session level (`a_net_removal_does_not_beat_a_concurrent_delete`,
  `an_attachment_added_and_removed_does_not_beat_a_concurrent_delete`) and
  the model level; the simulation picks items in creation order instead of
  by random ID (a seed is still not an exact replay, M2-R14).
- Low (tests): a swapped or flipped attachment file fails with
  `Decryption` and leaves the rest readable
  (`a_tampered_or_swapped_attachment_file_fails_to_open`); the metadata
  same-value collision is reported as `Merged`
  (`a_same_value_metadata_stamp_collision_is_stored_as_received_and_reported`);
  the session types' `Debug` output is pinned exactly
  (`session_debug_output_holds_no_secrets`), plus `dup_item` on a deleted
  item and a non-`Decryption` unlock error.
- Low (gate): the design snapshots are described as rendering only inside
  the design canvas (`docs/design/README.md`); every fuzz crate forbids
  `unsafe_code` in its `[lints.rust]` (checked: `fuzz_target!` compiles
  under `forbid`); the second-round gate's pnpm count is annotated.

Gate after the third review round (Windows, 2026-10-27, every run completed
and its log read): `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace` (429 passed, 4
ignored), `cargo test --release --workspace -- --ignored` (4 passed: the
50 MiB attachment round trip, the two baseline Argon2 profiles, and the
60-seed × 12-round sync simulation with the new orphan check in 130 s),
`cargo deny check` for the workspace and all four fuzz crates, `cargo audit
--deny warnings` for all five lockfiles, `RUSTFLAGS="--cfg fuzzing" cargo
check --bins` for all four fuzz crates (with `unsafe_code = "forbid"`),
`pnpm install --frozen-lockfile`, `pnpm lint` (0 warnings), `pnpm
typecheck`, `pnpm test` (106 passed) and `pnpm audit`: all green. Coverage
(`cargo llvm-cov -p <crate>`, lines / regions): mosaic-core 98.31 % /
96.83 %, mosaic-vault 98.37 % / 93.22 %, mosaic-totp 99.43 % / 99.25 %,
mosaic-generator 99.13 % / 98.66 %. (The fuzz run this round left out
was done after the fourth round, below.)

### M2 integration review, fourth round, 2026-10-27

The adversarial review loop ended after four rounds with 14 actionable
findings (1 high, 7 medium, 6 low); each was verified against the code
(reproduced by a test wherever possible) and fixed, spec and ADR first for
behaviour changes. One is recorded for M6 (as the finding asked), none was
rejected outright, and one test request was narrowed to what the design
claims:

- High (sync): an oversized split lost a committed edit depending on
  delivery order (the device that kept its own version made no copy; the
  peer fast-forwarded to the split record). While fixing it, the new
  oversized case of the sync simulation showed that a pair-based split is
  wrong in general: three devices merging different pairs reached one
  vector with different records and raised `DeviceIdReused`. Redesigned
  (ADR-0016 amendment, CRYPTO_SPEC §3 and §10): the record is the merge
  result reduced as a pure function of that result (each content field
  keeps its shown value, then the largest fields leave if needed;
  metadata, history, counters, attachment references and deletion entries
  stay), and every value that left becomes a conflict copy holding only
  that value and the mark under the value's own stamp, with the ID
  `UUIDv8(HKDF(VK, "", ctx("conflict-copy-id") || item_id || key || stamp))`,
  so every device builds the same copy under the same ID. A first version
  that also copied the record's type into the copy failed the long
  simulation (a peer's copy carried the receiving device's own creation
  stamp); the copy now carries nothing from the record. `ApplyOutcome::Split`
  is `{ conflict_copies: usize }`. Tests:
  `a_split_record_delivered_first_to_the_losing_peer_loses_nothing` (model)
  and `a_split_record_reaching_the_losing_device_first_loses_nothing`
  (session), `an_oversized_merge_is_split_deterministically`,
  `concurrent_large_notes_split_and_converge_under_shuffled_delivery` (sync
  simulation, 2 seeds normally, 22 more in the `--ignored` run; 48 seeds
  passed once in release).
- Medium (sync): the split decision depended on the per-device repair; the
  split record dropped the deletion entry (delete undone without notice)
  and the purging peer got `DeviceIdReused`. Fixed by the same redesign
  (deletion entries always stay; a repairing device's record dominates the
  plain merge). Test: `a_split_after_a_concurrent_delete_keeps_the_notice_and_converges`
  (the finding's 3-device schedule), plus the purge seeds of the
  simulation case. The suggested "no `DeviceIdReused` for a `deleted`
  stamp under equal vectors" exception is not needed and was not made.
- Medium (crypto): hostile `passwordrules` could shrink the alphabet
  instead of the length. CRYPTO_SPEC §13 and ADR-0011 amended first: an
  alphabet below 2 characters is refused, and a length is used only if
  `n × log2(|alphabet|) + log2(p(n)) ≥ log2(10^4)` (a 4-digit PIN), with no
  exception for a short site `maxlength` (more conservative than the
  review's offer, since the page sets `maxlength` too). New public
  constant `MIN_RULES_BITS`. Test:
  `hostile_rules_never_force_a_password_below_the_entropy_floor` (the
  three cases of the finding and more; a 4-digit PIN still passes; every
  vendored quirks entry still generates).
- Medium (sync): password history was trimmed by the register's stamp,
  which a delete-race repair renews. It is trimmed by the stamp encoded in
  the history key now (ADR-0009 amendment, CRYPTO_SPEC §10). Test:
  `a_restored_history_is_trimmed_by_the_original_edit_order` (the
  finding's schedule; fails on the old code), and the history proptest
  now includes deletes.
- Medium (tests): the ADR-0016 attachment join was untested. Tests: the
  attachment cases in `an_oversized_merge_is_split_deterministically`,
  `a_split_against_a_deletion_keeps_the_attachments_and_the_notice` (a
  deleted kept side), and the session test
  `concurrent_edits_of_a_large_note_converge_through_a_split` (each device
  adds an attachment first; both referenced everywhere, equal usage, no
  file pruned).
- Medium (tests): the ADR-0015 recovery had no randomized test. Added
  `reused_writer_ids_lose_nothing_once_the_writers_meet` (2 writers sharing
  an ID, 1 or 2 honest devices, random edits of title, password, tags and
  URLs, random gossip; no unsuperseded value lost, all replicas equal),
  the `item_merge` fuzz mode 2 (the same from fuzz input) and a reuse merge
  in modes 0/1, and the missing payload-order unit case. Narrowed: the
  finding asked for "no loss" under any gossip order, but ADR-0015 claims
  detection only where the reuse shows; an honest device that merges both
  writers' versions before they meet loses both values, which
  `reuse_that_shows_nowhere_is_not_detected` now pins (needs a forged
  record, since writer IDs are fresh per unlock). The property test found
  a real bug on the way: a collision repair wrote a password back into a
  shared `history:*` key where it stayed invisible; it gets its own key
  now (`a_history_key_written_by_two_writers_keeps_both_passwords`,
  ADR-0009/ADR-0015 amendments).
- Medium (sync): a move into a vault a peer rotates concurrently makes the
  item vanish on that peer. Verified
  (`characterization_a_move_into_a_concurrently_rotated_vault_vanishes_on_the_rotator`)
  and recorded under M2-R12 / THREAT_MODEL R16 with the M6 fix, as the
  finding proposed.
- Low (crypto): metadata replacing an unreadable metadata row reported
  `New`; now `ReplacedUnreadable` (ADR-0012 amendment, R17, CORE_API).
  Test: `received_metadata_replacing_an_unreadable_row_is_reported`.
- Low (tests): `conflict_copy_of` validation untested. Test:
  `conflict_copy_of_is_validated_and_kept`; the golden item now carries
  the member (re-blessed, diff reviewed: only that member added, tombstone
  golden unchanged), the dictionaries list it, new seeds
  `item_plaintext/conflict_copy{,_nil}`.
- Low (tests, sync): the split fallbacks. The old fallback that dropped
  the other side's attachment references is gone; `split_fallbacks` covers
  moving a whole field out, the unmarked copy, and `SizeLimit` when the
  attachment references alone exceed the limit.
- Low (tests): the generator floor property could pass vacuously. It runs
  with a counting runner now (at least a quarter of 1024 rule sets must
  generate), and `satisfiable_rules_are_never_reported_unsatisfiable`
  brute-forces small alphabets.
- Low (tests): store path branches. Tests:
  `a_relative_store_name_shares_the_lock_with_every_spelling` (bare,
  `./` and absolute spellings, serialized current directory),
  `only_the_verbatim_disk_prefix_is_removed`, and the Windows
  `a_windows_symlink_to_an_open_store_is_refused` (returns early without
  the symlink privilege).
- Low (gate): fuzzing; done, see below.

No new dependency. The conflict-copy ID is a new HKDF label
(`conflict-copy-id`) under the existing HKDF-SHA-256 (CRYPTO_SPEC §2, §3;
ADR-0016 amendment).

Gate after the fourth review round (Windows, 2026-10-27, every run completed
and its log read): `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace` (448 passed, 4
ignored), `cargo test --release --workspace -- --ignored` (4 passed in
5 min 15 s: the 50 MiB attachment round trip, the two baseline Argon2
profiles, and the 60-seed × 12-round sync simulation plus 22 oversized
seeds in 176 s), `cargo deny check` and `cargo audit --deny warnings` for
the workspace and all four fuzz crates, `RUSTFLAGS="--cfg fuzzing" cargo
check --bins` for all four fuzz crates, `pnpm install --frozen-lockfile`,
`pnpm lint` (0 warnings), `pnpm typecheck`, `pnpm test` (106 passed) and
`pnpm audit`: all green. Coverage (`cargo llvm-cov -p <crate>`, lines /
regions, after `cargo llvm-cov clean --workspace`; a first run on stale
profile data reported 85 %): mosaic-core 98.59 % / 97.11 %, mosaic-vault
98.40 % / 93.25 %, mosaic-totp 99.43 % / 99.25 %, mosaic-generator
99.17 % / 98.72 %.

Fuzzing (Docker `rustlang/rust:nightly`, Linux, ASan, `cargo-fuzz@0.13.2`,
seeds and dictionaries as in CI, corpus kept inside the container): every
new or changed target ran without a crash: `item_merge` (with the new mode
2 and seeds) 180 s, 4825 executions; `item_plaintext` (new seeds and
dictionary entry) 60 s, 148 k; `apply_remote` 180 s, 5816 (the target
builds two sessions per input); `password_rules` (entropy floor) 60 s,
211 k. The slow targets ran while the machine was also measuring
coverage, so their execution counts are low; CI's fuzz-smoke job runs all
targets again.

Next steps for M2:

1. Commit by explicit path: the integration as one `feat(vault)` commit
   (session, otp, model and store review fixes of all four rounds,
   docs, CI, Dependabot; no new dependency in the third or fourth round),
   and the design tokens / UX guidelines / `docs/design/`
   change separately after the ux-reviewer signs it off. Then mark M2 done
   here once CI is green on all three OSes, including the release
   `--ignored` run.

## Open risks / findings

| ID | Severity | Item | Status |
|---|---|---|---|
| M1-R1 | low | Upstream `hmac`/`hkdf` stack residue (key-pad block, last HKDF output block) during derivation | accepted, THREAT_MODEL R11, ADR-0005 |
| M1-R2 | low | Secret Key parsing still branches on whether a character is `-`/whitespace and on the final validity flag. Neither reveals the key: separators carry no key bits and invalid input is rejected | accepted |
| M1-R3 | low | The independent baseline vector is checked in, but CI does not re-run the Python script (it needs argon2-cffi and 128 MiB). Re-run it when the derivation changes | accepted |
| M1-R4 | low | Fuzzing cannot run natively on Windows MSVC (ASan runtime mismatch); local runs use Docker, CI runs on Linux | accepted |
| M1-R5 | info | The main-workspace `cargo deny check` warns `license-exception-not-encountered` for the `libfuzzer-sys` NCSA exception, which only applies to the fuzz lockfile | accepted |
| M1-R6 | low | Constant-time unpadding is best-effort at the source level (`subtle` selects, no early exit). The compiler gives no hard guarantee. Code that later processes the unpadded plaintext (M2 item parsing) takes time proportional to its length. Only a local attacker who can time client-side decryption could exploit this | accepted |
| M2-R1 | medium | Wrapped vault keys and vault metadata have no upload queue: only records and attachments go through the outbox. A vault created or renamed offline reaches other devices only once M6 uploads them | open, M6 (ADR-0012 consequences, CORE_API "Expected additions") |
| M2-R2 | low | After a key rotation, other devices reject records sealed under the old generation (§7.1). Every device re-seals what it holds when it accepts the new key (ADR-0012 item 7), so no version is lost, but the server should still refuse uploads below a vault's current generation and accept one wrapped key per generation (ADR-0013) | open, M6 |
| M2-R3 | low | Writer-ID reuse: fixed in the third review round by fresh writer IDs per unlock and collision-safe merges (ADR-0015). Left: two sessions on one store through a hard link (the lock follows the resolved path) can still overwrite each other's upsert from a stale cache, each under its own ID | accepted (hard links only), THREAT_MODEL R13 |
| M2-R4 | info | A move that races an edit of the source on another device leaves the moved copy and the surviving source (the source carries a "deleted on another device" notice, the copy the old values) | accepted, ADR-0012 item 6; see M2-R9 |
| M2-R7 | medium | A delete racing an edit can lose a newer value, depending on delivery order: a device that has not seen the newest value restores its own stale one under fresh stamps, and the newer version is then dominated (sync review: P1 lost when the tombstone arrives first). The fix needs the deleter's pre-delete state (in the tombstone or kept by the deleting device) and restores into history or conflict copies, which changes CRYPTO_SPEC §7.2/§10 | accepted for M2, THREAT_MODEL R15; designed with the M6 protocol (spec and ADR first) |
| M2-R8 | medium | A delete racing a move of the same item is undone without a notice: the delete merges into the source's tombstone, the copy stays live and unflagged. Needs move pointers (`moved_to` in the source tombstone, `moved_from` in the copy) so a concurrent delete flags the copy | accepted for M2, THREAT_MODEL R15; spec and ADR with M6 |
| M2-R9 | low | Concurrent moves of one item leave two live copies without a "moved concurrently" notice, and a move racing an edit splits the values between the revived source and the copy. Same move pointers as M2-R8 | accepted for M2, THREAT_MODEL R15 |
| M2-R10 | low | A delete or move that loses against a concurrent edit has already pruned the attachment files on its device; the revived item references files only the editing device holds | open, M6: GC blobs only when no live version references them, fetch missing files (ADR-0012 consequences) |
| M2-R11 | info | Vault metadata renames are not part of the 3-device simulation (metadata has no upload queue yet, M2-R1); concurrent renames, renames under a shared device ID and renames of damaged metadata are covered by session tests, and metadata now merges with `VaultMeta::merge_and_repair` | open, M6 with the metadata queue |
| M2-R12 | medium | An edit the server accepted under generation `n` is lost if another device rotates to `n + 1` before pulling it and the editor never returns; peers that accept `n + 1` first refuse the record as a rollback, so the outcome depends on delivery order (sync review t4, test `a_record_in_flight_during_a_rotation_is_refused_by_peers`). A move into a vault that a peer rotates concurrently makes the whole item vanish on that peer without a notice (it refuses the copy and fast-forwards to the source tombstone) until the mover accepts `n + 1` and re-seals the copy (fourth review, `characterization_a_move_into_a_concurrently_rotated_vault_vanishes_on_the_rotator`). Fix in the M6 protocol: rotation conditional on the vault head (CAS on generation plus last-seen `server_seq`), no uploads below the current generation, a move's tombstone uploaded only after the server accepted the copy, and a peer that cannot read the copy flags the source instead of fast-forwarding | accepted for M2, THREAT_MODEL R16; spec and ADR with M6 |
| M2-R13 | low | Whether the delete-race repair runs depends on which versions a device receives: if the device that won the race dismisses the notice before a third device pulls, and the server coalesces versions, the third device never sees the deletion and silently keeps the deleter's state (sync review c2). Same root as M2-R7 (the repair needs to see the deletion) | accepted for M2, folded into the M2-R7 redesign (THREAT_MODEL R15) |
| M2-R14 | low | The `apply_remote` fuzz target builds its sessions from OS randomness (keys, IDs, nonces; the session has no seeded mode and the crypto core takes no injected RNG), so a failure that depends on ID order or key bytes may not reproduce from the saved input. Documented in the target; the substituted IDs are derived from the input. The same holds for the sync simulation's seeds: items are picked in creation order now, but random device IDs break ties between concurrent writes, so a seed is not an exact replay; failures get a deterministic test (as for seed 229651) | accepted |
| M2-R15 | medium | A revocation (§8.3) must re-seal every attachment under a fresh ID and FK, or a removed member and the server can replace attachment contents (ADR-0017). Not reachable in M2: no other account holds a vault key, and `rotate_vault_key` is not a revocation | open, M6/M7 with sharing and the revoke flow; THREAT_MODEL R18 |
| M2-R16 | low | Vault metadata merges are not split like items (ADR-0016): a metadata merge above 64 KiB still fails. It needs tens of kilobytes of concurrent names or descriptions | accepted; M6 caps metadata values with the metadata queue (M2-R1) |
| M2-R17 | low | A local record or metadata row that cannot be read has no rollback anchor; the received replacement is reported (`ReplacedUnreadable`, for metadata since the fourth review) but not verified | accepted, THREAT_MODEL R17; M6 can require checkpoint evidence |
| M2-R18 | info | Fresh writer IDs add one version-vector entry (about 44 bytes) per session that edits a record; an HOTP item grows by one per session that shows a code. Collision repairs can bring back superseded values as conflict copies. An oversized split leaves one conflict-copy item per value that left the record (copies have no type of their own; the UI shows the original's through `conflict_copy_of`); devices at different key generations derive different copy IDs and can leave a duplicate | accepted (ADR-0015, ADR-0016 and its amendment); vector pruning is a future format change |
| M2-R19 | low | Writer-ID reuse is detected only where it shows (equal vectors with different contents, a counter above the replica's own, one stamp with two contents in a field). An honest device that merges two colliding versions showing none of these before the writers meet drops both writers' values (`reuse_that_shows_nowhere_is_not_detected`). Needs a forged or corrupted record or a UUIDv7 collision, since writer IDs are fresh per unlock | accepted (ADR-0015 amendment); the randomized test covers the case where the writers meet first |
| M2-R20 | low | An oversized merge whose record does not fit even without every content field (its password history or attachment references alone exceed 1 MiB), or a value that does not fit a copy of its own, fails with `SizeLimit` and stays unmerged until the user shrinks the item | accepted (ADR-0016 amendment item 4); needs thousands of attachments or history entries near the limit |
| M2-R5 | low | `serde_json` scratch copies of escaped strings during plaintext parsing | accepted, THREAT_MODEL R14 |
| M2-R6 | info | Hardware-protected TOTP vaults (§7.4) are listed as unavailable: TK unlock and acting on `OtpConfig::hardware_protected` are M3 | open, M3 |

See the THREAT_MODEL §5 residual-risk register.

## Needs owner

- **License**: choose a project license (for example AGPL-3.0 for the server
  and Apache-2.0/MIT for clients, or a single license). Crates are
  `publish = false` until decided.
- **Name availability**: "Mosaic" on crates.io / npm / domains / app stores,
  and a trademark check.
- **Signing**: Apple Developer ID, Windows code-signing certificate, extension
  store accounts (Chrome Web Store, Edge Add-ons, AMO). Needed from M3/M4 and
  for releases in M9.
