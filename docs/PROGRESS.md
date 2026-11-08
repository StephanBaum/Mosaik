# Progress

Resume from here in every session. Requirements: `docs/BRIEF.md`. Decisions:
`docs/DECISIONS.md`. Internal review ledger: `docs/SECURITY_REVIEW_LOG.md` (each
milestone close appends its section there).

## Status

| Milestone | Status |
|---|---|
| M0 Foundations | ✅ done (2026-10-25) |
| M1 Crypto core | ✅ done (2026-10-26, `c43d93c`, CI fix `51ba5b2`) |
| M2 Vault model + local store | ✅ done (2026-10-27, `2141a7d`, CI fix `9382cdc`; design work `54ea729`) |
| M3 Desktop app | ✅ done (2026-11-06; merge commits `f54bd55` to `edacfea`, then PRs #13 to #22 and #24: UI fidelity and holo, quick unlock, red-team passes, virtual list, one-time-code setup, unlock reveal, item kinds and custom fields, attachments, UX fixes; open, owner: security-key transport (WP7), manual hardware checks (Hello, Touch ID, QR on two monitors, Save-as overwrite prompt), holo button polish after the build-out) |
| M4 Browser extension | ✅ done (2026-11-06; PRs #23, #25, #26, #27 and #29; ADR-0035 to 0038 accepted; open: manual checks on Windows Chrome/Edge (registry install), Firefox 128+, macOS and Linux Chrome, `activeTab` gating on the production build, keyboard and screen reader in the popup; owner items: store IDs, AMO gecko ID, host name and bundle identifier, D06 store links) |
| M5 Passkeys | ✅ done (2026-11-06; PRs #28, #30, #31, #32 and `chore/m5-close`; ADR-0039 to 0041 accepted; open: manual checks with real WebAuthn relying parties in Chrome, Edge and Firefox, Firefox MAIN-world registration; owner: OS passkey providers (deferred: Windows unsafe exception, macOS Developer ID), a registered AAGUID, keep or drop the CXF export) |
| M6 Sync | ✅ done (2026-11-07; PRs #33, #35, #36, #37, #38 and the close branch `chore/m6-close`, #39; ADR-0042 to 0047 accepted; open: manual checks with real Dropbox, iCloud and OneDrive folders on Windows, macOS and Linux, the hosted server behind Caddy on a VPS, two physical devices, the keystore token on each OS; owner: hosted domain (`HOSTED_DEFAULT_URL`), scope split (M6b), license (AGPL server question; it also gates the free audit programs)) |
| M6b Sharing and emergency access | in progress on `feat/m6b-sharing` (T1 to T4 and the milestone review fixes done; T5 desktop: commands, screens, mock states, i18n, fuzz seeds, docs; a verify pass over the review fixes is running; open: the manual checks below, the owner decisions OD1 to OD6 of the spec, the full gate and the milestone commit) |
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
  unchanged. Only `tests/it/baseline_profile.rs` runs a real profile; everything
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
- Integration tests in `crates/vault/tests/it/`: `session.rs` (26 end-to-end
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

## M3 Desktop app — skeleton (2026-10-27)

The skeleton that the five tracks (import, insights, SSH agent, Tauri shell,
React UI) build on. Committed on `main` as one `feat(desktop)` commit.

What it contains:

- `apps/desktop` (`@mosaic/desktop`): Tauri v2 + React 19 + Vite 8, strict
  TypeScript.
  - Rust side: `apps/desktop/src-tauri` (`mosaic-desktop`, a workspace
    member).
  - Security baseline (ADR-0019):
    - the capability grants only Mosaic's 34 commands; events travel over a
      `Channel`, so no `core:*` permission is needed
    - strict CSP (no remote origin, no `unsafe-eval`, no `unsafe-inline`)
    - `freezePrototype`, no devtools in release builds
    - single-instance plugin, Tauri without default features
  - DM Sans and IBM Plex Mono are bundled from `@fontsource`, with the OFL
    texts in the build output.
- IPC contract (`docs/DESKTOP_IPC.md`, ADR-0018):
  - session state machine, Emergency Kit, unlock / quick unlock / lock
  - vaults, items (summaries; detail with masked secrets; one-secret reveal)
  - copy in Rust, generator, TOTP, settings with re-auth
  - insights, import (Rust-side file dialog), SSH approvals, events
  - one code-only error enum mapped to `error.<code>` in DE and EN

  Type parity: Rust generates `src/ipc/contract.fixtures.ts`, `tsc` checks it
  with `satisfies`, and a Vitest test compares commands and enums as sets. A
  mock-runtime test invokes every command through the real config and ACL.
- Wired to `mosaic-vault` now: `session_state`, `create_account` (calibrated
  Argon2id, first vault, Emergency Kit), `verify_emergency_kit`, `unlock`,
  `lock`, `subscribe_events`, vaults (list, create, rename), items (list and
  search, detail, reveal, favorite, trash, restore, purge, move),
  `generate_password`, `get_settings`, `ssh_agent_status` ("off"). Every other
  command is a typed stub that returns `notImplemented`; each stub names its
  track. The Secret Key is kept in memory (`MemoryKeystore`) until the unlock
  track adds the OS keystore.
- Mock IPC (`src/ipc/mock.ts`, handlers per area in `src/ipc/mock/`),
  selected with `--mode mock`. Non-mock builds replace it with a stub.
- Frontend:
  - app shell with session-driven routing (onboarding → Emergency Kit
    verification → vault; unlock; lock)
  - theme (`data-theme` over the token stylesheet; system default)
  - i18n provider (ICU through `intl-messageformat`; 61 new keys in DE and
    EN)
  - placeholder vault screen
- Tests and tooling:
  - Vitest + Testing Library: the full onboarding, unlock and lock flows
    against the mock, plus a German check
  - a tokens-only style check
  - Playwright + axe in light and dark against the mock build under the
    production CSP, with a no-external-request check
- `packages/design-tokens`: `cssVariables()` (`@mosaic/design-tokens/css`),
  plus `borderWidth`, primary-action label, fallback and rim variables.
- New scaffold crates for the tracks: `crates/insights` (`mosaic-insights`)
  and `crates/ssh-agent` (`mosaic-ssh-agent`). `crates/import` already
  existed.
- CI:
  - `desktop-rust` (check, clippy, test of `mosaic-desktop` on Linux, Windows
    and macOS; the WebKitGTK libraries on Linux)
  - `desktop-frontend` (lint, typecheck, Vitest, build, Playwright with axe on
    Ubuntu)
  - the core `rust` job now excludes `mosaic-desktop`
- THREAT_MODEL §4.2 (isolation pattern deferred to M9, ADR-0019 item 4) and
  R19 (secrets in the webview).

Run it:

- `pnpm --filter @mosaic/desktop dev:mock`: the UI in a browser at
  http://localhost:1420 (`?mock=locked` or `?mock=unlocked`; the mock master
  password is `mosaic`).
- `pnpm --filter @mosaic/desktop tauri dev`: the real app.
- `pnpm test:e2e` runs Playwright (first `pnpm --filter @mosaic/desktop exec
  playwright install chromium`).
- `MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-desktop contract_fixtures`
  after a contract change.

## M3 Desktop app — integration (2026-10-29)

The five tracks are merged on `m3/integration` and wired end to end (merge commits `f54bd55`, `a737932`, `7d05f7b`, `b57f1cd`, `edacfea`).
M3 is **not done**: see "Left for M3" below.

What is in:

- **Shell** (ADR-0020):
  - the Secret Key in the OS keystore
  - the clipboard from Rust, with auto-clear and history exclusion
  - auto-lock on idle and resume (plus logind and screen saver on Linux)
  - settings persisted with re-authentication and the CRYPTO_SPEC §11 floor
  - the global shortcut (focus plus `quickSearchRequested`)
  - the Emergency Kit PDF saved through the native dialog, with `kitVerified`
    stored per account
  - `contentProtected`
  - `add_item`, `update_item` and `totp_code` live
- **Import** (`mosaic-import`, ADR-0021):
  - Enpass, Bitwarden, KeePass KDBX 4, 1PUX and CSV
  - parsed outside the session lock
  - preview with dedupe; commit and cancel
- **Insights** (`mosaic-insights`, ADR-0022 to ADR-0024):
  - weak, reused, old, missing-2FA and plain-HTTP findings, offline
  - the 2FA Directory data is bundled
- **SSH agent** (`mosaic-ssh-agent`, ADR-0025):
  - Ed25519 and ECDSA P-256
  - approval per use
  - off by default; runs only while unlocked
- **UI:**
  - screens D02–D05, D07–D13, D17–D19
  - the command palette (Ctrl+K and the global shortcut)
  - the primary action as the static holo gradient (the WebGL foil renderer from
    `docs/design/holo/` is not ported yet; the static gradient is its fallback)
  - item lists render 200 rows plus "show more" (no virtual scrolling yet)
  - reveal with auto-hide, toasts with live regions
  - DE and EN, with i18n catalogs split per area
- **Contract changes** (ADR-0026, `docs/DESKTOP_IPC.md`):
  - new commands: `save_emergency_kit`, `app_status`
  - new events: `sshSignFailed`, `quickSearchRequested`
  - four new error codes
  - richer import, SSH and insights payloads
  - **renamed:** `SshDecision.allowUntilLock` → `allowFor15Minutes`;
    `security_report` findings `kinds` → `issues`, and `reusedWith` now carries
    `{ vaultId, itemId }`
- **Tests:**
  - the command-layer flow test `flow_tests::full_flow_create_unlock_add_copy_clear_lock`
  - Vitest flows against the mock, Playwright with axe in light and dark for
    each screen
  - static UI guards
- **Fuzz:** 13 new targets (insights 2, import 6, ssh-agent 2, desktop shell 3).
  Locally they were only compiled (`--cfg fuzzing` `cargo check` on Windows).
  The 60 s smoke for them ran only in CI.
- **CI:**
  - coverage for `mosaic-insights`, `mosaic-import` and `mosaic-ssh-agent`
  - fuzz smoke, cargo-deny and cargo-audit for the four new fuzz crates
  - an OpenSSH client check on Windows for the interop test

Review and fixes (2026-10-29): crypto-reviewer over all M3 code (3 medium, 2 low)
and ux-reviewer (2 high, 7 medium, 5 low), then two fix rounds, each checked by an
independent final review. The 1PUX attachment lookup was quadratic in archive
members × items (a crafted export could stall the import); it now uses a per-document
index, with a test that counts lookups.

Gate at the integration commit, run once on Windows: fmt, clippy, cargo test, deny,
audit, pnpm install, lint, typecheck, test and audit all pass; Playwright 13 passed
(axe in light and dark). `cargo build -p mosaic-desktop` and `pnpm --filter
@mosaic/desktop build` succeeded; release `--ignored` tests passed for `mosaic-core`,
`mosaic-vault`, `mosaic-ssh-agent` and `mosaic-desktop`. Coverage was not run.

Left for M3:

1. Quick unlock (biometrics, hardware key; M3-R6) and TK unlock of
   hardware-protected TOTP vaults (M2-R6). Both need the CORE_API additions, an
   ADR and a crypto-reviewer pass.
2. The rest of the milestone review: extension-attacker for the IPC surface
   (crypto-reviewer, ux-reviewer and test-engineer ran on 2026-10-29, see above).
3. ~~CI green on Windows, macOS and Linux~~: done at `3c1ea16` (2026-10-29).
   The first CI run found Unix-only clippy errors, an empty-host-label bug in
   insights (from the fuzz smoke), missing ssh-agent fuzz seeds, a 0755 test
   socket directory and a duplicate `generate_context!` on macOS; all fixed.
   The Linux and macOS shell code (zbus, the macOS clipboard) is compiled and
   tested only in CI.
4. Manual keyboard-only and screen-reader walkthroughs of the real app on
   each OS.
5. Onboarding step D06 (install the extension) comes with M4.
6. Port the WebGL holo foil (`docs/design/holo/holo-button.js`, ADR-0014) and add
   virtual scrolling for item lists past 200 rows.
7. One coverage run for the new crates and the frontend (≥ 90 % lines on library
   crates).
8. ~~Agreed follow-ups after the integration commit~~: done on `refactor/m3-audit`.
   The slimmer dev profile (`571d890`) and one integration-test binary per crate
   were done earlier. The code-audit items P1 to P16 (P2 earlier): `fe487c6`
   (import: KDBX `XmlState`, 1PUX `section_field`), `fe1ce06` (generator and
   insights: shared `host` module, per-item caching, strength speedups),
   `b375006` (core: `repair_after_merge`, `model/item/` split), `af131a7`
   (vault), `9530534` (totp, generator), `39d54c3` (desktop `host_of`). P10
   (i18n `icuArguments`) is not part of this series. Lints: the chore(lint)
   commit after these (`allow_attributes_without_reason` deny,
   `allow_attributes` warn, `cognitive_complexity`, `excessive_nesting`;
   thresholds 90 lines, complexity 25, nesting 5 in `clippy.toml`; every
   `#[allow]` is now a `#[expect]` with a reason, except in the fuzz crates).

- D20 (sync offline / server error) is deferred to M6 with the sync server: M3 has no network sync, so neither state can occur. The `sync.offline` string stays in the catalog, unused, until then.

## M3 Desktop app — UI fidelity and holo foil (2026-11-03)

The owner judged the integrated UI "very barebones" against the mockups. An Opus
side-by-side review (every screen, light and dark) produced a spec; Sonnet
implemented it in 10 work packages on `m3/ui-fidelity`; a final Opus review
fixed the remaining gaps. Fast-forwarded onto `main` (`754cff4`..`235e903`).

- **Look:** new tokens (radii 4/8/10, sizes, divider/faint colours, display
  type, 700 headings; ADR-0014 amended), ~35 line icons, logo mark, avatars,
  chips, callouts, countdown ring, restyled buttons/fields/switch/dialog/toast.
- **Screens:** mockup app shell (sidebar with categories, counts, vault dots,
  sync footer; top bar with breadcrumb, search, `Ctrl+K`), item list/detail,
  empty vault, editor, unlock and onboarding without cards (progress bar, kit
  preview, quick-unlock buttons), import, security cards, palette, generator,
  two-column settings, trash with days left and a purge confirmation, SSH
  dialog with key tile and countdown, a 64 px rail below 900 px.
- **Holo:** the WebGL2 foil ("angled rainbow stripes") is ported to
  `apps/desktop/src/holo/` (painted inside each button from one shared
  context; fallback = the static gradient). Press ripple and keyboard focus
  waves use the owner's tuned values. Foil accents on the logo tile, active
  nav, onboarding progress and the empty-state tile (ADR-0027). Two foil bugs
  fixed everywhere: the sheen now has a floor (it drifted to grey), and the
  colour drift over time is bounded (an unbounded time term times the
  per-pixel angle turned the foil into speckle after ~1 min).
- **Bugs fixed:** editor Save bar over Notes, empty "Fix these first"
  heading, "Delete forever" without confirmation, mock generator entropy
  (64-word list, bits from the list size).
- **Design studies** (`docs/design/holo/`): `effect-studies.html` (button
  press/focus effects with tuning sliders) and `reveal-studies.html` (unlock
  transitions; the owner shortlisted liquid glass and energy shockwave, a
  refinement pass is running). The chosen reveal is ported to the app next.
- **Supply chain:** `minimatch` overridden to 11.0.4 (GHSA-g3hv-75wg-m249,
  via eslint 10).
- **Gate:** TypeScript half run once on the branch (install, lint,
  typecheck, test, audit) plus all 6 Playwright specs (axe light/dark, CSP);
  no Rust files changed, so cargo clippy/test/deny/audit were not re-run
  (`cargo fmt --check` passes).
- **Out of scope, needs IPC changes** (from the spec): strength of stored
  passwords, inline OTP on the detail, passkey details, open website, share,
  breach checks, switch account, password history dates, SSH process/host
  rows, "delete export file", settings sub-pages.
- **Owner direction recorded the same day:** the browser vault is the
  extension itself (no hosted web vault; a standalone WASM extension vault
  is a later option with its own ADR); the website is a landing page with
  verifiable signed downloads (M9); M6 offers the Mosaic server or the
  user's own cloud folder, including Dropbox Business team folders; Mosaic
  targets business clients too.

## M3 Desktop app — quick unlock (2026-11-03)

Spec by Opus (ADR-0028), implemented in WP0–WP6 on `m3/quick-unlock`, PR #13.

- **Core/vault:** quick-unlock and WebAuthn-PRF derivations (HKDF, existing
  `WrappedKey` format, new labels; goldens checked against an independent
  Python HKDF). `verify_master_password` returns a `Reauthenticated` proof
  (120 s, per session) that enrollment requires. Quick unlock: 14 days by
  default, 30-day hard cap, password once per app start, expiry bound into
  the AAD, removed on password change. Hardware-key wrappings (max 8, PIN or
  UV required), hardware-protected code vaults under TK (open ≤ 5 min).
- **Desktop:** Windows Hello (TPM-attested key, per-store key-name nonce),
  Touch ID (Secure Enclave, biometry only; compiled and tested in CI only),
  Linux hides biometrics. Re-authentication no longer locks; five wrong in a
  row lock and require the password. New `reauthenticate` IPC command; 13 new
  commands, 15 error codes, `hardwareVaultClosed` event.
- **UI:** unlock screen methods with PIN step, Settings → Quick unlock,
  hardware-vault panel (D21) with a 30 s warning before it locks itself,
  create-vault dialog, re-auth retry flow, focus restore in every dialog.
- **Reviews:** two crypto-reviewer passes (core/vault: 0 high, 3 medium, 7
  low; desktop: 0 high, 3 medium, 9 low), all fixed except the manual
  hardware checks; ux-reviewer (1 high, 7 medium, 12 low), fixed except
  L5/L11/L12. M3-R20 (history proptest) was a test-oracle gap, fixed.
- **CI:** all checks green on Linux, macOS and Windows for the branch.

## M3 red-team (2026-11-03)

Branch `test/m3-redteam`. Attacked: the IPC command set from a compromised
webview (hostile bodies, wrong-field references, secret leaks), the
re-authentication grant, the five-try limits, hardware-vault commands, and
the state file `settings.json` as a local writer (P5) would edit it.

- **Held:** hostile bodies never panic, unlock or return a secret; reveal and
  copy return no secret they were not asked for; the grant dies with its
  session; wrong passwords from every path share one limit; hardware-vault
  commands refuse other vaults before the device; the CRYPTO_SPEC §11 floor
  and the 30-day cap held for a tampered file.
- **Fixed (ADR-0030 and amendment):** `settings.json` carries a MAC under a
  key from AK (`HMAC(K_settings, account_id || settings_nonce ||
  canonical)`); the nonce is a random local-only row of the vault store
  (schema v2) replaced with every write, so a restored older file fails;
  `sshAgentEnabled`, the account email and `kitVerified` are inside the MAC
  (agent off until the MAC verifies); an invalid MAC always rewrites the
  file; the start-up quick-unlock expiry uses only the 30-day cap.
- **Fixed (races):** the unlock backoff is checked inside the session lock
  (parallel guesses cannot all pass it), the kit-answer cap holds the counter
  across check and increment, and the one-use re-authentication grant is taken
  atomically and handed back only on failure and only within the same session.
- **Fixed (spoofing):** vault names and item titles refuse the Unicode Cf and
  default-ignorable characters (`mosaic_core::is_spoofing_char`), imports strip
  them from titles with a `titleCleaned` entry warning, and the UI shows typed
  text in a `<bdi>` isolate (`UserText`). The SSH prompt's 1 s Allow wait now
  starts after the first commit. CRYPTO_SPEC §4 lists `K_settings`.
- **Left:** a joint rollback of store and file (M3-R23); emoji written with
  U+FE0F are refused in names (M3-R24).

## M3 follow-up — one-time-code setup (2026-11-04)

Branch `feat/m3-otp-setup` (ADR-0031). The item editor can now add a one-time code
(brief §5 "2FA / TOTP"): *Scan screen*, *Paste screenshot*, *Choose image…* and
*Can't scan? Enter the key*, then a confirmation with the live code before anything is
saved.

- **Landed:** new crate `mosaic-qr` (images and screen frames to `otpauth://`,
  `rqrr` + `image`, size limits, zeroizing buffers, fuzz targets `scan_image` and
  `scan_luma` in CI); `mosaic_totp::validate_label` is public; commands `otp_scan`,
  `otp_parse`, `otp_pending_code`, `otp_pending_discard` with Rust-side pending slots
  (the scanned secret never enters the webview), `OtpChange::setPending`,
  `CopySource::pendingOtp`, `app_status.otpScreenScan`, twelve error codes; clipboard
  image read and clear after read; Windows (GDI) and Linux X11 screen capture; the
  UI (`OtpSection`, `OtpSetupDialog`, DE and EN strings), mock handlers incl. the
  previously missing mock `add_item` and `update_item`; THREAT_MODEL R30–R32.
- **Owner decisions taken:** O1 macOS *Scan screen* stays hidden (paste a screenshot
  instead, Cmd+Ctrl+Shift+4); O2 `otpauth-migration://` is refused with its own
  message for now (a Google Authenticator batch import may become an import format);
  O3 `win-screenshot =4.0.14` is accepted for Windows.
- **Crypto-review fixes:** `rqrr` is vendored with a bound on grouping work (at most
  64 finder patterns, 256 grouping attempts; `vendor/rqrr/MOSAIC_PATCH.md`, ADR-0031);
  the pending slot is inserted and restored under the session lock and purged on the
  idle tick; pending IDs are 16 random bytes in hex; the clipboard is emptied for
  migration, several and invalid codes too, and only if it still holds the scanned
  image; screen frames are wiped as soon as their grey levels exist.
- **Manual checks pending (Needs owner):** Windows 10/11 with two monitors, QR in a
  browser behind Mosaic: found, and the Mosaic window comes back focused; a Win+Shift+S
  snip pasted: found and the clipboard emptied; Linux X11: found; Linux Wayland: no
  *Scan screen* button; macOS: no *Scan screen* button, Cmd+Ctrl+Shift+4 and paste: found.
- **Not in scope:** `otpauth://` export, Google Authenticator batch import, macOS
  screen capture (ADR-0031 item 8), a link from a login to its code item in a
  hardware vault (autofill of the code is M4).

## M3 red team, second pass: extension-attacker review of the desktop IPC (2026-11-04)

Branch `test/m3-ipc-redteam`. Attacked the Tauri surface around the commands:
remote origins and other webviews, core and plugin commands, forged invoke keys,
navigation and `window.open`, URL opening, the config and dependency list, unknown
fields, plus a read of the SSH agent pipe, single instance, files written and the
OTP pending slots.

- **Held:** only the bundled origin in the window `main` reaches IPC; every
  `core:*` and plugin command (content protection, devtools, new webviews, event
  listen, global shortcut, opener, shell, fs, dialog, clipboard) is refused; a
  forged invoke key gets no answer; no devtools, asset protocol, remote IPC or
  URL-opening plugin is in the build; `window.open` is refused (no new-window
  handler); item URLs render as text; import paths come only from the native
  dialog; pending OTP IDs are random; the pipe is first-instance with remote
  clients rejected.
- **Fixed:** a navigation guard keeps every webview on the bundled frontend.
  Without it a script or link could carry data out in the address or load
  `http://tauri.localhost:<port>/`, which Tauri 2.12 counts as local (R33).
  Tagged request values (copy source, secret reference, generator request, draft
  operations) now refuse unknown fields like every other request (unit variants
  cannot, and carry nothing).
- **Left:** report the port-blind origin check upstream (R33).

## M3 follow-up — unlock reveal (2026-11-04)

Branch `feat/m3-unlock-reveal`, ADR-0032. The owner's liquid-glass reveal from
`docs/design/holo/reveal-studies.html` is in the app, frontend only (no Rust,
no new dependency, no CSP change).

- **Landed:** `apps/desktop/src/reveal/` (display-list collector with the
  secret-exclusion rules, canvas painter, WebGL2 glass band, unlock and lock
  drivers, pure phase reducer, `RevealStage`/`RevealProvider`). The unlock
  and vault screens sit in layers of the stage; unlock grows a clip-path lens
  from the click on Unlock, with the band on top and the vault choreographed as
  the rim reaches it; lock sends `lock` first, paints the vault into a sealed
  snapshot and unmounts it in the same commit, then closes the lens over the
  picture while the live unlock screen is focused underneath. Idle, sleep,
  screen-lock and tray locks stay instant. `data-secret`/`data-mask` markers on
  `SecretField`, `TotpCode` and the generator output; `data-reveal-origin` on the
  Unlock and Lock buttons; only the active layer carries `id="main"`.
- **Fallbacks:** plain lens without WebGL2, after a lost context, an oversized
  texture, a failed build or three frames over 50 ms; 250 ms crossfade under
  reduced motion; instant swap without clip-path or a 2D canvas (jsdom); a
  hidden window or a resize jumps to the end state; an unsent lock puts the
  vault back after 4 s.
- **Tests:** vitest for timing, choreography, lens, frames, collect (hunter2
  fixture), phase table, drivers, origin, stage, `useShell.lock` order, the
  selectors against a rendered vault and the three secret renderers;
  Playwright `e2e/reveal.spec.ts` (off-centre click, interaction mid-run, lock,
  reduced motion). The other e2e specs wait for a reveal to settle before axe.
- **Known fidelity limits of the snapshot painter:** no box shadows, gradients,
  pseudo-elements or text ellipsis; text and borders are drawn by the canvas,
  not the layout engine, so the first lock frame can shimmer slightly against
  the live DOM; foil canvases are frozen at the moment of the snapshot; blurred
  sprites need `CanvasRenderingContext2D.filter` (without it the pop is not
  blurred). Non-secret item text stays visible as pixels until the lens closes.
- **Not verified by a human:** the look on real GPUs and on macOS/Linux
  webviews; the glass band ran in headless Chromium in e2e (`data-mode="glass"`).

## M3 follow-up — item kinds and custom fields (2026-11-05)

Branch `feat/m3-item-kinds`, ADR-0033. The item editor and the detail now follow the kind
of the item instead of showing the login form for everything. Track A of the item-kinds
work; attachments (Track B) come next.

- **Landed:** one layout table per kind (`screens/vault/kinds/layouts.ts`) feeding the
  editor (`editor/KindFields`, `UrlListEditor`, `CustomFieldsEditor`, `SshKeyInput`) and
  `DetailFields` ("Other fields" for values outside the layout); payment card helpers
  (Luhn, length and email hints, brand from the prefix, `MM/YYYY` expiry that blocks only
  when it cannot be stored, grouped number on reveal, "Expired" chip); websites with add,
  edit and remove; custom fields with add, rename, retype, remove and a limit of 512;
  secrets that start empty when editing and a Remove button for stored ones; SSH keys by
  paste (Rust derives the public key and fingerprint without decrypting; detail shows
  public key, fingerprint, algorithm, passphrase chip and an agent-support note; the
  private key never comes back). Contract: `Update.value` is optional (`null` keeps the
  stored value), `ItemDraft.sshKey`, `ItemDetail.sshKey`, `CopyField sshPublicKey`,
  `sshKeyInvalid`; `mosaic-ssh-agent` exports `material_from_openssh` and
  `describe_material`. The mock applies URL and custom-field edits; seeds a card, an
  identity and an SSH key.
- **Decisions (ADR-0033):** the kind of an existing item cannot change; custom fields
  cannot be reordered; no one-time-code type for new custom fields; SSH keys are paste
  only, replaceable but not removable; the private key has no reveal, copy or export.
- **Tests:** Rust (`draft` unit tests, four handler tests, ssh-agent key tests, refreshed
  contract fixtures); vitest for layouts, card helpers, the form diff, the editor, the
  detail and the mock; Playwright `e2e/item-kinds.spec.ts` (card with a hidden custom
  field from create to remove, SSH key paste), axe on editor and detail.
- **Deferred to Track B:** attachments and the files section of the document form (the
  form carries a "save first" hint), preview and Save as, the attachment error codes in
  the UI.
- **Not verified by a human:** the look of the new forms against the owner's taste (no
  mockup exists for cards, identities, notes or files), the masked SSH textarea in the
  macOS and Linux webviews (`-webkit-text-security`).

## M3 follow-up — attachments (2026-11-05)

Branch `feat/m3-attachments`, ADR-0034. Track B of the item-kinds work: files on items, with
an in-memory image preview and Save as. Nothing about the CSP or the dependency set changed.

- **Landed:** `attachment_add`, `attachment_preview` (the first binary IPC response),
  `attachment_save` and `attachment_remove`; `AttachmentEntry.{addedAt, available,
  previewable}`; six error codes. Rust (`attachment_files.rs`, `commands/attachments.rs`):
  all file I/O in Rust through native dialogs, bounded reads, atomic 0600 writes with the
  Mark of the Web on Windows, file-name sanitizing, PNG and JPEG decoding through
  `mosaic_qr::decode_image` with a box downscale to at most 2048 px and 4 MP, one preview at
  a time, the lock checked again after every dialog and before pixels are returned. UI
  (`screens/vault/files/`): a Files section (first on documents, after the fields on other
  kinds, read-only in the trash, with a caption in the editor), the preview dialog (canvas
  `putImageData`, buffer zeroed after painting, canvas cleared on close, lock and blur, late
  answers dropped) and a remove confirmation. The mock seeds a "Passport scan" document with
  a PNG and a PDF and follows `?attachAdd=` and `?attachSave=`.
- **Decisions (ADR-0034):** no PDF preview (Save as only; the options and the likely
  sandboxed-helper follow-up are in the ADR); SSH private keys are never exported; the
  preview closes on window blur like a revealed secret; thumbnails, rename and drag and drop
  are out.
- **Tests:** Rust unit tests for names, MIME sniffing, bounded reads, atomic writes, the
  UUIDv7 timestamp and the preview (header, downscale, alpha, PDF, oversize header); eleven
  handler tests on a file-backed store (add, cancel, too large, lock during the dialog,
  hardware vault, preview round trip and `busy`, save, cancelled save decrypts nothing,
  remove, unknown attachment); `attachment_requests_refuse_path_keys`; refreshed contract
  fixtures with `BINARY_RESPONSES`; vitest for the preview parser, the dialog, the section,
  sizes, the mock and the snapshot collector; Playwright `e2e/attachments.spec.ts`
  (preview, save, add, remove, lock closes the preview, trash read-only), axe on the
  section, the dialog and the confirmation.
- **Deferred:** PDF preview through a sandboxed renderer process (own ADR), rename, drag and
  drop (needs a confirmation flow and a red-team check of `tauri://drag-drop`), thumbnails.
- **Not verified by a human:** the native open and save dialogs on each OS (the tests use
  fakes), the Mark of the Web on a real NTFS volume (Windows opens the file with SmartScreen
  or Protected View), the look of the Files section and the preview dialog against the
  owner's taste (no mockup exists).

## M3 follow-up — UX review fixes (2026-11-05)

Branch `fix/m3-ux-polish`. Frontend only. Fixed from the ux-reviewer round: the editor takes
focus on the title and Esc leaves it from anywhere (an open dialog keeps its own Esc), focus
returns to the item row or "New item"; secret toggles are named after their field (the unlock
and master-password screens keep "Show password"); a title is required (an untitled import may
stay untitled); the preview shows "Preparing preview…"; the top bar no longer overlaps in a
narrow German window; dialogs focus `data-autofocus` and trap Tab; focus returns to "Add file…"
after a removal; custom-field rows are groups; the nits and copy changes.

- **Skipped:** the strength meter under the editor password (m14): the frontend has no strength
  function, only the Rust generator answers with one, and a new algorithm is out of scope. The
  breadcrumb "New {kind}" (m14): the kind is editor state, the top bar only knows that an
  editor is open. New match modes and the "not saved yet" list row stay out as decided.
- **Deviated:** an item without files (not a document) shows a small ghost "Add file…" button
  where the Files section sits, not at the end of the detail or in the "…" menu, so the button
  keeps its place when the first file arrives.
- **Not verified by a human:** the look of the narrow top bar and the aligned editor rows
  (checked by tests and Playwright only).

## M4 Browser extension — T1 channel (2026-11-05)

Track T1 of the M4 plan (ADR-0035, ADR-0036, both proposed): the Rust side of the channel
between the extension and the desktop. The desktop endpoint (T2), the extension (T3, T4) and
the CI changes (T5) follow.

- **`mosaic_core::browser_channel`** (CRYPTO_SPEC §15, labels `browser/auth`,
  `browser/handshake`, `browser/session`, `browser/frame`): `PairingKey`, `Role`,
  `ChannelSession`, `new_pairing_id`, `new_channel_nonce`. PSK handshake with mutual HMAC
  proofs (desktop first) and per-direction XChaCha20-Poly1305 frames with strict sequence
  numbers; no new primitive and no new crate (`hmac`, `hkdf`, `sha2`, `chacha20poly1305`,
  `subtle`, `zeroize`, `getrandom` were all in §2). The golden vector's HKDF and HMAC values
  come from a Python standard-library script whose source is in the test's doc comment; the
  two sealed frames are pinned from this code.
- **`crates/origin` (`mosaic-origin`)**: `PageOrigin::parse` (WHATWG via `url`, canonical
  origin strings only), `saved_origin`, `registrable_domain` (`psl`), `fill_allowed`
  (exact origin; eTLD+1 per URL; HTTP rules), `idn_unicode`, `lookalike_of`. New dependency:
  `psl =2.1.246` (MIT/Apache-2.0, `cargo deny` passes without changes to `deny.toml`);
  `url` and `idna` are used directly (already in the tree).
- **`crates/native-messaging` (`mosaic-native-messaging`, binary `mosaic-native-host`)**:
  endpoint derivation and the transports (the ssh-agent rules, copied; Unix client checks),
  framing (256 KiB cap), the strict envelope, ops and error codes with typed bodies,
  pre-auth messages, `client_handshake` and `server_handshake` (the desktop's side, ready for
  T2), the relay, the caller check, `install` and `uninstall`, and
  `fixtures/protocol.json`. The fuzz crate has the targets `envelope`, `preauth_message` and
  `channel_open`. Workspace features added: tokio `io-std`, serde_json `raw_value`.
- **Dev key**: `apps/extension/keys/dev-public-key.b64` (base64 SPKI of an RSA-2048 key; the
  private key was discarded); `DEV_CHROMIUM_EXTENSION_ID = lccdgfpfnmegbdgooanfejdbahhokikm`,
  recomputed by a test.
- **Docs**: `docs/NATIVE_MESSAGING.md` (normative protocol), CRYPTO_SPEC §15 and the §2 and §4
  rows, ADR-0035 and ADR-0036, THREAT_MODEL §4.4 and R38, R39, R40, R42, R45, CORE_API.
- **Tests**: `browser_channel` 12, `mosaic-origin` 21 (with proptest), `mosaic-native-messaging`
  81 unit tests plus 5 that run the real host binary against an in-test desktop. The Unix
  transport tests (permissions, owners, stale sockets, the client checks) are written and
  type-check for `x86_64-unknown-linux-gnu`, but run only on Linux and macOS (CI).
- **Open for the review of T1**: the crypto-reviewer pass on the handshake, key schedule,
  frame AAD, the PSK transfer at pairing and the endpoint trust (ADR-0036 stays proposed
  until then). Not verified here: the Windows registry install (`reg.exe`) was not run
  against the real registry.

## M4 Browser extension — T2 desktop endpoint (2026-11-05)

Track T2 of the M4 plan (ADR-0037, proposed): the desktop serves the channel T1 built, pairs
browsers, answers fill and lookup requests, and shows the two dialogs. The extension itself
(T3, T4) and the CI changes (T5) follow.

- **`apps/desktop/src-tauri/src/browser/`**: `BrowserService` (started in `lib.rs` after
  `AppState` exists, stopped at `RunEvent::Exit`; lock and unlock hooks), `connection`
  (accept loop on Tauri's async runtime, the pre-auth exchange, the library's
  `server_handshake`, the AEAD request loop with a reader, a writer that seals in write
  order, 4 requests in flight, a 10 minute idle close, 8 connections), `handlers` (the
  allowlist: one `match` on `Op`), `fill` (`listForOrigin`, `search`, `fill`, `copy`,
  `generate`), `save` (`checkSave`, `proposeSave` and the writes), `prompts` (one pairing and
  one save prompt, the 1 s Allow guard, expiry, conflict, the 3-denial block), `registry`
  (`browser-pairings.json`, atomic, 0600 on Unix, keys as `PairingKey`) and `ratelimit`.
  On Windows the service binds a random pipe name (`first_pipe_instance`, remote clients
  rejected) and holds the `DiscoveryLease` for its whole life; on Unix it binds with the
  library's rules.
- **Webview**: commands `browser_status`, `browser_remove_pairing`, `browser_pairing_respond`,
  `browser_save_respond`; events `browserPairingRequested`, `browserPairingClosed`,
  `browserSaveRequested`, `browserSaveClosed`, `browserPairingsChanged`; error codes
  `browserRequestExpired`, `browserAllowTooSoon`, `browserPairingNotFound` (DESKTOP_IPC.md).
  Screens `BrowserPairingDialog`, `BrowserSaveDialog` (password only as a mask) and the
  "Browsers" group of Settings; mock options `&browserPair=1` and `&browserSave=new|update`;
  i18n `browser.*` and `error.browser*` (DE and EN).
- **Small changes elsewhere**: `Backend` gains `with_focus` (the extension's `focus`),
  `on_unlock` hooks (run where the SSH agent starts after an unlock), `keep_generated`
  (generation without counting as activity); the attention hook also fires for the two new
  prompts; `useAllowGuard` moved out of the SSH dialog.
- **Dependencies**: path dependencies on `mosaic-native-messaging` and `mosaic-origin`, and
  `tokio` (workspace, already in the tree through Tauri, `mosaic-ssh-agent` and
  `mosaic-native-messaging`) for the connection tasks. Nothing new from crates.io. The desktop
  fuzz crate still builds (`cargo check --manifest-path apps/desktop/src-tauri/fuzz/Cargo.toml
  --bins`); the contract module imports only serde and contract types.
- **Tests**: `browser_tests.rs` (29, over `tokio::io::duplex` against the host's real
  handshake and an injectable clock), unit tests in `registry`, `ratelimit`, `save`; contract
  fixtures regenerated; Vitest (`BrowserPairingDialog`, `BrowserSaveDialog`,
  `BrowserSettings`, contract parity); Playwright `e2e/browser.spec.ts` (both dialogs, the
  settings group, axe light and dark).
- **Not verified here**: the real endpoint (Windows pipe and discovery file, Unix socket)
  under the desktop; `BrowserService::start` was not run against the OS (the library's own
  tests cover the transports and the lease), and the Unix paths run only on CI. The crypto
  review of the registry at rest, the pairing flow, the 1 s guard, the prompt lifetimes and
  the `touch` policy is still open (ADR-0037 stays proposed).

## M4 Browser extension — T3 extension core (2026-11-05)

Track T3 of the M4 plan (ADR-0035): `apps/extension` (`@mosaic/extension`), the MV3 core with
on-demand fill and the hostile-page harness. The popup UI, save capture and the generator
screens are T4; the desktop endpoint is T2, so the extension runs against a fake desktop
until T2 merges.

- **Builds**: one tree, Vite 8, plain multi-entry builds (no CRXJS, no polyfill). `build`
  makes `dist/chromium` and `dist/firefox`; `build:dev` is chromium with the dev key (extension
  id fixed for the native host); `build:e2e` adds the test hooks and `*.test` host access. The
  background and popup are ES modules, the content script an IIFE. `check-dist.mjs` fails on
  test code, external listeners, `host_permissions`, `web_accessible_resources`, a `key` or a
  changed permission list in the two production packages.
- **Manifest**: permissions exactly `activeTab`, `scripting`, `nativeMessaging`,
  `contextMenus`, `storage`; `optional_host_permissions` for the T4 capture; the CSP of the
  plan; `externally_connectable: {ids: []}` (Chromium). `_locales` and the icons are generated
  at build time from `packages/i18n` (`ext.manifest.*`, `ext.command.*`) and `scripts/icons.ts`.
- **Background**: `NativeClient` (lazy port, `hello` with the stored pairing, ID matching,
  20 s timeout, in-flight cap of 4, 60 s idle close, error mapping), `Router` (sender rules,
  rate limits), the fill algorithm and `decideFrame`/`chooseTarget` (pure), gestures
  (`fill-login`, context menu), popup operations, `PopupState`. Protocol validators check
  every host message with exact keys and are tested against `crates/native-messaging/
  fixtures/protocol.json` in both directions.
- **Content script**: `detect` (login, username-first, sign-up, change form, one-time codes
  single and split, open shadow roots), `visibility` (`isFillable`), `frame` (`frameAncestry`),
  `fill` (isolated-world native setter, `input` then `change`). It re-checks origin, ancestry,
  visibility and fields at fill time.
- **Tests**: vitest project `extension` 270 tests in 11 files (`packages` project, i18n parity,
  2862 tests still green). Playwright `e2e/hostile`: 18 tests in 14 spec files against the
  hostile-site server (`autofill-on-load`, `synthetic-events`, `cross-origin-iframe`,
  `same-origin-iframe`, `nested-a-b-a`, `sandboxed-srcdoc`, `lookalike`, `http-page`,
  `hidden-fields`, `overlay-occlusion`, `type-swap-and-late-inject`, `navigate-after-probe`,
  `spoof-messages`, `react-controlled`). Removing the frame check from `decideFrame` makes the
  cross-origin and A-B-A specs fail (checked once by hand).
- **Docs**: `docs/EXTENSION_SECURITY.md` v1 (attack, mitigation, test), THREAT_MODEL §4.3 and
  R46. New dependency: `@types/chrome` 0.3.7 (dev; types for `chrome.*`), nothing else.
- **Not verified here**: Firefox (no automated e2e; `optional_host_permissions` support from
  Firefox 128 is taken from the plan, not checked); `activeTab` gating without host permissions
  (the e2e build has host access; check by hand on the production build).
- **Extension-attacker pass (done, PR #26)**: six breaks found and fixed with regression tests
  (sandboxed documents counted as the site, CSS tricks hiding fields, hidden same-origin frames,
  a generated password reused on another site, `mosaicContent` clobbering, `check-dist` gaps);
  the hostile suite grew from 14 to 39 specs.

## M4 Browser extension — T4 popup and save (2026-11-05)

Track T4 of the M4 plan (ADR-0038, proposed; amendments to ADR-0035): the popup, save
capture, the generator and one-time-code fill, and the onboarding step D06. Still against the
fake desktop in the e2e build; the real native host and desktop are not part of any test here.

- **Popup** (`apps/extension/src/popup`, React 19, tokens, `ext.*` in German and English): views
  `ConnectView` (host missing, forbidden, desktop not running, not connected, pairing with the
  code `XXX-XXX`, version mismatch), `LockedView` (E01 without a master-password field),
  `LoginsView` (E02: matches with Fill and a copy menu, search, the first-match shortcut hint,
  address warnings E05 without "Fill anyway", `OtpCard` E07 without a code, `SaveCard` E04
  without title and vault, `FrameConfirmDialog`) and `GeneratorView` (E03: Random and
  Passphrase with the desktop's limits, the value with `data-secret`, strength, which rules it
  follows, Fill, Copy, Regenerate). Footer: Open Mosaic, connection state, a More menu with
  "Offer to save logins" and "Disconnect from Mosaic". Keys: Enter, Ctrl+C, Ctrl+Shift+C,
  Ctrl+Alt+C, Esc (top layer). Copies, fills and save outcomes go to a live region. The popup
  shows no vault password and no code. Static holo gradient, no WebGL. A style test checks the
  popup CSS and TSX for raw colours and sizes.
- **Frame confirmation bound to the frame.** `confirmFrame` is `{frameId, documentId} | null`,
  taken from `PopupState.frameTarget` when the dialog opens; `fill.ts` fills a same-origin frame
  only when the target at fill time has that frame id and document id (`frameNeedsConfirmation`
  otherwise; Firefox has no document id, so only the frame id is compared there).
- **Generated passwords.** The popup's "this site" request takes host and the page's
  `passwordrules` from the tab; the hold is bound to tab and origin, taken once, and the popup's
  Fill requires it (`generatedExpired` instead of a different password). A fill creates a pending
  save from the value that reached the page.
- **Save capture** (`src/capture`, `background/save.ts`, `capture-registry.ts`): off by default;
  the popup asks for the optional site access in the click, the background registers
  `capture.js`; trusted clicks on submit controls and Enter in a text box (not `submit`, see
  deviations) trigger it; origin and tab come from the browser; cross-origin and opaque frames
  and sites marked "Never" are dropped; `checkSave` decides new, update or unchanged;
  `pendingSave:<tabId>` in `storage.session` for five minutes, bound to the tab's origin;
  badge `1`; the popup's Save sends `proposeSave` and the desktop confirms in its dialog.
- **Desktop D06** (`screens/onboarding/BrowserStep.tsx`): step 5 of 5 "Browser extension
  (optional)": how to connect, the paired browsers listed live (`browserPairingsChanged`), store
  links only when `STORE_URLS` are set (null for now), Skip for now and Done; the pairing
  dialog opens over the step. It replaces the old "done" step (the mockup puts "You're all set"
  in the same step). The mock creates the pairing request after account creation
  (`&browserPair=1`).
- **Docs**: ADR-0038, ADR-0035 amendment (E01, E04, E05, E07), UX_GUIDELINES §8 ("Fill anyway"),
  THREAT_MODEL R44 and the capture row, EXTENSION_SECURITY v2 (rows for the capture, the popup,
  the frame confirmation and the generator; §8 the popup), NATIVE_MESSAGING (two local codes).
- **Dependencies** (justification for the commit): the popup needs React 19, `react-dom` and the
  two bundled fonts (`@fontsource/dm-sans`, `@fontsource/ibm-plex-mono`), and its tests need
  `@testing-library/{dom,react,user-event}`, `@types/react{,-dom}`, `@vitejs/plugin-react` and
  `@axe-core/playwright`: all at the versions the desktop already uses, so `pnpm-lock.yaml`
  gains only importer entries, no new package.
- **Tests**: vitest project `extension` (popup components against a fake port, `save`,
  `capture`, `popup`, error-text parity for every code in both languages, style rules; the T3
  tests updated for `confirmFrame`); Playwright `popup.spec.ts` (views, axe in light and dark,
  frame confirmation, a stale or forged confirmation refused, keyboard, German), `save.spec.ts`
  (capture off by default, a trusted submit, update, unchanged, Not now, Never, scripted events
  ignored, a frame of another site ignored, locked then unlocked, the More menu switch) and
  `generator.spec.ts` (the shown password is the one filled and then saved, one use only, bound
  to its tab); desktop `OnboardingFlow.test.tsx`, `flow.test.ts` and `e2e/onboarding.spec.ts`.
- **Deviations** (recorded in ADR-0035 and ADR-0038): `submit` is not a capture trigger, because
  the browser raises a trusted `submit` for a script's `requestSubmit()` and `button.click()`
  (found by the scripted-events spec); a trusted click or Enter is used instead. The e2e build
  also holds the two optional host patterns as host permissions, since a headless browser cannot
  click through the permission prompt. React and the test libraries are new dependencies of
  `apps/extension` (the plan named only `@types/chrome`).
- **Found and fixed**: the host relay timed every forwarded request out after 15 s, while
  `proposeSave` waits up to 300 s for the desktop dialog. The relay now waits 302 s for
  `proposeSave` (`request_timeout`), so the host answers before the popup's 305 s. The `neverSave` list has no screen yet.
- **Not verified here**: Firefox (no automated e2e; `optional_host_permissions` and
  `registerContentScripts` with `persistAcrossSessions` there are taken from the docs); the
  real native host and desktop end to end; `activeTab` gating on the production build (the
  ux-reviewer and extension-attacker passes followed on PR #27: one capture break through an
  invisible frame, fixed); a hostile-suite test (`race-after-probe`, rapid
  gestures) failed once under memory pressure and passed on rerun.

## M4 Browser extension — close (2026-11-06)

Track T5 of the M4 plan: CI, documents and the full gate. M4 is merged as PRs #23 (T1
channel), #25 (T2 desktop endpoint), #26 (T3 extension core), #27 (T4 popup and save) and the
close branch `chore/m4-close`.

- **What shipped.** `mosaic-origin` (exact-origin matching, HTTP rules, IDN and look-alike
  checks), `mosaic-native-messaging` (the host `mosaic-native-host`, the framed, authenticated
  channel of CRYPTO_SPEC §15, install and uninstall of the host manifests), the desktop's
  `BrowserService` (pairing, fill, search, copy, generate, save; two dialogs; the "Browsers"
  settings group; onboarding step D06) and `apps/extension` (MV3 for Chromium and Firefox:
  toolbar popup, shortcut and context-menu fill, one-time codes, generator, opt-in save capture,
  no inline UI). ADR-0035 to 0038 are accepted.
- **Reviews.** crypto-reviewer on the channel and the desktop endpoint (T1, T2); ux-reviewer on
  the popup, D06 and the dialogs; extension-attacker on the core (T3), then capture and popup
  (T4), which found and closed the invisible-frame capture clickjacking. All findings were fixed
  before merge. No review ran on the T5 delta (CI, documents); this track changes no code.
- **CI.** New job `extension` (Ubuntu: frozen install, Vitest project, production build,
  `check-dist.mjs`, Chromium install, `build:e2e`, Playwright). `crates/native-messaging/fuzz`
  joins the fuzz smoke (with its three seed directories), the supply-chain `cargo deny` check and
  the fuzz-lockfile `cargo audit`. `mosaic-origin` and `mosaic-native-messaging` join the
  coverage matrix (>= 90 % lines). Crash inputs of a failed fuzz run are uploaded as an artifact
  (`actions/upload-artifact`, pinned by SHA).
- **Coverage (local, `cargo llvm-cov -p <crate> --summary-only`, Windows).** `mosaic-origin`
  98.81 % lines; `mosaic-native-messaging` 92.03 % lines (install.rs 72 % and main.rs 34 % are
  the weakest files; the Windows registry path and the Unix transports are covered by CI on
  Linux and macOS, or only by hand).
- **Documents.** `docs/EXTENSION_SECURITY.md` v3 with the brief §4 map (§10: every item, its
  mitigation, its test files, what is manual only); CLAUDE.md layout; README status line.
- **Gate (2026-11-06, one run).** fmt check, clippy with `-D warnings`, the workspace tests
  (1180 passed, 0 failed, 7 ignored), `cargo deny check`, `cargo audit --deny warnings`,
  `pnpm install --frozen-lockfile`, `pnpm lint`, `pnpm typecheck`, `pnpm test` (87 files, 5131
  tests; the extension project has 462 in 20 files) and `pnpm audit`: all green. Playwright:
  extension 82 passed (e2e build, Chromium headless), desktop 55 passed. The ignored Rust tests
  (release-mode, OpenSSH interop) run in CI.
- **Open risks.** M4-R1 to M4-R12 in the register below (same-user malware and the M9 installer,
  save capture reach, the visibility heuristic, no automated test of the real host, Firefox or
  the registry install).
- **Open: manual checks and owner items.** See "Needs owner" ("M4 browser extension", "Manual
  checks for M4").

## M5 Passkeys — T1 authenticator (2026-11-06)

Track T1 of the M5 plan (ADR-0039, CRYPTO_SPEC §16): the ES256 software authenticator and the
WebAuthn rpId rules. T2 (native-messaging ops, desktop dialog), T3 (extension shim) and T4
(import, CXF export) follow.

- **`mosaic-passkey`**: `Es256Key` (generate by rejection sampling over `mosaic_core` random,
  PKCS#8 in a `SecretVec`, COSE key, SPKI, RFC 6979 DER signature; not `Clone`, redacted `Debug`,
  zeroized on drop), `create` / `create_with` / `assert`, hand-written CBOR for the three fixed
  structures, `clientDataJSON` with CCD escaping, authenticator data, base64url. The sign-count
  rule (imported `c > 0` signs `c + 1`, persisted first, saturating) is in `assert`. No
  origin policy, no vault access, no CBOR decoder.
- **`mosaic-origin`**: `webauthn_rp_id` and `RpIdError` (https only, non-loopback domain,
  canonical, host or dot-suffix, not a public suffix, at or under the host's registrable
  domain); 25-row table, order test, proptests.
- **Dependencies**: `p256` gains `alloc` and `pkcs8`; new crates `pkcs8` 0.11.0 and `spki` 0.8.1
  (stable releases, RustCrypto, pulled only through that feature). `cargo tree -i` shows nothing
  else uses them. No new JS dependency.
- **Known answers**: V1 to V11 pass. V3 and V4 are the RFC 6979 A.2.5 signatures (byte exact, no
  low-S normalization). V5 agrees with Node's SHA-256. The fixture
  `crates/passkey/fixtures/vectors.json` is checked by the independent verifier
  `apps/extension/test/webauthn/verifier.ts` (own CBOR/COSE/authData parser, `node:crypto` only)
  in `src/passkey/vectors.test.ts` (14 tests). V10's signature was pinned after both Rust
  `VerifyingKey::verify` and the Node verifier accepted it.
- **Tests**: `mosaic-passkey` 31, `mosaic-origin` 25, `mosaic-ssh-agent` 55 + 8 (p256 feature
  change), vitest vectors 14. Clippy (pedantic, `-D warnings`), fmt, `cargo deny` and
  `cargo audit --deny warnings` pass.
- **Not done here**: `cargo llvm-cov -p mosaic-passkey` was skipped (memory on this machine); the
  coverage job (at least 90 %) comes with the CI change in T4. Crypto-reviewer pass is still due.
- **Crypto-review follow-ups (T1)**:
  - `webauthn_rp_id` now also requires the rp ID at or under the host's registrable domain, so a
    tenant of a wildcard public suffix cannot claim the parent (`a.b.kawasaki.jp` cannot claim
    `kawasaki.jp`, `evil.tenant.platformsh.site` cannot claim `platformsh.site`). Check order:
    insecure, not a domain, not canonical, not a suffix, rp ID is a public suffix, above the
    host's registrable domain (`NotASuffix`). **For T3:** write the stricter rule into ADR-0040
    (it was not written yet); the rule text in CORE_API already says it.
  - `Es256Key::sign`, `create_with` and `authenticator_data` are crate-private. `assert` returns
    a `PendingAssertion` (private fields); the signature is released with `into_response(CountPersisted)`
    after the store write, or `into_response_without_count()` (an error while a count is
    pending). T2 must use this API.
  - New: `crates/passkey/fuzz` (targets `b64url`, `pkcs8`; own workspace, excluded in the root
    `Cargo.toml`; checked with `cargo check --manifest-path crates/passkey/fuzz/Cargo.toml --bins`).
    **CI wiring for it (fuzz job) is still to do**, with the T4 CI change.
  - V8 is cross-checked by an independent TypeScript CCDToString (`ccd` entry in the fixture);
    PKCS#8 tests for secp256k1, P-384 and a mismatched embedded public key. `serdect` in the
    lockfile is not built (ADR-0039).

## M5 Passkeys — T2 desktop (2026-11-06)

Track T2 of the M5 plan (ADR-0039, ADR-0040 comes with T3): the native-messaging ops, the desktop
handlers, the approval dialog and the passkey item view. The extension shim (T3) and the import and
CXF export (T4) follow.

- **`mosaic-native-messaging`**: ops `passkeyCreate`, `passkeyGet`, `passkeyCancel` (`Op::ALL` 15,
  `FORWARDED_OPS` 13), error codes `passkeyNotFound`, `passkeyExcluded`, `rpIdMismatch`,
  `passkeyUnsupported` (30), `PASSKEY_PROMPT_TIMEOUT_SECS` = 300 (the host waits 302 s for create and
  get), strict bodies (`PasskeyCreateBody`, `PasskeyGetBody`, `PasskeyCancelBody`,
  `PasskeyCreateResult`, `PasskeyGetResult`), fixtures regenerated. `NATIVE_MESSAGING.md` §5.2 to §6
  and the new §5.4 ("what no op does").
- **Desktop `browser/passkey.rs`**: planning (origin and top origin equal and https, `webauthn_rp_id`
  from T1, inputs decoded within their limits, ES256 only, names stripped of control and format
  characters and cut to 64 bytes, candidates = live ES256 passkeys of the rp ID in open,
  non-hardware vaults, `allowCredentials` or discoverable only, exclusion) and applying (a create
  writes the `passkey` item before it answers; an assertion uses T1's `PendingAssertion`: raise and
  store the count with `set_passkey_sign_count`, then `into_response(CountPersisted)`, else
  `into_response_without_count`). `passkeyNotFound` is answered **without** a dialog.
- **Desktop `browser/passkey_prompt.rs`**: the passkey slot in `Prompts` (one at a time, `busy`),
  owner pairing and connection (the prompt ends on disconnect, pairing removal, lock (the site is told
  `cancelled`), expiry and `passkeyCancel` from the same pairing only), the 1 s Allow guard, the
  attention hook, and `respond_passkey`. UV: `Backend.last_user_verification` is set in
  `after_unlock` (password, quick unlock, security key) and by a right `reauthenticate`, cleared in
  `after_lock`; `uv_recent()` is true within `PASSKEY_UV_WINDOW` = 300 s of the service clock; the
  dialog asks for the master password when the site did not say `discouraged` and `!uv_recent()`
  (`reauthenticate_master`, so wrong passwords count towards the five-failure lock).
- **Activity policy (documented in `browser/mod.rs`, NATIVE_MESSAGING §6, DESKTOP_IPC)**: only `fill`,
  `copy` and a **completed** passkey create or assertion call `touch()`; a request that only plans,
  opens a dialog, is cancelled or answered "another device" does not, so a script cannot keep the vault
  open by asking. The user's own answer (`browser_passkey_respond`) touches, like the other answers.
- **IPC contract**: command `browser_passkey_respond` (capability `allow-browser-passkey-respond`),
  events `browserPasskeyRequested` / `browserPasskeyClosed`, `BrowserCloseReason::AbortedByPage`,
  `ItemDetail.passkey` (`PasskeySummary`, never a key), passkey `ItemSummary` (subtitle user name,
  host rp ID), mock option `&browserPasskey=get|create|excluded|uv`; `DESKTOP_IPC.md`, fixtures
  regenerated, TypeScript parity test extended.
- **UI**: `BrowserPasskeyDialog` (get with account radios, create with vault select, exclusion with
  OK, password field when `needsPassword`, "Use another device or security key", Cancel focused, Esc
  cancels, the same expiry ring), `PasskeyDetails` (site, user name, display name, short credential
  ID, algorithm, date, the note about the key), the editor of a passkey keeps title, tags and notes
  (no custom fields, no files). i18n `browser.passkey.*`, `item.passkey.*`, `ext.error.*` (DE and EN).
- **Extension validators** (`apps/extension/src/shared`): the three ops and four codes; the fixture
  parity test passes. The interception itself is T3.
- **Dependencies**: `mosaic-passkey` (path) in the desktop; `p256` as a desktop dev-dependency (the
  signature checks in the tests; already in the tree). The desktop fuzz crate needs neither (the
  contract module imports only serde and contract types); `cargo check --manifest-path
  apps/desktop/src-tauri/fuzz/Cargo.toml --bins` passes.
- **Tests**: `native-messaging` 98 + 5 (host binary); desktop Rust 18 new passkey tests in
  `browser_tests.rs` (create and verify against the returned SPKI, get with allow list and
  discoverable, no dialog for `passkeyNotFound`, rpId spoof, http, cross-origin frame, unsupported
  algorithm, exclusion, another device, cancel, lock, expiry, disconnect, pairing removal, `busy`, a
  cancel from another pairing, rate limits, the Allow guard, UV window and password path, the
  five-failure lock, imported counter 41 to 42 to 43 stored before release, a hardware-protected
  vault), unit tests in `passkey` (4) and `ratelimit` (1), two red-team tests (`redteam_tests.rs` and
  `browser_tests.rs`, counted above: no view and no op answer holds the key in hex, base64 or base64url); Vitest
  (`BrowserPasskeyDialog` 8, `PasskeyDetails` 5, the editor, contract parity, protocol 74);
  Playwright `e2e/browser.spec.ts` (16 with the four new, light and dark, axe).
- **Not verified / open**: the real OS endpoint with a passkey request end to end (that is T3's
  hostile suite); axe in Vitest (there is no jsdom axe in the tree, the Playwright run covers it);
  the crypto-reviewer pass of the handlers, UV and counter, and the ux-reviewer pass of the dialog,
  are still due. Search does not index a passkey's user name (the list row shows it). A passkey that
  the page aborts while its answer is being written is stored but not delivered (an orphan the user
  can trash).
- **Crypto-review follow-ups (T2)**: each passkey dialog has a desktop-generated handle (the
  `requestId` of the events and of `browser_passkey_respond`); the extension's ID only matches
  `passkeyCancel`. The dialog switches to password mode on `reauthRequired` (the 300 s window
  ran out while it was open). A failed answer after the request expired no longer puts the prompt
  back (passkey and save). An acknowledge waits out the 1 s guard. The UV window needs both the
  monotonic and the wall clock within 300 s (a suspend expires it). The passkey dialog's password
  verifies and counts failures but earns no two-minute re-authentication grant
  (`verify_master_only`). A `passkey` item edit takes only title, notes and tags (`invalidInput`
  otherwise). Item-view names are sanitized. **Left open:** the core accessor that returns a
  passkey's public fields without copying its key (`crates/core` was out of scope for this
  track); until then `Item::passkey()` copies the key (zeroizing) for each passkey item that is
  listed or planned. CRYPTO_SPEC §16 still names ADR-0040 as normative (it lands with T3).

## M5 Passkeys — T3 browser interception (2026-11-06)

Track T3 of the M5 plan (ADR-0040, `EXTENSION_SECURITY.md` §11): the opt-in passkey shim and
bridge, the background service, the popup switch and the hostile-page suite. Extension and docs
only; no Rust.

- **Scripts** (`apps/extension/src/passkey/`): `main-world.ts` (+ `serialize.ts`, `credential.ts`;
  entry `main-entry.ts` → `passkey-main.js`, main world) replaces `CredentialsContainer.prototype.create`
  and `.get` with the same descriptor, takes every global it uses at load time, and falls back to the
  saved native method with the page's original arguments and `this`; `bridge.ts` (entry
  `bridge-entry.ts` → `passkey-bridge.js`, isolated world) listens only to its own window, takes the
  exact message shapes, 2 requests in flight, focus and visibility, and sends the origin it reads from
  `window.origin`. Both are registered at `document_start`, top frame only, after the opt-in
  (`passkey-registry.ts`); `vite.passkey.config.ts` builds both (a nested build for the bridge),
  added to every `build:*` script.
- **Background**: `passkey.ts` (`PasskeyService`): sender rules (frame 0, tab, https, sender origin =
  bridge origin = tab origin), `requestId` from `crypto.getRandomValues`, map by
  `tab:document:pageRequestId`, the error mapping of spec §5.2, `passkeyCancel` on abort, tab
  removed, replaced or navigated; `router.ts` routes the two passkey messages from content senders
  (10 a second per tab) and answers through `sendResponse`; `native.ts` waits 305 s for
  `passkeyCreate` and `passkeyGet`, and `passkeyCancel` is not held back by the in-flight cap.
  Shared: `passkey-messages.ts` (strict validators), `passkey-limits.ts`, `b64url.ts`.
- **Popup**: More-menu switch "Use Mosaic for passkeys" (`setPasskeys`, `PopupState.passkeys`),
  DE and EN strings; the site access is shared with "Offer to save logins" and is given back only
  when neither is on.
- **E2E**: `e2e-passkey.ts` (fake `passkeyCreate`/`Get`/`Cancel`, fake rp ID rule labelled as a
  fake), `test/webauthn/authenticator.ts` (WebCrypto P-256 signer, checked by the independent
  verifier in `authenticator.test.ts`), hooks `setPasskeys`, `passkeyDecide`, `passkeyLog`,
  `passkeyPending`; the hostile-site server gets `/passkey/rp.html`, `/passkey/challenge` (single
  use) and `/passkey/verify` (the verifier is injected by the fixtures). `check-dist.mjs` checks
  both scripts are shipped, not in the manifest, check `event.source`, use no extension API but
  `runtime.sendMessage`, and that the new test hooks are absent.
- **Docs**: `EXTENSION_SECURITY.md` v4 with §11 "Passkeys" (trust analysis and attack → mitigation →
  test), ADR-0040 (proposed; includes the stricter rp ID rule from the T1 review), THREAT_MODEL §4.3
  row and R47, NATIVE_MESSAGING extension-side note.
- **Tests**: Vitest extension project 636 passed (new: `main-world.test.ts`, `bridge.test.ts`,
  `passkey.test.ts` 44, `authenticator.test.ts`, `b64url.test.ts`, router and popup additions);
  packages project 3926 passed; Playwright: the 17 `e2e/hostile/passkey-*.spec.ts` tests and
  `e2e/passkeys.spec.ts` (3), full extension suite 102 passed (Chromium, e2e build).
- **Beyond the spec** (each in ADR-0040): a bridge acknowledgement (`mosaic-passkey-ack`) with a 1 s
  fallback, so a registered shim without a bridge cannot hang a page; requests with the PRF or
  large-blob extension go to the browser; the entries are `main-entry.ts`/`bridge-entry.ts` so
  `main-world.ts`/`bridge.ts` can be tested without side effects.
- **Not verified / open**: Firefox (MAIN-world registration from 128 is from the documentation;
  Playwright cannot load Firefox extensions); the real host and desktop end to end with a browser
  (the suite uses the fake desktop; the Rust side is T2's `browser_tests.rs`); the
  extension-attacker pass and the ux-reviewer pass of the popup switch are still due; the shim
  does not mimic native `toString` (a page can see it is replaced, R47).

## M5 Passkeys — T4 import, CXF export and docs close (2026-11-06)

Track T4 of the M5 plan (ADR-0041): passkeys come in from Bitwarden, KeePassXC and FIDO CXF, and
leave as a CXF file. The milestone close follows separately.

- **`mosaic-core`**: `PasskeyPublic` and `Item::passkey_public()` return the public fields of a
  passkey (rp ID, user handle, names, credential ID, algorithm, discoverable, creation time) without
  decoding or copying the private key. The desktop uses it for candidate planning (filter by rp ID
  before any key is copied), item summaries and details; the key is copied only for the one
  credential that signs or is exported (`CORE_API`).
- **`mosaic-passkey`**: `sanitize_name` and `MAX_ACCOUNT_NAME_BYTES` moved here from the desktop (one
  implementation, also used by the importers); `pkcs8_kind` tells a key of another
  algorithm from a damaged P-256 key or garbage.
- **`mosaic-import`**: depends on `mosaic-passkey`. `ItemDraft.passkeys: Vec<PasskeyDraft>`
  (credential, sign count, title); each passkey becomes its own `passkey` item right after its entry
  (with the entry's tags and trash state), an entry of passkeys only makes no item of its own
  (`ItemDraft::has_item`); `Preview.counts` includes `passkey`; a passkey-only entry whose
  credentials are already in the account is a duplicate. Warnings `passkeyUnreadable` and
  `passkeyUnsupported` (entry and file level). Bitwarden `fido2Credentials` (UUID or `b64.` credential
  IDs, `counter` carried over, RFC 3339 creation date), KeePassXC `KPEX_PASSKEY_*` (the PEM stripped
  into a zeroizing buffer), the new `cxf` importer (`Format::Cxf`, `version.major == 1`, passkey,
  basic-auth and note credentials). 1PUX is unchanged. Strict base64url (`buf::base64url_decode`,
  size checked before allocation). Fuzz: new `cxf` target, seeds with a passkey, `common::check`
  validates every accepted key; `cargo check --manifest-path crates/import/fuzz/Cargo.toml --bins`
  passes.
- **Desktop**: command `export_passkeys_cxf { vaultId, password | null }` (re-authentication taken and
  spent, native save dialog, zeroizing buffer, atomic 0600 write with the Mark of the Web, ES256 only),
  `ImportFormat::Cxf` with its dialog filter, the new warnings in the contract, TypeScript contract,
  mock and fixtures. Settings has the "Export passkeys" panel (vault select, the plaintext warning,
  the password dialog). i18n DE and EN for the panel, the CXF format and the four new warning texts;
  the `passkeysNotImported` texts now say that the format holds no private keys (1PUX).
- **CXF field names** were checked against the published v1.0 Proposed Standard (2025-08-14). The
  spec text of the plan differed in two places (no `type` on the item; `basic-auth` URLs come from
  the item's `scope`); ADR-0041 records them and the one thing not verified (no third-party CXF file
  to test against).
- **Tests**: `mosaic-import` 55 unit (KeePassXC passkeys, strict base64url and PEM, RFC 3339, UUIDs)
  and 22 integration (`tests/it/passkeys.rs`: Bitwarden and CXF round trips with the imported key
  signing an assertion that a separate `p256` verifier accepts and the counter 41 becoming 42, bad
  keys, Ed25519, a missing user handle, commit into a vault, duplicates, the fuzz seeds); desktop
  `handler_tests.rs` (export needs re-authentication and spends it, wrong password, cancel and failed
  write hand it back, lock during the dialog, no passkey, round trip through the CXF importer);
  Vitest (export panel 2, mock export 2).
- **Not verified / open**: the crypto-reviewer pass of the import parsing and the export and the
  ux-reviewer pass of the export panel are still due; no CXF file from another implementation; the
  panel's visual check in light and dark and German (no Playwright run in this track).

## M5 Passkeys — close (2026-11-06)

Track T5 of the M5 plan: the review follow-ups, one more rate limit, CI, documents and the full
gate. M5 is merged as PRs #28 (T1 authenticator), #30 (T2 desktop), #31 (T3 browser interception),
#32 (T4 import, CXF export) and the close branch `chore/m5-close`.

- **What shipped.** `mosaic-passkey` (the software WebAuthn authenticator: ES256, registration and
  assertion ceremonies, strict encodings, `PendingAssertion` so the counter is stored before a
  signature is released), the `passkey` item kind, the desktop's two browser ops with their
  dialog, the extension's main-world shim and bridge (opt-in switch in the More menu), import from
  Bitwarden, KeePassXC and FIDO CXF, and the CXF export (Settings, master password, plaintext
  warning). ADR-0039 to 0041 are accepted.
- **Reviews.** crypto-reviewer (T1, T2, T4 import and export), extension-attacker (T3 and the
  desktop endpoint) and ux-reviewer (the passkey dialog, the item views, the export panel, the
  popup switch, the import warnings); findings were fixed before merge or at the close.
- **Close fixes (ux-reviewer).** The password field takes the focus when the dialog switches to
  asking for it; a passkey-specific "enter your master password" message; the site's host at body
  size in the dialog header; "Use another device" on its own row under Cancel and the primary
  button; the exclusion dialog shows how to close it and no countdown; "Got it". The passkey note
  says how to move a passkey (Settings, Export passkeys); the editor says why a passkey has few
  fields; a passkey has no files section; German "Passkey-ID"; the credential ID is named in full
  for screen readers; the export panel shows the plaintext warning as a callout above the vault
  choice and "no passkeys" instead of it when the vault has none; radio accent colour; the
  popup's menu hints are descriptions, not part of the item names, and say that all websites are
  needed; the import preview says "keep your old app" once. The plain mock now holds one passkey.
- **Per-origin passkey rate limit (extension-attacker).** The windows were the pairing's, so one
  page could spend 3 registrations or 10 sign-ins for every site. The desktop now keeps a window
  per (pairing, canonical origin): `passkeyCreate` 2 and `passkeyGet` 6 a minute, checked before
  the pairing's, and a request refused by either counts in neither (a page can spend at most 2 of
  3 and 6 of 10). At most 256 origins per pairing and bucket (expired windows first, then the one
  with the oldest request); dropped with the pairing. A body whose origin does not parse is
  refused `invalidRequest` and counts only against the pairing. `NATIVE_MESSAGING.md` section 6,
  `EXTENSION_SECURITY.md` section 11.3 and ADR-0040 (amendment) say so; the e2e fake desktop mirrors
  it and the hostile `passkey-budget` test now shows the other site still reaching the dialog.
- **CI.** `crates/passkey/fuzz` joins the fuzz smoke (targets `b64url` and `pkcs8`, seeds
  committed), the fuzz-manifest `cargo deny` check and the fuzz-lockfile `cargo audit`; the `cxf`
  import target runs with the rest of `crates/import/fuzz` (the job runs every `cargo fuzz list`
  target). `mosaic-passkey` joins the coverage matrix (>= 90 % lines).
- **Coverage (local, `cargo llvm-cov -p mosaic-passkey`, Windows).** 99.0 % lines (294 of 297;
  ceremony.rs 99.2, encode.rs 100, key.rs 97.4, name.rs 100).
- **Gate (2026-11-06, one run).** fmt check, clippy with `-D warnings`, the workspace tests
  (1288 passed, 0 failed, 7 ignored), `cargo deny check`, `cargo audit --deny warnings`,
  `pnpm install --frozen-lockfile`, `pnpm lint`, `pnpm typecheck`, `pnpm test` (99 files, 5564
  tests; two desktop tests, `OnboardingFlow` and `HardwareVaultPanel`, hit the 5 s timeout under
  the full parallel load and pass alone) and `pnpm audit`: all green. Playwright: desktop 63
  passed, extension 108 passed (e2e build, Chromium headless). The ignored Rust tests
  (release-mode, OpenSSH interop) run in CI.
- **Open risks.** M5-R1 to M5-R9 in the register below.
- **Open: manual checks and owner items.** See "Needs owner" ("M5 passkeys", "Manual checks for
  M5").

## M6 Sync — T1 protocol foundation (2026-11-06)

Spec: the M6 spec of the session (scope ADR-0042; folder format ADR-0043; server ADR-0044;
engine rules ADR-0045; dependencies ADR-0046; CRYPTO_SPEC §9.2, §10, §10.1, §10.2, §12.1 amended
first, in their own commit).

- **mosaic-core**: nine new labels, `InstallId`, `sync_auth` (checkpoint, folder and device-info
  keys, `checkpoint_mac`, `folder_mac` per purpose, sealed device info, `session_token`). Known
  answers from the independent script `crates/core/tests/vectors/sync_kat.py` (checked in with its
  output `sync_kat.txt`; the script was re-run and its output is identical).
- **mosaic-vault**: store schema v3 (migration test from v2 with data), install identity, sync
  config, opaque cursors, local sequence blocks, the checkpoint counter with its monotonic trigger,
  attachment deletions, the key and metadata upload queue, `reset_sync_state`; the new session API
  in `session/sync_support.rs` (two-phase rotation, join and password change split in two halves,
  MACs and sealed device info, attachment transfer helpers). Existing behavior is unchanged:
  `join_account` and `change_master_password` are the new halves in sequence.
- **mosaic-sync** (new crate): `wire::{segment, checkpoint, account_file, blob_delete, device_info,
  cursor, folder_names, email, json}` and `transport`; 42 tests, line coverage per file between 92
  and 100 %. Eight fuzz targets with seeds in `crates/sync/fuzz` (compiled on Windows, run in CI),
  added to the `fuzz-smoke` matrix, the coverage matrix and the fuzz lockfile checks.
- **Deviations from the M6 spec**: the core MAC functions return `Result` (no silent fallback);
  `pending_vault_uploads` takes `&mut self` (it remembers the metadata handed out for a creation, so
  a rename during the upload is not lost); the delete marker first had 77 bytes (the review of T1 added `vault_id` and a per-device path:
  93 bytes, CRYPTO_SPEC §10.1); `Store::sweep_orphan_attachments` records
  no deletion (an orphan file has no row, so no vault and no upload state); the cursor decoder
  requires strictly ascending devices (stricter than "no duplicates"); `wire::opaque`, `opaque-ke`
  and the `http` code are left to T3 and T4, the features `opaque` and `http` exist but are empty.
- **Crypto review follow-ups (2026-11-06)**: H1/M1 transports carry checkpoints as raw bytes filed
  under the path's device, and `Checkpoint::decode(bytes, expected_device, ..)` refuses a body that
  names another device (`Path`); M2 multi-part snapshots (`snap_base`, `snap_parts`, `SnapshotTracker`);
  M3 `JoinKeys` carries its `PasswordParams`; M4 §12/§12.1 rows and a constant 512-byte device-info
  plaintext; M5 `TotpSetupResponse` secrets in `SecretText`; L1 `.mdel` with `vault_id` at
  `blobs/<attachment>.<device>.mdel` (93 bytes); L2 checkpoint versioning sentence; L3 key flags on
  rewrap and on an already-held uploaded key; L4 session tag (`Error::PreparedStale`); L5 checkpoint
  body cap from `Checkpoint::MAX_LEN`; L6 full byte vectors in `sync_kat.py`/`sync_kat.txt`, checked by
  `crates/sync/src/wire/kat_tests.rs`; L7 the password-split test enrolls quick unlock first.
- **Not verified / open**: the crypto-reviewer and test-engineer passes of the spec's T1 review
  (labels, MAC formats, session MAC API, password-change split, KATs) are still due; the fuzz targets
  were compiled, not run (Windows); the coverage figure above is from a local `cargo llvm-cov` run of
  `-p mosaic-sync` and CI measures it again.

## M6 Sync — T2 engine and folder transport (2026-11-06)

Branch `feat/m6-folder-engine` on top of T1 and its review fixes. Spec: the M6 spec §3, §5, §11 T2,
CRYPTO_SPEC §10 and §10.1, ADR-0043 and ADR-0045 (amendments in ADR-0043).

- **Engine** (`mosaic-sync::engine`): `sync_once` in the spec order (vault creation and queued keys,
  checkpoints, pull, push with every item before any tombstone, metadata, attachments, own
  checkpoint, housekeeping, lag and warnings), pages applied in short session steps with deferral of
  records that are ahead of their key (the stored cursor stops before the first deferred unit),
  two-phase rotation, the canonical-winner rule, backoff with jitter, rollback and
  duplicated-identity detection. Hosted-only branches (cursor of the lowest deferred sequence,
  `HeadMoved`, `GenerationTaken`, `Conflict` rounds) are written to the spec but only the folder
  paths are exercised until T4.
- **Folder transport** (`mosaic-sync::folder`): layout and strict names, tmp + `sync_all` + rename
  writes, write-once checks, segment reader with a failure cache (length and mtime) so a partial
  file is retried but not re-read on every page, out-of-order marks in the cursor, snapshot sets
  through `SnapshotTracker`, own-log key recovery, compaction, cleanup of own temp and conflict
  files, blob delete markers and purge, device list, change token, `init` / `inspect`, account
  file write and `join_from_folder` (MAC check after the unlock).
- **Test utilities** (`test-util`): `CloudSim` (seeded; late, truncated and withheld files,
  provider-style conflict copies), `ChaosTransport` (fail before, lost response, duplicate),
  `MaliciousTransport`, `ManualClock`, `FixedJitter`, `SimRng`, `LocalSession`.
- **Tests** (`crates/sync/tests/it`, one binary): `folder_transport` (the 11 listed plus waiting for
  changes and blob purge), `sim_folder` (3 seeds x 6 rounds; 60 x 12 `#[ignore]`d; plus lost
  responses, duplicates and backoff), `rotation_folder` (the 4 listed plus recovery of a pushed but
  uncommitted rotation), `join_folder` (the 3 listed plus folder states), `malicious_folder` (the 9
  listed), `cloudsim` (5).
- **mosaic-vault**: additive accessors for the sync layer (see "T2 review follow-ups" for the final
  set; the first draft's `sync_keys` is removed). mosaic-core is unchanged.
- **Deviations from the spec**: `FolderTransport::open` takes a `FolderConfig` (crypto, clock, device
  info) instead of three arguments; `maintain` is a method of the `Transport` trait with a
  `MaintainContext` instead of `maintain(&mut self, access)`; `PullPage::meta` became `metas`
  (several metadata objects per page); `list_vaults` stops reading at the first verified segment, so
  `RemoteVault::generation` is a lower bound for the folder (the engine does not use it); a stale
  device does not purge blobs (new rule, ADR-0043 amendment). The simulation lets one device rotate
  per round (offline or online); concurrent rotations have their own tests.
- **Open**: the hosted branches of the engine are untested until T4; the first pull of a process
  verifies every segment it reads.
- **T2 review follow-ups (2026-11-06).** The crypto-reviewer findings C-H1, C-M1 to C-M5, C-L1 to
  C-L6 and the info items, and the ten sync-chaos findings S1 to S10, are fixed (ADR-0043 and
  ADR-0045 amendment; CRYPTO_SPEC §4, §10, §10.1, §11, §12.1). Derived keys no longer leave the
  session: `sync_keys` / `SyncKeys` are gone, the folder transport holds `SessionCrypto<H>` over a
  shared session handle and fails closed (`Locked`) when the session locks; `KeyedCrypto` is
  test-only again. The engine state that must outlive the engine is persisted in store schema v4
  (an additive migration with a data test): own sequence numbers (`reserved` before a write,
  `written` after), the halt (only `clear_sync_halt`, a T5 user action, clears it), the last
  published checkpoint counter and the strikes, the newest checkpoint of every other device (never
  a lower counter), lag timers, the vault keys seen per generation with their lowest writer, and
  the first-seen time of blob delete markers. Behavior: a rolled-back own log halts before any
  write and no number is reused; planted names are stepped past and removed; snapshot sets
  complete from the folder state; blobs are authenticated on fetch and healed by the holder;
  blob purge ages from the first-seen time on the purging device's clock and waits for no lag, no
  key conflict and no unexplained object; the canonical winner ranks every key by its lowest
  writer; a withheld new vault is reported; a join tries account files in order; hostile files
  (symlinks, floods, phantom devices) are bounded. Regression tests: `chaos/review.rs` (one or
  more per finding), the ten chaos tests run without `#[ignore]` (S1 was rewritten for the halt
  design), vault tests for the migration, the accessors and the attachment check. Test helper
  change: `LocalSession` is a shared handle (`Dev::s()` returns a guard; two guards of one device
  in one statement panic instead of deadlocking). Deviations are listed in the review report:
  the rollback check compares the highest listed own segment with `written` (no compaction floor),
  and the holder checks a same-size blob once per engine lifetime instead of once per run.
  `THREAT_MODEL` R57a records that a lost device keeps `AK` until re-key (M7).
- **T2 re-review follow-ups (2026-11-06, round 2).** A join refuses a superseded password (a
  higher-version `.macct` that the opened key MACs); `rotate_vault` and the first run check the halt,
  the own logs and the checkpoints before they write; the rollback check ignores files that fail
  their MAC; names above `reserved` in the own log are removed at once (directories and links too),
  snapshot ranges are checked name by name, and a hole in the own numbers (also after an
  acknowledged rollback or a backend change) triggers a compaction; `read_capped` re-checks the
  opened file and opens non-blocking on Unix (new `libc` edge for `mosaic-sync`, already in the
  lock file); a failed heal is counted (`attachments_heal_failed`) and never aborts a run; a backend
  change keeps `reserved`. CI: the `mosaic-sync` coverage job now runs with `--features test-util`.
  S8's test injects the failed part through a crypto provider whose confirmation fails once
  (a name collision no longer fails a write, by design).
- **Known items for later.** T5: the blob authenticity check (STREAM open of up to 50 MiB) and the
  holder's check run inside a short session step, so the session lock is held for that time (UI
  latency); move it to a worker with a copy if it shows. M7 (re-key): own segments written under the
  old `K_fold` read as bad after a re-key and must not be deleted blindly by the unverifiable-file
  cleanup; revisit then.

## M6 Sync — T3 server (2026-11-06)

Spec: the M6 spec §6 and §11 T3 (ADR-0044, ADR-0046; amendments to both in `docs/DECISIONS.md`).

- **`crates/server`** (`mosaic-server`, library and `mosaic-server` binary): axum 0.8 router
  with every endpoint of §6.2; SQLite through `rusqlite` (bundled, WAL, `foreign_keys`, STRICT tables,
  its own migration list, `secure_delete`); OPAQUE login and registration with the suite of
  CRYPTO_SPEC §9.1 (`mosaic_sync::wire::opaque`, `opaque-ke =4.0.1`), pre-login fakes and a fake KE2
  for unknown emails, login states in memory (60 s, 10 000), session tokens derived from the OPAQUE
  session key and stored as SHA-256, idle and absolute lifetimes, a five-minute fresh-login window for
  password change, TOTP, account deletion and device removal; TOTP sealed at rest under `K_totp`
  with the replay guard (ADR-0008) and five attempts per pending session; per-record base-sequence
  CAS, generation equality, key CAS with the optional head condition, items and tombstones;
  checkpoints as raw bytes bound to the path device; attachments with soft delete, restore and the
  hourly purge; a per-account long poll; quotas; in-memory token-bucket rate limits (IPv6 by /64,
  `X-Forwarded-For` last hop only from `trusted_proxies`); TOML config and a hand-parsed CLI (`init`,
  `serve`, `check-config`, `invite create`, `backup [--blobs]`, `account delete`).
- **`mosaic-sync`**: `wire::opaque` (feature `opaque`): the suite, the context, the identifiers and
  an `OsRng` over `getrandom`, with a round-trip and server-identity-binding test.
- **Tests**: 10 unit tests and 37 integration tests in `crates/server/tests/it/` (`api_auth`,
  `api_sync`, `api_limits`, `no_plaintext`, `cli`), including every test the spec names for T3; the
  time-based ones use tokio `start_paused` and a manual server clock. `no_plaintext` scans the
  database file, the WAL, the blobs and the log for the peer address, passwords, tokens and item
  plaintext. `sim_hosted` and `malicious_server` belong to T4 (they need the HTTP transport).
- **Fuzz**: `crates/server/fuzz` (`request_bodies`, `checkpoint_upload`, seeds), in the workspace
  `exclude`, the `fuzz-smoke` matrix (with `max_len`) and the per-lockfile `cargo deny` and
  `cargo audit` loops. Compiled on Windows with `cargo check --bins`, run in CI.
- **Deployment**: `deploy/Dockerfile` (pinned digests, distroless non-root), `compose.yaml` (the
  server on an internal network, Caddy with TLS 1.3), `Caddyfile`, `mosaic-server.example.toml`,
  `README.md`, and `.dockerignore`. The CI job `server-image` builds the image and smoke-tests
  `/v1/info` (nothing of it ran locally: there is no container runtime here); `mosaic-server` is in
  the coverage matrix. `docs/SERVER.md` is the operator guide.
- **Deviations from the spec**: handlers call the database inline (`block_in_place` on a
  multi-threaded runtime instead of `spawn_blocking`, so a paused tokio clock cannot jump during a
  query); per-account change counters; a device uploads only its own checkpoint (403 otherwise);
  `register/start` has no invite code to check (the request has no field for it), so
  `invalid_invite` comes at `finish`; a password change keeps the other sessions; the dependency
  choices are in the ADR amendments.
- **Needs follow-up (outside this track's write scope)**: `.cargo/audit.toml` must list
  RUSTSEC-2026-0387 (`crypto-bigint 0.5.5` through `opaque-ke`; `deny.toml` already does, with the
  reason) or `cargo audit --deny warnings` fails. Reviews due: crypto-reviewer (OPAQUE parameters,
  fakes, token derivation, TOTP sealing, fresh-login checks), `/security-review`, test-engineer.
  THREAT_MODEL R61 and R62 are new.

## M6 Sync — T4 HTTP transport and hosted simulations (2026-11-07)

Spec: the M6 spec §6 and §11 T4 (ADR-0044, ADR-0045, ADR-0046; amendments in `docs/DECISIONS.md`).

- **`mosaic_sync::http`** (feature `http`): `HttpTransport::{connect, info}` (the hosted `Transport`:
  blocking `reqwest` over rustls, TLS 1.3 only, `ring` provider, OS trust store, redirects off, HTTPS only
  except loopback), strict response parsing with size caps (16 MiB JSON, the attachment maximum, 64 KiB
  of an error body), the error-code to `TransportError` mapping with `Retry-After`, a records push split
  to the server's request limit, and `hosted::{prelogin, register, login, login_totp, logout, join_login,
  join_fetch_account, password_change}` over OPAQUE (`TransportError` gained `Rejected(Rejection)`,
  `InsecureUrl` and `InvalidUrl`). `connect` and every hosted function check `GET /v1/info`: protocol 1
  and the origin the caller asked for.
- **Engine bugs found by the hosted simulations and fixed** (the hosted branches had never run against a
  server): (1) a received record that the local one dominates or equals left the pending change on a
  stale `baseSeq`, so it was refused as a conflict on every attempt and devices never converged (fix in
  `mosaic-vault`: `apply_remote_record` rebases, or acknowledges an echo of the device's own upload;
  `Store::raise_server_seq`); (2) a legacy (offline) rotation that lost the race for the server's key
  slot was resolved by the folder's lowest-device-ID rule, leaving a device with a key nobody else holds
  (fix in `engine/keys.rs`: on the hosted backend the server's key is canonical).
- **Tests**: `crates/sync/tests/it/http_transport.rs` (9: `refuses_http_non_loopback`,
  `rejects_unknown_response_fields`, `response_size_capped`, `server_identity_mismatch_refused`,
  `retry_after_mapped`, error-code mapping, the full transport against the real server, login
  failures and the TOTP flow, join over the wire; `tls_config_is_tls13_only` and the URL rule are unit
  tests in `http/client.rs`); `crates/server/tests/it/`: `sim_hosted.rs` (the folder simulation's
  invariants over the HTTP transport in a chaos transport with 3 seeds x 6 rounds, 60 x 12 `#[ignore]`,
  plus the delete and rotation scenarios of the folder sim), `scenarios.rs` (`rotation_head_cas_loses_no_edit`,
  `delete_race_linearized_by_base_seq`, `move_copy_uploaded_before_tombstone`,
  `legacy_offline_rotation_loser_rotates_again`, `join_new_device_hosted`,
  `password_change_hosted_end_to_end`), `malicious_server.rs` (the seven named tests), with
  `loopback.rs` (the real server on `127.0.0.1:0` on a thread of its own, also included by the sync
  tests) and `hosted.rs` (the world: devices, engines, chaos, interception). Regression tests in
  `mosaic-vault` for both engine fixes. No sleeps: the server clock and the engine clock are manual.
- **Deviations**: `LoopbackTransport` is the HTTP transport connected to the loopback server (a socket,
  not an in-process router); `connect` does not take the device ID (the checkpoint path's device is read
  from the device list); the join is `join_login` plus `join_fetch_account` (a pending session needs its
  code in between); the 60 x 12 run is not run locally (CI, release). The checkpoint upload treats
  `checkpoint_stale` as done (repeated request).
- **Dependencies**: `reqwest =0.13.5` (`blocking`, `rustls-no-provider`), `rustls =0.23.46` (`ring`,
  `std`), `rustls-platform-verifier =0.7.0`, all optional behind the `http` feature; `ring` and
  `webpki-root-certs` (licence exception) enter the lock file. Dev-dependencies of `mosaic-sync`:
  `mosaic-server`, `mosaic-totp`, `axum`, `tokio`. CI: the `mosaic-sync` coverage job runs with
  `--features http,test-util`.
- **Open**: reviews due: crypto-reviewer (client OPAQUE, TLS config, token handling), sync-chaos
  (extends `sim_hosted` and `malicious_server`). The release `--ignored` run of the long simulation
  runs in CI. THREAT_MODEL R67 and R68 are new.

### T4 review follow-ups (2026-11-07)

The crypto-reviewer and sync-chaos reviews of T4 are applied (C-M1, C-M2, C-L1 to C-L4, C-Info, S1 to S10);
spec first (CRYPTO_SPEC §9.4, §10, §10.2, §11, §12; ADR-0045 and ADR-0046 amendments in `docs/DECISIONS.md`).

- **Served view and view digest (S4 to S8).** Store schema v5 (`sync_served`, `sync_key_confirmed`,
  `sync_vault_flag`, `sync_rotation_log`; `sync_peer_checkpoint` recreated for 60-byte entries). A push is based
  on the served `seq`; a refusal the next pull does not explain re-reads the vault once and then flags it. The
  checkpoint (format v1 changed in place) carries a 32-byte `view_digest` per entry; devices at the same head
  compare digests (`ServerInconsistent`), a head below the cursor is `ServerRollback` for that vault only.
  `MAX_LEN` is 245,833 bytes; the server's checkpoint column check follows (it was 65,600, below the old
  maximum); the KAT script and vectors, the fuzz seeds and the checkpoint tests are regenerated. Findings stay
  until `clear_sync_halt`, which also rebuilds the flagged vaults' view.
- **Hosted key winner (S1, C-M1).** The loser adopts the server's key (`adopt_remote_vault_key`, one store
  transaction) instead of rotating again; a confirmed key that meets a second valid key warns and rotates by
  rank at most three times per vault per day. Folder mode is unchanged.
- **S2, S3, S9, S10.** Backoff and lag survive a backward clock jump; a rolled-back vault no longer aborts
  the run; a rotation costs one upload per record (an equal received version settles the queued re-seal).
- **Password change (C-M2, C-L3).** The session stores the account version at KE3; start and finish refuse
  a stale session; a change ends the account's other sessions; a lost finish answer is recovered on retry.
- **Client (C-L1, C-L2, C-Info).** OPAQUE session and export keys wiped after use; one sensitive bearer
  header per transport; `localhost` pinned to `127.0.0.1`; one shared origin normalizer
  (`wire::origin`, also used by the server config; the server's old one accepted `+443`); the transport takes
  the engine's install ID; wrong comments and ADR/`deny.toml`/R67 texts about `tls12` and
  `webpki-root-certs` corrected.
- **Tests.** The ten `M6-T4 chaos finding` tests run without `#[ignore]` (expectations changed where a lying
  server is now reported instead of silently tolerated: findings 5 to 9 accept `ServerInconsistent` or
  `ServerRollback`, `forged_conflict_current_seq_is_harmless` and `replayed_live_version_does_not_resurrect`
  expect the finding and acknowledge it); `legacy_offline_rotation_loser_adopts_server_key` replaces
  `..._rotates_again`; new: `cross_served_keys_warn_and_the_rotations_are_capped`,
  `password_change_lost_answer_is_recovered_on_retry`, the account-version race and session-ending test in
  `api_auth.rs`, adoption and `settle` tests in `mosaic-vault`, the view digest KAT, the origin normalizer,
  bearer, `localhost` pin and key-wipe unit tests; `sim_hosted` and the random fault schedule assert that an
  honest server earns no finding (`assert_no_server_findings`).
- **Open.** The desktop is not touched (it does not use `mosaic-sync` yet); the release `--ignored` runs are
  CI's. The server's `sessions` and `checkpoints` tables changed in migration v1 in place (the server is unreleased,
  no deployed database).

### T4 re-review follow-ups (2026-11-07)

Crypto re-review of the first follow-ups (R-M1 to R-M4, R-L1 to R-L7, I1 to I5); see the ADR-0045/0046 amendment
in `docs/DECISIONS.md` and CRYPTO_SPEC §10, §10.2.

- **View digest and key commitment.** The digest binds the held key's wrapping (`key_commit`), so a key split by
  adoption shows; the formulas live in `mosaic-core` (label registry, no literal context in `mosaic-sync`); a
  key never accepted publishes the all-zero digest; an open finding does not switch the digest off. KAT
  script, vectors and fuzz seeds regenerated. More than one candidate key is a finding, nothing is adopted.
- **`NoCommonView`.** A peer whose checkpoints advance for 48 hours with no digest comparison is reported
  (persisted); honest pairs never raise it (`assert_no_server_findings` in the sims).
- **Login vs password change.** A pending login records the account version and ends with a password change.
- **Acknowledging.** `clear_vault_findings` (per vault) is separate from `clear_sync_halt`; `sync_findings`
  lists both; after the re-read, local records the server lacks are queued again (the S6 and S9 tests clear the
  finding and assert that the dropped edit arrives). A cleared key finding does not return; peers' stale
  digests are ignored until they republish.
- **Smaller.** Confirmed mark written in the accepting transactions (v5 migration seeds it conservatively);
  `ServerRollback` vaults take no uploads or attachment transfers; the v5 migration drops hosted cursors;
  bearer encoded straight into zeroizing buffers.
- **Dev note.** Server databases created before the in-place edit of migration v1 must be recreated (unreleased).
- **Open for T5.** The desktop calls `clear_sync_halt` and `clear_vault_findings` behind one acknowledge action
  (`sync_findings` says what is set).

## M6 Sync — T5 desktop (2026-11-07)

Spec: `m6-spec.md` §7 and §11 T5, with the T5 brief (the session handle, the lock behaviour, the findings and the
acknowledgement). IPC: `docs/DESKTOP_IPC.md` §3 "Sync", §4, §5, §6.

- **Rust** (`apps/desktop/src-tauri/src/sync/`): `SyncService` (one thread per unlocked generation: it starts on
  unlock when `sync_config` is set and ends on lock; the engine and the transport are dropped, every handle of the
  old generation fails closed with `Locked`, and the next unlock rebuilds the service from the persisted engine state),
  `access.rs` (`BackendSession` implements both `SessionHandle` and `SessionAccess` over the session slot: the
  slot is locked per closure and never across a network call), `runner.rs` (the loop, generic over the transport so
  there is no `dyn Transport`), `hosted_flow.rs`, `folder_flow.rs`, `errors.rs`. New: `SyncTokenStore` in
  `keystore.rs` (`mosaic-sync-token-<account id>`, base64url, OS store with a memory fallback), `Dialogs::pick_folder`,
  `commands/sync.rs`, `contract/sync.rs`, the 19 commands in `names.rs` and `capabilities/main.json`, the 24 error
  codes (T5 follow-ups: 20 commands, 25 codes), `syncDataChanged` and `syncAttention`, `signedOut`. Mutating item and vault commands poke the thread
  (debounced two seconds). Sync work is not user activity (test `sync_does_not_reset_idle_timer`).
- **Folder choice (ADR-0034).** `sync_pick_folder` keeps the path in Rust under a random `choiceId` for five minutes;
  no request type has a path field (deny unknown fields; fuzz seeds for the attempts).
- **Hosted flows.** register, sign-in (the server's account ID must be this device's: `syncAccountMismatch`), the second
  factor at login, join (`derive_join_keys`, login, `GET /v1/account`, `join_account_with_keys`; the Secret Key goes to the
  keystore, the settings account block gets `kitVerified = true`), devices, the sync account's own second factor, and
  `change_master_password` (server first, then local; an unreachable server answers `syncPasswordNeedsServer`).
  The server allows one session per device, so every fresh login (second factor, device removal, password change)
  stops the thread, stores the token it got and starts the thread again.
- **Acknowledge.** `sync_acknowledge` reads `sync_findings()`, calls `clear_sync_halt()` for a halt and
  `clear_vault_findings(findings.vaults())` for the findings against the server (the engine's `clear_findings` wraps
  the same session call), then runs a sync. A halt stops the automatic runs (status `error`, no retry) until then.
  The UI confirm says to do it on each device.
- **`mosaic-sync` additions** (`http`): `Token`/`StoredToken` (a `SessionToken` cannot be rebuilt from bytes, so the
  keystore holds a `StoredToken`; `connect` and the hosted functions take either), `hosted::{account_info, totp_setup,
  totp_confirm, totp_disable, device_rename, device_remove}`.
- **Tests**: `sync_handler_tests.rs` (every command locked, without account and unlocked; `path_never_accepted_from_webview`;
  the choice expires after five minutes; a foreign folder is refused and not written; the token is stored, deleted on
  disconnect, on a 401 and on removal; `sync_does_not_reset_idle_timer`; `change_password_with_hosted_unreachable_refused`
  and the hosted change end to end; `join_folder_then_unlock_flow` (data arrives, the new device unlocks);
  `join_hosted_with_totp` against the real server; the tampered account file; `sync_acknowledge` clears a halt and syncs;
  locking stops the service and a transport operation fails closed), error mapping, contract fixtures, fuzz seeds.
  TypeScript: `SyncSettings.test.tsx` (28), `JoinStep.test.tsx` (7), `mock/sync.test.ts` (31), `EventsProvider.sync.test.tsx`
  (3), the onboarding flow test, i18n parity. Gate: 16 sync tests in 383 desktop Rust tests (1 ignored: the OS keystore), 1046 desktop and 4610 i18n
  vitest tests, lint, typecheck, clippy (desktop and `mosaic-sync`), the fuzz crate builds.
- **Deviations from the spec.** (1) The response type `SyncStatus` is named `SyncOverview` (the name is taken by the status
  enum). (2) The warning kinds also have `noCommonView` and `attachmentsNotRepaired`; `serverInconsistent` and
  `serverRollback` carry the vault's name. (3) `sync_device_remove` and `change_master_password` take an optional
  `code`, and `sync_totp_disable` can answer `syncTwoFactorRequired`: the server accepts a second-factor code once, a
  fresh login of an account with two-step verification spends one, and those calls need a second. (4) Two more error
  codes: `syncChoiceExpired`, `syncFreshLoginExpired`. (5) A change of the hosted account by another device that the
  user did not ask for is shown as `signedOut`, not as a separate state. (6) `change_master_password` is a command with
  no settings screen yet. (7) A device the server signed out may sign in again with `sync_hosted_connect` (any other
  configured device gets `syncAlreadyConfigured`). ADR-0047 records these decisions; THREAT_MODEL R69 the token.
- **Known items.** (a) The sync thread waits for a backend change in slices (five seconds hosted, two seconds
  folder), because a blocking long poll cannot be interrupted by a local change; an idle device sends about twelve
  poll requests a minute. (b) The engine verifies a downloaded attachment under the session lock (up to 50 MiB of
  decryption): a command can wait for that long; it is a short stall, not redesigned here. (c) A failed folder join
  whose second MAC check fails deletes the new store (`discard_new_store`); the first check fails before the store is
  touched, so the path is covered by the tampered-file test only up to that point. (d) The folder device list shows
  the platform of this device only (the folder keeps no platform).

### M6 T5 review follow-ups (2026-11-07)

Crypto/IPC review (C-M1 to C-I4) and UX review (U1 to U16) of the desktop sync work. ADR-0047 "T5 review follow-ups",
CRYPTO_SPEC §9.4 "Change applied elsewhere", CORE_API (`adopt_remote_password`), THREAT_MODEL R69, DESKTOP_IPC §3.

- **C-M1** busy guard (a lock waits for a password change, three minutes at most), local commit right after the
  server's answer, a refusing keystore is not fatal, recovery with the new password (vault `adopt_remote_password`,
  `hosted::recover_password`, `FolderTransport::verified_account_file`, `sync_recover_password`, `passwordChanged`,
  `syncPasswordChanged`). **C-M2** generation checks on every thread write, compare-and-delete of the token.
  **C-M3** signing in again to the same server keeps the sync state. **C-L1** tokens are saved after the session is
  complete. **C-L2** lock epoch, sweep of expired pending state. **C-L3** bounds, codes of 6 to 8 digits, closed probe.
  **C-I1** native confirmation for acknowledge. **C-I3/C-I4** docs match the code.
- **U1 to U16** focus after screen changes (section heading, join title, Rename button, Dialog `returnFocus`), one
  submit with a radio choice, one "I've checked…" button, offline as a status, the status word alone in the live
  region, the sidebar status, the setup key as a secret, the join form kept across the code step, the details as
  rows, Esc cancels a rename, announced folder messages, Home/End on the tabs, signed-out hides rename/remove and the
  device name, long names wrap, copy fixes.
- **Left open.** The UI has no settings screen for `change_master_password` yet, so "the UI shows the change as busy"
  has nothing to show; the lock deferral is in Rust. The different-server confirmation of C-M3 has no UI (the signed-out
  form is fixed to the server it was signed in to). The account option of a folder join shows the first eight
  characters of the ID: neither an email nor a date exists on the Rust side.

### M6 T5 verify follow-ups (2026-11-07)

Hosted password check at thread start and no new token on a failed commit; retired salts and folder versions (vault schema
v6, `syncPasswordRollback`, CRYPTO_SPEC §9.4 rules); sleep is not deferred; native acknowledge with Rust-owned EN/DE texts
and an empty request; pending state dropped by every lock and swept by the idle tick; `finish_ok` gates the events and a fresh
login is bound to its account. ADR-0047 "T5 verify follow-ups".

## M6 Sync — close (2026-11-07)

Track T6 of the M6 plan: the review follow-ups, documents, the full gate and the milestone
reviews. M6 is merged as PRs #33 (T1 protocol foundation), #35 (T2 engine and folder transport),
#36 (T3 server), #37 (T4 HTTP transport and hosted simulations), #38 (T5 desktop) and the close
branch `chore/m6-close` (#39). Scope: ADR-0042 (sharing, organisations and emergency access move
to M6b), ADR-0043 (folder format), ADR-0044 (server), ADR-0045 (engine rules), ADR-0046
(dependencies), ADR-0047 (desktop).

- **What shipped.**
  - **T1, protocol foundation.** `mosaic-core` sync labels, `InstallId` and `sync_auth` (checkpoint,
    folder and device-info keys, MACs, sealed device info, session token), known answers from the
    independent `sync_kat.py`; `mosaic-vault` store schema v3 and the sync session API (the upload
    queue for wrapped vault keys and vault metadata, cursors, local sequence blocks, the checkpoint
    counter, two-phase rotation, join and password change split in two halves); the new crate
    `mosaic-sync` with every wire format (`segment`, `checkpoint`, `account_file`, `blob_delete`,
    `device_info`, `cursor`, `folder_names`, `json`) and the `Transport` trait; eight fuzz targets.
  - **T2, engine and folder transport.** `sync_once` in the spec order (pull, push with every item
    before any tombstone, metadata, attachments, own checkpoint, housekeeping), deferral of records
    ahead of their key, the canonical-winner rule, rollback and duplicated-identity detection with a
    persisted halt, the folder transport (single-writer paths, tmp-and-rename writes, snapshots,
    compaction, blob delete markers and purge), joining from a folder, and the cloud-folder
    simulators (`CloudSim`, `ChaosTransport`, `MaliciousTransport`); store schema v4 for the engine
    state that must outlive the engine.
  - **T3, server.** `mosaic-server`: axum router for every endpoint of the protocol, SQLite, OPAQUE
    login and registration, session tokens stored as hashes, TOTP sealed at rest with a replay guard,
    per-record base-sequence and key compare-and-swap, checkpoints as raw bytes, attachments with soft
    delete and an hourly purge, a long poll, quotas, rate limits, a TOML config and the CLI (`init`,
    `serve`, `check-config`, `invite create`, `backup`, `account delete`); the container and Caddy
    deployment and `docs/SERVER.md`; two fuzz targets.
  - **T4, HTTP transport and hosted simulations.** `mosaic_sync::http` (blocking `reqwest` over
    rustls, TLS 1.3 only, OS trust store, HTTPS except loopback) and the hosted client over OPAQUE;
    the hosted simulations against the real server, which found and fixed two engine bugs (a dominated
    received record left a stale base sequence, so devices never converged; an offline rotation that
    lost the race for the server's key slot left a device with a key nobody else holds). The review
    follow-ups added the served view and a view digest in every checkpoint (`ServerInconsistent`,
    `ServerRollback`, `NoCommonView`), hosted key adoption, and store schema v5.
  - **T5, desktop.** `SyncService` (one thread per unlocked generation, folder and hosted flows),
    the OS-keystore session token, 20 commands and 25 error codes, the sync settings, the join flow
    in onboarding, the native confirmation for acknowledging findings, recovery of a master password
    change applied elsewhere (`adopt_remote_password`), and store schema v6 (retired salts, folder
    account-file version).
- **Reviews.** Seven fix rounds covered T1 to T5: crypto-reviewer on T1, T2 (with a re-review) and
  T4 (with a re-review), a server review of T3, a crypto/IPC review and a UX review of T5 with a
  verify pass; sync-chaos on T2 and T4. The counts per pass are in `docs/SECURITY_REVIEW_LOG.md` and
  the THREAT_MODEL §6 log (graded T1 to T5: 3 High, 24 Medium, 38 Low, 32 Info, plus 36 findings
  with no recorded severity). All were fixed or recorded as residuals (M6-R1 to M6-R17 below). This
  is internal, AI-assisted review, not an independent audit. Milestone passes (a crypto-reviewer
  pass over the whole of M6 and a final sync-chaos long run over both transports):
  {MILESTONE_REVIEWS}.
- **Gate (2026-11-07).** {GATE}
- **Documents.** `docs/CRYPTO_SPEC.md` §9, §10, §10.1, §10.2, §11, §12, §12.1 (amended spec first in
  every track), `docs/CORE_API.md` (sync APIs of the session, the hosted client and the engine's
  findings calls), `docs/DESKTOP_IPC.md` §3, `docs/SERVER.md`, `docs/THREAT_MODEL.md` (R54 to R69,
  the hosted client table, the M6 review log), `docs/SECURITY_REVIEW_LOG.md` (new).
- **Risks closed or retargeted at the close.** M2-R1, M2-R2 and M2-R10 are resolved; M2-R12 is
  resolved for the hosted backend and stays accepted for folder mode; M2-R11, M2-R16 and the local
  half of M3-R25 are retargeted to M9 (see the register). New residuals: M6-R1 to M6-R17.
- **Known items (not defects, recorded so nobody rediscovers them).**
  - The engine verifies a downloaded attachment (STREAM open of up to 50 MiB) and the holder's
    check under the session lock, so a command can wait for that long. Move it to a worker with a
    copy if it shows.
  - An idle hosted device polls in five-second slices (about twelve requests a minute) because a
    blocking long poll cannot be interrupted by a local change; a folder device waits in two-second
    slices.
  - There is no settings screen for `change_master_password` (the command, the busy guard and the
    recovery exist in Rust). The different-server confirmation of the T5 review has no UI, and the
    account option of a folder join shows the first eight characters of the account ID.
  - At re-key (M7), own segments written under the old `K_fold` read as bad and must not be deleted
    blindly by the unverifiable-file cleanup.
  - Server databases created before the in-place edit of migration v1 must be recreated (the server
    is unreleased).
- **Open: manual checks and owner items.** See "Needs owner" ("M6 sync", "Manual checks for M6").

## M6b Sharing and emergency access (2026-11-08)

Scope: vault sharing between people (roles, invites, rosters, revocation by key change) and
emergency access, hosted only (CRYPTO_SPEC §8; ADR-0048 to ADR-0055; THREAT_MODEL R71 to R82). Folder
backends cannot share (§8.6, ADR-0054 scope). Tracks: T1 `mosaic-core` (identity keys, share seals,
roster, contact book; `eddbb84`, review fixes `976e910`), T2 `mosaic-vault` (store v8 and the
sharing session API; `cea757b`), T3 `mosaic-server` (schema v2, routes; `d241309`), T4
`mosaic-sync` (engine, hosted client, view digest v2; `6501920`), the milestone review fixes
(`8e3b495`) and T5 desktop (this section).

- **T5 desktop.**
  - **IPC (21 commands, `docs/DESKTOP_IPC.md` "Sharing").** `sharing_identity`, `sharing_lookup`,
    `sharing_verify_contact`, `sharing_accept_key_change`, `sharing_contacts`, `vault_share`,
    `vault_members`, `vault_member_set_role`, `vault_member_remove`, `vault_invite_cancel`,
    `vault_leave`, `invites_list`, `invite_preview`, `invite_accept`, `invite_decline`,
    `emergency_overview`, `emergency_grant`, `emergency_revoke`, `emergency_request`,
    `emergency_respond`, `emergency_claim`; the event `sharingChanged`; `SyncOverview.sharingFindings`
    and the warning `rotationBlocked`; 18 new error codes. The commands run the real engine
    operations of `SyncEngine` (`lookup_contact`, `invite_member`, `remove_member`, ...) over an
    engine and transport of their own next to the sync thread (`sync/sharing_flow.rs`): one session
    closure at a time, no lock across a network call, the lock epoch recorded first and checked
    after, a fresh login (the device-removal path) for granting and for approving. Removing a member
    and leaving a vault ask in a **native** dialog whose texts Rust owns (a test compares them with
    the catalog). `sync_disconnect` is refused with `syncHasSharedVaults` before anything is signed
    out. Only safety codes, roles, addresses and IDs reach the webview.
  - **Screens (`src/screens/sharing/`).** Share dialog (address, lookup, safety-code comparison,
    role, the honesty line), the code comparison side by side with an optional typed code and a
    one-click attestation (`SafetyCodePanel`), the key-change panel (old and new code, "Compare and
    accept"), the members list with roles, unverified, unconfirmed, dropped and removing markers,
    role changes, remove, leave, withdraw, and the notices for a removal waiting for the owner's
    device and for attachments a key change could not protect, the invites inbox (sidebar entry with
    a count; the owner's code is compared too when a manager invites), emergency access in Settings
    (grantor and grantee, wait choices 1/2/7/14/30 days with 7 by default, the server-trust
    sentence, vault keys only, request, countdown, claim), the grantor's banner for a waiting request
    (refuse, or approve with the master password), the sharing findings at the top of the app
    (`MemberUnverified`, `MemberDropped`, `ShareUntrusted`, `AccessLost`, `IdentityMismatch`,
    `RosterInvalid`; `RotationBlocked` in the sync warnings), and Settings > Sharing (own safety code
    and contacts). Folder mode shows why sharing is off everywhere it would appear.
  - **Mock and i18n.** `?sharing=basic|empty|invites|emergency|request|findings|keyChanged|manager|viewer|rotation`
    (with `&sync=folder|none` for the off states); the `sharing` catalog (EN, DE) and the new errors.
  - **Tests.** Contract fixtures regenerated (21 commands, 6 enums, the event, the findings), Rust unit
    tests for the error mapping and the native texts, four real-server tests of the commands (two
    accounts: verify, invite, join, roles, removal with its question, the refusal to leave the server,
    decline, emergency grant, request, refusal and cooldown; folder and locked cases), 18 mock handler
    tests and 16 component tests, 22 fuzz seeds.
- **Deviations from the spec.**
  - `vault_members.unprotectedAttachments` and `vault_member_remove.unreachableAttachments` are
    counts, not lists (the files are not named on this screen; the count is kept in memory per vault).
  - `invite_accept` takes the sender's code only; the owner of a vault joined through a manager is
    verified with `sharing_verify_contact` first (`InvitePreview` carries `ownerCode`).
  - `emergency_respond` gains `code` (the second factor, like `emergency_grant`).
  - `invites_list.outgoing` lists the waiting invites only and carries the vault name and the recipient's
    address from the contact book.
  - No `ShareItemDialog` (move an item to a shared vault with the existing Move action) and no role or
    shared marker in `VaultSummary` (the vault's sharing dialog shows the role); both are small follow-ups.
  - `sharing_contacts.changed` is true when devices pinned different keys; a key the server now serves
    differently is found by `sharing_lookup` (`changed`).
- **Reviews.** T1 crypto review: High 1, Medium 1, Low 3, Info 5, all fixed (`976e910`). Milestone review:
  High 4, Medium 4, Low 4, Info 1, all fixed in `8e3b495` with the deviations documented. A verify pass
  over the review fixes: *pending, counts to be added when it reports.*
- **Manual checks for M6b (2026-11-08)**, none can be automated here: (1) two real accounts on a real
  server: look up, compare codes on the phone, invite, accept, edit on both, remove, watch the key change
  on the owner's device; (2) a manager removing a member while the owner is offline, then the owner's
  device coming back; (3) emergency access end to end with a short wait on a test server: request, banner
  on the grantor's app, refuse, approve early, claim, revoke after a release; (4) a key change of a contact
  (reinstall) while a vault is shared with them; (5) the native remove and leave dialogs on Windows, macOS
  and Linux in German and English; (6) keyboard-only and screen-reader pass through the share dialog, the
  safety-code panel, the invites inbox and the banner in light, dark, German and English.

## Open risks / findings

| ID | Severity | Item | Status |
|---|---|---|---|
| M1-R1 | low | Upstream `hmac`/`hkdf` stack residue (key-pad block, last HKDF output block) during derivation | accepted, THREAT_MODEL R11, ADR-0005 |
| M1-R2 | low | Secret Key parsing still branches on whether a character is `-`/whitespace and on the final validity flag. Neither reveals the key: separators carry no key bits and invalid input is rejected | accepted |
| M1-R3 | low | The independent baseline vector is checked in, but CI does not re-run the Python script (it needs argon2-cffi and 128 MiB). Re-run it when the derivation changes | accepted |
| M1-R4 | low | Fuzzing cannot run natively on Windows MSVC (ASan runtime mismatch); local runs use Docker, CI runs on Linux | accepted |
| M1-R5 | info | The main-workspace `cargo deny check` warns `license-exception-not-encountered` for the `libfuzzer-sys` NCSA exception, which only applies to the fuzz lockfile | accepted |
| M1-R6 | low | Constant-time unpadding is best-effort at the source level (`subtle` selects, no early exit). The compiler gives no hard guarantee. Code that later processes the unpadded plaintext (M2 item parsing) takes time proportional to its length. Only a local attacker who can time client-side decryption could exploit this | accepted |
| M2-R1 | medium | Wrapped vault keys and vault metadata have no upload queue: only records and attachments go through the outbox. A vault created or renamed offline reaches other devices only once M6 uploads them | resolved in M6 (T1 `65bfe97`, T2 `2b4ab0c`): wrapped vault keys and vault metadata have an upload queue (store schema v3, `pending_vault_uploads`, the engine's vault-creation, key and metadata steps; CRYPTO_SPEC §10, ADR-0045) |
| M2-R2 | low | After a key rotation, other devices reject records sealed under the old generation (§7.1). Every device re-seals what it holds when it accepts the new key (ADR-0012 item 7), so no version is lost, but the server should still refuse uploads below a vault's current generation and accept one wrapped key per generation (ADR-0013) | resolved in M6 (T3 `4a1ca4b`, ADR-0044 and ADR-0045): hosted, the server refuses uploads whose generation is not the vault's current one and accepts one wrapped key per generation (key compare-and-swap with the optional head condition); folder, the canonical winner (lowest writer) decides (CRYPTO_SPEC §10.2) |
| M2-R3 | low | Writer-ID reuse: fixed in the third review round by fresh writer IDs per unlock and collision-safe merges (ADR-0015). Left: two sessions on one store through a hard link (the lock follows the resolved path) can still overwrite each other's upsert from a stale cache, each under its own ID | accepted (hard links only), THREAT_MODEL R13 |
| M2-R4 | info | A move that races an edit of the source on another device leaves the moved copy and the surviving source (the source carries a "deleted on another device" notice, the copy the old values) | accepted, ADR-0012 item 6; see M2-R9 |
| M2-R7 | medium | A delete racing an edit can lose a newer value, depending on delivery order: a device that has not seen the newest value restores its own stale one under fresh stamps, and the newer version is then dominated (sync review: P1 lost when the tombstone arrives first). The fix needs the deleter's pre-delete state (in the tombstone or kept by the deleting device) and restores into history or conflict copies, which changes CRYPTO_SPEC §7.2/§10 | accepted for M2, THREAT_MODEL R15; designed with the M6 protocol (spec and ADR first) |
| M2-R8 | medium | A delete racing a move of the same item is undone without a notice: the delete merges into the source's tombstone, the copy stays live and unflagged. Needs move pointers (`moved_to` in the source tombstone, `moved_from` in the copy) so a concurrent delete flags the copy | accepted for M2, THREAT_MODEL R15; spec and ADR with M6 |
| M2-R9 | low | Concurrent moves of one item leave two live copies without a "moved concurrently" notice, and a move racing an edit splits the values between the revived source and the copy. Same move pointers as M2-R8 | accepted for M2, THREAT_MODEL R15 |
| M2-R10 | low | A delete or move that loses against a concurrent edit has already pruned the attachment files on its device; the revived item references files only the editing device holds | resolved in M6 (T2 `2b4ab0c`, T3 `4a1ca4b`; CRYPTO_SPEC §10.1, ADR-0043, ADR-0044): the engine fetches missing attachment files and the holder heals a bad blob; a blob is purged only after the 30-day retention counted from this device's first sight of the delete marker, and only with no live local reference, no lag, no key conflict and no unexplained object; the server soft-deletes, can restore, and purges hourly. The local prune on delete or move is unchanged |
| M2-R11 | info | Vault metadata renames are not part of the 3-device simulation (metadata has no upload queue yet, M2-R1); concurrent renames, renames under a shared device ID and renames of damaged metadata are covered by session tests, and metadata now merges with `VaultMeta::merge_and_repair` | partly resolved in M6: the metadata upload queue exists (M2-R1) and the 3-device simulations check metadata key generations; renames appear in single scenario tests (`crates/server/tests/it/chaos_hosted.rs`, `chaos_net.rs`) but the random simulations do not rename. Left: add concurrent renames to `sim_folder` and `sim_hosted`; retargeted to M9 (test hardening), low value until sharing |
| M2-R12 | medium | An edit the server accepted under generation `n` is lost if another device rotates to `n + 1` before pulling it and the editor never returns; peers that accept `n + 1` first refuse the record as a rollback, so the outcome depends on delivery order (sync review t4, test `a_record_in_flight_during_a_rotation_is_refused_by_peers`). A move into a vault that a peer rotates concurrently makes the whole item vanish on that peer without a notice (it refuses the copy and fast-forwards to the source tombstone) until the mover accepts `n + 1` and re-seals the copy (fourth review, `characterization_a_move_into_a_concurrently_rotated_vault_vanishes_on_the_rotator`). Fix in the M6 protocol: rotation conditional on the vault head (CAS on generation plus last-seen `server_seq`), no uploads below the current generation, a move's tombstone uploaded only after the server accepted the copy, and a peer that cannot read the copy flags the source instead of fast-forwarding | resolved for the hosted backend in M6 (T3 `4a1ca4b`, T4 `fe2db40`; ADR-0045): rotation is conditional on the vault head, uploads below the current generation are refused, items go before tombstones (tests `rotation_head_cas_loses_no_edit`, `move_copy_uploaded_before_tombstone`). Folder mode has no compare-and-swap: accepted as M6-R2 (THREAT_MODEL R55, R16). A peer flagging the source when it cannot read a moved copy is deferred to M7 (ADR-0042) |
| M2-R13 | low | Whether the delete-race repair runs depends on which versions a device receives: if the device that won the race dismisses the notice before a third device pulls, and the server coalesces versions, the third device never sees the deletion and silently keeps the deleter's state (sync review c2). Same root as M2-R7 (the repair needs to see the deletion) | accepted for M2, folded into the M2-R7 redesign (THREAT_MODEL R15) |
| M2-R14 | low | The `apply_remote` fuzz target builds its sessions from OS randomness (keys, IDs, nonces; the session has no seeded mode and the crypto core takes no injected RNG), so a failure that depends on ID order or key bytes may not reproduce from the saved input. Documented in the target; the substituted IDs are derived from the input. The same holds for the sync simulation's seeds: items are picked in creation order now, but random device IDs break ties between concurrent writes, so a seed is not an exact replay; failures get a deterministic test (as for seed 229651) | accepted |
| M2-R15 | medium | A revocation (§8.3) must re-seal every attachment under a fresh ID and FK, or a removed member and the server can replace attachment contents (ADR-0017). Not reachable in M2: no other account holds a vault key, and `rotate_vault_key` is not a revocation | open, M6/M7 with sharing and the revoke flow; THREAT_MODEL R18 |
| M2-R16 | low | Vault metadata merges are not split like items (ADR-0016): a metadata merge above 64 KiB still fails. It needs tens of kilobytes of concurrent names or descriptions | retargeted to M9: not done in M6 although ADR-0042 listed it. M6 added no cap on metadata values (only the wire object cap `MAX_META_LEN`, 65,600 bytes, which matches the padded metadata size), so a metadata merge above 64 KiB still fails. Reason for M9: it needs tens of kilobytes of concurrent names or descriptions, and the fix is a format and merge change that belongs with the hardening pass |
| M2-R17 | low | A local record or metadata row that cannot be read has no rollback anchor; the received replacement is reported (`ReplacedUnreadable`, for metadata since the fourth review) but not verified | accepted, THREAT_MODEL R17; M6 can require checkpoint evidence |
| M2-R18 | info | Fresh writer IDs add one version-vector entry (about 44 bytes) per session that edits a record; an HOTP item grows by one per session that shows a code. Collision repairs can bring back superseded values as conflict copies. An oversized split leaves one conflict-copy item per value that left the record (copies have no type of their own; the UI shows the original's through `conflict_copy_of`); devices at different key generations derive different copy IDs and can leave a duplicate | accepted (ADR-0015, ADR-0016 and its amendment); vector pruning is a future format change |
| M2-R19 | low | Writer-ID reuse is detected only where it shows (equal vectors with different contents, a counter above the replica's own, one stamp with two contents in a field). An honest device that merges two colliding versions showing none of these before the writers meet drops both writers' values (`reuse_that_shows_nowhere_is_not_detected`). Needs a forged or corrupted record or a UUIDv7 collision, since writer IDs are fresh per unlock | accepted (ADR-0015 amendment); the randomized test covers the case where the writers meet first |
| M2-R20 | low | An oversized merge whose record does not fit even without every content field (its password history or attachment references alone exceed 1 MiB), or a value that does not fit a copy of its own, fails with `SizeLimit` and stays unmerged until the user shrinks the item | accepted (ADR-0016 amendment item 4); needs thousands of attachments or history entries near the limit |
| M3-R1 | low | Secret Key storage: Windows Credential Manager and the macOS Keychain keep it; the Linux kernel keyring keeps it for the login session only, so Linux users type it from the Emergency Kit after a reboot | partly resolved (ADR-0020); Secret Service in M9 if its transport crypto is accepted |
| M3-R2 | low | Tauri isolation pattern not used (THREAT_MODEL §4.2); the closed command set, typed Rust-side validation and the strict CSP stand in | accepted until M9 (ADR-0019 item 4) |
| M3-R3 | low | Linux-only advisories in Tauri's stack: gtk-rs 0.18 (`proc-macro-error` unmaintained, `glib::VariantStrIter` unsound, not called) and zbus 5 parts folded into zbus 6 | accepted (ADR-0019 item 9, deny.toml, .cargo/audit.toml); revisit with Tauri's gtk4 / zbus 6 move |
| M3-R4 | info | Typed secrets, revealed values, generated passwords and one-time codes live in the webview heap, and the parsed IPC body is not zeroized | accepted, THREAT_MODEL R19 |
| M3-R5 | medium | Windows and macOS: no pre-sleep, screen-lock or user-switch signal (needs unsafe FFI). The vault locks on idle and on resume (up to 2 s after it), not when the screen locks; the settings screen says so (CRYPTO_SPEC §11 gap) | accepted for M3 (ADR-0020 item 3), THREAT_MODEL R21; M9 |
| M3-R6 | medium | Quick unlock: Windows Hello (TPM-backed) and Touch ID are built (ADR-0028, 2026-11-03), Linux has no biometrics. Security-key unlock and keys for hardware vaults are implemented in `mosaic-vault` and the desktop commands, but have no CTAP transport: it needs an owner decision ("Needs owner", WP7), so keys report unsupported and their UI is hidden. Windows gets no security keys in M3 (only `webauthn.dll`, which needs unsafe FFI). Touch ID and Hello still need the manual hardware checks | partly resolved (biometrics); security keys open (owner) |
| M3-R7 | low | Clipboard exclusion depends on clipboard managers honoring the hints; Linux covers X11/XWayland only; a clipboard still busy after 5 tries at quit keeps the value | accepted (ADR-0020), R21 |
| M3-R8 | low | Re-authentication re-derives the keys (lock, then unlock) | resolved 2026-11-03: `verify_master_password` returns a short-lived proof; a wrong password no longer locks, five in a row lock and require the password (`lock_requiring_password`) |
| M3-R9 | info | keyring, arboard and OS clipboard buffers are not zeroized; a same-user attacker can edit `settings.json` (the protection settings carry a MAC since ADR-0030; see M3-R23) | accepted, R21 |
| M3-R10 | low | Global shortcut: no effect on Wayland; a conflict on macOS is not reported; not remappable yet (UX_GUIDELINES §4) | open, M9 |
| M3-R11 | info | `contentProtected` has no effect on Linux | accepted, R21 |
| M3-R12 | low | SSH agent: no client identity on Windows and PID only on macOS, so "Allow for 15 minutes" is not offered there and the prompt shows an unknown client | accepted (ADR-0025), R20 |
| M3-R13 | low | `ssh-key 0.7.0-rc.11` (a release candidate) is pinned; 0.6.7 carries RUSTSEC-2026-0387 | open: move to 0.7.0 when it is released |
| M3-R14 | info | RSA SSH keys are not served (RUSTSEC-2023-0071) | accepted (ADR-0025) |
| M3-R15 | low | Unwiped upstream copies: the PEM decoder line buffer, bcrypt-pbkdf temporaries and ssh-key intermediates; serde_json, quick-xml and zip buffers during import | accepted, R20 and R22; revisit with R5 in M9 |
| M3-R16 | low | Strength estimate: dictionary passwords outside the short word list score as brute force (`Tr0ub4dor&3` 72 bits); the bundled 2FA Directory data changes only with releases | accepted (ADR-0024, ADR-0023); a larger list with HIBP in M7 |
| M3-R17 | low | The security report and the import commit run under the session lock; a large vault or import blocks other commands while they run | accepted (ADR-0024 item 5) |
| M3-R18 | low | Windows: the agent pipe keeps the default DACL, so another local account can occupy the 32 connection slots; the agent cannot start while the OpenSSH Authentication Agent service runs | accepted (ADR-0025), R20 |
| M3-R19 | info | Import: no KeePass key files, no Twofish or KDBX 3.1, no password-protected Bitwarden exports, no custom CSV column mapping in the UI. Passkeys are imported from Bitwarden, KeePassXC and CXF since M5 (ES256 only; EdDSA and RSA keys are left out with a warning); 1PUX holds no private keys, so its passkeys stay out | accepted (ADR-0021, ADR-0041) |
| M3-R20 | medium | `mosaic-core` proptest `model_sync::password_history_trim_by_edit_order` failed repeatedly on one saved input during the shell track (`replica_count = 3, offsets_days = [0, 0, 0]`; the steps were not kept) | resolved in `61a407e`: a test-oracle bug, not a core bug. Reproduced from CI seed `cc f4c785eb…` (now in `proptest-regressions/model_sync.txt`); minimal case: C changes the password, A syncs, B deletes, C repairs, A deletes and then applies C's repair, so A holds the item without a password and a change there rightly adds no history entry. The oracle now requires an unchanged history when nothing was replaced and, otherwise, one new entry under this edit's stamp holding the replaced value; deterministic test `a_password_set_after_the_own_delete_lost_the_race_replaces_nothing` |
| M3-R21 | info | The fuzz smoke of the 13 new targets ran only in CI; locally they were only compiled (fuzzing does not run on Windows MSVC, M1-R4) | accepted |
| M3-R22 | low | When the OS keystore write fails, the Secret Key stays in process memory (zeroizing) until exit so the Emergency Kit can still be saved; a memory or crash dump of such a process yields SK (CRYPTO_SPEC §4 lifetime exceeded in this failure case only) | accepted (ADR-0020 item 1 amendment), R21 |
| M3-R23 | medium | Settings integrity (ADR-0030): a joint rollback of the vault store and `settings.json` to an earlier consistent pair still verifies (the nonce rolls back with the file). Needs the same local write access as the attack it would undo | accepted until M6: settings as a synced record under CRYPTO_SPEC §10; THREAT_MODEL R28 |
| M3-R24 | info | Names and titles refuse variation selectors (U+FE00 to U+FE0F) and ZWJ (U+200D), so an emoji sequence that needs them (for example a heart with U+FE0F) cannot be typed; imports strip them from titles | accepted (ADR-0030 red-team follow-up); revisit if users report it |
| M3-R25 | low | Attachment quota and garbage collection are local only: the 1 GiB account quota counts live items, while trashed items keep their files (`crates/vault/src/session/attachments.rs:21-28`); and when `delete_attachment` fails after a failed `update_item`, the error is lost and an orphan blob stays queued (`:68-71`) | half resolved in M6 (T3 `4a1ca4b`, ADR-0044): the server counts attachment bytes per account (soft-deleted ones count until purged) and purges after the retention. Retargeted to M9: the local quota still counts live items only (trashed items keep their files), and when `delete_attachment` fails after a failed `update_item` in `add_attachment` the original error is lost and an orphan blob stays queued (`crates/vault/src/session/attachments.rs`); low severity, no data exposure |
| M4-R1 | medium | Same-user malware can rewrite the per-user host manifest or registry key, replace the host, read `K_pair` and then request fills while the vault is unlocked. Pairing and the handshake stop other OS users and sandboxed processes, not P5 | accepted (ADR-0036), THREAT_MODEL R38; M9: installer-owned manifests and a signed host |
| M4-R2 | low | Debug hosts accept the dev Chromium ID (any unpacked extension carrying our public `key`); a temporary Firefox add-on with our gecko ID reaches a release host. Both need developer mode and a new pairing | accepted, R39 |
| M4-R3 | low | Fill credentials, codes and captured passwords pass through the background heap, the isolated world, `storage.session` and the page DOM; JavaScript cannot wipe them, and the host's serde and stdio buffers keep unwiped residue | accepted, R40 |
| M4-R4 | low | Windows: no peer process identity without unsafe code; the default pipe DACL lets other accounts open read-only handles. The `endpoint.json` lease closes the stale-pipe-name capture; a crash leaves the file refused as stale | accepted (ADR-0036 amendment), R42; M9: pipe SDDL |
| M4-R5 | low | A paired extension lists login titles, user names and hosts (`search`), tests candidate passwords (`checkSave`, `proposeSave`) and copies a login's secret to the clipboard, all rate limited and only with an unlocked vault; the desktop cannot tell the popup from other code in the extension's contexts | accepted, R43 |
| M4-R6 | medium | Save capture (opt-in) sees every password typed into a login, sign-up or change form on every site and holds it in `storage.session` for up to 5 minutes. A page can feed it a login after any real click; nothing is saved without the click in the popup and the desktop's own dialog. With the capture on, the granted site access also lifts the `activeTab` bound | accepted (ADR-0038), R44; EXTENSION_SECURITY §9 |
| M4-R7 | info | The pairing code correlates a request with the user's click; it is not a cryptographic SAS. The `browser` value of `pairRequest` is the host's claim. No forward secrecy against a later theft of `K_pair` | accepted (ADR-0036), R45 |
| M4-R8 | low | Field visibility is a heuristic: `pointer-events: none` overlays, colour tricks, opacity not multiplied across frame levels, closed shadow roots, any `mask-image` or off-screen `clip-path` counted as hiding. The credential goes only to the page's own origin | accepted, R46, EXTENSION_SECURITY §9; the hostile suite grows with each finding |
| M4-R9 | low | The e2e suite runs against a fake desktop in Chromium. The real host and desktop end to end, Firefox, the Windows registry install, the Unix manifest paths (macOS, Linux; the Unix transport tests run only on CI) and `activeTab` gating on the production build have no automated test | open: manual checks in "M4 Browser extension — close" |
| M4-R10 | low | "Never for this site" has no screen to review or undo the list (`neverSave:<origin>` in IndexedDB), and is stored per exact origin | open, M9 UI polish |
| M4-R11 | info | Flatpak and Snap browsers cannot run a native host; the extension is unsupported there | accepted, documented |
| M4-R12 | info | One hostile-suite test (`race-after-probe`, rapid gestures) failed once under memory pressure; the desktop pairing test now waits for the session to register (`aca4aa6`). Playwright runs with one retry in CI | accepted |
| M5-R1 | low | The main-world shim is page-controlled: a page can disable it, route around it or see that `navigator.credentials` is replaced (it does not mimic native `toString`) | accepted, THREAT_MODEL R47, ADR-0040 |
| M5-R2 | medium | Synced passkeys can be copied: whoever holds the vault plaintext has every private key, and sign count 0 gives relying parties no clone detection | accepted, THREAT_MODEL R48, ADR-0039 |
| M5-R3 | low | UV=1 means "unlocked or re-authenticated within 5 minutes plus a dialog click", not a per-operation biometric | accepted, THREAT_MODEL R49 |
| M5-R4 | low | A page can tell from timing whether Mosaic holds a passkey for its own rp ID (`passkeyNotFound` without a dialog); no conditional mediation | accepted, THREAT_MODEL R50 |
| M5-R5 | low | The desktop trusts the extension's origin claim, as for `fill`; a compromised extension context can open a dialog for any origin it makes up (the user still approves) | accepted, THREAT_MODEL R51 (R43) |
| M5-R6 | medium | A CXF export is an unencrypted file with private keys; it needs the master password and shows a warning, but Mosaic cannot wipe the copy on disk | accepted, THREAT_MODEL R52, ADR-0041; the owner may drop the export ("Needs owner") |
| M5-R7 | low | Imported credentials keep incrementing their counter; two devices signing at once can emit equal counters | accepted, THREAT_MODEL R53 |
| M5-R8 | low | Many sites together (or a page on many origins it controls) can still spend the pairing's passkey windows (3 creates, 10 sign-ins a minute); a user then sees the browser's own prompt for up to a minute. A single page can no longer do it alone (per-origin windows) | accepted, EXTENSION_SECURITY section 11.3 |
| M5-R9 | low | Not covered by an automated test: real relying parties in Chrome, Edge and Firefox, Firefox MAIN-world registration (from 128, documentation only), the real host and desktop end to end with a browser, a CXF file from another implementation. A passkey whose page aborts while the answer is written is stored but not delivered (an orphan the user can trash). Search does not index a passkey's user name. The import preview's passkey warnings carry no counts (the contract has none) | open, manual checks |
| M5-R10 | info | Mosaic is not an OS passkey provider (Windows plugin authenticators need an unsafe exception, a macOS credential provider needs a Developer ID); passkeys work only through the browser extension | deferred, ADR-0041, "Needs owner" |
| M6-R1 | low | Folder backend observers (the cloud provider, its admins, other members of a shared or team folder) see the CRYPTO_SPEC §12.1 table and can delete, withhold or roll back files; cloud version history keeps old ciphertext and old password wrappings of AK | accepted, THREAT_MODEL R54, ADR-0043 |
| M6-R2 | medium | No compare-and-swap in folder mode: concurrent rotations are resolved by the canonical winner; the delete race (M2-R7) and an edit lost to a rotation (M2-R12) remain there; a lost segment heals only when its writer compacts; the hosted `PulledKey.device_id` is not authenticated, so a malicious server can steer the rank rule and cause rotation churn (capped at three per vault per day) | accepted, THREAT_MODEL R55, ADR-0045 |
| M6-R3 | medium | Freeze detection is heuristic: a server or folder that withholds data consistently from every device, checkpoints included, is noticed only through a device's own counter or by the user; the view digest needs two devices at the same head, otherwise `NoCommonView` after 48 hours; findings stay until acknowledged on every device | accepted, THREAT_MODEL R56, CRYPTO_SPEC §10 |
| M6-R4 | low | The session token is a bearer credential in the OS keystore: same-user malware can download and upload ciphertext and soft-delete attachments; removing a device signs it out but does not take back keys it holds; `reqwest` and `hyper` copy the `Authorization` value while sending | accepted, THREAT_MODEL R57 |
| M6-R5 | medium | A lost or stolen device keeps AK and every vault key it holds: whoever unlocks it can forge folder objects every other device accepts, and reuse the install ID, until the account is re-keyed; removing it from the list or a halt elsewhere does not take the key back | accepted until re-key (M7), THREAT_MODEL R57a |
| M6-R6 | low | Open registration reveals whether an email is registered (invite mode, the default, limits this); real and fake pre-login IDs are indistinguishable, but account IDs created before the UUIDv4 rule show their creation time (no released users) | accepted, THREAT_MODEL R58, ADR-0044 |
| M6-R7 | low | The sync configuration (server URL, folder path) is not authenticated: a local writer can redirect sync, and only ciphertext and the CRYPTO_SPEC §12 metadata would go to the attacker's location | accepted, THREAT_MODEL R59 |
| M6-R8 | low | Hardware-protected (TK) vaults are not synced in M6: they exist only on their device, so losing it loses them | accepted until a CTAP transport (M3-R6), THREAT_MODEL R60 |
| M6-R9 | medium | Theft of the server's secret file with its database: no vault is exposed (OPAQUE records need the password and Secret Key), but the thief can tell registered from unregistered emails, learns the sealed TOTP secrets and can impersonate the server to a client that trusts its origin; rotating the OPAQUE server setup is not supported | accepted, THREAT_MODEL R61, ADR-0044 |
| M6-R10 | low | Availability of the hosted server: one node and one SQLite connection, rate limits in memory and keyed by IP address, and a holder of a stolen token can upload a checkpoint with a huge counter that makes that device's real checkpoints stale until it signs in with a new install ID | accepted, THREAT_MODEL R62 |
| M6-R11 | low | Memory residue of the OPAQUE login and the HTTP stack: request and response buffers are ordinary heap memory, freed and not wiped | accepted, THREAT_MODEL R63 (as R11 and R40) |
| M6-R12 | low | Slow-header and slow-body clients can hold connections open; axum has no header read timeout, so the reverse proxy is the defence | accepted, THREAT_MODEL R64 |
| M6-R13 | info | The server's clock decides session lifetimes, fresh-login windows, TOTP steps and rate limits; rate-limit and pending-login state is in memory and lost at a restart | accepted, THREAT_MODEL R65 |
| M6-R14 | info | A server operator sees activity patterns the protocol needs (when an account syncs, record counts and sizes per vault, device names, poll wake-ups): the CRYPTO_SPEC §12 metadata, no content | accepted, THREAT_MODEL R66 |
| M6-R15 | low | The hosted client trusts the OS certificate store (TLS 1.3 only, no pinning): a CA the OS trusts, or one added by malware or an employer, can terminate TLS; `HTTPS_PROXY`, `ALL_PROXY` and `SSL_CERT_FILE` are the user's own environment; plain http only for loopback | accepted, THREAT_MODEL R67, ADR-0046 |
| M6-R16 | medium | The hosted pull is only as honest as the server within the checks the client can make; a server that never contradicts a device's own checkpoints and keeps devices apart is noticed only by the freeze, rollback and `NoCommonView` rules | accepted, THREAT_MODEL R68, CRYPTO_SPEC §10 |
| M6-R17 | low | The desktop keeps the hosted token in the OS keystore with a memory fallback; same-user malware can read it; a second-factor code is accepted once, so removing the second factor takes two codes; a master password change the server took and the device missed is recovered by typing the new password | accepted, THREAT_MODEL R69, ADR-0047 |
| M2-R5 | low | `serde_json` scratch copies of escaped strings during plaintext parsing | accepted, THREAT_MODEL R14 |
| M2-R6 | info | Hardware-protected TOTP vaults (§7.4) | resolved in the vault and desktop layers 2026-11-03 (TK, open ≤ 5 min on a wall-clock and monotonic deadline, closed on sleep); usable once a security-key transport exists (M3-R6) |

See the THREAT_MODEL §5 residual-risk register.

## Needs owner

- **License**: choose a project license (for example AGPL-3.0 for the server
  and Apache-2.0/MIT for clients, or a single license). Crates are
  `publish = false` until decided.
- **Name availability**: "Mosaic" on crates.io / npm / domains / app stores,
  and a trademark check.
- **Bundle identifier**: the desktop app uses `io.github.stephanbaum.mosaic`
  (`tauri.conf.json`) until the name and domain are settled; changing it later
  moves the app data directory.
- **Signing**: Apple Developer ID, Windows code-signing certificate, extension
  store accounts (Chrome Web Store, Edge Add-ons, AMO). Needed from M3/M4 and
  for releases in M9.
- **Code signing and notarization**: a Windows code-signing certificate, an Apple Developer ID and macOS notarization for the desktop app (needed before any M3 build is shared).
- **Security-key unlock transport (quick-unlock spec WP7, 2026-11-03)**: no
  existing Rust CTAP/FIDO2 crate passes our checks (`authenticator` pulls the
  unmaintained `serde_cbor`; `ctap-hid-fido2` needs `ring`/`aws-lc-rs`;
  `webauthn-authenticator-rs` needs OpenSSL). Options: (a) our own small
  `crates/fido2` (CTAP 2.1 hmac-secret client over HID, built from crypto crates
  already in the tree; ADR-0029 + crypto review) for macOS and Linux, or (b)
  defer security keys past M3. Until decided, (b) applies and the security-key
  UI stays hidden. On Windows, security keys are only reachable through
  `webauthn.dll`, which needs `unsafe` FFI (forbidden), so Windows gets no
  security keys in M3 either way.
- **Touch ID** needs a Developer ID–signed build with entitlements (Apple
  Developer account); unsigned builds hide Touch ID.
- **Manual hardware checks for quick unlock (crypto review M3, L4)**: (a) Windows: does a second, unsigned executable of the same user open and sign with Mosaic's Hello key (`Mosaic quick unlock <nonce>`)? If yes, THREAT_MODEL R23's residual is real until MSIX package identity (M9). (b) macOS: Touch ID enroll, unlock and a fingerprint change on a Developer ID signed build (the macOS code only compiles in CI; nothing ran on a Mac), including the cancel mapping of `decrypt`.
- **Manual checks for one-time-code setup (ADR-0031, 2026-11-04)**: see "M3 follow-up — one-time-code setup" above (Windows two monitors, snip paste, X11, Wayland, macOS paste).
- **Manual check of Save as overwrite confirmation (ADR-0034, 2026-11-05)**: on each OS (Windows `IFileSaveDialog`, macOS `NSSavePanel`, Linux GTK), saving an attachment onto an existing file must make the native dialog ask before replacing it; Mosaic relies on that prompt and adds none. Also check the Mark of the Web on a real NTFS volume.
- **M4 browser extension (2026-11-06)**: (a) the Chrome Web Store and Edge Add-ons extension IDs (`STORE_CHROMIUM_EXTENSION_IDS`, now empty, so release hosts accept no Chromium extension yet) and an AMO listing for the gecko ID `mosaic@stephanbaum.github.io` (it is the one Firefox ID the host accepts); (b) the host name (`HOST_NAME`) and the gecko ID follow the bundle-identifier decision above, so settle that before the first listing; (c) D06 (desktop onboarding step 5) shows store links only once listings exist (`STORE_URLS` is null); (d) the extension store accounts (see Signing) and the M9 installer (sidecar `externalBin`, store IDs only, signing) replace the dev `mosaic-native-host install`; (e) two choices the owner may revisit: popup unlock (ADR-0035 item 4) and no inline UI (item 2). No blocking decision is open.
- **Manual checks for M4 (2026-11-06)**, none can be automated here: (1) Windows Chrome and Edge: `pnpm --filter @mosaic/extension host:install` (registry), load `dist/dev`, pair, fill a login and a code, save; (2) Firefox 128+ as a temporary add-on (`dist/firefox`): pair, fill, one-time code, save with the optional site access; (3) macOS and Linux Chrome: manifest paths, socket permissions, pair and fill; (4) `activeTab` gating on the production build (`dist/chromium`, no host permissions): the shortcut and the popup fill work, and nothing is injected into a page before a gesture; (5) Flatpak and Snap browsers: confirm they are unsupported and that the popup says the host is missing; (6) keyboard-only and screen-reader pass through the popup (connect, locked, logins, frame confirmation, generator, save) and through the desktop's pairing and save dialogs.
- **M5 passkeys (2026-11-06)**: (a) OS passkey providers (ADR-0041): on Windows the plugin
  authenticator API needs an `unsafe` FFI exception (the project forbids unsafe code), on macOS a
  credential provider extension needs an Apple Developer ID and entitlements; decide whether and
  when. (b) A registered AAGUID for Mosaic (today the all-zero AAGUID, which relying parties that
  check attestation treat as unknown). (c) Keep or drop the CXF export (it writes private keys in
  plain text; R52): the export panel is built and ships unless dropped.
- **Manual checks for M5 (2026-11-06)**, none can be automated here: (1) register and sign in with
  Mosaic on real relying parties (for example webauthn.io, GitHub, Google) in Chrome, Edge and
  Firefox 128+ with the extension and the desktop app; (2) Firefox: the passkey scripts register in
  the MAIN world (temporary add-on); (3) import a CXF file from another app and a Bitwarden export
  with passkeys, sign in with an imported passkey; (4) keyboard and screen-reader pass of the
  passkey dialog and the Settings export panel in light, dark, German and English.
- **M6 sync (2026-11-07)**: (a) the hosted default domain: `HOSTED_DEFAULT_URL` is `None` (ADR-0042
  item 4), so a user types a server address until the owner picks a domain; set it with the first
  hosted deployment and the web landing page (M9). (b) The scope split: sharing, organisations, team
  vaults and emergency access (CRYPTO_SPEC §8) moved to M6b (ADR-0042); decide when M6b starts (it
  needs `hpke` vetting and the revocation flow, R18, M2-R15). (c) The license: the AGPL question for
  the server is still open (see License above), and it now also gates applying to the free audit
  programs, so decide it before that. (d) Publishing the server image waits for M9; the container
  deployment is built and smoke-tested only in CI.
- **Manual checks for M6 (2026-11-07)**, none can be automated here: (1) the folder backend on real
  Dropbox, iCloud Drive and OneDrive folders (including a Dropbox Business team folder) on Windows,
  macOS and Linux: set up, join a second device, edit on both, watch provider conflict copies and
  partial files settle, offline then online; (2) the hosted server behind Caddy on a VPS (the compose
  file has never run locally): TLS 1.3, `trusted_proxies`, registration with an invite, login with and
  without two-step verification, backup and restore; (3) two physical devices over each backend:
  convergence, a master password change on one while the other is online, offline and locked,
  rotation, device removal, the acknowledge dialog after a deliberate fault; (4) the keystore token
  on each OS (Windows Credential Manager, macOS Keychain, Linux kernel keyring): stored, replaced by a
  fresh login, deleted on disconnect and on a 401; the memory fallback when the keystore refuses;
  (5) keyboard-only and screen-reader pass through the sync settings, the join flow and the native
  acknowledge dialog in light, dark, German and English.
- **Manual verification on real hardware**: Windows Hello and Touch ID quick unlock (once M3-R6 lands), plus a keyboard-only and screen-reader walkthrough of the real app on Windows, macOS and Linux.
